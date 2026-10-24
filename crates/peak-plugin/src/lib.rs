//! ReplicantPeak: a firmware-linked wavetable synthesizer.
//!
//! The plugin is deliberately split in three: [`peak_core`] owns validation and
//! DSP, [`peak_ui`] owns the panel, and this crate is the host adapter. The
//! firmware gate is enforced twice — the editor refuses to draw an unlocked
//! panel, and the engine renders silence without a bank.

// vst3-sys' vtable macro emits a trailing semicolon in expression position. The
// lint fires inside the upstream expansion, so it cannot be fixed here;
// allowing it keeps `clippy -D warnings` meaningful for the rest of the crate.
#![allow(semicolon_in_expressions_from_non_local_macros)]

mod params;

use std::num::NonZeroU32;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crossbeam::channel::{bounded, Receiver, Sender};
use nih_plug::prelude::*;
use nih_plug_egui::{create_egui_editor, egui};
use peak_core::{Bank, Engine, Event, LinkedFirmware, Patch, PresetBank, P};
use peak_ui::{FirmwareStamp, LinkStatus, PanelHost, PanelState};

use params::{FirmwareStore, PeakParams};

/// State shared with the editor. The audio thread only ever touches `active`;
/// the rest is GUI/worker territory.
struct Shared {
    status: Mutex<LinkStatus>,
    bank: Mutex<Option<Arc<Bank>>>,
    presets: Mutex<PresetBank>,
    /// Name of the preset last selected, so the display survives a repaint
    /// without being a host parameter.
    preset_name: Mutex<String>,
    preset_index: AtomicUsize,
    active: AtomicUsize,
    emulator: Mutex<Option<peak_emulator::Emulator>>,
    emulator_cycles: AtomicUsize,
}

impl Shared {
    fn new() -> Self {
        Self {
            status: Mutex::new(LinkStatus::Idle),
            bank: Mutex::new(None),
            presets: Mutex::new(PresetBank::new()),
            preset_name: Mutex::new("INIT PATCH".to_string()),
            preset_index: AtomicUsize::new(0),
            active: AtomicUsize::new(0),
            emulator: Mutex::new(None),
            emulator_cycles: AtomicUsize::new(0),
        }
    }

    fn bank_linked(&self) -> bool {
        self.bank.lock().map(|b| b.is_some()).unwrap_or(false)
    }

    fn preset_name(&self) -> String {
        self.preset_name
            .lock()
            .map(|n| n.clone())
            .unwrap_or_else(|_| "INIT PATCH".to_string())
    }

    fn set_preset(&self, index: usize, name: String) {
        self.preset_index.store(index, Ordering::Relaxed);
        if let Ok(mut slot) = self.preset_name.lock() {
            *slot = name;
        }
    }

    fn set_status(&self, status: LinkStatus) {
        if let Ok(mut guard) = self.status.lock() {
            *guard = status;
        }
    }

    fn status(&self) -> LinkStatus {
        self.status
            .lock()
            .map(|g| g.clone())
            .unwrap_or(LinkStatus::Idle)
    }
}

pub struct ReplicantPeak {
    params: Arc<PeakParams>,
    engine: Engine,
    shared: Arc<Shared>,
    link_tx: Sender<LinkedFirmware>,
    link_rx: Receiver<LinkedFirmware>,
    /// Reused every block so `process()` never allocates a patch.
    patch: Patch,
    /// Scratch channel for hosts that hand us mono.
    mono_scratch: Vec<f32>,
    /// Firmware path already handed to the linker, so a saved session relinks
    /// once per path instead of on every editor frame.
    relinked_path: Option<String>,
}

impl Default for ReplicantPeak {
    fn default() -> Self {
        let (link_tx, link_rx) = bounded(4);
        Self {
            params: Arc::new(PeakParams::default()),
            engine: Engine::new(48_000.0),
            shared: Arc::new(Shared::new()),
            link_tx,
            link_rx,
            patch: Patch::default(),
            mono_scratch: Vec::new(),
            relinked_path: None,
        }
    }
}

