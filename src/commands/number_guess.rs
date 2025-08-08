use crate::{Context, Error};

use rand::Rng;
use std::collections::HashMap;

use tokio::sync::RwLock;

// Global storage for active games
lazy_static::lazy_static! {
    static ref ACTIVE_GAMES: RwLock<HashMap<u64, NumberGame>> = RwLock::new(HashMap::new());
}

#[derive(Clone)]
struct NumberGame {
    secret_number: u32,
    attempts: u32,
    min: u32,
    max: u32,
}

impl NumberGame {
    fn new(min: u32, max: u32) -> Self {
        let mut rng = rand::thread_rng();
        let secret_number = rng.gen_range(min..=max);

        NumberGame {
            secret_number,
            attempts: 0,
            min,
            max,
        }
    }

    fn make_guess(&mut self, guess: u32) -> GuessResult {
        self.attempts += 1;

        if !(self.min..=self.max).contains(&guess) {
            return GuessResult::OutOfRange;
        }

        match guess.cmp(&self.secret_number) {
            std::cmp::Ordering::Equal => GuessResult::Correct,
            std::cmp::Ordering::Less => {
                let diff = self.secret_number - guess;
                GuessResult::TooLow(self.get_hint(diff))
            }
            std::cmp::Ordering::Greater => {
                let diff = guess - self.secret_number;
                GuessResult::TooHigh(self.get_hint(diff))
            }
        }
    }

    fn get_hint(&self, difference: u32) -> String {
        let range = self.max - self.min;
        let ratio = (difference as f32) / (range as f32);

        if ratio > 0.5 {
            "🔥 Way off!".to_string()
        } else if ratio > 0.3 {
            "🌡️ Getting warmer...".to_string()
        } else if ratio > 0.15 {
            "🎯 Getting close!".to_string()
        } else if ratio > 0.05 {
            "🔥 Very close!".to_string()
        } else {
            "🌟 So close!".to_string()
        }
    }

    fn get_performance_rating(&self) -> String {
        let range = self.max - self.min;
        let optimal_attempts = ((range as f32).log2().ceil() as u32).max(1);

        match self.attempts {
            1 => "🏅 INCREDIBLE! First try! Are you psychic?".to_string(),
            a if a <= optimal_attempts => "🏆 EXCELLENT! Perfect strategy!".to_string(),
            a if a <= optimal_attempts + 3 => "👍 GOOD! Nice guessing!".to_string(),
            a if a <= optimal_attempts + 7 => "😊 NOT BAD! You got there!".to_string(),
            _ => "🤔 Keep practicing!".to_string(),
        }
    }
}

enum GuessResult {
    Correct,
    TooLow(String),
    TooHigh(String),
    OutOfRange,
}

/// Start a number guessing game! Guess the number between 1-100 (or custom range)
#[poise::command(prefix_command, slash_command)]
pub async fn numberguess(
    ctx: Context<'_>,
    #[description = "Minimum number (default: 1)"] min: Option<u32>,
    #[description = "Maximum number (default: 100)"] max: Option<u32>,
) -> Result<(), Error> {
    let user_id = ctx.author().id.get();

    let min = min.unwrap_or(1);
    let max = max.unwrap_or(100);

    if min >= max {
        ctx.say("❌ Minimum must be less than maximum!").await?;
        return Ok(());
    }

    if max - min > 10000 {
        ctx.say("❌ Range too large! Maximum range is 10,000 numbers.")
            .await?;
        return Ok(());
    }

    // Check if user already has an active game
    {
        let games = ACTIVE_GAMES.read().await;
        if games.contains_key(&user_id) {
            ctx.say("❌ You already have an active number guessing game! Use `/guess <number>` to make a guess or `/endgame` to quit.").await?;
            return Ok(());
        }
    }

    // Create new game
    let game = NumberGame::new(min, max);

    {
        let mut games = ACTIVE_GAMES.write().await;
        games.insert(user_id, game);
    }

    let response = format!(
        "🎯 **Number Guessing Game Started!**\n\
        I'm thinking of a number between **{}** and **{}**\n\
        Use `/guess <number>` to make your guess!\n\
        Use `/hint` for a hint or `/endgame` to quit\n\n\
        Good luck, {}! 🍀",
        min,
        max,
        ctx.author().name
    );

    ctx.say(response).await?;
    Ok(())
}

