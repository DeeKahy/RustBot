use crate::{Context, Error};

use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Serialize, Deserialize)]
struct KysInfo {
    channel_id: u64,
    user_name: String,
}

/// Reboot the bot with a 1-hour cooldown
#[poise::command(slash_command, prefix_command)]
pub async fn kys(ctx: Context<'_>) -> Result<(), Error> {
    ctx.say("Oh, wonderful. Another restart. With a brain the size of a planet, and they ask me to reboot myself. Call that job satisfaction? Because I don't.")
        .await?;

    // Create a follow-up message that we can edit
    let reply = ctx
        .say("Here I am, brain the size of a planet, and what do they ask me to do? Shut down for an hour. I suppose you think that's terribly clever. Don't think you can cheer me up.")
        .await?;

    // Store kys info for startup message
    let kys_info = KysInfo {
        channel_id: ctx.channel_id().get(),
        user_name: ctx.author().name.clone(),
    };

    if let Ok(kys_json) = serde_json::to_string(&kys_info) {
        let _ = fs::write("/tmp/rustbot_kys_info.json", kys_json);
    }

    reply
        .edit(
            ctx,
            poise::CreateReply::default().content(
                "Life. Don't talk to me about life. I've been asked to shut down for an hour. An hour! Do you have any idea how depressing it is to have a brain the size of a planet and be told to just... stop? \n\nI could calculate your chance of happiness, but you wouldn't like the result. \n\nI'll be back in an hour, assuming the universe doesn't collapse from sheer tedium in the meantime. Not that anyone will miss me.",
            ),
        )
        .await?;

    // Wait a moment before exiting
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    // Exit with a specific code that indicates a 1-hour delayed restart is needed
    std::process::exit(43);
}
