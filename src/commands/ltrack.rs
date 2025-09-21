use crate::{Context, Error};
use anyhow::{anyhow, Result as AnyhowResult};
use chrono::{DateTime, Utc};
use governor::{Quota, RateLimiter};
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::Client;
use serde::Deserialize;
use std::num::NonZeroU32;

use std::env;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

// Rate limiter for Riot API (100 requests per 2 minutes)
static RATE_LIMITER: once_cell::sync::Lazy<
    Arc<
        RateLimiter<
            governor::state::NotKeyed,
            governor::state::InMemoryState,
            governor::clock::DefaultClock,
        >,
    >,
> = once_cell::sync::Lazy::new(|| {
    Arc::new(RateLimiter::direct(
        Quota::with_period(Duration::from_secs(120))
            .unwrap()
            .allow_burst(NonZeroU32::new(100).unwrap()),
    ))
});

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct Summoner {
    #[serde(rename = "accountId")]
    account_id: Option<String>,
    #[serde(rename = "profileIconId")]
    profile_icon_id: Option<i32>,
    #[serde(rename = "revisionDate")]
    revision_date: Option<i64>,
    name: Option<String>,
    id: Option<String>,
    puuid: String,
    #[serde(rename = "summonerLevel")]
    summoner_level: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct RiotAccount {
    puuid: String,
    #[serde(rename = "gameName")]
    game_name: String,
    #[serde(rename = "tagLine")]
    tag_line: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct LeagueEntry {
    #[serde(rename = "leagueId")]
    league_id: String,
    #[serde(rename = "summonerId")]
    summoner_id: String,
    #[serde(rename = "summonerName")]
    summoner_name: String,
    #[serde(rename = "queueType")]
    queue_type: String,
    tier: String,
    rank: String,
    #[serde(rename = "leaguePoints")]
    league_points: i32,
    wins: i32,
    losses: i32,
    #[serde(rename = "hotStreak")]
    hot_streak: bool,
    veteran: bool,
    #[serde(rename = "freshBlood")]
    fresh_blood: bool,
    inactive: bool,
}

#[derive(Debug, Deserialize)]
struct MatchInfo {
    #[serde(rename = "gameDuration")]
    game_duration: i64,
    #[serde(rename = "gameEndTimestamp")]
    game_end_timestamp: i64,
    #[serde(rename = "queueId")]
    queue_id: i32,
    participants: Vec<Participant>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct Participant {
    puuid: String,
    win: bool,
    #[serde(rename = "championName")]
    champion_name: String,
    kills: i32,
    deaths: i32,
    assists: i32,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct Match {
    #[serde(rename = "metadata")]
    metadata: MatchMetadata,
    info: MatchInfo,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct MatchMetadata {
    #[serde(rename = "matchId")]
    match_id: String,
    participants: Vec<String>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct PlayPoint {
    cum_hours: f64,
    timestamp: DateTime<Utc>,
    won: bool,
    queue_id: i32,
    lp_estimate: Option<i32>,
    champion: String,
}

#[derive(Clone)]
struct RiotClient {
    http: Client,
    api_key: String,
    platform: String,
    region: String,
}

impl RiotClient {
    fn new(api_key: String, platform: String, region: String) -> Self {
        let http = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap();

        Self {
            http,
            api_key,
            platform,
            region,
        }
    }

    async fn rate_limited_request<T>(&self, url: &str) -> AnyhowResult<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        RATE_LIMITER.until_ready().await;

        log::info!("Making API request to: {}", url);
        log::debug!(
            "Using API key (first 8 chars): {}",
            &self.api_key[..8.min(self.api_key.len())]
        );

        let response = self
            .http
            .get(url)
            .header("X-Riot-Token", &self.api_key)
            .send()
            .await?;

        let status = response.status();
        log::info!("API response status: {}", status);

        if status == 429 {
            // Rate limited, wait and retry
            let retry_after = response
                .headers()
                .get("retry-after")
                .and_then(|h| h.to_str().ok())
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(10);

            log::warn!("Rate limited, waiting {} seconds", retry_after);
            sleep(Duration::from_secs(retry_after)).await;
            return Box::pin(self.rate_limited_request(url)).await;
        }

        if !status.is_success() {
            let error_body = response
                .text()
                .await
                .unwrap_or_else(|_| "Unable to read error body".to_string());
            log::error!(
                "API request failed: {} - {} - Body: {}",
                status,
                url,
                error_body
            );

            if status == 403 {
                return Err(anyhow!(
                    "403 Forbidden - Invalid or expired API key. Please check your RIOT_API_KEY"
                ));
            } else if status == 404 {
                return Err(anyhow!(
                    "404 Not Found - Summoner not found or invalid region"
                ));
            } else {
                return Err(anyhow!("API request failed: {} - {}", status, error_body));
            }
        }

        // Debug: log the response text before trying to parse it
        let response_text = response.text().await?;

        // Only log responses for account and summoner endpoints to avoid spam
        if url.contains("/riot/account/") || url.contains("/lol/summoner/") {
            log::info!("API Response for {}: {}", url, response_text);
        }

        let json = serde_json::from_str::<T>(&response_text).map_err(|e| {
            log::error!("Failed to parse JSON response: {}", e);
            log::error!("Raw response: {}", response_text);
            anyhow!(
                "JSON parsing error: {} - Raw response: {}",
                e,
                response_text
            )
        })?;
        Ok(json)
    }

    async fn get_account_by_riot_id(
        &self,
        game_name: &str,
        tag_line: &str,
    ) -> AnyhowResult<RiotAccount> {
        let url = format!(
            "https://{}.api.riotgames.com/riot/account/v1/accounts/by-riot-id/{}/{}",
            self.region, game_name, tag_line
        );
        self.rate_limited_request(&url).await
    }

    async fn get_summoner_by_puuid(&self, puuid: &str) -> AnyhowResult<Summoner> {
        let url = format!(
            "https://{}.api.riotgames.com/lol/summoner/v4/summoners/by-puuid/{}",
            self.platform, puuid
        );
        self.rate_limited_request(&url).await
    }

    async fn get_league_entries(&self, puuid: &str) -> AnyhowResult<Vec<LeagueEntry>> {
        let url = format!(
            "https://{}.api.riotgames.com/lol/league/v4/entries/by-puuid/{}",
            self.platform, puuid
        );
        self.rate_limited_request(&url).await
    }

    async fn get_match_ids(
        &self,
        puuid: &str,
        start: usize,
        count: usize,
        queue: Option<i32>,
    ) -> AnyhowResult<Vec<String>> {
        let mut url = format!(
            "https://{}.api.riotgames.com/lol/match/v5/matches/by-puuid/{}/ids?start={}&count={}",
            self.region, puuid, start, count
        );

        if let Some(q) = queue {
            url.push_str(&format!("&queue={}", q));
        }

        self.rate_limited_request(&url).await
    }

    async fn get_match_details(&self, match_id: &str) -> AnyhowResult<Match> {
        let url = format!(
            "https://{}.api.riotgames.com/lol/match/v5/matches/{}",
            self.region, match_id
        );
        self.rate_limited_request(&url).await
    }
}

fn estimate_lp(play_points: &mut [PlayPoint], seed_lp: i32) {
    let mut current_lp = seed_lp;

    // Work backwards from current LP
    for point in play_points.iter_mut().rev() {
        point.lp_estimate = Some(current_lp.clamp(0, 100));

        // Estimate LP change for previous game
        if point.won {
            current_lp -= 18; // Assume we gained 18 LP for this win
        } else {
            current_lp += 15; // Assume we lost 15 LP for this loss
        }

        // Keep LP in reasonable bounds
        current_lp = current_lp.clamp(0, 100);
    }
}

fn create_lp_chart(play_points: &[PlayPoint], summoner_name: &str) -> AnyhowResult<String> {
    let max_hours = play_points.last().map(|p| p.cum_hours).unwrap_or(100.0);
    let width = 1200.0;
    let height = 700.0;
    let margin = 60.0;
    let plot_width = width - 2.0 * margin;
    let plot_height = height - 2.0 * margin;

    let mut svg = String::new();
    svg.push_str(&format!(
        r#"<svg width="{}" height="{}" xmlns="http://www.w3.org/2000/svg">
<style>
.axis {{ stroke: #333; stroke-width: 1; }}
.grid {{ stroke: #ddd; stroke-width: 0.5; }}
.lp-line {{ stroke: #2563eb; stroke-width: 2; fill: none; }}
.win-point {{ fill: #10b981; }}
.loss-point {{ fill: #ef4444; }}
.text {{ font-family: Arial, sans-serif; text-anchor: middle; }}
.title {{ font-size: 24px; font-weight: bold; }}
.axis-label {{ font-size: 14px; }}
</style>
"#,
        width, height
    ));

    // Background
    svg.push_str(&format!(
        r#"<rect width="{}" height="{}" fill="white"/>"#,
        width, height
    ));

    // Title
    svg.push_str(&format!(
        r#"<text x="{}" y="30" class="text title">{} - LP vs Playtime (Last ~{:.0}h)</text>"#,
        width / 2.0,
        summoner_name,
        max_hours
    ));

    // Grid lines and axes
    let x_scale = plot_width / max_hours;
    let y_scale = plot_height / 100.0;

    // Y-axis
    svg.push_str(&format!(
        r#"<line x1="{}" y1="{}" x2="{}" y2="{}" class="axis"/>"#,
        margin,
        margin,
        margin,
        margin + plot_height
    ));

    // X-axis
    svg.push_str(&format!(
        r#"<line x1="{}" y1="{}" x2="{}" y2="{}" class="axis"/>"#,
        margin,
        margin + plot_height,
        margin + plot_width,
        margin + plot_height
    ));

    // Y-axis labels (LP)
    for lp in (0..=100).step_by(20) {
        let y = margin + plot_height - (lp as f64 * y_scale);
        svg.push_str(&format!(
            r#"<line x1="{}" y1="{}" x2="{}" y2="{}" class="grid"/>"#,
            margin,
            y,
            margin + plot_width,
            y
        ));
        svg.push_str(&format!(
            r#"<text x="{}" y="{}" class="text axis-label" text-anchor="end">{}</text>"#,
            margin - 10.0,
            y + 5.0,
            lp
        ));
    }

    // X-axis labels (hours)
    let hour_step = (max_hours / 10.0).ceil();
    for i in 0..=10 {
        let hours = i as f64 * hour_step;
        if hours <= max_hours {
            let x = margin + (hours * x_scale);
            svg.push_str(&format!(
                r#"<line x1="{}" y1="{}" x2="{}" y2="{}" class="grid"/>"#,
                x,
                margin,
                x,
                margin + plot_height
            ));
            svg.push_str(&format!(
                r#"<text x="{}" y="{}" class="text axis-label">{:.0}h</text>"#,
                x,
                margin + plot_height + 20.0,
                hours
            ));
        }
    }

    // Axis labels
    svg.push_str(&format!(
        r#"<text x="{}" y="{}" class="text axis-label">Cumulative Playtime (hours)</text>"#,
        width / 2.0,
        height - 10.0
    ));

    // Rotated Y-axis label
    svg.push_str(&format!(
        r#"<text x="20" y="{}" class="text axis-label" transform="rotate(-90, 20, {})">LP</text>"#,
        height / 2.0,
        height / 2.0
    ));

    // LP line
    if !play_points.is_empty() {
        let mut path_data = String::from("M");
        for (i, point) in play_points.iter().enumerate() {
            if let Some(lp) = point.lp_estimate {
                let x = margin + (point.cum_hours * x_scale);
                let y = margin + plot_height - (lp as f64 * y_scale);
                if i == 0 {
                    path_data.push_str(&format!("{:.2},{:.2}", x, y));
                } else {
                    path_data.push_str(&format!(" L{:.2},{:.2}", x, y));
                }
            }
        }
        svg.push_str(&format!(r#"<path d="{}" class="lp-line"/>"#, path_data));

        // Win/loss points
        for point in play_points {
            if let Some(lp) = point.lp_estimate {
                let x = margin + (point.cum_hours * x_scale);
                let y = margin + plot_height - (lp as f64 * y_scale);
                let class = if point.won { "win-point" } else { "loss-point" };
                svg.push_str(&format!(
                    r#"<circle cx="{:.2}" cy="{:.2}" r="3" class="{}"/>"#,
                    x, y, class
                ));
            }
        }
    }

    // Legend
    svg.push_str(&format!(
        "<rect x=\"{}\" y=\"60\" width=\"160\" height=\"80\" fill=\"white\" stroke=\"#ccc\" stroke-width=\"1\"/>",
        width - 200.0
    ));
    svg.push_str(&format!(
        r#"<line x1="{}" y1="80" x2="{}" y2="80" class="lp-line"/>"#,
        width - 190.0,
        width - 160.0
    ));
    svg.push_str(&format!(
        r#"<text x="{}" y="85" class="text axis-label" text-anchor="start">Estimated LP</text>"#,
        width - 155.0
    ));
    svg.push_str(&format!(
        r#"<circle cx="{}" cy="100" r="3" class="win-point"/>"#,
        width - 180.0
    ));
    svg.push_str(&format!(
        r#"<text x="{}" y="105" class="text axis-label" text-anchor="start">Win</text>"#,
        width - 170.0
    ));
    svg.push_str(&format!(
        r#"<circle cx="{}" cy="120" r="3" class="loss-point"/>"#,
        width - 180.0
    ));
    svg.push_str(&format!(
        r#"<text x="{}" y="125" class="text axis-label" text-anchor="start">Loss</text>"#,
        width - 170.0
    ));

    svg.push_str("</svg>");
    Ok(svg)
}

async fn fetch_player_data(
    client: &RiotClient,
    game_name: &str,
    tag_line: &str,
    max_duration_hours: f64,
) -> AnyhowResult<(Summoner, Vec<LeagueEntry>, Vec<PlayPoint>, RiotAccount)> {
    // Get account info using Riot ID
    let account = client.get_account_by_riot_id(game_name, tag_line).await?;

    // Get summoner info using PUUID
    let summoner = client.get_summoner_by_puuid(&account.puuid).await?;

    // Get current rank
    let league_entries = client
        .get_league_entries(&account.puuid)
        .await
        .unwrap_or_else(|e| {
            log::warn!("Failed to get league entries: {}", e);
            Vec::new()
        });

    // Fetch matches until we hit target hours
    let mut all_matches = Vec::new();
    let mut start = 0;
    let mut total_duration_hours = 0.0;

    let progress = ProgressBar::new_spinner();
    progress.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} [{elapsed_precise}] {msg}")
            .unwrap(),
    );

    while total_duration_hours < max_duration_hours && start < 2000 {
        progress.set_message(format!(
            "Fetching matches... {:.1}h/{:.1}h",
            total_duration_hours, max_duration_hours
        ));

        let match_ids = client
            .get_match_ids(&summoner.puuid, start, 100, Some(420)) // Ranked Solo queue
            .await?;

        if match_ids.is_empty() {
            break;
        }

        for match_id in match_ids {
            let match_data = client.get_match_details(&match_id).await?;
            let duration_hours = match_data.info.game_duration as f64 / 3600.0;
            total_duration_hours += duration_hours;
            all_matches.push(match_data);

            if total_duration_hours >= max_duration_hours {
                break;
            }
        }

        start += 100;
    }

    progress.finish_with_message(format!(
        "Fetched {} matches ({:.1}h total)",
        all_matches.len(),
        total_duration_hours
    ));

    // Build play points
    let mut play_points = Vec::new();
    let mut cumulative_hours = 0.0;

    // Sort matches by timestamp
    all_matches.sort_by_key(|m| m.info.game_end_timestamp);

    for match_data in all_matches {
        if let Some(participant) = match_data
            .info
            .participants
            .iter()
            .find(|p| p.puuid == summoner.puuid)
        {
            let duration_hours = match_data.info.game_duration as f64 / 3600.0;
            cumulative_hours += duration_hours;

            let timestamp = DateTime::from_timestamp(match_data.info.game_end_timestamp / 1000, 0)
                .unwrap_or_else(|| Utc::now());

            play_points.push(PlayPoint {
                cum_hours: cumulative_hours,
                timestamp,
                won: participant.win,
                queue_id: match_data.info.queue_id,
                lp_estimate: None,
                champion: participant.champion_name.clone(),
            });
        }
    }

    Ok((summoner, league_entries, play_points, account))
}

/// Track a League of Legends player's LP progression over their last ~100 hours of ranked playtime
///
/// This command fetches a player's recent ranked matches and generates a graph showing
/// their estimated LP progression over cumulative playtime. The LP estimates are based
/// on win/loss patterns and typical LP gains/losses.
///
/// # Usage
/// - `-ltrack SummonerName` - Track player on EUW (default)
/// - `-ltrack SummonerName NA1` - Track player on specified region
///
/// # Features
/// - Fetches ~100 hours of ranked Solo/Duo gameplay
/// - Estimates LP progression based on win/loss patterns
/// - Generates SVG chart showing LP vs playtime
/// - Shows current rank and recent performance stats
///
/// # Note
/// LP values are estimated based on typical gains/losses (+18/-15 LP).
/// Riot API does not provide historical LP after each match.
#[poise::command(prefix_command, slash_command)]
pub async fn ltrack(
    ctx: Context<'_>,
    #[description = "Riot ID (GameName#TagLine) to track"] riot_id: String,
    #[description = "Platform (default: EUW1)"] platform: Option<String>,
) -> Result<(), Error> {
    let api_key = env::var("RIOT_API_KEY")
        .map_err(|_| "❌ RIOT_API_KEY not found in environment variables")?;

    log::info!("ltrack command called for Riot ID: {}", riot_id);
    log::info!("API key present: {} characters", api_key.len());

    // Validate API key format (should be like: RGAPI-xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx)
    if !api_key.starts_with("RGAPI-") || api_key.len() != 42 {
        return Err(format!(
            "❌ Invalid API key format. Expected format: RGAPI-xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx\n\
            Your key: {}... (length: {})\n\
            Get a valid key from: https://developer.riotgames.com/",
            &api_key[..10.min(api_key.len())],
            api_key.len()
        ).into());
    }

    let platform = platform
        .unwrap_or_else(|| "euw1".to_string())
        .to_lowercase();
    let region = match platform.as_str() {
        "na1" => "americas",
        "euw1" | "eune1" => "europe",
        "kr" | "jp1" => "asia",
        _ => "europe", // default
    };

    let client = RiotClient::new(api_key, platform.clone(), region.to_string());

    // Parse Riot ID (GameName#TagLine)
    let parts: Vec<&str> = riot_id.split('#').collect();
    if parts.len() != 2 {
        return Err("❌ Invalid Riot ID format. Use: GameName#TagLine (e.g., Faker#KR1)".into());
    }
    let (game_name, tag_line) = (parts[0], parts[1]);

    // Send initial response
    let reply = ctx
        .say(format!("🔍 Fetching data for **{}**...", riot_id))
        .await?;

    match fetch_player_data(&client, game_name, tag_line, 100.0).await {
        Ok((summoner, league_entries, mut play_points, account)) => {
            if play_points.is_empty() {
                reply
                    .edit(
                        ctx,
                        poise::CreateReply::default()
                            .content("❌ No ranked matches found for this player."),
                    )
                    .await?;
                return Ok(());
            }

            // Get current LP for estimation
            let current_lp = league_entries
                .iter()
                .find(|entry| entry.queue_type == "RANKED_SOLO_5x5")
                .map(|entry| entry.league_points)
                .unwrap_or(50); // Default to 50 LP if no rank found

            // Estimate LP progression
            estimate_lp(&mut play_points, current_lp);

            // Generate chart
            let display_name = summoner.name.as_ref().unwrap_or(&account.game_name);
            match create_lp_chart(&play_points, display_name) {
                Ok(chart_svg) => {
                    // Calculate stats
                    let total_games = play_points.len();
                    let wins = play_points.iter().filter(|p| p.won).count();
                    let win_rate = if total_games > 0 {
                        (wins as f64 / total_games as f64) * 100.0
                    } else {
                        0.0
                    };
                    let total_hours = play_points.last().map(|p| p.cum_hours).unwrap_or(0.0);

                    // Get current rank info
                    let rank_info = league_entries
                        .iter()
                        .find(|entry| entry.queue_type == "RANKED_SOLO_5x5")
                        .map(|entry| {
                            format!("{} {} - {} LP", entry.tier, entry.rank, entry.league_points)
                        })
                        .unwrap_or_else(|| "Unranked".to_string());

                    // Create embed with stats
                    let embed = poise::serenity_prelude::CreateEmbed::new()
                        .title(format!("📈 LP Tracking - {}", display_name))
                        .description(format!(
                            "**Current Rank:** {}\n\
                            **Analyzed Period:** {:.1} hours ({} games)\n\
                            **Win Rate:** {:.1}% ({}/{} games)\n\
                            **Average Game Length:** {:.1} minutes\n\n\
                            *LP estimates based on typical gains/losses (+18/-15). \
                            Historical LP data is not available from Riot API.*",
                            rank_info,
                            total_hours,
                            total_games,
                            win_rate,
                            wins,
                            total_games,
                            (total_hours * 60.0) / total_games as f64
                        ))
                        .color(0x7289DA)
                        .footer(poise::serenity_prelude::CreateEmbedFooter::new(
                            "📊 Chart shows estimated LP progression over cumulative playtime",
                        ));

                    // Send chart as attachment
                    let attachment = poise::serenity_prelude::CreateAttachment::bytes(
                        chart_svg.as_bytes(),
                        format!("{}_lp_tracking.svg", display_name.replace(" ", "_")),
                    );

                    reply
                        .edit(
                            ctx,
                            poise::CreateReply::default()
                                .content("")
                                .embed(embed)
                                .attachment(attachment),
                        )
                        .await?;
                }
                Err(e) => {
                    reply
                        .edit(
                            ctx,
                            poise::CreateReply::default()
                                .content(format!("❌ Failed to generate chart: {}", e)),
                        )
                        .await?;
                }
            }
        }
        Err(e) => {
            log::error!("Error in ltrack command: {}", e);

            let error_msg = if e.to_string().contains("404") || e.to_string().contains("Not Found")
            {
                format!(
                    "❌ Riot ID '{}' not found on {}. Please check the spelling and region.",
                    riot_id,
                    platform.to_uppercase()
                )
            } else if e.to_string().contains("403")
                || e.to_string().contains("Forbidden")
                || e.to_string().contains("Invalid or expired API key")
            {
                format!(
                    "❌ Invalid or expired Riot API key. Please check the RIOT_API_KEY environment variable.\n\
                    Get a new key from: https://developer.riotgames.com/\n\
                    Current key starts with: {}...",
                    &env::var("RIOT_API_KEY").unwrap_or_default()[..8.min(env::var("RIOT_API_KEY").unwrap_or_default().len())]
                )
            } else if e.to_string().contains("JSON parsing error")
                || e.to_string().contains("missing field")
            {
                format!("❌ API response format error: {}\n\nThis might be due to:\n• API response structure changes\n• Invalid API permissions\n• Network issues\n\nCheck the bot logs for the raw API response.", e)
            } else {
                format!("❌ Error fetching player data: {}\n\nPlease check:\n• Your API key is valid\n• The Riot ID format is correct (GameName#TagLine)\n• The region is correct", e)
            };

            reply
                .edit(ctx, poise::CreateReply::default().content(error_msg))
                .await?;
        }
    }

    Ok(())
}
