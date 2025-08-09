use crate::{Context, Error};
use poise::serenity_prelude::{self as serenity, Mentionable};
use std::collections::HashMap;
use std::fmt;
use tokio::sync::RwLock;

// Global storage for active games
lazy_static::lazy_static! {
    static ref ACTIVE_GAMES: RwLock<HashMap<u64, TicTacToeGame>> = RwLock::new(HashMap::new());
}

#[derive(Clone, Copy, PartialEq)]
enum Player {
    X,
    O,
}

impl fmt::Display for Player {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Player::X => write!(f, "❌"),
            Player::O => write!(f, "⭕"),
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Cell {
    Empty,
    Occupied(Player),
}

impl fmt::Display for Cell {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Cell::Empty => write!(f, "⬜"),
            Cell::Occupied(player) => write!(f, "{}", player),
        }
    }
}

#[derive(Clone)]
struct TicTacToeGame {
    board: [[Cell; 3]; 3],
    current_player: Player,
    player_x_id: u64,
    player_o_id: Option<u64>, // None for AI mode
    is_ai_game: bool,
    message_id: Option<u64>, // For editing the game message
    channel_id: u64,
}

impl TicTacToeGame {
    // Helper function to delete the previous message and send a new one
    async fn delete_and_send_message(
        &self,
        ctx: &Context<'_>,
        content: String,
    ) -> Result<u64, Error> {
        // Delete the previous message if it exists
        if let Some(msg_id) = self.message_id {
            let msg_id = serenity::MessageId::new(msg_id);
            let channel_id = serenity::ChannelId::new(self.channel_id);

            // Try to delete the previous message (ignore errors if message doesn't exist)
            let _ = channel_id
                .delete_message(&ctx.serenity_context().http, msg_id)
                .await;
        }

        // Send the new message and return its ID
        let reply = ctx.say(content).await?;
        Ok(reply.message().await?.id.get())
    }

    fn new_two_player(player_x_id: u64, player_o_id: u64, channel_id: u64) -> Self {
        TicTacToeGame {
            board: [[Cell::Empty; 3]; 3],
            current_player: Player::X,
            player_x_id,
            player_o_id: Some(player_o_id),
            is_ai_game: false,
            message_id: None,
            channel_id,
        }
    }

    fn new_vs_ai(player_x_id: u64, channel_id: u64) -> Self {
        TicTacToeGame {
            board: [[Cell::Empty; 3]; 3],
            current_player: Player::X,
            player_x_id,
            player_o_id: None,
            is_ai_game: true,
            message_id: None,
            channel_id,
        }
    }

    fn display_board(&self) -> String {
        let mut board_str = String::new();
        board_str.push_str("```\n");

        for (row_idx, row) in self.board.iter().enumerate() {
            for (col_idx, cell) in row.iter().enumerate() {
                match cell {
                    Cell::Empty => board_str.push_str(&format!(" {} ", row_idx * 3 + col_idx + 1)),
                    Cell::Occupied(Player::X) => board_str.push_str(" X "),
                    Cell::Occupied(Player::O) => board_str.push_str(" O "),
                }
                if col_idx < 2 {
                    board_str.push('|');
                }
            }
            board_str.push('\n');
            if row_idx < 2 {
                board_str.push_str("---|---|---\n");
            }
        }
        board_str.push_str("```");
        board_str
    }

    fn make_move(&mut self, position: usize) -> Result<(), String> {
        if !(1..=9).contains(&position) {
            return Err("Position must be between 1-9!".to_string());
        }

        let row = (position - 1) / 3;
        let col = (position - 1) % 3;

        if self.board[row][col] != Cell::Empty {
            return Err("That position is already taken!".to_string());
        }

        self.board[row][col] = Cell::Occupied(self.current_player);
        Ok(())
    }

    fn check_winner(&self) -> Option<Player> {
        // Check rows
        for row in &self.board {
            if let Cell::Occupied(player) = row[0] {
                if row[1] == Cell::Occupied(player) && row[2] == Cell::Occupied(player) {
                    return Some(player);
                }
            }
        }

        // Check columns
        for col in 0..3 {
            if let Cell::Occupied(player) = self.board[0][col] {
                if self.board[1][col] == Cell::Occupied(player)
                    && self.board[2][col] == Cell::Occupied(player)
                {
                    return Some(player);
                }
            }
        }

        // Check diagonals
        if let Cell::Occupied(player) = self.board[0][0] {
            if self.board[1][1] == Cell::Occupied(player)
                && self.board[2][2] == Cell::Occupied(player)
            {
                return Some(player);
            }
        }

        if let Cell::Occupied(player) = self.board[0][2] {
            if self.board[1][1] == Cell::Occupied(player)
                && self.board[2][0] == Cell::Occupied(player)
            {
                return Some(player);
            }
        }

        None
    }

