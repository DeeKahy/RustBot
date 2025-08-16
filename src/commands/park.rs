use crate::{Context, Error};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::fs;

#[derive(Serialize, Deserialize, Clone)]
struct UserParkingInfo {
    phone_number: String,
    plate: String,
}

#[derive(Serialize, Deserialize, Default)]
struct ParkingData {
    users: HashMap<u64, UserParkingInfo>,
}

const PARKING_DATA_FILE: &str = "/tmp/rustbot_parking_data.json";

fn load_parking_data() -> ParkingData {
    match fs::read_to_string(PARKING_DATA_FILE) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => ParkingData::default(),
    }
}

fn save_parking_data(data: &ParkingData) -> Result<(), Error> {
    let json = serde_json::to_string_pretty(data)?;
    fs::write(PARKING_DATA_FILE, json)?;
    Ok(())
}

/// Park your vehicle using mobile parking service
#[poise::command(
    prefix_command,
    slash_command,
    subcommands("park_now", "park_info", "park_clear")
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
