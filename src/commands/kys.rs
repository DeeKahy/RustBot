use crate::utils::{is_protected_user, send_dm_to_deekahy};
use crate::{Context, Error};

use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Serialize, Deserialize)]
struct KysInfo {
    channel_id: u64,
    user_name: String,
    timestamp: u64,
}

/// Reboot the bot with a 1-hour cooldown
#[poise::command(slash_command, prefix_command)]
pub async fn kys(ctx: Context<'_>) -> Result<(), Error> {
    // Check if the user is authorized
    if !is_protected_user(&ctx.author().name) {
        ctx.say("❌ You don't have permission to use this command!")
            .await?;
        return Ok(());
    }

    log::info!("KYS command invoked by user: {}", ctx.author().name);

    ctx.say("Oh, wonderful. Another restart. With a brain the size of a planet, and they ask me to reboot myself. Call that job satisfaction? Because I don't.")
        .await?;

    // Create a follow-up message that we can edit
    let reply = ctx
        .say("Here I am, brain the size of a planet, and what do they ask me to do? Shut down for an hour. I suppose you think that's terribly clever. Don't think you can cheer me up.")
        .await?;

    // Store kys info for startup message with timestamp
    let kys_info = KysInfo {
        channel_id: ctx.channel_id().get(),
        user_name: ctx.author().name.clone(),
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    };

    match serde_json::to_string(&kys_info) {
        Ok(kys_json) => {
            if let Err(e) = fs::write("/tmp/rustbot_kys_info.json", kys_json) {
                log::error!("Failed to write kys info file: {}", e);
                ctx.say("❌ Failed to prepare for restart. Please try again.")
                    .await?;
                return Ok(());
            }
        }
        Err(e) => {
            log::error!("Failed to serialize kys info: {}", e);
            ctx.say("❌ Failed to prepare for restart. Please try again.")
                .await?;
            return Ok(());
        }
    }

    reply
        .edit(
            ctx,
            poise::CreateReply::default().content(
                "Life. Don't talk to me about life. I've been asked to shut down for an hour. An hour! Do you have any idea how depressing it is to have a brain the size of a planet and be told to just... stop? \n\nI could calculate your chance of happiness, but you wouldn't like the result. \n\nI'll be back in an hour, assuming the universe doesn't collapse from sheer tedium in the meantime. Not that anyone will miss me.\n\nhttps://cdn.discordapp.com/attachments/645984611738058756/1402999194259558421/image0.gif",
            ),
        )
        .await?;

    // Send DM to deekahy about shutdown
    if let Err(e) = send_dm_to_deekahy(
        &ctx.serenity_context().http,
        "🔄 Bot is shutting down for 1 hour due to -kys command. I'll be back shortly!",
    )
    .await
    {
        log::warn!("Failed to send shutdown DM to deekahy: {}", e);
    }

    log::info!("KYS command preparing to exit with code 43 for 1-hour delay");

    // Wait a moment before exiting to ensure the message is sent
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    // Exit with a specific code that indicates a 1-hour delayed restart is needed
    log::info!("Exiting with code 43 for 1-hour restart delay");
    std::process::exit(43);
}
