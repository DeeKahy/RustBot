//! Image rendering for the `-stats` command.
//!
//! Produces a single "infographic" PNG containing a horizontal bar chart of the
//! most active users (with their Discord avatars), an hourly-activity histogram,
//! and a message-share pie chart. All drawing is done on top of the `image`
//! crate via `imageproc`; text uses a bundled font (see `assets/fonts`) through
//! `rusttype`, so there is no runtime font lookup and nothing native to link.

use image::{Rgba, RgbaImage};
use imageproc::drawing;
use imageproc::rect::Rect;
use rusttype::{Font, Scale};

// Bundled fonts, compiled into the binary. Paths are relative to this file.
static FONT_BYTES: &[u8] = include_bytes!("../../assets/fonts/DejaVuSans.ttf");
static FONT_BOLD_BYTES: &[u8] = include_bytes!("../../assets/fonts/DejaVuSans-Bold.ttf");

lazy_static::lazy_static! {
    static ref FONT: Font<'static> = Font::try_from_bytes(FONT_BYTES).unwrap();
    static ref FONT_BOLD: Font<'static> = Font::try_from_bytes(FONT_BOLD_BYTES).unwrap();
}

// Layout constants.
const WIDTH: u32 = 1000;
const PAD: i32 = 40;
const ROW_H: i32 = 56;
/// Side length avatars are decoded/resized to before compositing.
pub const AVATAR_D: u32 = 40;

// Colours (Discord-ish dark theme).
const BG: Rgba<u8> = Rgba([30, 31, 34, 255]);
const PANEL: Rgba<u8> = Rgba([43, 45, 49, 255]);
const TEXT: Rgba<u8> = Rgba([237, 238, 240, 255]);
const MUTED: Rgba<u8> = Rgba([148, 155, 164, 255]);
const TRACK: Rgba<u8> = Rgba([56, 58, 64, 255]);

/// Slice/bar palette. Index-aligned with [`SLICE_EMOJI`] so the on-image colours
/// match the coloured-square emojis used in the embed legend.
pub const PALETTE: [[u8; 3]; 9] = [
    [237, 66, 69],   // red
    [230, 126, 34],  // orange
    [254, 231, 92],  // yellow
    [87, 242, 135],  // green
    [52, 152, 219],  // blue
    [155, 89, 182],  // purple
    [121, 85, 72],   // brown
    [79, 84, 92],    // dark grey
    [185, 187, 190], // light grey ("Others")
];
pub const SLICE_EMOJI: [&str; 9] = ["🟥", "🟧", "🟨", "🟩", "🟦", "🟪", "🟫", "⬛", "⬜"];

fn rgb(idx: usize) -> Rgba<u8> {
    let c = PALETTE[idx % PALETTE.len()];
    Rgba([c[0], c[1], c[2], 255])
}

/// One bar in the "most active users" chart.
pub struct BarEntry {
    pub label: String,
    pub value: u32,
    pub avatar: Option<RgbaImage>,
    pub color_idx: usize,
}

/// One wedge in the message-share pie chart.
pub struct Slice {
    pub label: String,
    pub count: u32,
    pub color_idx: usize,
}

/// Everything the renderer needs to draw the infographic.
pub struct Infographic<'a> {
    pub channel: &'a str,
    pub subtitle: String,
    pub bars: Vec<BarEntry>,
    pub slices: Vec<Slice>,
    pub hourly: [u32; 24],
    pub tz_label: &'a str,
}

// --- small drawing helpers -------------------------------------------------

fn draw_text(
    canvas: &mut RgbaImage,
    color: Rgba<u8>,
    x: i32,
    y: i32,
    size: f32,
    bold: bool,
    s: &str,
) {
    let font: &Font = if bold { &FONT_BOLD } else { &FONT };
    drawing::draw_text_mut(canvas, color, x, y, Scale::uniform(size), font, s);
}

fn text_width(size: f32, bold: bool, s: &str) -> i32 {
    let font: &Font = if bold { &FONT_BOLD } else { &FONT };
    drawing::text_size(Scale::uniform(size), font, s).0
}

