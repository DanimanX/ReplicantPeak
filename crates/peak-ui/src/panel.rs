//! The Peak-style front panel.
//!
//! Layout follows the hardware: three oscillator columns on the left, the
//! display, then filter, envelopes, voice and FX across two rows. Values come
//! from the host every frame, so the panel has no state of its own beyond the
//! drag bookkeeping and the firmware field.

use crate::host::{Accent, LinkStatus, PanelHost, PanelState};
use crate::theme::*;
use crate::widgets;
use egui::{pos2, vec2, Align2, Color32, FontId, Id, Mesh, Rect, Sense, Shape, Stroke, Ui};
use peak_core::params::{format, from_norm, to_norm, Unit};
use peak_core::P;

pub fn show(ui: &mut Ui, host: &mut dyn PanelHost, state: &mut PanelState) {
    let origin = ui.max_rect().min;
    let panel = layout::panel_rect(origin);

    ui.painter()
        .rect_filled(panel, egui::CornerRadius::ZERO, BG);
    draw_grid(ui, panel);
    // The wavetable scopes animate; ask the host for a steady 30 fps.
    ui.ctx()
        .request_repaint_after(std::time::Duration::from_millis(33));
    // Hover state is per-frame: last frame's knob must not stay lit.
    state.hovered = None;

    header(ui, host, state, layout::header(origin));

    let body = layout::body(origin);
    let row1_h = 310.0;
    let row2_h = 220.0;
    let row3_h = body.height() - row1_h - row2_h - 2.0 * layout::GAP;
    let rows = layout::rows(body, &[row1_h, row2_h, row3_h], layout::GAP);

    // Row 1: oscillators and display.
    let r1 = layout::columns(rows[0], 4, layout::GAP);
    let osc_w = 232.0;
    let osc1 = Rect::from_min_size(r1[0].min, vec2(osc_w, r1[0].height()));
    let osc2 = Rect::from_min_size(
        pos2(osc1.right() + layout::GAP, r1[1].min.y),
        vec2(osc_w, r1[1].height()),
    );
    let osc3 = Rect::from_min_size(
        pos2(osc2.right() + layout::GAP, r1[2].min.y),
        vec2(osc_w, r1[2].height()),
    );
    let screen = Rect::from_min_max(
        pos2(osc3.right() + layout::GAP, rows[0].top()),
        pos2(rows[0].right(), rows[0].bottom()),
    );

    for (i, rect) in [osc1, osc2, osc3].iter().enumerate() {
        oscillator(ui, host, state, *rect, i);
    }
    display(ui, host, state, screen);

    // Row 2: filter, two envelopes, voice.
    let filter_rect = Rect::from_min_size(rows[1].min, vec2(420.0, rows[1].height()));
    let amp_rect = Rect::from_min_size(
        pos2(filter_rect.right() + layout::GAP, rows[1].top()),
        vec2(232.0, rows[1].height()),
    );
    let fenv_rect = Rect::from_min_size(
        pos2(amp_rect.right() + layout::GAP, rows[1].top()),
        vec2(232.0, rows[1].height()),
    );
    let voice_rect = Rect::from_min_max(
        pos2(fenv_rect.right() + layout::GAP, rows[1].top()),
        pos2(rows[1].right(), rows[1].bottom()),
    );

    filter_section(ui, host, state, filter_rect);
    envelope(
        ui,
        host,
        state,
        amp_rect,
        "AMP ENV",
        P::AmpAttack,
        P::AmpHold,
        P::AmpDecay,
        P::AmpSustain,
        P::AmpRelease,
    );
    envelope(
        ui,
        host,
        state,
        fenv_rect,
        "FILTER ENV",
        P::FilterAttack,
        P::FilterHold,
        P::FilterDecay,
        P::FilterSustain,
        P::FilterRelease,
    );
    voice_section(ui, host, state, voice_rect);

    // Row 3: LFOs and FX.
    let lfo1 = Rect::from_min_size(rows[2].min, vec2(240.0, rows[2].height()));
    let lfo2 = Rect::from_min_size(
        pos2(lfo1.right() + layout::GAP, rows[2].top()),
        vec2(240.0, rows[2].height()),
    );
    let fx = Rect::from_min_max(
        pos2(lfo2.right() + layout::GAP, rows[2].top()),
        pos2(rows[2].right(), rows[2].bottom()),
    );
    lfo_section(ui, host, state, lfo1, 1);
    lfo_section(ui, host, state, lfo2, 2);
    fx_section(ui, host, state, fx);

    footer(
        ui,
        host,
        state,
        pos2(origin.x, panel.bottom() - 26.0),
        panel.width(),
    );

    // Drag & drop anywhere on the panel relinks the firmware.
    handle_dropped_files(ui, host, state);

    // The gate is modal: without a validated image the panel is visible but
    // inert, and the engine is silent by contract.
    if host.firmware().is_none() {
        gate(ui, host, state, panel);
    }
}

fn draw_grid(ui: &Ui, panel: Rect) {
    let painter = ui.painter();
    let step = 34.0;
    let color = Color32::from_rgba_unmultiplied(40, 46, 70, 26);
    let mut x = panel.left();
    while x < panel.right() {
        painter.line_segment(
            [pos2(x, panel.top()), pos2(x, panel.bottom())],
            Stroke::new(1.0_f32, color),
        );
        x += step;
    }
    let mut y = panel.top();
    while y < panel.bottom() {
        painter.line_segment(
            [pos2(panel.left(), y), pos2(panel.right(), y)],
            Stroke::new(1.0_f32, color),
        );
        y += step;
    }
}

