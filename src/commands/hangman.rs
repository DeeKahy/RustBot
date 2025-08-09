use crate::{Context, Error};

use rand::Rng;
use std::collections::{HashMap, HashSet};
use tokio::sync::RwLock;

// Global storage for active games
lazy_static::lazy_static! {
    static ref ACTIVE_GAMES: RwLock<HashMap<u64, HangmanGame>> = RwLock::new(HashMap::new());
}

#[derive(Clone)]
struct HangmanGame {
    word: String,
    guessed_letters: HashSet<char>,
    wrong_guesses: Vec<char>,
    max_wrong_guesses: usize,
    category: String,
}

impl HangmanGame {
    fn new() -> Self {
        let (word, category) = Self::get_random_word();
        HangmanGame {
            word: word.to_uppercase(),
            guessed_letters: HashSet::new(),
            wrong_guesses: Vec::new(),
            max_wrong_guesses: 6,
            category: category.to_string(),
        }
    }

    fn new_with_word(word: &str, category: &str) -> Self {
        HangmanGame {
            word: word.to_uppercase(),
            guessed_letters: HashSet::new(),
            wrong_guesses: Vec::new(),
            max_wrong_guesses: 6,
            category: category.to_string(),
        }
    }

    fn get_random_word() -> (&'static str, &'static str) {
        let word_categories = vec![
            // Programming terms
            ("RUST", "Programming Language"),
            ("PYTHON", "Programming Language"),
            ("JAVASCRIPT", "Programming Language"),
            ("FUNCTION", "Programming Concept"),
            ("VARIABLE", "Programming Concept"),
            ("LOOP", "Programming Concept"),
            ("ARRAY", "Data Structure"),
            ("STRUCT", "Programming Concept"),
            ("ENUM", "Programming Concept"),
            ("TRAIT", "Programming Concept"),
            ("VECTOR", "Data Structure"),
            ("HASHMAP", "Data Structure"),
            ("ITERATOR", "Programming Concept"),
            ("CLOSURE", "Programming Concept"),
            ("OWNERSHIP", "Rust Concept"),
            ("BORROWING", "Rust Concept"),
            ("LIFETIME", "Rust Concept"),
            ("MEMORY", "Computer Science"),
            ("ALGORITHM", "Computer Science"),
            ("RECURSION", "Programming Concept"),
            ("INHERITANCE", "Programming Concept"),
            ("POLYMORPHISM", "Programming Concept"),
            ("ENCAPSULATION", "Programming Concept"),
            ("ABSTRACTION", "Programming Concept"),
            // Discord/Gaming terms
            ("DISCORD", "Platform"),
            ("CHANNEL", "Discord Feature"),
            ("SERVER", "Technology"),
            ("MESSAGE", "Communication"),
            ("REACTION", "Discord Feature"),
            ("EMOJI", "Communication"),
            ("MODERATOR", "Role"),
            ("ADMINISTRATOR", "Role"),
            ("COMMAND", "Bot Feature"),
            ("PREFIX", "Bot Feature"),
            // General words
            ("CHALLENGE", "General"),
            ("ADVENTURE", "General"),
            ("DISCOVERY", "General"),
            ("CREATIVITY", "General"),
            ("KNOWLEDGE", "General"),
            ("WISDOM", "General"),
            ("LEARNING", "General"),
            ("PRACTICE", "General"),
            ("PATIENCE", "General"),
            ("PERSEVERANCE", "General"),
        ];

        let mut rng = rand::thread_rng();
        let index = rng.gen_range(0..word_categories.len());
        word_categories[index]
    }

    fn display_word(&self) -> String {
        self.word
            .chars()
            .map(|c| {
                if c.is_alphabetic() {
                    if self.guessed_letters.contains(&c) {
                        format!("**{}**", c)
                    } else {
                        "**_**".to_string()
                    }
                } else {
                    c.to_string()
                }
            })
            .collect::<Vec<String>>()
            .join(" ")
    }