/// Validate `bytes` (or read `path`) off the audio thread and hand the bank to
/// the engine through the channel.
fn spawn_link(
    tx: Sender<LinkedFirmware>,
    shared: Arc<Shared>,
    store: FirmwareStore,
    path: String,
    bytes: Option<Vec<u8>>,
    clear_stamp_on_failure: bool,
) {
    shared.set_status(LinkStatus::Working);
    std::thread::spawn(move || {
        let data = match bytes {
            Some(bytes) => Ok(bytes),
            None => std::fs::read(&path),
        };
        let data = match data {
            Ok(data) => data,
            Err(error) => {
                let message = format!("{path}: {error}");
                if clear_stamp_on_failure {
                    if let Ok(mut slot) = store.lock() {
                        *slot = None;
                    }
                }
                shared.set_status(LinkStatus::Failed(message));
                return;
            }
        };

        match peak_core::link_firmware(&data, Some(path.clone())) {
            Err(error) => {
                if clear_stamp_on_failure {
                    if let Ok(mut slot) = store.lock() {
                        *slot = None;
                    }
                }
                shared.set_status(LinkStatus::Failed(error.to_string()));
            }
            Ok(linked) => {
                let stamp = FirmwareStamp::from_image(&linked.image, Some(path));
                if let Ok(mut slot) = store.lock() {
                    *slot = Some(stamp);
                }
                if let Ok(mut bank) = shared.bank.lock() {
                    *bank = Some(linked.bank.clone());
                }
                // Initialize the ARM Cortex-M hardware emulator with the binary
                if let Ok(binary) = peak_emulator::FirmwareBinary::from_bytes(&data) {
                    if let Ok(mut emu) = peak_emulator::Emulator::new(binary, false) {
                        let _ = emu.step(10_000);
                        if let Ok(mut slot) = shared.emulator.lock() {
                            *slot = Some(emu);
                        }
                        shared.emulator_cycles.store(10_000, Ordering::Relaxed);
                    }
                }
                shared.set_status(LinkStatus::Idle);
                // If the queue is full the engine keeps the previous bank; the
                // stamp already reflects the new one, so the panel is honest.
                let _ = tx.try_send(linked);
            }
        }
    });
}

/// Translate a host note event into an engine event. Events the engine does not
/// use are dropped here rather than growing the staging buffer.
fn map_note_event(event: NoteEvent<()>) -> Option<Event> {
    match event {
        // MIDI 1.0 says a note-on with velocity 0 is a note-off; some hosts and
        // controllers still send it that way.
        NoteEvent::NoteOn { note, velocity, .. } if velocity <= 0.0 => {
            Some(Event::NoteOff { note })
        }
        NoteEvent::NoteOn { note, velocity, .. } => Some(Event::NoteOn { note, velocity }),
        NoteEvent::NoteOff { note, .. } => Some(Event::NoteOff { note }),
        NoteEvent::Choke { .. } => Some(Event::AllNotesOff),
        NoteEvent::MidiPitchBend { value, .. } => Some(Event::PitchBend {
            normalized: value * 2.0 - 1.0,
        }),
        NoteEvent::MidiCC { cc, .. } if cc == 120 || cc == 123 => Some(Event::AllNotesOff),
        _ => None,
    }
}

/// Editor-frame relink: if the session has a firmware stamp but the engine has
/// no bank and no link is in flight, start one. `spawn_link` switches the status
/// to `Working` before returning, so this runs at most once per attempt.
fn relink_from_state(state: &EditorState) {
    if state.shared.bank_linked() || state.shared.status() != LinkStatus::Idle {
        return;
    }
    let Some(path) = state
        .params
        .firmware
        .lock()
        .ok()
        .and_then(|guard| guard.clone())
        .and_then(|stamp| stamp.path)
    else {
        return;
    };
    spawn_link(
        state.tx.clone(),
        state.shared.clone(),
        state.params.firmware.clone(),
        path,
        None,
        true,
    );
}

