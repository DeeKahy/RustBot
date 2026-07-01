use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;

use crate::{Context, Error};
use lazy_static::lazy_static;
use poise::serenity_prelude as serenity;
use songbird::input::{Compose, YoutubeDl};
use songbird::tracks::Track;
use songbird::{Event, EventContext, EventHandler as VoiceEventHandler, Songbird};

lazy_static! {
    // Shared HTTP client handed to songbird's yt-dlp source (reqwest 0.12, the
    // version songbird speaks).
    static ref HTTP_CLIENT: reqwest::Client = reqwest::Client::new();
}

/// How long the bot may sit in a voice channel with nothing queued before it
/// disconnects on its own.
const IDLE_TIMEOUT: Duration = Duration::from_secs(90);

/// Periodic voice event that leaves the channel once the queue has run dry, so
/// the bot doesn't idle in voice forever after the last track ends.
struct IdleLeaver {
    manager: Arc<Songbird>,
    guild_id: serenity::GuildId,
}

#[async_trait]
impl VoiceEventHandler for IdleLeaver {
    async fn act(&self, _ctx: &EventContext<'_>) -> Option<Event> {
        match self.manager.get(self.guild_id) {
            Some(call) if !call.lock().await.queue().is_empty() => None,
            // Empty queue (or we're no longer connected): disconnect and stop the timer.
            _ => {
                let _ = self.manager.remove(self.guild_id).await;
                Some(Event::Cancel)
            }
        }
    }
}

// We stash a human-readable title on each queued track (as its songbird track
// `data`) so `-queue` can list it without re-invoking yt-dlp.
type TrackTitle = Arc<String>;

/// The voice channel the command author is currently sitting in, if any.
///
/// The guild cache reference isn't `Send`, so we copy the `ChannelId` out inside
/// this scope and never hold the reference across an `.await`.
fn author_voice_channel(ctx: &Context<'_>) -> Option<serenity::ChannelId> {
    let guild = ctx.guild()?;
    guild
        .voice_states
        .get(&ctx.author().id)
        .and_then(|vs| vs.channel_id)
}

/// Leading run of ASCII digits parsed as a u64 (tolerates trailing `?query`/`#frag`).
fn leading_u64(s: &str) -> Option<u64> {
    s.chars()
        .take_while(char::is_ascii_digit)
        .collect::<String>()
        .parse()
        .ok()
}

/// Parse a Discord channel URL such as
/// `https://discord.com/channels/<guild>/<channel>` into its guild + channel IDs.
/// Works for the discord.com / discordapp.com / ptb / canary variants.
fn parse_channel_link(token: &str) -> Option<(serenity::GuildId, serenity::ChannelId)> {
    let rest = token.split("/channels/").nth(1)?;
    let mut segs = rest.split('/');
    let guild = leading_u64(segs.next()?)?;
    let channel = leading_u64(segs.next()?)?;
    Some((
        serenity::GuildId::new(guild),
        serenity::ChannelId::new(channel),
    ))
}

/// Split `-play` input into an optional explicit voice-channel target (from a
/// pasted Discord channel link) and the remaining play query (URL or search terms).
fn split_target(input: &str) -> (Option<(serenity::GuildId, serenity::ChannelId)>, String) {
    let mut target = None;
    let mut rest = Vec::new();
    for tok in input.split_whitespace() {
        if target.is_none() {
            if let Some(t) = parse_channel_link(tok) {
                target = Some(t);
                continue;
            }
        }
        rest.push(tok);
    }
    (target, rest.join(" "))
}

