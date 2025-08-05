// Commands module - imports all individual command files
pub mod coinflip;
pub mod general;
pub mod hello;
pub mod pfp;
pub mod ping;
pub mod spamping;
pub mod uwu;

// Re-export all commands for easy access from main.rs
pub use coinflip::coinflip;
pub use hello::hello;
pub use pfp::pfp;
pub use ping::ping;
pub use spamping::spamping;
pub use uwu::uwu;

// You can add more command modules here as needed
// Example:
// pub mod moderation;
// pub mod fun;
// pub mod admin;
//
// And then re-export them:
// pub use moderation::*;
// pub use fun::*;
// pub use admin::*;
