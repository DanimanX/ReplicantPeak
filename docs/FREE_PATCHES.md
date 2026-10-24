# Free Peak patch sources

We looked for free Novation Peak/Summit banks while preparing 1.0.1. The short
version: free patches exist, but they are licensed for use on a Peak, not for
redistribution inside another product. ReplicantPeak therefore ships its own
factory bank and this list, rather than copying third-party files into the
repository.

## Free (registration or checkout may be required)

- **Novation — 1,000 artist patches for Peak & Summit.** Released in weekly sets
  through Novation Components. Free with a Novation account; the patches are
  Novation/artist property and are delivered through Components, not as a
  redistributable asset.
  <https://us.novationmusic.com/articles/1000-new-peak-summit-patches/>
- **Novation — Peak downloads and manuals.** Firmware, Components, the Peak
  manual and the MIDI/SysEx documentation live here.
  <https://downloads.novationmusic.com/novation/peak>
- **Anton Anru — “Hybrid Bass”, 22 free Peak/Summit presets.** A free pack from a
  commercial sound designer. Check the store's terms before redistributing;
  the free download is intended for your own Peak.
  <https://antonanru.sellfy.store/p/peak-bass/>
- **dettenbach audio — B+B demo pack, 10 free Peak patches.** The demo is free,
  but the included licence says “all rights reserved”, forbids copying,
  duplicating, uploading or reselling the sounds. Use it on your own Peak only.
  <http://dettenbach.com/sounds/synthesizer_presets/bb_for_novation_peak/>
- **Patchstorage — Novation Peak community patches.** Per-upload licences; read
  each patch page before using it in a release.
  <https://patchstorage.com/?s=Novation+Peak>
- **PresetPatch — Novation Peak.** Community and commercial patches; each entry
  states its own terms.
  <https://www.presetpatch.com/synth/novation-peak>

## Paid (not free, listed for completeness)

- LFO Store — *Best Analog & Ambient Sounds* (128 presets).
- Synth Patches — *Boards of Canada / Tycho / Tame Impala* (128 patches).
- Limbic Bits — *Limbic Interferences* (72 presets).

## Why they are not bundled

1. **Licence.** Most free packs grant use on the hardware, not redistribution.
   The dettenbach demo explicitly forbids copying and uploading.
2. **Format.** Peak patches are 512-byte SysEx program dumps inside a
   `F0 00 20 29 01 10 00 7E … F7` message. They are not JSON, and Novation does
   not publish a complete byte map. A lossy guess would produce patches that
   look imported but sound wrong.
3. **Provenance.** Novation's factory and artist banks are part of the Peak
   product. Shipping them in an MIT-licensed plugin would misrepresent both the
   licence and the sound.

## What ReplicantPeak ships instead

The factory bank is original: 19 patches written against this engine, including
seven 1.0.1 patches that exercise the new architecture (`NEON SAW`, `FM GLASS`,
`PULSE WIDTH`, `BANDPASS SCREAM`, `S&H VOID`, `HOLD ORGAN`, `DRIFT KEYS`).
They are MIT-licensed with the rest of the code. If you own free Peak patches,
keep them in your Peak/Components library; they are the reference material, not
a redistributable dependency.