fn header(ui: &mut Ui, host: &mut dyn PanelHost, state: &mut PanelState, rect: Rect) {
    let painter = ui.painter();
    painter.text(
        pos2(rect.left(), rect.center().y - 12.0),
        Align2::LEFT_CENTER,
        "REPLICANT",
        FontId::monospace(26.0),
        CYAN,
    );
    let replicant_w = 26.0 * 0.6 * 9.0;
    painter.text(
        pos2(rect.left() + replicant_w + 8.0, rect.center().y - 12.0),
        Align2::LEFT_CENTER,
        "PEAK",
        FontId::monospace(26.0),
        MAGENTA,
    );
    painter.text(
        pos2(rect.left(), rect.center().y + 14.0),
        Align2::LEFT_CENTER,
        "WAVETABLE SYNTHESIZER  \u{00b7}  8 VOICES  \u{00b7}  FIRMWARE-LINKED ENGINE",
        FontId::monospace(8.0),
        DIM,
    );

    // Firmware chip.
    let chip_rect = Rect::from_min_size(
        pos2(rect.right() - 560.0, rect.center().y - 13.0),
        vec2(292.0, 26.0),
    );
    match host.firmware() {
        Some(fw) => {
            let accent = if fw.verified { LIME } else { AMBER };
            painter.rect_filled(
                chip_rect,
                egui::CornerRadius::same(2),
                with_alpha(accent, 0.08),
            );
            painter.rect_stroke(
                chip_rect,
                egui::CornerRadius::same(2),
                Stroke::new(1.0_f32, with_alpha(accent, 0.7)),
                egui::StrokeKind::Inside,
            );
            painter.text(
                pos2(chip_rect.left() + 10.0, chip_rect.center().y),
                Align2::LEFT_CENTER,
                format!("{}  {}", fw.kind.label(), fw.version_label),
                FontId::monospace(10.0),
                accent,
            );
            painter.text(
                pos2(chip_rect.right() - 10.0, chip_rect.center().y),
                Align2::RIGHT_CENTER,
                if fw.verified {
                    "VERIFIED"
                } else {
                    "UNVERIFIED"
                },
                FontId::monospace(8.0),
                with_alpha(accent, 0.9),
            );
        }
        None => {
            let pulse = 0.45 + 0.25 * (ui.input(|i| i.time) as f32 * 2.0).sin();
            painter.rect_filled(
                chip_rect,
                egui::CornerRadius::same(2),
                with_alpha(RED, 0.07),
            );
            painter.rect_stroke(
                chip_rect,
                egui::CornerRadius::same(2),
                Stroke::new(1.0_f32, with_alpha(RED, pulse)),
                egui::StrokeKind::Inside,
            );
            painter.text(
                chip_rect.center(),
                Align2::CENTER_CENTER,
                "NO FIRMWARE  \u{00b7}  LINK REQUIRED",
                FontId::monospace(10.0),
                RED,
            );
        }
    }

    // Voice meter.
    let voic_rect = Rect::from_min_size(
        pos2(rect.right() - 250.0, rect.center().y - 9.0),
        vec2(250.0, 18.0),
    );
    let max = host.max_voices().max(1) as f32;
    let cell_w = voic_rect.width() / max;
    for i in 0..max as usize {
        let cell = Rect::from_min_size(
            pos2(voic_rect.left() + i as f32 * cell_w, voic_rect.top()),
            vec2(cell_w - 2.0, 5.0),
        );
        let on = i < host.active_voices();
        painter.rect_filled(
            cell,
            egui::CornerRadius::same(1),
            if on { CYAN } else { with_alpha(EDGE, 0.9) },
        );
    }
    painter.text(
        pos2(voic_rect.left(), voic_rect.bottom() + 2.0),
        Align2::LEFT_TOP,
        format!("VOICES {}/{}", host.active_voices(), host.max_voices()),
        FontId::monospace(7.5),
        DIM,
    );

    // Patch name, editable in the preview, read-only in a host for now.
    painter.text(
        pos2(voic_rect.right(), voic_rect.bottom() + 16.0),
        Align2::RIGHT_BOTTOM,
        host.preset_name(),
        FontId::monospace(8.0),
        with_alpha(TEXT, 0.8),
    );
    let _ = state;
}

/// The preset browser, drawn inside the display: previous/next, slot number and
/// a store button. Hidden behind a single row so it never fights the readout.
fn preset_bar(ui: &mut Ui, host: &mut dyn PanelHost, state: &mut PanelState, inner: Rect) {
    let names = host.preset_names();
    let count = names.len();
    let index = host.preset_index().min(count.saturating_sub(1));

    let bar = Rect::from_min_max(
        pos2(inner.left(), inner.bottom() - 20.0),
        pos2(inner.right(), inner.bottom()),
    );
    let button_w = 22.0;
    let prev = Rect::from_min_size(bar.min, vec2(button_w, bar.height()));
    let next = Rect::from_min_size(
        pos2(prev.right() + 4.0, bar.top()),
        vec2(button_w, bar.height()),
    );
    let store = Rect::from_min_size(
        pos2(bar.right() - 58.0, bar.top()),
        vec2(58.0, bar.height()),
    );

    let enabled = count > 0 && host.is_live();
    let prev_clicked = widgets::neon_button(ui, prev, "<", CYAN, false).clicked();
    let next_clicked = widgets::neon_button(ui, next, ">", CYAN, false).clicked();
    let store_clicked = widgets::neon_button(ui, store, "STORE", AMBER, false).clicked();

    if prev_clicked && enabled {
        let target = if index == 0 { count - 1 } else { index - 1 };
        host.select_preset(target);
        state.preset_msg = None;
    }
    if next_clicked && enabled {
        let target = if index + 1 >= count { 0 } else { index + 1 };
        host.select_preset(target);
        state.preset_msg = None;
    }
    if store_clicked && enabled {
        state.preset_msg = Some(match host.store_preset() {
            Ok(name) => format!("STORED {name}").to_uppercase(),
            Err(error) => format!("NOT STORED: {error}").to_uppercase(),
        });
    }

    let counter = if count == 0 {
        "NO PRESETS".to_string()
    } else {
        format!("{:02}/{:02}", index + 1, count)
    };
    let painter = ui.painter();
    painter.text(
        pos2(next.right() + 8.0, bar.center().y),
        Align2::LEFT_CENTER,
        counter,
        FontId::monospace(8.5),
        DIM,
    );
    painter.text(
        pos2(next.right() + 58.0, bar.center().y),
        Align2::LEFT_CENTER,
        host.preset_name().to_uppercase(),
        FontId::monospace(8.5),
        with_alpha(TEXT, 0.75),
    );
}

