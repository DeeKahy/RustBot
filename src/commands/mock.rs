use crate::{Context, Error};

/// Transform text into mocking alternating case
fn mockify(text: &str) -> String {
    let mut result = String::new();
    let mut is_uppercase = false; // Start with lowercase for first letter

    for ch in text.chars() {
        if ch.is_alphabetic() {
            if is_uppercase {
                result.push(ch.to_uppercase().next().unwrap_or(ch));
            } else {
                result.push(ch.to_lowercase().next().unwrap_or(ch));
            }
            is_uppercase = !is_uppercase; // Alternate for next letter
        } else {
            result.push(ch); // Preserve non-alphabetic characters
        }
    }

    result
}

/// Transform text into mocking alternating case, or reply to a message to mock it
#[poise::command(prefix_command, slash_command)]
pub async fn mock(
    ctx: Context<'_>,
    #[description = "Text to mock (leave empty to mock replied message)"]
    #[rest]
    text: Option<String>,
) -> Result<(), Error> {
    log::info!("Mock command called by {}", ctx.author().name);

    let (text_to_mock, is_reply) = if let Some(text) = text {
        // User provided text directly
        (text, false)
    } else {
        // Check if this is a prefix command with a replied message
        match ctx {
            poise::Context::Prefix(prefix_ctx) => {
                if let Some(replied_message) = prefix_ctx.msg.referenced_message.as_ref() {
                    // Delete the invoker's message when replying to another message
                    if let Err(e) = prefix_ctx.msg.delete(&ctx.http()).await {
                        log::warn!("Failed to delete invoker's message: {}", e);
                    }
                    (replied_message.content.clone(), true)
                } else {
                    ctx.say(
                        "❌ Please provide text to mock or reply to a message with this command!",
                    )
                    .await?;
                    return Ok(());
                }
            }
            _ => {
                ctx.say("❌ Please provide text to mock! (Message replies only work with prefix commands)")
                    .await?;
                return Ok(());
            }
        }
    };

    if text_to_mock.trim().is_empty() {
        ctx.say("❌ Cannot mock empty text!").await?;
        return Ok(());
    }

    let mocked = mockify(&text_to_mock);

    // Create response with user mention if replying to someone else's message
    let response = if is_reply {
        match ctx {
            poise::Context::Prefix(prefix_ctx) => {
                if let Some(replied_message) = prefix_ctx.msg.referenced_message.as_ref() {
                    if replied_message.author.id != ctx.author().id {
                        format!("*{} says:*\n{}", replied_message.author.name, mocked)
                    } else {
                        mocked
                    }
                } else {
                    mocked
                }
            }
            _ => mocked,
        }
    } else {
        mocked
    };

    if let Err(e) = ctx.say(response).await {
        ctx.say(format!("❌ {e}")).await?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mockify_basic() {
        let input = "hello world";
        let result = mockify(input);
        assert_eq!(result, "hElLo WoRlD");
    }

    #[test]
    fn test_mockify_with_punctuation() {
        let input = "Hello, World!";
        let result = mockify(input);
        assert_eq!(result, "hElLo, WoRlD!");
    }

    #[test]
    fn test_mockify_mixed_case() {
        let input = "ThIs Is A tEsT";
        let result = mockify(input);
        // Should follow alternating pattern regardless of input case
        assert_eq!(result, "tHiS iS a TeSt");
    }

    #[test]
    fn test_mockify_numbers_and_symbols() {
        let input = "test123!@#test";
        let result = mockify(input);
        assert_eq!(result, "tEsT123!@#tEsT");
    }

    #[test]
    fn test_mockify_empty_string() {
        let result = mockify("");
        assert_eq!(result, "");
    }

    #[test]
    fn test_mockify_only_punctuation() {
        let input = "!@#$%^&*()";
        let result = mockify(input);
        assert_eq!(result, "!@#$%^&*()");
    }

    #[test]
    fn test_mockify_preserves_spaces() {
        let input = "a b c d e";
        let result = mockify(input);
        assert_eq!(result, "a B c D e");
    }
}
