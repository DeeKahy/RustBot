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
- 🔧 **Modular Design**: Easy to add new commands and features
- 📝 **Logging**: Built-in logging system for debugging and monitoring
- ⚡ **Async**: Built with Tokio for high performance
- 🎯 **Modern Framework**: Uses Poise for both prefix and slash commands
- 🐳 **Docker Support**: Multi-platform Docker images for easy deployment
- 🏠 **CasaOS Ready**: Optimized for CasaOS home server deployment

## Commands

- `-ping` - Responds with "Pong!" and shows latency
- `-hello [name]` - Says hello to you or the specified name
- `-help [command]` - Shows all available commands or detailed help for a specific command
- `-spamping @user [count]` - Creates a thread and pings the user repeatedly
- `-uwu <text>` - Transform text into uwu speak
- `-coinflip` - Flip a coin (heads or tails)
- `-pfp [user]` - Get user's profile picture
- `-yourmom` - Shows a random server member's profile picture
- `-kys` - Reboot bot with 1-hour cooldown

## Quick Start with Docker (Recommended)

### Option 1: CasaOS Deployment

**Docker Image**: `deekahy/rustbot:latest`

1. Add new application in CasaOS
2. Use image: `deekahy/rustbot:latest`
3. Set environment variable: `DISCORD_TOKEN=your_discord_bot_token`
4. Set memory limit: `256m` and CPU limit: `0.5`
5. Start the container

**Platforms Supported**: Linux AMD64, Linux ARM64 (Raspberry Pi 4+)

### Option 2: Docker Compose

```bash
# Quick start
DISCORD_TOKEN=your_token docker-compose -f docker-compose.casaos.yml up -d

# Or copy and edit the compose file
cp docker-compose.casaos.yml docker-compose.yml
# Edit docker-compose.yml to set your DISCORD_TOKEN
docker-compose up -d
```

### Option 3: Direct Docker Run

```bash
docker run -d \
  --name rustbot \
  --restart unless-stopped \
  -e DISCORD_TOKEN=your_discord_token_here \
  -e RUST_LOG=info \
  --memory=256m \
  --cpus=0.5 \
  deekahy/rustbot:latest
```

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

## Docker Deployment 🐳

RustBot can be easily deployed using Docker for consistent cross-platform deployment.

### Prerequisites for Docker

- Docker installed on your system
- Docker Hub account (for pushing images)

### Building and Running with Docker

#### Option 1: Using the build script (Recommended)

1. **Build and optionally push to Docker Hub:**
```bash
./docker-build.sh your_docker_username
```

2. **Run the container:**
```bash
# With environment variable
docker run -e DISCORD_TOKEN=your_token_here your_docker_username/rustbot:latest

# Or with .env file
docker run --env-file .env your_docker_username/rustbot:latest
```

#### Option 2: Manual Docker commands

1. **Build the image:**
```bash
docker build -t your_docker_username/rustbot:latest .
```

2. **Push to Docker Hub:**
```bash
docker login
docker push your_docker_username/rustbot:latest
```

3. **Run the container:**
```bash
docker run -e DISCORD_TOKEN=your_token_here your_docker_username/rustbot:latest
```

#### Option 3: Using Docker Compose

1. **Set your Discord token:**
```bash
export DISCORD_TOKEN=your_token_here
```

2. **Run with Docker Compose:**
```bash
docker-compose up -d
```

### Docker Features

- **Native Compilation**: The Docker image compiles the Rust code natively on the target architecture
- **Multi-stage Build**: Optimized build process with dependency caching
- **Environment Variables**: Easy configuration through environment variables
- **Auto-restart**: Container restarts automatically if the bot crashes (when using docker-compose)
- **Minimal Image**: Based on official Rust image with only necessary dependencies

### Docker Environment Variables

- `DISCORD_TOKEN` - Your Discord bot token (required)
- `RUST_LOG` - Log level (optional, defaults to `info`)

### Pulling from Docker Hub

Once pushed, others can run your bot directly:

```bash
docker run -e DISCORD_TOKEN=their_token_here your_docker_username/rustbot:latest
```

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