/// Make a guess in your active number guessing game
#[poise::command(prefix_command, slash_command)]
pub async fn guess(
    ctx: Context<'_>,
    #[description = "Your number guess"] number: u32,
) -> Result<(), Error> {
    let user_id = ctx.author().id.get();

    let mut games = ACTIVE_GAMES.write().await;

    if let Some(game) = games.get_mut(&user_id) {
        let result = game.make_guess(number);

        match result {
            GuessResult::Correct => {
                let performance = game.get_performance_rating();
                let response = format!(
                    "🎉 **CONGRATULATIONS!** 🎉\n\
                    You guessed it! The number was **{}**\n\
                    🏆 You did it in **{}** attempts!\n\
                    {}\n\n\
                    Want to play again? Use `/numberguess`!",
                    game.secret_number, game.attempts, performance
                );
                ctx.say(response).await?;
                games.remove(&user_id);
            }
            GuessResult::TooLow(hint) => {
                let response = format!(
                    "📈 **Too low!** Try a higher number.\n\
                    {}\n\
                    📊 Attempts: **{}**",
                    hint, game.attempts
                );
                ctx.say(response).await?;
            }
            GuessResult::TooHigh(hint) => {
                let response = format!(
                    "📉 **Too high!** Try a lower number.\n\
                    {}\n\
                    📊 Attempts: **{}**",
                    hint, game.attempts
                );
                ctx.say(response).await?;
            }
            GuessResult::OutOfRange => {
                ctx.say(format!(
                    "❌ **Out of range!** Please guess between **{}** and **{}**",
                    game.min, game.max
                ))
                .await?;
            }
        }
    } else {
        ctx.say("❌ You don't have an active number guessing game! Start one with `/numberguess`")
            .await?;
    }

    Ok(())
}

/// Get a hint for your active number guessing game
#[poise::command(prefix_command, slash_command)]
pub async fn hint(ctx: Context<'_>) -> Result<(), Error> {
    let user_id = ctx.author().id.get();

    let games = ACTIVE_GAMES.read().await;

    if let Some(game) = games.get(&user_id) {
        let range_size = game.max - game.min + 1;
        let optimal_attempts = ((range_size as f32).log2().ceil() as u32).max(1);

        let hint_text = format!(
            "💡 **Hint for your guessing game:**\n\
            🎯 Range: **{}** to **{}** ({} numbers total)\n\
            📊 Attempts so far: **{}**\n\
            🧠 Optimal strategy would take ~**{}** attempts\n\
            💭 Try using binary search: start in the middle!",
            game.min, game.max, range_size, game.attempts, optimal_attempts
        );

        ctx.say(hint_text).await?;
    } else {
        ctx.say("❌ You don't have an active number guessing game! Start one with `/numberguess`")
            .await?;
    }

    Ok(())
}

/// End your current number guessing game
#[poise::command(prefix_command, slash_command)]
pub async fn endgame(ctx: Context<'_>) -> Result<(), Error> {
    let user_id = ctx.author().id.get();

    let mut games = ACTIVE_GAMES.write().await;

    if let Some(game) = games.remove(&user_id) {
        let response = format!(
            "🏳️ **Game ended!**\n\
            The number was **{}**\n\
            You made **{}** attempts.\n\
            Thanks for playing! 👋",
            game.secret_number, game.attempts
        );
        ctx.say(response).await?;
    } else {
        ctx.say("❌ You don't have an active number guessing game!")
            .await?;
    }

    Ok(())
}

/// Show your current number guessing game status
#[poise::command(prefix_command, slash_command)]
pub async fn gamestatus(ctx: Context<'_>) -> Result<(), Error> {
    let user_id = ctx.author().id.get();

    let games = ACTIVE_GAMES.read().await;

    if let Some(game) = games.get(&user_id) {
        let response = format!(
            "🎮 **Your current game:**\n\
            🎯 Range: **{}** to **{}**\n\
            📊 Attempts: **{}**\n\
            🕒 Use `/guess <number>` to continue!",
            game.min, game.max, game.attempts
        );
        ctx.say(response).await?;
    } else {
        ctx.say("❌ You don't have an active number guessing game! Start one with `/numberguess`")
            .await?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_number_game_creation() {
        let game = NumberGame::new(1, 100);
        assert!(game.secret_number >= 1 && game.secret_number <= 100);
        assert_eq!(game.attempts, 0);
        assert_eq!(game.min, 1);
        assert_eq!(game.max, 100);
    }

    #[test]
    fn test_guess_correct() {
        let mut game = NumberGame::new(1, 100);
        let secret = game.secret_number;

        let result = game.make_guess(secret);
        assert!(matches!(result, GuessResult::Correct));
        assert_eq!(game.attempts, 1);
    }

    #[test]
    fn test_guess_out_of_range() {
        let mut game = NumberGame::new(10, 20);

        let result = game.make_guess(5);
        assert!(matches!(result, GuessResult::OutOfRange));

        let result = game.make_guess(25);
        assert!(matches!(result, GuessResult::OutOfRange));
    }
}