    fn display_hangman(&self) -> String {
        let stages = [
            // Stage 0: Empty gallows
            "```\n  +---+\n  |   |\n      |\n      |\n      |\n      |\n=========\n```",
            // Stage 1: Head
            "```\n  +---+\n  |   |\n  O   |\n      |\n      |\n      |\n=========\n```",
            // Stage 2: Body
            "```\n  +---+\n  |   |\n  O   |\n  |   |\n      |\n      |\n=========\n```",
            // Stage 3: Left arm
            "```\n  +---+\n  |   |\n  O   |\n /|   |\n      |\n      |\n=========\n```",
            // Stage 4: Right arm
            "```\n  +---+\n  |   |\n  O   |\n /|\\  |\n      |\n      |\n=========\n```",
            // Stage 5: Left leg
            "```\n  +---+\n  |   |\n  O   |\n /|\\  |\n /    |\n      |\n=========\n```",
            // Stage 6: Right leg (game over)
            "```\n  +---+\n  |   |\n  O   |\n /|\\  |\n / \\  |\n      |\n=========\n```",
        ];

        stages[self.wrong_guesses.len()].to_string()
    }

    fn guess_letter(&mut self, letter: char) -> GuessResult {
        let letter = letter.to_uppercase().next().unwrap();

        if !letter.is_alphabetic() {
            return GuessResult::InvalidInput;
        }

        if self.guessed_letters.contains(&letter) || self.wrong_guesses.contains(&letter) {
            return GuessResult::AlreadyGuessed;
        }

        if self.word.contains(letter) {
            self.guessed_letters.insert(letter);

            // Count how many times this letter appears
            let count = self.word.chars().filter(|&c| c == letter).count();

            GuessResult::Correct(count)
        } else {
            self.wrong_guesses.push(letter);
            GuessResult::Wrong
        }
    }

    fn is_word_guessed(&self) -> bool {
        self.word
            .chars()
            .filter(|c| c.is_alphabetic())
            .all(|c| self.guessed_letters.contains(&c))
    }

    fn is_game_over(&self) -> bool {
        self.wrong_guesses.len() >= self.max_wrong_guesses
    }

    fn get_progress_info(&self) -> String {
        let total_letters = self.word.chars().filter(|c| c.is_alphabetic()).count();
        let guessed_letters = self
            .word
            .chars()
            .filter(|c| c.is_alphabetic() && self.guessed_letters.contains(c))
            .count();

        format!(
            "📊 **Progress:** {}/{} letters | **Wrong:** {}/{} | **Category:** {}",
            guessed_letters,
            total_letters,
            self.wrong_guesses.len(),
            self.max_wrong_guesses,
            self.category
        )
    }

    fn get_guessed_info(&self) -> String {
        let mut info = String::new();

        if !self.guessed_letters.is_empty() {
            let mut correct: Vec<char> = self.guessed_letters.iter().copied().collect();
            correct.sort();
            info.push_str(&format!(
                "✅ **Correct:** {}\n",
                correct
                    .iter()
                    .map(|c| c.to_string())
                    .collect::<Vec<String>>()
                    .join(", ")
            ));
        }

        if !self.wrong_guesses.is_empty() {
            info.push_str(&format!(
                "❌ **Wrong:** {}",
                self.wrong_guesses
                    .iter()
                    .map(|c| c.to_string())
                    .collect::<Vec<String>>()
                    .join(", ")
            ));
        }

        info
    }
}

enum GuessResult {
    Correct(usize),
    Wrong,
    AlreadyGuessed,
    InvalidInput,
}

/// Start a Hangman word guessing game!
#[poise::command(prefix_command, slash_command)]
pub async fn hangman(
    ctx: Context<'_>,
    #[description = "Custom word to guess (optional)"] custom_word: Option<String>,
) -> Result<(), Error> {
    let user_id = ctx.author().id.get();

    // Check if user already has an active game
    {
        let games = ACTIVE_GAMES.read().await;
        if games.contains_key(&user_id) {
            ctx.say("❌ You already have an active Hangman game! Use `/letter <letter>` to guess or `/endhangman` to quit.").await?;
            return Ok(());
        }
    }

    let game = match custom_word {
        Some(word) => {
            let word = word.trim().to_uppercase();
            if word.is_empty() || word.len() > 20 {
                ctx.say("❌ Custom word must be between 1-20 characters!")
                    .await?;
                return Ok(());
            }
            if !word.chars().all(|c| c.is_alphabetic() || c.is_whitespace()) {
                ctx.say("❌ Custom word can only contain letters and spaces!")
                    .await?;
                return Ok(());
            }
            HangmanGame::new_with_word(&word, "Custom Word")
        }
        None => HangmanGame::new(),
    };

    let response = format!(
        "🎪 **Hangman Game Started!**\n\n\
        {}\n\n\
        **Word:** {}\n\n\
        {}\n\n\
        Use `/letter <letter>` to guess a letter!\n\
        Use `/hangmanstatus` to see your progress\n\
        Use `/endhangman` to quit\n\n\
        Good luck, {}! 🍀",
        game.display_hangman(),
        game.display_word(),
        game.get_progress_info(),
        ctx.author().name
    );

    {
        let mut games = ACTIVE_GAMES.write().await;
        games.insert(user_id, game);
    }

    ctx.say(response).await?;
    Ok(())
}

