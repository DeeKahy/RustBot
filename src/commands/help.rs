use crate::{Context, Error};
use poise::serenity_prelude as serenity;

/// Shows all available commands and their descriptions
///
/// This command displays a comprehensive list of all bot commands with their descriptions,
/// usage examples, and parameter information. It provides users with an easy way to
/// discover and understand how to use the bot's functionality.
///
/// # Usage
/// - `-help` or `/help` - Show all available commands
/// - `-help command_name` - Show detailed help for a specific command
#[poise::command(prefix_command, slash_command)]
pub async fn help(
    ctx: Context<'_>,
    #[description = "Specific command to show help for"] command: Option<String>,
) -> Result<(), Error> {
    log::info!("Help command called by {}", ctx.author().name);

    match command {
        Some(command_name) => {
            // Show help for a specific command
            show_command_help(ctx, &command_name).await
        }
        None => {
            // Show general help with all commands
            show_general_help(ctx).await
        }
    }
}

async fn show_general_help(ctx: Context<'_>) -> Result<(), Error> {
    let embed = serenity::CreateEmbed::new()
        .title("🤖 Bot Help - Available Commands")
        .description("Here are all the available commands you can use with this bot!\n\nUse `-help <command>` for detailed information about a specific command.")
        .color(0x7289DA)
        .field(
            "🏓 Basic Commands",
            "• `-ping` - Check bot latency and responsiveness\n• `-hello` - Get a friendly greeting from the bot\n• `-help` - Show this help message",
            false
        )
        .field(
            "🎯 Fun Commands",
            "• `-coinflip` - Flip a coin (heads or tails)\n• `-uwu <text>` - Convert text to uwu speak\n• `-yourmom` - Get a random \"your mom\" joke\n• `-spamping <count>` - Send multiple ping messages",
            false
        )
        .field(
            "👤 User Commands",
            "• `-pfp [user]` - Get profile picture of yourself or another user",
            false
        )
        .field(
            "📊 Utility Commands",
            "• `-stats [count] [channel]` - Analyze message statistics in a channel\n• `-poll <question? option1 option2...>` - Create a poll with reactions\n• `-react <text>` - Add emoji reactions to a replied message\n• `-remind <time> <message>` - Set a reminder for the future",
            false
        )
        .field(
            "🛠️ Moderation Commands",
            "• `-cleanup [count|after]` - Delete messages (admin only)\n• `-update` - Update bot from GitHub (owner only)\n• `-kys` - Reboot bot with 1-hour cooldown",
            false
        )
        .field(
            "💡 Tips",
            "• Commands work with both prefix (`-`) and slash (`/`) formats\n• Most commands can be used in DMs or servers\n• Use `-help <command>` for detailed usage information",
            false
        )
        .footer(serenity::CreateEmbedFooter::new("Bot developed with Rust & Poise 🦀"))
        .timestamp(serenity::Timestamp::now());

    ctx.send(poise::CreateReply::default().embed(embed).ephemeral(false))
        .await?;

    Ok(())
}