fn oscillator(
    ui: &mut Ui,
    host: &mut dyn PanelHost,
    state: &mut PanelState,
    rect: Rect,
    index: usize,
) {
    let (level, wave, shape, tune, fine) = match index {
        0 => (
            P::Osc1Level,
            P::Osc1Shape,
            P::Osc1ShapeAmt,
            P::Osc1Tune,
            P::Osc1Fine,
        ),
        1 => (
            P::Osc2Level,
            P::Osc2Shape,
            P::Osc2ShapeAmt,
            P::Osc2Tune,
            P::Osc2Fine,
        ),
        _ => (
            P::Osc3Level,
            P::Osc3Shape,
            P::Osc3ShapeAmt,
            P::Osc3Tune,
            P::Osc3Fine,
        ),
    };
    let fm = match index {
        1 => Some(P::Fm12),
        2 => Some(P::Fm23),
        _ => None,
    };
    let accent = osc_accent(index);
    let content = widgets::section(
        ui,
        rect,
        &format!("OSC {}", index + 1),
        accent,
        Some(if index == 2 {
            "LOW / FM IN"
        } else {
            "NCO / GPU SCOPE"
        }),
    );

    let wave_h = 16.0;
    let view_h = 88.0;
    let bottom_h = 74.0;
    let gap = 4.0;
    let shape_h = (content.height() - wave_h - view_h - bottom_h - gap * 3.0).clamp(72.0, 118.0);

    let wave_rect = Rect::from_min_size(content.min, vec2(content.width(), wave_h));
    let view_rect = Rect::from_min_size(
        pos2(content.left(), wave_rect.bottom() + gap),
        vec2(content.width(), view_h),
    );
    let shape_rect = Rect::from_center_size(
        pos2(content.center().x, view_rect.bottom() + gap + shape_h * 0.5),
        vec2(content.width() * 0.90, shape_h),
    );

    wave_strip(ui, host, wave_rect, wave, accent);
    wavetable_view(ui, host, state, view_rect, wave, shape, index, accent);
    param_knob(ui, host, state, shape_rect, shape, accent);

    let params = [level, tune, fine, fm.unwrap_or(level)];
    let count = if fm.is_some() { 4 } else { 3 };
    let knob_w = content.width() / count as f32;
    let knob_y = content.bottom() - bottom_h;
    for (i, p) in params.iter().take(count).enumerate() {
        let r = Rect::from_min_size(
            pos2(content.left() + i as f32 * knob_w, knob_y),
            vec2(knob_w, bottom_h),
        );
        param_knob(ui, host, state, r, *p, accent);
    }
}

/// The compact wave selector above each scope. The arrows step one table; the
/// middle strip can be dragged across the whole bank.
fn wave_strip(ui: &mut Ui, host: &mut dyn PanelHost, rect: Rect, param: P, accent: Color32) {
    let names = peak_core::params::WAVE_NAMES;
    let count = names.len();
    let value = host.value(param).round().clamp(0.0, (count - 1) as f32) as usize;
    let arrow = 16.0;
    let left = Rect::from_min_size(rect.min, vec2(arrow, rect.height()));
    let right = Rect::from_min_size(
        pos2(rect.right() - arrow, rect.top()),
        vec2(arrow, rect.height()),
    );
    let mid = Rect::from_min_max(
        pos2(left.right(), rect.top()),
        pos2(right.left(), rect.bottom()),
    );

    let painter = ui.painter();
    painter.rect_filled(rect, egui::CornerRadius::same(2), PANEL_LO);
    painter.rect_stroke(
        rect,
        egui::CornerRadius::same(2),
        Stroke::new(1.0_f32, with_alpha(EDGE_HI, 0.65)),
        egui::StrokeKind::Inside,
    );

    let prev = ui.interact(
        left,
        Id::new(("wave-prev", param.spec().id)),
        Sense::click(),
    );
    let next = ui.interact(
        right,
        Id::new(("wave-next", param.spec().id)),
        Sense::click(),
    );
    let scrub = ui.interact(
        mid,
        Id::new(("wave-scrub", param.spec().id)),
        Sense::click_and_drag(),
    );

    painter.text(
        left.center(),
        Align2::CENTER_CENTER,
        "\u{2039}",
        FontId::monospace(11.0),
        if prev.hovered() { accent } else { DIM },
    );
    painter.text(
        right.center(),
        Align2::CENTER_CENTER,
        "\u{203a}",
        FontId::monospace(11.0),
        if next.hovered() { accent } else { DIM },
    );
    painter.text(
        mid.center(),
        Align2::CENTER_CENTER,
        format!(
            "{:02}/{:02}  {}",
            value + 1,
            count,
            names[value].to_uppercase()
        ),
        FontId::monospace(8.5),
        accent,
    );

    let mut target = None;
    if prev.clicked() {
        target = Some((value + count - 1) % count);
    }
    if next.clicked() {
        target = Some((value + 1) % count);
    }
    if scrub.dragged() {
        let x = ui
            .input(|i| i.pointer.interact_pos())
            .map(|p| p.x)
            .unwrap_or(mid.center().x);
        let t = ((x - mid.left()) / mid.width().max(1.0)).clamp(0.0, 0.999);
        target = Some((t * count as f32) as usize);
    }
    if scrub.hovered() {
        let scroll = ui.input(|i| i.raw_scroll_delta.y);
        if scroll.abs() > 0.1 {
            let step = if scroll > 0.0 { 1 } else { count - 1 };
            target = Some((value + step) % count);
        }
    }
    if let Some(index) = target {
        host.begin_edit(param);
        host.set(param, index as f32);
        host.end_edit(param);
    }
}