/// Play a YouTube video's audio in a voice channel.
///
/// In a server: `-play <youtube url | search terms>` joins the channel you're in.
/// From a DM (or to target a specific channel): also paste a channel link, e.g.
/// `-play <url> https://discord.com/channels/<server>/<channel>`.
/// If something is already playing, the new track is queued.
#[poise::command(prefix_command, slash_command)]
pub async fn play(
    ctx: Context<'_>,
    #[description = "YouTube URL or search terms (optionally + a Discord channel link)"]
    #[rest]
    input: String,
) -> Result<(), Error> {
    let (target, query) = split_target(input.trim());
    if query.is_empty() {
        ctx.say(
            "❌ Give me a YouTube URL or something to search for: `-play <url or search>`\n\
             From a DM, also paste the voice-channel link: \
             `-play <url> https://discord.com/channels/<server>/<channel>`",
        )
        .await?;
        return Ok(());
    }

    // Pick the target channel: an explicit channel link (works anywhere, incl.
    // DMs), otherwise the channel the author is currently sitting in (server only).
    let (guild_id, connect_to) = match target {
        Some((g, c)) => {
            // Guard against strangers puppeting the bot: the caller must be a
            // member of the server the channel link points to.
            if g.member(ctx.serenity_context(), ctx.author().id)
                .await
                .is_err()
            {
                ctx.say("❌ I couldn't verify you're a member of that server.")
                    .await?;
                return Ok(());
            }
            (g, c)
        }
        None => {
            let g = match ctx.guild_id() {
                Some(g) => g,
                None => {
                    ctx.say(
                        "❌ In a DM I don't know which channel to join — paste a channel link:\n\
                         `-play <url> https://discord.com/channels/<server>/<channel>`",
                    )
                    .await?;
                    return Ok(());
                }
            };
            match author_voice_channel(&ctx) {
                Some(c) => (g, c),
                None => {
                    ctx.say("❌ Join a voice channel first, or paste a channel link to pick one.")
                        .await?;
                    return Ok(());
                }
            }
        }
    };

    let manager = match songbird::get(ctx.serenity_context()).await {
        Some(m) => m.clone(),
        None => {
            ctx.say("❌ Voice support isn't initialised. Ping the bot owner.")
                .await?;
            return Ok(());
        }
    };

    // Send an immediate reply so the user sees something the instant they run the
    // command; we edit this same message as each step completes (or fails).
    let reply = ctx.say("🔊 Joining voice channel…").await?;

    // Join (or move to) the author's channel first, so a connection problem
    // surfaces right away rather than after yt-dlp has run.
    let fresh_join = manager.get(guild_id).is_none();
    let handler_lock = match manager.join(guild_id, connect_to).await {
        Ok(handler) => handler,
        Err(e) => {
            log::error!("voice join failed in guild {guild_id}: {e:?}");
            reply
                .edit(
                    ctx,
                    poise::CreateReply::default().content(format!(
                        "❌ Couldn't connect to the voice channel.\n\
                         ```\n{e:?}\n```\n\
                         (This is usually a network/voice-server issue, not the URL.)"
                    )),
                )
                .await?;
            return Ok(());
        }
    };

    // On the initial join, arm an idle timer so the bot leaves once the queue
    // empties. Only added once per session to avoid stacking timers.
    if fresh_join {
        handler_lock.lock().await.add_global_event(
            Event::Periodic(IDLE_TIMEOUT, None),
            IdleLeaver {
                manager: manager.clone(),
                guild_id,
            },
        );
    }

    reply
        .edit(
            ctx,
            poise::CreateReply::default().content(format!("🔍 Loading `{query}`…")),
        )
        .await?;

    // Resolve the source (runs yt-dlp; also validates the URL / search hit).
    let is_url = query.starts_with("http://") || query.starts_with("https://");
    let mut src = if is_url {
        YoutubeDl::new(HTTP_CLIENT.clone(), query.clone())
    } else {
        YoutubeDl::new_search(HTTP_CLIENT.clone(), query.clone())
    };

    let title = match src.aux_metadata().await {
        Ok(meta) => meta.title.unwrap_or_else(|| query.clone()),
        Err(e) => {
            log::error!("yt-dlp metadata failed for {query:?}: {e:?}");
            reply
                .edit(
                    ctx,
                    poise::CreateReply::default()
                        .content(format!("❌ Couldn't load `{query}`.\n```\n{e:?}\n```")),
                )
                .await?;
            return Ok(());
        }
    };

    // Attach the resolved title as the track's `data` so `-queue` can show it.
    let track_title: TrackTitle = Arc::new(title.clone());
    let track = Track::new_with_data(src.into(), track_title);
    let position = {
        let mut handler = handler_lock.lock().await;
        handler.enqueue(track).await;
        handler.queue().len()
    };

    let content = if position <= 1 {
        format!("▶️ Now playing: **{title}**")
    } else {
        format!("➕ Queued **{title}** (position {} in line)", position - 1)
    };
    reply
        .edit(ctx, poise::CreateReply::default().content(content))
        .await?;

    Ok(())
}

