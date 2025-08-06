use crate::{Context, Error};
use chrono::{DateTime, Duration, Utc};
use poise::serenity_prelude as serenity;
use serde::{Deserialize, Serialize};
use serenity::{Color, CreateEmbed, CreateEmbedFooter};
use std::fs;
use std::sync::Arc;
use tokio::time::{interval, Duration as TokioDuration};

#[derive(Serialize, Deserialize, Clone)]
struct Reminder {
    id: u64,
    user_id: u64,
    channel_id: u64,
    message: String,
    remind_at: DateTime<Utc>,
    created_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize)]
struct RemindersData {
    reminders: Vec<Reminder>,
    next_id: u64,
}

impl Default for RemindersData {
    fn default() -> Self {
        Self {
            reminders: Vec::new(),
            next_id: 1,
        }
    }
}

const REMINDERS_FILE: &str = "/tmp/rustbot_reminders.json";

fn load_reminders() -> RemindersData {
    match fs::read_to_string(REMINDERS_FILE) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => RemindersData::default(),
    }
}

fn save_reminders(data: &RemindersData) -> Result<(), Error> {
    let json = serde_json::to_string_pretty(data)?;
    fs::write(REMINDERS_FILE, json)?;
    Ok(())
}

fn parse_time_duration(time_str: &str) -> Option<Duration> {
    let time_str = time_str.to_lowercase();

    // Extract number and unit
    let mut number_str = String::new();
    let mut unit_str = String::new();

    for char in time_str.chars() {
        if char.is_ascii_digit() {
            number_str.push(char);
        } else {
            unit_str.push(char);
        }
    }

    let number: i64 = number_str.parse().ok()?;

    match unit_str.trim() {
        "s" | "sec" | "second" | "seconds" => Some(Duration::seconds(number)),
        "m" | "min" | "minute" | "minutes" => Some(Duration::minutes(number)),
        "h" | "hr" | "hour" | "hours" => Some(Duration::hours(number)),
        "d" | "day" | "days" => Some(Duration::days(number)),
        "w" | "week" | "weeks" => Some(Duration::weeks(number)),
        _ => None,
    }
}

