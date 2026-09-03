use crate::{Context, Error};
use poise::serenity_prelude as serenity;

/// Point people at the website instead of inviting a bot that is being retired
///
/// Previously this produced an OAuth invite URL. The bot is being retired, so it
/// now explains that and links to parkering.smallapp.cc.
///
/// # Usage
/// - `-invite` or `/invite` - Explain the retirement and link the website
#[poise::command(prefix_command, slash_command)]
pub async fn invite(ctx: Context<'_>) -> Result<(), Error> {
    log::info!("Invite command called by {}", ctx.author().name);

    // The bot is being retired, so handing out a link to add it to another
    // server would be setting that server up to lose the feature shortly.
    // Point people at the website instead, which is where parking now lives.
    let embed = serenity::CreateEmbed::new()
        .title("🚧 This bot is being retired")
        .description(
            "Parking has moved to a website, and this bot will be shut down soon, without warning. \
             There is no point adding it to another server.",
        )
        .color(0xC0362C)
        .field(
            "🅿️ Parking",
            "Go to **https://parkering.smallapp.cc** and sign in with your @student.aau.dk email. \
             Save your number plate and phone number, then pick which days and times you want.",
            false,
        )
        .field(
            "❓ Why",
            "The website checks that you are actually a student before it registers anything, \
             it can notify you when your parking succeeds or fails, and it does not depend on Discord.",
            false,
        )
        .footer(serenity::CreateEmbedFooter::new("parkering.smallapp.cc"))
        .timestamp(serenity::Timestamp::now());

    ctx.send(poise::CreateReply::default().embed(embed).ephemeral(true))
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
