use crate::utils::{get_git_branch, is_protected_user};
use crate::{Context, Error};

use serde::{Deserialize, Serialize};
use std::fs;
use std::process::{Command, Stdio};

#[derive(Serialize, Deserialize)]
struct UpdateInfo {
    channel_id: u64,
    user_name: String,
}

/// Update the bot by pulling latest changes from GitHub and restarting
#[poise::command(slash_command, prefix_command)]
pub async fn update(ctx: Context<'_>) -> Result<(), Error> {
    // Check if the user is authorized
    if !is_protected_user(&ctx.author().name) {
        ctx.say("❌ You don't have permission to use this command!")
            .await?;
        return Ok(());
    }

    ctx.say("🔄 Starting update process...").await?;

    // Create a follow-up message that we can edit
    let reply = ctx.say("📥 Pulling latest changes from GitHub...").await?;

    // Pull the latest changes
    let branch = get_git_branch();
    let git_pull = Command::new("git")
        .args(["pull", "origin", &branch])
        .current_dir("/app")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();

    match git_pull {
        Ok(output) => {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                log::info!("Git pull successful: {stdout}");

                reply
                    .edit(
                        ctx,
                        poise::CreateReply::default().content(
                            "✅ Successfully pulled latest changes!\n🔨 Compiling new version...",
                        ),
                    )
                    .await?;

                // Build the new version
                let cargo_build = Command::new("cargo")
                    .args(["build", "--release"])
                    .current_dir("/app")
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .output();

                match cargo_build {
                    Ok(build_output) => {
                        if build_output.status.success() {
                            log::info!("Build successful");
                            reply
                                .edit(
                                    ctx,
                                    poise::CreateReply::default()
                                        .content("✅ Compilation successful!\n🔄 Restarting bot in 3 seconds..."),
                                )
                                .await?;

                            // Store update info for startup message
                            let update_info = UpdateInfo {
                                channel_id: ctx.channel_id().get(),
                                user_name: ctx.author().name.clone(),
                            };

                            if let Ok(update_json) = serde_json::to_string(&update_info) {
                                let _ = fs::write("/tmp/rustbot_update_info.json", update_json);
                            }

                            // Wait a moment before exiting
                            tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

                            // Exit with a specific code that indicates a restart is needed
                            std::process::exit(42);
                        } else {
                            let stderr = String::from_utf8_lossy(&build_output.stderr);
                            log::error!("Build failed: {stderr}");
                            reply
                                .edit(
                                    ctx,
                                    poise::CreateReply::default().content(format!(
                                        "❌ Build failed:\n```\n{}\n```",
                                        stderr.chars().take(1900).collect::<String>()
                                    )),
                                )
                                .await?;
                        }
                    }
                    Err(e) => {
                        log::error!("Failed to run cargo build: {e}");
                        reply
                            .edit(
                                ctx,
                                poise::CreateReply::default()
                                    .content(format!("❌ Failed to run cargo build: {e}")),
                            )
                            .await?;
                    }
                }
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                log::error!("Git pull failed: {stderr}");
                reply
                    .edit(
                        ctx,
                        poise::CreateReply::default().content(format!(
                            "❌ Git pull failed:\n```\n{}\n```",
                            stderr.chars().take(1900).collect::<String>()
                        )),
                    )
                    .await?;
            }
        }
        Err(e) => {
            log::error!("Failed to run git pull: {e}");
            reply
                .edit(
                    ctx,
                    poise::CreateReply::default()
                        .content(format!("❌ Failed to run git pull: {e}")),
                )
                .await?;
        }
    }

    Ok(())
}
