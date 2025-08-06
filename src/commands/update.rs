use crate::{Context, Error};

use std::process::{Command, Stdio};

/// Update the bot by pulling latest changes from GitHub and restarting
#[poise::command(slash_command, prefix_command)]
pub async fn update(ctx: Context<'_>) -> Result<(), Error> {
    // Check if the user is authorized
    if ctx.author().name != "deekahy" {
        ctx.say("❌ You don't have permission to use this command!")
            .await?;
        return Ok(());
    }

    ctx.say("🔄 Starting update process...").await?;

    // Create a follow-up message that we can edit
    let reply = ctx.say("📥 Pulling latest changes from GitHub...").await?;

    // Pull the latest changes
    let git_pull = Command::new("git")
        .args(&["pull", "origin", "main"])
        .current_dir("/app")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();

    match git_pull {
        Ok(output) => {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                log::info!("Git pull successful: {}", stdout);

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
                    .args(&["build", "--release"])
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

                            // Wait a moment before exiting
                            tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

                            // Exit with a specific code that indicates a restart is needed
                            std::process::exit(42);
                        } else {
                            let stderr = String::from_utf8_lossy(&build_output.stderr);
                            log::error!("Build failed: {}", stderr);
                            reply
                                .edit(
                                    ctx,
                                    poise::CreateReply::default().content(&format!(
                                        "❌ Build failed:\n```\n{}\n```",
                                        stderr.chars().take(1900).collect::<String>()
                                    )),
                                )
                                .await?;
                        }
                    }
                    Err(e) => {
                        log::error!("Failed to run cargo build: {}", e);
                        reply
                            .edit(
                                ctx,
                                poise::CreateReply::default()
                                    .content(&format!("❌ Failed to run cargo build: {}", e)),
                            )
                            .await?;
                    }
                }
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                log::error!("Git pull failed: {}", stderr);
                reply
                    .edit(
                        ctx,
                        poise::CreateReply::default().content(&format!(
                            "❌ Git pull failed:\n```\n{}\n```",
                            stderr.chars().take(1900).collect::<String>()
                        )),
                    )
                    .await?;
            }
        }
        Err(e) => {
            log::error!("Failed to run git pull: {}", e);
            reply
                .edit(
                    ctx,
                    poise::CreateReply::default()
                        .content(&format!("❌ Failed to run git pull: {}", e)),
                )
                .await?;
        }
    }

    Ok(())
}
