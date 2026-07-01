# RustBot 🤖

A modern Discord bot built with Rust using the Serenity library and Poise command framework. This bot provides a foundation for building Discord bots with an easy-to-extend command system.

## Features

- 🏓 **Ping Command**: Basic ping-pong functionality with latency measurement
- 👋 **Hello Command**: Friendly greeting command with optional name parameter
- 📖 **Help Command**: Comprehensive help system showing all available commands
- 🚨 **Spam Ping**: Creates a thread and repeatedly pings a user until they respond
- 🎯 **UwU Command**: Transform text into uwu speak
- 🪙 **Coin Flip**: Random coin flip command
- 👤 **Profile Picture**: Get user's profile picture
- 🎲 **Your Mom**: Displays a random server member's profile picture with a funny message
- ⏰ **Reminder System**: Set personal reminders with flexible time formats
- 🔧 **Modular Design**: Easy to add new commands and features
- 📝 **Logging**: Built-in logging system for debugging and monitoring
- ⚡ **Async**: Built with Tokio for high performance
- 🎯 **Modern Framework**: Uses Poise for both prefix and slash commands
- 🔊 **Voice / Music**: Play YouTube audio in a voice channel (songbird), incl. from a DM via a channel link
- ❄️ **Nix Flake**: Reproducible build + hardened NixOS service module (multi-instance)

## Commands

- `-ping` - Responds with "Pong!" and shows latency
- `-hello [name]` - Says hello to you or the specified name
- `-help [command]` - Shows all available commands or detailed help for a specific command
- `-status` - Shows comprehensive bot diagnostic information and health status
- `-spamping @user [count]` - Creates a thread and pings the user repeatedly
- `-uwu <text>` - Transform text into uwu speak
- `-coinflip` - Flip a coin (heads or tails)
- `-pfp [user]` - Get user's profile picture
- `-yourmom` - Shows a random server member's profile picture
- `-kys` - Reboot bot with 1-hour cooldown
- `-remind set <time> [message]` - Set a reminder (message optional when replying)
- `-remind list` - List your active reminders
- `-remind remove <id>` - Remove a specific reminder
- `-remind clear` - Clear all your reminders
- `-react <text>` - Add emoji reactions to a replied message spelling out the text
- `-play <url|search> [channel link]` - Play a YouTube video's audio in a voice channel. In a server it joins your current channel; paste a `https://discord.com/channels/<server>/<channel>` link to target a specific one (works from a DM)
- `-skip` / `-stop` / `-queue` / `-leave` - Voice playback controls (leave also aliased `-disconnect` / `-dc`)

## Deployment (Nix flake)

RustBot ships as a flake providing `packages.default` (the bot) and
`nixosModules.default` (a hardened, multi-instance systemd service). The build
bundles `yt-dlp` + `ffmpeg` onto the binary's PATH and links `libopus`; voice
also needs a working outbound network path to Discord (IPv4).

Add it as an input and configure one or more instances:

```nix
# flake.nix
inputs.rustbot.url = "github:DeeKahy/RustBot";

# in your nixosConfiguration modules:
{
  imports = [ inputs.rustbot.nixosModules.default ];
  services.rustbot.instances.main = {
    # Root-only env file with DISCORD_TOKEN, PROTECTED_USERS, RUST_LOG, …
    environmentFile = "/var/lib/rustbot/env";
  };
}
```

Build/run locally with `nix run github:DeeKahy/RustBot` (needs `DISCORD_TOKEN`
in the environment or a `.env`).

## Development Setup

### Prerequisites

- Rust (latest stable version)
- A Discord application and bot token

### 1. Clone and Setup

```bash
git clone <your-repo-url>
cd RustBot
```

### 2. Get a Discord Bot Token

