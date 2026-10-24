//! A mip-mapped wavetable bank.
//!
//! Eight factory tables follow Peak's oscillator shapes; four "user" tables are
//! derived deterministically from the firmware digest, so a given `.syx` always
//! produces the same bank. Tables are band-limited by construction: each mip
//! level is synthesised additively with half the harmonics of the level below,
//! which keeps aliasing down without per-sample filtering.

use std::sync::Arc;

pub const TABLE_SIZE: usize = 2048;
pub const MIP_LEVELS: usize = 10;
/// Harmonics in the widest table. Halved once per mip level.
const MAX_HARMONICS: usize = TABLE_SIZE / 2;

pub const FACTORY_TABLES: usize = 8;
pub const USER_TABLES: usize = 4;
pub const TABLES: usize = FACTORY_TABLES + USER_TABLES;

#[derive(Clone)]
struct Table {
    /// One band-limited cycle per mip level, `TABLE_SIZE` samples each.
    levels: Vec<Box<[f32]>>,
}

impl Table {
    #[inline]
    fn read(&self, level: usize, phase: f32) -> f32 {
        let data = &self.levels[level.min(self.levels.len() - 1)];
        let x = phase.rem_euclid(1.0) * TABLE_SIZE as f32;
        let i0 = x as usize & (TABLE_SIZE - 1);
        let frac = x - x.floor();
        let a = data[i0];
        let b = data[(i0 + 1) & (TABLE_SIZE - 1)];
        a + (b - a) * frac
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

    #[inline]
    pub fn sample(&self, table: usize, level: usize, phase: f32) -> f32 {
        self.tables[table % self.tables.len()].read(level, phase)
    }

    /// Crossfade between two neighbouring tables, which is what the SHAPE knob
    /// sweeps through.
    #[inline]
    pub fn sample_morph(&self, a: usize, b: usize, morph: f32, level: usize, phase: f32) -> f32 {
        let sa = self.sample(a, level, phase);
        if b == a || morph <= 0.0 {
            return sa;
        }
        let sb = self.sample(b, level, phase);
        sa + (sb - sa) * morph
    }

    /// One cycle of a table at the widest mip level, for the panel's scope.
    pub fn cycle(&self, table: usize) -> &[f32] {
        &self.tables[table % self.tables.len()].levels[0]
    }

    /// Build the whole bank. Costs a few tens of milliseconds; run it off the
    /// audio thread.
    pub fn generate(seed: u64) -> Arc<Bank> {
        let mut tables = Vec::with_capacity(TABLES);
        for spec in factory_specs().iter() {
            tables.push(synth(spec));
        }
        let mut rng = SplitMix64::new(seed);
        for _ in 0..USER_TABLES {
            tables.push(synth(&user_spec(&mut rng)));
        }
        Arc::new(Bank { tables, seed })
    }
}

struct Harmonics {
    /// (harmonic number, amplitude) pairs, phases are fixed per spec.
    amps: Vec<(usize, f32)>,
    /// Per-harmonic phase offset in cycles; used by the seeded user tables.
    phases: Vec<f32>,
    tilt: f32,
}

impl Harmonics {
    fn new() -> Self {
        Self {
            amps: Vec::new(),
            phases: Vec::new(),
            tilt: 1.0,
        }
    }
}

fn factory_specs() -> [Harmonics; FACTORY_TABLES] {
    let mut sine = Harmonics::new();
    sine.amps.push((1, 1.0));

    let mut tri = Harmonics::new();
    for h in (1..MAX_HARMONICS).step_by(2) {
        let sign = if (h / 2) % 2 == 0 { 1.0 } else { -1.0 };
        tri.amps.push((h, sign / (h * h) as f32));
    }

    let mut saw = Harmonics::new();
    for h in 1..MAX_HARMONICS {
        saw.amps.push((h, 1.0 / h as f32));
    }

    let mut square = Harmonics::new();
    for h in (1..MAX_HARMONICS).step_by(2) {
        square.amps.push((h, 1.0 / h as f32));
    }

    let mut p25 = Harmonics::new();
    let mut p12 = Harmonics::new();
    for h in 1..MAX_HARMONICS {
        p25.amps.push((h, pulse_amp(h, 0.25)));
        p12.amps.push((h, pulse_amp(h, 0.12)));
    }

    let mut organ = Harmonics::new();
    for (h, a) in [
        (1, 1.0),
        (2, 0.55),
        (3, 0.42),
        (4, 0.30),
        (6, 0.22),
        (8, 0.16),
        (12, 0.1),
    ] {
        organ.amps.push((h, a));
    }

    let mut formant = Harmonics::new();
    for h in 1..96 {
        let f = h as f32;
        let a = gauss(f, 4.0, 2.2) + 0.7 * gauss(f, 11.0, 3.0) + 0.35 * gauss(f, 20.0, 4.0);
        if a > 1e-3 {
            formant.amps.push((h, a / f.sqrt()));
        }
    }

    [sine, tri, saw, square, p25, p12, organ, formant]
}

fn pulse_amp(h: usize, duty: f32) -> f32 {
    let d = std::f32::consts::PI * h as f32 * duty;
    (2.0 / (h as f32 * std::f32::consts::PI)) * d.sin()
}

fn gauss(x: f32, mu: f32, sigma: f32) -> f32 {
    (-0.5 * ((x - mu) / sigma).powi(2)).exp()
}

fn user_spec(rng: &mut SplitMix64) -> Harmonics {
    let mut spec = Harmonics::new();
    let roll = rng.next_f32();
    let limit = 24 + (roll * 56.0) as usize;
    for h in 1..=limit {
        let amp = rng.next_f32().powf(2.2) / (h as f32).powf(1.25);
        if amp > 1e-4 {
            spec.amps.push((h, amp));
            spec.phases.push(rng.next_f32());
        }
    }
    spec
}

/// Additive synthesis of one table, one mip level at a time.
fn synth(spec: &Harmonics) -> Table {
    let mut levels = Vec::with_capacity(MIP_LEVELS);
    for level in 0..MIP_LEVELS {
        let limit = (MAX_HARMONICS >> level).max(1);
        let mut buf = vec![0.0f32; TABLE_SIZE];
        for (i, &(h, amp)) in spec.amps.iter().enumerate() {
            if h > limit {
                break;
            }
            let phase0 = spec.phases.get(i).copied().unwrap_or(0.0) * std::f32::consts::TAU;
            let w = std::f32::consts::TAU * h as f32 / TABLE_SIZE as f32;
            // Rotate a unit complex number instead of calling sin() per sample.
            let (mut c, mut s) = (phase0.cos(), phase0.sin());
            let (dw_c, dw_s) = (w.cos(), w.sin());
            let gain = amp / spec.tilt;
            for sample in buf.iter_mut() {
                *sample += s * gain;
                let nc = c * dw_c - s * dw_s;
                let ns = s * dw_c + c * dw_s;
                c = nc;
                s = ns;
            }
        }

        // Remove DC and normalise, so every table arrives at a comparable level
        // and the SHAPE knob does not jump in loudness.
        let mean = buf.iter().sum::<f32>() / TABLE_SIZE as f32;
        let mut peak = 0.0f32;
        for sample in buf.iter_mut() {
            *sample -= mean;
            peak = peak.max(sample.abs());
        }
        if peak > 0.0 {
            let scale = 0.99 / peak;
            for sample in buf.iter_mut() {
                *sample *= scale;
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
            assert_eq!(bank.cycle(t).len(), TABLE_SIZE);
        }
    }

    #[test]
    fn tables_are_finite_dc_free_and_normalised() {
        let bank = Bank::generate(7);
        for t in 0..TABLES {
            for level in 0..MIP_LEVELS {
                let mut peak = 0.0f32;
                for i in 0..TABLE_SIZE {
                    let s = bank.sample(t, level, i as f32 / TABLE_SIZE as f32);
                    assert!(s.is_finite(), "table {t} level {level} is not finite");
                    peak = peak.max(s.abs());
                }
                assert!(peak <= 1.0, "table {t} level {level} peak {peak}");
                assert!(peak > 0.2, "table {t} level {level} is nearly silent");
            }
        }
    }

    #[test]
    fn factory_tables_carry_the_right_harmonics() {
        let bank = Bank::generate(1);
        // SINE: fundamental only.
        let sine = bank.cycle(0);
        assert!(bin_magnitude(sine, 1) > 0.9);
        assert!(bin_magnitude(sine, 2) < 1e-3);
        assert!(bin_magnitude(sine, 3) < 1e-3);

        // SAW: 1/h across the spectrum.
        let saw = bank.cycle(2);
        let h1 = bin_magnitude(saw, 1);
        for h in [2, 3, 4, 8] {
            let expected = h1 / h as f32;
            assert!(
                (bin_magnitude(saw, h) - expected).abs() < 0.05,
                "saw harmonic {h} deviates"
            );
        }

        // SQUARE: odd harmonics only.
        let square = bank.cycle(3);
        assert!(bin_magnitude(square, 2) < 1e-3);
        assert!(bin_magnitude(square, 4) < 1e-3);
        assert!(bin_magnitude(square, 5) > 0.1);
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
            for (x, y) in a.cycle(t).iter().zip(b.cycle(t).iter()) {
                assert_eq!(x, y, "same seed must produce the same bank");
            }
        }
        // A different digest changes at least one user table.
        let differs = (FACTORY_TABLES..TABLES).any(|t| {
            a.cycle(t)
                .iter()
                .zip(c.cycle(t).iter())
                .any(|(x, y)| x != y)
        });
        assert!(differs, "different seeds must change the user tables");
    }

    #[test]
    fn morph_interpolates_between_tables() {
        let bank = Bank::generate(3);
        let phase = 0.25;
        let a = bank.sample(0, 0, phase);
        let b = bank.sample(2, 0, phase);
        let mid = bank.sample_morph(0, 2, 0.5, 0, phase);
        assert!(((a + b) * 0.5 - mid).abs() < 1e-6);
        assert_eq!(bank.sample_morph(0, 2, 0.0, 0, phase), a);
    }
}
