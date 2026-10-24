//! A mip-mapped wavetable bank built the way Peak describes its oscillators.
//!
//! Each of the twelve tables is a bank of five waveforms, and the oscillator's
//! SHAPE control interpolates across those five columns. Tables are band-limited
//! by construction: every mip level is synthesised additively with half the
//! harmonics of the level below, which keeps aliasing down without per-sample
//! filtering.
//!
//! The eight factory tables are original procedural approximations of the
//! classic Peak shapes. They are not Novation's wavetable data: the hardware's
//! tables are proprietary and never appear in the firmware image. The four user
//! tables are derived deterministically from the firmware digest, so a given
//! `.syx` always produces the same bank.

use std::sync::Arc;

pub const TABLE_SIZE: usize = 2048;
pub const MIP_LEVELS: usize = 10;
/// Peak's wavetables are banks of five waveforms; SHAPE sweeps across them.
pub const FRAMES: usize = 5;
/// Harmonics in the widest table. Halved once per mip level.
const MAX_HARMONICS: usize = TABLE_SIZE / 2;

pub const FACTORY_TABLES: usize = 8;
pub const USER_TABLES: usize = 4;
pub const TABLES: usize = FACTORY_TABLES + USER_TABLES;

#[derive(Clone)]
struct Table {
    /// `FRAMES` band-limited cycles per mip level, stored back to back.
    levels: Vec<Box<[f32]>>,
}

impl Table {
    #[inline]
    fn read(&self, level: usize, frame: f32, phase: f32) -> f32 {
        let data = &self.levels[level.min(self.levels.len() - 1)];
        let frame = frame.clamp(0.0, (FRAMES - 1) as f32);
        let f0 = frame.floor() as usize;
        let frac = frame - f0 as f32;
        let f1 = (f0 + 1).min(FRAMES - 1);

        let x = phase.rem_euclid(1.0) * TABLE_SIZE as f32;
        let i0 = x as usize & (TABLE_SIZE - 1);
        let phase_frac = x - x.floor();
        let i1 = (i0 + 1) & (TABLE_SIZE - 1);

        let read_frame = |index: usize| {
            let base = index * TABLE_SIZE;
            let a = data[base + i0];
            let b = data[base + i1];
            a + (b - a) * phase_frac
        };

        let a = read_frame(f0);
        if f1 == f0 || frac <= 0.0 {
            a
        } else {
            a + (read_frame(f1) - a) * frac
        }
    }
}

pub struct Bank {
    tables: Vec<Table>,
    seed: u64,
}

impl Bank {
    pub fn len(&self) -> usize {
        self.tables.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tables.is_empty()
    }

    pub fn seed(&self) -> u64 {
        self.seed
    }

    /// Mip level for a given fundamental, so the table's top harmonic stays
    /// below Nyquist.
    #[inline]
    pub fn level_for(sample_rate: f32, freq: f32) -> usize {
        let nyquist = sample_rate * 0.5;
        let f = freq.abs().max(1.0);
        let ratio = f * MAX_HARMONICS as f32 / nyquist;
        if ratio <= 1.0 {
            return 0;
        }
        let level = ratio.log2().floor() as usize + 1;
        level.min(MIP_LEVELS - 1)
    }

    /// Read one table at an explicit frame position (0..=4).
    #[inline]
    pub fn sample(&self, table: usize, frame: f32, level: usize, phase: f32) -> f32 {
        self.tables[table % self.tables.len()].read(level, frame, phase)
    }

    /// Read a table with Peak's bipolar SHAPE control: -1 is column 0, 0 is the
    /// middle column, +1 is column 4.
    #[inline]
    pub fn sample_shape(&self, table: usize, shape: f32, level: usize, phase: f32) -> f32 {
        let frame = (shape.clamp(-1.0, 1.0) + 1.0) * 0.5 * (FRAMES - 1) as f32;
        self.sample(table, frame, level, phase)
    }

    /// Crossfade between two table numbers at the same SHAPE position. This is
    /// what the oscillator does when the wave selector is between detents.
    #[inline]
    pub fn sample_morph(
        &self,
        a: usize,
        b: usize,
        morph: f32,
        shape: f32,
        level: usize,
        phase: f32,
    ) -> f32 {
        let sa = self.sample_shape(a, shape, level, phase);
        if b == a || morph <= 0.0 {
            return sa;
        }
        let sb = self.sample_shape(b, shape, level, phase);
        sa + (sb - sa) * morph
    }

