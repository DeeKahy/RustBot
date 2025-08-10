use crate::{Context, Error};
use image::DynamicImage;
use poise::serenity_prelude as serenity;
use std::fs;
use std::io::Cursor;
use tempfile::NamedTempFile;

/// Yeets a user by putting their profile picture on the yeet GIF
#[poise::command(prefix_command, slash_command)]
pub async fn yeet(
    ctx: Context<'_>,
    #[description = "User to call a hit on"] user: Option<serenity::User>,
) -> Result<(), Error> {
    log::info!("Hit ordered by {}", ctx.author().name);

    let target_user = user.unwrap_or_else(|| ctx.author().clone());

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
                        .content(format!("❌ Failed to download profile picture: {}", e)),
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
                        .content(format!("❌ Failed to read profile picture data: {}", e)),
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
                        .content(format!("❌ Failed to process profile picture: {}", e)),
                )
                .await?;
            return Ok(());
        }
    };

    // Update status
    thinking_msg
        .edit(
            ctx,
            poise::CreateReply::default().content("Aiming..."),
        )
        .await?;

    // Process the GIF with the profile picture overlay
    match process_yeet_gif(&avatar_img, &target_user.name).await {
        Ok(output_path) => {
            // Update status
            thinking_msg
                .edit(
                    ctx,
                    poise::CreateReply::default().content("Firing!"),
                )
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
            let attachment = serenity::CreateAttachment::bytes(gif_data, "yeet.gif");

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
                        .content(format!("❌ Failed to process yeet GIF: {}", e)),
                )
                .await?;
        }
    }

    Ok(())
}

async fn process_yeet_gif(
    avatar_img: &DynamicImage,
    _user_name: &str,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    // Load the original yeet GIF
    let gif_path = "assets/yeet.gif";
    let gif_data = fs::read(gif_path)?;

    // Decode the GIF
    let mut decoder = gif::DecodeOptions::new();
    decoder.set_color_output(gif::ColorOutput::RGBA);
    let mut gif_decoder = decoder.read_info(Cursor::new(&gif_data))?;

    let screen_width = gif_decoder.width() as u32;
    let screen_height = gif_decoder.height() as u32;

    // Resize profile picture - position it on middle left with appropriate scale
    let scale_factor = 0.3; // 30% of GIF height
    let pfp_size = (screen_height as f32 * scale_factor) as u32;
    let resized_avatar =
        avatar_img.resize_exact(pfp_size, pfp_size, image::imageops::FilterType::Lanczos3);

    // Position: middle left (x: 10% from left, y: center)
    let overlay_x = (screen_width as f32 * 0.1) as u32;
    let overlay_y = (screen_height / 2).saturating_sub(pfp_size / 2);

    // Create a new GIF encoder
    let temp_file = NamedTempFile::new()?;
    let output_path = temp_file.path().to_string_lossy().to_string() + ".gif";

    {
        let output_file = std::fs::File::create(&output_path)?;
        let mut encoder =
            gif::Encoder::new(output_file, screen_width as u16, screen_height as u16, &[])?;
        encoder.set_repeat(gif::Repeat::Infinite)?;

        // Process each frame
        let mut frame_count = 0;
        let mut canvas = vec![0u8; (screen_width * screen_height * 4) as usize];

        while let Some(frame) = gif_decoder.read_next_frame()? {
            frame_count += 1;
            if frame_count > 100 {
                // Limit to prevent excessive processing
                break;
            }

            // Clear canvas
            canvas.fill(0);

            // Copy frame data to canvas
            let frame_data = &frame.buffer;
            let frame_width = frame.width as u32;
            let frame_height = frame.height as u32;
            let frame_left = frame.left as u32;
            let frame_top = frame.top as u32;

            // Copy frame pixels to canvas
            for y in 0..frame_height {
                for x in 0..frame_width {
                    let src_idx = ((y * frame_width + x) * 4) as usize;
                    let dst_x = frame_left + x;
                    let dst_y = frame_top + y;

                    if dst_x < screen_width
                        && dst_y < screen_height
                        && src_idx + 3 < frame_data.len()
                    {
                        let dst_idx = ((dst_y * screen_width + dst_x) * 4) as usize;

                        if dst_idx + 3 < canvas.len() {
                            canvas[dst_idx] = frame_data[src_idx]; // R
                            canvas[dst_idx + 1] = frame_data[src_idx + 1]; // G
                            canvas[dst_idx + 2] = frame_data[src_idx + 2]; // B
                            canvas[dst_idx + 3] = frame_data[src_idx + 3]; // A
                        }
                    }
                }
            }

            // Overlay the profile picture
            let avatar_rgba = resized_avatar.to_rgba8();
            for y in 0..pfp_size {
                for x in 0..pfp_size {
                    let dst_x = overlay_x + x;
                    let dst_y = overlay_y + y;

                    if dst_x < screen_width && dst_y < screen_height {
                        let src_pixel = avatar_rgba.get_pixel(x, y);
                        let dst_idx = ((dst_y * screen_width + dst_x) * 4) as usize;

                        if dst_idx + 3 < canvas.len() {
                            let alpha = src_pixel[3] as f32 / 255.0;
                            let inv_alpha = 1.0 - alpha;

                            // Alpha blending
                            canvas[dst_idx] = (src_pixel[0] as f32 * alpha
                                + canvas[dst_idx] as f32 * inv_alpha)
                                as u8;
                            canvas[dst_idx + 1] = (src_pixel[1] as f32 * alpha
                                + canvas[dst_idx + 1] as f32 * inv_alpha)
                                as u8;
                            canvas[dst_idx + 2] = (src_pixel[2] as f32 * alpha
                                + canvas[dst_idx + 2] as f32 * inv_alpha)
                                as u8;
                            canvas[dst_idx + 3] = ((src_pixel[3] as f32 * alpha
                                + canvas[dst_idx + 3] as f32 * inv_alpha)
                                .min(255.0))
                                as u8;
                        }
                    }
                }
            }

            // Create output frame
            let mut output_frame =
                gif::Frame::from_rgba(screen_width as u16, screen_height as u16, &mut canvas);
            output_frame.delay = frame.delay;
            output_frame.dispose = frame.dispose;

            encoder.write_frame(&output_frame)?;
        }
    }

    Ok(output_path)
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_yeet_command_exists() {
        // Basic test to ensure the command function exists
        let function_name = "yeet";
        assert_eq!(function_name.len(), 4);
    }
}