/// Skip the track that's currently playing.
#[poise::command(prefix_command, slash_command, guild_only)]
pub async fn skip(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = match ctx.guild_id() {
        Some(g) => g,
        None => {
            ctx.say("❌ This command only works inside a server.")
                .await?;
            return Ok(());
        }
    };

    let manager = songbird::get(ctx.serenity_context())
        .await
        .ok_or("Voice support not initialised")?
        .clone();

    if let Some(handler_lock) = manager.get(guild_id) {
        let handler = handler_lock.lock().await;
        let queue = handler.queue();
        if queue.is_empty() {
            ctx.say("❌ Nothing is playing.").await?;
        } else {
            let _ = queue.skip();
            ctx.say(format!(
                "⏭️ Skipped. {} left in the queue.",
                queue.len().saturating_sub(1)
            ))
            .await?;
        }
    } else {
        ctx.say("❌ I'm not in a voice channel.").await?;
    }

    Ok(())
}

/// Stop playback and clear the queue (stays in the channel).
#[poise::command(prefix_command, slash_command, guild_only)]
pub async fn stop(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = match ctx.guild_id() {
        Some(g) => g,
        None => {
            ctx.say("❌ This command only works inside a server.")
                .await?;
            return Ok(());
        }
    };

    let manager = songbird::get(ctx.serenity_context())
        .await
        .ok_or("Voice support not initialised")?
        .clone();

    if let Some(handler_lock) = manager.get(guild_id) {
        let handler = handler_lock.lock().await;
        handler.queue().stop();
        ctx.say("⏹️ Stopped and cleared the queue.").await?;
    } else {
        ctx.say("❌ I'm not in a voice channel.").await?;
    }

    Ok(())
}

/// Show what's playing and what's queued up next.
#[poise::command(prefix_command, slash_command, guild_only)]
pub async fn queue(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = match ctx.guild_id() {
        Some(g) => g,
        None => {
            ctx.say("❌ This command only works inside a server.")
                .await?;
            return Ok(());
        }
    };

    let manager = songbird::get(ctx.serenity_context())
        .await
        .ok_or("Voice support not initialised")?
        .clone();

    let tracks = match manager.get(guild_id) {
        Some(handler_lock) => {
            let handler = handler_lock.lock().await;
            handler.queue().current_queue()
        }
        None => {
            ctx.say("❌ I'm not in a voice channel.").await?;
            return Ok(());
        }
    };

    if tracks.is_empty() {
        ctx.say("The queue is empty.").await?;
        return Ok(());
    }

    let mut lines = Vec::new();
    for (i, handle) in tracks.iter().take(10).enumerate() {
        // Every track we enqueue carries its title as `Arc<String>` data.
        let title = handle.data::<String>();
        if i == 0 {
            lines.push(format!("▶️ {title}"));
        } else {
            lines.push(format!("{i}. {title}"));
        }
    }
    if tracks.len() > 10 {
        lines.push(format!("…and {} more", tracks.len() - 10));
    }

    ctx.say(format!("🎶 **Queue**\n{}", lines.join("\n")))
        .await?;
    Ok(())
}

/// Leave the voice channel (also clears the queue).
#[poise::command(prefix_command, slash_command, guild_only, aliases("disconnect", "dc"))]
pub async fn leave(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = match ctx.guild_id() {
        Some(g) => g,
        None => {
            ctx.say("❌ This command only works inside a server.")
                .await?;
            return Ok(());
        }
    };

    let manager = songbird::get(ctx.serenity_context())
        .await
        .ok_or("Voice support not initialised")?
        .clone();

    if manager.get(guild_id).is_some() {
        if let Err(e) = manager.remove(guild_id).await {
            ctx.say(format!("❌ Failed to leave: {e}")).await?;
        } else {
            ctx.say("👋 Left the voice channel.").await?;
        }
    } else {
        ctx.say("❌ I'm not in a voice channel.").await?;
    }

    Ok(())
}