    fn is_board_full(&self) -> bool {
        for row in &self.board {
            for cell in row {
                if *cell == Cell::Empty {
                    return false;
                }
            }
        }
        true
    }

    fn switch_player(&mut self) {
        self.current_player = match self.current_player {
            Player::X => Player::O,
            Player::O => Player::X,
        };
    }

    fn get_ai_move(&self) -> Option<usize> {
        // Simple AI strategy:
        // 1. Try to win
        // 2. Block opponent from winning
        // 3. Take center if available
        // 4. Take corners
        // 5. Take any available spot

        // Try to win
        if let Some(pos) = self.find_winning_move(Player::O) {
            return Some(pos);
        }

        // Block opponent
        if let Some(pos) = self.find_winning_move(Player::X) {
            return Some(pos);
        }

        // Take center if available
        if self.board[1][1] == Cell::Empty {
            return Some(5);
        }

        // Take corners
        let corners = [1, 3, 7, 9];
        for &corner in &corners {
            let row = (corner - 1) / 3;
            let col = (corner - 1) % 3;
            if self.board[row][col] == Cell::Empty {
                return Some(corner);
            }
        }

        // Take any available spot
        for pos in 1..=9 {
            let row = (pos - 1) / 3;
            let col = (pos - 1) % 3;
            if self.board[row][col] == Cell::Empty {
                return Some(pos);
            }
        }

        None
    }

    fn find_winning_move(&self, player: Player) -> Option<usize> {
        for pos in 1..=9 {
            let row = (pos - 1) / 3;
            let col = (pos - 1) % 3;

            if self.board[row][col] == Cell::Empty {
                // Try this move
                let mut test_board = self.clone();
                test_board.board[row][col] = Cell::Occupied(player);
                if test_board.check_winner() == Some(player) {
                    return Some(pos);
                }
            }
        }
        None
    }

    fn get_current_player_id(&self) -> Option<u64> {
        match self.current_player {
            Player::X => Some(self.player_x_id),
            Player::O => self.player_o_id,
        }
    }
}

/// Start a Tic-Tac-Toe game! Play against another player or the AI
#[poise::command(prefix_command, slash_command)]
pub async fn tictactoe(
    ctx: Context<'_>,
    #[description = "Player to challenge (leave empty to play vs AI)"] opponent: Option<
        serenity::User,
    >,
) -> Result<(), Error> {
    let user_id = ctx.author().id.get();

    // Check if user already has an active game
    {
        let games = ACTIVE_GAMES.read().await;
        if games.contains_key(&user_id) {
            ctx.say("❌ You already have an active Tic-Tac-Toe game! Use `/move <position>` to play or `/endttt` to quit.").await?;
            return Ok(());
        }
    }

    let game = match opponent {
        Some(opponent_user) => {
            if opponent_user.id == ctx.author().id {
                ctx.say("❌ You can't play against yourself! Try `/tictactoe` without mentioning anyone to play vs AI.").await?;
                return Ok(());
            }

            if opponent_user.bot {
                ctx.say("❌ You can't play against bots! Try `/tictactoe` without mentioning anyone to play vs AI.").await?;
                return Ok(());
            }

            // Check if opponent already has a game
            {
                let games = ACTIVE_GAMES.read().await;
                if games.contains_key(&opponent_user.id.get()) {
                    ctx.say(format!(
                        "❌ {} already has an active game!",
                        opponent_user.name
                    ))
                    .await?;
                    return Ok(());
                }
            }

            TicTacToeGame::new_two_player(user_id, opponent_user.id.get(), ctx.channel_id().get())
        }
        None => TicTacToeGame::new_vs_ai(user_id, ctx.channel_id().get()),
    };

    let game_type = if game.is_ai_game {
        "🤖 **vs AI**"
    } else {
        "👥 **Two Player**"
    };

    let opponent_mention = if let Some(opponent_id) = game.player_o_id {
        format!("<@{}>", opponent_id)
    } else {
        "AI".to_string()
    };

    let response = format!(
        "⭕ **Tic-Tac-Toe Game Started!** {}\n\
        **Player X:** {}\n\
        **Player O:** {}\n\n\
        {} goes first!\n\
        Use `/move <position>` where position is 1-9:\n\
        {}\n\
        Current turn: {} {}",
        game_type,
        ctx.author().mention(),
        opponent_mention,
        ctx.author().mention(),
        game.display_board(),
        game.current_player,
        ctx.author().mention()
    );

    // Send initial message and store its ID
    let reply = ctx.say(response).await?;
    let mut game_with_msg_id = game.clone();
    game_with_msg_id.message_id = Some(reply.message().await?.id.get());

    // Store game for both players
    {
        let mut games = ACTIVE_GAMES.write().await;
        games.insert(user_id, game_with_msg_id.clone());
        if let Some(opponent_id) = game_with_msg_id.player_o_id {
            games.insert(opponent_id, game_with_msg_id);
        }
    }
    Ok(())
}

