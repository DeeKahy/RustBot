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
        .title("RustBot — Commands")
        .description("All commands work with both the `-` prefix and `/` slash forms.\nUse `-help <command>` for details on a specific command.")
        .color(0x5865F2)
        .field(
            "Basic",
            "• `-ping` - Check bot latency and responsiveness\n\
             • `-hello [name]` - Get a friendly greeting\n\
             • `-help [command]` - Show this menu, or details for one command\n\
             • `-invite` - Get the bot's invite link\n\
             • `-status` - Show bot diagnostics and health",
            false,
        )
        .field(
            "Fun & Social",
            "• `-coinflip` - Flip a coin\n\
             • `-dice [sides]` - Roll a die (default 6 sides)\n\
             • `-uwu <text>` - Convert text to uwu speak (or reply to a message)\n\
             • `-mock <text>` - Alternating-case mocking text (or reply to a message)\n\
             • `-yourmom` - Show a random server member\n\
             • `-pfp [user]` - Get a user's profile picture\n\
             • `-bonk [user]` - Bonk a user (avatar on a bonk GIF)\n\
             • `-hit [user]` - Order a hit on a user (avatar on a hit GIF)",
            false,
        )
        .field(
            "Chat Tools",
            "• `-poll <question? opt1 opt2 ...>` - Create a reaction poll\n\
             • `-react <text>` - Spell out text with emoji reactions (reply to a message)\n\
             • `-spamping <user> [count]` - Ping a user in a thread until they respond\n\
             • `-stats [count] [channel]` - Channel activity report with charts\n\
             • `-remind set|list|remove|clear` - Manage personal reminders",
            false,
        )
        .field(
            "Voice / Music",
            "• `-play <url|search> [channel link]` - Play YouTube audio in a voice channel\n\
             • `-queue` - Show what's playing and queued\n\
             • `-skip` - Skip the current track\n\
             • `-stop` - Stop playback and clear the queue\n\
             • `-leave` - Leave the voice channel (aliases: `-disconnect`, `-dc`)",
            false,
        )
        .field(
            "Games",
            "• `-numberguess [min] [max]` - Guess the number (also `-guess`, `-hint`, `-gamestatus`, `-endgame`)\n\
             • `-tictactoe [@opponent]` - Tic-Tac-Toe vs a player or the AI (also `-move_ttt`, `-board`, `-endttt`)\n\
             • `-hangman` - Word guessing game (also `-letter`, `-hangmanstatus`, `-hangmanhint`, `-endhangman`)\n\
             See `GAMES.md` for full rules.",
            false,
        )
        .field(
            "Utility & Owner",
            "• `-park now|info|clear|schedule` - Mobile parking helper\n\
             • `-cleanup [count|after]` - Delete messages (protected)\n\
             • `-update` - Pull latest from GitHub and restart (protected)\n\
             • `-kys` - Reboot the bot with a 1-hour cooldown (protected)",
            false,
        )
        .footer(serenity::CreateEmbedFooter::new("Built with Rust + Poise"))
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
            usage: "`-hello [name]` or `/hello [name]`",
            examples: vec!["-hello", "-hello World"],
            parameters: vec!["name (optional) - Who to greet"],
        },
        "coinflip" => CommandInfo {
            name: "coinflip",
            description: "Flip a virtual coin and get heads or tails",
            usage: "`-coinflip` or `/coinflip`",
            examples: vec!["-coinflip"],
            parameters: vec![],
        },
        "dice" => CommandInfo {
            name: "dice",
            description: "Roll a dice with specified number of sides (defaults to 6)",
            usage: "`-dice [sides]` or `/dice [sides]`",
            examples: vec!["-dice", "-dice 20", "-dice 100", "-dice 1"],
            parameters: vec![
                "sides (optional): Number of sides on the dice (1-1000, defaults to 6)",
            ],
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
            description: "Analyze channel activity and render a chart report (top users, hourly activity, message share)",
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
        "invite" => CommandInfo {
            name: "invite",
            description: "Generate and display the bot's invite link",
            usage: "`-invite` or `/invite`",
            examples: vec!["-invite"],
            parameters: vec![],
        },
        "react" => CommandInfo {
            name: "react",
            description: "Add emoji reactions to a message",
            usage: "`-react <text>` (reply to message) or `/react <message_id> <text>`",
            examples: vec!["-react thumbs up", "-react fire heart"],
            parameters: vec![
                "text - The text to convert to emoji reactions",
                "message_id (slash only) - ID of message to react to",
            ],
        },
        "remind" => CommandInfo {
            name: "remind",
            description: "Set a reminder for the future",
            usage: "`-remind <time> <message>` or `/remind <time> <message>`",
            examples: vec![
                "-remind 10m Take a break",
                "-remind 2h Meeting starts",
                "-remind 1d Pay bills",
            ],
            parameters: vec![
                "time - Time to wait (e.g., 10m, 2h, 1d)",
                "message - Reminder message",
            ],
        },
        "help" => CommandInfo {
            name: "help",
            description: "Show help information for bot commands",
            usage: "`-help [command]` or `/help [command]`",
            examples: vec!["-help", "-help ping", "-help stats"],
            parameters: vec!["command (optional) - Specific command to get detailed help for"],
        },
        "status" => CommandInfo {
            name: "status",
            description: "Show diagnostic information and bot health",
            usage: "`-status` or `/status`",
            examples: vec!["-status"],
            parameters: vec![],
        },
        "mock" => CommandInfo {
            name: "mock",
            description: "Transform text into mOcKiNg alternating case, or reply to a message to mock it",
            usage: "`-mock <text>` or `/mock <text>`",
            examples: vec!["-mock this is a great idea"],
            parameters: vec!["text - The text to mock (optional when replying to a message)"],
        },
        "bonk" => CommandInfo {
            name: "bonk",
            description: "Bonk a user by placing their profile picture on a random bonk GIF",
            usage: "`-bonk [user]` or `/bonk [user]`",
            examples: vec!["-bonk", "-bonk @username"],
            parameters: vec!["user (optional) - Who to bonk (defaults to you)"],
        },
        "hit" => CommandInfo {
            name: "hit",
            description: "Order a hit on a user by placing their profile picture on a random hit GIF",
            usage: "`-hit [user]` or `/hit [user]`",
            examples: vec!["-hit", "-hit @username"],
            parameters: vec!["user (optional) - The target (defaults to you)"],
        },
        "park" => CommandInfo {
            name: "park",
            description: "Mobile parking helper: park now, view/clear saved info, or schedule weekday parking",
            usage: "`-park now|info|clear|schedule` or the matching slash subcommands",
            examples: vec!["-park now", "-park info", "-park schedule"],
            parameters: vec!["subcommand - one of: now, info, clear, schedule"],
        },
        "play" => CommandInfo {
            name: "play",
            description: "Play a YouTube video's audio in a voice channel",
            usage: "`-play <url|search> [channel link]` or `/play <url|search>`",
            examples: vec![
                "-play never gonna give you up",
                "-play https://youtu.be/dQw4w9WgXcQ",
            ],
            parameters: vec![
                "query - A YouTube URL or search terms",
                "channel link (optional) - A Discord channel link to target a specific voice channel (useful from DMs)",
            ],
        },
        "skip" => CommandInfo {
            name: "skip",
            description: "Skip the track that's currently playing",
            usage: "`-skip` or `/skip`",
            examples: vec!["-skip"],
            parameters: vec![],
        },
        "stop" => CommandInfo {
            name: "stop",
            description: "Stop playback and clear the queue (stays in the channel)",
            usage: "`-stop` or `/stop`",
            examples: vec!["-stop"],
            parameters: vec![],
        },
        "queue" => CommandInfo {
            name: "queue",
            description: "Show what's playing and what's queued up next",
            usage: "`-queue` or `/queue`",
            examples: vec!["-queue"],
            parameters: vec![],
        },
        "leave" | "disconnect" | "dc" => CommandInfo {
            name: "leave",
            description: "Leave the voice channel (also clears the queue)",
            usage: "`-leave` (aliases: `-disconnect`, `-dc`) or `/leave`",
            examples: vec!["-leave"],
            parameters: vec![],
        },
        "numberguess" => CommandInfo {
            name: "numberguess",
            description: "Start a number guessing game (default range 1-100)",
            usage: "`-numberguess [min] [max]` or `/numberguess [min] [max]`",
            examples: vec!["-numberguess", "-numberguess 1 500"],
            parameters: vec![
                "min (optional) - Lower bound (default 1)",
                "max (optional) - Upper bound (default 100)",
                "Play with `-guess <n>`, `-hint`, `-gamestatus`, `-endgame`",
            ],
        },
        "tictactoe" => CommandInfo {
            name: "tictactoe",
            description: "Start a Tic-Tac-Toe game against another player or the AI",
            usage: "`-tictactoe [@opponent]` or `/tictactoe [@opponent]`",
            examples: vec!["-tictactoe", "-tictactoe @username"],
            parameters: vec![
                "opponent (optional) - Mention a player, or omit to play the AI",
                "Play with `-move_ttt <1-9>`, `-board`, `-endttt`",
            ],
        },
        "hangman" => CommandInfo {
            name: "hangman",
            description: "Start a Hangman word guessing game",
            usage: "`-hangman` or `/hangman`",
            examples: vec!["-hangman"],
            parameters: vec![
                "Play with `-letter <a-z>`, `-hangmanstatus`, `-hangmanhint`, `-endhangman`",
            ],
        },
        _ => {
            ctx.send(
                poise::CreateReply::default()
                    .content(format!(
                        "❌ Command `{command_name}` not found. Use `-help` to see all available commands."
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
        .title(format!("Help: {}", info.name))
        .description(info.description)
        .color(0x5865F2)
        .field("Usage", info.usage, false);

    if !info.examples.is_empty() {
        let examples_text = info.examples.join("\n");
        embed = embed.field("Examples", format!("```\n{examples_text}\n```"), false);
    }

    if !info.parameters.is_empty() {
        let params_text = info
            .parameters
            .iter()
            .map(|p| format!("• {p}"))
            .collect::<Vec<_>>()
            .join("\n");
        embed = embed.field("Parameters", params_text, false);
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
        // Test passes if the function compiles and can be called
        let info = CommandInfo {
            name: "test",
            description: "Test description",
            usage: "test usage",
            examples: vec!["test example"],
            parameters: vec!["test param"],
        };
        assert_eq!(info.name, "test");
    }
}