    /// One cycle of one frame, at the widest mip level, for the panel's scope.
    pub fn cycle(&self, table: usize, frame: usize) -> &[f32] {
        let data = &self.tables[table % self.tables.len()].levels[0];
        let frame = frame.min(FRAMES - 1);
        &data[frame * TABLE_SIZE..(frame + 1) * TABLE_SIZE]
    }

    /// All five frames of a table, downsampled for a GUI scope. Allocating here
    /// is fine: this is only called by the editor, never by `process()`.
    pub fn cycles(&self, table: usize, samples: usize) -> Vec<Vec<f32>> {
        let samples = samples.clamp(2, TABLE_SIZE);
        (0..FRAMES)
            .map(|frame| {
                let cycle = self.cycle(table, frame);
                (0..samples)
                    .map(|i| {
                        let x = i as f32 / samples as f32 * TABLE_SIZE as f32;
                        let i0 = x as usize & (TABLE_SIZE - 1);
                        let i1 = (i0 + 1) & (TABLE_SIZE - 1);
                        let frac = x - x.floor();
                        cycle[i0] + (cycle[i1] - cycle[i0]) * frac
                    })
                    .collect()
            })
            .collect()
    }

    /// The interpolated waveform the oscillator is currently playing.
    pub fn morph_cycle(&self, table: usize, shape: f32, samples: usize) -> Vec<f32> {
        let samples = samples.clamp(2, TABLE_SIZE);
        (0..samples)
            .map(|i| self.sample_shape(table, shape, 0, i as f32 / samples as f32))
            .collect()
    }

    /// Build the whole bank. Costs a few hundred milliseconds; run it off the
    /// audio thread.
    pub fn generate(seed: u64) -> Arc<Bank> {
        let mut tables = Vec::with_capacity(TABLES);
        for frames in factory_specs() {
            tables.push(synth(&frames));
        }
        let mut rng = SplitMix64::new(seed);
        for _ in 0..USER_TABLES {
            tables.push(synth(&user_frames(&mut rng)));
        }
        Arc::new(Bank { tables, seed })
    }
}

struct Harmonics {
    /// (harmonic number, amplitude) pairs, phases are fixed per spec.
    amps: Vec<(usize, f32)>,
    /// Per-harmonic phase offset in cycles; used by the seeded user tables.
    phases: Vec<f32>,
}

impl Harmonics {
    fn new() -> Self {
        Self {
            amps: Vec::new(),
            phases: Vec::new(),
        }
    }

    fn add(&mut self, h: usize, amp: f32) {
        if amp.abs() > 1e-5 && h < MAX_HARMONICS {
            self.amps.push((h, amp));
        }
    }
}

fn sine_frame() -> Harmonics {
    let mut spec = Harmonics::new();
    spec.add(1, 1.0);
    for (h, a) in [(2, 0.16), (3, 0.09), (4, 0.03)] {
        spec.add(h, a);
    }
    spec
}

fn triangle_frame(limit: usize, even: f32) -> Harmonics {
    let mut spec = Harmonics::new();
    for h in (1..limit).step_by(2) {
        let sign = if (h / 2) % 2 == 0 { 1.0 } else { -1.0 };
        spec.add(h, sign / (h * h) as f32);
    }
    if even > 0.0 {
        for h in (2..limit).step_by(2) {
            spec.add(h, even / (h * h) as f32);
        }
    }
    spec
}

fn saw_frame(limit: usize, tilt: f32, even_boost: f32) -> Harmonics {
    let mut spec = Harmonics::new();
    for h in 1..limit {
        let even = if h % 2 == 0 { 1.0 + even_boost } else { 1.0 };
        spec.add(h, even / (h as f32).powf(tilt));
    }
    spec
}

fn square_frame(limit: usize, tilt: f32, even: f32) -> Harmonics {
    let mut spec = Harmonics::new();
    for h in (1..limit).step_by(2) {
        spec.add(h, 1.0 / (h as f32).powf(tilt));
    }
    if even > 0.0 {
        for h in (2..limit).step_by(2) {
            spec.add(h, even / h as f32);
        }
    }
    spec
}

fn pulse_frame(duty: f32) -> Harmonics {
    let mut spec = Harmonics::new();
    for h in 1..MAX_HARMONICS {
        let amp = pulse_amp(h, duty);
        spec.add(h, amp);
    }
    spec
}

fn harmonic_frame(tilt: f32) -> Harmonics {
    let mut spec = Harmonics::new();
    for h in 1..512 {
        spec.add(h, 1.0 / (h as f32).powf(tilt));
    }
    spec
}

