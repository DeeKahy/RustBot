use poise::serenity_prelude as serenity;
use std::env;

/// Check if a user is authorized to use protected commands
pub fn is_protected_user(username: &str) -> bool {
    let protected_users = env::var("PROTECTED_USERS").unwrap_or_else(|_| "deekahy".to_string()); // Default fallback

    protected_users
        .split_whitespace()
        .any(|user| user.trim().eq_ignore_ascii_case(username))
}

/// Get the git branch to use for updates
pub fn get_git_branch() -> String {
    env::var("GIT_BRANCH").unwrap_or_else(|_| "main".to_string()) // Default to main branch
}

/// Send a DM to deekahy using their user ID
pub async fn send_dm_to_deekahy(
    http: &serenity::Http,
    message: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let deekahy_id = serenity::UserId::new(398107630524039170);

    match deekahy_id.to_user(http).await {
        Ok(user) => match user.create_dm_channel(http).await {
            Ok(dm_channel) => match dm_channel.say(http, message).await {
                Ok(_) => {
                    log::info!("Successfully sent DM to deekahy");
                    Ok(())
                }
                Err(e) => {
                    log::error!("Failed to send DM to deekahy: {}", e);
                    Err(e.into())
                }
            },
            Err(e) => {
                log::error!("Failed to create DM channel with deekahy: {}", e);
                Err(e.into())
            }
        },
        Err(e) => {
            log::error!("Failed to get deekahy user: {}", e);
            Err(e.into())
        }
    }
}
