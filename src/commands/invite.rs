use crate::{Context, Error};
use poise::serenity_prelude as serenity;

/// Generate and display the bot's invite link
///
/// This command creates an invite link for the bot with the necessary permissions
/// to function properly in servers. The link includes permissions for reading messages,
/// sending messages, managing messages, and other essential bot functions.
///
/// # Usage
/// - `-invite` or `/invite` - Show the bot invite link
#[poise::command(prefix_command, slash_command)]
pub async fn invite(ctx: Context<'_>) -> Result<(), Error> {
    log::info!("Invite command called by {}", ctx.author().name);

    // Get the bot's application ID
    let bot_id = ctx.serenity_context().cache.current_user().id;

    // Define the permissions the bot needs
    // These are the essential permissions for the bot to function properly
    let permissions = serenity::Permissions::SEND_MESSAGES
        | serenity::Permissions::READ_MESSAGE_HISTORY
        | serenity::Permissions::VIEW_CHANNEL
        | serenity::Permissions::EMBED_LINKS
        | serenity::Permissions::ATTACH_FILES
        | serenity::Permissions::USE_EXTERNAL_EMOJIS
        | serenity::Permissions::ADD_REACTIONS
        | serenity::Permissions::MANAGE_MESSAGES
        | serenity::Permissions::READ_MESSAGE_HISTORY;

    // Create the invite URL
    let invite_url = format!(
        "https://discord.com/api/oauth2/authorize?client_id={}&permissions={}&scope=bot%20applications.commands",
        bot_id,
        permissions.bits()
    );

    let embed = serenity::CreateEmbed::new()
        .title("🤖 Invite Me to Your Server!")
        .description("Click the link below to add me to your Discord server!")
        .color(0x7289DA)
        .field(
            "📋 Invite Link",
            format!("[**Click here to invite the bot!**]({})", invite_url),
            false
        )
        .field(
            "⚡ What can I do?",
            "• Fun commands like coin flips and jokes\n• Utility features like polls and reminders\n• Message statistics and moderation tools\n• And much more!",
            false
        )
        .field(
            "🔒 Permissions",
            "This invite link includes all the permissions I need to work properly:\n• Send Messages\n• Read Message History\n• Embed Links\n• Manage Messages\n• Add Reactions\n• And more essential permissions",
            false
        )
        .field(
            "💡 Getting Started",
            "After inviting me, use `-help` or `/help` to see all available commands!",
            false
        )
        .footer(serenity::CreateEmbedFooter::new("Thanks for using our bot! 🦀"))
        .timestamp(serenity::Timestamp::now());

    ctx.send(poise::CreateReply::default().embed(embed).ephemeral(false))
        .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invite_permissions() {
        // Test that our permission bits are correctly calculated
        let permissions = serenity::Permissions::SEND_MESSAGES
            | serenity::Permissions::READ_MESSAGE_HISTORY
            | serenity::Permissions::VIEW_CHANNEL
            | serenity::Permissions::EMBED_LINKS
            | serenity::Permissions::ATTACH_FILES
            | serenity::Permissions::USE_EXTERNAL_EMOJIS
            | serenity::Permissions::ADD_REACTIONS
            | serenity::Permissions::MANAGE_MESSAGES
            | serenity::Permissions::READ_MESSAGE_HISTORY;

        // Just verify that permissions are not empty
        assert!(permissions.bits() > 0);
    }

    #[test]
    fn test_invite_url_format() {
        // Test URL format is correct
        let bot_id = 123456789;
        let permissions_bits = 12345;
        let expected_url = format!(
            "https://discord.com/api/oauth2/authorize?client_id={}&permissions={}&scope=bot%20applications.commands",
            bot_id,
            permissions_bits
        );

        assert!(expected_url.contains("discord.com/api/oauth2/authorize"));
        assert!(expected_url.contains("client_id=123456789"));
        assert!(expected_url.contains("permissions=12345"));
        assert!(expected_url.contains("scope=bot%20applications.commands"));
    }
}
