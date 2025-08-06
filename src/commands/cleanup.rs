use crate::{Context, Error};
use poise::serenity_prelude as serenity;
use tokio::time::{sleep, Duration};

/// Delete messages in the current channel
#[poise::command(prefix_command, slash_command)]
pub async fn cleanup(
    ctx: Context<'_>,
    #[description = "Number of messages to delete"] count: Option<u64>,
    #[description = "Delete all messages after the replied message"] after: Option<bool>,
) -> Result<(), Error> {
    log::info!(
        "Cleanup command called by {} with count: {:?}, after: {:?}",
        ctx.author().name,
        count,
        after
    );

    // Check if user has manage messages permission
    let permissions = ctx.author_permissions().await;
    match permissions {
        Ok(perms) if !perms.manage_messages() => {
            ctx.say("❌ You don't have permission to delete messages! You need the 'Manage Messages' permission.")
                .await?;
            return Ok(());
        }
        Err(_) => {
            ctx.say("❌ Could not check your permissions. Make sure this command is used in a server channel.")
                .await?;
            return Ok(());
        }
        _ => {}
    }

    // Check if we're in a guild (server) and not in DMs
    let Some(_guild_id) = ctx.guild_id() else {
        ctx.say("❌ This command can only be used in servers, not in DMs!")
            .await?;
        return Ok(());
    };

    let channel_id = ctx.channel_id();

    // Handle "after" mode - delete messages after the replied message
    if after.unwrap_or(false) {
        let Some(replied_msg) = ctx.msg().referenced_message.as_ref() else {
            ctx.say("❌ You must reply to a message when using the `after` option!")
                .await?;
            return Ok(());
        };

        let after_id = replied_msg.id;

        ctx.say("🧹 Starting cleanup after the specified message... This may take a while to avoid rate limits.")
            .await?;

        let mut deleted_count = 0u64;
        let mut last_message_id = None;

        loop {
            // Fetch messages after the specified message
            let mut get_messages = serenity::GetMessages::new().limit(100);

            if let Some(last_id) = last_message_id {
                get_messages = get_messages.before(last_id);
            }

            let messages = match channel_id
                .messages(&ctx.serenity_context().http, get_messages)
                .await
            {
                Ok(msgs) => msgs,
                Err(e) => {
                    ctx.say(format!("❌ Error fetching messages: {}", e))
                        .await?;
                    return Ok(());
                }
            };

            if messages.is_empty() {
                break;
            }

            // Filter messages that are after the replied message
            let messages_to_delete: Vec<_> = messages
                .into_iter()
                .filter(|msg| msg.id > after_id)
                .collect();

            if messages_to_delete.is_empty() {
                break;
            }

            // Update last_message_id for pagination
            last_message_id = messages_to_delete.last().map(|msg| msg.id);

            // Delete messages in batches of 100 (Discord's limit)
            for chunk in messages_to_delete.chunks(100) {
                let message_ids: Vec<serenity::MessageId> =
                    chunk.iter().map(|msg| msg.id).collect();

                if message_ids.len() == 1 {
                    // Single message deletion
                    if let Err(e) = channel_id
                        .delete_message(&ctx.serenity_context().http, message_ids[0])
                        .await
                    {
                        log::warn!("Failed to delete message {}: {}", message_ids[0], e);
                        continue;
                    }
                } else {
                    // Bulk deletion for multiple messages
                    if let Err(e) = channel_id
                        .delete_messages(&ctx.serenity_context().http, &message_ids)
                        .await
                    {
                        log::warn!("Failed to bulk delete messages: {}", e);
                        // Try individual deletion as fallback
                        for msg_id in message_ids {
                            if let Err(e) = channel_id
                                .delete_message(&ctx.serenity_context().http, msg_id)
                                .await
                            {
                                log::warn!(
                                    "Failed to delete message {} individually: {}",
                                    msg_id,
                                    e
                                );
                            } else {
                                deleted_count += 1;
                            }
                            // Small delay to avoid rate limits
                            sleep(Duration::from_millis(100)).await;
                        }
                        continue;
                    }
                }

                deleted_count += message_ids.len() as u64;

                // Rate limit protection - wait between batches
                sleep(Duration::from_millis(500)).await;
            }
        }

        ctx.say(format!(
            "✅ Cleanup complete! Deleted {} messages after the specified message.",
            deleted_count
        ))
        .await?;

        return Ok(());
    }

    // Handle count mode - delete the last X messages
    let delete_count = count.unwrap_or(10);

    if delete_count == 0 {
        ctx.say("❌ Count must be greater than 0!").await?;
        return Ok(());
    }

    if delete_count > 1000 {
        ctx.say("❌ Cannot delete more than 1000 messages at once!")
            .await?;
        return Ok(());
    }

    ctx.say(format!(
        "🧹 Starting cleanup of {} messages... This may take a while to avoid rate limits.",
        delete_count
    ))
    .await?;

    let mut deleted_count = 0u64;
    let mut remaining = delete_count;

    while remaining > 0 {
        let batch_size = std::cmp::min(remaining, 100);

        // Fetch messages
        let messages = match channel_id
            .messages(
                &ctx.serenity_context().http,
                serenity::GetMessages::new().limit(batch_size as u8),
            )
            .await
        {
            Ok(msgs) => msgs,
            Err(e) => {
                ctx.say(format!("❌ Error fetching messages: {}", e))
                    .await?;
                return Ok(());
            }
        };

        if messages.is_empty() {
            break;
        }

        // Collect message IDs
        let message_ids: Vec<serenity::MessageId> = messages.iter().map(|msg| msg.id).collect();

        if message_ids.len() == 1 {
            // Single message deletion
            if let Err(e) = channel_id
                .delete_message(&ctx.serenity_context().http, message_ids[0])
                .await
            {
                log::warn!("Failed to delete message {}: {}", message_ids[0], e);
            } else {
                deleted_count += 1;
            }
        } else {
            // Bulk deletion for multiple messages
            if let Err(e) = channel_id
                .delete_messages(&ctx.serenity_context().http, &message_ids)
                .await
            {
                log::warn!("Failed to bulk delete messages: {}", e);
                // Try individual deletion as fallback
                for msg_id in message_ids {
                    if let Err(e) = channel_id
                        .delete_message(&ctx.serenity_context().http, msg_id)
                        .await
                    {
                        log::warn!("Failed to delete message {} individually: {}", msg_id, e);
                    } else {
                        deleted_count += 1;
                    }
                    // Small delay to avoid rate limits
                    sleep(Duration::from_millis(100)).await;
                }
            } else {
                deleted_count += message_ids.len() as u64;
            }
        }

        remaining = remaining.saturating_sub(message_ids.len() as u64);

        // Rate limit protection - wait between batches
        sleep(Duration::from_millis(1000)).await; // 1 second delay to be safe
    }

    ctx.say(format!(
        "✅ Cleanup complete! Deleted {} messages.",
        deleted_count
    ))
    .await?;

    Ok(())
}
