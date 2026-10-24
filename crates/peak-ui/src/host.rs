//! The interface between the panel and whatever hosts it: the plugin wrapper or
//! the standalone panel renderer.
//!
//! The panel never touches the filesystem or the audio engine directly; it asks
//! the host for values and hands back edits.

use peak_core::firmware::FwKind;
use peak_core::P;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Accent {
    Osc,
    Filter,
    Env,
    Lfo,
    Fx,
    Voice,
}

/// What the plugin knows about the linked firmware. Serialised with the
/// session so reopening a project can relink without asking again.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FirmwareStamp {
    pub version_label: String,
    pub sha_short: String,
    pub kind: FwKind,
    pub verified: bool,
    pub messages: usize,
    pub bytes: usize,
    /// Last known location, used to relink silently when it still resolves.
    pub path: Option<String>,
}

impl FirmwareStamp {
    pub fn from_image(image: &peak_core::FirmwareImage, path: Option<String>) -> Self {
        Self {
            version_label: image.version_label(),
            sha_short: image.sha_short(),
            kind: image.kind,
            verified: image.known.is_some(),
            messages: image.messages,
            bytes: image.bytes,
            path,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LinkStatus {
    Idle,
    Working,
    Failed(String),
}

/// A host-side panel model. Implemented by the plugin editor and by the
/// standalone preview.
pub trait PanelHost {
    fn value(&self, param: P) -> f32;
    fn set(&mut self, param: P, value: f32);
    /// Wrap a group of edits in host automation begin/end calls.
    fn begin_edit(&mut self, _param: P) {}
    fn end_edit(&mut self, _param: P) {}

    /// A copy of the stamp: the plugin keeps it behind a mutex, so a borrow
    /// cannot outlive the call.
    fn firmware(&self) -> Option<FirmwareStamp>;
    fn link_status(&self) -> LinkStatus;

    /// Start validating `path` and building the bank. Must not block.
    fn link_path(&mut self, path: &str);
    /// Same, for a file dropped onto the panel.
    fn link_bytes(&mut self, name: &str, bytes: &[u8]);

    /// One cycle of the current oscillator shape, for the scope.
    fn scope_wave(&self) -> Vec<f32>;
    fn active_voices(&self) -> usize;
    fn max_voices(&self) -> usize;
    fn preset_name(&self) -> String;
    fn set_preset_name(&mut self, _name: String) {}

    /// True when the host has an engine that can actually make sound; the
    /// preview renders the same panel but reports silence.
    fn is_live(&self) -> bool {
        true
    }
}

/// UI-only state: text field contents, which knob is being dragged, drag & drop
/// bookkeeping.
#[derive(Default)]
pub struct PanelState {
    pub path_input: String,
    pub engaged: Option<P>,
    pub drag_start: Option<(P, f32)>,
    pub hovered: Option<P>,
    pub gate_error: Option<String>,
    pub link_attempted: bool,
    pub show_help: bool,
}

impl PanelState {
    pub fn new() -> Self {
        Self::default()
    }
}
