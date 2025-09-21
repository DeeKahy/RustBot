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
        Quota::with_period(Duration::from_secs(60))
            .unwrap()
            .allow_burst(NonZeroU32::new(200).unwrap()),
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
    game_duration: i64,
    kills: i32,
    deaths: i32,
    assists: i32,
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

fn format_duration(seconds: i64) -> String {
    let minutes = seconds / 60;
    if minutes < 60 {
        format!("{}m", minutes)
    } else {
        let hours = minutes / 60;
        let remaining_minutes = minutes % 60;
        format!("{}h {}m", hours, remaining_minutes)
    }
}

fn get_rank_display(league_entries: &[LeagueEntry]) -> String {
    if let Some(ranked_entry) = league_entries
        .iter()
        .find(|entry| entry.queue_type == "RANKED_SOLO_5x5")
    {
        format!(
            "{} {} - {} LP",
            ranked_entry.tier, ranked_entry.rank, ranked_entry.league_points
        )
    } else {
        "Unranked".to_string()
    }
}

fn calculate_performance_stats(play_points: &[PlayPoint]) -> (f64, f64, i64, String, String) {
    let total_games = play_points.len();
    let wins = play_points.iter().filter(|p| p.won).count();
    let win_rate = if total_games > 0 {
        (wins as f64 / total_games as f64) * 100.0
    } else {
        0.0
    };

    let total_duration: i64 = play_points.iter().map(|p| p.game_duration).sum();
    let avg_duration = if total_games > 0 {
        total_duration / total_games as i64
    } else {
        0
    };

    let total_hours = play_points.last().map(|p| p.cum_hours).unwrap_or(0.0);

    // Recent performance (last 10 games)
    let recent_games = play_points.iter().rev().take(10).collect::<Vec<_>>();
    let recent_wins = recent_games.iter().filter(|p| p.won).count();
    let recent_performance = if recent_games.len() > 0 {
        format!(
            "{}/{} ({}%)",
            recent_wins,
            recent_games.len(),
            ((recent_wins as f64 / recent_games.len() as f64) * 100.0) as i32
        )
    } else {
        "No recent games".to_string()
    };

    // Streak calculation
    let mut current_streak = 0;
    let mut streak_type = "None";
    for point in play_points.iter().rev() {
        if current_streak == 0 {
            current_streak = 1;
            streak_type = if point.won { "Win" } else { "Loss" };
        } else if (point.won && streak_type == "Win") || (!point.won && streak_type == "Loss") {
            current_streak += 1;
        } else {
            break;
        }
    }
    let streak_display = if current_streak > 0 {
        format!("{} {} streak", current_streak, streak_type)
    } else {
        "No streak".to_string()
    };

    (
        win_rate,
        total_hours,
        avg_duration,
        recent_performance,
        streak_display,
    )
}

async fn fetch_player_data(
    client: &RiotClient,
    game_name: &str,
    tag_line: &str,
    max_duration_hours: f64,
    ctx: &Context<'_>,
    reply: &poise::ReplyHandle<'_>,
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
        let progress_msg = format!(
            "Fetching matches... {:.1}h/{:.1}h",
            total_duration_hours, max_duration_hours
        );
        progress.set_message(progress_msg.clone());

        // Update Discord message
        let _ = reply
            .edit(
                *ctx,
                poise::CreateReply::default()
                    .content(format!("🔍 **{}** - {}", game_name, progress_msg)),
            )
            .await;

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
                game_duration: match_data.info.game_duration,
                kills: participant.kills,
                deaths: participant.deaths,
                assists: participant.assists,
            });
        }
    }

    Ok((summoner, league_entries, play_points, account))
}

