use crate::{Context, Error};
use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose, Engine as _};
use chrono::{DateTime, Datelike, Duration, TimeZone, Utc, Weekday};
use chrono_tz::Europe::Copenhagen;
use governor::{
    clock::DefaultClock,
    state::{direct::NotKeyed, InMemoryState},
    Quota, RateLimiter,
};
use nonzero_ext::*;
use parking_lot::RwLock;
use poise::serenity_prelude as serenity;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use serenity::{Http, UserId};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tokio::time::{interval, Duration as TokioDuration};
use uuid::Uuid;

// Rate limiter: 3 requests per user per hour for parking
type ParkingRateLimiter = RateLimiter<NotKeyed, InMemoryState, DefaultClock>;

#[derive(Serialize, Deserialize, Clone)]
struct UserParkingInfo {
    phone_number: String, // Encrypted
    plate: String,        // Encrypted
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
    #[serde(skip)]
    encryption_key: Option<Vec<u8>>,
}

// Global state for parking data with thread-safe access
lazy_static::lazy_static! {
    static ref PARKING_DATA: Arc<RwLock<ParkingData>> = Arc::new(RwLock::new(ParkingData::default()));
    static ref RATE_LIMITERS: Arc<RwLock<HashMap<u64, ParkingRateLimiter>>> = Arc::new(RwLock::new(HashMap::new()));
}

const PARKING_DATA_FILE: &str = "/var/lib/rustbot/parking_data.json";
const ENCRYPTION_KEY_FILE: &str = "/var/lib/rustbot/parking_key";

// Encryption functions
fn generate_encryption_key() -> Vec<u8> {
    Aes256Gcm::generate_key(&mut OsRng).to_vec()
}

fn load_or_create_encryption_key() -> Result<Vec<u8>, Error> {
    if Path::new(ENCRYPTION_KEY_FILE).exists() {
        let key_data = fs::read(ENCRYPTION_KEY_FILE)?;
        if key_data.len() == 32 {
            Ok(key_data)
        } else {
            log::warn!("Invalid encryption key size, generating new key");
            let key = generate_encryption_key();
            fs::write(ENCRYPTION_KEY_FILE, &key)?;
            Ok(key)
        }
    } else {
        let key = generate_encryption_key();
        fs::write(ENCRYPTION_KEY_FILE, &key)?;
        Ok(key)
    }
}

fn encrypt_data(
    data: &str,
    key: &[u8],
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let cipher = Aes256Gcm::new_from_slice(key)?;
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let ciphertext = cipher
        .encrypt(&nonce, data.as_bytes())
        .map_err(|e| format!("Encryption failed: {}", e))?;

    // Combine nonce and ciphertext, then base64 encode
    let mut combined = nonce.to_vec();
    combined.extend_from_slice(&ciphertext);
    Ok(general_purpose::STANDARD.encode(combined))
}

fn decrypt_data(
    encrypted: &str,
    key: &[u8],
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let cipher = Aes256Gcm::new_from_slice(key)?;
    let combined = general_purpose::STANDARD.decode(encrypted)?;

    if combined.len() < 12 {
        return Err("Invalid encrypted data".into());
    }

    let (nonce_bytes, ciphertext) = combined.split_at(12);
    let nonce = Nonce::from_slice(nonce_bytes);
    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| format!("Decryption failed: {}", e))?;

    Ok(String::from_utf8(plaintext)?)
}

// Validation functions
fn validate_danish_phone_number(phone: &str) -> bool {
    phone.len() == 8 && phone.chars().all(|c| c.is_ascii_digit())
}

fn validate_danish_license_plate(plate: &str) -> bool {
    let trimmed = plate.trim();
    !trimmed.is_empty() && trimmed.len() <= 10 && trimmed.len() >= 2
}

fn is_valid_time(hour: u8, minute: u8) -> bool {
    hour <= 23 && minute <= 59
}

// Time handling functions
fn is_weekday(datetime: &DateTime<chrono_tz::Tz>) -> bool {
    matches!(
        datetime.weekday(),
        Weekday::Mon | Weekday::Tue | Weekday::Wed | Weekday::Thu | Weekday::Fri
    )
}

