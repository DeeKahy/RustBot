use crate::{Context, Error};
use poise::serenity_prelude as serenity;
use serenity::ReactionType;
use std::collections::HashMap;

/// React to a message with emoji letters
#[poise::command(prefix_command, slash_command)]
pub async fn react(
    ctx: Context<'_>,
    #[description = "Text to react with (e.g., 'lol', 'cool')"] text: String,
    #[description = "Message ID to react to (for slash commands)"] message_id: Option<String>,
) -> Result<(), Error> {
    log::info!(
        "React command called by {} with text: '{}'",
        ctx.author().name,
        text
    );

    if text.trim().is_empty() {
        ctx.say("❌ Please provide text to react with! Example: `-react lol`")
            .await?;
        return Ok(());
    }

    // Delete the invoker's message if it's a prefix command
    if let poise::Context::Prefix(prefix_ctx) = &ctx {
        if let Err(e) = prefix_ctx.msg.delete(&ctx.http()).await {
            log::warn!("Failed to delete invoker's message: {}", e);
        }
    }

    // Get the message to react to
    let replied_message = match ctx {
        poise::Context::Prefix(prefix_ctx) => {
            if let Some(referenced_msg) = &prefix_ctx.msg.referenced_message {
                referenced_msg.as_ref().clone()
            } else {
                ctx.say("❌ Please reply to a message to react to it!")
                    .await?;
                return Ok(());
            }
        }
        poise::Context::Application(_) => {
            // For slash commands, require message_id parameter
            let msg_id_str = message_id
                .ok_or("❌ For slash commands, please provide the message ID to react to!")?;

            let msg_id = msg_id_str.parse::<u64>().map_err(|_| {
                "❌ Invalid message ID format! Please provide a valid Discord message ID."
            })?;

            match ctx.channel_id().message(&ctx.http(), msg_id).await {
                Ok(msg) => msg,
                Err(_) => {
                    ctx.say("❌ Could not find a message with that ID in this channel!")
                        .await?;
                    return Ok(());
                }
            }
        }
    };

    // Create emoji mapping with fallbacks
    let emoji_map = create_emoji_mapping();

    let text_lower = text.to_lowercase();
    let mut used_emojis = std::collections::HashSet::new();
    let mut reactions_added = 0;

    // Process each character
    for ch in text_lower.chars() {
        if let Some(emoji_options) = emoji_map.get(&ch) {
            // Find the first unused emoji for this character
            let mut emoji_to_use = None;
            for emoji in emoji_options {
                if !used_emojis.contains(emoji) {
                    emoji_to_use = Some(emoji);
                    break;
                }
            }

            if let Some(emoji) = emoji_to_use {
                // Try to add the reaction
                match replied_message
                    .react(&ctx.http(), ReactionType::Unicode(emoji.to_string()))
                    .await
                {
                    Ok(_) => {
                        used_emojis.insert(emoji);
                        reactions_added += 1;
                        log::debug!("Added reaction {emoji} for character '{ch}'");

                        // Small delay to avoid rate limiting
                        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                    }
                    Err(e) => {
                        log::warn!("Failed to add reaction {emoji} for character '{ch}': {e}");
                    }
                }
            } else {
                log::debug!("No available emoji for character '{ch}' (all used)");
            }
        } else {
            log::debug!("No emoji mapping for character '{ch}'");
        }
    }

    // Only send error messages for slash commands or if no reactions were added
    if reactions_added == 0 {
        if let poise::Context::Application(_) = &ctx {
            ctx.say(
                "❌ Couldn't add any reactions. The emojis might already be used or unavailable.",
            )
            .await?;
        }
    }

    log::info!("React command completed. Added {reactions_added} reactions");
    Ok(())
}