/// Analyze a League of Legends player's ranked performance with detailed statistics
///
/// This command fetches a player's recent ranked matches and provides comprehensive
/// statistics including win rates, champion performance, KDA, damage stats, and more.
///
/// # Usage
/// - `-ltrack Faker#KR1` - Analyze player with 20 hours (default)
/// - `-ltrack Faker#KR1 EUW1` - Analyze player on specified region
/// - `-ltrack Faker#KR1 EUW1 50` - Analyze 50 hours of match history
/// - `-ltrack Player#TAG NA1 10` - Quick 10-hour analysis
///
/// # Features
/// - Comprehensive ranked statistics (1-200 hours of gameplay)
/// - Current rank and LP information
/// - Win/loss streaks and recent performance
/// - Champion performance breakdown
/// - KDA and damage statistics
/// - Performance trends and analysis
///
/// # Note
/// LP values are estimated based on typical gains/losses (+18/-15 LP).
/// Riot API does not provide historical LP after each match.
#[poise::command(prefix_command, slash_command)]
pub async fn ltrack(
    ctx: Context<'_>,
    #[description = "Riot ID (GameName#TagLine) to track"] riot_id: String,
    #[description = "Platform (default: EUW1)"] platform: Option<String>,
    #[description = "Hours of match history to analyze (default: 20)"] hours: Option<f64>,
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
    let target_hours = hours.unwrap_or(20.0).max(1.0).min(200.0); // Clamp between 1 and 200 hours

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

    match fetch_player_data(&client, game_name, tag_line, target_hours, &ctx, &reply).await {
        Ok((summoner, league_entries, play_points, account)) => {
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

            let display_name = summoner.name.as_ref().unwrap_or(&account.game_name);
            let current_rank = get_rank_display(&league_entries);
            let (win_rate, total_hours, avg_duration, recent_performance, streak_display) =
                calculate_performance_stats(&play_points);

            // Calculate detailed stats
            let total_games = play_points.len();
            let wins = play_points.iter().filter(|p| p.won).count();
            let losses = total_games - wins;

            // Champion analysis
            let mut champion_stats: std::collections::HashMap<&str, (i32, i32)> =
                std::collections::HashMap::new();
            for point in &play_points {
                let entry = champion_stats.entry(&point.champion).or_insert((0, 0));
                if point.won {
                    entry.0 += 1;
                } else {
                    entry.1 += 1;
                }
            }

            let most_played = champion_stats
                .iter()
                .max_by_key(|(_, (w, l))| w + l)
                .map(|(champ, (w, l))| format!("{} ({}/{})", champ, w, l))
                .unwrap_or("None".to_string());

            let best_performer = champion_stats
                .iter()
                .filter(|(_, (w, l))| w + l >= 3) // At least 3 games
                .max_by(|(_, (w1, l1)), (_, (w2, l2))| {
                    let wr1 = *w1 as f64 / (*w1 + *l1) as f64;
                    let wr2 = *w2 as f64 / (*w2 + *l2) as f64;
                    wr1.partial_cmp(&wr2).unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|(champ, (w, l))| {
                    let wr = (*w as f64 / (*w + *l) as f64) * 100.0;
                    format!("{} ({:.0}% over {} games)", champ, wr, w + l)
                })
                .unwrap_or("Not enough data".to_string());

            // Performance metrics
            let total_kills: i32 = play_points.iter().map(|p| p.kills).sum();
            let total_deaths: i32 = play_points.iter().map(|p| p.deaths).sum();
            let total_assists: i32 = play_points.iter().map(|p| p.assists).sum();

            let avg_kda = if total_deaths > 0 {
                format!(
                    "{:.1}",
                    (total_kills + total_assists) as f64 / total_deaths as f64
                )
            } else {
                "Perfect".to_string()
            };

            let avg_kills = total_kills as f64 / total_games as f64;
            let avg_deaths = total_deaths as f64 / total_games as f64;
            let avg_assists = total_assists as f64 / total_games as f64;

            // Create comprehensive embed
            let embed = poise::serenity_prelude::CreateEmbed::new()
                .title(format!("📊 {} - Ranked Analysis", display_name))
                .color(0x3498db)
                .field("🏆 Current Rank", current_rank, true)
                .field(
                    "📈 Win Rate",
                    format!("{:.1}% ({}/{})", win_rate, wins, losses),
                    true,
                )
                .field("🔥 Current Streak", streak_display, true)
                .field(
                    "⏱️ Time Analyzed",
                    format!("{:.1} hours", total_hours),
                    true,
                )
                .field("🎮 Games Played", total_games.to_string(), true)
                .field("⏰ Avg Game Length", format_duration(avg_duration), true)
                .field("📊 Recent Form (Last 10)", recent_performance, true)
                .field(
                    "🎯 Average KDA",
                    format!(
                        "{:.1}/{:.1}/{:.1} ({} KDA)",
                        avg_kills, avg_deaths, avg_assists, avg_kda
                    ),
                    true,
                )
                .field("🔄 Most Played", most_played, true)
                .field("⭐ Best Performer", best_performer, false)
                .footer(poise::serenity_prelude::CreateEmbedFooter::new(format!(
                    "Summoner Level {} • Platform: {}",
                    summoner.summoner_level.unwrap_or(0),
                    platform.to_uppercase()
                )))
                .timestamp(chrono::Utc::now());

            reply
                .edit(ctx, poise::CreateReply::default().embed(embed))
                .await?;
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
