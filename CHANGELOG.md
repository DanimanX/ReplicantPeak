# Changelog

Format based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/);
versioning follows [SemVer](https://semver.org/).

## [1.0.1] — 2026-10-23

### Added

- **Per-oscillator cyberpunk wavetable scopes.** Each oscillator column now has
  a five-column waterfall rendered as GPU-rasterised egui meshes, with a live
  interpolated waveform, neon gradients, a phase marker and a column indicator.
  Drag the scope to sweep SHAPE; click the arrows, drag the wave strip or scroll
  to select the table.
- **Peak-style five-column wavetables.** Every table is now a bank of five
  band-limited frames, and the new bipolar SHAPE parameters interpolate across
  them. Mip selection and anti-aliasing are unchanged.
- **Peak-faithful voice controls:** AHDSR envelopes with HOLD, per-voice
  free-running LFOs with FADE and sample-and-hold, Osc 3 audio-rate filter
  modulation, FM O1>O2 and O2>O3, oscillator Drift/Diverge, and a bandwidth-
  limited noise source.
- **Six filter modes:** LP 12, LP 24, BP 12, HP 24, BP 24 and HP 12. The first
  four keep their v1.0.0 indices so existing presets load unchanged.
- **Seven new factory presets:** `NEON SAW`, `FM GLASS`, `PULSE WIDTH`,
  `BANDPASS SCREAM`, `S&H VOID`, `HOLD ORGAN`, `DRIFT KEYS`.
- `docs/PEAK_FIDELITY.md`: architecture comparison, sources and an explicit
  list of what is still not modelled.
- `docs/FREE_PATCHES.md`: researched free Peak banks and their licence terms.

### Changed

- Oscillator `FINE` is now ±100 cents, matching the hardware. `COARSE` keeps its
  existing ±24 semitone range, which is the equivalent of Peak's Range + Coarse
  pair without adding a third oscillator control.
- Oscillator `SHAPE` is now a bipolar waveform-shape amount; the old wave
  selector keeps its parameter id and is labelled `WAVE` in the panel.
- The panel is 60 px taller (1180 × 820) to make room for the three scopes.
- Ambient motion is on by default at a subtle level: Drift 6%, Diverge 4%, and
  a very narrow per-voice pan.
- Parameter count is 60. New parameters are appended to the table, so v1.0.0
  JSON presets still map their first 47 values correctly.

### Fixed

- The cutoff smoother is advanced once per sample instead of once per voice,
  which made its time constant depend on polyphony.
- Note-on now randomises oscillator phase, so stacked voices no longer start
  phase-coherent.
- The previous `SHAPE` morph indexed across tables rather than inside one
  table; it now follows Peak's five-column model.

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