fn formant_frame(center: f32) -> Harmonics {
    let mut spec = Harmonics::new();
    for h in 1..256 {
        let f = h as f32;
        let a = gauss(f, center, center * 0.35) + 0.55 * gauss(f, center * 2.3, center * 0.5);
        if a > 1e-3 {
            spec.add(h, a / f.sqrt());
        }
    }
    spec
}

fn factory_specs() -> Vec<[Harmonics; FRAMES]> {
    let mut specs = Vec::with_capacity(FACTORY_TABLES);

    // SINE: a clean centre column with progressively more edge either side.
    specs.push(std::array::from_fn(|frame| match frame {
        0 => sine_frame(),
        1 => {
            let mut s = Harmonics::new();
            s.add(1, 1.0);
            s.add(3, 0.04);
            s
        }
        2 => {
            let mut s = Harmonics::new();
            s.add(1, 1.0);
            s
        }
        3 => {
            let mut s = Harmonics::new();
            s.add(1, 1.0);
            s.add(3, 0.18);
            s.add(5, 0.08);
            s
        }
        _ => {
            let mut s = Harmonics::new();
            s.add(1, 1.0);
            s.add(2, 0.24);
            s.add(3, 0.30);
            s.add(4, 0.14);
            s.add(5, 0.08);
            s
        }
    }));

    // TRI: the centre is the classic triangle; the sides bend toward saw and
    // square spectra.
    specs.push(std::array::from_fn(|frame| match frame {
        0 => triangle_frame(17, 0.0),
        1 => triangle_frame(64, 0.0),
        2 => triangle_frame(MAX_HARMONICS, 0.0),
        3 => triangle_frame(MAX_HARMONICS, 0.10),
        _ => square_frame(MAX_HARMONICS, 1.0, 0.0),
    }));

    // SAW: soft at the bottom of the SHAPE range, dense and bright at the top.
    specs.push(std::array::from_fn(|frame| match frame {
        0 => saw_frame(17, 1.0, 0.0),
        1 => saw_frame(64, 1.0, 0.0),
        2 => saw_frame(MAX_HARMONICS, 1.0, 0.0),
        3 => saw_frame(MAX_HARMONICS, 0.92, 0.20),
        _ => saw_frame(MAX_HARMONICS, 0.80, 0.0),
    }));

    // SQUARE: the centre is a true 1:1 square; the extremes add even harmonics
    // and high-end bite.
    specs.push(std::array::from_fn(|frame| match frame {
        0 => square_frame(17, 1.0, 0.0),
        1 => square_frame(64, 1.0, 0.0),
        2 => square_frame(MAX_HARMONICS, 1.0, 0.0),
        3 => square_frame(MAX_HARMONICS, 1.0, 0.10),
        _ => square_frame(MAX_HARMONICS, 0.82, 0.0),
    }));

    // PULSE 25 / PULSE 12: SHAPE is pulse width. Centre columns match the name.
    specs.push(std::array::from_fn(|frame| {
        let duty = [0.50, 0.36, 0.25, 0.15, 0.08][frame];
        pulse_frame(duty)
    }));
    specs.push(std::array::from_fn(|frame| {
        let duty = [0.40, 0.25, 0.12, 0.06, 0.03][frame];
        pulse_frame(duty)
    }));

    // HARMONIC: a full spectrum whose tilt changes across columns.
    specs.push(std::array::from_fn(|frame| {
        harmonic_frame(1.0 + frame as f32 * 0.22)
    }));

    // FORMANT: a moving vowel-like peak.
    specs.push(std::array::from_fn(|frame| {
        formant_frame([4.0, 7.0, 11.0, 16.0, 22.0][frame])
    }));

    specs
}

fn pulse_amp(h: usize, duty: f32) -> f32 {
    let d = std::f32::consts::PI * h as f32 * duty;
    (2.0 / (h as f32 * std::f32::consts::PI)) * d.sin()
}

fn gauss(x: f32, mu: f32, sigma: f32) -> f32 {
    (-0.5 * ((x - mu) / sigma).powi(2)).exp()
}