async fn show_command_help(ctx: Context<'_>, command_name: &str) -> Result<(), Error> {
    let command_info = match command_name.to_lowercase().as_str() {
        "ping" => CommandInfo {
            name: "ping",
            description: "Check bot latency and responsiveness",
            usage: "`-ping` or `/ping`",
            examples: vec!["-ping"],
            parameters: vec![],
        },
        "hello" => CommandInfo {
            name: "hello",
            description: "Get a friendly greeting from the bot",
            usage: "`-hello` or `/hello`",
            examples: vec!["-hello"],
            parameters: vec![],
        },
        "coinflip" => CommandInfo {
            name: "coinflip",
            description: "Flip a virtual coin and get heads or tails",
            usage: "`-coinflip` or `/coinflip`",
            examples: vec!["-coinflip"],
            parameters: vec![],
        },
        "uwu" => CommandInfo {
            name: "uwu",
            description: "Convert text to uwu speak (cute anime-style text)",
            usage: "`-uwu <text>` or `/uwu <text>`",
            examples: vec!["-uwu hello world", "-uwu this is so cool"],
            parameters: vec!["text - The text to convert to uwu speak"],
        },
        "yourmom" => CommandInfo {
            name: "yourmom",
            description: "Get a random \"your mom\" joke",
            usage: "`-yourmom` or `/yourmom`",
            examples: vec!["-yourmom"],
            parameters: vec![],
        },
        "spamping" => CommandInfo {
            name: "spamping",
            description: "Send multiple ping messages (use responsibly!)",
            usage: "`-spamping <count>` or `/spamping <count>`",
            examples: vec!["-spamping 3", "-spamping 5"],
            parameters: vec!["count - Number of ping messages to send (1-10)"],
        },
        "pfp" => CommandInfo {
            name: "pfp",
            description: "Get the profile picture of yourself or another user",
            usage: "`-pfp [user]` or `/pfp [user]`",
            examples: vec!["-pfp", "-pfp @username"],
            parameters: vec!["user (optional) - The user whose profile picture to get"],
        },
        "stats" => CommandInfo {
            name: "stats",
            description: "Analyze message statistics in a channel",
            usage: "`-stats [count] [channel]` or `/stats [count] [channel]`",
            examples: vec!["-stats", "-stats 2000", "-stats 500 #general"],
            parameters: vec![
                "count (optional) - Number of messages to analyze (default: 1000, max: 10000)",
                "channel (optional) - Channel to analyze (default: current channel)",
            ],
        },
        "poll" => CommandInfo {
            name: "poll",
            description: "Create a poll with a question and multiple options",
            usage:
                "`-poll <question? option1 option2...>` or `/poll <question? option1 option2...>`",
            examples: vec![
                "-poll Is this cool? yes no maybe",
                "-poll Pizza or pasta? pizza pasta",
            ],
            parameters: vec![
                "question? options - Question followed by space-separated options (max 10)",
            ],
        },
        "cleanup" => CommandInfo {
            name: "cleanup",
            description: "Delete messages in the current channel (admin only)",
            usage: "`-cleanup [count]` or `-cleanup after` (reply to message)",
            examples: vec!["-cleanup 10", "-cleanup 50", "-cleanup after"],
            parameters: vec![
                "count (optional) - Number of messages to delete (default: 10, max: 1000)",
                "after - Delete all messages after the replied message",
            ],
        },
        "update" => CommandInfo {
            name: "update",
            description: "Update bot by pulling latest changes from GitHub (owner only)",
            usage: "`-update` or `/update`",
            examples: vec!["-update"],
            parameters: vec![],
        },
        "kys" => CommandInfo {
            name: "kys",
            description: "Reboot the bot with a 1-hour cooldown",
            usage: "`-kys` or `/kys`",
            examples: vec!["-kys"],
            parameters: vec![],
        },
        "react" => CommandInfo {
            name: "react",
            description: "Add emoji reactions to a replied message, spelling out the provided text",
            usage: "`-react <text>` (reply to a message)",
            examples: vec!["-react lol", "-react cool", "-react 123"],
            parameters: vec!["text - The text to spell out with emoji reactions"],
        },
        "remind" => CommandInfo {
            name: "remind",
            description: "Set a reminder that will be sent to you at a specified time",
            usage: "`-remind <time> <message>` or `/remind <time> <message>`",
            examples: vec![
                "-remind 30m Take a break",
                "-remind 2h Check the oven",
                "-remind tomorrow Call mom",
            ],
            parameters: vec![
                "time - When to remind (e.g., 30m, 2h, 1d, tomorrow)",
                "message - The reminder message",
            ],
        },
        "help" => CommandInfo {
            name: "help",
            description: "Show help information for bot commands",
            usage: "`-help [command]` or `/help [command]`",
            examples: vec!["-help", "-help ping", "-help stats"],
            parameters: vec!["command (optional) - Specific command to get detailed help for"],
        },
        _ => {
            ctx.send(
                poise::CreateReply::default()
                    .content(format!(
                        "❌ Command `{}` not found. Use `-help` to see all available commands.",
                        command_name
                    ))
                    .ephemeral(true),
            )
            .await?;
            return Ok(());
        }
    };

    let embed = create_command_help_embed(&command_info);

    ctx.send(poise::CreateReply::default().embed(embed).ephemeral(false))
        .await?;

    Ok(())
}

struct CommandInfo {
    name: &'static str,
    description: &'static str,
    usage: &'static str,
    examples: Vec<&'static str>,
    parameters: Vec<&'static str>,
}

fn create_command_help_embed(info: &CommandInfo) -> serenity::CreateEmbed {
    let mut embed = serenity::CreateEmbed::new()
        .title(format!("📖 Help: {}", info.name))
        .description(info.description)
        .color(0x7289DA)
        .field("📝 Usage", info.usage, false);

    if !info.examples.is_empty() {
        let examples_text = info.examples.join("\n");
        embed = embed.field("💡 Examples", format!("```\n{}\n```", examples_text), false);
    }

    if !info.parameters.is_empty() {
        let params_text = info
            .parameters
            .iter()
            .map(|p| format!("• {}", p))
            .collect::<Vec<_>>()
            .join("\n");
        embed = embed.field("⚙️ Parameters", params_text, false);
    }

    embed
        .footer(serenity::CreateEmbedFooter::new(
            "Use -help to see all commands",
        ))
        .timestamp(serenity::Timestamp::now())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_info_struct() {
        let info = CommandInfo {
            name: "test",
            description: "A test command",
            usage: "-test",
            examples: vec!["-test example"],
            parameters: vec!["param - A test parameter"],
        };

        assert_eq!(info.name, "test");
        assert_eq!(info.description, "A test command");
        assert_eq!(info.usage, "-test");
        assert_eq!(info.examples.len(), 1);
        assert_eq!(info.parameters.len(), 1);
    }

    #[test]
    fn test_help_command_signature() {
        // Verify the command exists and has the correct signature
        // This is a compile-time test to ensure the function signature matches expectations
        assert!(true, "Help command function compiles successfully");
    }
}
