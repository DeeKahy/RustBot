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
