# Peak fidelity notes

This release is a deliberate architecture pass, not a claim that ReplicantPeak is
a bit-exact model of a Novation Peak. The Peak's oscillators run on an FPGA at a
24 MHz carrier, its filter and drive stages are analogue, and its wavetables and
factory patch bank are proprietary. None of that is present in the firmware
`.syx` the plugin links, so a literal 1:1 clone is not possible from the public
material. What *is* documented is the voice architecture, and that is what this
release follows.

## Sources used

- Novation, *Peak 1.2 Manual* (English), especially the oscillator, LFO,
  envelope, filter, FX and appendix chapters:
  <https://fael-downloads-prod.focusrite.com/customer/dev/s3fs-public/downloads/Peak%201.2%20Manual%20-%20English.pdf>
- Novation, *Peak* product page and feature list:
  <https://novationmusic.com/en/synths/peak>
- Novation, *Peak appendix* / MIDI parameter list:
  <https://userguides.novationmusic.com/hc/en-gb/articles/25494651392146-Peak-appendix>
- Sound On Sound, *Novation Peak* review:
  <https://www.soundonsound.com/reviews/novation-peak>
- MIDI CC/NRPN reference: <https://midi.guide/d/novation/summit-and-peak/>

## What changed

### Oscillators

Peak has three “New Oxford” numerically controlled oscillators per voice. Each
one selects a waveform (sine, triangle, saw, pulse, or one of 60 wavetables) and
a bipolar SHAPE amount. The 60 wavetables are each a bank of five waveforms, and
SHAPE interpolates across those five columns.

ReplicantPeak now models that structure directly:

- Every table stores **five band-limited frames per mip level**.
- The existing `WAVE` parameter selects the table; the new `SHAPE` parameter is
  bipolar, with 0 at the middle column. The panel's scope drags SHAPE and the
  wave strip selects the table.
- The eight factory tables are original procedural approximations. They are not
  Novation's data and are not labelled as such.
- The four user tables are still derived from the firmware digest, now with
  five correlated frames each.
- `FINE` is ±100 cents, matching the hardware. `COARSE` keeps the existing ±24
  semitone range as a combined Range + Coarse control; adding a separate foot
  range switch would need another oscillator parameter and panel control.
- Free-running phase on note-on, slow **Drift**, and static per-oscillator
  **Diverge** reproduce the analogue character of the NCO bank.
- **FM O1>O2** and **FM O2>O3** are phase-modulation routes, matching Peak's
  documented FM destinations.

Not modelled: Vsync (the virtual-oscillator sync algorithm is not documented in
enough detail to reproduce honestly), the 60 factory wavetables, Saw Density /
Density Detune, fixed-note mode, and the alternative tuning tables.

### Filter

Peak's filter is a resonant multi-mode design with LP, BP and HP shapes and a
12/24 dB per octave slope switch, plus pre-filter overdrive, envelope amount,
LFO 1 depth, Osc 3 audio-rate modulation and key tracking.

ReplicantPeak now exposes six modes in the original preset order:

`LP 12`, `LP 24`, `BP 12`, `HP 24`, `BP 24`, `HP 12`

The 24 dB modes cascade two topology-preserving state-variable stages. The
filter section also exposes `OSC3 MOD` (audio-rate Osc 3 → cutoff) and
`NOISE LP` (the noise generator's own low-pass). Overdrive remains pre-filter,
and the global distortion remains post-VCA, as on the hardware.

The filter is still a digital SVF, not a component-level model of Peak's
analogue filter. The resonance curve and drive are tuned by ear, not measured
against a unit.

### Envelopes

Peak has an Amp envelope and two modulation envelopes, all AHDSR (attack, hold,
decay, sustain, release). This release adds **HOLD** to the amp and filter
envelopes and keeps the exponential decay/release shape. The two-mod-envelope
and 16-slot modulation matrix are not exposed by the current parameter set; the
filter envelope remains the dedicated filter modulator.

### LFOs

Peak's panel LFOs are per voice, free-running by default, with triangle, saw,
square and sample-and-hold shapes, a fade-in time, and optional key sync.

ReplicantPeak now gives every voice its own LFO 1 and LFO 2, starts them at a
free-running phase, adds **S&H**, adds **FADE** (0–5 s), and sums both LFOs when
they share a destination. The destination set remains Pitch, Filter, Shape and
Amp.

### Voice and FX

- 8 voices by default, up to 16 (the Peak is 8; the Summit is 16).
- Per-voice panning is now very narrow, so width comes from chorus/delay rather
  than from an artificial alternating pan.
- Ring modulation is still `Osc 1 × Osc 2`.
- The FX chain remains distortion → chorus → delay → reverb, with the same
  controls as before.

## What “1:1” would still require

1. The actual Peak wavetable data (60 tables × 5 columns).
2. A measured model of the analogue filter, VCA and overdrive stages.
3. The full 16-slot modulation matrix, two mod envelopes, four LFOs, Vsync,
   Saw Density, arpeggiator and voice modes.
4. A preset importer for Peak's proprietary `.syx` program dumps, which
   Novation does not publicly document byte-for-byte.

The implementation is intentionally honest about these gaps: the UI, parameter
ranges and signal topology follow the manual, while the exact analogue transfer
functions are approximations.
