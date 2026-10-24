# Changelog

Format based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/);
versioning follows [SemVer](https://semver.org/).

## [1.0.0] — 2026-10-23

First release. The 1.0.0 artifacts were rebuilt after the fixes below, so the
tag and the release assets carry them.

### Fixed

- **The plugin was silent in every DAW.** `MIDI_INPUT` was left at its default
  `MidiConfig::None`, so the CLAP/VST3 wrappers never delivered note events and
  `process()` always saw an empty event queue. It is now `MidiConfig::Basic`,
  and `cargo test` includes a probe that loads the built library through CLAP
  and fails if the plugin declares no note input.
- A note-on with velocity 0 is now treated as a note-off.
- Opening a saved session could leave the engine without its wavetable bank:
  hosts restore state *after* `initialize()`, and the old one-shot flag made
  sure the relink was never retried. The firmware path is now relinked once per
  distinct path, from `initialize()` and again from the editor.
- Panel screenshot script: absolute output paths were being prefixed with the
  repository root.

### Added

- Presets: a 12-preset factory bank, a `<`/`>` browser with `STORE` in the
  display, user presets as JSON in the platform data directory, and host-side
  loading that goes through automation begin/end for every parameter.
- `crates/peak-plugin/tests/clap_probe.rs`: loads the built cdylib through
  CLAP and checks note/audio ports.

### Added

- `peak-core`: 8-voice engine (up to 16) with three wavetable oscillators,
  noise, ring modulation, a state-variable filter with its own envelope, two
  LFOs and serial FX (distortion, chorus, delay, reverb). `process()` allocates
  no memory and never blocks.
- `peak-core`: mip-mapped wavetable bank with 8 factory tables and 4 user tables
  derived from the SHA-256 of the linked firmware.
- `peak-core`: strict `.syx` validation for the Peak (framing, manufacturer id,
  product byte) against a table of 10 known builds.
- `peak-core`: single 47-parameter table with logarithmic tapers, formatting and
  conversion to and from normalised space.
- `peak-core`: `preset` module with the factory bank, user preset storage and a
  corrupt-file-tolerant loader.
- `peak-ui`: complete Peak-style panel, with a firmware gate that accepts
  drag-and-drop, a typed path or the link previously saved with the session.
- `peak-plugin`: NIH-plug adapter exporting VST3 and CLAP, with automatable
  parameters and persisted firmware link.
- `peak-panel`: binary that renders the real panel without an audio device.
- `xtask`: plugin bundling through NIH-plug's standard bundler.
- `scripts/fetch-firmware.sh`: downloads the firmware 2.2/build 2108 +
  FPGA 2.1/build 469 pair from the official channel and verifies its SHA-256.
- `scripts/screenshot.sh`: deterministic panel capture, under Xvfb when there is
  no display.
- `scripts/package-bundles.py`: packages the built bundles into one archive per
  plugin format.
- CI: GitHub Actions builds and tests on Linux, Windows and macOS, then attaches
  the per-format archives to the release for the pushed tag.

### Notes

- Audio Unit is not produced: NIH-plug implements only the VST3 and CLAP
  wrappers, so there is no AU export to bundle.
- The Peak firmware is not distributed with the project: it is Focusrite/
  Novation property, and every checkout downloads its own copy.
- Tested on Linux; the panel uses egui/eframe and needs X11 or Wayland.