fn create_emoji_mapping() -> HashMap<char, Vec<&'static str>> {
    let mut map = HashMap::new();

    // Letters with regional indicators as primary option, then fallbacks
    map.insert('a', vec!["🇦", "🅰️", "🔺", "🅰", "4️⃣"]);
    map.insert('b', vec!["🇧", "🅱️", "🅱", "6️⃣", "🪨"]);
    map.insert('c', vec!["🇨", "©️", "🌙", "☪️", "🥐"]);
    map.insert('d', vec!["🇩", "↩️", "🌛", "🌜", "🎯"]);
    map.insert('e', vec!["🇪", "3️⃣", "💶", "📧", "🔱"]);
    map.insert('f', vec!["🇫", "🎏", "🪦", "📠", "🔥"]);
    map.insert('g', vec!["🇬", "🔄", "🌀", "🎯", "⚙️"]);
    map.insert('h', vec!["🇭", "🏨", "🏥", "🏡", "♓"]);
    map.insert('i', vec!["🇮", "ℹ️", "1️⃣", "🍦", "🧊"]);
    map.insert('j', vec!["🇯", "🎷", "🗾", "🪝", "🕺"]);
    map.insert('k', vec!["🇰", "🎋", "🦘", "🥝", "🔑"]);
    map.insert('l', vec!["🇱", "🇮", "1️⃣", "🏩", "📱"]); // L is tricky, use I as fallback
    map.insert('m', vec!["🇲", "Ⓜ️", "〽️", "🎵", "🗻"]);
    map.insert('n', vec!["🇳", "📈", "🎵", "🌃", "♑"]);
    map.insert(
        'o',
        vec!["🇴", "⭕", "🅾️", "🅾", "0️⃣", "🔴", "🟡", "🟢", "🔵", "🟣"],
    );
    map.insert('p', vec!["🇵", "🅿️", "🅿", "🪩", "📌"]);
    map.insert('q', vec!["🇶", "🎯", "🔍", "❓", "🪙"]);
    map.insert('r', vec!["🇷", "®️", "🚀", "🌈", "♻️"]);
    map.insert('s', vec!["🇸", "💲", "5️⃣", "🐍", "⚡"]);
    map.insert('t', vec!["🇹", "✝️", "🌴", "🍵", "📐"]);
    map.insert('u', vec!["🇺", "⛎", "🔄", "🌙", "⚓"]);
    map.insert('v', vec!["🇻", "✅", "♈", "🎭", "🔽"]);
    map.insert('w', vec!["🇼", "〰️", "🤷", "🌊", "💧"]);
    map.insert('x', vec!["🇽", "❌", "✖️", "❎", "🔀"]);
    map.insert('y', vec!["🇾", "💴", "🧘", "☯️", "🌟"]);
    map.insert('z', vec!["🇿", "💤", "⚡", "🦓", "0️⃣"]);

    // Numbers
    map.insert('0', vec!["0️⃣", "⭕", "🅾️", "🔴"]);
    map.insert('1', vec!["1️⃣", "🇮", "ℹ️", "🥇"]);
    map.insert('2', vec!["2️⃣", "🦢", "🥈", "🪝"]);
    map.insert('3', vec!["3️⃣", "🇪", "🥉", "🔱"]);
    map.insert('4', vec!["4️⃣", "🇦", "🍀", "🔲"]);
    map.insert('5', vec!["5️⃣", "🇸", "🖐️", "⭐"]);
    map.insert('6', vec!["6️⃣", "🇧", "🎯", "🔯"]);
    map.insert('7', vec!["7️⃣", "🎰", "🔧", "📐"]);
    map.insert('8', vec!["8️⃣", "♾️", "🎱", "⚡"]);
    map.insert('9', vec!["9️⃣", "🌀", "🎯", "🔄"]);

    // Special characters and punctuation
    map.insert(' ', vec!["⬜", "▫️", "⚪"]);
    map.insert('!', vec!["❗", "❕", "‼️", "⚠️"]);
    map.insert('?', vec!["❓", "❔", "🤔", "🔍"]);
    map.insert('.', vec!["🔸", "🔹", "⚫", "⚪"]);
    map.insert(',', vec!["〰️", "💧", "🌊", "🔸"]);
    map.insert(';', vec!["😉", "😏", "🔸", "💧"]);
    map.insert(':', vec!["😮", "😯", "⚫", "🔸"]);
    map.insert('(', vec!["◀️", "🌙", "🌛", "⚫"]);
    map.insert(')', vec!["▶️", "🌛", "🌜", "⚫"]);
    map.insert('-', vec!["➖", "〰️", "💨", "⚫"]);
    map.insert('+', vec!["➕", "✅", "🔄", "⚡"]);
    map.insert('=', vec!["🟰", "➖", "〰️", "⚫"]);
    map.insert('*', vec!["⭐", "✨", "🌟", "💫"]);
    map.insert('/', vec!["〰️", "💨", "⚡", "🔸"]);
    map.insert('\\', vec!["〰️", "💨", "⚡", "🔹"]);
    map.insert('&', vec!["🤝", "⚡", "🔗", "💫"]);
    map.insert('#', vec!["#️⃣", "🔲", "⚫", "🔸"]);
    map.insert('@', vec!["🇦", "🅰️", "⚫", "🔘"]);

    map
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_emoji_mapping_exists() {
        let map = create_emoji_mapping();

        // Test that basic letters exist
        assert!(map.contains_key(&'a'));
        assert!(map.contains_key(&'z'));
        assert!(map.contains_key(&'l'));

        // Test that numbers exist
        assert!(map.contains_key(&'0'));
        assert!(map.contains_key(&'9'));

        // Test that 'l' has fallbacks including I
        let l_options = map.get(&'l').unwrap();
        assert!(l_options.contains(&"🇮")); // I as fallback for L
        assert!(l_options.len() > 1); // Multiple fallbacks
    }

    #[test]
    fn test_duplicate_handling() {
        let map = create_emoji_mapping();

        // Test that 'o' has many options for handling duplicates
        let o_options = map.get(&'o').unwrap();
        assert!(o_options.len() >= 5); // Should have multiple circle-like emojis
    }
}
