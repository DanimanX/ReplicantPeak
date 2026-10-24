//! Standalone panel host.
//!
//! This runs the exact panel the plugin's editor runs, without an audio device,
//! so the README screenshot is a render of the real UI rather than a mockup.
//!
//! * `peak-panel` — open the panel.
//! * `PEAK_PANEL_SHOT=assets/panel.png peak-panel` — render once and exit.
//! * `PEAK_FIRMWARE=/path/to/peak-firmware.syx` — link an image on startup
//!   (otherwise the gate is shown, which is what a first run looks like).
//! * `PEAK_PANEL_PRESET=3` — start on the fourth preset of the bank.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crossbeam::channel::{bounded, Receiver, Sender};
use eframe::egui;
use peak_core::{Bank, LinkedFirmware, Patch, PresetBank, P};
use peak_ui::{FirmwareStamp, LinkStatus, PanelHost, PanelState};

fn main() {
    let shot = std::env::var("PEAK_PANEL_SHOT").ok().map(PathBuf::from);
    let delay_ms: u64 = std::env::var("PEAK_PANEL_SHOT_DELAY_MS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(900);

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([peak_ui::PANEL_W, peak_ui::PANEL_H])
            .with_title("ReplicantPeak"),
        ..Default::default()
    };

    if let Err(error) = eframe::run_native(
        "peak-panel",
        options,
        Box::new(move |cc| {
            peak_ui::panel::install_style(&cc.egui_ctx);
            Ok(Box::new(Preview::new(shot, delay_ms)))
        }),
    ) {
        eprintln!("peak-panel: {error}");
        std::process::exit(1);
    }
}

struct Preview {
    patch: Patch,
    panel: PanelState,
    firmware: Option<FirmwareStamp>,
    status: LinkStatus,
    bank: Option<Arc<Bank>>,
    tx: Sender<Result<LinkedFirmware, String>>,
    rx: Receiver<Result<LinkedFirmware, String>>,
    shot: Option<PathBuf>,
    delay: Duration,
    started: Instant,
    shot_requested: bool,
    presets: PresetBank,
    preset_index: usize,
}

impl Preview {
    fn new(shot: Option<PathBuf>, delay_ms: u64) -> Self {
        let (tx, rx) = bounded(4);
        let mut preview = Self {
            patch: Patch::default(),
            panel: PanelState::new(),
            firmware: None,
            status: LinkStatus::Idle,
            bank: None,
            tx,
            rx,
            shot,
            delay: Duration::from_millis(delay_ms),
            started: Instant::now(),
            shot_requested: false,
            presets: PresetBank::new(),
            preset_index: 0,
        };

        if let Some(index) = std::env::var("PEAK_PANEL_PRESET")
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
        {
            preview.select_preset(index);
        }

        if let Ok(path) = std::env::var("PEAK_FIRMWARE") {
            preview.link_path(&path);
        } else {
            let default = PathBuf::from("../firmware/peak-firmware-2.2-build2108.syx");
            if default.exists() {
                preview.link_path(&default.to_string_lossy());
            }
        }
        preview
    }

    fn poll_link(&mut self) {
        while let Ok(result) = self.rx.try_recv() {
            match result {
                Ok(linked) => {
                    self.firmware = Some(FirmwareStamp::from_image(
                        &linked.image,
                        linked.path.clone(),
                    ));
                    self.bank = Some(linked.bank);
                    self.status = LinkStatus::Idle;
                }
                Err(error) => self.status = LinkStatus::Failed(error),
            }
        }
    }

    fn maybe_screenshot(&mut self, ctx: &egui::Context) {
        let Some(path) = self.shot.clone() else {
            return;
        };
        if self.shot_requested || self.started.elapsed() < self.delay {
            return;
        }
        self.shot_requested = true;
        ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
        // The image arrives as an event on a later frame.
        let _ = path;
    }
}

impl PanelHost for Preview {
    fn value(&self, param: P) -> f32 {
        self.patch.get(param)
    }

    fn set(&mut self, param: P, value: f32) {
        self.patch.set(param, value);
    }

    fn firmware(&self) -> Option<FirmwareStamp> {
        self.firmware.clone()
    }

    fn link_status(&self) -> LinkStatus {
        self.status.clone()
    }

    fn link_path(&mut self, path: &str) {
        let tx = self.tx.clone();
        self.status = LinkStatus::Working;
        let path = path.to_string();
        std::thread::spawn(move || {
            let result = match std::fs::read(&path) {
                Err(error) => Err(format!("{path}: {error}")),
                Ok(bytes) => peak_core::link_firmware(&bytes, Some(path.clone()))
                    .map_err(|error| error.to_string()),
            };
            let _ = tx.try_send(result);
        });
    }

    fn link_bytes(&mut self, name: &str, bytes: &[u8]) {
        let tx = self.tx.clone();
        self.status = LinkStatus::Working;
        let name = name.to_string();
        let bytes = bytes.to_vec();
        std::thread::spawn(move || {
            let result =
                peak_core::link_firmware(&bytes, Some(name)).map_err(|error| error.to_string());
            let _ = tx.try_send(result);
        });
    }

    fn scope_wave(&self) -> Vec<f32> {
        let Some(bank) = &self.bank else {
            return Vec::new();
        };
        let wave = self.patch.get(P::Osc1Shape).round() as usize;
        let shape = self.patch.get(P::Osc1ShapeAmt);
        bank.morph_cycle(wave, shape, 128)
    }