impl ReplicantPeak {
    /// Hand the firmware path saved with the session to the linker, at most once
    /// per distinct path. A failed attempt is not retried automatically: the
    /// panel reports the error and the user can point at a file again.
    fn relink_saved(&mut self) {
        if self.engine.has_bank() {
            return;
        }
        let Some(path) = self
            .params
            .firmware
            .lock()
            .ok()
            .and_then(|guard| guard.clone())
            .and_then(|stamp| stamp.path)
        else {
            return;
        };
        if self.relinked_path.as_deref() == Some(path.as_str()) {
            return;
        }
        self.relinked_path = Some(path.clone());
        spawn_link(
            self.link_tx.clone(),
            self.shared.clone(),
            self.params.firmware.clone(),
            path,
            None,
            true,
        );
    }
}

struct EditorHost<'a> {
    params: &'a Arc<PeakParams>,
    shared: &'a Arc<Shared>,
    tx: &'a Sender<LinkedFirmware>,
    setter: &'a ParamSetter<'a>,
}

impl PanelHost for EditorHost<'_> {
    fn value(&self, param: P) -> f32 {
        self.params.float(param).value()
    }

    fn set(&mut self, param: P, value: f32) {
        self.setter.set_parameter(self.params.float(param), value);
    }

    fn begin_edit(&mut self, param: P) {
        self.setter.begin_set_parameter(self.params.float(param));
    }

    fn end_edit(&mut self, param: P) {
        self.setter.end_set_parameter(self.params.float(param));
    }

    fn firmware(&self) -> Option<FirmwareStamp> {
        self.params
            .firmware
            .lock()
            .ok()
            .and_then(|guard| guard.clone())
    }

    fn link_status(&self) -> LinkStatus {
        self.shared.status()
    }

    fn link_path(&mut self, path: &str) {
        spawn_link(
            self.tx.clone(),
            self.shared.clone(),
            self.params.firmware.clone(),
            path.to_string(),
            None,
            false,
        );
    }

    fn link_bytes(&mut self, name: &str, bytes: &[u8]) {
        spawn_link(
            self.tx.clone(),
            self.shared.clone(),
            self.params.firmware.clone(),
            name.to_string(),
            Some(bytes.to_vec()),
            false,
        );
    }

    fn scope_wave(&self) -> Vec<f32> {
        let wave = self.params.float(P::Osc1Shape).value().round() as usize;
        let shape = self.params.float(P::Osc1ShapeAmt).value();
        match self.shared.bank.lock().ok().and_then(|b| b.clone()) {
            Some(bank) => bank.morph_cycle(wave, shape, 128),
            None => Vec::new(),
        }
    }

    fn wavetable(&self, wave: usize, samples: usize) -> Vec<Vec<f32>> {
        match self.shared.bank.lock().ok().and_then(|b| b.clone()) {
            Some(bank) => bank.cycles(wave, samples),
            None => Vec::new(),
        }
    }

    fn active_voices(&self) -> usize {
        self.shared.active.load(Ordering::Relaxed)
    }

    fn max_voices(&self) -> usize {
        self.params.float(P::Polyphony).value().round() as usize
    }

    fn preset_name(&self) -> String {
        self.shared.preset_name()
    }

    fn preset_names(&self) -> Vec<String> {
        self.shared
            .presets
            .lock()
            .map(|bank| bank.names())
            .unwrap_or_default()
    }

    fn preset_index(&self) -> usize {
        self.shared.preset_index.load(Ordering::Relaxed)
    }

    /// Loading a preset is a full parameter gesture: the host sees one edit per
    /// parameter inside a begin/end pair, so undo and automation stay coherent.
    fn select_preset(&mut self, index: usize) {
        let entry = self
            .shared
            .presets
            .lock()
            .ok()
            .and_then(|bank| bank.get(index).cloned());
        let Some(entry) = entry else {
            return;
        };
        for p in P::ALL {
            self.setter.begin_set_parameter(self.params.float(p));
            self.setter
                .set_parameter(self.params.float(p), entry.patch.get(p));
            self.setter.end_set_parameter(self.params.float(p));
        }
        self.shared.set_preset(index, entry.name);
    }

    fn store_preset(&mut self) -> Result<String, String> {
        let mut patch = Patch::default();
        self.params.write_into(&mut patch);
        patch.name = self.shared.preset_name();
        let mut bank = self
            .shared
            .presets
            .lock()
            .map_err(|_| "preset bank is poisoned".to_string())?;
        let index = bank
            .store(&self.shared.preset_name(), &patch)
            .map_err(|error| format!("cannot store preset: {error}"))?;
        let name = bank
            .get(index)
            .map(|e| e.name.clone())
            .unwrap_or_else(|| "USER PATCH".to_string());
        drop(bank);
        self.shared.set_preset(index, name.clone());
        Ok(name)
    }

    fn oled_pixels(&self) -> Option<Vec<u8>> {
        if let Ok(mut emu_opt) = self.shared.emulator.lock() {
            if let Some(emu) = emu_opt.as_mut() {
                // Step instructions periodically to keep firmware emulation running
                let _ = emu.step(2_000);
                self.shared
                    .emulator_cycles
                    .fetch_add(2_000, Ordering::Relaxed);
                let patch_idx = self.preset_index();
                let patch_name = self.preset_name();
                let voices = self.active_voices();
                let cycles = self.shared.emulator_cycles.load(Ordering::Relaxed) as u64;
                if let Ok(mut disp) = emu.context_mut().mmio.display.lock() {
                    disp.render_patch_screen(patch_idx, &patch_name, "SYNTH", cycles, voices);
                    return disp.pixels.lock().ok().map(|p| p.clone());
                }
            }
        }
        None
    }

    fn voice_led_mask(&self) -> u8 {
        let active = self.active_voices().min(8);
        if active == 0 {
            0
        } else {
            (1u8 << active) - 1
        }
    }

    fn emulator_status(&self) -> (bool, u64) {
        let is_running = self
            .shared
            .emulator
            .lock()
            .map(|e| e.is_some())
            .unwrap_or(false);
        (
            is_running,
            self.shared.emulator_cycles.load(Ordering::Relaxed) as u64,
        )
    }

    fn import_syx(&mut self, data: &[u8]) -> Result<String, String> {
        let (name, patch) = peak_core::preset::parse_syx_patch(data)?;
        let mut bank = self
            .shared
            .presets
            .lock()
            .map_err(|_| "preset bank poisoned".to_string())?;
        let index = bank
            .store(&name, &patch)
            .map_err(|e| format!("cannot store .syx preset: {e}"))?;
        drop(bank);
        self.select_preset(index);
        Ok(name)
    }
}

