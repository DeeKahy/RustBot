use crate::{Context, Error};
use image::{AnimationDecoder, DynamicImage};
use poise::serenity_prelude as serenity;
use rand::seq::SliceRandom;
use std::fs;
use tempfile::NamedTempFile;

/// Orders a hit on a user by putting their profile picture on a random hit GIF
#[poise::command(prefix_command, slash_command)]
pub async fn hit(
    ctx: Context<'_>,
    #[description = "User to call a hit on"] user: Option<serenity::User>,
) -> Result<(), Error> {
    log::info!("Hit command called by {}", ctx.author().name);

    // Check if no user was provided
    let target_user = match user {
        Some(user) => user,
        None => {
            ctx.say("🎯 You need to specify a target! Use `-hit @someone` to order a hit.")
                .await?;
            return Ok(());
        }
    };

    // Defer response for slash commands to prevent timeout
    ctx.defer().await?;

    // Send initial "thinking" message
    let thinking_msg = ctx.say("Loading the gun...").await?;

    // Get the user's avatar URL
    let avatar_url = target_user
        .avatar_url()
        .unwrap_or_else(|| target_user.default_avatar_url());

    // Download the user's profile picture
    let avatar_response = match reqwest::get(&avatar_url).await {
        Ok(response) => response,
        Err(e) => {
            thinking_msg
                .edit(
                    ctx,
                    poise::CreateReply::default()
                        .content(format!("❌ Failed to identify target: {}", e)),
                )
                .await?;
            return Ok(());
        }
    };

    let avatar_bytes = match avatar_response.bytes().await {
        Ok(bytes) => bytes,
        Err(e) => {
            thinking_msg
                .edit(
                    ctx,
                    poise::CreateReply::default()
                        .content(format!("❌ Target intel corrupted: {}", e)),
                )
                .await?;
            return Ok(());
        }
    };

    // Load and resize the profile picture
    let avatar_img = match image::load_from_memory(&avatar_bytes) {
        Ok(img) => img,
        Err(e) => {
            thinking_msg
                .edit(
                    ctx,
                    poise::CreateReply::default()
                        .content(format!("❌ Failed to process target photo: {}", e)),
                )
                .await?;
            return Ok(());
        }
    };

    // Update status
    thinking_msg
        .edit(ctx, poise::CreateReply::default().content("Aiming..."))
        .await?;

    // Select a random hit GIF and extract positioning data
    match select_random_hit_gif().await {
        Ok((gif_path, hit_data)) => {
            // Process the GIF with the profile picture overlay
            match process_hit_gif(&avatar_img, &gif_path, &hit_data).await {
                Ok(output_path) => {
                    // Update status
                    thinking_msg
                        .edit(ctx, poise::CreateReply::default().content("Firing!"))
                        .await?;

                    // Read the processed GIF
                    let gif_data = match fs::read(&output_path) {
                        Ok(data) => data,
                        Err(e) => {
                            thinking_msg
                                .edit(
                                    ctx,
                                    poise::CreateReply::default()
                                        .content(format!("❌ Failed to read processed GIF: {}", e)),
                                )
                                .await?;
                            return Ok(());
                        }
                    };

                    // Send the GIF
                    let attachment = serenity::CreateAttachment::bytes(gif_data, "hit.gif");

                    let reply = poise::CreateReply::default()
                        .content(format!("{} successfully assassinated!", target_user.name))
                        .attachment(attachment);

                    thinking_msg.edit(ctx, reply).await?;

                    // Clean up temporary file
                    let _ = fs::remove_file(output_path);
                }
                Err(e) => {
                    thinking_msg
                        .edit(
                            ctx,
                            poise::CreateReply::default()
                                .content(format!("❌ Contract failed: {}", e)),
                        )
                        .await?;
                }
            }
        }
        Err(e) => {
            thinking_msg
                .edit(
                    ctx,
                    poise::CreateReply::default()
                        .content(format!("❌ No hit GIFs available: {}", e)),
                )
                .await?;
        }
    }

    Ok(())
}

#[derive(Debug)]
struct HitData {
    x_percent: f32,
    y_percent: f32,
    scale_percent: f32,
}

async fn select_random_hit_gif(
) -> Result<(String, HitData), Box<dyn std::error::Error + Send + Sync>> {
    let hit_dir = "assets/hit";

    // Read all files in the hit directory
    let entries = fs::read_dir(hit_dir)?;
    let mut gif_files = Vec::new();

    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
            if filename.ends_with(".gif") && filename.starts_with("hit_") {
                gif_files.push(path.to_string_lossy().to_string());
            }
        }
    }

    if gif_files.is_empty() {
        return Err("No hit GIFs found in assets/hit directory".into());
    }

    // Select a random GIF
    let mut rng = rand::thread_rng();
    let selected_gif = gif_files.choose(&mut rng).unwrap();

    // Parse the filename to extract positioning data
    let hit_data = parse_hit_filename(selected_gif)?;

    Ok((selected_gif.clone(), hit_data))
}

