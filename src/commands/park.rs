use crate::{Context, Error};
use chrono::{DateTime, Datelike, Duration, TimeZone, Utc, Weekday};
use chrono_tz::Europe::Copenhagen;
use poise::serenity_prelude as serenity;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use serenity::{Http, UserId};
use std::collections::HashMap;
use std::fs;
use std::sync::Arc;
use tokio::time::{interval, Duration as TokioDuration};

#[derive(Serialize, Deserialize, Clone)]
struct UserParkingInfo {
    phone_number: String,
    plate: String,
}

#[derive(Serialize, Deserialize, Clone)]
struct ParkingSchedule {
    user_id: u64,
    hour: u8,
    minute: u8,
    enabled: bool,
    last_parked: Option<DateTime<Utc>>,
    missed_requests: Vec<DateTime<Utc>>,
}

#[derive(Serialize, Deserialize, Default)]
struct ParkingData {
    users: HashMap<u64, UserParkingInfo>,
    schedules: HashMap<u64, ParkingSchedule>,
}

const PARKING_DATA_FILE: &str = "/var/lib/rustbot/parking_data.json";

fn ensure_data_directory() -> Result<(), Error> {
    let dir = std::path::Path::new("/var/lib/rustbot");
    if !dir.exists() {
        std::fs::create_dir_all(dir)?;
    }
    Ok(())
}

fn load_parking_data() -> ParkingData {
    if let Err(e) = ensure_data_directory() {
        log::warn!("Failed to create data directory: {}", e);
    }

    match fs::read_to_string(PARKING_DATA_FILE) {
        Ok(content) => {
            // Try to parse as current format first
            match serde_json::from_str::<ParkingData>(&content) {
                Ok(mut data) => {
                    // Migrate old schedules that don't have missed_requests field
                    for schedule in data.schedules.values_mut() {
                        if schedule.missed_requests.is_empty() {
                            // This handles the case where old data doesn't have the field
                        }
                    }
                    data
                }
                Err(_) => {
                    log::warn!("Failed to parse parking data, starting fresh");
                    ParkingData::default()
                }
            }
        }
        Err(_) => ParkingData::default(),
    }
}

fn save_parking_data(data: &ParkingData) -> Result<(), Error> {
    ensure_data_directory()?;
    let json = serde_json::to_string_pretty(data)?;
    fs::write(PARKING_DATA_FILE, json)?;
    Ok(())
}

