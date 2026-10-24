//! Procedural hardware: knobs, arc meters, neon buttons, bracket frames.
//!
//! Everything is painted with egui's `Painter`; there are no image assets, so
//! the panel scales with DPI and the screenshot tool renders the same code the
//! plugin runs.

use crate::theme::*;
use egui::{
    pos2, Align2, Color32, FontId, Id, Pos2, Rect, Response, Sense, Shape, Stroke, Ui, Vec2,
};

const SWEEP_START: f32 = 0.75 * std::f32::consts::PI;
const SWEEP: f32 = 1.5 * std::f32::consts::PI;

fn arc_points(center: Pos2, radius: f32, from: f32, to: f32) -> Vec<Pos2> {
    let steps = 40;
    (0..=steps)
        .map(|i| {
            let t = i as f32 / steps as f32;
            let angle = from + (to - from) * t;
            center + Vec2::new(angle.cos(), angle.sin()) * radius
        })
        .collect()
}

pub struct KnobOutput {
    pub response: Response,
    pub change: Option<f32>,
    pub reset: bool,
}

/// A rotary with drag, shift-for-fine, wheel and double-click-to-reset.
// Every argument is a distinct knob property; a builder would be longer than
// the call sites it replaces.
#[allow(clippy::too_many_arguments)]
pub fn knob(
    ui: &Ui,
    rect: Rect,
    id: egui::Id,
    label: &str,
    value_text: &str,
    norm: f32,
    accent: Color32,
    engaged: bool,
    default_norm: f32,
) -> KnobOutput {
    let response = ui.interact(rect, id, Sense::click_and_drag());
    let mut change = None;
    let mut reset = false;

    if response.drag_started() {
        ui.memory_mut(|m| m.request_focus(id));
    }

    let fine = ui.input(|i| i.modifiers.shift);
    if response.dragged() {
        let delta = -response.drag_delta().y;
        let scale = if fine { 0.0012 } else { 0.006 };
        if delta != 0.0 {
            change = Some((norm + delta * scale).clamp(0.0, 1.0));
        }
    }

    if response.hovered() {
        let scroll = ui.input(|i| i.raw_scroll_delta.y);
        if scroll.abs() > 0.01 {
            let scale = if fine { 0.0008 } else { 0.0035 };
            change = Some((norm + scroll * scale).clamp(0.0, 1.0));
        }
    }

    if response.double_clicked() {
        reset = true;
        change = Some(default_norm);
    }

    let painter = ui.painter();
    let width = rect.width();
    let label_h = 11.0;
    let value_h = 12.0;
    let radius = (width.min(rect.height() - label_h - value_h) * 0.5 - 3.0).clamp(9.0, 38.0);
    let center = pos2(rect.center().x, rect.top() + label_h + 2.0 + radius + 1.0);

    painter.text(
        pos2(rect.center().x, rect.top()),
        Align2::CENTER_TOP,
        label,
        FontId::monospace(8.5),
        if engaged { accent } else { DIM },
    );

    // Detents around the track.
    for i in 0..=10 {
        let t = i as f32 / 10.0;
        let angle = SWEEP_START + SWEEP * t;
        let dir = Vec2::new(angle.cos(), angle.sin());
        let (r0, r1) = (radius + 3.0, radius + 6.0);
        let color = if t <= norm + 0.001 && norm > 0.001 {
            with_alpha(accent, 0.55)
        } else {
            with_alpha(EDGE_HI, 0.8)
        };
        painter.line_segment(
            [center + dir * r0, center + dir * r1],
            Stroke::new(1.0_f32, color),
        );
    }

    // Track and value arc, with a cheap glow pass.
    let track = arc_points(center, radius, SWEEP_START, SWEEP_START + SWEEP);
    painter.add(Shape::line(track, Stroke::new(2.5_f32, PANEL_LO)));
    if norm > 0.001 {
        let end = SWEEP_START + SWEEP * norm;
        let lit = arc_points(center, radius, SWEEP_START, end);
        painter.add(Shape::line(lit.clone(), Stroke::new(6.0_f32, glow(accent))));
        painter.add(Shape::line(
            lit.clone(),
            Stroke::new(3.0_f32, with_alpha(accent, 0.55)),
        ));
        painter.add(Shape::line(lit, Stroke::new(1.6_f32, accent)));
        let tip = center + Vec2::new(end.cos(), end.sin()) * (radius + 3.0);
        painter.circle_filled(tip, 2.2, accent);
    }

    // Body.
    painter.circle_filled(center, radius, PANEL_HI);
    painter.circle_stroke(center, radius, Stroke::new(1.0_f32, EDGE_HI));
    painter.circle_stroke(
        center,
        radius - 2.5,
        Stroke::new(1.0_f32, with_alpha(EDGE, 0.65)),
    );
    painter.circle_filled(center, radius * 0.42, with_alpha(PANEL_LO, 0.85));

    // Pointer.
    let angle = SWEEP_START + SWEEP * norm.clamp(0.0, 1.0);
    let dir = Vec2::new(angle.cos(), angle.sin());
    let pointer_color = if engaged || response.hovered() {
        accent
    } else {
        with_alpha(TEXT, 0.85)
    };
    painter.line_segment(
        [
            center + dir * (radius * 0.32),
            center + dir * (radius * 0.92),
        ],
        Stroke::new(2.0_f32, pointer_color),
    );
    if engaged {
        painter.circle_stroke(
            center,
            radius + 4.0,
            Stroke::new(1.0_f32, with_alpha(accent, 0.5)),
        );
    }

    painter.text(
        pos2(rect.center().x, center.y + radius + 4.0),
        Align2::CENTER_TOP,
        value_text,
        FontId::monospace(9.5),
        if engaged {
            accent
        } else {
            with_alpha(TEXT, 0.75)
        },
    );

    KnobOutput {
        response,
        change,
        reset,
    }
}

