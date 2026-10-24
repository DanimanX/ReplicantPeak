//! Cyberpunk palette and panel metrics.
//!
//! The Peak's hardware is a black anodised panel with a green OLED and white
//! silkscreen; this is the same layout after a night in a neon-lit basement:
//! deep blue-black metal, cyan and magenta silkscreen, and an amber CRT.

use egui::Color32;

pub const PANEL_W: f32 = 1180.0;
pub const PANEL_H: f32 = 820.0;

pub const BG: Color32 = Color32::from_rgb(5, 5, 9);
pub const PANEL: Color32 = Color32::from_rgb(10, 10, 17);
pub const PANEL_HI: Color32 = Color32::from_rgb(17, 18, 28);
pub const PANEL_LO: Color32 = Color32::from_rgb(7, 7, 12);
pub const EDGE: Color32 = Color32::from_rgb(34, 36, 52);
pub const EDGE_HI: Color32 = Color32::from_rgb(58, 62, 88);

pub const CYAN: Color32 = Color32::from_rgb(0, 238, 255);
pub const MAGENTA: Color32 = Color32::from_rgb(255, 43, 214);
pub const AMBER: Color32 = Color32::from_rgb(255, 190, 70);
pub const LIME: Color32 = Color32::from_rgb(176, 255, 64);
pub const RED: Color32 = Color32::from_rgb(255, 64, 96);
pub const TEXT: Color32 = Color32::from_rgb(206, 212, 232);
pub const DIM: Color32 = Color32::from_rgb(108, 114, 140);
pub const SCREEN_BG: Color32 = Color32::from_rgb(4, 12, 14);

/// Each oscillator gets its own neon, so the three scopes read as separate
/// instruments instead of three copies of the same cyan.
pub fn osc_accent(index: usize) -> Color32 {
    [CYAN, MAGENTA, LIME][index % 3]
}

pub fn accent_for(section: crate::host::Accent) -> Color32 {
    use crate::host::Accent::*;
    match section {
        Osc => CYAN,
        Filter => MAGENTA,
        Env => AMBER,
        Lfo => LIME,
        Fx => Color32::from_rgb(120, 160, 255),
        Voice => Color32::from_rgb(160, 168, 200),
    }
}

pub fn with_alpha(color: Color32, alpha: f32) -> Color32 {
    let a = (alpha.clamp(0.0, 1.0) * 255.0) as u8;
    Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), a)
}

pub fn glow(color: Color32) -> Color32 {
    // The panel's "light" is a soft halo; 18% keeps text-on-neon readable.
    with_alpha(color, 0.18)
}

pub mod layout {
    use super::*;
    use egui::{pos2, Rect, Vec2};

    pub const MARGIN: f32 = 18.0;
    pub const HEADER_H: f32 = 58.0;
    pub const GAP: f32 = 10.0;

    pub fn panel_rect(origin: egui::Pos2) -> Rect {
        Rect::from_min_size(origin, Vec2::new(PANEL_W, PANEL_H))
    }

    pub fn header(origin: egui::Pos2) -> Rect {
        Rect::from_min_size(
            origin + Vec2::new(MARGIN, 10.0),
            Vec2::new(PANEL_W - 2.0 * MARGIN, HEADER_H - 12.0),
        )
    }

    pub fn body(origin: egui::Pos2) -> Rect {
        Rect::from_min_max(
            origin + Vec2::new(MARGIN, HEADER_H + GAP),
            origin + Vec2::new(PANEL_W - MARGIN, PANEL_H - 34.0),
        )
    }

    /// Split a rect into `n` columns with a gap.
    pub fn columns(rect: Rect, n: usize, gap: f32) -> Vec<Rect> {
        if n == 0 {
            return Vec::new();
        }
        let w = (rect.width() - gap * (n as f32 - 1.0)) / n as f32;
        (0..n)
            .map(|i| {
                let x = rect.left() + i as f32 * (w + gap);
                Rect::from_min_size(pos2(x, rect.top()), Vec2::new(w, rect.height()))
            })
            .collect()
    }

    /// Split a rect into rows with a gap.
    pub fn rows(rect: Rect, heights: &[f32], gap: f32) -> Vec<Rect> {
        let mut y = rect.top();
        heights
            .iter()
            .map(|h| {
                let r = Rect::from_min_size(pos2(rect.left(), y), Vec2::new(rect.width(), *h));
                y += h + gap;
                r
            })
            .collect()
    }
}

/// Small utilities shared by the widgets.
pub fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let f = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t) as u8;
    Color32::from_rgb(f(a.r(), b.r()), f(a.g(), b.g()), f(a.b(), b.b()))
}
