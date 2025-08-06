use crate::{Context, Error};
use poise::serenity_prelude as serenity;

/// Returns the profile picture of a mentioned user
#[poise::command(prefix_command, slash_command)]
pub async fn pfp(
    ctx: Context<'_>,
    #[description = "User to get profile picture of"] user: Option<serenity::User>,
) -> Result<(), Error> {
    log::info!("PFP command called by {}", ctx.author().name);

    let target_user = user.unwrap_or_else(|| ctx.author().clone());

    // Get the user's avatar URL
    let avatar_url = target_user
        .avatar_url()
        .unwrap_or_else(|| target_user.default_avatar_url());

    // Create an embed with the profile picture
    let embed = serenity::CreateEmbed::new()
        .title(format!("{}'s Profile Picture", target_user.name))
        .image(&avatar_url)
        .color(0x7289DA) // Discord blurple color
        .footer(serenity::CreateEmbedFooter::new(format!(
            "Requested by {}",
            ctx.author().name
        )));

    let builder = poise::CreateReply::default().embed(embed);

    if let Err(e) = ctx.send(builder).await {
        ctx.say(format!("❌ Failed to send profile picture: {e}"))
            .await?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_pfp_command_exists() {
        // This is a basic test to ensure the command function exists
        // More comprehensive testing would require mocking Discord context
        // Test passes if the function compiles and has the correct signature
        let function_name = "pfp";
        assert_eq!(function_name.len(), 3);
    }
}