/// A five-column wavetable waterfall. All geometry is submitted as egui meshes,
/// so the GPU rasterises the gradients, glow layers and waveform fills.
#[allow(clippy::too_many_arguments)]
fn wavetable_view(
    ui: &mut Ui,
    host: &mut dyn PanelHost,
    state: &mut PanelState,
    rect: Rect,
    wave_param: P,
    shape_param: P,
    index: usize,
    accent: Color32,
) {
    let wave = host.value(wave_param).round() as usize;
    let frames = host.wavetable(wave, 96);
    let response = ui.interact(
        rect,
        Id::new(("wavetable-view", index)),
        Sense::click_and_drag(),
    );

    if response.drag_started() {
        state.engaged = Some(shape_param);
        host.begin_edit(shape_param);
    }
    if response.dragged() {
        let spec = shape_param.spec();
        let norm = to_norm(spec, host.value(shape_param));
        let fine = ui.input(|i| i.modifiers.shift);
        let scale = if fine { 0.0015 } else { 0.006 };
        host.set(
            shape_param,
            from_norm(spec, norm + response.drag_delta().x * scale),
        );
    }
    if response.drag_stopped() {
        state.engaged = None;
        host.end_edit(shape_param);
    }
    if response.double_clicked() {
        host.begin_edit(shape_param);
        host.set(shape_param, 0.0);
        host.end_edit(shape_param);
    }
    if response.hovered() {
        state.hovered = Some(shape_param);
        let scroll = ui.input(|i| i.raw_scroll_delta.y);
        if scroll.abs() > 0.1 {
            let count = peak_core::params::WAVE_NAMES.len();
            let step = if scroll > 0.0 { 1 } else { count - 1 };
            host.begin_edit(wave_param);
            host.set(wave_param, ((wave + step) % count) as f32);
            host.end_edit(wave_param);
        }
    }

    let painter = ui.painter();
    gradient_rect(
        painter,
        rect,
        Color32::from_rgb(4, 9, 13),
        Color32::from_rgb(2, 4, 7),
    );

    // Perspective floor grid.
    let grid = with_alpha(accent, 0.11);
    let mut row = 0;
    while row <= 4 {
        let t = row as f32 / 4.0;
        let y = rect.top() + rect.height() * (0.28 + 0.68 * t);
        let inset = rect.width() * 0.12 * (1.0 - t) + 2.0;
        painter.line_segment(
            [pos2(rect.left() + inset, y), pos2(rect.right() - inset, y)],
            Stroke::new(1.0_f32, grid),
        );
        row += 1;
    }
    for i in 0..=6 {
        let t = i as f32 / 6.0;
        let top = pos2(
            rect.left() + rect.width() * (0.25 + 0.5 * t),
            rect.top() + 8.0,
        );
        let bottom = pos2(rect.left() + rect.width() * t, rect.bottom() - 2.0);
        painter.line_segment(
            [top, bottom],
            Stroke::new(1.0_f32, with_alpha(accent, 0.07)),
        );
    }

    if frames.len() < 2 {
        painter.text(
            rect.center(),
            Align2::CENTER_CENTER,
            "NO WAVETABLE BANK",
            FontId::monospace(8.0),
            with_alpha(DIM, 0.9),
        );
        widgets::scanlines(painter, rect, 3.0, 0.12);
        return;
    }

    let shape = host.value(shape_param).clamp(-1.0, 1.0);
    let position = (shape + 1.0) * 0.5 * (frames.len() - 1) as f32;
    let f0 = position.floor() as usize;
    let f1 = (f0 + 1).min(frames.len() - 1);
    let frac = position - f0 as f32;
    let current: Vec<f32> = frames[f0]
        .iter()
        .zip(frames[f1].iter())
        .map(|(a, b)| a + (b - a) * frac)
        .collect();

    // Waterfall: the far columns sit higher and narrower, so the five frames
    // read as a depth stack rather than five overlapping lines.
    let plane_top = rect.top() + 18.0;
    let plane_bottom = rect.bottom() - 10.0;
    let span = plane_bottom - plane_top;
    let last = (frames.len() - 1) as f32;
    let secondary = if accent == MAGENTA { CYAN } else { MAGENTA };
    for frame in (0..frames.len()).rev() {
        let t = frame as f32 / last;
        let depth = (t - 0.5).abs() * 2.0;
        let center_y = plane_top + span * (0.12 + 0.64 * t);
        let amp = span * 0.115;
        let width_scale = 0.78 + 0.22 * (1.0 - t);
        let points = trace_points(rect, &frames[frame], center_y, amp, width_scale);
        let color = mix(accent, secondary, t);
        fill_trace(
            painter,
            &points,
            plane_bottom,
            with_alpha(color, 0.10 * (1.0 - depth * 0.55)),
        );
        painter.add(Shape::line(
            points,
            Stroke::new(1.0_f32, with_alpha(color, 0.22 + 0.34 * (1.0 - depth))),
        ));
    }

    // The interpolated waveform the oscillator is actually reading.
    let points = trace_points(rect, &current, plane_bottom - 3.0, span * 0.30, 1.0);
    let bright = mix(accent, Color32::WHITE, 0.45);
    painter.add(Shape::line(
        points.clone(),
        Stroke::new(5.0_f32, with_alpha(accent, 0.13)),
    ));
    painter.add(Shape::line(
        points.clone(),
        Stroke::new(2.0_f32, with_alpha(bright, 0.72)),
    ));
    painter.add(Shape::line(points.clone(), Stroke::new(1.2_f32, bright)));

    // A phase marker runs along the live waveform.
    let t = (ui.input(|i| i.time) as f32 * 0.35).rem_euclid(1.0);
    let marker = points[(t * (points.len() - 1) as f32) as usize];
    painter.circle_filled(marker, 5.0, with_alpha(accent, 0.16));
    painter.circle_filled(marker, 1.8, Color32::WHITE);

    // Neon sweep.
    let sweep = ((ui.input(|i| i.time) as f32 * 0.20).rem_euclid(1.0)) * rect.width();
    painter.line_segment(
        [
            pos2(rect.left() + sweep, rect.top() + 6.0),
            pos2(rect.left() + sweep, rect.bottom() - 2.0),
        ],
        Stroke::new(1.0_f32, with_alpha(accent, 0.16)),
    );

    painter.text(
        pos2(rect.left() + 5.0, rect.top() + 4.0),
        Align2::LEFT_TOP,
        format!("COL {:.1}/{}", position + 1.0, frames.len()),
        FontId::monospace(7.5),
        with_alpha(accent, 0.85),
    );
    painter.text(
        pos2(rect.right() - 5.0, rect.top() + 4.0),
        Align2::RIGHT_TOP,
        format!("{:+.0}%", shape * 100.0),
        FontId::monospace(7.5),
        with_alpha(TEXT, 0.75),
    );
    painter.text(
        pos2(rect.right() - 5.0, rect.bottom() - 9.0),
        Align2::RIGHT_BOTTOM,
        "5 COL",
        FontId::monospace(6.5),
        with_alpha(DIM, 0.9),
    );

    // Column indicator: the active interpolated column is lit.
    for frame in 0..frames.len() {
        let y = rect.top() + 20.0 + frame as f32 * 7.0;
        let active = (position - frame as f32).abs() < 0.5;
        let dot = Rect::from_min_size(pos2(rect.right() - 5.0, y), vec2(2.5, 4.0));
        painter.rect_filled(
            dot,
            egui::CornerRadius::same(1),
            if active {
                accent
            } else {
                with_alpha(accent, 0.22)
            },
        );
    }

    widgets::scanlines(painter, rect, 3.0, 0.10);
    painter.rect_stroke(
        rect,
        egui::CornerRadius::same(2),
        Stroke::new(1.0_f32, with_alpha(accent, 0.55)),
        egui::StrokeKind::Inside,
    );
}

fn trace_points(
    rect: Rect,
    samples: &[f32],
    center_y: f32,
    amp: f32,
    width_scale: f32,
) -> Vec<egui::Pos2> {
    if samples.len() < 2 {
        return Vec::new();
    }
    let usable = rect.width() * width_scale;
    let left = rect.center().x - usable * 0.5;
    let step = usable / (samples.len() - 1) as f32;
    samples
        .iter()
        .enumerate()
        .map(|(i, s)| pos2(left + i as f32 * step, center_y - s.clamp(-1.5, 1.5) * amp))
        .collect()
}

