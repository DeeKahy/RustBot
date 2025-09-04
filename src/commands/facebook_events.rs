use std::collections::HashMap;
use std::fs;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use chrono::DateTime;
use poise::serenity_prelude::{self as serenity, ChannelId, CreateEmbed, Http};
use reqwest::Client;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tokio_cron_scheduler::{Job, JobScheduler};

use crate::{Context, Error};

// Data structures for scraped Facebook events
#[derive(Debug, Serialize, Deserialize, Clone)]
struct FacebookEvent {
    id: String,
    name: String,
    description: Option<String>,
    start_time: Option<String>,
    location: Option<String>,
    url: String,
    page_name: String,
}

// Configuration structures
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
struct FacebookMonitorConfig {
    pub guild_channels: HashMap<u64, Vec<FacebookChannelConfig>>, // guild_id -> configs
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct FacebookChannelConfig {
    pub channel_id: u64,
    pub facebook_pages: Vec<String>, // Facebook page IDs or usernames
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
struct FacebookEventHistory {
    pub seen_events: HashMap<String, u64>, // event_id -> first_seen_timestamp
}

const CONFIG_FILE: &str = "/tmp/facebook_monitor_config.json";
const HISTORY_FILE: &str = "/tmp/facebook_event_history.json";

// Global state for the scheduler
static SCHEDULER: Mutex<Option<JobScheduler>> = Mutex::const_new(None);

/// Set up Facebook event monitoring for a channel
#[poise::command(
    slash_command,
    prefix_command,
    required_permissions = "MANAGE_CHANNELS"
)]
pub async fn facebook_monitor(
    ctx: Context<'_>,
    #[description = "Facebook page URLs or usernames (comma-separated)"] pages: String,
) -> Result<(), Error> {
    let guild_id = ctx
        .guild_id()
        .ok_or("This command can only be used in servers")?;
    let channel_id = ctx.channel_id();

    // Parse Facebook pages
    let facebook_pages: Vec<String> = pages
        .split(',')
        .map(|s| {
            let trimmed = s.trim();
            // Extract page ID/username from Facebook URLs
            if trimmed.starts_with("https://www.facebook.com/") {
                trimmed
                    .trim_start_matches("https://www.facebook.com/")
                    .trim_end_matches('/')
                    .to_string()
            } else if trimmed.starts_with("facebook.com/") {
                trimmed
                    .trim_start_matches("facebook.com/")
                    .trim_end_matches('/')
                    .to_string()
            } else {
                trimmed.to_string()
            }
        })
        .collect();

    if facebook_pages.is_empty() {
        ctx.say("❌ Please provide at least one Facebook page!")
            .await?;
        return Ok(());
    }

    // Load existing config
    let mut config = load_config().unwrap_or_default();

    // Add or update the configuration for this guild
    let guild_configs = config.guild_channels.entry(guild_id.get()).or_default();

    // Check if this channel already has a config
    if let Some(existing_config) = guild_configs
        .iter_mut()
        .find(|c| c.channel_id == channel_id.get())
    {
        existing_config.facebook_pages = facebook_pages.clone();
    } else {
        guild_configs.push(FacebookChannelConfig {
            channel_id: channel_id.get(),
            facebook_pages: facebook_pages.clone(),
        });
    }

    // Save config
    save_config(&config)?;

    let pages_display = facebook_pages.join(", ");
    ctx.say(format!(
        "✅ Facebook event monitoring set up for this channel!\n\
        **Monitoring pages:** {}\n\
        **Channel:** <#{}>\n\
        \n\
        The bot will check for new events every 2 hours and post them here.\n\
        \n\
        ⚠️ **Note:** This uses web scraping of public Facebook pages. Some events may not be detected if Facebook changes their page structure.",
        pages_display,
        channel_id.get()
    )).await?;

    // Start the scheduler if it's not already running
    start_facebook_event_scheduler(ctx.serenity_context().http.clone()).await;

    Ok(())
}

