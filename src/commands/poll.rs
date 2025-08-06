use crate::{Context, Error};
use poise::serenity_prelude as serenity;
use serenity::{Color, CreateEmbed, CreateEmbedFooter, ReactionType};

/// Creates a poll with a question and multiple options
#[poise::command(prefix_command, slash_command)]
pub async fn poll(
    ctx: Context<'_>,
    #[description = "Poll format: 'question? option1 option2 option3'"]
    #[rest]
    input: String,
) -> Result<(), Error> {
    log::info!(
        "Poll command called by {} with input: '{}'",
        ctx.author().name,
        input
    );

    if input.trim().is_empty() {
        ctx.say("❌ Please provide a poll in the format: `question? option1 option2 option3`")
            .await?;
        return Ok(());
    }

    // Split the input at the question mark
    let parts: Vec<&str> = input.splitn(2, '?').collect();

    if parts.len() != 2 {
        ctx.say("❌ Invalid format! Please use: `question? option1 option2 option3`")
            .await?;
        return Ok(());
    }

    let question = parts[0].trim();
    let options_str = parts[1].trim();

    if question.is_empty() {
        ctx.say("❌ Question cannot be empty!").await?;
        return Ok(());
    }

    // Split options by whitespace and filter out empty strings
    let options: Vec<&str> = options_str.split_whitespace().collect();

    if options.is_empty() {
        ctx.say("❌ Please provide at least one option!").await?;
        return Ok(());
    }

    if options.len() > 10 {
        ctx.say("❌ Maximum 10 options allowed!").await?;
        return Ok(());
    }

    // Emoji reactions for options (up to 10)
    let reaction_emojis = ["1️⃣", "2️⃣", "3️⃣", "4️⃣", "5️⃣", "6️⃣", "7️⃣", "8️⃣", "9️⃣", "🔟"];

    // Build the options text with emojis
    let mut options_text = String::new();
    for (i, option) in options.iter().enumerate() {
        if i < reaction_emojis.len() {
            options_text.push_str(&format!("{} {}\n", reaction_emojis[i], option));
        }
    }

    // Create embed
    let embed = CreateEmbed::new()
        .title("📊 Poll")
        .description(format!("**{}**\n\n{}", question, options_text))
        .color(Color::BLUE)
        .footer(CreateEmbedFooter::new(format!(
            "Poll created by {}",
            ctx.author().name
        )))
        .timestamp(chrono::Utc::now());

    // Send the poll message
    let reply = ctx.send(poise::CreateReply::default().embed(embed)).await?;

    // Add reactions for each option
    let message = reply.message().await?;
    for emoji in reaction_emojis
        .iter()
        .take(options.len().min(reaction_emojis.len()))
    {
        if let Err(e) = message
            .react(&ctx.http(), ReactionType::Unicode(emoji.to_string()))
            .await
        {
            log::warn!("Failed to add reaction {}: {}", emoji, e);
        }
    }

    log::info!("Poll created successfully with {} options", options.len());
    Ok(())
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_poll_parsing() {
        let input = "Is this cool? yes no maybe";
        let parts: Vec<&str> = input.splitn(2, '?').collect();

        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0].trim(), "Is this cool");

        let options: Vec<&str> = parts[1].split_whitespace().collect();
        assert_eq!(options, vec!["yes", "no", "maybe"]);
    }

    #[test]
    fn test_poll_no_question_mark() {
        let input = "Is this cool yes no maybe";
        let parts: Vec<&str> = input.splitn(2, '?').collect();

        assert_eq!(parts.len(), 1);
    }

    #[test]
    fn test_poll_empty_options() {
        let input = "Is this cool?";
        let parts: Vec<&str> = input.splitn(2, '?').collect();

        assert_eq!(parts.len(), 2);
        let options: Vec<&str> = parts[1].split_whitespace().collect();
        assert!(options.is_empty());
    }

    #[test]
    fn test_poll_max_options() {
        let input = "Test? 1 2 3 4 5 6 7 8 9 10 11";
        let parts: Vec<&str> = input.splitn(2, '?').collect();
        let options: Vec<&str> = parts[1].split_whitespace().collect();

        assert_eq!(options.len(), 11);
        assert!(options.len() > 10);
    }
}