/// Horizontal bar meter, used for envelope stages and the voice count.
pub fn meter(painter: &egui::Painter, rect: Rect, level: f32, accent: Color32, bg: Color32) {
    painter.rect_filled(rect, egui::CornerRadius::same(2), bg);
    let w = (rect.width() * level.clamp(0.0, 1.0)).max(0.0);
    if w > 0.5 {
        let lit = Rect::from_min_size(rect.min, Vec2::new(w, rect.height()));
        painter.rect_filled(lit, egui::CornerRadius::same(2), with_alpha(accent, 0.75));
        painter.rect_filled(
            Rect::from_min_size(rect.min, Vec2::new(w, 1.0)),
            egui::CornerRadius::same(1),
            accent,
        );
    }
}

pub fn led(painter: &egui::Painter, center: Pos2, radius: f32, color: Color32, on: bool) {
    let c = if on { color } else { with_alpha(color, 0.16) };
    if on {
        painter.circle_filled(center, radius * 2.2, glow(color));
    }
    painter.circle_filled(center, radius, c);
    painter.circle_stroke(
        center,
        radius,
        Stroke::new(1.0_f32, with_alpha(EDGE_HI, 0.9)),
    );
}

/// A frame with cut corners and a title rail, the panel's section chrome.
pub fn section(ui: &Ui, rect: Rect, title: &str, accent: Color32, subtitle: Option<&str>) -> Rect {
    let painter = ui.painter();
    painter.rect_filled(rect, egui::CornerRadius::same(3), PANEL);
    painter.rect_stroke(
        rect,
        egui::CornerRadius::same(3),
        Stroke::new(1.0_f32, with_alpha(EDGE, 0.95)),
        egui::StrokeKind::Inside,
    );

    // Corner brackets.
    let b = 9.0;
    for (corner, dx, dy) in [
        (rect.left_top(), 1.0, 1.0),
        (rect.right_top(), -1.0, 1.0),
        (rect.left_bottom(), 1.0, -1.0),
        (rect.right_bottom(), -1.0, -1.0),
    ] {
        let p = pos2(corner.x + dx, corner.y + dy);
        painter.line_segment(
            [p, pos2(p.x + dx * b, p.y)],
            Stroke::new(1.4_f32, with_alpha(accent, 0.75)),
        );
        painter.line_segment(
            [p, pos2(p.x, p.y + dy * b)],
            Stroke::new(1.4_f32, with_alpha(accent, 0.75)),
        );
    }

    painter.text(
        pos2(rect.left() + 9.0, rect.top() + 6.0),
        Align2::LEFT_TOP,
        title,
        FontId::monospace(8.5),
        with_alpha(accent, 0.9),
    );
    if let Some(sub) = subtitle {
        painter.text(
            pos2(rect.right() - 9.0, rect.top() + 6.0),
            Align2::RIGHT_TOP,
            sub,
            FontId::monospace(7.5),
            with_alpha(DIM, 0.9),
        );
    }
    // Content area below the title rail.
    Rect::from_min_max(
        pos2(rect.left() + 8.0, rect.top() + 18.0),
        pos2(rect.right() - 8.0, rect.bottom() - 8.0),
    )
}

