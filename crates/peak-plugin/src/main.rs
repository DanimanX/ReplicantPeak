//! Standalone plugin host.
//!
//! Runs ReplicantPeak directly as a desktop synthesizer application connected
//! to the system's audio output (ALSA/JACK/PulseAudio/PipeWire on Linux, CoreAudio on macOS,
//! WASAPI on Windows) and physical/virtual MIDI inputs (via midir), without needing a DAW.

use nih_plug::prelude::*;
use peak_plugin::ReplicantPeak;

fn main() {
    nih_export_standalone::<ReplicantPeak>();
}