/// Remove Facebook event monitoring from this channel
#[poise::command(
    slash_command,
    prefix_command,
    required_permissions = "MANAGE_CHANNELS"
)]
pub async fn facebook_unmonitor(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx
        .guild_id()
        .ok_or("This command can only be used in servers")?;
    let channel_id = ctx.channel_id();

    // Load existing config
    let mut config = load_config().unwrap_or_default();

    // Remove the configuration for this channel
    if let Some(guild_configs) = config.guild_channels.get_mut(&guild_id.get()) {
        guild_configs.retain(|c| c.channel_id != channel_id.get());
        if guild_configs.is_empty() {
            config.guild_channels.remove(&guild_id.get());
        }
    }

    // Save config
    save_config(&config)?;

    ctx.say("✅ Facebook event monitoring removed from this channel!")
        .await?;
    Ok(())
}

/// List Facebook event monitoring configurations for this server
#[poise::command(
    slash_command,
    prefix_command,
    required_permissions = "MANAGE_CHANNELS"
)]
pub async fn facebook_list(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx
        .guild_id()
        .ok_or("This command can only be used in servers")?;

    // Load config
    let config = load_config().unwrap_or_default();

    if let Some(guild_configs) = config.guild_channels.get(&guild_id.get()) {
        if guild_configs.is_empty() {
            ctx.say("📋 No Facebook event monitoring configured for this server.")
                .await?;
            return Ok(());
        }

        let mut response = String::from("📋 **Facebook Event Monitoring Configurations:**\n\n");

        for config in guild_configs {
            let pages_display = config.facebook_pages.join(", ");
            response.push_str(&format!(
                "**Channel:** <#{}>\n**Pages:** {}\n\n",
                config.channel_id, pages_display
            ));
        }

        ctx.say(response).await?;
    } else {
        ctx.say("📋 No Facebook event monitoring configured for this server.")
            .await?;
    }

    Ok(())
}

/// Manually trigger a Facebook events check (for testing)
#[poise::command(slash_command, prefix_command, required_permissions = "ADMINISTRATOR")]
pub async fn facebook_check(ctx: Context<'_>) -> Result<(), Error> {
    ctx.say("🔍 Checking for new Facebook events...").await?;

    let result = check_facebook_events(ctx.serenity_context().http.clone()).await;

    match result {
        Ok(events_found) => {
            ctx.say(format!(
                "✅ Facebook events check completed! Found {} new events.",
                events_found
            ))
            .await?;
        }
        Err(e) => {
            ctx.say(format!("❌ Facebook events check failed: {}", e))
                .await?;
        }
    }

    Ok(())
}

// Helper functions
fn load_config() -> Result<FacebookMonitorConfig, Box<dyn std::error::Error + Send + Sync>> {
    if !std::path::Path::new(CONFIG_FILE).exists() {
        return Ok(FacebookMonitorConfig::default());
    }

    let config_str = fs::read_to_string(CONFIG_FILE)?;
    let config: FacebookMonitorConfig = serde_json::from_str(&config_str)?;
    Ok(config)
}

fn save_config(
    config: &FacebookMonitorConfig,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config_str = serde_json::to_string_pretty(config)?;
    fs::write(CONFIG_FILE, config_str)?;
    Ok(())
}

fn load_event_history() -> Result<FacebookEventHistory, Box<dyn std::error::Error + Send + Sync>> {
    if !std::path::Path::new(HISTORY_FILE).exists() {
        return Ok(FacebookEventHistory::default());
    }

    let history_str = fs::read_to_string(HISTORY_FILE)?;
    let history: FacebookEventHistory = serde_json::from_str(&history_str)?;
    Ok(history)
}

fn save_event_history(
    history: &FacebookEventHistory,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let history_str = serde_json::to_string_pretty(history)?;
    fs::write(HISTORY_FILE, history_str)?;
    Ok(())
}