fn fill_trace(painter: &egui::Painter, points: &[egui::Pos2], base_y: f32, color: Color32) {
    if points.len() < 2 {
        return;
    }
    let mut mesh = Mesh::default();
    for p in points {
        mesh.colored_vertex(*p, color);
        mesh.colored_vertex(pos2(p.x, base_y), Color32::TRANSPARENT);
    }
    for i in 0..points.len() - 1 {
        let a = (i * 2) as u32;
        mesh.add_triangle(a, a + 1, a + 2);
        mesh.add_triangle(a + 1, a + 3, a + 2);
    }
    painter.add(Shape::mesh(mesh));
}

fn gradient_rect(painter: &egui::Painter, rect: Rect, top: Color32, bottom: Color32) {
    let mut mesh = Mesh::default();
    mesh.colored_vertex(rect.left_top(), top);
    mesh.colored_vertex(rect.right_top(), top);
    mesh.colored_vertex(rect.left_bottom(), bottom);
    mesh.colored_vertex(rect.right_bottom(), bottom);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(1, 3, 2);
    painter.add(Shape::mesh(mesh));
}

fn display(ui: &mut Ui, host: &mut dyn PanelHost, state: &mut PanelState, rect: Rect) {
    let content = widgets::section(ui, rect, "DISPLAY", CYAN, Some("OLED 256\u{00d7}64"));

    let screen = Rect::from_min_max(content.min, content.max);
    ui.painter()
        .rect_filled(screen, egui::CornerRadius::same(2), SCREEN_BG);
    ui.painter().rect_stroke(
        screen,
        egui::CornerRadius::same(2),
        Stroke::new(1.0_f32, with_alpha(EDGE_HI, 0.8)),
        egui::StrokeKind::Inside,
    );

    let inner = screen.shrink(10.0);
    let engaged = state.engaged.or(state.hovered);

    // Preset browser: the display is the only place with room, and it is where
    // the hardware shows the patch number too. Drawn before the painter borrow
    // because the buttons need to interact with the `Ui`.
    preset_bar(ui, host, state, inner);

    let painter = ui.painter();

    // Header line inside the screen.
    painter.text(
        pos2(inner.left(), inner.top()),
        Align2::LEFT_TOP,
        "PEAK",
        FontId::monospace(11.0),
        with_alpha(CYAN, 0.9),
    );
    painter.text(
        pos2(inner.left() + 42.0, inner.top()),
        Align2::LEFT_TOP,
        host.preset_name().to_uppercase(),
        FontId::monospace(11.0),
        AMBER,
    );
    match host.firmware() {
        Some(fw) => {
            painter.text(
                pos2(inner.right(), inner.top()),
                Align2::RIGHT_TOP,
                format!("{} {}", fw.version_label, fw.sha_short),
                FontId::monospace(8.0),
                with_alpha(LIME, 0.85),
            );
        }
        None => {
            painter.text(
                pos2(inner.right(), inner.top()),
                Align2::RIGHT_TOP,
                "NO LINK",
                FontId::monospace(8.0),
                RED,
            );
        }
    }

    // Scope.
    let scope_rect = Rect::from_min_max(
        pos2(inner.left(), inner.top() + 22.0),
        pos2(inner.right(), inner.top() + 110.0),
    );
    painter.rect_filled(
        scope_rect,
        egui::CornerRadius::ZERO,
        Color32::from_rgb(3, 9, 11),
    );
    let wave = host.scope_wave();
    widgets::scope(painter, scope_rect.shrink2(vec2(4.0, 6.0)), &wave, CYAN);

    // Parameter readout.
    let readout_y = scope_rect.bottom() + 12.0;
    match engaged {
        Some(p) => {
            let spec = p.spec();
            painter.text(
                pos2(inner.left(), readout_y),
                Align2::LEFT_TOP,
                spec.label,
                FontId::monospace(9.0),
                DIM,
            );
            painter.text(
                pos2(inner.left(), readout_y + 14.0),
                Align2::LEFT_TOP,
                format(spec, host.value(p)),
                FontId::monospace(17.0),
                AMBER,
            );
        }
        None => {
            painter.text(
                pos2(inner.left(), readout_y),
                Align2::LEFT_TOP,
                "IDLE",
                FontId::monospace(9.0),
                DIM,
            );
            let (text, color) = match &state.preset_msg {
                Some(msg) => (msg.as_str(), AMBER),
                None => ("TURN A KNOB", with_alpha(AMBER, 0.35)),
            };
            painter.text(
                pos2(inner.left(), readout_y + 14.0),
                Align2::LEFT_TOP,
                text,
                FontId::monospace(17.0),
                color,
            );
        }
    }

    // Right column: firmware facts.
    let facts_x = inner.right() - 150.0;
    let facts = match host.firmware() {
        Some(fw) => vec![
            ("MSG", format!("{}", fw.messages)),
            ("SIZE", format!("{:.1} KB", fw.bytes as f32 / 1024.0)),
            ("BANK", format!("{} TABLES", peak_core::wavetable::TABLES)),
        ],
        None => vec![("STATUS", "SILENT".to_string())],
    };
    for (i, (key, value)) in facts.iter().enumerate() {
        let y = readout_y + i as f32 * 16.0;
        painter.text(
            pos2(facts_x, y),
            Align2::LEFT_TOP,
            *key,
            FontId::monospace(7.5),
            DIM,
        );
        painter.text(
            pos2(facts_x + 34.0, y),
            Align2::LEFT_TOP,
            value,
            FontId::monospace(8.0),
            with_alpha(TEXT, 0.8),
        );
    }

    widgets::scanlines(painter, screen, 3.0, 0.10);
}

fn filter_section(ui: &mut Ui, host: &mut dyn PanelHost, state: &mut PanelState, rect: Rect) {
    let accent = accent_for(Accent::Filter);
    let content = widgets::section(ui, rect, "FILTER", accent, Some("STATE VARIABLE"));

    let params = [
        P::Cutoff,
        P::Reso,
        P::Drive,
        P::FilterEnvAmt,
        P::Keytrack,
        P::Osc3FilterMod,
        P::NoiseTone,
    ];
    let widths = [0.22, 0.13, 0.13, 0.13, 0.13, 0.13, 0.13];
    let mut x = content.left();
    for (p, weight) in params.iter().zip(widths) {
        let w = content.width() * weight;
        let r = Rect::from_min_size(pos2(x, content.top() + 2.0), vec2(w, 118.0));
        param_knob(ui, host, state, r, *p, accent);
        x += w;
    }

    // Filter type selector.
    let types = peak_core::params::FILTER_NAMES;
    let current = host.value(P::FilterType).round() as usize;
    let bar = Rect::from_min_max(
        pos2(content.left(), content.bottom() - 34.0),
        pos2(content.right(), content.bottom() - 12.0),
    );
    let cell = bar.width() / types.len() as f32;
    for (i, name) in types.iter().enumerate() {
        let r = Rect::from_min_size(
            pos2(bar.left() + i as f32 * cell, bar.top()),
            vec2(cell - 4.0, bar.height()),
        );
        if widgets::neon_button(ui, r, name, accent, i == current).clicked() {
            host.begin_edit(P::FilterType);
            host.set(P::FilterType, i as f32);
            host.end_edit(P::FilterType);
        }
    }
}

