use crate::{Context, Error};
use anyhow::{anyhow, Result as AnyhowResult};
use chrono::{DateTime, Utc};
use governor::{Quota, RateLimiter};
use indicatif::{ProgressBar, ProgressStyle};
use moka::future::Cache;
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

// Cache for individual match data (matches don't change once completed)
static MATCH_CACHE: once_cell::sync::Lazy<Cache<String, Match>> =
    once_cell::sync::Lazy::new(|| {
        Cache::builder()
            .max_capacity(10_000) // Cache up to 10k matches
            .time_to_live(std::time::Duration::from_secs(86400 * 7)) // 7 days
            .build()
    });

// Cache for player data (summoner info, league entries, match lists)
static PLAYER_CACHE: once_cell::sync::Lazy<Cache<String, CachedPlayerData>> =
    once_cell::sync::Lazy::new(|| {
        Cache::builder()
            .max_capacity(1_000) // Cache up to 1k players
            .time_to_live(std::time::Duration::from_secs(3600)) // 1 hour
            .build()
    });

#[derive(Debug, Clone)]
struct CachedPlayerData {
    summoner: Summoner,
    league_entries: Vec<LeagueEntry>,
    account: RiotAccount,
    matches: Vec<Match>,
    last_updated: DateTime<Utc>,
    oldest_match_timestamp: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Clone)]
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

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
struct RiotAccount {
    puuid: String,
    #[serde(rename = "gameName")]
    game_name: String,
    #[serde(rename = "tagLine")]
    tag_line: String,
}

#[derive(Debug, Deserialize, Clone)]
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

#[derive(Debug, Deserialize, Clone)]
struct MatchInfo {
    #[serde(rename = "gameDuration")]
    game_duration: i64,
    #[serde(rename = "gameEndTimestamp")]
    game_end_timestamp: i64,
    #[serde(rename = "queueId")]
    queue_id: i32,
    participants: Vec<Participant>,
}

#[derive(Debug, Deserialize, Clone)]
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

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
struct Match {
    #[serde(rename = "metadata")]
    metadata: MatchMetadata,
    info: MatchInfo,
}