/// Park your vehicle using mobile parking service
#[poise::command(
    prefix_command,
    slash_command,
    subcommands("park_now", "park_info", "park_clear", "park_schedule")
)]
pub async fn park(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Park your vehicle now
#[poise::command(prefix_command, slash_command, rename = "now")]
pub async fn park_now(
    ctx: Context<'_>,
    #[description = "Vehicle registration number (license plate) - optional if previously saved"]
    plate: Option<String>,
    #[description = "Phone number (without country code) - optional if previously saved"]
    phone_number: Option<String>,
) -> Result<(), Error> {
    let user_id = ctx.author().id.get();
    let mut data = load_parking_data();

    log::info!(
        "Park command called by {} with plate: '{:?}' and phone: '{:?}'",
        ctx.author().name,
        plate,
        phone_number
    );

    // Determine which info to use
    let (final_plate, final_phone) = match (plate, phone_number) {
        // Both provided - use new info and save it
        (Some(p), Some(ph)) => {
            // Validate phone number (should be digits only)
            if !ph.chars().all(|c| c.is_ascii_digit()) {
                ctx.send(poise::CreateReply::default()
                    .content("❌ Phone number should contain only digits (no spaces, dashes, or country code)")
                    .ephemeral(true))
                    .await?;
                return Ok(());
            }

            // Validate plate (basic validation - not empty and reasonable length)
            if p.trim().is_empty() || p.len() > 10 {
                ctx.send(
                    poise::CreateReply::default()
                        .content("❌ Invalid license plate format")
                        .ephemeral(true),
                )
                .await?;
                return Ok(());
            }

            let user_info = UserParkingInfo {
                phone_number: ph.clone(),
                plate: p.to_uppercase(),
            };

            data.users.insert(user_id, user_info);
            if let Err(e) = save_parking_data(&data) {
                log::warn!("Failed to save parking data: {}", e);
            }

            (p.to_uppercase(), ph)
        }
        // Only plate provided - use stored phone if available
        (Some(p), None) => {
            if p.trim().is_empty() || p.len() > 10 {
                ctx.send(
                    poise::CreateReply::default()
                        .content("❌ Invalid license plate format")
                        .ephemeral(true),
                )
                .await?;
                return Ok(());
            }

            match data.users.get(&user_id).cloned() {
                Some(stored_info) => {
                    // Update stored plate
                    let mut updated_info = stored_info.clone();
                    updated_info.plate = p.to_uppercase();
                    let phone_number = stored_info.phone_number.clone();
                    data.users.insert(user_id, updated_info);
                    if let Err(e) = save_parking_data(&data) {
                        log::warn!("Failed to save parking data: {}", e);
                    }
                    (p.to_uppercase(), phone_number)
                }
                None => {
                    ctx.send(poise::CreateReply::default()
                        .content("❌ I don't remember your phone number. Please provide both plate and phone number for the first time.")
                        .ephemeral(true))
                        .await?;
                    return Ok(());
                }
            }
        }
        // Only phone provided - use stored plate if available
        (None, Some(ph)) => {
            if !ph.chars().all(|c| c.is_ascii_digit()) {
                ctx.send(poise::CreateReply::default()
                    .content("❌ Phone number should contain only digits (no spaces, dashes, or country code)")
                    .ephemeral(true))
                    .await?;
                return Ok(());
            }

            match data.users.get(&user_id).cloned() {
                Some(stored_info) => {
                    // Update stored phone
                    let mut updated_info = stored_info.clone();
                    updated_info.phone_number = ph.clone();
                    let plate = stored_info.plate.clone();
                    data.users.insert(user_id, updated_info);
                    if let Err(e) = save_parking_data(&data) {
                        log::warn!("Failed to save parking data: {}", e);
                    }
                    (plate, ph)
                }
                None => {
                    ctx.send(poise::CreateReply::default()
                        .content("❌ I don't remember your license plate. Please provide both plate and phone number for the first time.")
                        .ephemeral(true))
                        .await?;
                    return Ok(());
                }
            }
        }
        // Neither provided - use stored info if available
        (None, None) => match data.users.get(&user_id) {
            Some(stored_info) => (stored_info.plate.clone(), stored_info.phone_number.clone()),
            None => {
                ctx.send(poise::CreateReply::default()
                    .content("❌ I don't remember your information. Please provide both your license plate and phone number.")
                    .ephemeral(true))
                    .await?;
                return Ok(());
            }
        },
    };

    // Send initial response
    let initial_reply = ctx
        .send(
            poise::CreateReply::default()
                .content("🚗 Processing parking request...")
                .ephemeral(true),
        )
        .await?;

    // Create the HTTP client
    let client = Client::new();

    // Prepare the request payload
    let payload = json!({
        "email": "",
        "PhoneNumber": format!("45{}", final_phone),
        "VehicleRegistrationCountry": "DK",
        "Duration": 600,
        "VehicleRegistration": final_plate,
        "parkingAreas": [
            {
                "ParkingAreaId": 1956,
                "ParkingAreaKey": "ADK-4688"
            }
        ],
        "UId": "12cdf204-d969-469a-9bd5-c1f1fc59ee34",
        "Lang": "da"
    });

    // Make the API request
    match client
        .post("https://api.mobile-parking.eu/v10/permit/Tablet/confirm")
        .json(&payload)
        .send()
        .await
    {
        Ok(response) => {
            let status = response.status();
            let response_text = response.text().await.unwrap_or_default();

            if status.is_success() {
                // Parse response to get meaningful information if possible
                let success_message = if let Ok(_json_response) =
                    serde_json::from_str::<serde_json::Value>(&response_text)
                {
                    format!(
                        "✅ **Parking confirmed!**\n🚗 **Plate:** {}\n📱 **Phone:** +45 {}\n⏱️ **Duration:** 10 hours. Validate that you got an SMS with the correct information.\n📍 **Area:** ADK-4688\n\n💾 *Your information has been saved for next time*",
                        final_plate,
                        final_phone
                    )
                } else {
                    format!(
                        "✅ **Parking request sent!**\n🚗 **Plate:** {}\n📱 **Phone:** +45 {}\n⏱️ **Duration:** 10 hours\n\n💾 *Your information has been saved for next time*",
                        final_plate,
                        final_phone
                    )
                };

                initial_reply
                    .edit(
                        ctx,
                        poise::CreateReply::default()
                            .content(success_message)
                            .ephemeral(true),
                    )
                    .await?;

                log::info!(
                    "Parking request successful for user {} - plate: {}, phone: +45{}",
                    ctx.author().name,
                    final_plate,
                    final_phone
                );
            } else {
                let error_message = format!(
                    "❌ **Parking request failed**\n**Status:** {}\n**Response:** {}",
                    status,
                    if response_text.is_empty() {
                        "No response body"
                    } else {
                        &response_text
                    }
                );

                initial_reply
                    .edit(
                        ctx,
                        poise::CreateReply::default()
                            .content(error_message)
                            .ephemeral(true),
                    )
                    .await?;

                log::error!(
                    "Parking request failed for user {} - Status: {}, Response: {}",
                    ctx.author().name,
                    status,
                    response_text
                );
            }
        }
        Err(e) => {
            let error_message = format!("❌ **Network error occurred**\n**Error:** {}", e);

            initial_reply
                .edit(
                    ctx,
                    poise::CreateReply::default()
                        .content(error_message)
                        .ephemeral(true),
                )
                .await?;

            log::error!(
                "Network error during parking request for user {}: {}",
                ctx.author().name,
                e
            );
        }
    }

    Ok(())
}

/// View your saved parking information
#[poise::command(prefix_command, slash_command, rename = "info")]
pub async fn park_info(ctx: Context<'_>) -> Result<(), Error> {
    let user_id = ctx.author().id.get();
    let data = load_parking_data();

    log::info!("Park info command called by {}", ctx.author().name);

    match data.users.get(&user_id) {
        Some(info) => {
            let message = format!(
                "📋 **Your Saved Parking Information**\n🚗 **Plate:** {}\n📱 **Phone:** +45{}\n\n💡 *Use `/park now` without arguments to park with this info*",
                info.plate,
                info.phone_number
            );
            ctx.send(
                poise::CreateReply::default()
                    .content(message)
                    .ephemeral(true),
            )
            .await?;
        }
        None => {
            ctx.send(poise::CreateReply::default()
                .content("📭 No parking information saved. Use `/park now <plate> <phone>` to save your info.")
                .ephemeral(true))
                .await?;
        }
    }

    Ok(())
}

/// Clear your saved parking information
#[poise::command(prefix_command, slash_command, rename = "clear")]
pub async fn park_clear(ctx: Context<'_>) -> Result<(), Error> {
    let user_id = ctx.author().id.get();
    let mut data = load_parking_data();

    log::info!("Park clear command called by {}", ctx.author().name);

    if data.users.remove(&user_id).is_some() {
        if let Err(e) = save_parking_data(&data) {
            ctx.send(
                poise::CreateReply::default()
                    .content(format!("❌ Failed to clear data: {}", e))
                    .ephemeral(true),
            )
            .await?;
            return Ok(());
        }

        ctx.send(poise::CreateReply::default()
            .content("🗑️ **Parking information cleared**\nYour saved plate and phone number have been removed.")
            .ephemeral(true))
            .await?;

        log::info!("Cleared parking data for user {}", ctx.author().name);
    } else {
        ctx.send(
            poise::CreateReply::default()
                .content("📭 No parking information found to clear.")
                .ephemeral(true),
        )
        .await?;
    }

    Ok(())
}

/// Schedule automatic parking for weekdays
#[poise::command(
    prefix_command,
    slash_command,
    rename = "schedule",
    subcommands("schedule_set", "schedule_status", "schedule_disable")
)]
pub async fn park_schedule(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Set automatic parking schedule for weekdays
#[poise::command(prefix_command, slash_command, rename = "set")]
pub async fn schedule_set(
    ctx: Context<'_>,
    #[description = "Hour (0-23)"] hour: u8,
    #[description = "Minute (0-59)"] minute: u8,
) -> Result<(), Error> {
    let user_id = ctx.author().id.get();
    let mut data = load_parking_data();

    log::info!(
        "Schedule set command called by {} with time: {}:{}",
        ctx.author().name,
        hour,
        minute
    );

    // Validate time
    if hour > 23 || minute > 59 {
        ctx.send(
            poise::CreateReply::default()
                .content("❌ Invalid time format! Hour must be 0-23, minute must be 0-59")
                .ephemeral(true),
        )
        .await?;
        return Ok(());
    }

    // Check if user has parking info
    if !data.users.contains_key(&user_id) {
        ctx.send(poise::CreateReply::default()
            .content("❌ You need to save your parking information first. Use `/park now <plate> <phone>` to set it up.")
            .ephemeral(true))
            .await?;
        return Ok(());
    }

    let schedule = ParkingSchedule {
        user_id,
        hour,
        minute,
        enabled: true,
        last_parked: None,
        missed_requests: Vec::new(),
    };

    data.schedules.insert(user_id, schedule);

    if let Err(e) = save_parking_data(&data) {
        ctx.send(
            poise::CreateReply::default()
                .content(format!("❌ Failed to save schedule: {}", e))
                .ephemeral(true),
        )
        .await?;
        return Ok(());
    }

    let message = format!(
        "⏰ **Automatic parking scheduled!**\n🕐 **Time:** {:02}:{:02} (Danish time)\n📅 **Days:** Monday to Friday\n🔔 **Notifications:** You'll receive a DM when parking is registered and when it expires\n⏰ **DST:** Automatically adjusts for daylight saving time\n\n💡 *Use `/park schedule status` to check your schedule*",
        hour,
        minute
    );

    ctx.send(
        poise::CreateReply::default()
            .content(message)
            .ephemeral(true),
    )
    .await?;

    log::info!(
        "Parking schedule set for user {} at {:02}:{}",
        ctx.author().name,
        hour,
        minute
    );

    Ok(())
}

/// Check your automatic parking schedule status
#[poise::command(prefix_command, slash_command, rename = "status")]
pub async fn schedule_status(ctx: Context<'_>) -> Result<(), Error> {
    let user_id = ctx.author().id.get();
    let data = load_parking_data();

    log::info!("Schedule status command called by {}", ctx.author().name);

    match data.schedules.get(&user_id) {
        Some(schedule) if schedule.enabled => {
            let last_parked_text = match schedule.last_parked {
                Some(last) => format!("🕐 **Last parked:** <t:{}:F>", last.timestamp()),
                None => "🕐 **Last parked:** Never".to_string(),
            };

            let message = format!(
                "⏰ **Your Parking Schedule**\n🕐 **Time:** {:02}:{:02} (Danish time)\n📅 **Days:** Monday to Friday\n📊 **Status:** ✅ Enabled\n{}\n⏰ **DST:** Automatically adjusts for daylight saving time\n\n💡 *Use `/park schedule disable` to turn off*",
                schedule.hour,
                schedule.minute,
                last_parked_text
            );

            ctx.send(
                poise::CreateReply::default()
                    .content(message)
                    .ephemeral(true),
            )
            .await?;
        }
        Some(_) => {
            ctx.send(poise::CreateReply::default()
                .content("⏰ **Your Parking Schedule**\n📊 **Status:** ❌ Disabled\n\n💡 *Use `/park schedule set <hour> <minute>` to enable*")
                .ephemeral(true))
                .await?;
        }
        None => {
            ctx.send(poise::CreateReply::default()
                .content("📭 No parking schedule set. Use `/park schedule set <hour> <minute>` to create one.")
                .ephemeral(true))
                .await?;
        }
    }

    Ok(())
}

/// Disable your automatic parking schedule
#[poise::command(prefix_command, slash_command, rename = "disable")]
pub async fn schedule_disable(ctx: Context<'_>) -> Result<(), Error> {
    let user_id = ctx.author().id.get();
    let mut data = load_parking_data();

    log::info!("Schedule disable command called by {}", ctx.author().name);

    match data.schedules.get_mut(&user_id) {
        Some(schedule) if schedule.enabled => {
            schedule.enabled = false;

            if let Err(e) = save_parking_data(&data) {
                ctx.send(
                    poise::CreateReply::default()
                        .content(format!("❌ Failed to disable schedule: {}", e))
                        .ephemeral(true),
                )
                .await?;
                return Ok(());
            }

            ctx.send(poise::CreateReply::default()
                .content("⏰ **Automatic parking disabled**\nYour schedule has been turned off. Use `/park schedule set <hour> <minute>` to re-enable.")
                .ephemeral(true))
                .await?;

            log::info!("Disabled parking schedule for user {}", ctx.author().name);
        }
        Some(_) => {
            ctx.send(
                poise::CreateReply::default()
                    .content("📭 Your parking schedule is already disabled.")
                    .ephemeral(true),
            )
            .await?;
        }
        None => {
            ctx.send(
                poise::CreateReply::default()
                    .content("📭 No parking schedule found to disable.")
                    .ephemeral(true),
            )
            .await?;
        }
    }

    Ok(())
}

pub fn start_parking_scheduler(http: Arc<Http>) {
    tokio::spawn(async move {
        // First, check for any missed parking requests from when bot was down
        if let Err(e) = process_missed_parking_requests(&http).await {
            log::error!("Error processing missed parking requests: {e}");
        }

        let mut interval = interval(TokioDuration::from_secs(60)); // Check every minute

        loop {
            interval.tick().await;

            if let Err(e) = check_and_execute_parking(&http).await {
                log::error!("Error in parking scheduler: {e}");
            }

            if let Err(e) = check_parking_expiry(&http).await {
                log::error!("Error checking parking expiry: {e}");
            }
        }
    });
}

async fn check_and_execute_parking(http: &Http) -> Result<(), Error> {
    let mut data = load_parking_data();
    let now_utc = Utc::now();
    let now = now_utc.with_timezone(&Copenhagen);

    // Only run on weekdays (Monday = 1, Friday = 5)
    let weekday = now.weekday();
    if ![
        Weekday::Mon,
        Weekday::Tue,
        Weekday::Wed,
        Weekday::Thu,
        Weekday::Fri,
    ]
    .contains(&weekday)
    {
        return Ok(());
    }

    for (user_id, schedule) in data.schedules.iter_mut() {
        if !schedule.enabled {
            continue;
        }

        // Check if it's time to park (using Danish time)
        let target_time = Copenhagen
            .with_ymd_and_hms(
                now.year(),
                now.month(),
                now.day(),
                schedule.hour as u32,
                schedule.minute as u32,
                0,
            )
            .single();

        let target_time = match target_time {
            Some(time) => time,
            None => continue,
        };

        // Check if we should park now (within 1 minute window)
        let time_diff = (now - target_time).num_minutes().abs();
        if time_diff > 1 {
            continue;
        }

        // Check if we already parked today
        if let Some(last_parked) = schedule.last_parked {
            if last_parked.date_naive() == now_utc.date_naive() {
                continue; // Already parked today
            }
        }

        // Add to missed requests (in case bot shuts down before execution)
        schedule
            .missed_requests
            .push(target_time.with_timezone(&Utc));

        // Get user info
        let user_info = match data.users.get(user_id) {
            Some(info) => info.clone(),
            None => {
                // Send DM about missing info
                if let Err(e) = send_dm_to_user(
                    http,
                    UserId::new(*user_id),
                    "❌ **Automatic parking failed**\nYour parking information is missing. Please use `/park now <plate> <phone>` to set it up again.",
                ).await {
                    log::error!("Failed to send DM to user {}: {}", user_id, e);
                }
                continue;
            }
        };

        // Execute parking request
        match execute_parking_request(&user_info.plate, &user_info.phone_number).await {
            Ok(_) => {
                // Update last parked time and remove from missed requests
                schedule.last_parked = Some(now_utc);
                schedule
                    .missed_requests
                    .retain(|&req_time| req_time != target_time.with_timezone(&Utc));

                // Send success DM
                let message = format!(
                    "✅ **Automatic parking registered!**\n🚗 **Plate:** {}\n📱 **Phone:** +45{}\n⏱️ **Duration:** 10 hours\n📍 **Area:** ADK-4688\n\n📱 **Please check your SMS** for confirmation!",
                    user_info.plate,
                    user_info.phone_number
                );

                if let Err(e) = send_dm_to_user(http, UserId::new(*user_id), &message).await {
                    log::error!("Failed to send success DM to user {}: {}", user_id, e);
                }

                log::info!(
                    "Automatic parking executed for user {} at {:02}:{:02}",
                    user_id,
                    schedule.hour,
                    schedule.minute
                );
            }
            Err(e) => {
                // Keep in missed requests for retry
                // Send failure DM
                let message = format!(
                    "❌ **Automatic parking failed**\n**Error:** {}\n\n🔧 You may need to try parking manually with `/park now`",
                    e
                );

                if let Err(dm_err) = send_dm_to_user(http, UserId::new(*user_id), &message).await {
                    log::error!("Failed to send failure DM to user {}: {}", user_id, dm_err);
                }

                log::error!("Automatic parking failed for user {}: {}", user_id, e);
            }
        }
    }

    // Save updated data
    save_parking_data(&data)?;
    Ok(())
}

async fn process_missed_parking_requests(http: &Http) -> Result<(), Error> {
    let mut data = load_parking_data();
    let now = Utc::now();

    log::info!("Checking for missed parking requests...");

    for (user_id, schedule) in data.schedules.iter_mut() {
        if !schedule.enabled || schedule.missed_requests.is_empty() {
            continue;
        }

        // Process all missed requests from today (Danish time)
        let today = now.date_naive();
        let mut processed_requests = Vec::new();

        for &missed_time in &schedule.missed_requests {
            // Convert missed time to Danish timezone for comparison
            let missed_time_danish = missed_time.with_timezone(&Copenhagen);
            // Only process requests from today
            if missed_time_danish.date_naive() != today {
                processed_requests.push(missed_time);
                continue;
            }

            // Check if we already parked today
            if let Some(last_parked) = schedule.last_parked {
                if last_parked.date_naive() == today {
                    processed_requests.push(missed_time);
                    continue; // Already parked today
                }
            }

            log::info!(
                "Processing missed parking request for user {} from {}",
                user_id,
                missed_time
            );

            // Get user info
            let user_info = match data.users.get(user_id) {
                Some(info) => info.clone(),
                None => {
                    processed_requests.push(missed_time);
                    continue;
                }
            };

            // Execute the missed parking request
            match execute_parking_request(&user_info.plate, &user_info.phone_number).await {
                Ok(_) => {
                    // Update last parked time
                    schedule.last_parked = Some(now);
                    processed_requests.push(missed_time);

                    // Send success DM with note about recovery
                    let message = format!(
                        "✅ **Missed parking request recovered!**\n🚗 **Plate:** {}\n📱 **Phone:** +45{}\n⏱️ **Duration:** 10 hours\n📍 **Area:** ADK-4688\n⏰ **Originally scheduled:** <t:{}:t>\n\n📱 **Please check your SMS** for confirmation!\n\n🤖 *This was automatically processed after bot restart*",
                        user_info.plate,
                        user_info.phone_number,
                        missed_time.timestamp()
                    );

                    if let Err(e) = send_dm_to_user(http, UserId::new(*user_id), &message).await {
                        log::error!(
                            "Failed to send recovery success DM to user {}: {}",
                            user_id,
                            e
                        );
                    }

                    log::info!(
                        "Successfully processed missed parking request for user {}",
                        user_id
                    );
                    break; // Only process one missed request per day
                }
                Err(e) => {
                    // Keep the missed request for potential retry
                    log::error!(
                        "Failed to process missed parking request for user {}: {}",
                        user_id,
                        e
                    );

                    // Send failure DM
                    let message = format!(
                        "❌ **Missed parking request failed**\n**Error:** {}\n⏰ **Originally scheduled:** <t:{}:t>\n\n🔧 You may need to try parking manually with `/park now`\n\n🤖 *This was a recovery attempt after bot restart*",
                        e,
                        missed_time.timestamp()
                    );

                    if let Err(dm_err) =
                        send_dm_to_user(http, UserId::new(*user_id), &message).await
                    {
                        log::error!(
                            "Failed to send recovery failure DM to user {}: {}",
                            user_id,
                            dm_err
                        );
                    }
                }
            }
        }

        // Remove processed requests and old requests (older than today)
        schedule.missed_requests.retain(|&req_time| {
            !processed_requests.contains(&req_time) && req_time.date_naive() >= today
        });
    }

    // Save updated data
    save_parking_data(&data)?;
    log::info!("Finished processing missed parking requests");
    Ok(())
}

async fn check_parking_expiry(http: &Http) -> Result<(), Error> {
    let data = load_parking_data();
    let now = Utc::now();

    for (user_id, schedule) in data.schedules.iter() {
        if !schedule.enabled {
            continue;
        }

        if let Some(last_parked) = schedule.last_parked {
            // Check if parking expires in the next minute (10 hours after parking)
            let expiry_time = last_parked + Duration::hours(10);
            let time_until_expiry = (expiry_time - now).num_minutes();

            // Send reminder 1 minute before expiry
            if time_until_expiry == 1 {
                let expiry_time_danish = expiry_time.with_timezone(&Copenhagen);
                let message = format!(
                    "⏰ **Parking expires soon!**\n🚗 **Plate:** {}\n📍 **Area:** ADK-4688\n⏱️ **Expires:** <t:{}:t> ({})\n\n🚗 Your parking will expire in 1 minute!",
                    data.users.get(user_id).map(|u| u.plate.as_str()).unwrap_or("Unknown"),
                    expiry_time.timestamp(),
                    expiry_time_danish.format("%H:%M Danish time")
                );

                if let Err(e) = send_dm_to_user(http, UserId::new(*user_id), &message).await {
                    log::error!(
                        "Failed to send expiry warning DM to user {}: {}",
                        user_id,
                        e
                    );
                }
            }
        }
    }

    Ok(())
}

async fn execute_parking_request(
    plate: &str,
    phone_number: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let client = Client::new();

    let payload = json!({
        "email": "",
        "PhoneNumber": format!("45{}", phone_number),
        "VehicleRegistrationCountry": "DK",
        "Duration": 600,
        "VehicleRegistration": plate,
        "parkingAreas": [
            {
                "ParkingAreaId": 1956,
                "ParkingAreaKey": "ADK-4688"
            }
        ],
        "UId": "12cdf204-d969-469a-9bd5-c1f1fc59ee34",
        "Lang": "da"
    });

    let response = client
        .post("https://api.mobile-parking.eu/v10/permit/Tablet/confirm")
        .json(&payload)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        return Err(format!("API request failed: {} - {}", status, error_text).into());
    }

    Ok(())
}

async fn send_dm_to_user(
    http: &Http,
    user_id: UserId,
    message: &str,
) -> Result<(), serenity::Error> {
    let user = user_id.to_user(http).await?;
    let dm_channel = user.create_dm_channel(http).await?;
    dm_channel.say(http, message).await?;
    Ok(())
}