/// Editor-local state. The panel keeps its own scratch state; this mirrors the
/// handful of things the closure needs.
struct EditorState {
    params: Arc<PeakParams>,
    shared: Arc<Shared>,
    tx: Sender<LinkedFirmware>,
    panel: PanelState,
}

impl Plugin for ReplicantPeak {
    const NAME: &'static str = "ReplicantPeak";
    const VENDOR: &'static str = "DanimanX";
    const URL: &'static str = "https://github.com/DanimanX/ReplicantPeak";
    const EMAIL: &'static str = "";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: NonZeroU32::new(2),
        main_output_channels: NonZeroU32::new(2),
        ..AudioIOLayout::const_default()
    }];

    // Without this the CLAP/VST3 wrappers never hand the plugin any note
    // events, so the engine would be silent no matter what the host sends.
    const MIDI_INPUT: MidiConfig = MidiConfig::Basic;

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        context: &mut impl InitContext<Self>,
    ) -> bool {
        self.engine.set_sample_rate(buffer_config.sample_rate);
        self.mono_scratch = vec![0.0; buffer_config.max_buffer_size as usize];
        context.set_latency_samples(0);

        // A saved session knows where the image was. Relink in the background so
        // a reloaded project makes sound without opening the editor. Note that
        // hosts usually restore the state *after* `initialize()`, so the editor
        // path below tries again once the stamp is actually there.
        self.relink_saved();
        true
    }

    fn reset(&mut self) {
        self.engine.reset();
        self.shared.active.store(0, Ordering::Relaxed);
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        // Opening the editor is also a chance to relink: hosts restore the
        // saved state after `initialize()`, so this is often the first moment
        // the stored firmware path is visible.
        self.relink_saved();

        let state = EditorState {
            params: self.params.clone(),
            shared: self.shared.clone(),
            tx: self.link_tx.clone(),
            panel: PanelState::new(),
        };

        create_egui_editor(
            self.params.editor_state.clone(),
            state,
            |ctx, _state| peak_ui::panel::install_style(ctx),
            |ctx, setter, state| {
                // The host may restore a saved session's firmware path at any
                // point; when it does, the engine still needs its bank.
                relink_from_state(state);
                let frame = egui::Frame::NONE.fill(peak_ui::theme::BG);
                egui::CentralPanel::default().frame(frame).show(ctx, |ui| {
                    let mut host = EditorHost {
                        params: &state.params,
                        shared: &state.shared,
                        tx: &state.tx,
                        setter,
                    };
                    peak_ui::panel::show(ui, &mut host, &mut state.panel);
                });
                // The gate pulses and the voice meter moves even when the user
                // is not touching anything.
                ctx.request_repaint_after(Duration::from_millis(33));
            },
        )
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        // A link finished since the last block.
        while let Ok(linked) = self.link_rx.try_recv() {
            self.engine.set_bank(linked.bank);
        }

        // Fixed-size event staging: no allocation on the audio thread.
        let mut events = [Event::AllNotesOff; 128];
        let mut count = 0usize;
        while let Some(event) = context.next_event() {
            if let Some(event) = map_note_event(event) {
                if count < events.len() {
                    events[count] = event;
                    count += 1;
                }
            }
        }

        self.render(buffer.as_slice(), &events[..count]);
        ProcessStatus::Normal
    }
}

