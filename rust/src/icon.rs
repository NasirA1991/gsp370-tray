use ab_glyph::{Font, FontRef, PxScale, ScaleFont, point};
use ksni::Icon;
use std::f64::consts::{FRAC_PI_2, TAU};

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

// Subset of DejaVu Sans Bold containing only the ten digits.  Keeping the
// font embedded avoids fontconfig, Pango, Qt and any runtime font lookup; it
// costs ~2 KiB and gives the same smooth text as the old Qt icon.
const FONT_BYTES: &[u8] = include_bytes!("../assets/NotoSans-Medium-digits.ttf");

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

        // At 16-24 px a one-pixel 5x7 face is crisper than any antialiased
        // outline font: thin strokes, no grey wash, and still room for the
        // 8's counters.
        if size <= 24 {
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
                    1 => 0.74,
                    2 => if size <= 18 { 0.68 } else { 0.60 },
                    _ => 0.48,
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

fn accent(state: TrayState, battery: Option<u8>) -> [u8; 4] {
    if state != TrayState::Connected {
        return match state {
            TrayState::Permission | TrayState::Error => [255, 180, 84, 255],
            TrayState::Off => [154, 162, 173, 255],
            _ => [119, 128, 139, 255],
        };
    }
    match battery {
        Some(0..=10) => [255, 92, 92, 255],
        Some(11..=20) => [255, 180, 84, 255],
        Some(21..=40) => [245, 215, 110, 255],
        Some(_) => [94, 224, 138, 255],
        None => [119, 128, 139, 255],
    }
}

fn sample(
    x: f64,
    y: f64,
    size: f64,
    state: TrayState,
    battery: Option<u8>,
    text: Option<&TextMask>,
) -> [f32; 4] {
    let center = size / 2.0;
    let dx = x - center;
    let dy = y - center;
    let radius = (dx * dx + dy * dy).sqrt();
    if radius > size * 0.47 {
        return [0.0, 0.0, 0.0, 0.0];
    }

    let compact = size <= 24.0;
    let mut color: [f32; 4] = [24.0, 26.0, 30.0, 235.0 / 255.0];

    // At tray sizes (16-24 px) a ring leaves too little room for the digits.
    // Use a compact dark badge with large white text instead; the number is
    // the information the icon exists to convey.
    if !compact {
        let ring_center = size * 0.385;
        let ring_half = size * 0.0425;
        if (radius - ring_center).abs() <= ring_half {
            color = [72.0, 78.0, 87.0, 1.0];
            if state == TrayState::Connected {
                if let Some(level) = battery {
                    let mut angle = dy.atan2(dx);
                    angle = (angle + FRAC_PI_2).rem_euclid(TAU);
                    if angle / TAU <= level.min(100) as f64 / 100.0 {
                        let a = accent(state, battery);
                        color = [a[0] as f32, a[1] as f32, a[2] as f32, 1.0];
                    }
                }
            } else {
                let a = accent(state, battery);
                color = [a[0] as f32, a[1] as f32, a[2] as f32, 1.0];
            }
        }
    }

    if let Some(mask) = text {
        let mut coverage = mask.at(x, y);
        // Thin antialiased stems turn grey against the dark disc at panel
        // sizes.  Keep genuinely covered pixels white while preserving the
        // counters and the soft edge.
        if size <= 32.0 && coverage >= 0.18 {
            coverage = 1.0;
        }
        if coverage > 0.0 {
            color[0] = color[0] * (1.0 - coverage) + 245.0 * coverage;
            color[1] = color[1] * (1.0 - coverage) + 247.0 * coverage;
            color[2] = color[2] * (1.0 - coverage) + 250.0 * coverage;
        }
    }
    color
}

fn render(size: i32, state: TrayState, battery: Option<u8>) -> Vec<u8> {
    let label = if state == TrayState::Connected {
        battery.map(|b| b.min(100).to_string()).unwrap_or_default()
    } else {
        String::new()
    };
    let text = TextMask::new(size as u32, &label);
    let mut data = Vec::with_capacity((size * size * 4) as usize);

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

pub fn icons(state: TrayState, battery: Option<u8>) -> Vec<Icon> {
    SIZES
        .iter()
        .map(|&size| Icon {
            width: size,
            height: size,
            data: render(size, state, battery),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icon_sizes_and_argb() {
        let list = icons(TrayState::Connected, Some(87));
        assert_eq!(list.len(), SIZES.len());
        for icon in list {
            assert_eq!(icon.data.len(), (icon.width * icon.height * 4) as usize);
        }
    }
}