// A four-stage envelope is four named parameters plus the panel geometry they
// are drawn into; a struct would only move the same fields to the call site.
#[allow(clippy::too_many_arguments)]
fn envelope(
    ui: &mut Ui,
    host: &mut dyn PanelHost,
    state: &mut PanelState,
    rect: Rect,
    title: &str,
    attack: P,
    hold: P,
    decay: P,
    sustain: P,
    release: P,
) {
    let accent = accent_for(Accent::Env);
    let content = widgets::section(ui, rect, title, accent, None);

    let graph = Rect::from_min_max(content.min, pos2(content.right(), content.top() + 44.0));
    draw_env_graph(
        ui, graph, host, attack, hold, decay, sustain, release, accent,
    );

    let knob_y = graph.bottom() + 6.0;
    let cell = content.width() / 5.0;
    for (i, p) in [attack, hold, decay, sustain, release].iter().enumerate() {
        let r = Rect::from_min_size(
            pos2(content.left() + i as f32 * cell, knob_y),
            vec2(cell, content.bottom() - knob_y),
        );
        param_knob(ui, host, state, r, *p, accent);
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_env_graph(
    ui: &Ui,
    rect: Rect,
    host: &dyn PanelHost,
    attack: P,
    hold: P,
    decay: P,
    sustain: P,
    release: P,
    accent: Color32,
) {
    let painter = ui.painter();
    painter.rect_filled(rect, egui::CornerRadius::same(2), PANEL_LO);
    painter.rect_stroke(
        rect,
        egui::CornerRadius::same(2),
        Stroke::new(1.0_f32, with_alpha(EDGE, 0.8)),
        egui::StrokeKind::Inside,
    );
    let inner = rect.shrink(4.0);
    let a = host.value(attack).max(1.0) / 1000.0;
    let h = host.value(hold).max(0.0) / 1000.0;
    let d = host.value(decay).max(1.0) / 1000.0;
    let s = host.value(sustain);
    let r = host.value(release).max(1.0) / 1000.0;
    // A fixed 4s window keeps the graph stable while dragging; long stages clip.
    let window = 4.0f32.max(a + h + d + r);
    let x_at = |t: f32| inner.left() + inner.width() * (t / window).clamp(0.0, 1.0);
    let y_at = |v: f32| inner.bottom() - inner.height() * v.clamp(0.0, 1.0);

    let attack_end = a;
    let hold_end = a + h;
    let decay_end = a + h + d;
    let release_start = decay_end + 0.6 * (window - decay_end).max(0.0);
    let _release_end = release_start + r;

    let mut points = vec![
        pos2(x_at(0.0), y_at(0.0)),
        pos2(x_at(attack_end), y_at(1.0)),
    ];
    if h > 0.0 {
        points.push(pos2(x_at(hold_end), y_at(1.0)));
    }
    let decay_steps = 12;
    for i in 1..=decay_steps {
        let t = i as f32 / decay_steps as f32;
        // Exponential decay toward sustain.
        let v = 1.0 + (s - 1.0) * (1.0 - (-4.0 * t).exp()) / (1.0 - (-4.0f32).exp());
        points.push(pos2(x_at(hold_end + d * t), y_at(v)));
    }
    points.push(pos2(x_at(release_start), y_at(s)));
    for i in 1..=8 {
        let t = i as f32 / 8.0;
        let v = s * (-4.0 * t).exp();
        points.push(pos2(x_at(release_start + r * t), y_at(v)));
    }

    let fill: Vec<egui::Pos2> = std::iter::once(pos2(x_at(0.0), inner.bottom()))
        .chain(points.iter().copied())
        .chain(std::iter::once(pos2(
            points.last().map(|p| p.x).unwrap_or(inner.right()),
            inner.bottom(),
        )))
        .collect();
    painter.add(egui::Shape::convex_polygon(
        fill,
        with_alpha(accent, 0.10),
        Stroke::NONE,
    ));
    painter.add(egui::Shape::line(
        points.clone(),
        Stroke::new(2.6_f32, with_alpha(accent, 0.20)),
    ));
    painter.add(egui::Shape::line(points, Stroke::new(1.3_f32, accent)));
}

fn voice_section(ui: &mut Ui, host: &mut dyn PanelHost, state: &mut PanelState, rect: Rect) {
    let accent = accent_for(Accent::Voice);
    let content = widgets::section(ui, rect, "VOICE", accent, None);
    let cell = content.width() / 5.0;
    for (i, p) in [P::Polyphony, P::Glide, P::BendRange, P::Drift, P::Diverge]
        .iter()
        .enumerate()
    {
        let r = Rect::from_min_size(
            pos2(content.left() + i as f32 * cell, content.top() + 6.0),
            vec2(cell, 112.0),
        );
        param_knob(ui, host, state, r, *p, accent);
    }

    // Hardware 8 Voice LEDs (matching Novation Peak front panel)
    let leds_y = content.bottom() - 46.0;
    let led_step = content.width() / 8.0;
    let active_count = host.active_voices();
    let painter = ui.painter();
    for v in 0..8 {
        let center = pos2(content.left() + (v as f32 + 0.5) * led_step, leds_y);
        let is_lit = v < active_count;
        // Outer metallic bezel
        painter.circle_filled(center, 4.5_f32, PANEL_LO);
        painter.circle_stroke(
            center,
            4.5_f32,
            Stroke::new(1.0_f32, with_alpha(EDGE_HI, 0.8_f32)),
        );
        if is_lit {
            painter.circle_filled(center, 6.5_f32, with_alpha(CYAN, 0.35_f32));
            painter.circle_filled(center, 3.2_f32, CYAN);
            painter.circle_filled(center, 1.2_f32, Color32::WHITE);
        } else {
            painter.circle_filled(center, 3.2_f32, Color32::from_rgb(10, 25, 30));
        }
        painter.text(
            pos2(center.x, center.y + 6.0_f32),
            Align2::CENTER_TOP,
            format!("{}", v + 1),
            FontId::monospace(7.0_f32),
            if is_lit { CYAN } else { DIM },
        );
    }

    widgets::meter(
        ui.painter(),
        Rect::from_min_max(
            pos2(content.left(), content.bottom() - 26.0),
            pos2(content.right(), content.bottom() - 20.0),
        ),
        host.active_voices() as f32 / host.max_voices().max(1) as f32,
        CYAN,
        PANEL_LO,
    );
    ui.painter().text(
        pos2(content.left(), content.bottom() - 16.0),
        Align2::LEFT_TOP,
        format!("{} VOICES ACTIVE", host.active_voices()),
        FontId::monospace(7.5),
        DIM,
    );
}

fn lfo_section(
    ui: &mut Ui,
    host: &mut dyn PanelHost,
    state: &mut PanelState,
    rect: Rect,
    index: usize,
) {
    let (rate, shape, depth, dest, fade) = if index == 1 {
        (
            P::Lfo1Rate,
            P::Lfo1Shape,
            P::Lfo1Depth,
            P::Lfo1Dest,
            P::Lfo1Fade,
        )
    } else {
        (
            P::Lfo2Rate,
            P::Lfo2Shape,
            P::Lfo2Depth,
            P::Lfo2Dest,
            P::Lfo2Fade,
        )
    };
    let accent = accent_for(Accent::Lfo);
    let content = widgets::section(ui, rect, &format!("LFO {index}"), accent, None);

    let cell = content.width() / 4.0;
    for (i, p) in [rate, shape, depth, fade].iter().enumerate() {
        let r = Rect::from_min_size(
            pos2(content.left() + i as f32 * cell, content.top() + 2.0),
            vec2(cell, 92.0),
        );
        param_knob(ui, host, state, r, *p, accent);
    }

    let dests = peak_core::params::LFO_DESTS;
    let current = host.value(dest).round() as usize;
    let bar = Rect::from_min_max(
        pos2(content.left(), content.bottom() - 18.0),
        pos2(content.right(), content.bottom()),
    );
    let w = bar.width() / dests.len() as f32;
    for (i, name) in dests.iter().enumerate() {
        let r = Rect::from_min_size(
            pos2(bar.left() + i as f32 * w, bar.top()),
            vec2(w - 3.0, bar.height()),
        );
        if widgets::neon_button(ui, r, name, accent, i == current).clicked() {
            host.begin_edit(dest);
            host.set(dest, i as f32);
            host.end_edit(dest);
        }
    }
}

fn fx_section(ui: &mut Ui, host: &mut dyn PanelHost, state: &mut PanelState, rect: Rect) {
    let accent = accent_for(Accent::Fx);
    let content = widgets::section(
        ui,
        rect,
        "FX",
        accent,
        Some("DIST \u{2192} CHORUS \u{2192} DELAY \u{2192} REVERB"),
    );
    let params = [
        P::Dist,
        P::Chorus,
        P::DelayTime,
        P::DelayFeedback,
        P::DelayMix,
        P::ReverbSize,
        P::ReverbMix,
    ];
    let cell = content.width() / params.len() as f32;
    for (i, p) in params.iter().enumerate() {
        let r = Rect::from_min_size(
            pos2(content.left() + i as f32 * cell, content.top() + 2.0),
            vec2(cell, content.height() - 2.0),
        );
        param_knob(ui, host, state, r, *p, accent);
    }
}

fn footer(
    ui: &mut Ui,
    host: &mut dyn PanelHost,
    state: &mut PanelState,
    pos: egui::Pos2,
    width: f32,
) {
    let painter = ui.painter();
    let y = pos.y;
    let hint = if host.firmware().is_some() {
        "SHIFT = FINE  \u{00b7}  DOUBLE-CLICK = RESET  \u{00b7}  DROP A .SYX TO RELINK"
    } else {
        "DROP peak-firmware-*.syx ONTO THE PANEL  \u{00b7}  OR ENTER A PATH ABOVE"
    };
    painter.text(
        pos2(pos.x + 18.0, y),
        Align2::LEFT_TOP,
        hint,
        FontId::monospace(7.5),
        with_alpha(DIM, 0.9),
    );
    painter.text(
        pos2(pos.x + width - 18.0, y),
        Align2::RIGHT_TOP,
        format!(
            "PANEL v{} \u{00b7} {}",
            env!("CARGO_PKG_VERSION"),
            if host.is_live() { "LIVE" } else { "PREVIEW" }
        ),
        FontId::monospace(7.5),
        with_alpha(DIM, 0.7),
    );
    let _ = state;
}

fn param_knob(
    ui: &mut Ui,
    host: &mut dyn PanelHost,
    state: &mut PanelState,
    rect: Rect,
    p: P,
    accent: Color32,
) {
    let spec = p.spec();
    let value = host.value(p);
    let norm = to_norm(spec, value);
    let default_norm = to_norm(spec, spec.default);
    let id = Id::new(("knob", spec.id));
    let engaged = state.engaged == Some(p) || state.hovered == Some(p);

    let out = widgets::knob(
        ui,
        rect,
        id,
        spec.label,
        &format(spec, value),
        norm,
        accent,
        engaged,
        default_norm,
    );

    if out.response.hovered() {
        state.hovered = Some(p);
    }
    if out.response.drag_started() {
        state.engaged = Some(p);
        host.begin_edit(p);
    }
    if let Some(new_norm) = out.change {
        let mut v = from_norm(spec, new_norm);
        if matches!(spec.unit, Unit::Choice(_)) {
            v = v.round();
        }
        host.set(p, v);
    }
    if out.response.drag_stopped() && state.engaged == Some(p) {
        state.engaged = None;
        host.end_edit(p);
    }
    let _ = out.reset;
}

fn handle_dropped_files(ui: &mut Ui, host: &mut dyn PanelHost, state: &mut PanelState) {
    let dropped = ui.ctx().input(|i| i.raw.dropped_files.clone());
    if dropped.is_empty() {
        return;
    }
    for file in dropped {
        if let Some(bytes) = &file.bytes {
            host.link_bytes(&file.name, bytes);
            state.link_attempted = true;
        } else if let Some(path) = &file.path {
            host.link_path(&path.to_string_lossy());
            state.link_attempted = true;
        }
    }
}

fn gate(ui: &mut Ui, host: &mut dyn PanelHost, state: &mut PanelState, panel: Rect) {
    // Clone the painter so the interactive widgets below can borrow `ui` mutably.
    let painter = ui.painter().clone();
    painter.rect_filled(
        panel,
        egui::CornerRadius::ZERO,
        Color32::from_rgba_unmultiplied(3, 3, 6, 214),
    );

    let card = Rect::from_center_size(panel.center(), vec2(600.0, 360.0));
    painter.rect_filled(card, egui::CornerRadius::same(4), PANEL);
    painter.rect_stroke(
        card,
        egui::CornerRadius::same(4),
        Stroke::new(1.2_f32, with_alpha(CYAN, 0.65)),
        egui::StrokeKind::Inside,
    );
    for (corner, dx, dy) in [
        (card.left_top(), 1.0, 1.0),
        (card.right_top(), -1.0, 1.0),
        (card.left_bottom(), 1.0, -1.0),
        (card.right_bottom(), -1.0, -1.0),
    ] {
        let p = pos2(corner.x + dx * 2.0, corner.y + dy * 2.0);
        painter.line_segment([p, pos2(p.x + dx * 26.0, p.y)], Stroke::new(2.0_f32, CYAN));
        painter.line_segment([p, pos2(p.x, p.y + dy * 26.0)], Stroke::new(2.0_f32, CYAN));
    }

    painter.text(
        pos2(card.center().x, card.top() + 26.0),
        Align2::CENTER_TOP,
        "FIRMWARE LINK REQUIRED",
        FontId::monospace(19.0),
        CYAN,
    );
    painter.text(
        pos2(card.center().x, card.top() + 56.0),
        Align2::CENTER_TOP,
        "The oscillators stay silent until a Novation Peak image is linked.",
        FontId::monospace(9.0),
        with_alpha(TEXT, 0.8),
    );
    painter.text(
        pos2(card.center().x, card.top() + 72.0),
        Align2::CENTER_TOP,
        "Nothing leaves this machine: the file is validated locally and used as",
        FontId::monospace(9.0),
        with_alpha(DIM, 0.95),
    );
    painter.text(
        pos2(card.center().x, card.top() + 86.0),
        Align2::CENTER_TOP,
        "the seed for the wavetable bank.",
        FontId::monospace(9.0),
        with_alpha(DIM, 0.95),
    );

    // Drop zone.
    let drop = Rect::from_min_size(
        pos2(card.left() + 40.0, card.top() + 116.0),
        vec2(card.width() - 80.0, 84.0),
    );
    painter.rect_filled(drop, egui::CornerRadius::same(3), PANEL_LO);
    painter.rect_stroke(
        drop,
        egui::CornerRadius::same(3),
        Stroke::new(1.0_f32, with_alpha(EDGE_HI, 0.9)),
        egui::StrokeKind::Inside,
    );
    painter.text(
        drop.center() - vec2(0.0, 8.0),
        Align2::CENTER_CENTER,
        "\u{25bc}  DROP  peak-firmware-*.syx  HERE",
        FontId::monospace(11.0),
        with_alpha(AMBER, 0.9),
    );
    painter.text(
        drop.center() + vec2(0.0, 10.0),
        Align2::CENTER_CENTER,
        "or paste a path below",
        FontId::monospace(8.0),
        DIM,
    );

    // Path field + link button.
    let field = Rect::from_min_size(
        pos2(card.left() + 40.0, card.top() + 214.0),
        vec2(card.width() - 80.0 - 96.0, 28.0),
    );
    ui.scope_builder(egui::UiBuilder::new().max_rect(field), |ui| {
        let edit = egui::TextEdit::singleline(&mut state.path_input)
            .hint_text(" /path/to/peak-firmware-2.2-build2108.syx")
            .font(egui::TextStyle::Monospace)
            .desired_width(field.width());
        ui.put(field, edit);
    });
    let button = Rect::from_min_size(
        pos2(field.right() + 8.0, field.top()),
        vec2(88.0, field.height()),
    );
    let clicked = widgets::neon_button(ui, button, "LINK", CYAN, false).clicked();
    let submitted = ui.input(|i| i.key_pressed(egui::Key::Enter));
    if clicked || (submitted && !state.path_input.trim().is_empty()) {
        let path = state.path_input.trim().to_string();
        if path.is_empty() {
            state.gate_error = Some("empty path".into());
        } else {
            state.link_attempted = true;
            state.gate_error = None;
            host.link_path(&path);
        }
    }

    // Status.
    let status_y = card.top() + 254.0;
    match host.link_status() {
        LinkStatus::Working => {
            let t = ui.input(|i| i.time) as f32;
            let dots = ".".repeat((t * 3.0) as usize % 4);
            painter.text(
                pos2(card.center().x, status_y),
                Align2::CENTER_TOP,
                format!("VALIDATING & SYNTHESISING WAVETABLES{dots}"),
                FontId::monospace(9.5),
                AMBER,
            );
        }
        LinkStatus::Failed(err) => {
            painter.text(
                pos2(card.center().x, status_y),
                Align2::CENTER_TOP,
                format!("REJECTED: {err}"),
                FontId::monospace(9.0),
                RED,
            );
        }
        LinkStatus::Idle => {
            let msg = state.gate_error.clone().unwrap_or_else(|| {
                "accepts Peak firmware and FPGA images, builds 1.2 to 2.2".into()
            });
            painter.text(
                pos2(card.center().x, status_y),
                Align2::CENTER_TOP,
                msg,
                FontId::monospace(8.0),
                with_alpha(DIM, 0.9),
            );
        }
    }

    painter.text(
        pos2(card.center().x, card.bottom() - 30.0),
        Align2::CENTER_TOP,
        "GET ONE: novationmusic.com \u{2192} Peak \u{2192} Components  \u{00b7}  or scripts/fetch-firmware.sh",
        FontId::monospace(7.5),
        with_alpha(DIM, 0.75),
    );
    widgets::scanlines(&painter, card, 4.0, 0.06);
}

/// Draw the panel at a fixed size and return the size it used, so hosts can set
/// their window/bounds from one source of truth.
pub fn preferred_size() -> egui::Vec2 {
    vec2(PANEL_W, PANEL_H)
}

/// Configure fonts and spacing once per context.
pub fn install_style(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    style.visuals.window_fill = BG;
    style.visuals.panel_fill = BG;
    style.visuals.override_text_color = Some(TEXT);
    style.visuals.extreme_bg_color = PANEL_LO;
    style.visuals.selection.bg_fill = with_alpha(CYAN, 0.35);
    style.visuals.widgets.inactive.bg_fill = PANEL_LO;
    style.visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, DIM);
    style.visuals.widgets.hovered.bg_fill = PANEL_HI;
    style.visuals.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, CYAN);
    style.visuals.widgets.active.bg_fill = PANEL_HI;
    style.visuals.widgets.active.fg_stroke = Stroke::new(1.0_f32, CYAN);
    style.spacing.item_spacing = vec2(6.0, 6.0);
    ctx.set_style(style);
}