/// Reminder commands - set, list, remove reminders
#[poise::command(
    prefix_command,
    slash_command,
    subcommands("remind_set", "remind_list", "remind_remove", "remind_clear")
)]
pub async fn remind(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Set a new reminder
#[poise::command(prefix_command, slash_command, rename = "set")]
pub async fn remind_set(
    ctx: Context<'_>,
    #[description = "Time duration (e.g., 5m, 1h, 2d)"] time: String,
    #[description = "Reminder message"]
    #[rest]
    message: String,
) -> Result<(), Error> {
    log::info!(
        "Remind set command called by {} with time: '{}' and message: '{}'",
        ctx.author().name,
        time,
        message
    );

    if message.trim().is_empty() {
        ctx.say("❌ Please provide a reminder message!").await?;
        return Ok(());
    }

    let duration = match parse_time_duration(&time) {
        Some(d) => d,
        None => {
            ctx.say("❌ Invalid time format! Use formats like: 5m, 1h, 2d, 1w")
                .await?;
            return Ok(());
        }
    };

    let now = Utc::now();
    let remind_at = now + duration;

    // Load existing reminders
    let mut data = load_reminders();

    // Create new reminder
    let reminder = Reminder {
        id: data.next_id,
        user_id: ctx.author().id.get(),
        channel_id: ctx.channel_id().get(),
        message: message.trim().to_string(),
        remind_at,
        created_at: now,
    };

    // Add to list and increment ID
    data.reminders.push(reminder.clone());
    data.next_id += 1;

    // Save to file
    if let Err(e) = save_reminders(&data) {
        ctx.say(format!("❌ Failed to save reminder: {}", e))
            .await?;
        return Ok(());
    }

    // Create confirmation embed
    let embed = CreateEmbed::new()
        .title("⏰ Reminder Set!")
        .description(format!(
            "**Message:** {}\n**Remind at:** <t:{}:F>",
            reminder.message,
            remind_at.timestamp()
        ))
        .color(Color::DARK_GREEN)
        .footer(CreateEmbedFooter::new(format!(
            "Reminder ID: {}",
            reminder.id
        )))
        .timestamp(now);

    ctx.send(poise::CreateReply::default().embed(embed)).await?;

    log::info!(
        "Reminder {} set successfully for user {}",
        reminder.id,
        ctx.author().name
    );
    Ok(())
}

/// List your active reminders
#[poise::command(prefix_command, slash_command, rename = "list")]
pub async fn remind_list(ctx: Context<'_>) -> Result<(), Error> {
    log::info!("Remind list command called by {}", ctx.author().name);

    let data = load_reminders();
    let user_id = ctx.author().id.get();
    let now = Utc::now();

    // Filter reminders for this user that haven't expired yet
    let user_reminders: Vec<&Reminder> = data
        .reminders
        .iter()
        .filter(|r| r.user_id == user_id && r.remind_at > now)
        .collect();

    if user_reminders.is_empty() {
        ctx.say("📭 You have no active reminders!").await?;
        return Ok(());
    }

    let mut description = String::new();
    for reminder in user_reminders.iter().take(10) {
        // Limit to first 10 reminders
        description.push_str(&format!(
            "**ID {}:** {}\n⏰ <t:{}:R>\n\n",
            reminder.id,
            reminder.message,
            reminder.remind_at.timestamp()
        ));
    }

    if user_reminders.len() > 10 {
        description.push_str(&format!("... and {} more", user_reminders.len() - 10));
    }

    let embed = CreateEmbed::new()
        .title("📋 Your Active Reminders")
        .description(description)
        .color(Color::BLUE)
        .footer(CreateEmbedFooter::new(format!(
            "Total active reminders: {}",
            user_reminders.len()
        )))
        .timestamp(now);

    ctx.send(poise::CreateReply::default().embed(embed)).await?;

    log::info!(
        "Listed {} reminders for user {}",
        user_reminders.len(),
        ctx.author().name
    );
    Ok(())
}

/// Remove a specific reminder by ID
#[poise::command(prefix_command, slash_command, rename = "remove")]
pub async fn remind_remove(
    ctx: Context<'_>,
    #[description = "Reminder ID to remove"] id: u64,
) -> Result<(), Error> {
    log::info!(
        "Remind remove command called by {} for ID: {}",
        ctx.author().name,
        id
    );

    let mut data = load_reminders();
    let user_id = ctx.author().id.get();

    // Find the reminder
    let reminder_index = data
        .reminders
        .iter()
        .position(|r| r.id == id && r.user_id == user_id);

    match reminder_index {
        Some(index) => {
            let removed_reminder = data.reminders.remove(index);

            if let Err(e) = save_reminders(&data) {
                ctx.say(format!("❌ Failed to save changes: {}", e)).await?;
                return Ok(());
            }

            let embed = CreateEmbed::new()
                .title("🗑️ Reminder Removed")
                .description(format!("**Removed:** {}", removed_reminder.message))
                .color(Color::DARK_RED)
                .timestamp(Utc::now());

            ctx.send(poise::CreateReply::default().embed(embed)).await?;

            log::info!(
                "Reminder {} removed successfully by user {}",
                id,
                ctx.author().name
            );
        }
        None => {
            ctx.say(
                "❌ Reminder not found! Make sure you own this reminder and the ID is correct.",
            )
            .await?;
        }
    }

    Ok(())
}

/// Clear all your reminders
#[poise::command(prefix_command, slash_command, rename = "clear")]
pub async fn remind_clear(ctx: Context<'_>) -> Result<(), Error> {
    log::info!("Remind clear command called by {}", ctx.author().name);

    let mut data = load_reminders();
    let user_id = ctx.author().id.get();

    let initial_count = data.reminders.len();
    data.reminders.retain(|r| r.user_id != user_id);
    let removed_count = initial_count - data.reminders.len();

    if removed_count == 0 {
        ctx.say("📭 You have no reminders to clear!").await?;
        return Ok(());
    }

    if let Err(e) = save_reminders(&data) {
        ctx.say(format!("❌ Failed to save changes: {}", e)).await?;
        return Ok(());
    }

    let embed = CreateEmbed::new()
        .title("🧹 Reminders Cleared")
        .description(format!("Removed {} reminder(s)", removed_count))
        .color(Color::ORANGE)
        .timestamp(Utc::now());

    ctx.send(poise::CreateReply::default().embed(embed)).await?;

    log::info!(
        "Cleared {} reminders for user {}",
        removed_count,
        ctx.author().name
    );
    Ok(())
}

/// Start the reminder checker background task
pub fn start_reminder_checker(http: Arc<serenity::Http>) {
    tokio::spawn(async move {
        let mut interval = interval(TokioDuration::from_secs(60)); // Check every minute

        loop {
            interval.tick().await;

            if let Err(e) = check_and_send_reminders(&http).await {
                log::error!("Error checking reminders: {}", e);
            }
        }
    });
}

async fn check_and_send_reminders(http: &serenity::Http) -> Result<(), Error> {
    let mut data = load_reminders();
    let now = Utc::now();
    let mut sent_reminders = Vec::new();

    for (i, reminder) in data.reminders.iter().enumerate() {
        if reminder.remind_at <= now {
            // Send the reminder
            let channel_id = serenity::ChannelId::new(reminder.channel_id);
            let user_mention = format!("<@{}>", reminder.user_id);

            let embed = CreateEmbed::new()
                .title("⏰ Reminder!")
                .description(format!("**{}**\n\n{}", reminder.message, user_mention))
                .color(Color::GOLD)
                .footer(CreateEmbedFooter::new(format!(
                    "Set {} ago",
                    format_duration(now - reminder.created_at)
                )))
                .timestamp(now);

            match channel_id
                .send_message(
                    http,
                    serenity::CreateMessage::new()
                        .content(&user_mention)
                        .embed(embed),
                )
                .await
            {
                Ok(_) => {
                    log::info!("Sent reminder {} to user {}", reminder.id, reminder.user_id);
                    sent_reminders.push(i);
                }
                Err(e) => {
                    log::error!("Failed to send reminder {}: {}", reminder.id, e);
                }
            }
        }
    }

    // Remove sent reminders (in reverse order to maintain indices)
    for &index in sent_reminders.iter().rev() {
        data.reminders.remove(index);
    }

    if !sent_reminders.is_empty() {
        save_reminders(&data)?;
    }

    Ok(())
}

fn format_duration(duration: Duration) -> String {
    let total_seconds = duration.num_seconds();

    if total_seconds < 60 {
        format!("{}s", total_seconds)
    } else if total_seconds < 3600 {
        format!("{}m", total_seconds / 60)
    } else if total_seconds < 86400 {
        format!("{}h", total_seconds / 3600)
    } else {
        format!("{}d", total_seconds / 86400)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_time_duration() {
        assert_eq!(parse_time_duration("5m"), Some(Duration::minutes(5)));
        assert_eq!(parse_time_duration("1h"), Some(Duration::hours(1)));
        assert_eq!(parse_time_duration("2d"), Some(Duration::days(2)));
        assert_eq!(parse_time_duration("1w"), Some(Duration::weeks(1)));
        assert_eq!(parse_time_duration("30s"), Some(Duration::seconds(30)));
        assert_eq!(parse_time_duration("invalid"), None);
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(Duration::seconds(30)), "30s");
        assert_eq!(format_duration(Duration::minutes(5)), "5m");
        assert_eq!(format_duration(Duration::hours(2)), "2h");
        assert_eq!(format_duration(Duration::days(1)), "1d");
    }
}
