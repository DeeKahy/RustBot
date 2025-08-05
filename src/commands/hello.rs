use crate::{Context, Error};

/// A friendly hello command
#[poise::command(prefix_command, slash_command)]
pub async fn hello(
    ctx: Context<'_>,
    #[description = "Name to greet"] name: Option<String>,
) -> Result<(), Error> {
    log::info!("Hello command called by {}", ctx.author().name);

    let name = name.unwrap_or_else(|| ctx.author().name.clone());

    let response = format!("👋 Hello, {}! Nice to meet you!", name);
    if let Err(e) = ctx.say(response).await {
        ctx.say(format!("❌ {}", e)).await?;
    }

    Ok(())
}