/// Make a move in your active Tic-Tac-Toe game
#[poise::command(prefix_command, slash_command)]
pub async fn move_ttt(
    ctx: Context<'_>,
    #[description = "Position to place your mark (1-9)"] position: u32,
) -> Result<(), Error> {
    let user_id = ctx.author().id.get();

    // Get current game state and extract needed info
    let (mut updated_game, player_x_id, player_o_id) = {
        let games = ACTIVE_GAMES.read().await;
        match games.get(&user_id) {
            Some(game) => {
                let game_clone = game.clone();
                (
                    game_clone.clone(),
                    game_clone.player_x_id,
                    game_clone.player_o_id,
                )
            }
            None => {
                ctx.say(
                    "❌ You don't have an active Tic-Tac-Toe game! Start one with `/tictactoe`",
                )
                .await?;
                return Ok(());
            }
        }
    };

    // Check if it's the user's turn
    let current_player_id = updated_game.get_current_player_id();
    if current_player_id != Some(user_id) {
        if updated_game.is_ai_game {
            ctx.say("❌ It's the AI's turn! Wait for the AI to move.")
                .await?;
        } else {
            let other_player = if updated_game.player_x_id == user_id {
                updated_game.player_o_id.unwrap()
            } else {
                updated_game.player_x_id
            };
            ctx.say(format!("❌ It's <@{}>'s turn!", other_player))
                .await?;
        }
        return Ok(());
    }

    // Try to make the move
    match updated_game.make_move(position as usize) {
        Ok(_) => {
            // Check for win or tie
            if let Some(winner) = updated_game.check_winner() {
                let winner_id = if winner == Player::X {
                    updated_game.player_x_id
                } else {
                    updated_game.player_o_id.unwrap_or(0) // 0 for AI
                };

                let winner_text = if winner_id == 0 {
                    "🤖 **AI wins!**".to_string()
                } else {
                    format!("🎉 **<@{}> wins!**", winner_id)
                };

                let response = format!(
                    "{}\n{}\n\nGame over! Use `/tictactoe` to start a new game.",
                    winner_text,
                    updated_game.display_board()
                );

                // Remove game for both players
                {
                    let mut games = ACTIVE_GAMES.write().await;
                    games.remove(&player_x_id);
                    if let Some(opponent_id) = player_o_id {
                        games.remove(&opponent_id);
                    }
                }

                updated_game.delete_and_send_message(&ctx, response).await?;
                return Ok(());
            } else if updated_game.is_board_full() {
                let response = format!(
                    "🤝 **It's a tie!**\n{}\n\nGame over! Use `/tictactoe` to start a new game.",
                    updated_game.display_board()
                );

                // Remove game for both players
                {
                    let mut games = ACTIVE_GAMES.write().await;
                    games.remove(&player_x_id);
                    if let Some(opponent_id) = player_o_id {
                        games.remove(&opponent_id);
                    }
                }

                updated_game.delete_and_send_message(&ctx, response).await?;
                return Ok(());
            }

            updated_game.switch_player();

            // Handle AI move if it's an AI game and now AI's turn
            if updated_game.is_ai_game && updated_game.current_player == Player::O {
                if let Some(ai_move) = updated_game.get_ai_move() {
                    updated_game.make_move(ai_move).unwrap();

                    // Check for AI win or tie after AI move
                    if let Some(_winner) = updated_game.check_winner() {
                        let response = format!(
                            "🤖 **AI wins!**\nAI played position {}\n{}\n\nGame over! Use `/tictactoe` to start a new game.",
                            ai_move,
                            updated_game.display_board()
                        );

                        // Remove game
                        {
                            let mut games = ACTIVE_GAMES.write().await;
                            games.remove(&player_x_id);
                        }

                        updated_game.delete_and_send_message(&ctx, response).await?;
                        return Ok(());
                    } else if updated_game.is_board_full() {
                        let response = format!(
                            "🤝 **It's a tie!**\nAI played position {}\n{}\n\nGame over! Use `/tictactoe` to start a new game.",
                            ai_move,
                            updated_game.display_board()
                        );

                        // Remove game
                        {
                            let mut games = ACTIVE_GAMES.write().await;
                            games.remove(&player_x_id);
                        }

                        updated_game.delete_and_send_message(&ctx, response).await?;
                        return Ok(());
                    }

                    updated_game.switch_player();

                    // Show board after AI move
                    let response = format!(
                        "🤖 AI played position **{}**\n{}\n\nYour turn! {} Use `/move <position>`",
                        ai_move,
                        updated_game.display_board(),
                        ctx.author().mention()
                    );

                    let new_msg_id = updated_game.delete_and_send_message(&ctx, response).await?;
                    updated_game.message_id = Some(new_msg_id);
                }
            } else {
                // Show current board state for human vs human
                let current_player_mention = if updated_game.current_player == Player::X {
                    format!("<@{}>", updated_game.player_x_id)
                } else {
                    format!("<@{}>", updated_game.player_o_id.unwrap())
                };

                let response = format!(
                    "{}\n\nCurrent turn: {} {}",
                    updated_game.display_board(),
                    updated_game.current_player,
                    current_player_mention
                );

                let new_msg_id = updated_game.delete_and_send_message(&ctx, response).await?;
                updated_game.message_id = Some(new_msg_id);
            }

            // Always update both players' game states after any move
            {
                let mut games = ACTIVE_GAMES.write().await;
                games.insert(player_x_id, updated_game.clone());
                if let Some(opponent_id) = player_o_id {
                    games.insert(opponent_id, updated_game);
                }
            }
        }
        Err(msg) => {
            ctx.say(format!("❌ {}", msg)).await?;
        }
    }

    Ok(())
}

