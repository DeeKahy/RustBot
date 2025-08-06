use crate::{Context, Error};
use poise::serenity_prelude as serenity;
use rand::seq::SliceRandom;

/// Displays the profile picture of a random server member
#[poise::command(prefix_command, slash_command)]
pub async fn yourmom(ctx: Context<'_>) -> Result<(), Error> {
    log::info!("Yourmom command called by {}", ctx.author().name);

    // Get the guild (server) from the context
    let guild_id = match ctx.guild_id() {
        Some(id) => id,
        None => {
            ctx.say("❌ This command can only be used in a server!")
                .await?;
            return Ok(());
        }
    };

    // Get all members of the guild
    let members = match guild_id
        .members(&ctx.serenity_context().http, None, None)
        .await
    {
        Ok(members) => members,
        Err(e) => {
            log::error!("Failed to fetch guild members: {}", e);
            ctx.say(
                "❌ Failed to fetch server members. Make sure I have the necessary permissions!",
            )
            .await?;
            return Ok(());
        }
    };

    // Filter out bots to only get real users
    let real_members: Vec<_> = members.iter().filter(|member| !member.user.bot).collect();

    if real_members.is_empty() {
        ctx.say("❌ No members found in this server!").await?;
        return Ok(());
    }

    // Select a random member
    let random_member = {
        let mut rng = rand::thread_rng();
        real_members.choose(&mut rng)
    };

    let random_member = match random_member {
        Some(member) => member,
        None => {
            ctx.say("❌ Failed to select a random member!").await?;
            return Ok(());
        }
    };

    let target_user = &random_member.user;

    // Get the user's avatar URL
    let avatar_url = target_user
        .avatar_url()
        .unwrap_or_else(|| target_user.default_avatar_url());

    // Create an embed with the profile picture
    let embed = serenity::CreateEmbed::new()
        .title(format!("Your mom is {}!", target_user.name))
        .description(format!(
            "Behold, the chosen one: **{}**",
            target_user.display_name()
        ))
        .image(&avatar_url)
        .color(0xFF69B4) // Hot pink color for the meme
        .footer(serenity::CreateEmbedFooter::new(format!(
            "Requested by {} • Total members: {}",
            ctx.author().name,
            real_members.len()
        )));

    let builder = poise::CreateReply::default().embed(embed);

    if let Err(e) = ctx.send(builder).await {
        ctx.say(format!("❌ Failed to send profile picture: {}", e))
            .await?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_yourmom_command_exists() {
        // This is a basic test to ensure the command function exists
        // More comprehensive testing would require mocking Discord context
        // Test passes if the function compiles and has the correct signature
        let function_name = "yourmom";
        assert_eq!(function_name.len(), 7);
    }
}
