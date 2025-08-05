// This file serves as a placeholder for general commands
// Individual commands have been moved to their own files:
// - ping.rs
// - hello.rs
// spamping.rs

// You can add new general utility functions here that are shared
// across multiple commands, or create new command functions that
// don't fit into more specific categories.

use crate::{Context, Error};

// Utility function to print raw API errors - useful for debugging
pub async fn print_api_error(ctx: Context<'_>, error: &Error) -> Result<(), Error> {
    let error_msg = format!("❌ {}", error);
    ctx.say(error_msg).await?;
    Ok(())
}

// Example of a shared utility function:
pub fn get_user_display_name(user: &poise::serenity_prelude::User) -> String {
    user.global_name.as_ref().unwrap_or(&user.name).clone()
}

// This utility function is kept here as an example of shared functionality
// that could be used across multiple commands. You can add more utility
// functions here as needed.

// Example of how to add a new general command:
//
// /// Your new command description
// #[poise::command(prefix_command, slash_command)]
// pub async fn your_command(ctx: Context<'_>) -> Result<(), Error> {
//     ctx.say("Your response!").await?;
//     Ok(())
// }