/// Show your current Tic-Tac-Toe game board
#[poise::command(prefix_command, slash_command)]
pub async fn board(ctx: Context<'_>) -> Result<(), Error> {
    let user_id = ctx.author().id.get();

    let games = ACTIVE_GAMES.read().await;

    if let Some(game) = games.get(&user_id) {
        let current_player_mention = if let Some(current_id) = game.get_current_player_id() {
            if current_id == 0 {
                "🤖 AI".to_string()
            } else {
                format!("<@{}>", current_id)
            }
        } else {
            "🤖 AI".to_string()
        };

        let game_info = if game.is_ai_game {
            format!(
                "🤖 **Playing vs AI**\nYou: {} | AI: {}",
                Player::X,
                Player::O
            )
        } else {
            format!(
                "👥 **Two Player Game**\n<@{}>: {} | <@{}>: {}",
                game.player_x_id,
                Player::X,
                game.player_o_id.unwrap(),
                Player::O
            )
        };

        let response = format!(
            "{}\n{}\n\nCurrent turn: {} {}",
            game_info,
            game.display_board(),
            game.current_player,
            current_player_mention
        );

        ctx.say(response).await?;
    } else {
        ctx.say("❌ You don't have an active Tic-Tac-Toe game! Start one with `/tictactoe`")
            .await?;
    }

    Ok(())
}

/// End your current Tic-Tac-Toe game
#[poise::command(prefix_command, slash_command)]
pub async fn endttt(ctx: Context<'_>) -> Result<(), Error> {
    let user_id = ctx.author().id.get();

    let mut games = ACTIVE_GAMES.write().await;

    if let Some(game) = games.remove(&user_id) {
        // Also remove the game for the opponent if it's a two-player game
        if let Some(opponent_id) = game.player_o_id {
            games.remove(&opponent_id);
        }

        ctx.say("🏳️ **Tic-Tac-Toe game ended!** Thanks for playing! 👋")
            .await?;
    } else {
        ctx.say("❌ You don't have an active Tic-Tac-Toe game!")
            .await?;
    }

    Ok(())
}
