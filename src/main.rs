use std::env;
use std::fs;

use poise::serenity_prelude as serenity;
use serde::{Deserialize, Serialize};
use serenity::{ChannelId, Client, GatewayIntents};

mod commands;

use commands::{
    cleanup, coinflip, hello, help, kys, pfp, ping, poll, spamping, stats, update, uwu, yourmom,
};

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

#[derive(Serialize, Deserialize)]
struct UpdateInfo {
    channel_id: u64,
    user_name: String,
}

#[derive(Serialize, Deserialize)]
struct KysInfo {
    channel_id: u64,
    user_name: String,
}

// User data, which is stored and accessible in all command invocations
pub struct Data {}

async fn on_error(error: poise::FrameworkError<'_, Data, Error>) {
    // This is our global error handler
    match error {
        poise::FrameworkError::Setup { error, .. } => panic!("Failed to start bot: {:?}", error),
        poise::FrameworkError::Command { error, ctx, .. } => {
            println!("Error in command `{}`: {:?}", ctx.command().name, error,);
        }
        error => {
            if let Err(e) = poise::builtins::on_error(error).await {
                println!("Error while handling error: {}", e)
            }
        }
    }
}

#[tokio::main]
async fn main() {
    // Initialize logger
    env_logger::init();

    // Load environment variables from .env file
    dotenv::dotenv().ok();

    // Get the bot token from environment variables
    let token = env::var("DISCORD_TOKEN")
        .expect("Expected a Discord bot token in the environment variable DISCORD_TOKEN");

    // Set gateway intents
    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::GUILD_MEMBERS;

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![
                ping(),
                hello(),
                help(),
                spamping(),
                uwu(),
                coinflip(),
                pfp(),
                yourmom(),
                stats(),
                update(),
                kys(),
                poll(),
                cleanup(),
            ],
            prefix_options: poise::PrefixFrameworkOptions {
                prefix: Some("-".into()),
                ..Default::default()
            },
            on_error: |error| Box::pin(on_error(error)),
            pre_command: |ctx| {
                Box::pin(async move {
                    log::info!("Executing command {}...", ctx.command().qualified_name);
                })
            },
            post_command: |ctx| {
                Box::pin(async move {
                    log::info!("Executed command {}!", ctx.command().qualified_name);
                })
            },
            ..Default::default()
        })
        .setup(move |ctx, _ready, framework| {
            Box::pin(async move {
                log::info!("Logged in as {}", _ready.user.name);
                println!("🤖 {} is online and ready!", _ready.user.name);

                // Check if this is a restart after an update
                if let Ok(update_info_str) = fs::read_to_string("/tmp/rustbot_update_info.json") {
                    if let Ok(update_info) = serde_json::from_str::<UpdateInfo>(&update_info_str) {
                        let channel_id = ChannelId::new(update_info.channel_id);
                        match channel_id
                            .say(
                                &ctx.http,
                                format!(
                                    "✅ Update complete! {} is back online and ready! 🤖",
                                    _ready.user.name
                                ),
                            )
                            .await
                        {
                            Ok(_) => {
                                log::info!(
                                    "Successfully sent post-update startup message to channel {}",
                                    update_info.channel_id
                                );
                            }
                            Err(e) => {
                                log::error!(
                                    "Failed to send startup message to channel {}: {}",
                                    update_info.channel_id,
                                    e
                                );
                            }
                        }

                        // Clean up the update info file
                        if let Err(e) = fs::remove_file("/tmp/rustbot_update_info.json") {
                            log::warn!("Failed to remove update info file: {}", e);
                        }
                    }
                }

                // Check if this is a restart after a kys command (1-hour cooldown)
                if let Ok(kys_info_str) = fs::read_to_string("/tmp/rustbot_kys_info.json") {
                    if let Ok(kys_info) = serde_json::from_str::<KysInfo>(&kys_info_str) {
                        let channel_id = ChannelId::new(kys_info.channel_id);
                        match channel_id
                            .say(
                                &ctx.http,
                                format!(
                                    "🌅 Good morning! {} has awakened from their 1-hour slumber and is back online! 🤖",
                                    _ready.user.name
                                ),
                            )
                            .await
                        {
                            Ok(_) => {
                                log::info!(
                                    "Successfully sent post-kys startup message to channel {}",
                                    kys_info.channel_id
                                );
                            }
                            Err(e) => {
                                log::error!(
                                    "Failed to send kys startup message to channel {}: {}",
                                    kys_info.channel_id,
                                    e
                                );
                            }
                        }

                        // Clean up the kys info file
                        if let Err(e) = fs::remove_file("/tmp/rustbot_kys_info.json") {
                            log::warn!("Failed to remove kys info file: {}", e);
                        }
                    }
                }

                // Log all registered commands for debugging
                let commands = &framework.options().commands;
                log::info!("Registering {} commands:", commands.len());
                for command in commands {
                    log::info!(
                        "  - {} (prefix: {}, slash: {})",
                        command.name,
                        command.prefix_action.is_some(),
                        command.slash_action.is_some()
                    );
                }

                poise::builtins::register_globally(ctx, commands).await?;
                log::info!("All commands registered successfully");
                Ok(Data {})
            })
        })
        .build();

    let mut client = Client::builder(&token, intents)
        .framework(framework)
        .await
        .expect("Error creating client");

    // Start the bot
    log::info!("Starting bot...");
    if let Err(why) = client.start().await {
        log::error!("Client error: {:?}", why);
    }
}