1. Go to the [Discord Developer Portal](https://discord.com/developers/applications)
2. Create a new application
3. Go to the "Bot" section
4. Create a bot and copy the token
5. Enable the "Message Content Intent" in the bot settings

### 3. Configure Environment

Copy the example environment file and add your bot token:

```bash
cp .env.example .env
```

Edit `.env` and replace `your_bot_token_here` with your actual bot token:

```
DISCORD_TOKEN=your_actual_bot_token_here
```

### 4. Invite Bot to Server

Generate an invite link with the following permissions:
- Send Messages
- Read Messages
- Read Message History
- Use Slash Commands
- Create Public Threads
- Send Messages in Threads

You can use this URL template (replace `YOUR_CLIENT_ID` with your bot's client ID):
```
https://discord.com/api/oauth2/authorize?client_id=YOUR_CLIENT_ID&permissions=277025492032&scope=bot%20applications.commands
```

### 5. Run the Bot

```bash
cargo run
```

## Environment Variables

- `DISCORD_TOKEN` - Your Discord bot token (required)
- `RUST_LOG` - Log level (optional; `warn,rustbot=info,songbird=info` is a good default — plain `info` is very noisy)
- `GIT_BRANCH` - Git branch to pull from during `-update` (optional, defaults to `main`)
- `PROTECTED_USERS` - Space-separated usernames allowed to use protected commands like `-update`, `-cleanup`, and `-kys` (optional, defaults to `deekahy`)

## Adding New Commands

To add new commands, follow these steps:

### 1. Create a new command file

Create a new file like `src/commands/yourcommand.rs`:

```rust
use crate::{Context, Error};

/// Your new command description
#[poise::command(prefix_command, slash_command)]
pub async fn yourcommand(
    ctx: Context<'_>,
    #[description = "Optional parameter"] param: Option<String>,
) -> Result<(), Error> {
    let response = "Your response here!";
    // Simple error handling - just print the raw API error if something fails
    if let Err(e) = ctx.say(response).await {
        ctx.say(format!("❌ {}", e)).await?;
    }
    Ok(())
}
```

### 2. Update the commands module

In `src/commands/mod.rs`, add your new module:

```rust
pub mod yourcommand;
pub use yourcommand::yourcommand;
```

### 3. Add the command to the framework

In `src/main.rs`, add your command to the commands vector:

```rust
use commands::{hello, ping, spamping, yourcommand};

let framework = poise::Framework::builder()
    .options(poise::FrameworkOptions {
        commands: vec![ping(), hello(), spamping(), yourcommand()],
        // ... rest of the configuration
    })
```

## Project Structure

```
RustBot/
├── src/
│   ├── main.rs              # Main bot logic and setup
│   └── commands/
│       ├── mod.rs           # Commands module declaration and re-exports
│       ├── general.rs       # Shared utilities and helper functions
│       ├── ping.rs          # Ping command (separate file)
│       ├── hello.rs         # Hello command (separate file)
│       └── spamping.rs      # Spam ping command (separate file)
├── Cargo.toml               # Rust dependencies and project config
├── .env.example             # Example environment variables
├── .env                     # Your environment variables (create this)
├── LICENSE                  # MIT license
└── README.md                # This file
```

## Dependencies

- **serenity**: Discord API library for Rust
- **poise**: Modern command framework for Serenity
- **tokio**: Async runtime
- **env_logger**: Logging implementation
- **dotenv**: Environment variable loading
- **log**: Logging facade
- **chrono**: Date and time handling for message timestamps

## Environment Variables

- `DISCORD_TOKEN` - Your Discord bot token (required)
- `RUST_LOG` - Log level (optional, defaults to `info`)
- `GIT_BRANCH` - Git branch to pull from during updates (optional, defaults to `main`)
- `PROTECTED_USERS` - Space-separated list of usernames who can use protected commands like `-update`, `-cleanup`, and `-kys` (optional, defaults to `deekahy`)

## Example Usage

Once the bot is running and invited to your server:

```
User: !ping
Bot: 🏓 Pong! `45ms`

User: /ping
Bot: 🏓 Pong! `45ms`

User: !hello
Bot: 👋 Hello, YourUsername! Nice to meet you!

User: /hello World
Bot: 👋 Hello, World! Nice to meet you!

User: !help
Bot: [Displays comprehensive help embed with all available commands organized by category]

User: /help ping
Bot: [Shows detailed help for the ping command including usage, examples, and parameters]

User: !spamping @SomeUser
Bot: ✅ Spam ping started for @SomeUser in #🚨-spamping-someuser-until-they-respond! They will be pinged every 10 seconds until they respond.

[In the created thread]
Bot: 🚨 **SPAM PING ACTIVATED** 🚨
     @SomeUser, you are being pinged every 10 seconds until you respond!
     Type anything in this thread to stop the spam! 😈
     
Bot: 🔔 Ping #2: @SomeUser - Please respond!
Bot: 📢 Ping #3: @SomeUser - HELLO?! Are you there?
Bot: 🚨 Ping #4: @SomeUser - EMERGENCY PING! RESPOND NOW!
...
SomeUser: I'm alive!
Bot: 🎉 @SomeUser responded! Spam ping stopped after 4 pings. Welcome back to the land of the living! 🎉

User: !yourmom
Bot: [Embed with title "Your mom is RandomUser123!" showing RandomUser123's profile picture]
     Description: "Behold, the chosen one: RandomUser123"
     Footer: "Requested by YourUsername • Total members: 47"

User: /yourmom
Bot: [Same embed but triggered via slash command]

User: !remind set 30m Take out the trash
Bot: ⏰ Reminder Set!
     Message: Take out the trash
     Remind at: Today at 3:45 PM
     Reminder ID: 1

User: [Replies to someone's message about "team lunch"] !remind set 15m
Bot: ⏰ Reminder Set!
     Message: ⏰ Reminder
     Remind at: Today at 3:30 PM
     Reminder ID: 2

User: [Replies to someone's message] !remind set 1h Check on this
Bot: ⏰ Reminder Set!
     Message: Check on this
     Remind at: Today at 4:15 PM
     Reminder ID: 3

User: /remind list
Bot: 📋 Your Active Reminders
     ID 1: Take out the trash
     ⏰ in 25 minutes
     
     Total active reminders: 1

[30 minutes later]
Bot: ⏰ Reminder!
     Take out the trash
     
     @YourUsername
     Set 30 minutes ago

User: [Replies to someone's message] !remind set 2h Call mom
Bot: ⏰ Reminder Set!
     Message: Call mom
     Remind at: Today at 5:30 PM
     Reminder ID: 2

[15 minutes later, bot replies to the "team lunch" message]
Bot: [Replying to the "team lunch" message] ⏰ Reminder!
     ⏰ Reminder
     
     @YourUsername
     Set 15 minutes ago

[1 hour later, bot replies to the original message]
Bot: [Replying to the original message] ⏰ Reminder!
     Check on this
     
     @YourUsername
     Set 1 hour ago

[2 hours later, bot replies to the original message]
Bot: [Replying to the original message] ⏰ Reminder!
     Call mom
     
     @YourUsername
     Set 2 hours ago

User: /remind remove 3
Bot: 🗑️ Reminder Removed
     Removed: Call mom

User: !remind clear
Bot: 🧹 Reminders Cleared
     Removed 1 reminder(s)
```

## SpamPing Command Details

The `spamping` command has several built-in safety features and escalating intensity:

### 🛡️ **Safety Features**
- **Auto-stop after 50 pings** (~8 minutes) to prevent infinite spam
- **Thread isolation** - spam happens in a separate thread, not the main channel
- **User response detection** - stops immediately when target user types anything
- **Server-only** - cannot be used in DMs to prevent abuse

### 📈 **Escalating Intensity**
- **Pings 1-5**: Polite requests ("Please respond!")
- **Pings 6-10**: More urgent ("HELLO?! Are you there?")
- **Pings 11-15**: Emergency level ("EMERGENCY PING! RESPOND NOW!")
- **Pings 16-20**: Dramatic ("Are you still alive?! RESPOND!")
- **Pings 21+**: Persistent ("This is getting ridiculous... please respond!")

### ⚙️ **How It Works**
1. Creates a public thread in the current channel
2. Posts initial warning message with instructions
3. Starts background task that pings every 10 seconds
4. Monitors thread for any messages from the target user
5. Stops when user responds or after 50 attempts
```

## Reminder System Details

The reminder system allows users to set personal reminders that will be delivered back to them at specified times.

### ⏰ **Time Formats Supported**
- **Seconds**: `30s`, `45sec`, `1second`, `5seconds`
- **Minutes**: `5m`, `15min`, `30minute`, `45minutes`
- **Hours**: `1h`, `2hr`, `8hour`, `12hours`
- **Days**: `1d`, `3day`, `7days`
- **Weeks**: `1w`, `2week`, `4weeks`

### 🎯 **Features**
- **Personal reminders**: Only you can see and manage your reminders
- **Persistent storage**: Reminders survive bot restarts
- **Background monitoring**: Automatic delivery when time is reached
- **Multiple reminders**: Set as many as you need
- **Easy management**: List, remove, or clear all reminders
- **Cross-channel delivery**: Reminders are sent where they were originally set
- **Reply-to-message support**: When you set a reminder while replying to someone, the reminder will reply to that same message (without pinging the original author)

### 📝 **Commands**
- **`-remind set <time> [message]`**: Set a new reminder
  - Example: `-remind set 1h30m Meeting with team`
  - **Reply feature**: Use this while replying to a message to get reminded about that specific message
  - **No message needed**: When replying to a message, you can omit the message: `-remind set 30m`
  - **With custom message**: Or provide a custom message: `-remind set 30m Check on this later`
- **`-remind list`**: Show all your active reminders with IDs and times
- **`-remind remove <id>`**: Remove a specific reminder by ID
- **`-remind clear`**: Remove all your reminders at once

### 🔧 **How It Works**
1. Reminders are stored in JSON format with unique IDs
2. Background task checks every minute for due reminders
3. When time arrives, reminder is sent as a mention in the original channel
4. If the reminder was set as a reply, it will reply to the original message (mentioning you but not the original author)
5. Delivered reminders are automatically removed from storage
6. All times are calculated from when the reminder was set

### 💾 **Data Storage**
- Reminders are stored in `/tmp/rustbot_reminders.json`
- Each reminder includes: ID, user ID, channel ID, message, remind time, creation time, and optional reply-to message ID
- File persists between bot restarts for reliability
- Automatic migration from older reminder formats for backwards compatibility

## Command Types

This bot supports both **prefix commands** (starting with `!`) and **slash commands** (Discord's native `/` commands). Each command is automatically registered for both types when you include both `prefix_command` and `slash_command` in the `#[poise::command()]` attribute.

## Troubleshooting

### Bot doesn't respond
- Check that the bot has the "Message Content Intent" enabled
- Verify the bot has permission to read and send messages in the channel
- For slash commands, ensure the bot has "Use Slash Commands" permission
- Check the console for error messages

### "Token is invalid" error
- Make sure your `.env` file contains the correct Discord bot token
- Verify there are no extra spaces or quotes around the token

### Compilation errors
- Run `cargo update` to update dependencies
- Check that you're using Rust 2021 edition or later

### Slash commands not appearing
- Make sure you invited the bot with the `applications.commands` scope
- Wait a few minutes for Discord to register the commands globally
- Try running the bot again to re-register commands

### Spamping command issues
- The command only works in server channels, not DMs
- The bot needs permission to create threads in the channel
- If a user doesn't respond after 50 pings (~8 minutes), the spam automatically stops
- The target user must type anything in the created thread to stop the spam
- Make sure the bot has "Create Public Threads" and "Send Messages in Threads" permissions

### Spamping not working
- Verify the channel allows thread creation
- Check that the target user isn't a bot (bots don't get pinged the same way)
- Ensure the bot has proper thread permissions in the server

## Contributing

Feel free to submit issues and enhancement requests! When adding new features:

1. **Follow the modular structure**: Each command gets its own file
2. **Add appropriate logging**: Use `log::info!()` for command usage
3. **Update documentation**: Update this README if you add new commands
4. **Test your changes**: Run `cargo build` and test commands before submitting
5. **Support both types**: Include both `prefix_command` and `slash_command` when possible
6. **Use the command template**: Follow the pattern from existing command files

## Architecture Decisions

### Why Poise?
This project uses Poise instead of Serenity's standard framework because:
- The standard framework is deprecated and will be removed
- Poise supports both prefix and slash commands seamlessly
- Better error handling and type safety
- More modern and actively maintained
- Easier to use and extend

### Why Modular Command Files?
Each command is in its own file for several benefits:
- **Easier maintenance**: Find and edit specific commands quickly (`ping.rs`, `hello.rs`, etc.)
- **Better organization**: Each command is completely self-contained
- **Team development**: Multiple people can work on different commands without conflicts
- **Clean imports**: Clear dependency management and no file bloat
- **Scalability**: Easy to add many commands without cluttering any single file
- **Debugging**: Easier to isolate issues to specific command files

### Simple Error Handling Approach
All commands use a simple error handling pattern:
- **Raw API errors**: Print exactly what Discord's API returns (e.g., "Cannot execute action on this channel type")
- **No fancy parsing**: Errors are displayed as-is for better debugging
- **Fallback friendly**: Works as a debugging tool for any command
- **User-friendly**: Users see exactly what went wrong without interpretation

## License

This project is open source and available under the [MIT License](LICENSE).