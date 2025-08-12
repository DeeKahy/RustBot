use crate::{Context, Error};
use poise::serenity_prelude as serenity;
use serenity::Mentionable;
use tokio::time::{sleep, Duration};

/// Spam ping a user in a new thread until they respond
#[poise::command(prefix_command, slash_command)]
pub async fn spamping(
    ctx: Context<'_>,
    #[description = "User to spam ping"] user: serenity::User,
) -> Result<(), Error> {
    log::info!(
        "Spamping command called by {} for user {}",
        ctx.author().name,
        user.name
    );

    // Check if we're in a guild (server) and not in DMs
    let Some(_guild_id) = ctx.guild_id() else {
        ctx.say("❌ This command can only be used in servers, not in DMs!")
            .await?;
        return Ok(());
    };

    // Get the channel
    let channel = ctx
        .channel_id()
        .to_channel(&ctx.serenity_context().http)
        .await?;

    if let serenity::Channel::Guild(guild_channel) = channel {
        // Create a new thread
        let thread_name = format!("Spamping {} until they respond", user.name);

        let thread = match guild_channel
            .create_thread(
                &ctx.serenity_context().http,
                serenity::CreateThread::new(&thread_name)
                    .auto_archive_duration(serenity::AutoArchiveDuration::OneHour)
                    .kind(serenity::ChannelType::PublicThread),
            )
            .await
        {
            Ok(thread) => thread,
            Err(e) => {
                ctx.say(format!("❌ {e}")).await?;
                return Ok(());
            }
        };

        // Send initial message
        let initial_msg = format!(
            "**SPAM PING ACTIVATED**\n\n{}, you are being pinged every 10 seconds until you respond!\nType anything in this thread to stop the spam!",
            user.mention()
        );

        if let Err(e) = thread.say(&ctx.serenity_context().http, initial_msg).await {
            ctx.say(format!("❌ {e}")).await?;
            return Ok(());
        }

        // Confirm to the user who started it
        ctx.say(format!(
            "Spam ping started for {} in {}! They will be pinged every 10 seconds until they respond.",
            user.mention(),
            thread.mention()
        )).await?;

        // Clone necessary data for the spawned task
        let http = ctx.serenity_context().http.clone();
        let thread_id = thread.id;
        let user_id = user.id;
        let user_mention = user.mention().to_string();

        // Spawn the spam ping task
        tokio::spawn(async move {
            let mut ping_count = 1;

            loop {
                sleep(Duration::from_secs(10)).await;

                // Check if there are new messages from the target user in the thread
                if let Ok(messages) = thread_id
                    .messages(&http, serenity::GetMessages::new().limit(50))
                    .await
                {
                    // Check if the user has sent any messages in the last minute
                    let user_responded = messages.iter().any(|msg| {
                        msg.author.id == user_id
                            && msg.timestamp.timestamp() > (chrono::Utc::now().timestamp() - 60)
                    });

                    if user_responded {
                        let _ = thread_id
                            .say(
                                &http,
                                format!("{user_mention} responded! Spam ping stopped after {ping_count} pings. Thread will be deleted in 5 seconds..."),
                            )
                            .await;

                        // Wait 5 seconds to let people see the final message
                        sleep(Duration::from_secs(5)).await;

                        // Delete the thread
                        let _ = thread_id.delete(&http).await;
                        break;
                    }
                }

                // Send the ping
                ping_count += 1;
                let ping_message = match ping_count {
                    1..=5 => format!("Ping #{ping_count}: {user_mention} - Please respond!"),
                    6..=10 => format!(
                        "Ping #{ping_count}: {user_mention} - HELLO?! Are you there?"
                    ),
                    11..=15 => format!(
                        "Ping #{ping_count}: {user_mention} - EMERGENCY PING! RESPOND NOW!"
                    ),
                    16..=20 => format!(
                        "Ping #{ping_count}: {user_mention} - Are you still alive?! RESPOND!"
                    ),
                    _ => format!(
                        "Ping #{ping_count}: {user_mention} - This is getting ridiculous... please respond!"
                    ),
                };

                if let Err(e) = thread_id.say(&http, ping_message).await {
                    let _ = thread_id.say(&http, format!("❌ {e}")).await;
                    break;
                }

                // Stop after 50 pings (about 8 minutes) to prevent infinite spam
                if ping_count >= 50 {
                    let _ = thread_id
                        .say(
                            &http,
                            format!("Spam ping stopped after 50 attempts. {user_mention} might be AFK or ignoring us..."),
                        )
                        .await;
                    break;
                }
            }
        });
    } else {
        ctx.say("❌ This command can only be used in server channels!")
            .await?;
    }

    Ok(())
}
