//! Crisp runtime RGBA application icon for Age Max window chrome and taskbar.

use eframe::egui::IconData;
use std::sync::Arc;

/// Generate the Age Max window/taskbar icon as a 64x64 RGBA `IconData`.
pub fn window_icon() -> Arc<IconData> {
    const SIZE: u32 = 64;
    Arc::new(IconData {
        rgba: render_rgba(SIZE),
        width: SIZE,
        height: SIZE,
    })
}

/// Render an anti-aliased geometric Age Max badge (dark slate tile, cyan frame & split glyph).
pub fn render_rgba(size: u32) -> Vec<u8> {
    let mut pixels = vec![0u8; (size * size * 4) as usize];
    let s = size as f32;

    for y in 0..size {
        for x in 0..size {
            let u = (x as f32 + 0.5) / s;
            let v = (y as f32 + 0.5) / s;

            // Signed distance to rounded rectangle badge
            let radius = 0.22;
            let half = 0.46;
            let qx = (u - 0.5).abs() - (half - radius);
            let qy = (v - 0.5).abs() - (half - radius);
            let outside =
                (qx.max(0.0).powi(2) + qy.max(0.0).powi(2)).sqrt() + qx.max(qy).min(0.0) - radius;

            let coverage = (0.02 - outside) / 0.04;
            let alpha = coverage.clamp(0.0, 1.0);
            if alpha <= 0.001 {
                continue;
            }

            // Base dark navy gradient (#0B1220 -> #172554)
            let mut r = 13.0 + u * 8.0 + v * 10.0;
            let mut g = 20.0 + u * 14.0 + v * 12.0;
            let mut b = 38.0 + u * 28.0 + v * 22.0;

            // Crisp cyan perimeter ring inside rounded badge
            if outside > -0.055 && outside < -0.015 {
                r = 56.0;
                g = 189.0;
                b = 248.0;
            }

            // Left sidebar bar (representing workspace rail)
            if (0.19..=0.29).contains(&u) && (0.24..=0.76).contains(&v) {
                r = 99.0;
                g = 102.0;
                b = 241.0;
            }

            // Terminal prompt chevron ">"
            let upper_leg =
                ((v - (0.31 + (u - 0.37) * 0.85)).abs() < 0.048) && (0.37..=0.56).contains(&u);
            let lower_leg =
                ((v - (0.69 - (u - 0.37) * 0.85)).abs() < 0.048) && (0.37..=0.56).contains(&u);
            if upper_leg || lower_leg {
                r = 56.0;
                g = 211.0;
                b = 252.0;
            }

            // Active prompt cursor underscore "_"
            if (0.58..=0.80).contains(&u) && (0.63..=0.71).contains(&v) {
                r = 241.0;
                g = 245.0;
                b = 249.0;
            }

            let idx = ((y * size + x) * 4) as usize;
            pixels[idx] = r.clamp(0.0, 255.0) as u8;
            pixels[idx + 1] = g.clamp(0.0, 255.0) as u8;
            pixels[idx + 2] = b.clamp(0.0, 255.0) as u8;
            pixels[idx + 3] = (alpha * 255.0) as u8;
        }
    }

    pixels
}
