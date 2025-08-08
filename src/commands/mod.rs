// Commands module - imports all individual command files
pub mod cleanup;
pub mod coinflip;
pub mod dice;
pub mod general;
pub mod hello;
pub mod help;
pub mod invite;
pub mod kys;
pub mod pfp;
pub mod ping;
pub mod poll;
pub mod react;
pub mod remind;
pub mod spamping;
pub mod stats;
pub mod update;
pub mod uwu;
pub mod yourmom;

// Re-export all commands for easy access from main.rs
pub use cleanup::cleanup;
pub use coinflip::coinflip;
pub use dice::dice;
pub use hello::hello;
pub use help::help;
pub use invite::invite;
pub use kys::kys;
pub use pfp::pfp;
pub use ping::ping;
pub use poll::poll;
pub use react::react;
pub use remind::{remind, start_reminder_checker};
pub use spamping::spamping;
pub use stats::stats;
pub use update::update;
pub use uwu::uwu;
pub use yourmom::yourmom;

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