/// Guess a letter in your active Hangman game
#[poise::command(prefix_command, slash_command)]
pub async fn letter(
    ctx: Context<'_>,
    #[description = "Letter to guess"] letter: String,
) -> Result<(), Error> {
    let user_id = ctx.author().id.get();

    if letter.len() != 1 {
        ctx.say("❌ Please enter exactly one letter!").await?;
        return Ok(());
    }

    let letter_char = letter.chars().next().unwrap();

    let mut games = ACTIVE_GAMES.write().await;

    if let Some(game) = games.get_mut(&user_id) {
        let result = game.guess_letter(letter_char);

        match result {
            GuessResult::Correct(count) => {
                if game.is_word_guessed() {
                    let response = format!(
                        "🎉 **CONGRATULATIONS!** 🎉\n\
                        You guessed the word: **{}**\n\n\
                        {}\n\n\
                        🏆 You won with {} wrong guesses!\n\
                        🎯 Category: {}\n\n\
                        Want to play again? Use `/hangman`!",
                        game.word,
                        game.display_hangman(),
                        game.wrong_guesses.len(),
                        game.category
                    );
                    ctx.say(response).await?;
                    games.remove(&user_id);
                } else {
                    let count_text = if count == 1 {
                        "once".to_string()
                    } else {
                        format!("{} times", count)
                    };

                    let response = format!(
                        "✅ **Great guess!** The letter '{}' appears {} in the word!\n\n\
                        **Word:** {}\n\n\
                        {}\n\
                        {}",
                        letter_char.to_uppercase(),
                        count_text,
                        game.display_word(),
                        game.get_progress_info(),
                        game.get_guessed_info()
                    );
                    ctx.say(response).await?;
                }
            }
            GuessResult::Wrong => {
                if game.is_game_over() {
                    let response = format!(
                        "💀 **GAME OVER!** You've been hanged!\n\n\
                        {}\n\n\
                        🔤 The word was: **{}**\n\
                        🎯 Category: {}\n\n\
                        Better luck next time! Use `/hangman` to try again!",
                        game.display_hangman(),
                        game.word,
                        game.category
                    );
                    ctx.say(response).await?;
                    games.remove(&user_id);
                } else {
                    let response = format!(
                        "❌ **Wrong!** The letter '{}' is not in the word.\n\n\
                        {}\n\n\
                        **Word:** {}\n\n\
                        {}\n\
                        {}",
                        letter_char.to_uppercase(),
                        game.display_hangman(),
                        game.display_word(),
                        game.get_progress_info(),
                        game.get_guessed_info()
                    );
                    ctx.say(response).await?;
                }
            }
            GuessResult::AlreadyGuessed => {
                ctx.say(format!(
                    "❌ You've already guessed the letter '{}'!",
                    letter_char.to_uppercase()
                ))
                .await?;
            }
            GuessResult::InvalidInput => {
                ctx.say("❌ Please enter a valid letter (A-Z)!").await?;
            }
        }
    } else {
        ctx.say("❌ You don't have an active Hangman game! Start one with `/hangman`")
            .await?;
    }

    Ok(())
}