/// Truncate `s` (adding an ellipsis) until it fits within `max_w` pixels.
fn truncate(size: f32, bold: bool, s: &str, max_w: i32) -> String {
    if text_width(size, bold, s) <= max_w {
        return s.to_string();
    }
    let mut chars: Vec<char> = s.chars().collect();
    while !chars.is_empty() {
        chars.pop();
        let candidate: String = chars.iter().collect::<String>() + "…";
        if text_width(size, bold, &candidate) <= max_w {
            return candidate;
        }
    }
    "…".to_string()
}

fn fill_rect(canvas: &mut RgbaImage, color: Rgba<u8>, x: i32, y: i32, w: i32, h: i32) {
    if w <= 0 || h <= 0 {
        return;
    }
    drawing::draw_filled_rect_mut(canvas, Rect::at(x, y).of_size(w as u32, h as u32), color);
}

/// Paste a pre-sized square avatar as a circle with a subtle ring.
fn paste_avatar_circle(canvas: &mut RgbaImage, avatar: &RgbaImage, cx: i32, cy: i32, d: u32) {
    let r = d as f32 / 2.0;
    let (cw, ch) = canvas.dimensions();
    for ay in 0..d {
        for ax in 0..d {
            let dx = ax as f32 + 0.5 - r;
            let dy = ay as f32 + 0.5 - r;
            let dist = (dx * dx + dy * dy).sqrt();
            if dist > r {
                continue;
            }
            let px = cx + ax as i32;
            let py = cy + ay as i32;
            if px < 0 || py < 0 || px as u32 >= cw || py as u32 >= ch {
                continue;
            }
            // Thin ring near the edge.
            let color = if dist > r - 2.0 {
                Rgba([88, 101, 242, 255]) // blurple ring
            } else {
                *avatar.get_pixel(ax, ay)
            };
            canvas.put_pixel(px as u32, py as u32, color);
        }
    }
}

// --- the pie chart (also reused standalone) --------------------------------

/// Render the slices to an anti-aliased pie chart `RgbaImage` of the given size.
pub fn render_pie_image(slices: &[Slice], size: u32) -> Option<RgbaImage> {
    let total: u32 = slices.iter().map(|s| s.count).sum();
    if total == 0 || slices.is_empty() {
        return None;
    }

    const SS: u32 = 3; // supersample factor for cheap anti-aliasing
    let hi = size * SS;
    let center = hi as f32 / 2.0;
    let radius = center - (3 * SS) as f32;
    let gap = SS as f32;
    let tau = std::f32::consts::TAU;

    let mut bounds: Vec<(f32, f32, [u8; 3])> = Vec::with_capacity(slices.len());
    let mut acc = 0.0f32;
    for s in slices {
        let start = acc;
        acc += s.count as f32 / total as f32 * tau;
        bounds.push((start, acc, PALETTE[s.color_idx % PALETTE.len()]));
    }

    let mut img = RgbaImage::new(hi, hi);
    for (x, y, px) in img.enumerate_pixels_mut() {
        let dx = x as f32 + 0.5 - center;
        let dy = y as f32 + 0.5 - center;
        let dist = (dx * dx + dy * dy).sqrt();
        if dist > radius {
            *px = Rgba([0, 0, 0, 0]);
            continue;
        }
        let mut ang = dx.atan2(-dy);
        if ang < 0.0 {
            ang += tau;
        }
        let color = bounds
            .iter()
            .find(|(start, end, _)| ang >= *start && ang < *end)
            .map(|(_, _, c)| *c)
            .unwrap_or(bounds.last().unwrap().2);

        if bounds.len() > 1 {
            let on_edge = bounds.iter().any(|(start, _, _)| {
                let mut da = (ang - start).abs();
                if da > tau / 2.0 {
                    da = tau - da;
                }
                da * dist < gap
            });
            if on_edge {
                *px = Rgba([43, 45, 49, 255]);
                continue;
            }
        }
        *px = Rgba([color[0], color[1], color[2], 255]);
    }

    Some(image::imageops::resize(
        &img,
        size,
        size,
        image::imageops::FilterType::Triangle,
    ))
}

// --- the full infographic --------------------------------------------------

