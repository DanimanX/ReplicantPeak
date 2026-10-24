//! ReplicantPeak's front panel, shared by the CLAP/VST3 editor and the
//! standalone preview/screenshot renderer.
//!
//! `panel::show` paints the whole instrument into a `Ui`; the host trait in
//! [`host`] is the only thing the panel knows about the outside world.

pub mod host;
pub mod panel;
pub mod theme;
pub mod widgets;

pub use host::{FirmwareStamp, LinkStatus, PanelHost, PanelState};
pub use panel::{preferred_size, show};
pub use theme::{PANEL_H, PANEL_W};
