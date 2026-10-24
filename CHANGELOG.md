# Changelog

Format based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/);
versioning follows [SemVer](https://semver.org/).

## [1.0.0] — 2026-10-23

First release.

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
