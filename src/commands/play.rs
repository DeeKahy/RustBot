use crate::{Context, Error};
use lazy_static::lazy_static;
use poise::serenity_prelude as serenity;
use songbird::input::{Compose, YoutubeDl};
use songbird::typemap::TypeMapKey;

lazy_static! {
    // Shared HTTP client used by yt-dlp inputs to stream audio. songbird 0.4
    // expects a reqwest 0.11 `Client`, which is the same version this crate
    // already depends on.
    static ref HTTP_CLIENT: reqwest::Client = reqwest::Client::new();
}

// We stash a human-readable title on each queued track so `-queue` can list it
// without re-invoking yt-dlp.
struct TrackTitleKey;
impl TypeMapKey for TrackTitleKey {
    type Value = String;
}

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

    // Resolving a URL/search via yt-dlp can take a couple of seconds, so let the
    // user know we're working on it.
    ctx.defer_or_broadcast().await.ok();

    // Build the source and resolve its metadata up front. This doubles as
    // validation: a bad URL / no search hit fails here before we join.
    let is_url = query.starts_with("http://") || query.starts_with("https://");
    let mut src = if is_url {
        YoutubeDl::new(HTTP_CLIENT.clone(), query.clone())
    } else {
        YoutubeDl::new_search(HTTP_CLIENT.clone(), query.clone())
    };

    let title = match src.aux_metadata().await {
        Ok(meta) => meta.title.unwrap_or_else(|| query.clone()),
        Err(e) => {
            ctx.say(format!("❌ Couldn't find anything for `{query}`: {e}"))
                .await?;
            return Ok(());
        }
    };

    // Join (or move to) the author's channel.
    let handler_lock = match manager.join(guild_id, connect_to).await {
        Ok(handler) => handler,
        Err(e) => {
            ctx.say(format!("❌ Couldn't join the voice channel: {e}"))
                .await?;
            return Ok(());
        }
    };

    let (position, handle) = {
        let mut handler = handler_lock.lock().await;
        let handle = handler.enqueue_input(src.into()).await;
        (handler.queue().len(), handle)
    };

    handle
        .typemap()
        .write()
        .await
        .insert::<TrackTitleKey>(title.clone());

    if position <= 1 {
        ctx.say(format!("▶️ Now playing: **{title}**")).await?;
    } else {
        ctx.say(format!("➕ Queued **{title}** (position {} in line)", position - 1))
            .await?;
    }

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
        let title = handle
            .typemap()
            .read()
            .await
            .get::<TrackTitleKey>()
            .cloned()
            .unwrap_or_else(|| "unknown track".to_string());
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
