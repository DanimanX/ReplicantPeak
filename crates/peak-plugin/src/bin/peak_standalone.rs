//! Standalone desktop application for ReplicantPeak.
//! Connects directly to soundcard (ALSA/JACK/PipeWire/PulseAudio/CoreAudio/WASAPI)
//! and physical/virtual MIDI controllers without needing a DAW.

use nih_plug::prelude::*;
use peak_plugin::ReplicantPeak;

fn main() {
    nih_export_standalone::<ReplicantPeak>();
}