impl ReplicantPeak {
    /// Render one block into the host's channel slices.
    ///
    /// Kept separate from `process()` so the render path can be exercised
    /// without a host: the wrappers own `Buffer`, which cannot be constructed
    /// outside nih-plug.
    fn render(&mut self, channels: &mut [&mut [f32]], events: &[Event]) {
        self.params.write_into(&mut self.patch);
        match channels.len() {
            0 => {}
            1 => {
                let frames = channels[0].len();
                if self.mono_scratch.len() >= frames {
                    self.engine.process(
                        &self.patch,
                        events,
                        channels[0],
                        &mut self.mono_scratch[..frames],
                    );
                } else {
                    channels[0].iter_mut().for_each(|s| *s = 0.0);
                }
            }
            _ => {
                let (left, right) = channels.split_at_mut(1);
                self.engine.process(&self.patch, events, left[0], right[0]);
            }
        }

        self.shared
            .active
            .store(self.engine.active_voices(), Ordering::Relaxed);
    }
}

impl ClapPlugin for ReplicantPeak {
    const CLAP_ID: &'static str = "com.danimanx.replicant-peak";
    const CLAP_DESCRIPTION: Option<&'static str> =
        Some("Firmware-linked wavetable synthesizer inspired by the Novation Peak");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::Instrument,
        ClapFeature::Synthesizer,
        ClapFeature::Stereo,
    ];
}

impl Vst3Plugin for ReplicantPeak {
    const VST3_CLASS_ID: [u8; 16] = *b"ReplicantPeak002";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Instrument, Vst3SubCategory::Synth];
}

