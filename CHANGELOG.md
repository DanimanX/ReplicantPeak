# Changelog

Format based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/);
versioning follows [SemVer](https://semver.org/).

## [1.0.8] — 2026-10-24

### Added

- **Full Standalone Synthesizer Application (`peak-standalone` / `ReplicantPeak`):**
  - Included a first-class standalone desktop synthesizer binary (`crates/peak-plugin/src/bin/peak_standalone.rs` and `crates/peak-plugin/src/main.rs`).
  - Powered by NIH-plug's native standalone engine with `cpal` and `midir`, connecting directly to physical/virtual MIDI keyboards, USB controllers, and system audio interfaces (ALSA, JACK, PipeWire, PulseAudio on Linux; CoreAudio on macOS; WASAPI on Windows) without requiring a DAW.
  - Bundling and packaging support: `cargo xtask bundle peak-plugin` now generates `target/bundled/ReplicantPeak` standalone application alongside VST3 and CLAP bundles.
  - Release packaging (`scripts/package-bundles.py`) now packages `ReplicantPeak-<version>-<os>-standalone.zip` release archives.
  - Configurable command-line options (`--help`): backend selection (`auto`, `alsa`, `jack`, `dummy`), sample rate, buffer period size, input/output audio devices, MIDI input selection, and JACK port routing.
  - CI & Release workflows updated with `libasound2-dev` and `libjack-jackd2-dev` dependencies.

## [1.0.7] — 2026-10-24

### Fixed & Added

- **Persistent Firmware Path Across Instances (`firmware_path.txt`):**
  - Resolved issue where newly inserted plugin instances or standalone panel sessions always prompted with the "FIRMWARE LINK REQUIRED" gate upon loading.
  - Added user configuration storage (`peak_core::save_last_firmware_path`, `peak_core::load_last_firmware_path`, `peak_core::config_dir`) saving the canonical firmware location to `~/.config/ReplicantPeak/firmware_path.txt` (Linux/XDG), `%APPDATA%\ReplicantPeak\firmware_path.txt` (Windows), or `~/Library/Application Support/ReplicantPeak/firmware_path.txt` (macOS).
  - Background auto-relink on instance initialization: `ReplicantPeak::initialize()`, `ReplicantPeak::editor()`, and `relink_from_state` automatically detect and link the remembered firmware file, unlocking the synthesis engine seamlessly without user intervention.
  - Preserved drag-and-drop file paths: UI drag-and-drop handler now extracts and links the actual file path on disk rather than stripping it down to raw in-memory bytes.
  - Added in-memory firmware bytes fallback caching (`cached_peak_firmware.syx`) for sandboxed drag-and-drop environments.
  - Standalone preview panel (`peak-panel`) now checks and restores the remembered firmware path on launch.
  - Added unit test `relinks_automatically_from_persistent_path` verifying automated background relinking from persisted configuration.

## [1.0.6] — 2026-10-24

### Added

- **Cyberpunk / Blade Runner Video Demonstration (`assets/cyberpunk_blade_runner_demo.mp4`):**
  - Generated a 46-second Full HD (1920x1080 @ 30 fps) retro-futuristic video demonstration showcasing ReplicantPeak's synthesis engine in a Blade Runner neo-noir / Cyberpunk setting.
  - Multi-track soundtrack composed and synthesized directly via `peak-core` (`crates/peak-core/examples/cyberpunk_demo.rs`):
    - **CS-80 Expressive Brass Lead**: PolyBLEP Oxford Triple-Saw with 24 dB/oct resonant Chris Huggett OTA ladder filter sweep, Vangelis pitch vibrato, analog chorus, and tape echo.
    - **Atmospheric Pad**: Authentic `TnT Sadtouch` SysEx patch with slow 1.8-second swelling attack and stereo diffusion.
    - **Cyberpunk Analog Sub-Bass Drone**: Multi-stage diode saturation with asymmetric 2nd/4th harmonic coloration.
    - **16th-Note Dorian Sequence Arpeggio**: Crisp plucky envelope with synchronized stereo ping-pong delay.
  - Live animated visual telemetry:
    - Real-time oscilloscope rendered inside the authentic 256x64 OLED display reflecting the audio waveform.
    - Dynamic 8-Voice Activity LEDs illuminating in sync with sounding notes.
    - 64-band neon spectrum analyzer across the bottom grid horizon.
    - Cyberpunk telemetry HUD with real-time timecode, M4 cycle counter, and sectional track annotations.

## [1.0.5] — 2026-10-24

### Fixed & Improved

- **Oscillator Pitch Scaling & Ratio Calculation:**
  - Corrected octave conversion formula in `osc_ratio` from `(tune + fine/100).exp2()` to `((tune + fine/100) / 12.0).exp2()`. Previously, integer semitones were passed directly to base-2 exponentiation, shifting frequencies by octaves instead of semitones and pushing oscillators above the filter cutoff.
- **SysEx Preset Parser (`parse_syx_patch`) Calibration:**
  - Fixed coarse tune byte centering for Oscillator 2 (`byte 75`) and Oscillator 3 (`byte 96`) to subtract 64.0 (bipolar center) instead of 60.0, resolving unwanted detuning offsets.
  - Aligned Oscillator 1 fine tune mapping (`byte 54`), correctly anchoring Osc 1 at 0 cents.
  - Added Delay Feedback ingestion (`byte 234`).
  - Added full ingestion and bundling of `TnTSadtouch.syx` into `PEAK_DIST_03_SYX_ALL`.
- **DAHDSR Envelope Scaling & Range:**
  - Expanded envelope limits (`AmpAttack`, `AmpDecay`, `AmpRelease`, `FilterAttack`, etc.) up to 20–25 seconds matching the Novation Peak hardware specification.
  - Calibrated non-linear envelope slider mapping curve ($t^{3.8}$ polynomial), accurately reproducing the 1.8-second swelling pad attack for `TnT Sadtouch` as heard in the reference video (`https://www.youtube.com/watch?v=WvYsF5q3KMQ`).
- **Stereo Imaging & Panning Balance:**
  - Fixed panning sum in voice mixing to `(1.0 - pan)`, restoring balanced left/right acoustic energy across stereo channels.
- **Acoustic Fidelity Testing:**
  - Added automated acoustic test `tnt_sadtouch_fidelity_analysis` testing harmonic spectrum, attack times, and stereo correlation.

## [1.0.4] — 2026-10-24

### Added & Improved

- **Oxford Oscillator DSP Engine Overhaul (`New Oxford Oscillators`):**
  - Bandlimited PolyBLEP (polynomial bandlimited step) oscillator generator for analogue shapes (Saw, Pulse, Triangle, Sine), eliminating aliasing across the entire audio band.
  - Authentic Oxford **Triple-Saw (Saw Density / Detune)**: When Sawtooth is selected with positive SHAPE, blends in two symmetrically detuned side saws ($\pm 35$ cents) with independent phase accumulators to create a massive, lush analog supersaw cluster. When SHAPE is negative, morphs the ramp slope into a dual-ramp waveform.
  - Continuous **PolyBLEP Pulse Width Modulation (PWM)**: Modulates pulse duty cycle seamlessly from 2% to 98% with anti-aliased transitions.
  - Oxford Wavefolded Saturation: Shape-driven wavefolder for Triangle and Sine waveforms, generating rich analog overtones.
- **Analogue OTA Multimode Filter (Chris Huggett Architecture):**
  - Modeled after the Bass Station II and OSCar analog filters.
  - 12 dB/oct: 2-pole OTA state-variable filter with non-linear saturation on resonance integrators.
  - 24 dB/oct: 4-pole cascaded OTA ladder with global non-linear feedback and authentic self-oscillation into a warm, screaming sine wave across the keyboard ($Q \ge 0.95$).
  - Per-sample cutoff evaluation (zero decimation, removing 16-sample block rate) ensuring full audio-rate filter FM (`Osc3FilterMod`) and instantaneous sub-millisecond envelope attacks.
  - Filter Diverge: Modeled analog component tolerance variations ($\pm 1.5\%$) per voice for wide, organic polyphonic imaging.
- **Multi-Stage Asymmetric Analogue Saturation & Tone Circuit:**
  - Replaced sterile symmetric `tanh` with monotonic asymmetric transfer curves generating rich 2nd and 4th order even harmonics for pre-filter drive, post-filter drive, and master distortion.
  - 14 kHz analog 1-pole smoothing filter on master distortion to emulate analog guitar/synth pedal presence without harsh digital fizz.
  - AC-coupling DC blocker emulating physical hardware line output coupling capacitors.
- **SysEx Preset Ingestion (`.syx`):**
  - Full ingestion of `FM12`, `FM23`, `Osc3FilterMod`, `Drift`, and `Diverge` parameters from Novation Peak 527-byte patch dumps.
- **Acoustic Cycle-by-Cycle Verification:**
  - Automated acoustic test suite analyzing waveforms sound-by-sound for all 10 Montfrooij `PeakDist03` presets demonstrated in video `https://www.youtube.com/watch?v=9DpocpgXDaM`.

## [1.0.3] — 2026-10-24

### Added

- **Firmware Emulation Integration in Plugin Runtime:**
  - Integrated `peak-emulator` into `peak-plugin` and `peak-panel`.
  - When the firmware is linked, the STM32F429 ARM Cortex-M4 emulator is instantiated and executed, stepping instruction cycles in the background to drive front panel MMIO and display state.
  - Active instruction counter and MMIO communication exposed to the host interface.
- **Authentic Novation Peak OLED Display Panel (`peak-ui`):**
  - Integrated 256×64 monochrome OLED screen with dot-matrix scanlines, live oscilloscope waveform, patch metadata, and real-time CPU emulation cycle count.
  - 8 physical Voice Activity LEDs on the display panel with glowing cyan halos and white center hotspots, illuminating dynamically per sounding voice.
- **Master Volume Control:**
  - Added dedicated `VOLUME` knob to the front panel voice section (`P::Volume`, `-48.0 dB .. +6.0 dB`).
- **Novation Peak SysEx Preset Import (`.syx`):**
  - Added native SysEx parser for 527-byte Peak single patch dumps (`parse_syx_patch`), mapping Oscillators 1-3, Mixer, Filter, Envelopes, LFOs, FX, and Voice parameters.
  - Added `IMPORT .SYX` button with in-display path loader and drag-and-drop support for `.syx` patch files.
  - Bundled all 10 Montfrooij `PeakDist03` presets (`TnTDistCareFULL`, `TnTSyncPadKrunch`, etc.) into the factory library.
- **Sound Fidelity & Acoustic Tuning:**
  - Tuned saturation and resonance behavior against the reference demonstration in video [9DpocpgXDaM].
  - Updated README screenshot `assets/panel.png` capturing the new OLED screen and Volume knob.

## [1.0.2] — 2026-10-24

### Added

- **ARM Cortex-M4 Microcontroller Emulator (`crates/peak-emulator`):**
  - High-performance execution engine based on Unicorn (`unicorn-engine` v2.1) in ARM Thumb-2 mode.
  - Novation SysEx 7-to-8 bit stream unpacker extracting the 249 KB binary payload from `.syx` firmware images.
  - Full memory map: Flash (`0x08000000`), SRAM (`0x20000000`), Boot alias (`0x00000000`), and ARM Cortex-M bit-band regions (`0x42000000` / `0x22000000`).
  - MMIO virtual peripheral stubs for STM32F4:
    - RCC (Reset and Clock Control) with ready flags for HSE, PLL, and system clock switches.
    - PWR (Power Control) with VOSRDY and BRR (Backup Regulator Ready) stubs.
    - UART (USART6 / UART4) capturing serial logs and debug console output.
    - GPIO (GPIOA-GPIOE) with pin input injection for buttons/switches and output pin state tracking for LEDs.
    - SPI (SPI1-SPI4) stubs handling high-speed transfers to OLED and FPGA.
    - SysTick timer and simulated timer ticks to advance firmware delay loops.
  - CLI runner `emulate` with `--headless`, `--steps`, and `--trace` options.
  - **Novation Peak Hardware LED & OLED GUI:**
    - Live 256×64 monochrome OLED dot-matrix display rendering.
    - 8 physical Voice Activity LEDs with glowing bezels and indicators.
    - Hardware status LEDs (Animate 1/2, Arp, Filter 24dB).
    - Virtual navigation buttons (`PAGE <`, `PAGE >`, `PATCH`, `EXIT`, `INIT`, `COMPARE`).
    - CPU monitor, live register inspector, serial console terminal, and MMIO activity stream.
- **Hardware 8-Voice LEDs in `peak-ui`:**
  - Added the 8 discrete Voice LEDs with metallic bezels and cyan glow to the Voice section of the main front panel in `peak-ui` and `peak-panel`.

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
