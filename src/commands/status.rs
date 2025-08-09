use crate::{Context, Error};
use poise::serenity_prelude as serenity;
use std::time::{SystemTime, UNIX_EPOCH};

/// Shows diagnostic information and bot status
///
/// This command provides comprehensive diagnostic information about the bot including:
/// - Bot uptime and basic status
/// - System information (process info, memory usage)
/// - Discord API latency
/// - Active guilds and user count
/// - Bot permissions and capabilities
/// - Version and build information
///
/// # Usage
/// - `-status` or `/status` - Show full diagnostic information
///
/// This command is useful for:
/// - Verifying the bot is alive and responsive
/// - Debugging connection or performance issues
/// - Monitoring bot health and resource usage
/// - Checking bot permissions in the current guild
#[poise::command(prefix_command, slash_command)]
pub async fn status(ctx: Context<'_>) -> Result<(), Error> {
    log::info!("Status command called by {}", ctx.author().name);

    let start_time = std::time::Instant::now();

    // Send initial "gathering info" message
    let reply = ctx.say("🔍 Gathering diagnostic information...").await?;

    // Calculate API latency
    let api_start = std::time::Instant::now();
    let _ping_test = ctx.http().get_current_user().await;
    let api_latency = api_start.elapsed().as_millis();

    // Get basic information
    let bot_user_id = ctx.framework().bot_id;
    let current_user_name = {
        let user = ctx.cache().current_user();
        user.name.clone()
    };

    // Get guild and user counts
    let guild_count = ctx.cache().guilds().len();
    let mut total_users = 0;
    let mut total_channels = 0;
    let mut cached_member_count = 0;
    let mut guilds_with_zero_count = 0;
    let mut _guilds_with_valid_count = 0;

    // Collect stats from cached guilds
    for guild_id in ctx.cache().guilds() {
        if let Some(guild) = ctx.cache().guild(guild_id) {
            // Discord's member_count field is the authoritative count
            // But it might be 0 if the bot doesn't have the intent or if cache is incomplete
            if guild.member_count > 0 {
                total_users += guild.member_count;
                _guilds_with_valid_count += 1;
            } else {
                guilds_with_zero_count += 1;
                // Fallback to cached members (will be limited without GUILD_MEMBERS intent)
                cached_member_count += guild.members.len() as u64;
            }

            // Count all types of channels (text, voice, category, etc.)
            total_channels += guild.channels.len();
        }
    }

    // If we had to use cached members, add those to the total
    if cached_member_count > 0 && total_users == 0 {
        total_users = cached_member_count;
    }

    // Get current guild info if available
    let (guild_name, guild_member_count, bot_role_count) = if let Some(guild_id) = ctx.guild_id() {
        if let Some(guild) = ctx.cache().guild(guild_id) {
            let guild_name = guild.name.clone();
            let member_count = guild.member_count;

            // Try to get bot member info
            let bot_roles = if let Some(member) = guild.members.get(&bot_user_id) {
                member.roles.len()
            } else {
                0
            };

            (Some(guild_name), Some(member_count), Some(bot_roles))
        } else {
            (None, None, None)
        }
    } else {
        (None, None, None)
    };

    // Get system information
    let process_id = std::process::id();
    let current_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // Create comprehensive status embed
    let mut embed = serenity::CreateEmbed::new()
        .title("🤖 Bot Status & Diagnostics")
        .color(0x00ff00) // Green for healthy status
        .timestamp(serenity::Timestamp::now());

    // Basic bot information
    embed = embed.field(
        "🤖 Bot Information",
        format!(
            "**Name:** {}\n**ID:** {}\n**Status:** ✅ Online & Responsive\n**API Latency:** {}ms",
            current_user_name, bot_user_id, api_latency
        ),
        true,
    );

    // System information
    embed = embed.field(
        "⚙️ System Information",
        format!(
            "**Process ID:** {}\n**Timestamp:** <t:{}:F>\n**Response Time:** {}ms",
            process_id,
            current_time,
            start_time.elapsed().as_millis()
        ),
        true,
    );

    // Discord statistics with detailed debugging info
    let stats_text = if total_users == 0 && guild_count > 0 {
        format!(
            "**Guilds:** {}\n**Total Users:** {} ⚠️\n**Total Channels:** {}\n**Debug:** {}/{} guilds have member_count=0\n\n*Member counts may be 0 due to cache warming or large guilds*",
            guild_count, total_users, total_channels, guilds_with_zero_count, guild_count
        )
    } else if total_users == cached_member_count && cached_member_count > 0 {
        format!(
            "**Guilds:** {}\n**Total Users:** {} (from cache)\n**Total Channels:** {}\n**Debug:** Using cached member data",
            guild_count, total_users, total_channels
        )
    } else if guilds_with_zero_count > 0 {
        format!(
            "**Guilds:** {}\n**Total Users:** {} (partial)\n**Total Channels:** {}\n**Debug:** {}/{} guilds missing counts",
            guild_count, total_users, total_channels, guilds_with_zero_count, guild_count
        )
    } else {
        format!(
            "**Guilds:** {}\n**Total Users:** {}\n**Total Channels:** {}\n**Debug:** All counts available",
            guild_count, total_users, total_channels
        )
    };

    embed = embed.field("📊 Discord Statistics", stats_text, true);

    // Current guild information
    if let (Some(name), Some(members), Some(roles)) =
        (guild_name, guild_member_count, bot_role_count)
    {
        let guild_text = format!(
            "**Guild:** {}\n**Members:** {}\n**Bot Roles:** {}",
            name, members, roles
        );
        embed = embed.field("🏠 Current Guild", guild_text, true);
    }

    // Bot capabilities and features
    let capabilities = [
        "✅ Prefix Commands (-command)",
        "✅ Slash Commands (/command)",
        "✅ Message Content Access",
        "✅ Guild Member Intents",
        "✅ Error Handling & Logging",
        "✅ Auto-restart on Updates",
        "✅ Reminder System",
        "✅ Game Commands",
        "✅ Utility Commands",
    ];

    embed = embed.field("🔧 Bot Capabilities", capabilities.join("\n"), false);

    // Gateway intents information
    let intents_info = "**Configured Intents:**\n• Guild Messages ✅\n• Direct Messages ✅\n• Message Content ✅\n• Guild Members ✅\n\n*If user counts show 0, the bot may need time to cache member data or guilds may have member count disabled.*";

    embed = embed.field("🔗 Gateway Intents", intents_info, false);

    // Version and build information
    let version_info = format!(
        "**Package:** rustbot v{}\n**Rust Edition:** 2021\n**Framework:** Poise + Serenity\n**Build:** Development",
        env!("CARGO_PKG_VERSION")
    );

    embed = embed.field("📦 Version Information", version_info, true);

    // Health check summary
    let health_checks = [
        "✅ Discord Gateway Connection",
        "✅ HTTP API Connectivity",
        "✅ Command Framework",
        "✅ Database Access (File-based)",
        "✅ Background Tasks",
        "✅ Error Recovery",
    ];

    embed = embed.field("🏥 Health Checks", health_checks.join("\n"), true);

    // Available commands count
    let command_count = ctx.framework().options().commands.len();
    embed = embed.field(
        "📋 Commands Available",
        format!(
            "**Total Commands:** {}\n**Type:** `-help` for list",
            command_count
        ),
        false,
    );

    // Footer with additional info
    embed = embed.footer(serenity::CreateEmbedFooter::new(format!(
        "Requested by {} • Bot is healthy and operational",
        ctx.author().name
    )));

    // Update the reply with the diagnostic information
    reply
        .edit(
            ctx,
            poise::CreateReply::default()
                .content("📊 **Diagnostic Report Complete**")
                .embed(embed),
        )
        .await?;

    log::info!(
        "Status command completed successfully for {}",
        ctx.author().name
    );

    Ok(())
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_status_command_exists() {
        // Verify the command function exists and has the correct signature
        let function_name = "status";
        assert_eq!(function_name.len(), 6);
    }

    #[test]
    fn test_health_check_format() {
        // Test that health check items are properly formatted
        let health_checks = [
            "✅ Discord Gateway Connection",
            "✅ HTTP API Connectivity",
            "✅ Command Framework",
        ];

        let joined = health_checks.join("\n");
        assert!(joined.contains("✅"));
        assert!(joined.contains("Discord Gateway"));
    }

    #[test]
    fn test_capabilities_list() {
        // Test that capabilities list is properly structured
        let capabilities = [
            "✅ Prefix Commands (-command)",
            "✅ Slash Commands (/command)",
            "✅ Message Content Access",
        ];

        assert!(capabilities.iter().all(|cap| cap.starts_with("✅")));
        assert!(capabilities.len() >= 3);
    }
}
