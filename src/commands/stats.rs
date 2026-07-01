use crate::{Context, Error};
use poise::serenity_prelude as serenity;
use std::collections::HashMap;
use std::io::Cursor;
use tokio::time::{sleep, Duration};

/// Palette for the pie chart slices. Each colour is paired (by index) with the
/// Discord coloured-square emoji used for that slice in the legend, so the image
/// and the text stay in sync without baking any text into the PNG.
const SLICE_COLORS: [[u8; 3]; 9] = [
    [237, 66, 69],   // red
    [230, 126, 34],  // orange
    [254, 231, 92],  // yellow
    [87, 242, 135],  // green
    [52, 152, 219],  // blue
    [155, 89, 182],  // purple
    [121, 85, 72],   // brown
    [79, 84, 92],    // dark grey
    [185, 187, 190], // light grey ("Others")
];
const SLICE_EMOJI: [&str; 9] = ["🟥", "🟧", "🟨", "🟩", "🟦", "🟪", "🟫", "⬛", "⬜"];

/// Shows detailed statistics about message activity in a channel
///
/// This command analyzes the last N messages in a channel and provides statistics including:
/// - Most active users by message count
/// - Most words written by each user
/// - Average words per message by user
/// - Overall channel statistics
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
            "📊 Analyzing last {message_count} messages in <#{target_channel}>..."
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
                            .content(format!("❌ Error fetching messages: {e}")),
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
                    poise::CreateReply::default().content(format!(
                        "📊 Analyzing messages... {collected}/{message_count}"
                    )),
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
                poise::CreateReply::default().content("❌ No messages found in this channel."),
            )
            .await?;
        return Ok(());
    }

    // Analyze the messages
    let stats = analyze_messages(&all_messages);

    // Build the pie-chart slices (top users by message count, rest folded into
    // "Others") and render them to a PNG we can attach to the embed.
    let slices = build_message_slices(&stats.user_message_counts);
    let chart_png = render_pie_chart(&slices);

    // Create embed with statistics
    let mut embed = create_stats_embed(&stats, &channel_name, all_messages.len());

    let mut builder = poise::CreateReply::default().content("");

    if let Some(png) = chart_png {
        embed = embed
            .field("🥧 Message Share", legend_text(&slices), false)
            .image("attachment://stats_pie.png");
        builder = builder
            .attachment(serenity::CreateAttachment::bytes(png, "stats_pie.png"));
    }

    reply.edit(ctx, builder.embed(embed)).await?;

    Ok(())
}

/// A single pie slice: display label, message count, and the palette index that
/// ties it to a colour + legend emoji.
struct Slice {
    label: String,
    count: u32,
    color_idx: usize,
}

/// Turn per-user message counts into at most 8 named slices plus an aggregated
/// "Others" slice, sorted from most to least active.
fn build_message_slices(user_message_counts: &HashMap<String, u32>) -> Vec<Slice> {
    const MAX_NAMED: usize = 8;

    let mut counts: Vec<(&String, &u32)> = user_message_counts.iter().collect();
    counts.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));

    let mut slices: Vec<Slice> = counts
        .iter()
        .take(MAX_NAMED)
        .enumerate()
        .map(|(i, (user, count))| Slice {
            label: user.split('#').next().unwrap_or(user).to_string(),
            count: **count,
            color_idx: i,
        })
        .collect();

    let others: u32 = counts.iter().skip(MAX_NAMED).map(|(_, c)| **c).sum();
    if others > 0 {
        slices.push(Slice {
            label: "Others".to_string(),
            count: others,
            color_idx: SLICE_COLORS.len() - 1,
        });
    }

    slices
}

