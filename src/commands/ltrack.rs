use crate::{Context, Error};
use anyhow::{anyhow, Result as AnyhowResult};
use chrono::{DateTime, Utc};
use governor::{Quota, RateLimiter};
use indicatif::{ProgressBar, ProgressStyle};
use plotters::prelude::*;
use plotters_svg::SVGBackend;
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
struct Summoner {
    #[serde(rename = "accountId")]
    account_id: String,
    #[serde(rename = "profileIconId")]
    profile_icon_id: i32,
    #[serde(rename = "revisionDate")]
    revision_date: i64,
    name: String,
    id: String,
    puuid: String,
    #[serde(rename = "summonerLevel")]
    summoner_level: i64,
}

#[derive(Debug, Deserialize)]
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
struct Match {
    #[serde(rename = "metadata")]
    metadata: MatchMetadata,
    info: MatchInfo,
}

#[derive(Debug, Deserialize)]
struct MatchMetadata {
    #[serde(rename = "matchId")]
    match_id: String,
    participants: Vec<String>,
}

#[derive(Debug, Clone)]
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

        let response = self
            .http
            .get(url)
            .header("X-Riot-Token", &self.api_key)
            .send()
            .await?;

        if response.status() == 429 {
            // Rate limited, wait and retry
            let retry_after = response
                .headers()
                .get("retry-after")
                .and_then(|h| h.to_str().ok())
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(10);

            sleep(Duration::from_secs(retry_after)).await;
            return Box::pin(self.rate_limited_request(url)).await;
        }

        if !response.status().is_success() {
            return Err(anyhow!(
                "API request failed: {} - {}",
                response.status(),
                url
            ));
        }

        let json = response.json::<T>().await?;
        Ok(json)
    }

    async fn get_summoner_by_name(&self, name: &str) -> AnyhowResult<Summoner> {
        let url = format!(
            "https://{}.api.riotgames.com/lol/summoner/v4/summoners/by-name/{}",
            self.platform, name
        );
        self.rate_limited_request(&url).await
    }

    async fn get_league_entries(&self, summoner_id: &str) -> AnyhowResult<Vec<LeagueEntry>> {
        let url = format!(
            "https://{}.api.riotgames.com/lol/league/v4/entries/by-summoner/{}",
            self.platform, summoner_id
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

    async fn get_match(&self, match_id: &str) -> AnyhowResult<Match> {
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
    let mut buffer = String::new();
    {
        let backend = SVGBackend::with_string(&mut buffer, (1200, 700));
        let root = backend.into_drawing_area();
        root.fill(&WHITE)?;

        let max_hours = play_points.last().map(|p| p.cum_hours).unwrap_or(100.0);

        let mut chart = ChartBuilder::on(&root)
            .caption(
                &format!(
                    "{} - LP vs Playtime (Last ~{:.0}h)",
                    summoner_name, max_hours
                ),
                ("sans-serif", 40),
            )
            .margin(20)
            .x_label_area_size(60)
            .y_label_area_size(80)
            .build_cartesian_2d(0.0..max_hours, 0i32..100i32)?;

        chart
            .configure_mesh()
            .x_desc("Cumulative Playtime (hours)")
            .y_desc("LP")
            .axis_desc_style(("sans-serif", 20))
            .draw()?;

        // Draw LP line
        let lp_points: Vec<(f64, i32)> = play_points
            .iter()
            .filter_map(|p| p.lp_estimate.map(|lp| (p.cum_hours, lp)))
            .collect();

        if !lp_points.is_empty() {
            chart
                .draw_series(LineSeries::new(lp_points.iter().cloned(), &BLUE))?
                .label("Estimated LP")
                .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 10, y)], &BLUE));
        }

        // Draw win/loss markers
        for point in play_points {
            if let Some(lp) = point.lp_estimate {
                let color = if point.won { &GREEN } else { &RED };
                let marker = Circle::new((point.cum_hours, lp), 2, color.filled());
                chart.draw_series(std::iter::once(marker))?;
            }
        }

        chart.configure_series_labels().draw()?;
        root.present()?;
    }

    Ok(buffer)
}

async fn fetch_player_data(
    client: &RiotClient,
    summoner_name: &str,
    target_hours: f64,
) -> AnyhowResult<(Summoner, Vec<LeagueEntry>, Vec<PlayPoint>)> {
    // Get summoner info
    let summoner = client.get_summoner_by_name(summoner_name).await?;

    // Get current rank
    let league_entries = client.get_league_entries(&summoner.id).await?;

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

    while total_duration_hours < target_hours && start < 2000 {
        progress.set_message(format!(
            "Fetching matches... {:.1}h/{:.1}h",
            total_duration_hours, target_hours
        ));

        let match_ids = client
            .get_match_ids(&summoner.puuid, start, 100, Some(420)) // Ranked Solo queue
            .await?;

        if match_ids.is_empty() {
            break;
        }

        for match_id in match_ids {
            let match_data = client.get_match(&match_id).await?;
            let duration_hours = match_data.info.game_duration as f64 / 3600.0;
            total_duration_hours += duration_hours;
            all_matches.push(match_data);

            if total_duration_hours >= target_hours {
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

    Ok((summoner, league_entries, play_points))
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
    #[description = "Summoner name to track"] summoner_name: String,
    #[description = "Platform (default: EUW1)"] platform: Option<String>,
) -> Result<(), Error> {
    let api_key = env::var("RIOT_API_KEY")
        .map_err(|_| "❌ RIOT_API_KEY not found in environment variables")?;

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

    // Send initial response
    let reply = ctx
        .say(format!("🔍 Fetching data for **{}**...", summoner_name))
        .await?;

    match fetch_player_data(&client, &summoner_name, 100.0).await {
        Ok((summoner, league_entries, mut play_points)) => {
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
            match create_lp_chart(&play_points, &summoner.name) {
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
                        .title(format!("📈 LP Tracking - {}", summoner.name))
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
                        format!("{}_lp_tracking.svg", summoner.name.replace(" ", "_")),
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
            let error_msg = if e.to_string().contains("404") {
                format!(
                    "❌ Summoner '{}' not found on {}. Please check the spelling and region.",
                    summoner_name,
                    platform.to_uppercase()
                )
            } else if e.to_string().contains("403") {
                "❌ Invalid or expired Riot API key. Please check the RIOT_API_KEY environment variable.".to_string()
            } else {
                format!("❌ Error fetching player data: {}", e)
            };

            reply
                .edit(ctx, poise::CreateReply::default().content(error_msg))
                .await?;
        }
    }

    Ok(())
}
