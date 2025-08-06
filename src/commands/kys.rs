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
    // Check if the user has admin permissions
    let guild_id = match ctx.guild_id() {
        Some(id) => id,
        None => {
            ctx.say("❌ This command can only be used in servers!")
                .await?;
            return Ok(());
        }
    };

    // Get the member to check their permissions
    let member = guild_id
        .member(&ctx.serenity_context().http, ctx.author().id)
        .await?;

    // Check if the member has administrator permission in their roles
    let has_admin = member.roles.iter().any(|role_id| {
        if let Some(guild) = ctx.guild() {
            if let Some(role) = guild.roles.get(role_id) {
                role.permissions.administrator()
            } else {
                false
            }
        } else {
            false
        }
    });

    if !has_admin {
        ctx.say("❌ You need administrator permissions to use this command!")
            .await?;
        return Ok(());
    }

    ctx.say("🔄 Starting reboot process with 1-hour cooldown...")
        .await?;

    // Create a follow-up message that we can edit
    let reply = ctx
        .say("⏰ Bot will shutdown for 1 hour and then restart automatically...")
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
                "💤 Going to sleep for 1 hour... See you later! 😴\n⏰ Bot will automatically restart in 1 hour.",
            ),
        )
        .await?;

    // Wait a moment before exiting
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    // Exit with a specific code that indicates a 1-hour delayed restart is needed
    std::process::exit(43);
}
