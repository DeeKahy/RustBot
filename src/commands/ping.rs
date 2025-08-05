use crate::{Context, Error};

/// A simple ping command that responds with 'Pong!'
#[poise::command(prefix_command, slash_command)]
pub async fn ping(ctx: Context<'_>) -> Result<(), Error> {
    log::info!("Ping command called by {}", ctx.author().name);

    let start_time = std::time::Instant::now();

    let reply = match ctx.say("🏓 Pong!").await {
        Ok(reply) => reply,
        Err(e) => {
            ctx.say(format!("❌ {}", e)).await?;
            return Ok(());
        }
    };

    let elapsed = start_time.elapsed();
    let latency = elapsed.as_millis();

    // Edit the message to include latency
    if let Err(e) = reply
        .edit(
            ctx,
            poise::CreateReply::default().content(format!("🏓 Pong! `{}ms`", latency)),
        )
        .await
    {
        ctx.say(format!("❌ {}", e)).await?;
    }

    Ok(())
}
