use ab_glyph::{Font, FontRef, PxScale, ScaleFont, point};
use ksni::Icon;
use std::env;
use std::fs;
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrayState {
    Unknown,
    Connected,
    Off,
    NoDongle,
    Permission,
    Error,
}

const SIZES: [i32; 7] = [16, 22, 24, 32, 48, 64, 128];

// Subset of Noto Sans Medium containing only the ten digits.  Keeping the
// font embedded avoids fontconfig, Pango and Qt, and lets us render a smooth,
// not-too-heavy face without a runtime font dependency.
const FONT_BYTES: &[u8] = include_bytes!("../assets/NotoSans-Medium-digits.ttf");

// A crisp one-pixel 5x7 face for the 16 px tray icon.  Antialiased outlines
// turn into grey mush at that size; these keep the strokes thin and white.
const DIGITS5: [[u8; 7]; 10] = [
    [0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110],
    [0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110],
    [0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111],
    [0b11110, 0b00001, 0b00001, 0b01110, 0b00001, 0b00001, 0b11110],
    [0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010],
    [0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110],
    [0b00110, 0b01000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110],
    [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000],
    [0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110],
    [0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010, 0b01100],
];

struct TextMask {
    size: u32,
    alpha: Vec<f32>,
}

impl TextMask {
    fn new(size: u32, label: &str) -> Option<Self> {
        if label.is_empty() || !label.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }

        if size <= 18 {
            let glyphs: Vec<[u8; 7]> = label
                .bytes()
                .map(|b| DIGITS5[(b - b'0') as usize])
                .collect();
            let units = glyphs.len() * 5 + glyphs.len().saturating_sub(1);
            if units > size as usize {
                return None;
            }
            let left = (size as usize - units) / 2;
            let top = (size as usize - 7) / 2;
            let mut alpha = vec![0.0f32; (size * size) as usize];
            for (index, glyph) in glyphs.iter().enumerate() {
                for (gy, row) in glyph.iter().enumerate() {
                    for gx in 0..5 {
                        if (row & (1 << (4 - gx))) != 0 {
                            let px = left + index * 6 + gx;
                            let py = top + gy;
                            if px < size as usize && py < size as usize {
                                alpha[py * size as usize + px] = 1.0;
                            }
                        }
                    }
                }
            }
            return Some(Self { size, alpha });
        }

        let font = FontRef::try_from_slice(FONT_BYTES).ok()?;
        let digit_count = label.len();
        let compact = size <= 24;
        let font_size = size as f32
            * if compact {
                match digit_count {
                    1 => 0.80,
                    2 => 0.70,
                    _ => 0.52,
                }
            } else {
                match digit_count {
                    1 => 0.52,
                    2 => 0.46,
                    _ => 0.36,
                }
            };
        let scale = PxScale::from(font_size);
        let scaled = font.as_scaled(scale);
        let glyphs: Vec<_> = label.chars().map(|c| font.glyph_id(c)).collect();
        let width: f32 = glyphs.iter().map(|id| scaled.h_advance(*id)).sum();
        let baseline = size as f32 / 2.0 + (scaled.ascent() + scaled.descent()) / 2.0;
        let mut alpha = vec![0.0f32; (size * size) as usize];
        let mut x = (size as f32 - width) / 2.0;

        for id in glyphs {
            let glyph = id.with_scale_and_position(scale, point(x, baseline));
            if let Some(outline) = font.outline_glyph(glyph) {
                let bounds = outline.px_bounds();
                outline.draw(|gx, gy, coverage| {
                    let px = bounds.min.x as i32 + gx as i32;
                    let py = bounds.min.y as i32 + gy as i32;
                    if px >= 0 && py >= 0 && px < size as i32 && py < size as i32 {
                        let index = py as usize * size as usize + px as usize;
                        alpha[index] = alpha[index].max(coverage.clamp(0.0, 1.0));
                    }
                });
            }
            x += scaled.h_advance(id);
        }

