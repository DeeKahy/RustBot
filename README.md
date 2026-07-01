# RustBot

A Discord bot built in Rust with [Serenity](https://github.com/serenity-rs/serenity) and the
[Poise](https://github.com/serenity-rs/poise) command framework. Commands live in their own files
under `src/commands/` and are registered for both the `-` prefix and `/` slash forms.

## Features

- Utility: latency check, diagnostics, channel activity reports with rendered charts, reminders, polls
- Voice / music: play YouTube audio in a voice channel (via songbird), with queue controls; works from a DM via a channel link
- Fun: coin flip, dice, uwu/mock text transforms, profile-picture gags (bonk, hit, yourmom)
- Games: number guessing, Tic-Tac-Toe (vs a player or the AI), Hangman
- Moderation / owner: message cleanup, self-update from GitHub, cooldown-gated reboot
- Reproducible Nix flake build plus a hardened, multi-instance NixOS service module

## Commands

All commands accept both the `-` prefix and the `/` slash form. Run `-help` for the in-Discord menu,
or `-help <command>` for details on one command.

### Basic
- `-ping` - Latency and responsiveness
- `-hello [name]` - Friendly greeting
- `-help [command]` - Command menu, or details for one command
- `-invite` - Bot invite link
- `-status` - Diagnostics and health

### Chat tools
- `-stats [count] [channel]` - Channel activity report: bar chart of the most active users (with
  avatars), an hourly-activity histogram, a message-share pie chart, plus word/character leaders and
  highlights
- `-poll <question? opt1 opt2 ...>` - Reaction poll (up to 10 options)
- `-react <text>` - Spell out text with emoji reactions on a replied-to message
- `-spamping <user> [count]` - Ping a user in a dedicated thread until they respond
- `-remind set|list|remove|clear` - Personal reminders with flexible time formats

### Voice / music
- `-play <url|search> [channel link]` - Play a YouTube video's audio. In a server it joins your
  current voice channel; paste a `https://discord.com/channels/<server>/<channel>` link to target a
  specific one (works from a DM)
- `-queue` - Show what's playing and queued
- `-skip` / `-stop` - Skip the current track / stop and clear the queue
- `-leave` - Leave the voice channel (aliases: `-disconnect`, `-dc`)

### Fun
- `-coinflip`, `-dice [sides]`
- `-uwu <text>`, `-mock <text>` (both also work by replying to a message)
- `-pfp [user]`, `-yourmom`, `-bonk [user]`, `-hit [user]`

### Games
See [GAMES.md](GAMES.md) for full rules.
- `-numberguess [max]` (`-guess`, `-hint`, `-gamestatus`, `-endgame`)
- `-tictactoe [@opponent]` (`-move_ttt`, `-board`, `-endttt`)
- `-hangman` (`-letter`, `-hangmanstatus`, `-hangmanhint`, `-endhangman`)

### Moderation / owner
Protected commands are limited to the usernames in `PROTECTED_USERS`.
- `-cleanup [count|after]` - Delete messages in the current channel
- `-update` - Pull the latest changes from GitHub and restart
- `-kys` - Reboot the bot (1-hour cooldown)

## Deployment (Nix flake)

RustBot ships as a flake providing `packages.default` (the bot) and `nixosModules.default` (a
hardened, multi-instance systemd service). The build bundles `yt-dlp` and `ffmpeg` onto the binary's
PATH and links `libopus`; voice also needs a working outbound path to Discord (IPv4).

Add it as an input and configure one or more instances:

```nix
# flake.nix
inputs.rustbot.url = "github:DeeKahy/RustBot";

# in your nixosConfiguration modules:
{
  imports = [ inputs.rustbot.nixosModules.default ];
  services.rustbot.instances.main = {
    # Root-only env file with DISCORD_TOKEN, PROTECTED_USERS, RUST_LOG, ...
    environmentFile = "/var/lib/rustbot/env";
  };
}
```

Build and run locally with `nix run github:DeeKahy/RustBot` (needs `DISCORD_TOKEN` in the
environment or a `.env`).

## Development setup

### Prerequisites
- Rust (latest stable)
- A Discord application and bot token
- For voice: `yt-dlp` and `ffmpeg` on PATH, plus `libopus` (handled automatically by the Nix build)

### 1. Get a bot token
1. Open the [Discord Developer Portal](https://discord.com/developers/applications) and create an application
2. Under "Bot", create a bot and copy the token
3. Enable the "Message Content Intent"

### 2. Configure the environment
```bash
cp .env.example .env
# then edit .env and set DISCORD_TOKEN=...
```

### 3. Invite the bot
Generate an invite link with these permissions: Send Messages, Read Messages, Read Message History,
Use Slash Commands, Create Public Threads, Send Messages in Threads, Connect, Speak.

Template (replace `YOUR_CLIENT_ID`):
```
https://discord.com/api/oauth2/authorize?client_id=YOUR_CLIENT_ID&permissions=277025492032&scope=bot%20applications.commands
```

### 4. Run
```bash
cargo run
```

## Environment variables

- `DISCORD_TOKEN` - Discord bot token (required)
- `RUST_LOG` - Log level (optional; `warn,rustbot=info,songbird=info` is a good default -- plain `info` is very noisy)
- `GIT_BRANCH` - Branch to pull from during `-update` (optional, defaults to `main`)
- `PROTECTED_USERS` - Space-separated usernames allowed to run protected commands (`-update`, `-cleanup`, `-kys`); defaults to `deekahy`

## Adding a command

1. Create `src/commands/yourcommand.rs`:

   ```rust
   use crate::{Context, Error};

   /// Your new command description
   #[poise::command(prefix_command, slash_command)]
   pub async fn yourcommand(
       ctx: Context<'_>,
       #[description = "Optional parameter"] param: Option<String>,
   ) -> Result<(), Error> {
       ctx.say("Your response here").await?;
       Ok(())
   }
   ```

2. Declare and re-export it in `src/commands/mod.rs`:

   ```rust
   pub mod yourcommand;
   pub use yourcommand::yourcommand;
   ```

3. Add it to the `commands` vector in `src/main.rs`.

Keep both `prefix_command` and `slash_command` so the command is available in both forms, and add a
doc comment -- it becomes the command's description.

## Project structure

```
RustBot/
  src/
    main.rs            # Setup, framework options, command registration
    commands/          # One file per command (plus mod.rs re-exports)
  assets/              # Bundled assets (bonk/hit GIFs, chart fonts)
  flake.nix            # Nix package + multi-instance NixOS module
  Cargo.toml
  GAMES.md             # Game rules
  README.md
```

## Dependencies

serenity (Discord API), poise (command framework), tokio (async runtime), songbird (voice),
image/imageproc/rusttype (chart rendering), reqwest (HTTP), chrono (time), plus logging and
serialization crates. See `Cargo.toml` for the full list.

## Troubleshooting

- **Bot doesn't respond**: confirm the Message Content Intent is enabled and the bot can read/send in
  the channel; check the logs.
- **"Token is invalid"**: verify `DISCORD_TOKEN` in `.env` has no stray spaces or quotes.
- **Slash commands missing**: invite with the `applications.commands` scope and allow a few minutes
  for Discord to register them globally.
- **Voice won't play**: ensure `yt-dlp` and `ffmpeg` are on PATH and the host has outbound IPv4 to
  Discord; check for "Connect"/"Speak" permissions.
- **Compilation errors**: run `cargo update` and confirm a recent stable Rust toolchain.

## Contributing

Issues and pull requests are welcome. When adding features: keep one command per file, add
`log::info!` on command use, update this README and `-help`, and run `cargo build` / `cargo test`
before submitting.

## License

Open source under the [MIT License](LICENSE).
