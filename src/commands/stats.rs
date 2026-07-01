use crate::commands::stats_render::{self, BarEntry, Infographic, Slice, AVATAR_D, SLICE_EMOJI};
use crate::{Context, Error};
use chrono::{DateTime, NaiveDate, Timelike, Utc};
use chrono_tz::Europe::Copenhagen;
use image::RgbaImage;
use poise::serenity_prelude as serenity;
use std::collections::HashMap;
use tokio::time::{sleep, Duration};

const MAX_BARS: usize = 10; // rows in the "most active users" bar chart
const MAX_PIE: usize = 8; // named slices before folding into "Others"

/// Shows detailed statistics about message activity in a channel
///
/// Analyzes the last N messages in a channel and renders an infographic (bar
/// chart of the most active users with their avatars, an hourly-activity
/// histogram, and a message-share pie chart) plus a breakdown of channel
/// activity: word/character leaders, top words, and a few fun awards.
///
/// # Usage
/// - `-stats` or `/stats` - Analyze last 1000 messages in current channel
/// - `-stats 2000` - Analyze last 2000 messages in current channel
/// - `-stats 500 #general` - Analyze last 500 messages in #general channel
///
/// # Rate Limiting
/// This command implements proper rate limiting (1 second between API requests) to avoid
/// hitting Discord's API limits. Larger message counts will take longer to process.
///
/// # Limitations
/// - Maximum 10,000 messages can be analyzed at once
/// - Bot messages are excluded from analysis
/// - Requires message history permissions in the target channel
#[poise::command(prefix_command, slash_command)]
pub async fn stats(
    ctx: Context<'_>,
    #[description = "Number of messages to analyze (default: 1000, max: 10000)"] count: Option<u64>,
    #[description = "Channel to analyze (default: current channel)"] channel: Option<
        serenity::GuildChannel,
    >,
) -> Result<(), Error> {
    log::info!("Stats command called by {}", ctx.author().name);

    let message_count = count.unwrap_or(1000).min(10000); // Cap at 10k for safety
    let (target_channel, channel_name) = match &channel {
        Some(ch) => (ch.id, ch.name.clone()),
        None => {
            let channel_id = ctx.channel_id();
            let name = match ctx.guild_channel().await {
                Some(ch) => ch.name,
                None => "Unknown".to_string(),
            };
            (channel_id, name)
        }
    };

    // Send initial message
    let reply = ctx
        .say(format!(
            "Analyzing last {message_count} messages in <#{target_channel}>..."
        ))
        .await?;

    // Collect messages with rate limiting
    let mut all_messages = Vec::new();
    let mut last_message_id = None;
    let mut collected = 0u64;

    while collected < message_count {
        let batch_size = (message_count - collected).min(100); // Discord API limit is 100 per request

        let mut builder = serenity::GetMessages::new().limit(batch_size as u8);
        if let Some(before_id) = last_message_id {
            builder = builder.before(before_id);
        }

        let messages = match target_channel.messages(&ctx.http(), builder).await {
            Ok(msgs) => msgs,
            Err(e) => {
                reply
                    .edit(
                        ctx,
                        poise::CreateReply::default()
                            .content(format!("Error fetching messages: {e}")),
                    )
                    .await?;
                return Ok(());
            }
        };

        if messages.is_empty() {
            break; // No more messages to fetch
        }

        last_message_id = Some(messages.last().unwrap().id);
        collected += messages.len() as u64;
        all_messages.extend(messages);

        // Update progress every 500 messages
        if collected.is_multiple_of(500) || collected >= message_count {
            reply
                .edit(
                    ctx,
                    poise::CreateReply::default()
                        .content(format!("Analyzing messages... {collected}/{message_count}")),
                )
                .await?;
        }

        // Rate limiting - wait 1 second between requests to avoid hitting rate limits
        sleep(Duration::from_millis(1000)).await;
    }

    if all_messages.is_empty() {
        reply
            .edit(
                ctx,
                poise::CreateReply::default().content("No messages found in this channel."),
            )
            .await?;
        return Ok(());
    }

    // Analyze the messages
    let stats = analyze_messages(&all_messages);

    reply
        .edit(
            ctx,
            poise::CreateReply::default().content("Rendering charts..."),
        )
        .await?;

    // Fetch avatars for the top users so they can appear on the bar chart.
    let top_users = stats.top_users(MAX_BARS);
    let avatars = fetch_avatars(&top_users).await;

    let bars: Vec<BarEntry> = top_users
        .iter()
        .zip(avatars.into_iter())
        .enumerate()
        .map(|(i, (u, avatar))| BarEntry {
            label: u.display.clone(),
            value: u.messages,
            avatar,
            color_idx: i,
        })
        .collect();

    let slices = stats.pie_slices(MAX_PIE);

    let info = Infographic {
        channel: &channel_name,
        subtitle: stats.subtitle(all_messages.len()),
        bars,
        slices: slices.iter().map(|s| s.to_render()).collect(),
        hourly: stats.hourly,
        tz_label: "Europe/Copenhagen",
    };

    let chart_png = stats_render::render(&info);

    // Build the embed and reply.
    let embed = create_stats_embed(&stats, &channel_name, all_messages.len(), &slices);
    let mut builder = poise::CreateReply::default().content("");

    let mut embed = embed;
    if let Some(png) = chart_png {
        embed = embed.image("attachment://stats.png");
        builder = builder.attachment(serenity::CreateAttachment::bytes(png, "stats.png"));
    }

    reply.edit(ctx, builder.embed(embed)).await?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Analysis
// ---------------------------------------------------------------------------

#[derive(Default, Clone)]
struct UserAgg {
    display: String,
    avatar_url: String,
    messages: u32,
    words: u32,
    chars: u32,
    night: u32, // messages sent 00:00–05:59 local time
    links: u32,
    questions: u32,
}

struct MessageStats {
    users: HashMap<String, UserAgg>,
    total_messages: u32,
    total_words: u32,
    total_chars: u32,
    total_links: u32,
    total_questions: u32,
    total_attachments: u32,
    hourly: [u32; 24],
    per_day: HashMap<NaiveDate, u32>,
    word_freq: HashMap<String, u32>,
    longest: Option<(u32, String, String)>, // (chars, author, preview)
    first_ts: Option<DateTime<Utc>>,
    last_ts: Option<DateTime<Utc>>,
}

/// A pie slice paired with its emoji index, used for both the image and the
/// text legend in the embed.
struct PieSlice {
    label: String,
    count: u32,
    color_idx: usize,
}

impl PieSlice {
    fn to_render(&self) -> Slice {
        Slice {
            label: self.label.clone(),
            count: self.count,
            color_idx: self.color_idx,
        }
    }
}

const STOPWORDS: &[&str] = &[
    "that", "this", "have", "with", "just", "like", "what", "your", "they", "them", "then",
    "there", "here", "from", "about", "would", "could", "should", "were", "been", "being", "will",
    "yeah", "gonna", "wanna", "dont", "cant", "youre", "thats", "when", "which", "some", "into",
    "than", "also", "very", "much", "even", "more", "want", "know", "think", "really", "still",
    "because", "their", "these", "those", "http", "https", "www", "com",
];

fn analyze_messages(messages: &[serenity::Message]) -> MessageStats {
    let mut users: HashMap<String, UserAgg> = HashMap::new();
    let mut hourly = [0u32; 24];
    let mut per_day: HashMap<NaiveDate, u32> = HashMap::new();
    let mut word_freq: HashMap<String, u32> = HashMap::new();
    let mut total_words = 0u32;
    let mut total_chars = 0u32;
    let mut total_links = 0u32;
    let mut total_questions = 0u32;
    let mut total_attachments = 0u32;
    let mut longest: Option<(u32, String, String)> = None;
    let mut first_ts: Option<DateTime<Utc>> = None;
    let mut last_ts: Option<DateTime<Utc>> = None;

    for message in messages {
        if message.author.bot {
            continue;
        }

        let username = message.author.name.clone();
        let content = &message.content;
        let has_link = content.contains("http://") || content.contains("https://");
        let has_question = content.contains('?');

        // Local time bucketing.
        let utc: DateTime<Utc> = *message.timestamp;
        let local = utc.with_timezone(&Copenhagen);
        let hour = local.hour() as usize;
        hourly[hour] += 1;
        *per_day.entry(local.date_naive()).or_insert(0) += 1;
        first_ts = Some(first_ts.map_or(utc, |t| t.min(utc)));
        last_ts = Some(last_ts.map_or(utc, |t| t.max(utc)));

        let entry = users.entry(username.clone()).or_default();
        if entry.messages == 0 {
            entry.display = username.split('#').next().unwrap_or(&username).to_string();
            entry.avatar_url = message
                .author
                .avatar_url()
                .unwrap_or_else(|| message.author.default_avatar_url());
        }
        entry.messages += 1;

        let words: Vec<&str> = content.split_whitespace().collect();
        let word_count = words.len() as u32;
        entry.words += word_count;
        total_words += word_count;

        let char_count = content.chars().filter(|c| !c.is_whitespace()).count() as u32;
        entry.chars += char_count;
        total_chars += char_count;

        if (0..=5).contains(&hour) {
            entry.night += 1;
        }
        if has_link {
            entry.links += 1;
            total_links += 1;
        }
        if has_question {
            entry.questions += 1;
            total_questions += 1;
        }
        total_attachments += message.attachments.len() as u32;

        // Longest message (by character count).
        if longest
            .as_ref()
            .map(|(c, _, _)| char_count > *c)
            .unwrap_or(true)
            && char_count > 0
        {
            let preview: String = content
                .chars()
                .take(70)
                .collect::<String>()
                .replace('\n', " ");
            longest = Some((char_count, entry.display.clone(), preview));
        }

        // Word frequency (for "top words").
        for raw in &words {
            if raw.starts_with("http") {
                continue;
            }
            let w: String = raw
                .to_lowercase()
                .chars()
                .filter(|c| c.is_alphanumeric())
                .collect();
            if w.chars().count() >= 4 && !STOPWORDS.contains(&w.as_str()) {
                *word_freq.entry(w).or_insert(0) += 1;
            }
        }
    }

    let total_messages = users.values().map(|u| u.messages).sum();

    MessageStats {
        users,
        total_messages,
        total_words,
        total_chars,
        total_links,
        total_questions,
        total_attachments,
        hourly,
        per_day,
        word_freq,
        longest,
        first_ts,
        last_ts,
    }
}

impl MessageStats {
    fn avg_words(&self) -> f32 {
        if self.total_messages > 0 {
            self.total_words as f32 / self.total_messages as f32
        } else {
            0.0
        }
    }

    /// Users sorted by message count, descending (ties broken by name).
    fn ranked(&self) -> Vec<&UserAgg> {
        let mut v: Vec<&UserAgg> = self.users.values().collect();
        v.sort_by(|a, b| {
            b.messages
                .cmp(&a.messages)
                .then_with(|| a.display.cmp(&b.display))
        });
        v
    }

    fn top_users(&self, n: usize) -> Vec<UserAgg> {
        self.ranked().into_iter().take(n).cloned().collect()
    }

    /// Top `named` users as pie slices, remainder folded into an "Others" slice.
    fn pie_slices(&self, named: usize) -> Vec<PieSlice> {
        let ranked = self.ranked();
        let mut slices: Vec<PieSlice> = ranked
            .iter()
            .take(named)
            .enumerate()
            .map(|(i, u)| PieSlice {
                label: u.display.clone(),
                count: u.messages,
                color_idx: i,
            })
            .collect();
        let others: u32 = ranked.iter().skip(named).map(|u| u.messages).sum();
        if others > 0 {
            slices.push(PieSlice {
                label: "Others".to_string(),
                count: others,
                color_idx: stats_render::PALETTE.len() - 1,
            });
        }
        slices
    }

    fn subtitle(&self, analyzed: usize) -> String {
        let range = match (self.first_ts, self.last_ts) {
            (Some(a), Some(b)) => format!(
                " · {} – {}",
                a.with_timezone(&Copenhagen).format("%b %d"),
                b.with_timezone(&Copenhagen).format("%b %d")
            ),
            _ => String::new(),
        };
        format!(
            "{} messages · {} words · {:.1} w/msg avg{}",
            commafy(analyzed as u32),
            commafy(self.total_words),
            self.avg_words(),
            range
        )
    }

    fn peak_hour(&self) -> (usize, u32) {
        self.hourly
            .iter()
            .enumerate()
            .max_by_key(|(_, v)| **v)
            .map(|(h, v)| (h, *v))
            .unwrap_or((0, 0))
    }

    fn busiest_day(&self) -> Option<(NaiveDate, u32)> {
        self.per_day
            .iter()
            .max_by_key(|(_, v)| **v)
            .map(|(d, v)| (*d, *v))
    }

    fn top_words(&self, n: usize) -> Vec<(String, u32)> {
        let mut v: Vec<(String, u32)> = self
            .word_freq
            .iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect();
        v.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        v.into_iter().take(n).collect()
    }
}

fn commafy(n: u32) -> String {
    let s = n.to_string();
    let mut out = String::new();
    let bytes = s.as_bytes();
    for (i, b) in bytes.iter().enumerate() {
        if i > 0 && (bytes.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(*b as char);
    }
    out
}

// ---------------------------------------------------------------------------
// Avatars
// ---------------------------------------------------------------------------

async fn fetch_avatars(users: &[UserAgg]) -> Vec<Option<RgbaImage>> {
    let client = reqwest::Client::new();
    let mut out = Vec::with_capacity(users.len());
    for u in users {
        out.push(fetch_one_avatar(&client, &u.avatar_url).await);
    }
    out
}

async fn fetch_one_avatar(client: &reqwest::Client, url: &str) -> Option<RgbaImage> {
    let bytes = client.get(url).send().await.ok()?.bytes().await.ok()?;
    let img = image::load_from_memory(&bytes).ok()?;
    Some(image::imageops::resize(
        &img.to_rgba8(),
        AVATAR_D,
        AVATAR_D,
        image::imageops::FilterType::Lanczos3,
    ))
}

// ---------------------------------------------------------------------------
// Embed
// ---------------------------------------------------------------------------

fn create_stats_embed(
    stats: &MessageStats,
    channel_name: &str,
    analyzed_count: usize,
    slices: &[PieSlice],
) -> serenity::CreateEmbed {
    let mut embed = serenity::CreateEmbed::new()
        .title(format!("#{channel_name} — activity report"))
        .description(format!(
            "Analysis of **{}** messages\n• {} words · {} characters\n• {:.1} words/msg · {:.1} chars/msg avg",
            commafy(analyzed_count as u32),
            commafy(stats.total_words),
            commafy(stats.total_chars),
            stats.avg_words(),
            if stats.total_messages > 0 {
                stats.total_chars as f32 / stats.total_messages as f32
            } else {
                0.0
            },
        ))
        .color(0x5865F2);

    // Message-share legend (mirrors the pie chart colours).
    let total_msgs: u32 = slices.iter().map(|s| s.count).sum::<u32>().max(1);
    let legend = slices
        .iter()
        .map(|s| {
            format!(
                "{} **{}** — {} ({:.0}%)",
                SLICE_EMOJI[s.color_idx],
                s.label,
                s.count,
                s.count as f32 / total_msgs as f32 * 100.0
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    if !legend.is_empty() {
        embed = embed.field("Message share", legend, true);
    }

    // Word / character leaders.
    embed = embed.field("Most words", top_list(stats, |u| u.words, "words", 5), true);
    embed = embed.field(
        "Most characters",
        top_list(stats, |u| u.chars, "chars", 5),
        true,
    );

    // Yappiest (highest average words per message, min 5 messages).
    let mut yappers: Vec<(&UserAgg, f32)> = stats
        .users
        .values()
        .filter(|u| u.messages >= 5)
        .map(|u| (u, u.words as f32 / u.messages as f32))
        .collect();
    yappers.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    let yap_text = yappers
        .iter()
        .take(5)
        .enumerate()
        .map(|(i, (u, avg))| format!("{}. {} — {:.1} w/msg", i + 1, u.display, avg))
        .collect::<Vec<_>>()
        .join("\n");
    if !yap_text.is_empty() {
        embed = embed.field("Yappiest (5+ msgs)", yap_text, true);
    }

    // Top words.
    let top_words = stats.top_words(10);
    if !top_words.is_empty() {
        let words_text = top_words
            .iter()
            .map(|(w, c)| format!("`{w}` ×{c}"))
            .collect::<Vec<_>>()
            .join("  ");
        embed = embed.field("Top words", words_text, false);
    }

    // Awards.
    let mut awards: Vec<String> = Vec::new();
    if let Some(owl) = stats
        .users
        .values()
        .filter(|u| u.night > 0)
        .max_by_key(|u| u.night)
    {
        awards.push(format!(
            "**Night owl** (00–06h): {} ({} msgs)",
            owl.display, owl.night
        ));
    }
    if let Some(curious) = stats
        .users
        .values()
        .filter(|u| u.questions > 0)
        .max_by_key(|u| u.questions)
    {
        awards.push(format!(
            "**Most inquisitive**: {} ({} questions)",
            curious.display, curious.questions
        ));
    }
    if let Some(linker) = stats
        .users
        .values()
        .filter(|u| u.links > 0)
        .max_by_key(|u| u.links)
    {
        awards.push(format!(
            "**Link lord**: {} ({} links)",
            linker.display, linker.links
        ));
    }
    if !awards.is_empty() {
        embed = embed.field("Awards", awards.join("\n"), false);
    }

    // Highlights.
    let (peak_h, peak_c) = stats.peak_hour();
    let mut highlights = vec![
        format!("Peak hour: **{:02}:00** ({} msgs)", peak_h, peak_c),
        format!(
            "Questions: **{}** · Links: **{}** · Attachments: **{}**",
            commafy(stats.total_questions),
            commafy(stats.total_links),
            commafy(stats.total_attachments)
        ),
    ];
    if let Some((day, cnt)) = stats.busiest_day() {
        highlights.push(format!(
            "Busiest day: **{}** ({} msgs)",
            day.format("%b %d"),
            cnt
        ));
    }
    if let Some((chars, author, preview)) = &stats.longest {
        highlights.push(format!(
            "Longest msg: **{}** ({} chars) — “{}…”",
            author,
            chars,
            preview.trim()
        ));
    }
    embed = embed.field("Highlights", highlights.join("\n"), false);

    embed.footer(serenity::CreateEmbedFooter::new(
        "Bot messages excluded • Times in Europe/Copenhagen • Rate limited for API safety",
    ))
}

/// Build a numbered "top N" list from a per-user metric.
fn top_list(
    stats: &MessageStats,
    metric: impl Fn(&UserAgg) -> u32,
    unit: &str,
    n: usize,
) -> String {
    let mut v: Vec<&UserAgg> = stats.users.values().collect();
    v.sort_by(|a, b| {
        metric(b)
            .cmp(&metric(a))
            .then_with(|| a.display.cmp(&b.display))
    });
    let text = v
        .iter()
        .take(n)
        .enumerate()
        .map(|(i, u)| format!("{}. {} — {} {}", i + 1, u.display, commafy(metric(u)), unit))
        .collect::<Vec<_>>()
        .join("\n");
    if text.is_empty() {
        "—".to_string()
    } else {
        text
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn agg(display: &str, messages: u32) -> UserAgg {
        UserAgg {
            display: display.to_string(),
            messages,
            ..Default::default()
        }
    }

    fn stats_with(users: Vec<UserAgg>) -> MessageStats {
        let mut map = HashMap::new();
        let total: u32 = users.iter().map(|u| u.messages).sum();
        for u in users {
            map.insert(u.display.clone(), u);
        }
        MessageStats {
            users: map,
            total_messages: total,
            total_words: 0,
            total_chars: 0,
            total_links: 0,
            total_questions: 0,
            total_attachments: 0,
            hourly: [0; 24],
            per_day: HashMap::new(),
            word_freq: HashMap::new(),
            longest: None,
            first_ts: None,
            last_ts: None,
        }
    }

    #[test]
    fn test_commafy() {
        assert_eq!(commafy(5), "5");
        assert_eq!(commafy(1000), "1,000");
        assert_eq!(commafy(1234567), "1,234,567");
    }

    #[test]
    fn test_ranked_order() {
        let s = stats_with(vec![agg("a", 3), agg("b", 10), agg("c", 5)]);
        let ranked = s.ranked();
        assert_eq!(ranked[0].display, "b");
        assert_eq!(ranked[1].display, "c");
        assert_eq!(ranked[2].display, "a");
    }

    #[test]
    fn test_pie_slices_folds_others() {
        let users: Vec<UserAgg> = (0..12)
            .map(|i| agg(&format!("u{i}"), (20 - i) as u32))
            .collect();
        let s = stats_with(users);
        let slices = s.pie_slices(8);
        assert_eq!(slices.len(), 9);
        assert_eq!(slices.last().unwrap().label, "Others");
        // Others = remaining 4 users: (20-8)+(20-9)+(20-10)+(20-11) = 12+11+10+9 = 42
        assert_eq!(slices.last().unwrap().count, 42);
    }

    #[test]
    fn test_pie_slices_no_others() {
        let s = stats_with(vec![agg("a", 3), agg("b", 1)]);
        let slices = s.pie_slices(8);
        assert_eq!(slices.len(), 2);
        assert!(slices.iter().all(|x| x.label != "Others"));
    }

    #[test]
    fn test_peak_hour() {
        let mut s = stats_with(vec![agg("a", 1)]);
        s.hourly[13] = 7;
        s.hourly[2] = 3;
        assert_eq!(s.peak_hour(), (13, 7));
    }

    #[test]
    fn test_top_words_excludes_stopwords_and_short() {
        let mut s = stats_with(vec![agg("a", 1)]);
        s.word_freq.insert("rust".to_string(), 5);
        s.word_freq.insert("code".to_string(), 3);
        let tw = s.top_words(10);
        assert_eq!(tw[0], ("rust".to_string(), 5));
        assert_eq!(tw[1], ("code".to_string(), 3));
    }

    #[test]
    fn test_analyze_empty() {
        let stats = analyze_messages(&[]);
        assert_eq!(stats.total_messages, 0);
        assert_eq!(stats.total_words, 0);
        assert!(stats.busiest_day().is_none());
    }
}