    fn wavetable(&self, wave: usize, samples: usize) -> Vec<Vec<f32>> {
        match &self.bank {
            Some(bank) => bank.cycles(wave, samples),
            None => Vec::new(),
        }
    }

    fn active_voices(&self) -> usize {
        // No audio device here: the panel is a preview, not a synth instance.
        0
    }

    fn max_voices(&self) -> usize {
        self.patch.get(P::Polyphony).round() as usize
    }

    fn preset_name(&self) -> String {
        self.patch.name.clone()
    }

    fn set_preset_name(&mut self, name: String) {
        self.patch.name = name;
    }

    fn preset_names(&self) -> Vec<String> {
        self.presets.names()
    }

    fn preset_index(&self) -> usize {
        self.preset_index
    }

    fn select_preset(&mut self, index: usize) {
        let Some(entry) = self.presets.get(index) else {
            return;
        };
        let patch = entry.patch.clone();
        let name = entry.name.clone();
        self.patch = patch;
        self.patch.name = name;
        self.preset_index = index;
    }

    fn store_preset(&mut self) -> Result<String, String> {
        let index = self
            .presets
            .store(&self.patch.name.clone(), &self.patch)
            .map_err(|error| error.to_string())?;
        let name = self
            .presets
            .get(index)
            .map(|entry| entry.name.clone())
            .unwrap_or_else(|| self.patch.name.clone());
        self.patch.name = name.clone();
        self.preset_index = index;
        Ok(name)
    }

    fn is_live(&self) -> bool {
        false
    }
}

impl eframe::App for Preview {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_link();

        let frame = egui::Frame::NONE.fill(peak_ui::theme::BG);
        egui::CentralPanel::default().frame(frame).show(ctx, |ui| {
            // The panel needs `self` as the host and its own scratch state at
            // the same time, so the state is swapped out for the frame.
            let mut panel = std::mem::take(&mut self.panel);
            peak_ui::panel::show(ui, self, &mut panel);
            self.panel = panel;
        });

        self.maybe_screenshot(ctx);

        // Save the screenshot the frame after it was requested.
        let shots: Vec<_> = ctx.input(|input| {
            input
                .events
                .iter()
                .filter_map(|event| match event {
                    egui::Event::Screenshot { image, .. } => Some(image.clone()),
                    _ => None,
                })
                .collect()
        });
        for image in shots {
            let (width, height) = (image.width() as u32, image.height() as u32);
            let mut rgba = Vec::with_capacity((width * height * 4) as usize);
            for pixel in image.pixels.iter() {
                rgba.extend_from_slice(&[pixel.r(), pixel.g(), pixel.b(), pixel.a()]);
            }
            if let Some(path) = &self.shot {
                if let Some(parent) = path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                match image::save_buffer(path, &rgba, width, height, image::ColorType::Rgba8) {
                    Ok(()) => eprintln!("saved {} ({width}x{height})", path.display()),
                    Err(error) => eprintln!("could not save {}: {error}", path.display()),
                }
            }
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }

        if self.status == LinkStatus::Working || self.shot.is_some() {
            ctx.request_repaint_after(Duration::from_millis(16));
        } else {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The preview host is the same code path the plugin editor uses for
    /// presets, so exercising it here covers the browser without a DAW.
    fn preview() -> Preview {
        Preview::new(None, 0)
    }

    #[test]
    fn browser_starts_on_the_init_patch() {
        let preview = preview();
        let names = preview.preset_names();
        assert_eq!(names.len(), PresetBank::new().len());
        assert_eq!(preview.preset_index(), 0);
        assert_eq!(preview.preset_name(), "INIT PATCH");
    }

    #[test]
    fn selecting_a_preset_loads_its_parameters_and_name() {
        let mut preview = preview();
        let before = preview.value(P::Cutoff);
        preview.select_preset(2);
        assert_eq!(preview.preset_index(), 2);
        assert_ne!(preview.preset_name(), "INIT PATCH");
        assert_ne!(preview.value(P::Cutoff), before, "cutoff did not move");
        assert_eq!(preview.preset_name(), preview.preset_names()[2]);
    }

    #[test]
    fn out_of_range_selection_is_ignored() {
        let mut preview = preview();
        preview.select_preset(usize::MAX);
        assert_eq!(preview.preset_index(), 0);
        assert_eq!(preview.preset_name(), "INIT PATCH");
    }

    #[test]
    fn storing_writes_a_user_preset_into_the_bank() {
        let mut preview = preview();
        let dir = std::env::temp_dir().join(format!("replicant-panel-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        preview.presets = PresetBank::with_dir(Some(dir.clone()));
        preview.patch.name = "PANEL TAKE".to_string();
        preview.set(P::Reso, 0.44);

        let name = preview.store_preset().expect("store");
        assert_eq!(name, "PANEL TAKE");
        assert_eq!(preview.preset_index(), preview.presets.len() - 1);
        assert!(dir.join("panel_take.json").is_file());

        // And it survives a reload of the bank.
        let reloaded = PresetBank::with_dir(Some(dir.clone()));
        let entry = reloaded.get(reloaded.len() - 1).expect("entry");
        assert_eq!(entry.name, "PANEL TAKE");
        assert!((entry.patch.get(P::Reso) - 0.44).abs() < 1e-6);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