nih_export_clap!(ReplicantPeak);
nih_export_vst3!(ReplicantPeak);

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() <= (b.abs().max(1.0) * 1e-6)
    }

    /// The host parameters must expose exactly the ranges the engine and the
    /// panel assume. If someone edits one table without the other, this fails.
    #[test]
    fn parameter_ranges_match_core() {
        let params = PeakParams::default();
        for p in P::ALL {
            let spec = p.spec();
            let param = params.float(p);
            let range = param.range();
            match range {
                FloatRange::Linear { min, max } | FloatRange::Skewed { min, max, .. } => {
                    assert!(close(min, spec.min), "{p:?} min {} != {}", min, spec.min);
                    assert!(close(max, spec.max), "{p:?} max {} != {}", max, spec.max);
                }
                other => panic!("{p:?} uses an unexpected range: {other:?}"),
            }
            assert!(
                close(param.value(), spec.default),
                "{p:?} default {} != {}",
                param.value(),
                spec.default
            );
        }
    }

    #[test]
    fn every_param_has_a_unique_host_id() {
        let params = PeakParams::default();
        let map = params.param_map();
        assert_eq!(map.len(), P::COUNT, "param_map size");
        for p in P::ALL {
            assert!(
                map.iter().any(|(id, _, _)| id == p.spec().id),
                "{:?} is missing id {}",
                p,
                p.spec().id
            );
        }
    }

    /// nih-plug exposes no setter from outside the GUI context, so the check is
    /// that a host snapshot carries the same defaults the engine and panel use.
    #[test]
    fn host_snapshot_matches_core_defaults() {
        let params = PeakParams::default();
        let snapshot = params.to_patch();
        let core_default = Patch::default();
        for p in P::ALL {
            assert!(
                close(snapshot.get(p), core_default.get(p)),
                "{p:?}: host {} != core {}",
                snapshot.get(p),
                core_default.get(p)
            );
        }
    }

    #[test]
    fn parameters_report_names_and_units() {
        let params = PeakParams::default();
        for p in P::ALL {
            assert!(!params.float(p).name().is_empty(), "{p:?} has no name");
        }
        assert!(params.float(P::Cutoff).name().contains("CUTOFF"));
        assert_eq!(params.float(P::Volume).unit(), " dB");
        // Two LEVEL knobs in different sections must not share a display name.
        assert_ne!(
            params.float(P::Osc1Level).name(),
            params.float(P::Osc2Level).name()
        );
    }

    /// Choice parameters must advance in whole steps, not fractions, when a
    /// host nudges them.
    #[test]
    fn choice_parameters_step_by_one() {
        let params = PeakParams::default();
        for p in [P::FilterType, P::Osc1Shape, P::Lfo1Shape, P::Lfo1Dest] {
            let param = params.float(p);
            assert_eq!(param.next_step(0.0, false), 1.0, "{p:?} next step");
            assert_eq!(param.next_step(1.0, false), 2.0, "{p:?} next step");
            assert_eq!(param.previous_step(2.0, false), 1.0, "{p:?} prev step");
        }
        // Continuous controls are not quantised.
        let cutoff = params.float(P::Cutoff);
        assert!(cutoff.next_step(100.0, false) - 100.0 < 50.0);
    }

    /// A fresh instance has no firmware: it must report no bank and stay silent.
    #[test]
    fn starts_locked_and_silent() {
        let plugin = ReplicantPeak::default();
        assert!(!plugin.engine.has_bank());
        assert!(plugin.params.firmware.lock().unwrap().is_none());
    }

    /// The wrappers drop every note event unless the plugin declares MIDI input.
    /// This is what made the first release silent in a DAW.
    #[test]
    fn declares_midi_input() {
        assert!(
            ReplicantPeak::MIDI_INPUT >= MidiConfig::Basic,
            "the host wrappers only deliver note events when MIDI_INPUT is set"
        );
    }

    fn linked() -> ReplicantPeak {
        // The same synthetic image shape the core tests use: framing, Novation's
        // manufacturer id and the firmware product byte.
        let mut bytes = Vec::new();
        for i in 0..128u32 {
            bytes.extend_from_slice(&[
                0xF0,
                0x00,
                0x20,
                0x29,
                0x00,
                0x71,
                0x00,
                0x7E,
                (i % 128) as u8,
                0xF7,
            ]);
        }
        let linked = peak_core::link_firmware(&bytes, None).expect("synthetic image parses");
        let mut plugin = ReplicantPeak::default();
        plugin.engine.set_bank(linked.bank);
        plugin
    }

    fn rms(samples: &[f32]) -> f32 {
        let sum: f32 = samples.iter().map(|s| s * s).sum();
        (sum / samples.len().max(1) as f32).sqrt()
    }

    fn render(plugin: &mut ReplicantPeak, events: &[Event], frames: usize) -> (Vec<f32>, Vec<f32>) {
        let mut left = vec![0.0; frames];
        let mut right = vec![0.0; frames];
        {
            let mut channels: [&mut [f32]; 2] = [&mut left, &mut right];
            plugin.render(&mut channels, events);
        }
        (left, right)
    }

    /// The end-to-end path a host drives: link, note on, render.
    #[test]
    fn a_note_on_renders_audio_once_linked() {
        let mut plugin = linked();
        let events = [Event::NoteOn {
            note: 60,
            velocity: 0.9,
        }];
        let (left, _right) = render(&mut plugin, &events, 2048);
        assert!(
            rms(&left) > 1e-4,
            "linked plugin rendered silence: rms {}",
            rms(&left)
        );
        assert!(left.iter().all(|s| s.is_finite()), "non-finite samples");
        assert!(plugin.engine.active_voices() >= 1);
    }

    /// And the gate still holds: no bank, no sound, however many notes arrive.
    #[test]
    fn an_unlinked_plugin_stays_silent() {
        let mut plugin = ReplicantPeak::default();
        let events = [Event::NoteOn {
            note: 60,
            velocity: 1.0,
        }];
        let (left, right) = render(&mut plugin, &events, 512);
        assert!(left.iter().all(|s| *s == 0.0));
        assert!(right.iter().all(|s| *s == 0.0));
        assert_eq!(plugin.engine.active_voices(), 0);
    }

    /// A note-off must release the voice instead of leaving it ringing. The
    /// reverb and delay tails are still audible for a moment, so the check is
    /// that the level collapses by orders of magnitude and no voice stays busy.
    #[test]
    fn note_off_releases_the_voice() {
        let mut plugin = linked();
        let (held, _) = render(
            &mut plugin,
            &[Event::NoteOn {
                note: 60,
                velocity: 0.8,
            }],
            4_800,
        );
        let sustained = rms(&held[held.len() / 2..]);

        render(&mut plugin, &[Event::NoteOff { note: 60 }], 512);
        let (tail, _) = render(&mut plugin, &[], 96_000);
        let last = rms(&tail[tail.len() - 1024..]);

        assert!(
            last < sustained * 0.02,
            "voice still ringing after note-off: {last} vs {sustained}"
        );
        assert_eq!(plugin.engine.active_voices(), 0, "voice left active");
    }

    /// The plugin's preset bank is the factory bank plus user presets; the panel
    /// reads names and the current slot from exactly these.
    #[test]
    fn preset_bank_is_available_to_the_editor() {
        let shared = Shared::new();
        let names = shared.presets.lock().unwrap().names();
        assert!(names.len() >= 8, "expected a factory bank");
        assert_eq!(names[0], "INIT PATCH");
        assert_eq!(shared.preset_name(), "INIT PATCH");
        assert_eq!(shared.preset_index.load(Ordering::Relaxed), 0);

        shared.set_preset(3, names[3].clone());
        assert_eq!(shared.preset_name(), names[3]);
        assert_eq!(shared.preset_index.load(Ordering::Relaxed), 3);
    }

    #[test]
    fn zero_velocity_note_on_releases() {
        let event = NoteEvent::NoteOn {
            timing: 0,
            voice_id: None,
            channel: 0,
            note: 64,
            velocity: 0.0,
        };
        assert!(matches!(
            map_note_event(event),
            Some(Event::NoteOff { note: 64 })
        ));
    }

    #[test]
    fn mapped_events_match_the_engine() {
        use nih_plug::prelude::NoteEvent as N;
        let cases = [
            N::NoteOn {
                timing: 0,
                voice_id: None,
                channel: 0,
                note: 60,
                velocity: 0.5,
            },
            N::NoteOff {
                timing: 0,
                voice_id: None,
                channel: 0,
                note: 60,
                velocity: 0.0,
            },
        ];
        assert!(matches!(
            map_note_event(cases[0]),
            Some(Event::NoteOn { note: 60, .. })
        ));
        assert!(matches!(
            map_note_event(cases[1]),
            Some(Event::NoteOff { note: 60 })
        ));
    }
}