        Some(Self { size, alpha })
    }

    fn at(&self, x: f64, y: f64) -> f32 {
        let limit = self.size as f64 - 1.0;
        let x = x.clamp(0.0, limit);
        let y = y.clamp(0.0, limit);
        let x0 = x.floor() as usize;
        let y0 = y.floor() as usize;
        let x1 = (x0 + 1).min(self.size as usize - 1);
        let y1 = (y0 + 1).min(self.size as usize - 1);
        let fx = (x - x0 as f64) as f32;
        let fy = (y - y0 as f64) as f32;
        let a = self.alpha[y0 * self.size as usize + x0];
        let b = self.alpha[y0 * self.size as usize + x1];
        let c = self.alpha[y1 * self.size as usize + x0];
        let d = self.alpha[y1 * self.size as usize + x1];
        let top = a * (1.0 - fx) + b * fx;
        let bottom = c * (1.0 - fx) + d * fx;
        top * (1.0 - fy) + bottom * fy
    }
}

fn accent(state: TrayState, battery: Option<u8>) -> [u8; 3] {
    if state != TrayState::Connected {
        return match state {
            TrayState::Permission | TrayState::Error => [255, 180, 84],
            TrayState::Off => [154, 162, 173],
            _ => [119, 128, 139],
        };
    }
    match battery {
        Some(0..=10) => [255, 92, 92],
        Some(11..=20) => [255, 180, 84],
        Some(21..=40) => [245, 215, 110],
        Some(_) => [94, 224, 138],
        None => [119, 128, 139],
    }
}

fn config_home() -> Option<PathBuf> {
    if let Ok(value) = env::var("XDG_CONFIG_HOME") {
        if !value.is_empty() {
            return Some(PathBuf::from(value));
        }
    }
    env::var("HOME").ok().map(|home| PathBuf::from(home).join(".config"))
}

fn kde_window_background() -> Option<[u8; 3]> {
    let path = config_home()?.join("kdeglobals");
    let text = fs::read_to_string(path).ok()?;
    let mut in_window = false;
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_window = line.eq_ignore_ascii_case("[Colors:Window]");
            continue;
        }
        if !in_window {
            continue;
        }
        if let Some(value) = line.strip_prefix("BackgroundNormal=") {
            let channels: Vec<u8> = value
                .split(',')
                .filter_map(|part| part.trim().parse::<u8>().ok())
                .collect();
            if channels.len() >= 3 {
                return Some([channels[0], channels[1], channels[2]]);
            }
        }
    }
    None
}

/// Pick black-ish or white-ish foreground text for the current panel theme.
///
/// The tray's pixmap is not recoloured by Plasma, so we read KDE's window
/// background colour and choose a contrasting text colour.  An environment
/// override is available for tests and unusual themes.
pub fn theme_text_color() -> [u8; 3] {
    if let Ok(value) = env::var("GSP370_TEXT_COLOR") {
        let channels: Vec<u8> = value
            .split(',')
            .filter_map(|part| part.trim().parse::<u8>().ok())
            .collect();
        if channels.len() >= 3 {
            return [channels[0], channels[1], channels[2]];
        }
    }

    if let Some([r, g, b]) = kde_window_background() {
        let luminance = 0.2126 * r as f32 + 0.7152 * g as f32 + 0.0722 * b as f32;
        return if luminance >= 140.0 {
            [28, 30, 34]
        } else {
            [245, 247, 250]
        };
    }

    if let Ok(theme) = env::var("GTK_THEME") {
        if theme.to_ascii_lowercase().contains("dark") {
            return [245, 247, 250];
        }
    }
    [245, 247, 250]
}