pub fn neon_button(
    ui: &mut Ui,
    rect: Rect,
    label: &str,
    accent: Color32,
    active: bool,
) -> Response {
    // Labels repeat across sections (`PITCH` in both LFOs), so the rect is part
    // of the id; otherwise egui reports duplicate widgets and drops clicks.
    let id = Id::new((
        "neon",
        label,
        rect.left().round() as i32,
        rect.top().round() as i32,
    ));
    let response = ui.interact(rect, id, Sense::click());
    let painter = ui.painter();
    let bg = if active {
        with_alpha(accent, 0.18)
    } else if response.hovered() {
        with_alpha(accent, 0.10)
    } else {
        PANEL_LO
    };
    painter.rect_filled(rect, egui::CornerRadius::same(2), bg);
    painter.rect_stroke(
        rect,
        egui::CornerRadius::same(2),
        Stroke::new(
            if active { 1.2_f32 } else { 1.0_f32 },
            if active {
                accent
            } else {
                with_alpha(EDGE_HI, 0.9)
            },
        ),
        egui::StrokeKind::Inside,
    );
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        label,
        FontId::monospace(8.5),
        if active {
            accent
        } else {
            with_alpha(TEXT, 0.8)
        },
    );
    response
}

/// The oscilloscope in the middle screen.
pub fn scope(painter: &egui::Painter, rect: Rect, samples: &[f32], accent: Color32) {
    if samples.len() < 2 {
        return;
    }
    let mid = rect.center().y;
    let amp = rect.height() * 0.36;
    let step = rect.width() / (samples.len() - 1) as f32;
    let points: Vec<Pos2> = samples
        .iter()
        .enumerate()
        .map(|(i, s)| {
            pos2(
                rect.left() + i as f32 * step,
                mid - s.clamp(-1.5, 1.5) * amp,
            )
        })
        .collect();

    painter.add(Shape::line(
        points.clone(),
        Stroke::new(4.0_f32, with_alpha(accent, 0.12)),
    ));
    painter.add(Shape::line(
        points.clone(),
        Stroke::new(1.6_f32, with_alpha(accent, 0.9)),
    ));
    // A second, dimmer trace reads as phosphor afterglow.
    let ghost: Vec<Pos2> = points.iter().map(|p| pos2(p.x - 6.0, p.y)).collect();
    if ghost.first().map(|p| p.x) >= Some(rect.left() - 8.0) {
        painter.add(Shape::line(
            ghost,
            Stroke::new(1.0_f32, with_alpha(accent, 0.18)),
        ));
    }
    painter.line_segment(
        [pos2(rect.left(), mid), pos2(rect.right(), mid)],
        Stroke::new(1.0_f32, with_alpha(EDGE, 0.7)),
    );
}

pub fn scanlines(painter: &egui::Painter, rect: Rect, spacing: f32, alpha: f32) {
    let mut y = rect.top();
    while y < rect.bottom() {
        painter.line_segment(
            [pos2(rect.left(), y), pos2(rect.right(), y)],
            Stroke::new(
                1.0_f32,
                Color32::from_rgba_unmultiplied(0, 0, 0, (alpha * 255.0) as u8),
            ),
        );
        y += spacing;
    }
}

pub fn chip(painter: &egui::Painter, rect: Rect, text: &str, accent: Color32, filled: bool) {
    painter.rect_filled(
        rect,
        egui::CornerRadius::same(2),
        if filled {
            with_alpha(accent, 0.16)
        } else {
            PANEL_LO
        },
    );
    painter.rect_stroke(
        rect,
        egui::CornerRadius::same(2),
        Stroke::new(1.0_f32, with_alpha(accent, if filled { 0.9 } else { 0.4 })),
        egui::StrokeKind::Inside,
    );
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        text,
        FontId::monospace(8.0),
        if filled {
            accent
        } else {
            with_alpha(DIM, 0.95)
        },
    );
}