/// Show your current Hangman game status
#[poise::command(prefix_command, slash_command)]
pub async fn hangmanstatus(ctx: Context<'_>) -> Result<(), Error> {
    let user_id = ctx.author().id.get();

    let games = ACTIVE_GAMES.read().await;

    if let Some(game) = games.get(&user_id) {
        let response = format!(
            "🎪 **Your Hangman Game**\n\n\
            {}\n\n\
            **Word:** {}\n\n\
            {}\n\
            {}\n\n\
            Use `/letter <letter>` to guess!",
            game.display_hangman(),
            game.display_word(),
            game.get_progress_info(),
            game.get_guessed_info()
        );

        ctx.say(response).await?;
    } else {
        ctx.say("❌ You don't have an active Hangman game! Start one with `/hangman`")
            .await?;
    }

    Ok(())
}

/// Get a hint for your current Hangman game
#[poise::command(prefix_command, slash_command)]
pub async fn hangmanhint(ctx: Context<'_>) -> Result<(), Error> {
    let user_id = ctx.author().id.get();

    let games = ACTIVE_GAMES.read().await;

    if let Some(game) = games.get(&user_id) {
        let word_length = game.word.len();
        let unique_letters = game
            .word
            .chars()
            .filter(|c| c.is_alphabetic())
            .collect::<HashSet<_>>()
            .len();
        let vowels_in_word = game
            .word
            .chars()
            .filter(|c| "AEIOU".contains(*c))
            .collect::<HashSet<_>>()
            .len();

        let hint_text = format!(
            "💡 **Hint for your Hangman game:**\n\
            🎯 **Category:** {}\n\
            📏 **Length:** {} characters\n\
            🔤 **Unique letters:** {}\n\
            📢 **Vowels:** {} different vowel(s)\n\
            📊 **Progress:** You've found {} out of {} letters",
            game.category,
            word_length,
            unique_letters,
            vowels_in_word,
            game.guessed_letters.len(),
            unique_letters
        );

        ctx.say(hint_text).await?;
    } else {
        ctx.say("❌ You don't have an active Hangman game! Start one with `/hangman`")
            .await?;
    }

    Ok(())
}

/// End your current Hangman game
#[poise::command(prefix_command, slash_command)]
pub async fn endhangman(ctx: Context<'_>) -> Result<(), Error> {
    let user_id = ctx.author().id.get();

    let mut games = ACTIVE_GAMES.write().await;

    if let Some(game) = games.remove(&user_id) {
        let response = format!(
            "🏳️ **Hangman game ended!**\n\
            The word was: **{}**\n\
            🎯 Category: {}\n\
            📊 You made {} wrong guesses\n\
            Thanks for playing! 👋",
            game.word,
            game.category,
            game.wrong_guesses.len()
        );
        ctx.say(response).await?;
    } else {
        ctx.say("❌ You don't have an active Hangman game!").await?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hangman_game_creation() {
        let game = HangmanGame::new_with_word("TEST", "Test Category");
        assert_eq!(game.word, "TEST");
        assert_eq!(game.wrong_guesses.len(), 0);
        assert!(game.guessed_letters.is_empty());
    }

    #[test]
    fn test_guess_correct_letter() {
        let mut game = HangmanGame::new_with_word("TEST", "Test Category");
        let result = game.guess_letter('T');

        match result {
            GuessResult::Correct(count) => assert_eq!(count, 2), // T appears twice
            _ => panic!("Expected correct guess"),
        }

        assert!(game.guessed_letters.contains(&'T'));
    }

    #[test]
    fn test_guess_wrong_letter() {
        let mut game = HangmanGame::new_with_word("TEST", "Test Category");
        let result = game.guess_letter('X');

        assert!(matches!(result, GuessResult::Wrong));
        assert!(game.wrong_guesses.contains(&'X'));
    }

    #[test]
    fn test_word_completion() {
        let mut game = HangmanGame::new_with_word("TEST", "Test Category");
        game.guess_letter('T');
        game.guess_letter('E');
        game.guess_letter('S');

        assert!(game.is_word_guessed());
    }

    #[test]
    fn test_game_over() {
        let mut game = HangmanGame::new_with_word("TEST", "Test Category");

        // Make 6 wrong guesses
        for letter in ['A', 'B', 'C', 'D', 'F', 'G'] {
            game.guess_letter(letter);
        }

        assert!(game.is_game_over());
    }
}