fn is_schedule_time_match(
    schedule_time: &DateTime<chrono_tz::Tz>,
    current_time: &DateTime<chrono_tz::Tz>,
) -> bool {
    let time_diff = (*current_time - *schedule_time).num_seconds().abs();
    time_diff <= 30 // Within 30 seconds
}

// Rate limiting
fn create_rate_limiter() -> ParkingRateLimiter {
    RateLimiter::direct(Quota::per_hour(nonzero!(3u32)))
}

fn check_rate_limit(user_id: u64) -> bool {
    let mut limiters = RATE_LIMITERS.write();
    let limiter = limiters.entry(user_id).or_insert_with(create_rate_limiter);
    limiter.check().is_ok()
}

// Data persistence
fn ensure_data_directory() -> Result<(), Error> {
    let dir = Path::new("/var/lib/rustbot");
    if !dir.exists() {
        fs::create_dir_all(dir)?;

        // Set secure permissions (readable/writable only by owner)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(dir)?.permissions();
            perms.set_mode(0o700);
            fs::set_permissions(dir, perms)?;
        }
    }
    Ok(())
}

fn load_parking_data() -> Result<(), Error> {
    ensure_data_directory()?;

    let encryption_key = load_or_create_encryption_key()?;

    let mut data = if Path::new(PARKING_DATA_FILE).exists() {
        match fs::read_to_string(PARKING_DATA_FILE) {
            Ok(content) => {
                match serde_json::from_str::<ParkingData>(&content) {
                    Ok(mut parsed_data) => {
                        // Decrypt user data
                        for user_info in parsed_data.users.values_mut() {
                            if let Ok(decrypted_phone) =
                                decrypt_data(&user_info.phone_number, &encryption_key)
                            {
                                user_info.phone_number = decrypted_phone;
                            }
                            if let Ok(decrypted_plate) =
                                decrypt_data(&user_info.plate, &encryption_key)
                            {
                                user_info.plate = decrypted_plate;
                            }
                        }
                        parsed_data
                    }
                    Err(e) => {
                        log::warn!("Failed to parse parking data: {}, starting fresh", e);
                        ParkingData::default()
                    }
                }
            }
            Err(e) => {
                log::warn!("Failed to read parking data: {}, starting fresh", e);
                ParkingData::default()
            }
        }
    } else {
        ParkingData::default()
    };

    data.encryption_key = Some(encryption_key);

    // Clean up old missed requests
    let now = Utc::now();
    for schedule in data.schedules.values_mut() {
        cleanup_old_missed_requests(&mut schedule.missed_requests, now);
    }

    *PARKING_DATA.write() = data;
    Ok(())
}

fn save_parking_data() -> Result<(), Error> {
    ensure_data_directory()?;

    let data = PARKING_DATA.read();
    let encryption_key = data
        .encryption_key
        .as_ref()
        .ok_or_else(|| Error::from("Encryption key not available"))?;

    // Create a copy for serialization with encrypted data
    let mut save_data = ParkingData {
        users: HashMap::new(),
        schedules: data.schedules.clone(),
        encryption_key: None,
    };

    // Encrypt user data before saving
    for (user_id, user_info) in &data.users {
        let encrypted_phone = encrypt_data(&user_info.phone_number, encryption_key)
            .map_err(|e| Error::from(format!("Failed to encrypt phone: {}", e)))?;
        let encrypted_plate = encrypt_data(&user_info.plate, encryption_key)
            .map_err(|e| Error::from(format!("Failed to encrypt plate: {}", e)))?;

        save_data.users.insert(
            *user_id,
            UserParkingInfo {
                phone_number: encrypted_phone,
                plate: encrypted_plate,
            },
        );
    }

    let json = serde_json::to_string_pretty(&save_data)?;
    fs::write(PARKING_DATA_FILE, json)?;

    // Set secure file permissions
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(PARKING_DATA_FILE)?.permissions();
        perms.set_mode(0o600);
        fs::set_permissions(PARKING_DATA_FILE, perms)?;
    }

    Ok(())
}