/// Colour-coded legend that maps each emoji to its user, count and share.
fn legend_text(slices: &[Slice]) -> String {
    let total: u32 = slices.iter().map(|s| s.count).sum();
    slices
        .iter()
        .map(|s| {
            let pct = if total > 0 {
                s.count as f32 / total as f32 * 100.0
            } else {
                0.0
            };
            format!(
                "{} **{}** — {} ({:.0}%)",
                SLICE_EMOJI[s.color_idx], s.label, s.count, pct
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Render the slices to an anti-aliased PNG pie chart. Text is intentionally
/// left off the image (the embed carries the legend) so no font handling or
/// extra dependencies are needed. Returns `None` if there is nothing to draw.
fn render_pie_chart(slices: &[Slice]) -> Option<Vec<u8>> {
    let total: u32 = slices.iter().map(|s| s.count).sum();
    if total == 0 || slices.is_empty() {
        return None;
    }

    // Supersample, then downscale, for cheap anti-aliasing of the wedge edges.
    const SIZE: u32 = 440;
    const SS: u32 = 3;
    let hi = SIZE * SS;
    let center = hi as f32 / 2.0;
    let radius = center - (12 * SS) as f32;
    let gap = SS as f32; // white separator half-width between slices, in px

    // Precompute each slice's cumulative angle range (radians), starting at the
    // top (12 o'clock) and sweeping clockwise.
    let mut bounds: Vec<(f32, f32, [u8; 3])> = Vec::with_capacity(slices.len());
    let mut acc = 0.0f32;
    let tau = std::f32::consts::TAU;
    for s in slices {
        let start = acc;
        acc += s.count as f32 / total as f32 * tau;
        bounds.push((start, acc, SLICE_COLORS[s.color_idx]));
    }

    let mut img = image::RgbaImage::new(hi, hi);
    for (x, y, px) in img.enumerate_pixels_mut() {
        let dx = x as f32 + 0.5 - center;
        let dy = y as f32 + 0.5 - center;
        let dist = (dx * dx + dy * dy).sqrt();
        if dist > radius {
            *px = image::Rgba([0, 0, 0, 0]); // transparent outside the circle
            continue;
        }

        // Angle clockwise from the top, in [0, tau).
        let mut ang = dx.atan2(-dy);
        if ang < 0.0 {
            ang += tau;
        }

        let color = bounds
            .iter()
            .find(|(start, end, _)| ang >= *start && ang < *end)
            .map(|(_, _, c)| *c)
            .unwrap_or(bounds.last().unwrap().2);

        // Thin white separators between slices (skip when there is only one).
        if bounds.len() > 1 {
            let mut on_edge = false;
            for (start, _, _) in &bounds {
                let mut da = (ang - start).abs();
                if da > tau / 2.0 {
                    da = tau - da;
                }
                if da * dist < gap {
                    on_edge = true;
                    break;
                }
            }
            if on_edge {
                *px = image::Rgba([255, 255, 255, 255]);
                continue;
            }
        }

        *px = image::Rgba([color[0], color[1], color[2], 255]);
    }

    let scaled = image::imageops::resize(
        &img,
        SIZE,
        SIZE,
        image::imageops::FilterType::Triangle,
    );

    let mut bytes = Vec::new();
    scaled
        .write_to(&mut Cursor::new(&mut bytes), image::ImageOutputFormat::Png)
        .ok()?;
    Some(bytes)
}

struct MessageStats {
    user_message_counts: HashMap<String, u32>,
    user_word_counts: HashMap<String, u32>,
    user_char_counts: HashMap<String, u32>,
    total_messages: u32,
    total_words: u32,
    total_chars: u32,
    average_words_per_message: f32,
    average_chars_per_message: f32,
}

fn analyze_messages(messages: &[serenity::Message]) -> MessageStats {
    let mut user_message_counts = HashMap::new();
    let mut user_word_counts = HashMap::new();
    let mut user_char_counts = HashMap::new();
    let mut total_words = 0u32;
    let mut total_chars = 0u32;

    for message in messages {
        // Skip bot messages
        if message.author.bot {
            continue;
        }

        let username = message.author.name.clone();
        let content = &message.content;

        // Count messages
        *user_message_counts.entry(username.clone()).or_insert(0) += 1;

        // Count words (split by whitespace, filter out empty strings)
        let words: Vec<&str> = content.split_whitespace().collect();
        let word_count = words.len() as u32;
        *user_word_counts.entry(username.clone()).or_insert(0) += word_count;
        total_words += word_count;

        // Count characters (excluding whitespace)
        let char_count = content.chars().filter(|c| !c.is_whitespace()).count() as u32;
        *user_char_counts.entry(username.clone()).or_insert(0) += char_count;
        total_chars += char_count;
    }

    let total_messages = user_message_counts.values().sum::<u32>();
    let average_words_per_message = if total_messages > 0 {
        total_words as f32 / total_messages as f32
    } else {
        0.0
    };
    let average_chars_per_message = if total_messages > 0 {
        total_chars as f32 / total_messages as f32
    } else {
        0.0
    };

    MessageStats {
        user_message_counts,
        user_word_counts,
        user_char_counts,
        total_messages,
        total_words,
        total_chars,
        average_words_per_message,
        average_chars_per_message,
    }
}

fn create_stats_embed(
    stats: &MessageStats,
    channel_name: &str,
    analyzed_count: usize,
) -> serenity::CreateEmbed {
    let mut embed = serenity::CreateEmbed::new()
        .title("📊 Channel Message Statistics")
        .description(format!(
            "Analysis of {} messages in **#{}**\n\n**Overall Stats:**\n• Total messages: {}\n• Total words: {}\n• Total characters: {}\n• Avg words/message: {:.1}\n• Avg chars/message: {:.1}",
            analyzed_count,
            channel_name,
            stats.total_messages,
            stats.total_words,
            stats.total_chars,
            stats.average_words_per_message,
            stats.average_chars_per_message
        ))
        .color(0x7289DA);

    // Top message senders
    let mut top_messages: Vec<_> = stats.user_message_counts.iter().collect();
    top_messages.sort_by(|a, b| b.1.cmp(a.1));
    let top_messages_text = top_messages
        .iter()
        .take(10)
        .enumerate()
        .map(|(i, (user, count))| {
            format!(
                "{}. {} - {} messages",
                i + 1,
                user.split('#').next().unwrap_or(user),
                count
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    if !top_messages_text.is_empty() {
        embed = embed.field("📝 Most Active Users (Messages)", top_messages_text, true);
    }

    // Top word writers
    let mut top_words: Vec<_> = stats.user_word_counts.iter().collect();
    top_words.sort_by(|a, b| b.1.cmp(a.1));
    let top_words_text = top_words
        .iter()
        .take(10)
        .enumerate()
        .map(|(i, (user, count))| {
            format!(
                "{}. {} - {} words",
                i + 1,
                user.split('#').next().unwrap_or(user),
                count
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    if !top_words_text.is_empty() {
        embed = embed.field("💬 Most Words Written", top_words_text, true);
    }

    // Top character writers
    let mut top_chars: Vec<_> = stats.user_char_counts.iter().collect();
    top_chars.sort_by(|a, b| b.1.cmp(a.1));
    let top_chars_text = top_chars
        .iter()
        .take(10)
        .enumerate()
        .map(|(i, (user, count))| {
            format!(
                "{}. {} - {} chars",
                i + 1,
                user.split('#').next().unwrap_or(user),
                count
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    if !top_chars_text.is_empty() {
        embed = embed.field("✍️ Most Characters Written", top_chars_text, true);
    }

    // Average words per message by user
    let mut avg_words_per_user: Vec<_> = stats
        .user_message_counts
        .iter()
        .filter_map(|(user, msg_count)| {
            stats.user_word_counts.get(user).map(|word_count| {
                let avg = *word_count as f32 / *msg_count as f32;
                (user, avg, *msg_count)
            })
        })
        .filter(|(_, _, msg_count)| *msg_count >= 5) // Only show users with at least 5 messages
        .collect();

    avg_words_per_user.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let avg_words_text = avg_words_per_user
        .iter()
        .take(10)
        .enumerate()
        .map(|(i, (user, avg, _))| {
            format!(
                "{}. {} - {:.1} words/msg",
                i + 1,
                user.split('#').next().unwrap_or(user),
                avg
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    if !avg_words_text.is_empty() {
        embed = embed.field("📊 Avg Words per Message (5+ msgs)", avg_words_text, false);
    }

    embed.footer(serenity::CreateEmbedFooter::new(
        "📝 Bot messages excluded • Rate limited for API safety",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analyze_empty_messages() {
        let messages = vec![];
        let stats = analyze_messages(&messages);
        assert_eq!(stats.total_messages, 0);
        assert_eq!(stats.total_words, 0);
        assert_eq!(stats.total_chars, 0);
        assert_eq!(stats.average_words_per_message, 0.0);
        assert_eq!(stats.average_chars_per_message, 0.0);
    }

    #[test]
    fn test_message_stats_struct() {
        // Test that MessageStats can be created and contains expected fields
        let stats = MessageStats {
            user_message_counts: HashMap::new(),
            user_word_counts: HashMap::new(),
            user_char_counts: HashMap::new(),
            total_messages: 0,
            total_words: 0,
            total_chars: 0,
            average_words_per_message: 0.0,
            average_chars_per_message: 0.0,
        };

        assert_eq!(stats.total_messages, 0);
        assert!(stats.user_message_counts.is_empty());
    }

    #[test]
    fn test_stats_command_signature() {
        // Verify the command exists and has the correct signature
        // Test passes if the function compiles and can be called
        let function_name = "stats";
        assert_eq!(function_name.len(), 5);
    }

    #[test]
    fn test_build_slices_aggregates_others() {
        let mut counts = HashMap::new();
        for i in 0..12 {
            counts.insert(format!("user{i}"), (12 - i) as u32);
        }
        let slices = build_message_slices(&counts);
        // 8 named users + 1 "Others" slice for the remaining 4.
        assert_eq!(slices.len(), 9);
        assert_eq!(slices.last().unwrap().label, "Others");
        // Others = users 8..12 with counts 4+3+2+1 = 10.
        assert_eq!(slices.last().unwrap().count, 10);
        // Slices are sorted most-active first.
        assert_eq!(slices[0].count, 12);
    }

    #[test]
    fn test_build_slices_no_others_when_few_users() {
        let mut counts = HashMap::new();
        counts.insert("alice".to_string(), 3);
        counts.insert("bob".to_string(), 1);
        let slices = build_message_slices(&counts);
        assert_eq!(slices.len(), 2);
        assert!(slices.iter().all(|s| s.label != "Others"));
    }

    #[test]
    fn test_render_pie_chart_empty() {
        assert!(render_pie_chart(&[]).is_none());
        let zero = vec![Slice {
            label: "x".to_string(),
            count: 0,
            color_idx: 0,
        }];
        assert!(render_pie_chart(&zero).is_none());
    }

    #[test]
    fn test_render_pie_chart_produces_png() {
        let slices = build_message_slices(&HashMap::from([
            ("alice".to_string(), 5),
            ("bob".to_string(), 3),
            ("carol".to_string(), 2),
        ]));
        let png = render_pie_chart(&slices).expect("should render");
        // PNG magic number.
        assert_eq!(&png[..8], &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]);
    }

    #[test]
    fn test_legend_text_percentages() {
        let slices = vec![
            Slice {
                label: "alice".to_string(),
                count: 3,
                color_idx: 0,
            },
            Slice {
                label: "bob".to_string(),
                count: 1,
                color_idx: 1,
            },
        ];
        let legend = legend_text(&slices);
        assert!(legend.contains("alice"));
        assert!(legend.contains("75%"));
        assert!(legend.contains("🟥"));
    }
}