#[derive(Debug, Deserialize, Clone)]
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
) -> AnyhowResult<(Summoner, Vec<LeagueEntry>, Vec<PlayPoint>, RiotAccount, f64)> {
    let cache_key = format!("{}#{}:{}", game_name, tag_line, client.platform);
    let target_cutoff_time =
        Utc::now() - chrono::Duration::milliseconds((max_duration_hours * 3600.0 * 1000.0) as i64);

    // Check if we have cached data
    let cached_data = PLAYER_CACHE.get(&cache_key).await;
    let mut needs_fresh_account_data = false;
    let use_cache: bool;

    // Determine what data we need to fetch
    if let Some(ref cache) = cached_data {
        // Check if cached data is sufficient
        if cache.oldest_match_timestamp <= target_cutoff_time {
            // We have enough old data, just need to check for new matches
            log::info!(
                "Cache hit: Using cached data for {} (last updated: {}, {} matches cached)",
                cache_key,
                cache.last_updated.format("%Y-%m-%d %H:%M:%S"),
                cache.matches.len()
            );
            use_cache = true;
        } else {
            // Need to fetch more historical data
            log::info!(
                "Cache partial: Need more historical data for {} (oldest cached: {}, target cutoff: {})",
                cache_key,
                cache.oldest_match_timestamp.format("%Y-%m-%d %H:%M:%S"),
                target_cutoff_time.format("%Y-%m-%d %H:%M:%S")
            );
            use_cache = false;
        }

        // Check if account data is stale (refresh every 30 minutes for rank updates)
        if cache.last_updated < Utc::now() - chrono::Duration::minutes(30) {
            needs_fresh_account_data = true;
        }
    } else {
        log::info!("Cache miss: No cached data for {}", cache_key);
        needs_fresh_account_data = true;
        use_cache = false;
    }
    // Get fresh account/summoner/rank data if needed
    let (account, summoner, league_entries) = if needs_fresh_account_data {
        let account = client.get_account_by_riot_id(game_name, tag_line).await?;
        let summoner = client.get_summoner_by_puuid(&account.puuid).await?;
        let league_entries = client
            .get_league_entries(&account.puuid)
            .await
            .unwrap_or_else(|e| {
                log::warn!("Failed to get league entries: {}", e);
                Vec::new()
            });
        (account, summoner, league_entries)
    } else if use_cache && cached_data.is_some() {
        let cache = cached_data.as_ref().unwrap();
        (
            cache.account.clone(),
            cache.summoner.clone(),
            cache.league_entries.clone(),
        )
    } else {
        // This shouldn't happen, but fallback to fresh data
        let account = client.get_account_by_riot_id(game_name, tag_line).await?;
        let summoner = client.get_summoner_by_puuid(&account.puuid).await?;
        let league_entries = client
            .get_league_entries(&account.puuid)
            .await
            .unwrap_or_else(|e| {
                log::warn!("Failed to get league entries for {}: {}", cache_key, e);
                Vec::new()
            });
        (account, summoner, league_entries)
    };

    // Get matches (use cache when possible)
    let mut all_matches = if use_cache && cached_data.is_some() {
        // Start with cached matches
        cached_data.unwrap().matches.clone()
    } else {
        Vec::new()
    };

    // Determine if we need to fetch new matches
    let latest_cached_timestamp = all_matches
        .iter()
        .map(|m| m.info.game_end_timestamp)
        .max()
        .unwrap_or(0);

    // Calculate duration from cached matches first
    let cached_duration_hours: f64 = all_matches
        .iter()
        .map(|m| m.info.game_duration as f64 / 3600.0)
        .sum();

    // If we already have enough cached data, don't fetch more
    if cached_duration_hours >= max_duration_hours {
        log::info!(
            "Cache contains sufficient data ({:.1}h >= {:.1}h), skipping fetch",
            cached_duration_hours,
            max_duration_hours
        );
    }

    // Fetch new matches if needed
    let mut start = 0;
    let mut new_matches_count = 0;
    let mut should_fetch_more = cached_duration_hours < max_duration_hours;
    let mut current_duration_hours = cached_duration_hours;

    let progress = ProgressBar::new_spinner();
    progress.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} [{elapsed_precise}] {msg}")
            .unwrap(),
    );

    while should_fetch_more && start < 2000 {
        let progress_msg = if all_matches.is_empty() {
            format!(
                "Fetching match history... ({:.1}h/{:.1}h)",
                current_duration_hours, max_duration_hours
            )
        } else {
            format!(
                "Checking for new matches... ({} cached, {:.1}h/{:.1}h)",
                all_matches.len(),
                current_duration_hours,
                max_duration_hours
            )
        };

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

        let mut found_cached_match = false;
        for match_id in match_ids {
            // Check if we already have this match in cache
            if let Some(cached_match) = MATCH_CACHE.get(&match_id).await {
                if cached_match.info.game_end_timestamp <= latest_cached_timestamp {
                    found_cached_match = true;
                    continue; // Skip matches we already have
                }
                log::debug!("Using cached match: {}", match_id);
                all_matches.push(cached_match.clone());
                new_matches_count += 1;
                current_duration_hours += cached_match.info.game_duration as f64 / 3600.0;

                // Check if we have enough duration
                if current_duration_hours >= max_duration_hours {
                    should_fetch_more = false;
                    break;
                }
            } else {
                // Fetch new match data
                log::debug!("Fetching new match: {}", match_id);
                let match_data = client.get_match_details(&match_id).await?;

                // Cache the match for future use
                MATCH_CACHE
                    .insert(match_id.clone(), match_data.clone())
                    .await;

                if match_data.info.game_end_timestamp <= latest_cached_timestamp {
                    found_cached_match = true;
                } else {
                    all_matches.push(match_data.clone());
                    new_matches_count += 1;
                    current_duration_hours += match_data.info.game_duration as f64 / 3600.0;

                    // Check if we have enough duration
                    if current_duration_hours >= max_duration_hours {
                        should_fetch_more = false;
                        break;
                    }
                }
            }
        }

        // If we've found matches we already had cached, we can stop fetching
        if found_cached_match && new_matches_count > 0 {
            should_fetch_more = false;
            break;
        }

        start += 100;

        // Safety check to prevent infinite loops
        if start > 500 && new_matches_count == 0 {
            log::warn!("Stopping fetch after 500 matches with no new data for safety");
            break;
        }
    }

    // Sort all matches by timestamp
    all_matches.sort_by_key(|m| m.info.game_end_timestamp);

    // Filter matches to only include those within our time window and calculate duration
    let mut filtered_matches = Vec::new();
    let mut total_duration_hours = 0.0;

    // Start from the most recent matches and work backwards
    for match_data in all_matches.iter().rev() {
        let duration_hours = match_data.info.game_duration as f64 / 3600.0;
        if total_duration_hours + duration_hours <= max_duration_hours {
            total_duration_hours += duration_hours;
            filtered_matches.push(match_data.clone());
        } else {
            break;
        }
    }

    // Reverse to get chronological order again
    filtered_matches.reverse();

    progress.finish_with_message(format!(
        "Using {} matches ({} new, {:.1}h total)",
        filtered_matches.len(),
        new_matches_count,
        total_duration_hours
    ));

    let cache_hit_rate = if start > 0 {
        ((start - new_matches_count) as f64 / start as f64) * 100.0
    } else {
        100.0
    };

    log::info!(
        "Match processing complete for {}: {} total matches, {} new fetches, {:.1}% cache efficiency",
        cache_key,
        filtered_matches.len(),
        new_matches_count,
        cache_hit_rate
    );

    // Update cache with new data
    if !filtered_matches.is_empty() {
        let oldest_timestamp = DateTime::from_timestamp(
            filtered_matches.first().unwrap().info.game_end_timestamp / 1000,
            0,
        )
        .unwrap_or_else(|| Utc::now());

        let cached_player_data = CachedPlayerData {
            summoner: summoner.clone(),
            league_entries: league_entries.clone(),
            account: account.clone(),
            matches: all_matches, // Store all matches, not just filtered ones
            last_updated: Utc::now(),
            oldest_match_timestamp: oldest_timestamp,
        };

        PLAYER_CACHE
            .insert(cache_key.clone(), cached_player_data)
            .await;
        log::info!("Updated cache for player: {}", cache_key);
    }

    // Build play points from filtered matches
    let mut play_points = Vec::new();
    let mut cumulative_hours = 0.0;

    for match_data in filtered_matches {
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

    Ok((
        summoner,
        league_entries,
        play_points,
        account,
        cache_hit_rate,
    ))
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
        Ok((summoner, league_entries, play_points, account, cache_hit_rate)) => {
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
                    "Summoner Level {} • Platform: {} • Cache Efficiency: {:.1}%",
                    summoner.summoner_level.unwrap_or(0),
                    platform.to_uppercase(),
                    cache_hit_rate
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

/// View cache statistics and manage the ltrack cache
///
/// This command allows you to see how the caching system is performing
/// and optionally clear cached data.
///
/// # Usage
/// - `-cache_stats` - View current cache statistics
/// - `-cache_stats clear` - Clear all cached data
///
/// # Features
/// - Shows match cache hit counts and memory usage
/// - Shows player cache statistics
/// - Allows clearing cache to force fresh data fetching
#[poise::command(prefix_command, slash_command, owners_only)]
pub async fn cache_stats(
    ctx: Context<'_>,
    #[description = "Action to perform (clear to clear cache)"] action: Option<String>,
) -> Result<(), Error> {
    match action.as_deref() {
        Some("clear") => {
            // Clear both caches
            MATCH_CACHE.invalidate_all();
            PLAYER_CACHE.invalidate_all();

            // Wait for invalidation to complete
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;

            ctx.say("🗑️ **Cache Cleared**\nAll cached match and player data has been cleared. Next ltrack commands will fetch fresh data from Riot API.").await?;
            log::info!("Cache cleared by user command");
        }
        _ => {
            // Show cache statistics
            let match_cache_size = MATCH_CACHE.entry_count();
            let match_cache_weight = MATCH_CACHE.weighted_size();
            let player_cache_size = PLAYER_CACHE.entry_count();
            let player_cache_weight = PLAYER_CACHE.weighted_size();

            // Run sync to get accurate stats
            MATCH_CACHE.run_pending_tasks().await;
            PLAYER_CACHE.run_pending_tasks().await;

            let embed = poise::serenity_prelude::CreateEmbed::new()
                .title("📊 Cache Statistics")
                .color(0x00ff00)
                .field(
                    "🎮 Match Cache",
                    format!(
                        "**Entries:** {}\n**Memory Weight:** {}\n**Max Capacity:** 10,000\n**TTL:** 7 days",
                        match_cache_size,
                        match_cache_weight
                    ),
                    true,
                )
                .field(
                    "👤 Player Cache",
                    format!(
                        "**Entries:** {}\n**Memory Weight:** {}\n**Max Capacity:** 1,000\n**TTL:** 1 hour",
                        player_cache_size,
                        player_cache_weight
                    ),
                    true,
                )
                .field(
                    "💡 Cache Benefits",
                    "• Reduces API calls to Riot\n• Faster response times\n• Efficient for repeated queries\n• Automatic expiration",
                    false,
                )
                .footer(poise::serenity_prelude::CreateEmbedFooter::new(
                    "Use `-cache_stats clear` to clear all cached data"
                ))
                .timestamp(chrono::Utc::now());

            ctx.send(poise::CreateReply::default().embed(embed)).await?;

            log::info!(
                "Cache stats requested - Match cache: {} entries, Player cache: {} entries",
                match_cache_size,
                player_cache_size
            );
        }
    }

    Ok(())
}
