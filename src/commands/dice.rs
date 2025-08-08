use crate::{Context, Error};
use rand::Rng;
use tokio::time::{sleep, Duration};

/// Roll a dice with specified number of sides (defaults to 6)
#[poise::command(prefix_command, slash_command)]
pub async fn dice(
    ctx: Context<'_>,
    #[description = "Number of sides on the dice (1-1000, defaults to 6)"] sides: Option<u32>,
) -> Result<(), Error> {
    let sides = sides.unwrap_or(6);

    log::info!(
        "Dice command called by {} with {} sides",
        ctx.author().name,
        sides
    );

    // Validate the number of sides
    if sides < 1 {
        ctx.say("❌ A dice must have at least 1 side!").await?;
        return Ok(());
    }

    if sides > 1000 {
        ctx.say("❌ That's way too many sides! Please use a number between 1 and 1000.")
            .await?;
        return Ok(());
    }

    // Generate random number between 1 and sides (inclusive)
    let result = {
        let mut rng = rand::thread_rng();
        rng.gen_range(1..=sides)
    };

    // Choose emoji based on result and sides
    let dice_emoji = if sides <= 6 {
        match result {
            1 => "⚀",
            2 => "⚁",
            3 => "⚂",
            4 => "⚃",
            5 => "⚄",
            6 => "⚅",
            _ => "🎲",
        }
    } else {
        "🎲"
    };

    // Animation sequence for dice rolling
    let animation_frames = ["🎲", "🔄", "🎲", "🔄", "🎲"];

    // Send initial message
    let initial_message = format!(
        "🎲 **{}** is rolling a {}-sided dice...",
        ctx.author().name,
        sides
    );
    let reply = ctx.say(initial_message).await?;

    // Animate the dice roll
    for frame in animation_frames.iter() {
        sleep(Duration::from_millis(250)).await;
        let animation_text = format!(
            "{} **{}** is rolling a {}-sided dice...",
            frame,
            ctx.author().name,
            sides
        );

        if let Err(e) = reply
            .edit(ctx, poise::CreateReply::default().content(animation_text))
            .await
        {
            log::warn!("Failed to edit animation frame: {e}");
            break;
        }
    }

    // Final pause before result
    sleep(Duration::from_millis(400)).await;

    // Create the final result message
    let final_response = if sides == 6 && result <= 6 {
        format!(
            "{} **{}** rolled a **{}** on a {}-sided dice!",
            dice_emoji,
            ctx.author().name,
            result,
            sides
        )
    } else {
        format!(
            "🎲 **{}** rolled a **{}** on a {}-sided dice!",
            ctx.author().name,
            result,
            sides
        )
    };

    // Edit to show final result
    if let Err(e) = reply
        .edit(ctx, poise::CreateReply::default().content(final_response))
        .await
    {
        ctx.say(format!("❌ Failed to show result: {e}")).await?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dice_roll_range() {
        // Test that dice rolls are within the expected range
        let mut rng = rand::thread_rng();

        // Test standard 6-sided die
        for _ in 0..100 {
            let result = rng.gen_range(1..=6);
            assert!(
                result >= 1 && result <= 6,
                "6-sided die result {} out of range",
                result
            );
        }

        // Test 20-sided die
        for _ in 0..100 {
            let result = rng.gen_range(1..=20);
            assert!(
                result >= 1 && result <= 20,
                "20-sided die result {} out of range",
                result
            );
        }

        // Test 1-sided die (always 1)
        for _ in 0..10 {
            let result = rng.gen_range(1..=1);
            assert_eq!(result, 1, "1-sided die should always be 1");
        }
    }

    #[test]
    fn test_dice_roll_distribution() {
        // Test that all outcomes are possible for a 6-sided die
        let mut rng = rand::thread_rng();
        let mut outcomes = [false; 6];

        // Roll many times to ensure all outcomes occur
        for _ in 0..1000 {
            let result = rng.gen_range(1..=6);
            outcomes[result - 1] = true;
        }

        // All outcomes should have occurred at least once
        for (i, &occurred) in outcomes.iter().enumerate() {
            assert!(
                occurred,
                "Outcome {} should have occurred at least once",
                i + 1
            );
        }
    }
}
