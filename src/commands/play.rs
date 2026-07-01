use std::sync::Arc;

use crate::{Context, Error};
use lazy_static::lazy_static;
use poise::serenity_prelude as serenity;
use songbird::input::{Compose, YoutubeDl};
use songbird::tracks::Track;

lazy_static! {
    // Shared HTTP client handed to songbird's yt-dlp source. songbird 0.6 speaks
    // reqwest 0.12, so this uses the aliased `reqwest012` dep (cargo unifies it
    // with songbird's own copy) rather than the crate-wide reqwest 0.11.
    static ref HTTP_CLIENT: reqwest012::Client = reqwest012::Client::new();
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

/// Play a YouTube video's audio in your voice channel.
///
/// Usage: `-play <youtube url | search terms>`. If something is already playing,
/// the new track is added to the queue.
#[poise::command(prefix_command, slash_command, guild_only)]
pub async fn play(
    ctx: Context<'_>,
    #[description = "YouTube URL or search terms"]
    #[rest]
    query: String,
) -> Result<(), Error> {
    let query = query.trim().to_string();
    if query.is_empty() {
        ctx.say("❌ Give me a YouTube URL or something to search for: `-play <url or search>`")
            .await?;
        return Ok(());
    }

    let guild_id = match ctx.guild_id() {
        Some(g) => g,
        None => {
            ctx.say("❌ This command only works inside a server.").await?;
            return Ok(());
        }
    };

    let connect_to = match author_voice_channel(&ctx) {
        Some(c) => c,
        None => {
            ctx.say("❌ You need to be in a voice channel first.").await?;
            return Ok(());
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
                    poise::CreateReply::default().content(format!(
                        "❌ Couldn't load `{query}`.\n```\n{e:?}\n```"
                    )),
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
            ctx.say("❌ This command only works inside a server.").await?;
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
            ctx.say("❌ This command only works inside a server.").await?;
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
            ctx.say("❌ This command only works inside a server.").await?;
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

    ctx.say(format!("🎶 **Queue**\n{}", lines.join("\n"))).await?;
    Ok(())
}

/// Leave the voice channel (also clears the queue).
#[poise::command(
    prefix_command,
    slash_command,
    guild_only,
    aliases("disconnect", "dc")
)]
pub async fn leave(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = match ctx.guild_id() {
        Some(g) => g,
        None => {
            ctx.say("❌ This command only works inside a server.").await?;
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