// Utility functions
fn cleanup_old_missed_requests(requests: &mut Vec<DateTime<Utc>>, now: DateTime<Utc>) {
    let today = now.date_naive();
    requests.retain(|&req_time| req_time.date_naive() >= today);
}

fn generate_unique_request_id() -> String {
    Uuid::new_v4().to_string()
}

fn create_parking_payload(plate: &str, phone: &str) -> serde_json::Value {
    json!({
        "email": "",
        "PhoneNumber": format!("45{}", phone),
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
    })
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
    #[description = "Phone number (8 digits, no country code) - optional if previously saved"]
    phone_number: Option<String>,
) -> Result<(), Error> {
    let user_id = ctx.author().id.get();

    // Check rate limit
    if !check_rate_limit(user_id) {
        ctx.send(poise::CreateReply::default()
            .content("🚫 **Rate limit exceeded**\nYou can only park 3 times per hour. Please wait before trying again.")
            .ephemeral(true))
            .await?;
        return Ok(());
    }

    log::info!(
        "Park command called by {} with plate: '{:?}' and phone: '{:?}'",
        ctx.author().name,
        plate,
        phone_number
    );

    // Determine which info to use
    let current_user_info = {
        let data_guard = PARKING_DATA.read();
        data_guard.users.get(&user_id).cloned()
    };

    let (final_plate, final_phone) = match (plate, phone_number) {
        // Both provided - validate and save
        (Some(p), Some(ph)) => {
            if !validate_danish_phone_number(&ph) {
                ctx.send(poise::CreateReply::default()
                    .content("❌ **Invalid phone number**\nPhone number must be exactly 8 digits (Danish format, no country code)")
                    .ephemeral(true))
                    .await?;
                return Ok(());
            }

            if !validate_danish_license_plate(&p) {
                ctx.send(
                    poise::CreateReply::default()
                        .content("❌ **Invalid license plate**\nLicense plate format is invalid")
                        .ephemeral(true),
                )
                .await?;
                return Ok(());
            }

            let user_info = UserParkingInfo {
                phone_number: ph.clone(),
                plate: p.to_uppercase(),
            };

            {
                let mut data = PARKING_DATA.write();
                data.users.insert(user_id, user_info);
            }
            (p.to_uppercase(), ph)
        }
        // Only plate provided
        (Some(p), None) => {
            if !validate_danish_license_plate(&p) {
                ctx.send(
                    poise::CreateReply::default()
                        .content("❌ **Invalid license plate**\nLicense plate format is invalid")
                        .ephemeral(true),
                )
                .await?;
                return Ok(());
            }

            match current_user_info {
                Some(mut stored_info) => {
                    stored_info.plate = p.to_uppercase();
                    let phone_number = stored_info.phone_number.clone();
                    {
                        let mut data = PARKING_DATA.write();
                        data.users.insert(user_id, stored_info);
                    }
                    (p.to_uppercase(), phone_number)
                }
                None => {
                    ctx.send(poise::CreateReply::default()
                        .content("❌ **Phone number required**\nI don't have your phone number saved. Please provide both plate and phone number.")
                        .ephemeral(true))
                        .await?;
                    return Ok(());
                }
            }
        }
        // Only phone provided
        (None, Some(ph)) => {
            if !validate_danish_phone_number(&ph) {
                ctx.send(poise::CreateReply::default()
                    .content("❌ **Invalid phone number**\nPhone number must be exactly 8 digits (Danish format, no country code)")
                    .ephemeral(true))
                    .await?;
                return Ok(());
            }

            match current_user_info {
                Some(mut stored_info) => {
                    stored_info.phone_number = ph.clone();
                    let plate = stored_info.plate.clone();
                    {
                        let mut data = PARKING_DATA.write();
                        data.users.insert(user_id, stored_info);
                    }
                    (plate, ph)
                }
                None => {
                    ctx.send(poise::CreateReply::default()
                        .content("❌ **License plate required**\nI don't have your license plate saved. Please provide both plate and phone number.")
                        .ephemeral(true))
                        .await?;
                    return Ok(());
                }
            }
        }
        // Neither provided
        (None, None) => match current_user_info {
            Some(stored_info) => (stored_info.plate.clone(), stored_info.phone_number.clone()),
            None => {
                ctx.send(poise::CreateReply::default()
                    .content("❌ **Information required**\nI don't have your parking information. Please provide both your license plate and phone number.")
                    .ephemeral(true))
                    .await?;
                return Ok(());
            }
        },
    };

    // Save data after modifications
    if let Err(e) = save_parking_data() {
        log::warn!("Failed to save parking data: {}", e);
    }

    // Send initial response
    let initial_reply = ctx
        .send(
            poise::CreateReply::default()
                .content("🚗 Processing parking request...")
                .ephemeral(true),
        )
        .await?;

    // Execute parking request
    match execute_parking_request(&final_plate, &final_phone).await {
        Ok(_) => {
            let success_message = format!(
                "✅ **Parking confirmed!**\n🚗 **Plate:** {}\n📱 **Phone:** +45 {}\n⏱️ **Duration:** 10 hours\n📍 **Area:** ADK-4688\n\n📱 **Please check your SMS** for confirmation!\n💾 *Your information has been saved securely*",
                final_plate,
                final_phone
            );

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
        }
        Err(e) => {
            let error_message = format!(
                "❌ **Parking request failed**\n**Error:** {}\n\n🔧 Please try again in a few minutes.",
                e
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
                "Parking request failed for user {}: {}",
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

    log::info!("Park info command called by {}", ctx.author().name);

    let (user_info, schedule_info) = {
        let data = PARKING_DATA.read();
        let user_info = data.users.get(&user_id).cloned();
        let schedule_info = match data.schedules.get(&user_id) {
            Some(schedule) if schedule.enabled => {
                let last_parked = match schedule.last_parked {
                    Some(last) => format!("<t:{}:F>", last.timestamp()),
                    None => "Never".to_string(),
                };
                format!(
                    "\n\n⏰ **Schedule:** {:02}:{:02} (Mon-Fri)\n📊 **Status:** ✅ Enabled\n🕐 **Last auto-park:** {}",
                    schedule.hour, schedule.minute, last_parked
                )
            }
            Some(_) => "\n\n⏰ **Schedule:** ❌ Disabled".to_string(),
            None => "\n\n⏰ **Schedule:** Not set".to_string(),
        };
        (user_info, schedule_info)
    };

    match user_info {
        Some(info) => {
            let message = format!(
                "📋 **Your Parking Information**\n🚗 **License Plate:** {}\n📱 **Phone:** +45 {}{}\n\n💡 *Use `/park clear` to remove this information*",
                info.plate,
                info.phone_number,
                schedule_info
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
                .content("📭 **No parking information found**\nUse `/park now <plate> <phone>` to save your information.")
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

    log::info!("Park clear command called by {}", ctx.author().name);

    let removed = {
        let mut data = PARKING_DATA.write();
        let user_removed = data.users.remove(&user_id).is_some();
        let schedule_removed = data.schedules.remove(&user_id).is_some();
        user_removed || schedule_removed
    };

    if removed {
        if let Err(e) = save_parking_data() {
            log::warn!("Failed to save parking data after clear: {}", e);
        }

        ctx.send(poise::CreateReply::default()
            .content("🗑️ **Parking information cleared**\nYour saved plate, phone number, and schedule have been removed.")
            .ephemeral(true))
            .await?;

        log::info!("Cleared parking data for user {}", ctx.author().name);
    } else {
        ctx.send(
            poise::CreateReply::default()
                .content("📭 **No parking information found to clear**")
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

    log::info!(
        "Schedule set command called by {} with time: {}:{}",
        ctx.author().name,
        hour,
        minute
    );

    // Validate time
    if !is_valid_time(hour, minute) {
        ctx.send(
            poise::CreateReply::default()
                .content("❌ **Invalid time**\nHour must be 0-23, minute must be 0-59")
                .ephemeral(true),
        )
        .await?;
        return Ok(());
    }

    // Check if user has parking info and set schedule
    let success = {
        let mut data = PARKING_DATA.write();

        if !data.users.contains_key(&user_id) {
            false
        } else {
            let schedule = ParkingSchedule {
                user_id,
                hour,
                minute,
                enabled: true,
                last_parked: None,
                missed_requests: Vec::new(),
            };

            data.schedules.insert(user_id, schedule);
            true
        }
    };

    if !success {
        ctx.send(poise::CreateReply::default()
            .content("❌ **Parking information required**\nYou need to save your parking information first. Use `/park now <plate> <phone>` to set it up.")
            .ephemeral(true))
            .await?;
        return Ok(());
    }

    if let Err(e) = save_parking_data() {
        ctx.send(
            poise::CreateReply::default()
                .content(format!("❌ **Failed to save schedule:** {}", e))
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

    log::info!("Schedule status command called by {}", ctx.author().name);

    let schedule = {
        let data = PARKING_DATA.read();
        data.schedules.get(&user_id).cloned()
    };

    match schedule {
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
                .content("📭 **No parking schedule set**\nUse `/park schedule set <hour> <minute>` to create one.")
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

    log::info!("Schedule disable command called by {}", ctx.author().name);

    let result = {
        let mut data = PARKING_DATA.write();
        match data.schedules.get_mut(&user_id) {
            Some(schedule) if schedule.enabled => {
                schedule.enabled = false;
                "disabled"
            }
            Some(_) => "already_disabled",
            None => "not_found",
        }
    };

    match result {
        "disabled" => {
            if let Err(e) = save_parking_data() {
                ctx.send(
                    poise::CreateReply::default()
                        .content(format!("❌ **Failed to disable schedule:** {}", e))
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
        "already_disabled" => {
            ctx.send(
                poise::CreateReply::default()
                    .content("📭 **Your parking schedule is already disabled**")
                    .ephemeral(true),
            )
            .await?;
        }
        "not_found" => {
            ctx.send(
                poise::CreateReply::default()
                    .content("📭 **No parking schedule found to disable**")
                    .ephemeral(true),
            )
            .await?;
        }
        _ => {}
    }

    Ok(())
}

pub fn start_parking_scheduler(http: Arc<Http>) {
    tokio::spawn(async move {
        // Initialize parking data
        if let Err(e) = load_parking_data() {
            log::error!("Failed to load parking data: {}", e);
            return;
        }

        // Process missed parking requests from when bot was down
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
    let now_utc = Utc::now();
    let now = now_utc.with_timezone(&Copenhagen);

    // Only run on weekdays
    if !is_weekday(&now) {
        return Ok(());
    }

    let schedules_to_process: Vec<(u64, ParkingSchedule, UserParkingInfo)> = {
        let data = PARKING_DATA.read();

        data.schedules
            .iter()
            .filter_map(|(user_id, schedule)| {
                if !schedule.enabled {
                    return None;
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
                    .single()?;

                // Check if we should park now (within 30 seconds)
                if !is_schedule_time_match(&target_time, &now) {
                    return None;
                }

                // Check if we already parked today
                if let Some(last_parked) = schedule.last_parked {
                    if last_parked.date_naive() == now_utc.date_naive() {
                        return None; // Already parked today
                    }
                }

                // Get user info
                data.users
                    .get(user_id)
                    .map(|user_info| (*user_id, schedule.clone(), user_info.clone()))
            })
            .collect()
    };

    for (user_id, mut schedule, user_info) in schedules_to_process {
        // Add to missed requests (in case bot shuts down before execution)
        let target_time = Copenhagen
            .with_ymd_and_hms(
                now.year(),
                now.month(),
                now.day(),
                schedule.hour as u32,
                schedule.minute as u32,
                0,
            )
            .single()
            .unwrap();

        schedule
            .missed_requests
            .push(target_time.with_timezone(&Utc));

        // Execute parking request
        match execute_parking_request(&user_info.plate, &user_info.phone_number).await {
            Ok(_) => {
                // Update last parked time and remove from missed requests
                {
                    let mut data = PARKING_DATA.write();
                    if let Some(schedule) = data.schedules.get_mut(&user_id) {
                        schedule.last_parked = Some(now_utc);
                        schedule
                            .missed_requests
                            .retain(|&req_time| req_time != target_time.with_timezone(&Utc));
                    }
                }

                // Send success DM
                let message = format!(
                    "✅ **Automatic parking registered!**\n🚗 **Plate:** {}\n📱 **Phone:** +45{}\n⏱️ **Duration:** 10 hours\n📍 **Area:** ADK-4688\n\n📱 **Please check your SMS** for confirmation!",
                    user_info.plate,
                    user_info.phone_number
                );

                if let Err(e) = send_dm_to_user(http, UserId::new(user_id), &message).await {
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
                // Send failure DM
                let message = format!(
                    "❌ **Automatic parking failed**\n**Error:** {}\n\n🔧 You may need to try parking manually with `/park now`",
                    e
                );

                if let Err(dm_err) = send_dm_to_user(http, UserId::new(user_id), &message).await {
                    log::error!("Failed to send failure DM to user {}: {}", user_id, dm_err);
                }

                log::error!("Automatic parking failed for user {}: {}", user_id, e);
            }
        }
    }

    // Save updated data
    if let Err(e) = save_parking_data() {
        log::error!("Failed to save parking data: {}", e);
    }

    Ok(())
}

async fn process_missed_parking_requests(http: &Http) -> Result<(), Error> {
    let now = Utc::now();
    let today = now.date_naive();

    log::info!("Checking for missed parking requests...");

    let missed_requests_to_process: Vec<(u64, DateTime<Utc>, UserParkingInfo)> = {
        let data = PARKING_DATA.read();

        data.schedules
            .iter()
            .filter_map(|(user_id, schedule)| {
                if !schedule.enabled || schedule.missed_requests.is_empty() {
                    return None;
                }

                // Check if we already parked today
                if let Some(last_parked) = schedule.last_parked {
                    if last_parked.date_naive() == today {
                        return None; // Already parked today
                    }
                }

                // Find the most recent missed request from today
                let missed_request = schedule
                    .missed_requests
                    .iter()
                    .filter(|&&req_time| {
                        let req_time_danish = req_time.with_timezone(&Copenhagen);
                        req_time_danish.date_naive() == today
                    })
                    .max()?;

                data.users
                    .get(user_id)
                    .map(|user_info| (*user_id, *missed_request, user_info.clone()))
            })
            .collect()
    };

    for (user_id, missed_time, user_info) in missed_requests_to_process {
        log::info!(
            "Processing missed parking request for user {} from {}",
            user_id,
            missed_time
        );

        // Execute the missed parking request
        match execute_parking_request(&user_info.plate, &user_info.phone_number).await {
            Ok(_) => {
                // Update last parked time and remove processed request
                {
                    let mut data = PARKING_DATA.write();
                    if let Some(schedule) = data.schedules.get_mut(&user_id) {
                        schedule.last_parked = Some(now);
                        schedule
                            .missed_requests
                            .retain(|&req_time| req_time != missed_time);
                    }
                }

                // Send success DM with note about recovery
                let message = format!(
                    "✅ **Missed parking request recovered!**\n🚗 **Plate:** {}\n📱 **Phone:** +45{}\n⏱️ **Duration:** 10 hours\n📍 **Area:** ADK-4688\n⏰ **Originally scheduled:** <t:{}:t>\n\n📱 **Please check your SMS** for confirmation!\n\n🤖 *This was automatically processed after bot restart*",
                    user_info.plate,
                    user_info.phone_number,
                    missed_time.timestamp()
                );

                if let Err(e) = send_dm_to_user(http, UserId::new(user_id), &message).await {
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
            }
            Err(e) => {
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

                if let Err(dm_err) = send_dm_to_user(http, UserId::new(user_id), &message).await {
                    log::error!(
                        "Failed to send recovery failure DM to user {}: {}",
                        user_id,
                        dm_err
                    );
                }
            }
        }
    }

    // Clean up old missed requests
    {
        let mut data = PARKING_DATA.write();
        for schedule in data.schedules.values_mut() {
            cleanup_old_missed_requests(&mut schedule.missed_requests, now);
        }
    }

    // Save updated data
    if let Err(e) = save_parking_data() {
        log::error!("Failed to save parking data: {}", e);
    }

    log::info!("Finished processing missed parking requests");
    Ok(())
}

async fn check_parking_expiry(http: &Http) -> Result<(), Error> {
    let now = Utc::now();

    let expiry_notifications: Vec<(u64, String, DateTime<Utc>)> = {
        let data = PARKING_DATA.read();

        data.schedules
            .iter()
            .filter_map(|(user_id, schedule)| {
                if !schedule.enabled {
                    return None;
                }

                let last_parked = schedule.last_parked?;

                // Check if parking expires in the next minute (10 hours after parking)
                let expiry_time = last_parked + Duration::hours(10);
                let time_until_expiry = (expiry_time - now).num_minutes();

                // Send reminder 1 minute before expiry
                if time_until_expiry == 1 {
                    let plate = data.users.get(user_id)?.plate.clone();
                    Some((*user_id, plate, expiry_time))
                } else {
                    None
                }
            })
            .collect()
    };

    for (user_id, plate, expiry_time) in expiry_notifications {
        let expiry_time_danish = expiry_time.with_timezone(&Copenhagen);
        let message = format!(
            "⏰ **Parking expires soon!**\n🚗 **Plate:** {}\n📍 **Area:** ADK-4688\n⏱️ **Expires:** <t:{}:t> ({})\n\n🚗 Your parking will expire in 1 minute!",
            plate,
            expiry_time.timestamp(),
            expiry_time_danish.format("%H:%M Danish time")
        );

        if let Err(e) = send_dm_to_user(http, UserId::new(user_id), &message).await {
            log::error!(
                "Failed to send expiry warning DM to user {}: {}",
                user_id,
                e
            );
        }
    }

    Ok(())
}

async fn execute_parking_request(
    plate: &str,
    phone_number: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let client = Client::new();
    let payload = create_parking_payload(plate, phone_number);

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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    #[test]
    fn test_validate_danish_phone_number() {
        // Valid Danish phone numbers (8 digits)
        assert!(validate_danish_phone_number("12345678"));
        assert!(validate_danish_phone_number("87654321"));

        // Invalid phone numbers
        assert!(!validate_danish_phone_number("1234567")); // Too short
        assert!(!validate_danish_phone_number("123456789")); // Too long
        assert!(!validate_danish_phone_number("1234567a")); // Contains letter
        assert!(!validate_danish_phone_number("1234-5678")); // Contains dash
        assert!(!validate_danish_phone_number("+4512345678")); // Contains country code
        assert!(!validate_danish_phone_number("")); // Empty
    }

    #[test]
    fn test_validate_danish_license_plate() {
        // Valid Danish license plates
        assert!(validate_danish_license_plate("AB12345"));
        assert!(validate_danish_license_plate("XY98765"));
        assert!(validate_danish_license_plate("ab12345")); // Should accept lowercase

        // Invalid license plates
        assert!(!validate_danish_license_plate("")); // Empty
        assert!(!validate_danish_license_plate("A")); // Too short
        assert!(!validate_danish_license_plate("ABCDEFGHIJK")); // Too long
        assert!(!validate_danish_license_plate("AB1234567890")); // Way too long
        assert!(!validate_danish_license_plate("   ")); // Only whitespace
    }

    #[test]
    fn test_parking_schedule_validation() {
        // Valid times
        assert!(is_valid_time(0, 0));
        assert!(is_valid_time(23, 59));
        assert!(is_valid_time(12, 30));

        // Invalid times
        assert!(!is_valid_time(24, 0)); // Hour too high
        assert!(!is_valid_time(0, 60)); // Minute too high
        assert!(!is_valid_time(25, 30)); // Hour way too high
    }

    #[test]
    fn test_encryption_roundtrip() {
        let data = "sensitive data";
        let key = generate_encryption_key();

        let encrypted = encrypt_data(data, &key).unwrap();
        let decrypted = decrypt_data(&encrypted, &key).unwrap();

        assert_eq!(data, decrypted);
        assert_ne!(data, encrypted); // Make sure it's actually encrypted
    }

    #[test]
    fn test_parking_data_serialization() {
        let mut data = ParkingData::default();

        let user_info = UserParkingInfo {
            phone_number: "12345678".to_string(),
            plate: "AB12345".to_string(),
        };

        let schedule = ParkingSchedule {
            user_id: 123456789,
            hour: 8,
            minute: 30,
            enabled: true,
            last_parked: Some(Utc::now()),
            missed_requests: vec![],
        };

        data.users.insert(123456789, user_info);
        data.schedules.insert(123456789, schedule);

        // Test serialization
        let serialized = serde_json::to_string(&data).unwrap();
        let deserialized: ParkingData = serde_json::from_str(&serialized).unwrap();

        assert_eq!(data.users.len(), deserialized.users.len());
        assert_eq!(data.schedules.len(), deserialized.schedules.len());
    }

    #[test]
    fn test_is_weekday() {
        // Test using specific dates
        let monday = Utc.with_ymd_and_hms(2024, 1, 1, 12, 0, 0).unwrap(); // Was a Monday
        let saturday = Utc.with_ymd_and_hms(2024, 1, 6, 12, 0, 0).unwrap(); // Was a Saturday
        let sunday = Utc.with_ymd_and_hms(2024, 1, 7, 12, 0, 0).unwrap(); // Was a Sunday

        assert!(is_weekday(&monday.with_timezone(&Copenhagen)));
        assert!(!is_weekday(&saturday.with_timezone(&Copenhagen)));
        assert!(!is_weekday(&sunday.with_timezone(&Copenhagen)));
    }

    #[test]
    fn test_schedule_time_matching() {
        let schedule_time = Copenhagen.with_ymd_and_hms(2024, 1, 1, 8, 30, 0).unwrap();

        // Exact match
        let current_time = Copenhagen.with_ymd_and_hms(2024, 1, 1, 8, 30, 0).unwrap();
        assert!(is_schedule_time_match(&schedule_time, &current_time));

        // Within 30 seconds (should match)
        let current_time = Copenhagen.with_ymd_and_hms(2024, 1, 1, 8, 30, 25).unwrap();
        assert!(is_schedule_time_match(&schedule_time, &current_time));

        // More than 1 minute off (should not match)
        let current_time = Copenhagen.with_ymd_and_hms(2024, 1, 1, 8, 31, 30).unwrap();
        assert!(!is_schedule_time_match(&schedule_time, &current_time));
    }

    #[test]
    fn test_rate_limiting_structure() {
        // Test that rate limiter can be created and configured
        let rate_limiter = create_rate_limiter();
        assert!(rate_limiter.check().is_ok()); // First request should be allowed
    }

    #[test]
    fn test_missed_request_cleanup() {
        let now = Utc::now();
        let old_request = now - Duration::days(2);
        let recent_request = now - Duration::hours(2);

        let mut missed_requests = vec![old_request, recent_request];

        cleanup_old_missed_requests(&mut missed_requests, now);

        // Should only keep requests from today
        assert_eq!(missed_requests.len(), 1);
        assert_eq!(missed_requests[0], recent_request);
    }

    #[test]
    fn test_uid_generation() {
        let uid1 = generate_unique_request_id();
        let uid2 = generate_unique_request_id();

        // UIDs should be unique
        assert_ne!(uid1, uid2);

        // UIDs should be valid UUID format
        assert!(uuid::Uuid::parse_str(&uid1).is_ok());
        assert!(uuid::Uuid::parse_str(&uid2).is_ok());
    }

    #[test]
    fn test_parking_payload_creation() {
        let plate = "AB12345";
        let phone = "12345678";

        let payload = create_parking_payload(plate, phone);

        // Verify the payload structure
        assert_eq!(payload["VehicleRegistration"], plate);
        assert_eq!(payload["PhoneNumber"], "4512345678"); // Should add country code
        assert_eq!(payload["Duration"], 600); // 10 hours in minutes
        assert_eq!(payload["VehicleRegistrationCountry"], "DK");

        // Should have parking area info
        assert!(payload["parkingAreas"].is_array());
        let areas = payload["parkingAreas"].as_array().unwrap();
        assert_eq!(areas.len(), 1);
        assert_eq!(areas[0]["ParkingAreaId"], 1956);
        assert_eq!(areas[0]["ParkingAreaKey"], "ADK-4688");
    }
}