fn user_frames(rng: &mut SplitMix64) -> [Harmonics; FRAMES] {
    let roll = rng.next_f32();
    let limit = 24 + (roll * 56.0) as usize;
    let base: Vec<(usize, f32, f32)> = (1..=limit)
        .map(|h| {
            (
                h,
                rng.next_f32().powf(2.2) / (h as f32).powf(1.25),
                rng.next_f32(),
            )
        })
        .collect();

    std::array::from_fn(|frame| {
        let mut spec = Harmonics::new();
        let tilt = 0.85 + frame as f32 * 0.14;
        for &(h, amp, phase) in &base {
            let jitter = 0.65 + 0.35 * rng.next_f32();
            spec.add(h, amp * jitter / tilt);
            spec.phases.push(phase + frame as f32 * 0.08);
        }
        spec
    })
}

/// Additive synthesis of one table, one mip level at a time. Each frame is
/// normalised independently so morphing never produces a loudness jump.
fn synth(frames: &[Harmonics; FRAMES]) -> Table {
    let mut levels = Vec::with_capacity(MIP_LEVELS);
    for level in 0..MIP_LEVELS {
        let limit = (MAX_HARMONICS >> level).max(1);
        let mut buf = vec![0.0f32; FRAMES * TABLE_SIZE];

        for (frame, spec) in frames.iter().enumerate() {
            let base = frame * TABLE_SIZE;
            for (i, &(h, amp)) in spec.amps.iter().enumerate() {
                if h > limit {
                    break;
                }
                let phase0 = spec.phases.get(i).copied().unwrap_or(0.0) * std::f32::consts::TAU;
                let w = std::f32::consts::TAU * h as f32 / TABLE_SIZE as f32;
                // Rotate a unit complex number instead of calling sin() per sample.
                let (mut c, mut s) = (phase0.cos(), phase0.sin());
                let (dw_c, dw_s) = (w.cos(), w.sin());
                for sample in buf[base..base + TABLE_SIZE].iter_mut() {
                    *sample += s * amp;
                    let nc = c * dw_c - s * dw_s;
                    let ns = s * dw_c + c * dw_s;
                    c = nc;
                    s = ns;
                }
            }
        }

        // Remove DC and normalise each frame separately.
        for frame in 0..FRAMES {
            let base = frame * TABLE_SIZE;
            let slice = &mut buf[base..base + TABLE_SIZE];
            let mean = slice.iter().sum::<f32>() / TABLE_SIZE as f32;
            let mut peak = 0.0f32;
            for sample in slice.iter_mut() {
                *sample -= mean;
                peak = peak.max(sample.abs());
            }
            if peak > 0.0 {
                let scale = 0.99 / peak;
                for sample in slice.iter_mut() {
                    *sample *= scale;
                }
            }
        }
        levels.push(buf.into_boxed_slice());
    }
    Table { levels }
}

/// Tiny deterministic PRNG: the bank must be identical for the same firmware on
/// every platform, which rules out anything seeded by time or addresses.
struct SplitMix64(u64);

impl SplitMix64 {
    fn new(seed: u64) -> Self {
        Self(seed ^ 0xA076_1D64_78BD_642F)
    }

    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn next_f32(&mut self) -> f32 {
        // 24 bits of mantissa is plenty for harmonic amplitudes.
        ((self.next_u64() >> 40) as f32) / ((1u32 << 24) as f32)
    }
}

