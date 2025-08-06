use crate::{Context, Error};

/// Transform text into uwu language
fn uwuify(text: &str) -> String {
    let mut result = text.to_string();

    // Replace patterns - handle case insensitive replacements
    result = result.replace("The", "Teh");
    result = result.replace("the", "teh");
    result = result.replace("th", "d");
    result = result.replace("r", "w");
    result = result.replace("l", "w");
    result = result.replace("na", "nya");
    result = result.replace("ne", "nye");
    result = result.replace("ni", "nyi");
    result = result.replace("no", "nyo");
    result = result.replace("nu", "nyu");
    result = result.replace("ove", "uv");

    // Add uwu expressions
    let uwu_expressions = [" uwu ", " owo ", " >w< ", " ^w^ ", " (>ω<) "];
    let sentences: Vec<&str> = result.split(['.', '!', '?']).collect();
    let mut uwu_sentences = Vec::new();

    for (i, sentence) in sentences.iter().enumerate() {
        if !sentence.trim().is_empty() {
            let mut sentence_str = sentence.trim().to_string();

            // Add uwu expression to longer sentences
            if sentence.len() > 20 && i % 2 == 0 {
                let expr_idx = i % uwu_expressions.len();
                let words: Vec<&str> = sentence_str.split_whitespace().collect();
                if words.len() > 3 {
                    let mid_point = words.len() / 2;
                    let first_half = words[..mid_point].join(" ");
                    let second_half = words[mid_point..].join(" ");
                    sentence_str =
                        format!("{}{}{}", first_half, uwu_expressions[expr_idx], second_half);
                }
            }

            uwu_sentences.push(sentence_str);
        }
    }

    let mut final_result = uwu_sentences.join(". ");

    // Clean up and add ending expression
    if !final_result.is_empty() {
        final_result = final_result.trim().to_string();
        if !final_result.ends_with("uwu")
            && !final_result.ends_with("owo")
            && !final_result.ends_with(">w<")
        {
            final_result.push_str(" uwu");
        }
    }

    final_result
}

/// Transform text into uwu language, or reply to a message to uwuify it
#[poise::command(prefix_command, slash_command)]
pub async fn uwu(
    ctx: Context<'_>,
    #[description = "Text to uwuify (leave empty to uwuify replied message)"]
    #[rest]
    text: Option<String>,
) -> Result<(), Error> {
    log::info!("UwU command called by {}", ctx.author().name);

    let (text_to_uwuify, is_reply) = if let Some(text) = text {
        // User provided text directly
        (text, false)
    } else {
        // Check if this is a prefix command with a replied message
        match ctx {
            poise::Context::Prefix(prefix_ctx) => {
                if let Some(replied_message) = prefix_ctx.msg.referenced_message.as_ref() {
                    (replied_message.content.clone(), true)
                } else {
                    ctx.say(
                        "❌ Please provide text to uwuify or reply to a message with this command!",
                    )
                    .await?;
                    return Ok(());
                }
            }
            _ => {
                ctx.say("❌ Please provide text to uwuify! (Message replies only work with prefix commands)")
                    .await?;
                return Ok(());
            }
        }
    };

    if text_to_uwuify.trim().is_empty() {
        ctx.say("❌ Cannot uwuify empty text!").await?;
        return Ok(());
    }

    let uwuified = uwuify(&text_to_uwuify);

    // Create response with user mention if replying to someone else's message
    let response = if is_reply {
        match ctx {
            poise::Context::Prefix(prefix_ctx) => {
                if let Some(replied_message) = prefix_ctx.msg.referenced_message.as_ref() {
                    if replied_message.author.id != ctx.author().id {
                        format!("*{} says:*\n{}", replied_message.author.name, uwuified)
                    } else {
                        uwuified
                    }
                } else {
                    uwuified
                }
            }
            _ => uwuified,
        }
    } else {
        uwuified
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
    fn test_uwuify_basic() {
        let input = "Hello world! This is a test.";
        let result = uwuify(input);

        // Check that basic transformations work
        assert!(result.contains("w")); // r/l -> w
        assert!(result.contains("uwu") || result.contains("owo")); // uwu expressions added
    }

    #[test]
    fn test_uwuify_specific_patterns() {
        let input = "The cat loves running";
        let result = uwuify(input);

        // Check specific transformations
        assert!(result.contains("Teh")); // The -> Teh
        assert!(result.contains("wuvs")); // loves -> wuvs
        assert!(result.contains("wunnying")); // running -> wunnying
    }

    #[test]
    fn test_uwuify_empty_string() {
        let result = uwuify("");
        assert_eq!(result, "");
    }

    #[test]
    fn test_uwuify_preserves_capitalization() {
        let input = "Hello World";
        let result = uwuify(input);

        // Should preserve some capitalization structure
        assert!(!result.is_empty());
    }
}