async fn scrape_facebook_events(
    page_id: &str,
) -> Result<Vec<FacebookEvent>, Box<dyn std::error::Error + Send + Sync>> {
    let client = Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .build()?;

    // Try different Facebook page URL formats
    let urls = vec![
        format!("https://www.facebook.com/{}/events", page_id),
        format!("https://m.facebook.com/{}/events", page_id),
        format!("https://www.facebook.com/pg/{}/events", page_id),
    ];

    for url in urls {
        log::info!("Attempting to scrape Facebook events from: {}", url);

        match client.get(&url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    let html_content = response.text().await?;

                    if let Ok(events) = parse_facebook_events_html(&html_content, page_id) {
                        if !events.is_empty() {
                            log::info!("Successfully scraped {} events from {}", events.len(), url);
                            return Ok(events);
                        }
                    }
                }
            }
            Err(e) => {
                log::warn!("Failed to fetch {}: {}", url, e);
            }
        }
    }

    // If scraping fails, try RSS approach as fallback
    match try_rss_approach(page_id).await {
        Ok(events) if !events.is_empty() => {
            log::info!(
                "Successfully got {} events via RSS for {}",
                events.len(),
                page_id
            );
            Ok(events)
        }
        _ => {
            log::warn!("No events found for page: {}", page_id);
            Ok(vec![])
        }
    }
}

fn parse_facebook_events_html(
    html_content: &str,
    page_id: &str,
) -> Result<Vec<FacebookEvent>, Box<dyn std::error::Error + Send + Sync>> {
    let document = Html::parse_document(html_content);
    let mut events = Vec::new();

    // Look for event links in the HTML
    // Facebook uses different selectors, so we'll try multiple approaches
    let event_selectors = vec![
        r#"a[href*="/events/"]"#,
        r#"a[href*="facebook.com/events/"]"#,
        r#"[data-testid*="event"]"#,
    ];

    for selector_str in event_selectors {
        if let Ok(selector) = Selector::parse(selector_str) {
            for element in document.select(&selector) {
                if let Some(href) = element.value().attr("href") {
                    if let Some(event) = extract_event_from_link(href, page_id) {
                        // Check for duplicate events
                        if !events.iter().any(|e: &FacebookEvent| e.id == event.id) {
                            events.push(event);
                        }
                    }
                }
            }
        }
    }

    // Also look for structured data
    if let Ok(script_selector) = Selector::parse(r#"script[type="application/ld+json"]"#) {
        for script in document.select(&script_selector) {
            let script_text = script.text().collect::<String>();
            if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(&script_text) {
                if let Some(structured_events) =
                    extract_events_from_structured_data(&json_value, page_id)
                {
                    events.extend(structured_events);
                }
            }
        }
    }

    Ok(events)
}

fn extract_event_from_link(href: &str, page_id: &str) -> Option<FacebookEvent> {
    // Extract event ID from Facebook event URLs
    let event_id = if href.contains("/events/") {
        href.split("/events/")
            .nth(1)?
            .split('/')
            .next()?
            .split('?')
            .next()?
            .to_string()
    } else {
        return None;
    };

    // Skip if it's not a valid event ID (should be numeric)
    if !event_id.chars().all(|c| c.is_ascii_digit()) || event_id.is_empty() {
        return None;
    }

    let full_url = if href.starts_with("http") {
        href.to_string()
    } else {
        format!("https://www.facebook.com{}", href)
    };

    Some(FacebookEvent {
        id: event_id,
        name: "New Event".to_string(), // Will be updated if we can get more details
        description: None,
        start_time: None,
        location: None,
        url: full_url,
        page_name: page_id.to_string(),
    })
}

fn extract_events_from_structured_data(
    json_value: &serde_json::Value,
    page_id: &str,
) -> Option<Vec<FacebookEvent>> {
    let mut events = Vec::new();

    // Handle different structured data formats
    if let Some(json_obj) = json_value.as_object() {
        if json_obj.get("@type")?.as_str() == Some("Event") {
            if let Some(event) = parse_structured_event(json_obj, page_id) {
                events.push(event);
            }
        }
    } else if let Some(json_array) = json_value.as_array() {
        for item in json_array {
            if let Some(obj) = item.as_object() {
                if obj.get("@type")?.as_str() == Some("Event") {
                    if let Some(event) = parse_structured_event(obj, page_id) {
                        events.push(event);
                    }
                }
            }
        }
    }

    if events.is_empty() {
        None
    } else {
        Some(events)
    }
}

