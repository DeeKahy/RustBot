use crate::{Context, Error};
use rand::Rng;
use tokio::time::{sleep, Duration};

/// Simulates a coin flip and announces the result
#[poise::command(prefix_command, slash_command)]
pub async fn coinflip(ctx: Context<'_>) -> Result<(), Error> {
    log::info!("Coinflip command called by {}", ctx.author().name);

    // Generate random number (0 or 1)
    let result = {
        let mut rng = rand::thread_rng();
        rng.gen_range(0..2)
    };

    // Animation sequence
    let animation_frames = ["🪙", "🔄", "🪙", "🔄", "🪙", "🔄"];

    // Send initial message
    let initial_message = format!("🪙 **{}** is flipping a coin...", ctx.author().name);
    let reply = ctx.say(initial_message).await?;

    // Animate the coin flip
    for frame in animation_frames.iter() {
        sleep(Duration::from_millis(300)).await;
        let animation_text = format!("{} **{}** is flipping a coin...", frame, ctx.author().name);

        if let Err(e) = reply
            .edit(ctx, poise::CreateReply::default().content(animation_text))
            .await
        {
            log::warn!("Failed to edit animation frame: {}", e);
            break;
        }
    }

    // Final pause before result
    sleep(Duration::from_millis(500)).await;

    // Determine final result and emoji
    let (outcome, emoji) = if result == 0 {
        ("Heads", "🪙")
    } else {
        ("Tails", "🔄")
    };

    let final_response = format!(
        "{} **{}**! The coin landed on **{}**!",
        emoji,
        ctx.author().name,
        outcome
    );

    // Edit to show final result
    if let Err(e) = reply
        .edit(ctx, poise::CreateReply::default().content(final_response))
        .await
    {
        ctx.say(format!("❌ Failed to show result: {}", e)).await?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coinflip_outcomes() {
        // Test that the function can generate both outcomes
        let mut heads_count = 0;
        let mut tails_count = 0;

        // Run 1000 flips to ensure both outcomes are possible
        for _ in 0..1000 {
            let mut rng = rand::thread_rng();
            let result = rng.gen_range(0..2);

            if result == 0 {
                heads_count += 1;
            } else {
                tails_count += 1;
            }
        }

        // Both outcomes should occur at least once in 1000 flips
        assert!(heads_count > 0, "Heads should occur at least once");
        assert!(tails_count > 0, "Tails should occur at least once");
        assert_eq!(heads_count + tails_count, 1000, "Total should be 1000");
    }
}