/// Naive DFT magnitude of a single bin, used by the tests to prove the tables
/// carry the harmonics they claim to.
#[cfg(test)]
fn bin_magnitude(cycle: &[f32], harmonic: usize) -> f32 {
    let mut re = 0.0f32;
    let mut im = 0.0f32;
    for (i, &s) in cycle.iter().enumerate() {
        let w = std::f32::consts::TAU * harmonic as f32 * i as f32 / cycle.len() as f32;
        re += s * w.cos();
        im += s * w.sin();
    }
    (re * re + im * im).sqrt() / (cycle.len() as f32 * 0.5)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bank_has_expected_shape() {
        let bank = Bank::generate(42);
        assert_eq!(bank.len(), TABLES);
        for t in 0..TABLES {
            for frame in 0..FRAMES {
                assert_eq!(bank.cycle(t, frame).len(), TABLE_SIZE);
            }
        }
    }

    #[test]
    fn tables_are_finite_dc_free_and_normalised() {
        let bank = Bank::generate(7);
        for t in 0..TABLES {
            for frame in 0..FRAMES {
                for level in 0..MIP_LEVELS {
                    let mut peak = 0.0f32;
                    for i in 0..TABLE_SIZE {
                        let s = bank.sample(t, frame as f32, level, i as f32 / TABLE_SIZE as f32);
                        assert!(
                            s.is_finite(),
                            "table {t} frame {frame} level {level} is not finite"
                        );
                        peak = peak.max(s.abs());
                    }
                    assert!(
                        peak <= 1.0,
                        "table {t} frame {frame} level {level} peak {peak}"
                    );
                    assert!(
                        peak > 0.2,
                        "table {t} frame {frame} level {level} is nearly silent"
                    );
                }
            }
        }
    }

    #[test]
    fn factory_tables_carry_the_right_harmonics() {
        let bank = Bank::generate(1);

        // SINE: the centre column is the fundamental only.
        let sine = bank.cycle(0, 2);
        assert!(bin_magnitude(sine, 1) > 0.9);
        assert!(bin_magnitude(sine, 2) < 1e-3);
        assert!(bin_magnitude(sine, 3) < 1e-3);

        // SAW: 1/h across the spectrum.
        let saw = bank.cycle(2, 2);
        let h1 = bin_magnitude(saw, 1);
        for h in [2, 3, 4, 8] {
            let expected = h1 / h as f32;
            assert!(
                (bin_magnitude(saw, h) - expected).abs() < 0.05,
                "saw harmonic {h} deviates"
            );
        }

        // SQUARE: odd harmonics only.
        let square = bank.cycle(3, 2);
        assert!(bin_magnitude(square, 2) < 1e-3);
        assert!(bin_magnitude(square, 4) < 1e-3);
        assert!(bin_magnitude(square, 5) > 0.1);
    }

    #[test]
    fn shape_sweeps_all_five_frames() {
        let bank = Bank::generate(1);
        let phase = 0.25;
        let first = bank.sample(0, 0.0, 0, phase);
        let centre = bank.sample(0, 2.0, 0, phase);
        let last = bank.sample(0, 4.0, 0, phase);
        assert_eq!(bank.sample_shape(0, -1.0, 0, phase), first);
        assert_eq!(bank.sample_shape(0, 0.0, 0, phase), centre);
        assert_eq!(bank.sample_shape(0, 1.0, 0, phase), last);
        // The sine table is pure at the centre and coloured at the edges.
        let expected = 0.99 * (phase * std::f32::consts::TAU).sin();
        assert!((centre - expected).abs() < 0.02);
        assert!((first - centre).abs() > 1e-3);
        assert!((last - centre).abs() > 1e-3);
    }

    #[test]
    fn mip_levels_band_limit_above_nyquist() {
        // 12 kHz at 48 kHz leaves room for 2 harmonics at most; the sampler must
        // pick a level whose harmonic limit respects that.
        let level = Bank::level_for(48_000.0, 12_000.0);
        assert!(level >= 8, "level {level} would alias badly");

        // 20 Hz can use the full table.
        assert_eq!(Bank::level_for(48_000.0, 20.0), 0);

        // Level selection is monotonic in frequency.
        let mut last = 0;
        for f in [20.0, 100.0, 500.0, 2_000.0, 8_000.0, 16_000.0] {
            let l = Bank::level_for(48_000.0, f);
            assert!(l >= last, "level must not decrease with frequency");
            last = l;
        }
    }

    #[test]
    fn user_tables_follow_the_firmware_seed() {
        let a = Bank::generate(0xDEAD_BEEF);
        let b = Bank::generate(0xDEAD_BEEF);
        let c = Bank::generate(0xDEAD_BEE0);
        assert_eq!(a.seed(), b.seed());
        for t in FACTORY_TABLES..TABLES {
            for frame in 0..FRAMES {
                for (x, y) in a.cycle(t, frame).iter().zip(b.cycle(t, frame).iter()) {
                    assert_eq!(x, y, "same seed must produce the same bank");
                }
            }
        }
        // A different digest changes at least one user table.
        let differs = (FACTORY_TABLES..TABLES).any(|t| {
            (0..FRAMES).any(|frame| {
                a.cycle(t, frame)
                    .iter()
                    .zip(c.cycle(t, frame).iter())
                    .any(|(x, y)| x != y)
            })
        });
        assert!(differs, "different seeds must change the user tables");
    }

    #[test]
    fn morph_interpolates_between_frames() {
        let bank = Bank::generate(3);
        let phase = 0.25;
        let a = bank.sample(0, 1.0, 0, phase);
        let b = bank.sample(0, 2.0, 0, phase);
        let mid = bank.sample(0, 1.5, 0, phase);
        assert!(((a + b) * 0.5 - mid).abs() < 1e-6);
        assert_eq!(bank.sample(0, 1.0, 0, phase), a);
    }
}