fn parse_structured_event(
    event_obj: &serde_json::Map<String, serde_json::Value>,
    page_id: &str,
) -> Option<FacebookEvent> {
    let name = event_obj.get("name")?.as_str()?.to_string();
    let url = event_obj.get("url")?.as_str()?.to_string();

    // Extract event ID from URL
    let event_id = url
        .split("/events/")
        .nth(1)?
        .split('/')
        .next()?
        .split('?')
        .next()?
        .to_string();

    Some(FacebookEvent {
        id: event_id,
        name,
        description: event_obj
            .get("description")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        start_time: event_obj
            .get("startDate")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        location: event_obj
            .get("location")
            .and_then(|v| v.get("name"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        url,
        page_name: page_id.to_string(),
    })
}

async fn try_rss_approach(
    page_id: &str,
) -> Result<Vec<FacebookEvent>, Box<dyn std::error::Error + Send + Sync>> {
    // Try RSS.app service as a fallback
    let client = Client::new();
    let rss_url = format!("https://rss.app/feeds/v1.1/_facebook_{}.xml", page_id);

    match client.get(&rss_url).send().await {
        Ok(response) => {
            if response.status().is_success() {
                let rss_content = response.text().await?;
                parse_rss_for_events(&rss_content, page_id)
            } else {
                Ok(vec![])
            }
        }
        Err(_) => Ok(vec![]),
    }
}

fn parse_rss_for_events(
    rss_content: &str,
    page_id: &str,
) -> Result<Vec<FacebookEvent>, Box<dyn std::error::Error + Send + Sync>> {
    let document = Html::parse_document(rss_content);
    let mut events = Vec::new();

    if let Ok(item_selector) = Selector::parse("item") {
        for item in document.select(&item_selector) {
            if let Ok(title_selector) = Selector::parse("title") {
                if let Some(title_element) = item.select(&title_selector).next() {
                    let title = title_element.text().collect::<String>();

                    // Only consider items that mention "event"
                    if title.to_lowercase().contains("event") {
                        // Try to extract event info from RSS item
                        let event_id = format!(
                            "rss_{}_{}",
                            page_id,
                            title
                                .chars()
                                .filter(|c| c.is_alphanumeric())
                                .take(10)
                                .collect::<String>()
                        );

                        let url = if let Ok(link_selector) = Selector::parse("link") {
                            item.select(&link_selector)
                                .next()
                                .map(|e| e.text().collect::<String>())
                                .unwrap_or_else(|| format!("https://www.facebook.com/{}", page_id))
                        } else {
                            format!("https://www.facebook.com/{}", page_id)
                        };

                        events.push(FacebookEvent {
                            id: event_id,
                            name: title,
                            description: None,
                            start_time: None,
                            location: None,
                            url,
                            page_name: page_id.to_string(),
                        });
                    }
                }
            }
        }
    }

    Ok(events)
}

async fn check_facebook_events(
    http: Arc<Http>,
) -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
    // Load configuration
    let config = load_config()?;

    if config.guild_channels.is_empty() {
        return Ok(0); // No configurations, nothing to do
    }

    // Load event history
    let mut history = load_event_history()?;
    let mut new_events_count = 0;

    let current_time = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();

    // Clean up old events from history (older than 30 days)
    let thirty_days_ago = current_time - (30 * 24 * 60 * 60);
    history
        .seen_events
        .retain(|_, &mut timestamp| timestamp > thirty_days_ago);

    // Check each guild's configurations
    for guild_configs in config.guild_channels.values() {
        for channel_config in guild_configs {
            let channel_id = ChannelId::new(channel_config.channel_id);

            // Check each Facebook page for this channel
            for page_id in &channel_config.facebook_pages {
                match scrape_facebook_events(page_id).await {
                    Ok(events) => {
                        for event in events {
                            // Check if this is a new event
                            if !history.seen_events.contains_key(&event.id) {
                                // Mark as seen
                                history.seen_events.insert(event.id.clone(), current_time);

                                // Post to Discord
                                if let Err(e) =
                                    post_event_to_discord(&http, channel_id, &event).await
                                {
                                    log::error!(
                                        "Failed to post event {} to Discord: {}",
                                        event.id,
                                        e
                                    );
                                } else {
                                    new_events_count += 1;
                                    log::info!(
                                        "Posted new Facebook event {} to channel {}",
                                        event.id,
                                        channel_id
                                    );
                                }
                            }
                        }
                    }
                    Err(e) => {
                        log::error!(
                            "Failed to scrape events from Facebook page {}: {}",
                            page_id,
                            e
                        );

                        // Optionally, send an error message to the channel
                        let _ = channel_id
                            .say(
                                &http,
                                format!(
                                    "⚠️ Failed to check Facebook events for page `{}`: {}",
                                    page_id, e
                                ),
                            )
                            .await;
                    }
                }
            }
        }
    }

    // Save updated history
    save_event_history(&history)?;

    Ok(new_events_count)
}

async fn post_event_to_discord(
    http: &Http,
    channel_id: ChannelId,
    event: &FacebookEvent,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut embed = CreateEmbed::new()
        .title(&event.name)
        .url(&event.url)
        .color(0x1877f2); // Facebook blue

    // Add description if available
    if let Some(description) = &event.description {
        let truncated_desc = if description.len() > 1024 {
            format!("{}...", &description[..1021])
        } else {
            description.clone()
        };
        embed = embed.description(truncated_desc);
    }

    // Add start time if available
    if let Some(start_time_str) = &event.start_time {
        if let Ok(start_time) = DateTime::parse_from_rfc3339(start_time_str) {
            embed = embed.field(
                "📅 Start Time",
                format!("<t:{}:F>", start_time.timestamp()),
                true,
            );
        } else {
            embed = embed.field("📅 Time", start_time_str, true);
        }
    }

    // Add location if available
    if let Some(location) = &event.location {
        embed = embed.field("📍 Location", location, true);
    }

    // Add footer with page info
    embed = embed.footer(serenity::CreateEmbedFooter::new(format!(
        "New event from facebook.com/{}",
        event.page_name
    )));

    let message = "🎉 **New Facebook Event!**";

    channel_id
        .send_message(
            http,
            serenity::CreateMessage::new().content(message).embed(embed),
        )
        .await?;

    Ok(())
}

pub async fn start_facebook_event_scheduler(http: Arc<Http>) {
    let mut scheduler_guard = SCHEDULER.lock().await;

    if scheduler_guard.is_some() {
        log::info!("Facebook event scheduler is already running");
        return;
    }

    match JobScheduler::new().await {
        Ok(scheduler) => {
            // Check for Facebook events every 2 hours
            let job_http = Arc::clone(&http);
            match Job::new_async("0 0 */2 * * *", move |_uuid, _l| {
                let http_clone = Arc::clone(&job_http);
                Box::pin(async move {
                    log::info!("Running scheduled Facebook events check...");
                    match check_facebook_events(http_clone).await {
                        Ok(count) => {
                            log::info!(
                                "Facebook events check completed. Found {} new events.",
                                count
                            );
                        }
                        Err(e) => {
                            log::error!("Facebook events check failed: {}", e);
                        }
                    }
                })
            }) {
                Ok(job) => {
                    if let Err(e) = scheduler.add(job).await {
                        log::error!("Failed to add Facebook events job to scheduler: {}", e);
                        return;
                    }
                }
                Err(e) => {
                    log::error!("Failed to create Facebook events job: {}", e);
                    return;
                }
            }

            if let Err(e) = scheduler.start().await {
                log::error!("Failed to start Facebook events scheduler: {}", e);
                return;
            }

            *scheduler_guard = Some(scheduler);
            log::info!("Facebook event scheduler started successfully");
        }
        Err(e) => {
            log::error!("Failed to create Facebook events scheduler: {}", e);
        }
    }
}
