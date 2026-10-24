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
use peak_core::{Bank, Engine, Event, LinkedFirmware, Patch, P};
use peak_ui::{FirmwareStamp, LinkStatus, PanelHost, PanelState};

use params::{FirmwareStore, PeakParams};

/// State shared with the editor. The audio thread only ever touches `active`;
/// the rest is GUI/worker territory.
struct Shared {
    status: Mutex<LinkStatus>,
    bank: Mutex<Option<Arc<Bank>>>,
    active: AtomicUsize,
}

impl Shared {
    fn new() -> Self {
        Self {
            status: Mutex::new(LinkStatus::Idle),
            bank: Mutex::new(None),
            active: AtomicUsize::new(0),
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
    relink_attempted: bool,
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
            relink_attempted: false,
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
                shared.set_status(LinkStatus::Idle);
                // If the queue is full the engine keeps the previous bank; the
                // stamp already reflects the new one, so the panel is honest.
                let _ = tx.try_send(linked);
            }
        }
    });
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
        let shape = self.params.float(P::Osc1Shape).value();
        let samples = 128;
        match self.shared.bank.lock().ok().and_then(|b| b.clone()) {
            Some(bank) => {
                let a = (shape.floor() as usize).min(peak_core::wavetable::TABLES - 1);
                let b = (a + 1).min(peak_core::wavetable::TABLES - 1);
                let morph = shape - a as f32;
                (0..samples)
                    .map(|i| bank.sample_morph(a, b, morph, 0, i as f32 / samples as f32))
                    .collect()
            }
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
        "INIT PATCH".to_string()
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
        // a reloaded project makes sound without opening the editor.
        if !self.relink_attempted {
            self.relink_attempted = true;
            let stored = self
                .params
                .firmware
                .lock()
                .ok()
                .and_then(|guard| guard.clone());
            if let Some(stamp) = stored {
                if let Some(path) = stamp.path {
                    spawn_link(
                        self.link_tx.clone(),
                        self.shared.clone(),
                        self.params.firmware.clone(),
                        path,
                        None,
                        true,
                    );
                } else if let Ok(mut slot) = self.params.firmware.lock() {
                    // Nothing to relink from: the gate must ask again.
                    *slot = None;
                }
            }
        }
        true
    }

    fn reset(&mut self) {
        self.engine.reset();
        self.shared.active.store(0, Ordering::Relaxed);
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        // Opening the editor is also a chance to relink if the engine lost its
        // bank (for example after a sample-rate change in some hosts).
        if !self.engine.has_bank() && !self.relink_attempted {
            self.relink_attempted = true;
            if let Some(stamp) = self.params.firmware.lock().ok().and_then(|g| g.clone()) {
                if let Some(path) = stamp.path {
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
        }

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
            let mapped = match event {
                NoteEvent::NoteOn { note, velocity, .. } => Some(Event::NoteOn { note, velocity }),
                NoteEvent::NoteOff { note, .. } => Some(Event::NoteOff { note }),
                NoteEvent::Choke { .. } => Some(Event::AllNotesOff),
                NoteEvent::MidiPitchBend { value, .. } => Some(Event::PitchBend {
                    normalized: value * 2.0 - 1.0,
                }),
                NoteEvent::MidiCC { cc, .. } if cc == 120 || cc == 123 => Some(Event::AllNotesOff),
                _ => None,
            };
            if let Some(event) = mapped {
                if count < events.len() {
                    events[count] = event;
                    count += 1;
                }
            }
        }

        self.params.write_into(&mut self.patch);
        let channels = buffer.as_slice();
        match channels.len() {
            0 => {}
            1 => {
                let frames = channels[0].len();
                if self.mono_scratch.len() >= frames {
                    let (mono, scratch) = (&mut channels[0], &mut self.mono_scratch[..frames]);
                    self.engine
                        .process(&self.patch, &events[..count], mono, scratch);
                } else {
                    channels[0].iter_mut().for_each(|s| *s = 0.0);
                }
            }
            _ => {
                let (left, right) = channels.split_at_mut(1);
                self.engine
                    .process(&self.patch, &events[..count], left[0], right[0]);
            }
        }

        self.shared
            .active
            .store(self.engine.active_voices(), Ordering::Relaxed);
        ProcessStatus::Normal
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
}