/// Render the whole infographic and encode it as PNG bytes.
pub fn render(info: &Infographic) -> Option<Vec<u8>> {
    if info.bars.is_empty() {
        return None;
    }

    // --- work out the vertical layout up front so we can size the canvas ---
    let header_h = 78;
    let sec_gap = 26;
    let sec_title_h = 30;
    let bars_h = info.bars.len() as i32 * ROW_H;
    let hist_h = 170 + 26; // chart + hour labels
    let pie_size = 240u32;
    let legend_h = info.slices.len() as i32 * 26;
    let pie_block_h = (pie_size as i32).max(legend_h);

    let total_h = PAD
        + header_h
        + sec_gap
        + sec_title_h
        + bars_h
        + sec_gap
        + sec_title_h
        + hist_h
        + sec_gap
        + sec_title_h
        + pie_block_h
        + PAD;

    let mut c = RgbaImage::from_pixel(WIDTH, total_h as u32, BG);

    // --- header ---
    let mut y = PAD;
    draw_text(
        &mut c,
        TEXT,
        PAD,
        y,
        40.0,
        true,
        &format!("#{}", info.channel),
    );
    draw_text(&mut c, MUTED, PAD, y + 48, 22.0, false, &info.subtitle);
    y += header_h;

    // --- bar chart: most active users ---
    y += sec_gap;
    draw_text(&mut c, TEXT, PAD, y, 26.0, true, "Most active users");
    y += sec_title_h;

    let name_col_x = PAD + AVATAR_D as i32 + 14;
    let name_col_w = 180;
    let bar_x = name_col_x + name_col_w + 14;
    let count_col_w = 90;
    let bar_max_w = WIDTH as i32 - PAD - count_col_w - bar_x;
    let max_val = info.bars.iter().map(|b| b.value).max().unwrap_or(1).max(1);
    let bar_h = 30;

    for (i, b) in info.bars.iter().enumerate() {
        let row_y = y + i as i32 * ROW_H;
        let mid = row_y + ROW_H / 2;

        // avatar (or a coloured placeholder circle)
        let av_y = mid - AVATAR_D as i32 / 2;
        match &b.avatar {
            Some(av) => paste_avatar_circle(&mut c, av, PAD, av_y, AVATAR_D),
            None => {
                let placeholder = RgbaImage::from_pixel(AVATAR_D, AVATAR_D, rgb(b.color_idx));
                paste_avatar_circle(&mut c, &placeholder, PAD, av_y, AVATAR_D);
            }
        }

        // name
        let name = truncate(22.0, false, &b.label, name_col_w);
        draw_text(&mut c, TEXT, name_col_x, mid - 12, 22.0, false, &name);

        // bar track + fill
        let by = mid - bar_h / 2;
        fill_rect(&mut c, TRACK, bar_x, by, bar_max_w, bar_h);
        let w = ((b.value as f32 / max_val as f32) * bar_max_w as f32).round() as i32;
        fill_rect(&mut c, rgb(b.color_idx), bar_x, by, w.max(3), bar_h);

        // count
        draw_text(
            &mut c,
            TEXT,
            bar_x + w.max(3) + 8,
            mid - 11,
            20.0,
            true,
            &b.value.to_string(),
        );
    }
    y += bars_h;

    // --- hourly activity histogram ---
    y += sec_gap;
    draw_text(
        &mut c,
        TEXT,
        PAD,
        y,
        26.0,
        true,
        &format!("Activity by hour ({})", info.tz_label),
    );
    y += sec_title_h;

    let hist_area_w = WIDTH as i32 - 2 * PAD;
    let hist_chart_h = 150;
    let baseline = y + hist_chart_h;
    let gap = 5;
    let col_w = (hist_area_w - 23 * gap) / 24;
    let hmax = info.hourly.iter().copied().max().unwrap_or(1).max(1);
    let peak_hour = info
        .hourly
        .iter()
        .enumerate()
        .max_by_key(|(_, v)| **v)
        .map(|(h, _)| h)
        .unwrap_or(0);

    // baseline rule
    fill_rect(&mut c, TRACK, PAD, baseline, hist_area_w, 2);
    for (h, &count) in info.hourly.iter().enumerate() {
        let bx = PAD + h as i32 * (col_w + gap);
        let bh = ((count as f32 / hmax as f32) * hist_chart_h as f32).round() as i32;
        let color = if h == peak_hour {
            Rgba([87, 242, 135, 255]) // highlight the peak
        } else {
            Rgba([52, 152, 219, 255])
        };
        fill_rect(&mut c, color, bx, baseline - bh, col_w, bh.max(2));
        if h.is_multiple_of(3) {
            draw_text(&mut c, MUTED, bx, baseline + 6, 16.0, false, &h.to_string());
        }
    }
    y += hist_h;

    // --- message-share pie + legend ---
    y += sec_gap;
    draw_text(&mut c, TEXT, PAD, y, 26.0, true, "Message share");
    y += sec_title_h;

    if let Some(pie) = render_pie_image(&info.slices, pie_size) {
        image::imageops::overlay(&mut c, &pie, PAD as i64, y as i64);
    }

    let legend_x = PAD + pie_size as i32 + 40;
    let total_msgs: u32 = info.slices.iter().map(|s| s.count).sum::<u32>().max(1);
    for (i, s) in info.slices.iter().enumerate() {
        let ly = y + i as i32 * 26;
        // swatch (rounded-ish square panel bg behind for contrast is skipped)
        fill_rect(&mut c, PANEL, legend_x - 2, ly - 2, 24, 24);
        fill_rect(&mut c, rgb(s.color_idx), legend_x, ly, 16, 16);
        let pct = s.count as f32 / total_msgs as f32 * 100.0;
        let label = truncate(
            20.0,
            false,
            &format!("{}  —  {} ({:.0}%)", s.label, s.count, pct),
            WIDTH as i32 - legend_x - 30 - 30,
        );
        draw_text(&mut c, TEXT, legend_x + 30, ly - 2, 20.0, false, &label);
    }

    // encode
    let mut bytes = Vec::new();
    c.write_to(
        &mut std::io::Cursor::new(&mut bytes),
        image::ImageOutputFormat::Png,
    )
    .ok()?;
    Some(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn slice(label: &str, count: u32, idx: usize) -> Slice {
        Slice {
            label: label.to_string(),
            count,
            color_idx: idx,
        }
    }

    #[test]
    fn test_render_pie_image_empty() {
        assert!(render_pie_image(&[], 240).is_none());
        assert!(render_pie_image(&[slice("a", 0, 0)], 240).is_none());
    }

    #[test]
    fn test_render_pie_image_dimensions() {
        let pie = render_pie_image(&[slice("a", 3, 0), slice("b", 1, 1)], 240).unwrap();
        assert_eq!(pie.dimensions(), (240, 240));
    }

    #[test]
    fn test_truncate_fits() {
        let s = truncate(20.0, false, "hello", 1000);
        assert_eq!(s, "hello");
        let t = truncate(20.0, false, "a very long username indeed", 40);
        assert!(t.ends_with('…'));
        assert!(t.chars().count() < "a very long username indeed".chars().count());
    }

    #[test]
    fn test_render_infographic_png() {
        let info = Infographic {
            channel: "general",
            subtitle: "100 messages · 500 words".to_string(),
            bars: vec![
                BarEntry {
                    label: "alice".into(),
                    value: 50,
                    avatar: None,
                    color_idx: 0,
                },
                BarEntry {
                    label: "bob".into(),
                    value: 30,
                    avatar: None,
                    color_idx: 1,
                },
                BarEntry {
                    label: "carol".into(),
                    value: 20,
                    avatar: None,
                    color_idx: 2,
                },
            ],
            slices: vec![
                slice("alice", 50, 0),
                slice("bob", 30, 1),
                slice("carol", 20, 2),
            ],
            hourly: {
                let mut h = [0u32; 24];
                h[9] = 10;
                h[14] = 25;
                h[22] = 40;
                h
            },
            tz_label: "UTC",
        };
        let png = render(&info).expect("should render");
        assert_eq!(&png[..8], &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]);
    }

    #[test]
    fn test_render_empty_bars_none() {
        let info = Infographic {
            channel: "x",
            subtitle: String::new(),
            bars: vec![],
            slices: vec![],
            hourly: [0; 24],
            tz_label: "UTC",
        };
        assert!(render(&info).is_none());
    }
}