fn sample(
    x: f64,
    y: f64,
    size: f64,
    state: TrayState,
    battery: Option<u8>,
    text: Option<&TextMask>,
    text_color: [u8; 3],
) -> [f32; 4] {
    let center = size / 2.0;
    let dx = x - center;
    let dy = y - center;
    let radius = (dx * dx + dy * dy).sqrt();
    let compact = size <= 24.0;
    let has_text = text.is_some();

    // Compact connected icons are text-only.  Disconnected/unknown compact
    // icons, and all large icons, keep a thin ring so the state is visible.
    if !compact || !has_text {
        let ring_center = if compact { size * 0.40 } else { size * 0.385 };
        let ring_half = if compact { size * 0.065 } else { size * 0.0425 };
        if (radius - ring_center).abs() <= ring_half {
            let color = if state == TrayState::Connected {
                if let Some(level) = battery {
                    let mut angle = dy.atan2(dx);
                    angle = (angle + std::f64::consts::FRAC_PI_2)
                        .rem_euclid(std::f64::consts::TAU);
                    if angle / std::f64::consts::TAU <= level.min(100) as f64 / 100.0 {
                        accent(state, battery)
                    } else {
                        [72, 78, 87]
                    }
                } else {
                    [72, 78, 87]
                }
            } else {
                accent(state, battery)
            };
            return [color[0] as f32, color[1] as f32, color[2] as f32, 1.0];
        }
    }

    if let Some(mask) = text {
        let coverage = mask.at(x, y);
        if coverage > 0.0 {
            // A light gamma lift keeps medium-weight strokes from washing out
            // when the icon is composited over the panel, without adding a
            // background plate.
            let coverage = if size > 18.0 && coverage >= 0.20 {
                1.0
            } else {
                coverage.powf(0.65)
            };
            return [
                text_color[0] as f32,
                text_color[1] as f32,
                text_color[2] as f32,
                coverage.min(1.0),
            ];
        }
    }

    [0.0, 0.0, 0.0, 0.0]
}

fn render(size: i32, state: TrayState, battery: Option<u8>, text_color: [u8; 3]) -> Vec<u8> {
    let label = if state == TrayState::Connected {
        battery.map(|b| b.min(100).to_string()).unwrap_or_default()
    } else {
        String::new()
    };
    let text = TextMask::new(size as u32, &label);
    let mut data = Vec::with_capacity((size * size * 4) as usize);

    // The 16 px bitmap face is deliberately pixel-aligned.  Supersampling a
    // one-pixel stroke would average it into grey, so copy those pixels
    // directly instead.
    if size <= 18 {
        if let Some(mask) = text.as_ref() {
            for py in 0..size {
                for px in 0..size {
                    let coverage = mask.at(px as f64, py as f64);
                    if coverage > 0.5 {
                        data.extend_from_slice(&[
                            255,
                            text_color[0],
                            text_color[1],
                            text_color[2],
                        ]);
                    } else {
                        data.extend_from_slice(&[0, 0, 0, 0]);
                    }
                }
            }
            return data;
        }
    }

    for py in 0..size {
        for px in 0..size {
            let mut alpha_sum = 0.0f32;
            let mut rgb_sum = [0.0f32; 3];
            for oy in 0..3 {
                for ox in 0..3 {
                    let x = px as f64 + (ox as f64 + 0.5) / 3.0;
                    let y = py as f64 + (oy as f64 + 0.5) / 3.0;
                    let sample = sample(
                        x,
                        y,
                        size as f64,
                        state,
                        battery,
                        text.as_ref(),
                        text_color,
                    );
                    let alpha = sample[3];
                    alpha_sum += alpha;
                    rgb_sum[0] += sample[0] * alpha;
                    rgb_sum[1] += sample[1] * alpha;
                    rgb_sum[2] += sample[2] * alpha;
                }
            }

            if alpha_sum <= 0.0 {
                data.extend_from_slice(&[0, 0, 0, 0]);
            } else {
                let alpha = (alpha_sum / 9.0).clamp(0.0, 1.0);
                let r = (rgb_sum[0] / alpha_sum).clamp(0.0, 255.0) as u8;
                let g = (rgb_sum[1] / alpha_sum).clamp(0.0, 255.0) as u8;
                let b = (rgb_sum[2] / alpha_sum).clamp(0.0, 255.0) as u8;
                data.extend_from_slice(&[(alpha * 255.0) as u8, r, g, b]);
            }
        }
    }
    data
}

pub fn icons(state: TrayState, battery: Option<u8>, text_color: [u8; 3]) -> Vec<Icon> {
    SIZES
        .iter()
        .map(|&size| Icon {
            width: size,
            height: size,
            data: render(size, state, battery, text_color),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icon_sizes_and_argb() {
        let list = icons(TrayState::Connected, Some(87), [255, 255, 255]);
        assert_eq!(list.len(), SIZES.len());
        for icon in list {
            assert_eq!(icon.data.len(), (icon.width * icon.height * 4) as usize);
        }
    }

    #[test]
    fn icons_have_transparent_corners() {
        for icon in icons(TrayState::Connected, Some(87), [255, 255, 255]) {
            assert_eq!(&icon.data[..4], &[0, 0, 0, 0]);
        }
    }
}