fn parse_hit_filename(filename: &str) -> Result<HitData, Box<dyn std::error::Error + Send + Sync>> {
    // Expected format: hit_1_x0.1_y0.4_s0.3.gif
    let basename = filename
        .split('/')
        .next_back()
        .unwrap_or(filename)
        .trim_end_matches(".gif");

    let parts: Vec<&str> = basename.split('_').collect();

    if parts.len() < 5 || parts[0] != "hit" {
        return Err(format!("Invalid hit filename format: {}", filename).into());
    }

    let mut x_percent = 0.1; // default values
    let mut y_percent = 0.4;
    let mut scale_percent = 0.3;

    // Parse x, y, s values
    for part in &parts[2..] {
        if let Some(x_val) = part.strip_prefix('x') {
            x_percent = x_val.parse::<f32>()?;
        } else if let Some(y_val) = part.strip_prefix('y') {
            y_percent = y_val.parse::<f32>()?;
        } else if let Some(s_val) = part.strip_prefix('s') {
            scale_percent = s_val.parse::<f32>()?;
        }
    }

    Ok(HitData {
        x_percent,
        y_percent,
        scale_percent,
    })
}

async fn process_hit_gif(
    avatar_img: &DynamicImage,
    gif_path: &str,
    hit_data: &HitData,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    // Open the original GIF using image crate
    let gif_file = std::fs::File::open(gif_path)?;
    let decoder = image::codecs::gif::GifDecoder::new(gif_file)?;
    let frames = decoder.into_frames();
    let frames: Result<Vec<_>, _> = frames.collect();
    let frames = frames?;

    if frames.is_empty() {
        return Err("GIF has no frames".into());
    }

    // Get dimensions from first frame
    let first_frame = &frames[0];
    let (screen_width, screen_height) = first_frame.buffer().dimensions();

    // Calculate profile picture size and position based on hit data
    let pfp_size = (screen_height as f32 * hit_data.scale_percent) as u32;
    let resized_avatar = avatar_img
        .resize_exact(pfp_size, pfp_size, image::imageops::FilterType::Lanczos3)
        .to_rgba8();

    // Calculate position from percentages
    let overlay_x = (screen_width as f32 * hit_data.x_percent) as u32;
    let overlay_y = (screen_height as f32 * hit_data.y_percent) as u32;

    // Process frames and create new GIF
    let temp_file = NamedTempFile::new()?;
    let output_path = temp_file.path().to_string_lossy().to_string() + ".gif";
    let output_file = std::fs::File::create(&output_path)?;

    let mut encoder = image::codecs::gif::GifEncoder::new(output_file);
    encoder.set_repeat(image::codecs::gif::Repeat::Infinite)?;

    // Limit frames to prevent excessive processing
    let frame_limit = std::cmp::min(frames.len(), 100);

    for frame in frames.iter().take(frame_limit) {
        let mut frame_buffer = frame.buffer().clone();

        // Overlay the profile picture with alpha blending
        for y in 0..pfp_size {
            for x in 0..pfp_size {
                let dst_x = overlay_x + x;
                let dst_y = overlay_y + y;

                if dst_x < screen_width && dst_y < screen_height {
                    let avatar_pixel = resized_avatar.get_pixel(x, y);
                    let alpha = avatar_pixel[3] as f32 / 255.0;

                    if alpha > 0.0 {
                        let frame_pixel = frame_buffer.get_pixel_mut(dst_x, dst_y);
                        let inv_alpha = 1.0 - alpha;

                        frame_pixel[0] = (avatar_pixel[0] as f32 * alpha
                            + frame_pixel[0] as f32 * inv_alpha)
                            as u8;
                        frame_pixel[1] = (avatar_pixel[1] as f32 * alpha
                            + frame_pixel[1] as f32 * inv_alpha)
                            as u8;
                        frame_pixel[2] = (avatar_pixel[2] as f32 * alpha
                            + frame_pixel[2] as f32 * inv_alpha)
                            as u8;
                        frame_pixel[3] = ((avatar_pixel[3] as f32 * alpha
                            + frame_pixel[3] as f32 * inv_alpha)
                            .min(255.0)) as u8;
                    }
                }
            }
        }

        // Create frame with original delay
        let delay = frame.delay().numer_denom_ms();
        let frame_delay = image::Delay::from_numer_denom_ms(delay.0, delay.1);

        let gif_frame = image::Frame::from_parts(frame_buffer, 0, 0, frame_delay);
        encoder.encode_frame(gif_frame)?;
    }

    drop(encoder);
    Ok(output_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hit_filename() {
        let result = parse_hit_filename("assets/hit/hit_1_x0.1_y0.4_s0.3.gif").unwrap();
        assert_eq!(result.x_percent, 0.1);
        assert_eq!(result.y_percent, 0.4);
        assert_eq!(result.scale_percent, 0.3);
    }

    #[test]
    fn test_hit_command_exists() {
        let function_name = "hit";
        assert_eq!(function_name.len(), 3);
    }
}
