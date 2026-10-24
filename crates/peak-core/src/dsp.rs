//! The synthesis engine.
//!
//! Peak-inspired architecture: three wavetable oscillators plus noise and ring
//! modulation, a resonant state-variable filter with a dedicated envelope, two
//! global LFOs and a serial FX chain. Eight voices by default.
//!
//! Everything the audio thread touches is preallocated: `process()` never
//! allocates, locks or blocks, and every recursive state is flushed against
//! denormals.

use crate::params::{Patch, P};
use crate::wavetable::Bank;
use std::sync::Arc;

#[derive(Clone, Copy, Debug)]
pub enum Event {
    NoteOn { note: u8, velocity: f32 },
    NoteOff { note: u8 },
    AllNotesOff,
    PitchBend { normalized: f32 },
}

const MAX_VOICES: usize = 16;
const DENORMAL_GUARD: f32 = 1.0e-20;

#[inline]
fn flush(x: f32) -> f32 {
    if x.abs() < DENORMAL_GUARD {
        0.0
    } else {
        x
    }
}

#[inline]
fn midi_to_hz(note: f32) -> f32 {
    440.0 * ((note - 69.0) / 12.0).exp2()
}

/// One-pole parameter smoother. Prevents zipper noise on knob drags.
#[derive(Clone, Copy)]
pub struct Smoothed {
    current: f32,
    target: f32,
    coeff: f32,
}

impl Smoothed {
    fn new(value: f32) -> Self {
        Self {
            current: value,
            target: value,
            coeff: 0.2,
        }
    }

    fn set_time(&mut self, seconds: f32, sample_rate: f32) {
        let n = (seconds * sample_rate).max(1.0);
        self.coeff = (1.0 - (-1.0 / n).exp()).clamp(0.0, 1.0);
    }

    fn set_target(&mut self, value: f32) {
        self.target = value;
    }

    fn snap(&mut self, value: f32) {
        self.current = value;
        self.target = value;
    }

    #[inline]
    fn next(&mut self) -> f32 {
        self.current += (self.target - self.current) * self.coeff;
        flush(self.current)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum EnvState {
    Idle,
    Attack,
    Hold,
    Decay,
    Sustain,
    Release,
}

/// Peak's envelopes are AHDSR: attack, hold, decay, sustain and release.
pub struct Adsr {
    sample_rate: f32,
    attack: f32,
    hold: f32,
    decay: f32,
    sustain: f32,
    release: f32,
    state: EnvState,
    level: f32,
    attack_inc: f32,
    hold_left: u32,
    decay_coef: f32,
    release_coef: f32,
}

impl Adsr {
    fn new(sample_rate: f32) -> Self {
        let mut env = Self {
            sample_rate,
            attack: 0.005,
            hold: 0.0,
            decay: 0.6,
            sustain: 0.8,
            release: 0.3,
            state: EnvState::Idle,
            level: 0.0,
            attack_inc: 1.0,
            hold_left: 0,
            decay_coef: 0.001,
            release_coef: 0.001,
        };
        env.set_times(0.005, 0.0, 0.6, 0.8, 0.3);
        env
    }

    fn set_times(&mut self, attack: f32, hold: f32, decay: f32, sustain: f32, release: f32) {
        if (self.attack - attack).abs() < 1e-6
            && (self.hold - hold).abs() < 1e-6
            && (self.decay - decay).abs() < 1e-6
            && (self.sustain - sustain).abs() < 1e-6
            && (self.release - release).abs() < 1e-6
        {
            return;
        }
        self.attack = attack.max(0.0005);
        self.hold = hold.max(0.0);
        self.decay = decay.max(0.001);
        self.sustain = sustain.clamp(0.0, 1.0);
        self.release = release.max(0.001);

        self.attack_inc = 1.0 / (self.attack * self.sample_rate).max(1.0);
        self.hold_left = (self.hold * self.sample_rate).round() as u32;
        // Reach ~99% of the target within the stated time.
        self.decay_coef = 1.0 - (-4.6 / (self.decay * self.sample_rate).max(1.0)).exp();
        self.release_coef = 1.0 - (-4.6 / (self.release * self.sample_rate).max(1.0)).exp();
    }

    fn gate_on(&mut self) {
        self.state = EnvState::Attack;
        self.hold_left = (self.hold * self.sample_rate).round() as u32;
    }

    fn gate_off(&mut self) {
        if self.state != EnvState::Idle {
            self.state = EnvState::Release;
        }
    }

    fn kill(&mut self) {
        self.state = EnvState::Idle;
        self.level = 0.0;
    }

    fn is_active(&self) -> bool {
        self.state != EnvState::Idle
    }

    #[inline]
    fn next(&mut self) -> f32 {
        match self.state {
            EnvState::Idle => self.level = 0.0,
            EnvState::Attack => {
                self.level += self.attack_inc;
                if self.level >= 1.0 {
                    self.level = 1.0;
                    self.state = if self.hold_left > 0 {
                        EnvState::Hold
                    } else {
                        EnvState::Decay
                    };
                }
            }
            EnvState::Hold => {
                self.level = 1.0;
                if self.hold_left > 0 {
                    self.hold_left -= 1;
                }
                if self.hold_left == 0 {
                    self.state = EnvState::Decay;
                }
            }
            EnvState::Decay => {
                self.level += (self.sustain - self.level) * self.decay_coef;
                if (self.level - self.sustain).abs() < 1e-4 {
                    self.level = self.sustain;
                    self.state = EnvState::Sustain;
                }
            }
            EnvState::Sustain => self.level = self.sustain,
            EnvState::Release => {
                self.level -= self.level * self.release_coef;
                self.level = flush(self.level);
                if self.level < 1e-4 {
                    self.level = 0.0;
                    self.state = EnvState::Idle;
                }
            }
        }
        self.level
    }
}

/// Chris Huggett-inspired OTA multimode filter (Bass Station II / Peak architecture).
/// Provides 12 dB/oct (2-pole OTA state variable) and 24 dB/oct (4-pole cascaded OTA ladder
/// with non-linear saturation and authentic self-oscillation into a warm sine wave).
#[inline]
pub fn ota_sat(x: f32) -> f32 {
    x / (1.0 + 0.33 * x.abs())
}

#[inline]
pub fn analog_diode_sat(x: f32, drive: f32) -> f32 {
    let xin = x * drive;
    // Monotonic asymmetric transfer curve generating even harmonics (2nd and 4th)
    // with different positive/negative diode saturation thresholds without rectification.
    if xin >= 0.0 {
        xin / (1.0 + 0.75 * xin)
    } else {
        xin / (1.0 - 1.25 * xin)
    }
}

#[inline]
pub fn polyblep(t: f64, dt: f64) -> f32 {
    if t < dt {
        let d = t / dt;
        (2.0 * d - d * d - 1.0) as f32
    } else if t > 1.0 - dt {
        let d = (t - 1.0) / dt;
        (d * d + 2.0 * d + 1.0) as f32
    } else {
        0.0
    }
}

/// One-pole lowpass filter for analog smoothing in the distortion stage.
#[derive(Clone, Copy)]
pub struct OnePoleLp {
    z: f32,
    coeff: f32,
}

impl OnePoleLp {
    pub fn new(cutoff_hz: f32, sample_rate: f32) -> Self {
        let coeff =
            (1.0 - (-std::f32::consts::TAU * cutoff_hz / sample_rate).exp()).clamp(0.0, 1.0);
        Self { z: 0.0, coeff }
    }

    #[inline]
    pub fn tick(&mut self, input: f32) -> f32 {
        self.z += (input - self.z) * self.coeff;
        flush(self.z)
    }

    pub fn reset(&mut self) {
        self.z = 0.0;
    }
}

/// AC-coupling DC blocker emulating physical hardware output capacitors.
#[derive(Clone, Copy, Default)]
pub struct DcBlocker {
    x_prev: f32,
    y_prev: f32,
}

impl DcBlocker {
    #[inline]
    pub fn tick(&mut self, input: f32) -> f32 {
        let y = input - self.x_prev + 0.9985 * self.y_prev;
        self.x_prev = input;
        self.y_prev = flush(y);
        self.y_prev
    }

    pub fn reset(&mut self) {
        self.x_prev = 0.0;
        self.y_prev = 0.0;
    }
}

#[derive(Clone, Copy, Default)]
pub struct OtaFilter {
    // 2-pole OTA SVF states
    ic1: f32,
    ic2: f32,
    // 4-pole cascaded OTA ladder states
    s0: f32,
    s1: f32,
    s2: f32,
    s3: f32,
}

impl OtaFilter {
    pub fn reset(&mut self) {
        self.ic1 = 0.0;
        self.ic2 = 0.0;
        self.s0 = 0.0;
        self.s1 = 0.0;
        self.s2 = 0.0;
        self.s3 = 0.0;
    }

    #[inline]
    pub fn tick(
        &mut self,
        input: f32,
        cutoff_hz: f32,
        q: f32,
        mode: u8,
        poles: u8,
        sample_rate: f32,
    ) -> f32 {
        let nyquist = sample_rate * 0.49;
        let fc = cutoff_hz.clamp(12.0, nyquist);

        if poles == 1 {
            // 12 dB/oct: 2-pole OTA State Variable Filter
            let g = (std::f32::consts::PI * fc / sample_rate).tan();
            let damping = 2.0 - 1.95 * q.clamp(0.0, 0.999);
            let k = damping.max(0.02);
            let a1 = 1.0 / (1.0 + g * (g + k));
            let a2 = g * a1;
            let a3 = g * a2;

            let v3 = input - self.ic2;
            let v1 = a1 * self.ic1 + a2 * v3;
            let v1_sat = ota_sat(v1 * 1.2) / 1.2;
            let v2 = self.ic2 + a2 * self.ic1 + a3 * v3;
            self.ic1 = 2.0 * v1_sat - self.ic1;
            self.ic2 = 2.0 * v2 - self.ic2;
            self.ic1 = flush(self.ic1);
            self.ic2 = flush(self.ic2);

            let low = v2;
            let band = v1;
            let high = input - k * v1 - v2;
            match mode {
                0 => low,
                1 => band,
                _ => high,
            }
        } else {
            // 24 dB/oct: 4-pole cascaded OTA ladder with global non-linear feedback
            let w = (std::f32::consts::PI * fc / sample_rate).min(1.48);
            let g = w.tan();
            let big_g = g / (1.0 + g);

            // Feedback amount: reaches self-oscillation at q >= 0.95
            let k = 3.96 * q.clamp(0.0, 1.0);
            let fb = k * ota_sat(self.s3 * 1.4);
            let u = ota_sat(input * (1.0 + 0.3 * q) - fb);

            // Stage 0
            let v0 = big_g * (u - self.s0);
            let y0 = v0 + self.s0;
            self.s0 = y0 + v0;

            // Stage 1
            let v1 = big_g * (y0 - self.s1);
            let y1 = v1 + self.s1;
            self.s1 = y1 + v1;

            // Stage 2
            let v2 = big_g * (y1 - self.s2);
            let y2 = v2 + self.s2;
            self.s2 = y2 + v2;

            // Stage 3
            let v3 = big_g * (y2 - self.s3);
            let y3 = v3 + self.s3;
            self.s3 = y3 + v3;

            self.s0 = flush(self.s0);
            self.s1 = flush(self.s1);
            self.s2 = flush(self.s2);
            self.s3 = flush(self.s3);

            match mode {
                0 => y3,
                1 => 3.0 * (y1 - y3),
                _ => u - 4.0 * y0 + 6.0 * y1 - 4.0 * y2 + y3,
            }
        }
    }
}

/// Peak's panel LFOs are per voice and free-running unless key sync is on.
/// `elapsed` implements the fade-in control, which the panel exposes as FADE.
struct Lfo {
    phase: f32,
    value: f32,
    sample_hold: f32,
    rng: u32,
    elapsed: f32,
}

impl Lfo {
    fn new(seed: u32) -> Self {
        Self {
            phase: 0.0,
            value: 0.0,
            sample_hold: 0.0,
            rng: seed | 1,
            elapsed: 0.0,
        }
    }

    #[inline]
    fn random(&mut self) -> f32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 17;
        self.rng ^= self.rng << 5;
        (self.rng as f32 / u32::MAX as f32) * 2.0 - 1.0
    }

    /// A new voice starts at a free-running phase, which is what Peak does with
    /// KeySync off (the default).
    fn retrigger(&mut self, seed: u32) {
        self.rng = seed | 1;
        self.phase = (seed >> 8) as f32 / (u32::MAX >> 8) as f32;
        self.sample_hold = self.random();
        self.elapsed = 0.0;
    }

    #[inline]
    fn advance(&mut self, rate: f32, shape: u8, sample_rate: f32, fade_s: f32) -> f32 {
        self.elapsed += 1.0 / sample_rate;
        self.phase += rate / sample_rate;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
            self.sample_hold = self.random();
        }
        let p = self.phase;
        self.value = match shape {
            0 => (p * std::f32::consts::TAU).sin(),
            1 => {
                if p < 0.5 {
                    4.0 * p - 1.0
                } else {
                    3.0 - 4.0 * p
                }
            }
            2 => 2.0 * p - 1.0,
            3 => {
                if p < 0.5 {
                    1.0
                } else {
                    -1.0
                }
            }
            _ => self.sample_hold,
        };
        if fade_s > 0.0005 {
            self.value *= (self.elapsed / fade_s).min(1.0);
        }
        self.value
    }
}

struct Voice {
    note: u8,
    velocity: f32,
    gate: bool,
    age: u64,
    phase: [f64; 3],
    aux_phase1: [f64; 3],
    aux_phase2: [f64; 3],
    freq: f32,
    target_freq: f32,
    amp: Adsr,
    fenv: Adsr,
    filter: OtaFilter,
    lfo1: Lfo,
    lfo2: Lfo,
    drift_phase: f32,
    /// Static per-oscillator tuning offset, Peak's Diverge control (-1..1).
    diverge: [f32; 3],
    filter_diverge: f32,
    noise_lp: f32,
    rng: u32,
    level: f32,
}

impl Voice {
    fn new(sample_rate: f32, seed: u32) -> Self {
        Self {
            note: 0,
            velocity: 0.0,
            gate: false,
            age: 0,
            phase: [0.0; 3],
            aux_phase1: [0.0; 3],
            aux_phase2: [0.0; 3],
            freq: 0.0,
            target_freq: 0.0,
            amp: Adsr::new(sample_rate),
            fenv: Adsr::new(sample_rate),
            filter: OtaFilter::default(),
            lfo1: Lfo::new(seed ^ 0x9E37_79B9),
            lfo2: Lfo::new(seed ^ 0x85EB_CA6B),
            drift_phase: 0.0,
            diverge: [0.0; 3],
            filter_diverge: 0.0,
            noise_lp: 0.0,
            rng: seed | 1,
            level: 0.0,
        }
    }

    fn is_active(&self) -> bool {
        self.amp.is_active()
    }

    fn start(&mut self, note: u8, velocity: f32, age: u64, glide_freq: Option<f32>) {
        self.note = note;
        self.velocity = velocity;
        self.gate = true;
        self.age = age;
        self.amp.gate_on();
        self.fenv.gate_on();
        self.target_freq = midi_to_hz(note as f32);
        match glide_freq {
            Some(f) => self.freq = f,
            None => self.freq = self.target_freq,
        }
        self.filter.reset();
        self.noise_lp = 0.0;

        // Peak's NCOs free-run by default: notes do not all start at phase 0.
        let mut phases = [0.0f64; 3];
        let mut aux1 = [0.0f64; 3];
        let mut aux2 = [0.0f64; 3];
        for o in 0..3 {
            phases[o] = self.next_unit() as f64;
            aux1[o] = self.next_unit() as f64;
            aux2[o] = self.next_unit() as f64;
        }
        self.phase = phases;
        self.aux_phase1 = aux1;
        self.aux_phase2 = aux2;
        self.drift_phase = self.next_unit();
        let mut diverge = [0.0f32; 3];
        for d in &mut diverge {
            *d = self.next_unit() * 2.0 - 1.0;
        }
        self.diverge = diverge;
        self.filter_diverge = self.next_unit() * 2.0 - 1.0;
        let seed = self.next_u32();
        self.lfo1.retrigger(seed);
        self.lfo2.retrigger(seed.rotate_left(11));
        self.level = 0.0;
    }

    fn release(&mut self) {
        self.gate = false;
        self.amp.gate_off();
        self.fenv.gate_off();
    }

    #[inline]
    fn next_u32(&mut self) -> u32 {
        // xorshift32: cheap, good enough for a noise source.
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 17;
        self.rng ^= self.rng << 5;
        self.rng
    }

    #[inline]
    fn next_unit(&mut self) -> f32 {
        (self.next_u32() as f32 / u32::MAX as f32).clamp(0.0, 1.0)
    }

    #[inline]
    fn noise(&mut self, coeff: f32) -> f32 {
        let raw = self.next_unit() * 2.0 - 1.0;
        self.noise_lp += (raw - self.noise_lp) * coeff;
        self.noise_lp
    }
}

/// Everything the FX chain needs, preallocated for the sample rate.
///
/// Each stage keeps independent left/right state: a mono buffer shared by both
/// channels would smear the stereo image and double the modulation rate.
struct Fx {
    sample_rate: f32,
    chorus: [Vec<f32>; 2],
    chorus_pos: usize,
    chorus_phase: f32,
    delay: [Vec<f32>; 2],
    delay_pos: usize,
    damp: [f32; 2],
    combs: [[Vec<f32>; 4]; 2],
    comb_pos: [[usize; 4]; 2],
    allpass: [[Vec<f32>; 2]; 2],
    allpass_pos: [[usize; 2]; 2],
    reverb_lp: [f32; 2],
}

impl Fx {
    fn new(sample_rate: f32) -> Self {
        let chorus_len = ((sample_rate * 0.05) as usize).max(16);
        let max_delay = ((sample_rate * 1.5) as usize).max(16);
        let comb_len = |ms: f32| ((sample_rate * ms / 1000.0) as usize).max(8);
        // Right-channel delays are offset so the two sides decorrelate.
        let offset = ((sample_rate * 0.0013) as usize).max(3);
        let mk_combs = |off: usize| {
            [
                vec![0.0; comb_len(29.7) + off],
                vec![0.0; comb_len(37.1) + off],
                vec![0.0; comb_len(41.1) + off],
                vec![0.0; comb_len(43.7) + off],
            ]
        };
        Self {
            sample_rate,
            chorus: [vec![0.0; chorus_len], vec![0.0; chorus_len]],
            chorus_pos: 0,
            chorus_phase: 0.0,
            delay: [vec![0.0; max_delay], vec![0.0; max_delay]],
            delay_pos: 0,
            damp: [0.0; 2],
            combs: [mk_combs(0), mk_combs(offset)],
            comb_pos: [[0; 4]; 2],
            allpass: [
                [vec![0.0; comb_len(5.0)], vec![0.0; comb_len(1.7)]],
                [
                    vec![0.0; comb_len(5.0) + offset],
                    vec![0.0; comb_len(1.7) + offset],
                ],
            ],
            allpass_pos: [[0; 2]; 2],
            reverb_lp: [0.0; 2],
        }
    }

    fn reset(&mut self) {
        for ch in 0..2 {
            self.chorus[ch].iter_mut().for_each(|s| *s = 0.0);
            self.delay[ch].iter_mut().for_each(|s| *s = 0.0);
            for c in self.combs[ch].iter_mut() {
                c.iter_mut().for_each(|s| *s = 0.0);
            }
            for a in self.allpass[ch].iter_mut() {
                a.iter_mut().for_each(|s| *s = 0.0);
            }
        }
        self.damp = [0.0; 2];
        self.reverb_lp = [0.0; 2];
    }

    /// Advance the shared chorus LFO once per frame, not once per channel.
    #[inline]
    fn advance_chorus(&mut self) {
        self.chorus_phase += 0.31 / self.sample_rate;
        if self.chorus_phase >= 1.0 {
            self.chorus_phase -= 1.0;
        }
    }

    #[inline]
    fn chorus(&mut self, channel: usize, input: f32, depth: f32) -> f32 {
        if depth <= 1e-4 {
            return input;
        }
        let phase = (self.chorus_phase + channel as f32 * 0.25).rem_euclid(1.0);
        let modulation = (phase * std::f32::consts::TAU).sin();
        let len = self.chorus[channel].len();
        let pos = self.chorus_pos;
        self.chorus[channel][pos] = input;

        let delay = self.sample_rate * (0.012 + 0.004 * modulation);
        let mut p = pos as f32 - delay;
        while p < 0.0 {
            p += len as f32;
        }
        let i0 = p as usize % len;
        let i1 = (i0 + 1) % len;
        let frac = p - p.floor();
        let tapped =
            self.chorus[channel][i0] + (self.chorus[channel][i1] - self.chorus[channel][i0]) * frac;
        // Advance the write head once, after both channels have written.
        if channel == 1 {
            self.chorus_pos = (pos + 1) % len;
        }
        input + tapped * 0.5 * depth
    }

    #[inline]
    fn delay_line(
        &mut self,
        channel: usize,
        input: f32,
        time_s: f32,
        feedback: f32,
        mix: f32,
    ) -> f32 {
        if mix <= 1e-4 && feedback <= 1e-4 {
            return input;
        }
        let len = self.delay[channel].len();
        let d = ((time_s * self.sample_rate) as usize).clamp(1, len - 1);
        let read_pos = (self.delay_pos + len - d) % len;
        let delayed = self.delay[channel][read_pos];
        // Damp the repeats so feedback does not turn into a whistle.
        self.damp[channel] += (delayed - self.damp[channel]) * 0.35;
        self.delay[channel][self.delay_pos] = input + flush(self.damp[channel]) * feedback;
        if channel == 1 {
            self.delay_pos = (self.delay_pos + 1) % len;
        }
        input + delayed * mix
    }

    #[inline]
    fn reverb(&mut self, channel: usize, input: f32, size: f32, mix: f32) -> f32 {
        if mix <= 1e-4 {
            return input;
        }
        let fb = 0.70 + 0.27 * size.clamp(0.0, 1.0);
        let mut acc = 0.0;
        for i in 0..4 {
            let len = self.combs[channel][i].len();
            let pos = self.comb_pos[channel][i] % len;
            let out = self.combs[channel][i][pos];
            self.combs[channel][i][pos] = flush(input + out * fb);
            self.comb_pos[channel][i] = (pos + 1) % len;
            acc += out;
        }
        acc *= 0.25;

        for i in 0..2 {
            let len = self.allpass[channel][i].len();
            let pos = self.allpass_pos[channel][i] % len;
            let buf_out = self.allpass[channel][i][pos];
            self.allpass[channel][i][pos] = flush(acc + buf_out * 0.5);
            self.allpass_pos[channel][i] = (pos + 1) % len;
            acc = buf_out - acc * 0.5;
        }

        let lp = if channel == 0 { 0.55 } else { 0.45 };
        self.reverb_lp[channel] += (acc - self.reverb_lp[channel]) * lp;
        input + flush(self.reverb_lp[channel]) * mix
    }
}

/// Per-block, precomputed voice settings, so the inner loop stays branch-light.
#[derive(Clone, Copy)]
struct Voicing {
    osc_level: [f32; 3],
    osc_wave: [usize; 3],
    /// Bipolar Peak SHAPE amount for each oscillator, -1..=1.
    osc_shape: [f32; 3],
    osc_ratio: [f32; 3],
    noise: f32,
    ring: f32,
    cutoff: f32,
    reso: f32,
    drive: f32,
    env_amt: f32,
    keytrack: f32,
    filter_mode: u8,
    filter_poles: u8,
    osc3_filter_mod: f32,
    fm12: f32,
    fm23: f32,
    drift: f32,
    diverge: f32,
    glide_time: f32,
    bend_semitones: f32,
    volume: f32,
}

pub struct Engine {
    sample_rate: f32,
    bank: Option<Arc<Bank>>,
    voices: Vec<Voice>,
    fx: Fx,
    age: u64,
    pitch_bend: f32,
    bend_range: f32,
    peak: f32,
    /// Number of voices that produced sound in the last block, for the UI meter.
    active_voices: usize,
    /// Portamento time from the last block, seconds.
    glide_time: f32,
    /// Anti-zipper smoothing for the two controls that are audible per sample.
    cutoff_smooth: Smoothed,
    gain_smooth: Smoothed,
    dist_filter_l: OnePoleLp,
    dist_filter_r: OnePoleLp,
    dc_block_l: DcBlocker,
    dc_block_r: DcBlocker,
}

impl Engine {
    pub fn new(sample_rate: f32) -> Self {
        let sr = sample_rate.max(8_000.0);
        let mut cutoff_smooth = Smoothed::new(6_400.0);
        cutoff_smooth.set_time(0.004, sr);
        let mut gain_smooth = Smoothed::new(0.5);
        gain_smooth.set_time(0.010, sr);
        let dist_filter_l = OnePoleLp::new(14_000.0, sr);
        let dist_filter_r = OnePoleLp::new(14_000.0, sr);
        Self {
            sample_rate: sr,
            bank: None,
            voices: (0..MAX_VOICES)
                .map(|i| Voice::new(sr, 0x1234_5678 ^ (i as u32).wrapping_mul(0x9E37_79B9)))
                .collect(),
            fx: Fx::new(sr),
            age: 0,
            pitch_bend: 0.0,
            bend_range: 2.0,
            peak: 0.0,
            active_voices: 0,
            glide_time: 0.0,
            cutoff_smooth,
            gain_smooth,
            dist_filter_l,
            dist_filter_r,
            dc_block_l: DcBlocker::default(),
            dc_block_r: DcBlocker::default(),
        }
    }

    pub fn sample_rate(&self) -> f32 {
        self.sample_rate
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        let sr = sample_rate.max(8_000.0);
        if (sr - self.sample_rate).abs() < 0.5 {
            return;
        }
        self.sample_rate = sr;
        for (i, v) in self.voices.iter_mut().enumerate() {
            *v = Voice::new(sr, 0x1234_5678 ^ (i as u32).wrapping_mul(0x9E37_79B9));
        }
        self.fx = Fx::new(sr);
        self.dist_filter_l = OnePoleLp::new(14_000.0, sr);
        self.dist_filter_r = OnePoleLp::new(14_000.0, sr);
        self.dc_block_l = DcBlocker::default();
        self.dc_block_r = DcBlocker::default();
        self.peak = 0.0;
    }

    /// Install the bank derived from a validated firmware image.
    pub fn set_bank(&mut self, bank: Arc<Bank>) {
        self.bank = Some(bank);
        self.reset();
    }

    pub fn has_bank(&self) -> bool {
        self.bank.is_some()
    }

    pub fn bank(&self) -> Option<&Arc<Bank>> {
        self.bank.as_ref()
    }

    pub fn reset(&mut self) {
        for v in self.voices.iter_mut() {
            v.amp.kill();
            v.fenv.kill();
            v.gate = false;
            v.filter.reset();
        }
        self.fx.reset();
        self.dist_filter_l.reset();
        self.dist_filter_r.reset();
        self.dc_block_l.reset();
        self.dc_block_r.reset();
        self.peak = 0.0;
        self.gain_smooth.snap(0.0);
    }

    pub fn peak(&self) -> f32 {
        self.peak
    }

    pub fn active_voices(&self) -> usize {
        self.active_voices
    }

    fn note_on(&mut self, note: u8, velocity: f32) {
        let patch_glide = self.glide_time;
        let age = self.age;
        self.age = self.age.wrapping_add(1);

        // A retriggered note reuses its voice so legato playing does not eat
        // polyphony.
        if let Some(v) = self
            .voices
            .iter_mut()
            .find(|v| v.note == note && v.is_active())
        {
            let glide = if patch_glide > 0.0005 {
                Some(v.freq)
            } else {
                None
            };
            v.start(note, velocity, age, glide);
            return;
        }

        let index = self.voices.iter().position(|v| !v.is_active()).or_else(|| {
            // Steal the oldest released voice first, then the oldest voice.
            let released = self
                .voices
                .iter()
                .filter(|v| !v.gate)
                .min_by_key(|v| v.age)
                .map(|v| v.age);
            match released {
                Some(age) => self.voices.iter().position(|v| !v.gate && v.age == age),
                None => self
                    .voices
                    .iter()
                    .enumerate()
                    .min_by_key(|(_, v)| v.age)
                    .map(|(i, _)| i),
            }
        });

        if let Some(i) = index {
            let glide = if patch_glide > 0.0005 && self.voices[i].freq > 0.0 {
                Some(self.voices[i].freq)
            } else {
                None
            };
            self.voices[i].start(note, velocity, age, glide);
        }
    }

    fn note_off(&mut self, note: u8) {
        for v in self.voices.iter_mut() {
            if v.note == note && v.gate {
                v.release();
            }
        }
    }
}

impl Engine {
    /// Render one stereo block. `left` and `right` must be the same length.
    ///
    /// Without a linked firmware the engine is silent by design: the gate is
    /// enforced here, not only in the UI.
    pub fn process(
        &mut self,
        patch: &Patch,
        events: &[Event],
        left: &mut [f32],
        right: &mut [f32],
    ) {
        let frames = left.len().min(right.len());
        if frames == 0 {
            return;
        }

        let bank = match &self.bank {
            Some(b) => b.clone(),
            None => {
                left[..frames].iter_mut().for_each(|s| *s = 0.0);
                right[..frames].iter_mut().for_each(|s| *s = 0.0);
                self.peak = 0.0;
                self.active_voices = 0;
                return;
            }
        };

        self.glide_time = patch.get(P::Glide) / 1000.0;
        self.bend_range = patch.get(P::BendRange);

        for event in events {
            match *event {
                Event::NoteOn { note, velocity } => self.note_on(note, velocity.clamp(0.0, 1.0)),
                Event::NoteOff { note } => self.note_off(note),
                Event::AllNotesOff => {
                    for v in self.voices.iter_mut() {
                        v.release();
                    }
                }
                Event::PitchBend { normalized } => {
                    self.pitch_bend = normalized.clamp(-1.0, 1.0);
                }
            }
        }

        let poly = patch
            .get(P::Polyphony)
            .round()
            .clamp(1.0, MAX_VOICES as f32) as usize;
        self.voices.iter_mut().skip(poly).for_each(|v| {
            if v.is_active() {
                v.amp.kill();
                v.fenv.kill();
                v.gate = false;
            }
        });

        let filter_type = patch.get(P::FilterType).round().clamp(0.0, 5.0) as u8;
        let (filter_mode, filter_poles) = match filter_type {
            0 => (0u8, 1u8), // LP 12
            1 => (0, 2),     // LP 24
            2 => (1, 1),     // BP 12
            3 => (2, 2),     // HP 24 (kept at index 3 for v1.0.0 presets)
            4 => (1, 2),     // BP 24
            _ => (2, 1),     // HP 12
        };

        let voicing = Voicing {
            osc_level: [
                patch.get(P::Osc1Level),
                patch.get(P::Osc2Level),
                patch.get(P::Osc3Level),
            ],
            osc_wave: [
                patch.get(P::Osc1Shape).round().clamp(0.0, 11.0) as usize,
                patch.get(P::Osc2Shape).round().clamp(0.0, 11.0) as usize,
                patch.get(P::Osc3Shape).round().clamp(0.0, 11.0) as usize,
            ],
            osc_shape: [
                patch.get(P::Osc1ShapeAmt),
                patch.get(P::Osc2ShapeAmt),
                patch.get(P::Osc3ShapeAmt),
            ],
            osc_ratio: [
                (patch.get(P::Osc1Tune) + patch.get(P::Osc1Fine) / 100.0).exp2(),
                (patch.get(P::Osc2Tune) + patch.get(P::Osc2Fine) / 100.0).exp2(),
                (patch.get(P::Osc3Tune) + patch.get(P::Osc3Fine) / 100.0).exp2(),
            ],
            noise: patch.get(P::NoiseLevel),
            ring: patch.get(P::RingLevel),
            cutoff: patch.get(P::Cutoff),
            reso: patch.get(P::Reso),
            drive: patch.get(P::Drive),
            env_amt: patch.get(P::FilterEnvAmt),
            keytrack: patch.get(P::Keytrack),
            filter_mode,
            filter_poles,
            osc3_filter_mod: patch.get(P::Osc3FilterMod),
            fm12: patch.get(P::Fm12),
            fm23: patch.get(P::Fm23),
            drift: patch.get(P::Drift),
            diverge: patch.get(P::Diverge),
            glide_time: self.glide_time,
            bend_semitones: self.pitch_bend * self.bend_range,
            volume: patch.get(P::Volume),
        };

        let lfo1_rate = patch.get(P::Lfo1Rate);
        let lfo1_shape = patch.get(P::Lfo1Shape).round() as u8;
        let lfo1_depth = patch.get(P::Lfo1Depth);
        let lfo1_dest = patch.get(P::Lfo1Dest).round() as u8;
        let lfo1_fade = patch.get(P::Lfo1Fade) / 1000.0;
        let lfo2_rate = patch.get(P::Lfo2Rate);
        let lfo2_shape = patch.get(P::Lfo2Shape).round() as u8;
        let lfo2_depth = patch.get(P::Lfo2Depth);
        let lfo2_dest = patch.get(P::Lfo2Dest).round() as u8;
        let lfo2_fade = patch.get(P::Lfo2Fade) / 1000.0;

        // LFO 1 and 2 each drive one destination; two LFOs on the same
        // destination add up.
        let routes = [
            (0usize, lfo1_dest, lfo1_depth),
            (1usize, lfo2_dest, lfo2_depth),
        ];

        let amp_env = [
            patch.get(P::AmpAttack) / 1000.0,
            patch.get(P::AmpHold) / 1000.0,
            patch.get(P::AmpDecay) / 1000.0,
            patch.get(P::AmpSustain),
            patch.get(P::AmpRelease) / 1000.0,
        ];
        let filt_env = [
            patch.get(P::FilterAttack) / 1000.0,
            patch.get(P::FilterHold) / 1000.0,
            patch.get(P::FilterDecay) / 1000.0,
            patch.get(P::FilterSustain),
            patch.get(P::FilterRelease) / 1000.0,
        ];
        for v in self.voices.iter_mut() {
            v.amp
                .set_times(amp_env[0], amp_env[1], amp_env[2], amp_env[3], amp_env[4]);
            v.fenv.set_times(
                filt_env[0],
                filt_env[1],
                filt_env[2],
                filt_env[3],
                filt_env[4],
            );
        }

        let sr = self.sample_rate;
        let glide_coeff = if voicing.glide_time > 0.0005 {
            (1.0 - (-1.0 / (voicing.glide_time * sr)).exp()).clamp(0.0, 1.0)
        } else {
            1.0
        };

        let master = 10.0f32.powf(voicing.volume / 20.0) * 0.25;
        self.cutoff_smooth.set_target(voicing.cutoff);
        self.gain_smooth.set_target(master);
        let drive_gain = 1.0 + voicing.drive * 9.0;
        let drive_comp = 1.0 / (1.0 + voicing.drive * 3.0);
        let chorus_mix = patch.get(P::Chorus);
        let delay_time = patch.get(P::DelayTime) / 1000.0;
        let delay_fb = patch.get(P::DelayFeedback);
        let delay_mix = patch.get(P::DelayMix);
        let reverb_size = patch.get(P::ReverbSize);
        let reverb_mix = patch.get(P::ReverbMix);
        let dist = patch.get(P::Dist);
        let dist_gain = 1.0 + dist * 12.0;
        let noise_cut = 80.0 * 250.0f32.powf(patch.get(P::NoiseTone).clamp(0.0, 1.0));
        let noise_coeff =
            (1.0 - (-std::f32::consts::TAU * noise_cut / sr).exp()).clamp(0.0005, 1.0);

        let mut peak = 0.0f32;

        for i in 0..frames {
            // One smoother step per sample, shared by every voice.
            let base_cutoff = self.cutoff_smooth.next();
            let mut mix_l = 0.0f32;
            let mut mix_r = 0.0f32;

            for v in self.voices.iter_mut() {
                if !v.is_active() {
                    continue;
                }
                // Glide toward the target pitch.
                if (v.freq - v.target_freq).abs() > 1e-3 {
                    v.freq += (v.target_freq - v.freq) * glide_coeff;
                }

                let amp_level = v.amp.next();
                let fenv_level = v.fenv.next();
                if !v.is_active() {
                    continue;
                }

                // Peak's panel LFOs live per voice, so a chord does not move in
                // lockstep. Each voice applies its own modulation sums.
                let lfo1 = v.lfo1.advance(lfo1_rate, lfo1_shape, sr, lfo1_fade);
                let lfo2 = v.lfo2.advance(lfo2_rate, lfo2_shape, sr, lfo2_fade);
                let mut pitch_mod = 0.0;
                let mut cutoff_mod = 0.0;
                let mut shape_mod = 0.0;
                let mut amp_mod = 0.0;
                for (index, dest, depth) in routes {
                    let value = (if index == 0 { lfo1 } else { lfo2 }) * depth;
                    match dest {
                        0 => pitch_mod += value,
                        1 => cutoff_mod += value,
                        2 => shape_mod += value,
                        _ => amp_mod += value,
                    }
                }

                let bend_ratio = (voicing.bend_semitones / 12.0 + pitch_mod * 12.0 / 12.0).exp2();
                let base = v.freq * bend_ratio;
                let nyquist = sr * 0.45;

                // A very slow meander (Peak's Drift) plus a fixed per-oscillator
                // offset (Diverge) is what keeps the NCOs from sounding sterile.
                v.drift_phase += 0.09 / sr;
                if v.drift_phase >= 1.0 {
                    v.drift_phase -= 1.0;
                }
                let drift = (std::f32::consts::TAU * v.drift_phase).sin() * 0.62
                    + (std::f32::consts::TAU * v.drift_phase * 2.7).sin() * 0.38;
                let drift_ratio = (drift * voicing.drift * 12.0 / 1200.0).exp2();

                let mut osc = [0.0f32; 3];
                for o in 0..3 {
                    let diverge_ratio = (v.diverge[o] * voicing.diverge * 8.0 / 1200.0).exp2();
                    let f = base * voicing.osc_ratio[o] * drift_ratio * diverge_ratio;
                    if f >= nyquist {
                        continue;
                    }
                    let dt = (f / sr) as f64;
                    v.phase[o] += dt;
                    if v.phase[o] >= 1.0 {
                        v.phase[o] -= 1.0;
                    }
                    let shape = (voicing.osc_shape[o] + shape_mod).clamp(-1.0, 1.0);

                    // Phase modulation is Peak's FM O1>O2 / O2>O3 routing.
                    let mut read_phase = v.phase[o];
                    if o == 1 {
                        read_phase += (osc[0] * voicing.fm12 * 0.75) as f64;
                    } else if o == 2 {
                        read_phase += (osc[1] * voicing.fm23 * 0.75) as f64;
                    }
                    read_phase = read_phase.rem_euclid(1.0);

                    let wave_type = voicing.osc_wave[o];
                    osc[o] = match wave_type {
                        0 => {
                            let raw = (read_phase as f32 * std::f32::consts::TAU).sin();
                            if shape.abs() > 1e-4 {
                                analog_diode_sat(raw, 1.0 + shape.abs() * 3.5)
                            } else {
                                raw
                            }
                        }
                        1 => {
                            let p = read_phase as f32;
                            let raw = if p < 0.5 {
                                4.0 * p - 1.0
                            } else {
                                3.0 - 4.0 * p
                            };
                            if shape.abs() > 1e-4 {
                                let folded =
                                    (raw * (1.0 + shape.abs() * 2.0) * std::f32::consts::FRAC_PI_2)
                                        .sin();
                                analog_diode_sat(folded, 1.0 + shape.abs() * 1.5)
                            } else {
                                raw
                            }
                        }
                        2 => {
                            let p = read_phase;
                            let saw_center = (2.0 * p - 1.0) as f32 - polyblep(p, dt);
                            if shape > 0.005 {
                                let detune_cents = 35.0 * shape;
                                let ratio_side1 = (detune_cents / 1200.0).exp2() as f64;
                                let ratio_side2 = (-detune_cents / 1200.0).exp2() as f64;
                                let dt1 = dt * ratio_side1;
                                let dt2 = dt * ratio_side2;

                                v.aux_phase1[o] += dt1;
                                if v.aux_phase1[o] >= 1.0 {
                                    v.aux_phase1[o] -= 1.0;
                                }
                                v.aux_phase2[o] += dt2;
                                if v.aux_phase2[o] >= 1.0 {
                                    v.aux_phase2[o] -= 1.0;
                                }

                                let saw_side1 = (2.0 * v.aux_phase1[o] - 1.0) as f32
                                    - polyblep(v.aux_phase1[o], dt1);
                                let saw_side2 = (2.0 * v.aux_phase2[o] - 1.0) as f32
                                    - polyblep(v.aux_phase2[o], dt2);

                                let center_gain = 1.0 - 0.35 * shape;
                                let side_gain = 0.5 * shape;
                                (saw_center * center_gain + (saw_side1 + saw_side2) * side_gain)
                                    / (1.0 + 0.35 * shape)
                            } else if shape < -0.005 {
                                let s = -shape;
                                let dual = (4.0 * ((p * 2.0).fract()) - 2.0) as f32 * 0.5;
                                saw_center * (1.0 - s * 0.6) + dual * (s * 0.6)
                            } else {
                                saw_center
                            }
                        }
                        3 => {
                            let duty = (0.5 + 0.46 * shape).clamp(0.03, 0.97) as f64;
                            let naive = if read_phase < duty { 1.0_f32 } else { -1.0_f32 };
                            let blep1 = polyblep(read_phase, dt);
                            let phase_d = (read_phase - duty).rem_euclid(1.0);
                            let blep2 = polyblep(phase_d, dt);
                            naive + blep1 - blep2
                        }
                        _ => {
                            let level = Bank::level_for(sr, f);
                            bank.sample_shape(wave_type, shape, level, read_phase as f32)
                        }
                    };
                }

                let mut signal = osc[0] * voicing.osc_level[0]
                    + osc[1] * voicing.osc_level[1]
                    + osc[2] * voicing.osc_level[2];

                if voicing.ring > 1e-4 {
                    signal += osc[0] * osc[1] * voicing.ring;
                }
                if voicing.noise > 1e-4 {
                    signal += v.noise(noise_coeff) * voicing.noise;
                }

                // Pre-filter overdrive, modeled after Peak's analogue diode saturation
                let signal_driven = if voicing.drive > 1e-4 {
                    analog_diode_sat(signal, drive_gain) * drive_comp
                } else {
                    signal
                };

                // Filter: envelope, keytrack, LFO and audio-rate Osc 3 mod.
                let key_offset = (v.note as f32 - 60.0) / 12.0 * voicing.keytrack;
                let env_octaves = voicing.env_amt * fenv_level * 5.0;
                let lfo_octaves = cutoff_mod * 2.0;
                let osc3_octaves = osc[2] * voicing.osc3_filter_mod * 3.5;
                let diverge_offset = v.filter_diverge * voicing.diverge * 0.03;
                let cutoff = (base_cutoff
                    * (key_offset + env_octaves + lfo_octaves + osc3_octaves + diverge_offset)
                        .exp2())
                .clamp(15.0, nyquist);

                let filtered = v.filter.tick(
                    signal_driven,
                    cutoff,
                    voicing.reso,
                    voicing.filter_mode,
                    voicing.filter_poles,
                    sr,
                );

                // Post-filter drive (Peak's analogue post-filter saturation)
                let post_driven = if voicing.drive > 1e-4 {
                    analog_diode_sat(filtered, 1.0 + voicing.drive * 2.2)
                } else {
                    filtered
                };

                let amp = amp_level * v.velocity * (1.0 + amp_mod).clamp(0.0, 2.0);
                let out = post_driven * amp;
                // Peak pans alternate voices slightly for width.
                let pan = if v.note % 2 == 0 { 0.47 } else { 0.53 };
                mix_l += out * (1.4 - pan);
                mix_r += out * pan;
                v.level = out;
            }

            // Serial FX chain.
            let mut l = mix_l;
            let mut r = mix_r;
            if dist > 1e-4 {
                let sat_l = analog_diode_sat(l, dist_gain) / (1.0 + dist * 0.9);
                let sat_r = analog_diode_sat(r, dist_gain) / (1.0 + dist * 0.9);
                l = self.dist_filter_l.tick(sat_l);
                r = self.dist_filter_r.tick(sat_r);
            }
            if chorus_mix > 1e-4 {
                self.fx.advance_chorus();
                l = self.fx.chorus(0, l, chorus_mix);
                r = self.fx.chorus(1, r, chorus_mix);
            }
            if delay_mix > 1e-4 || delay_fb > 1e-4 {
                l = self.fx.delay_line(0, l, delay_time, delay_fb, delay_mix);
                r = self.fx.delay_line(1, r, delay_time, delay_fb, delay_mix);
            }
            if reverb_mix > 1e-4 {
                l = self.fx.reverb(0, l, reverb_size, reverb_mix);
                r = self.fx.reverb(1, r, reverb_size, reverb_mix);
            }

            let l_dc = self.dc_block_l.tick(l);
            let r_dc = self.dc_block_r.tick(r);
            let gain = self.gain_smooth.next();
            let l = (l_dc * gain).clamp(-1.2, 1.2);
            let r = (r_dc * gain).clamp(-1.2, 1.2);
            left[i] = l;
            right[i] = r;
            peak = peak.max(l.abs()).max(r.abs());
        }

        // Any voice whose amp envelope died is free again.
        self.peak = peak;
        self.active_voices = self.voices.iter().filter(|v| v.is_active()).count();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::params::Patch;
    use std::time::Instant;

    fn block(engine: &mut Engine, patch: &Patch, frames: usize) -> (Vec<f32>, Vec<f32>) {
        let mut l = vec![0.0; frames];
        let mut r = vec![0.0; frames];
        engine.process(patch, &[], &mut l, &mut r);
        (l, r)
    }

    fn rms(buf: &[f32]) -> f32 {
        (buf.iter().map(|s| s * s).sum::<f32>() / buf.len() as f32).sqrt()
    }

    #[test]
    fn silent_without_firmware_bank() {
        let mut engine = Engine::new(48_000.0);
        let patch = Patch::default();
        engine.process(
            &patch,
            &[Event::NoteOn {
                note: 60,
                velocity: 1.0,
            }],
            &mut vec![0.0; 512],
            &mut vec![0.0; 512],
        );
        let (l, r) = block(&mut engine, &patch, 4096);
        assert_eq!(rms(&l), 0.0);
        assert_eq!(rms(&r), 0.0);
        assert!(!engine.has_bank());
    }

    #[test]
    fn note_on_produces_sound() {
        let mut engine = Engine::new(48_000.0);
        engine.set_bank(Bank::generate(1));
        let patch = Patch::default();
        engine.process(
            &patch,
            &[Event::NoteOn {
                note: 60,
                velocity: 1.0,
            }],
            &mut vec![0.0; 64],
            &mut vec![0.0; 64],
        );
        let (l, r) = block(&mut engine, &patch, 8192);
        assert!(rms(&l) > 0.01, "rms {}", rms(&l));
        assert!(rms(&r) > 0.01);
        assert!(engine.active_voices() >= 1);
    }

    #[test]
    fn note_off_releases_to_silence() {
        let mut engine = Engine::new(48_000.0);
        engine.set_bank(Bank::generate(1));
        let mut patch = Patch::default();
        patch.set(P::AmpRelease, 30.0);
        for fx in [
            P::ReverbMix,
            P::DelayMix,
            P::Chorus,
            P::Dist,
            P::DelayFeedback,
        ] {
            patch.set(fx, 0.0);
        }
        engine.process(
            &patch,
            &[Event::NoteOn {
                note: 60,
                velocity: 1.0,
            }],
            &mut vec![0.0; 64],
            &mut vec![0.0; 64],
        );
        let _ = block(&mut engine, &patch, 4096);
        engine.process(
            &patch,
            &[Event::NoteOff { note: 60 }],
            &mut vec![0.0; 64],
            &mut vec![0.0; 64],
        );
        let (l, _) = block(&mut engine, &patch, 24_000);
        let tail = &l[l.len() - 2048..];
        assert!(rms(tail) < 1e-5, "tail rms {}", rms(tail));
        assert_eq!(engine.active_voices(), 0);
    }

    #[test]
    fn polyphony_limit_is_enforced() {
        let mut engine = Engine::new(48_000.0);
        engine.set_bank(Bank::generate(1));
        let mut patch = Patch::default();
        patch.set(P::Polyphony, 2.0);
        for note in 60..68 {
            engine.process(
                &patch,
                &[Event::NoteOn {
                    note,
                    velocity: 1.0,
                }],
                &mut [0.0; 32],
                &mut [0.0; 32],
            );
        }
        let _ = block(&mut engine, &patch, 2048);
        assert!(engine.active_voices() <= 2, "{}", engine.active_voices());
    }

    #[test]
    fn peakdist03_presets_render_finite_distorted_audio() {
        let mut engine = Engine::new(48_000.0);
        engine.set_bank(crate::wavetable::Bank::generate(42));
        for syx in crate::preset::PEAK_DIST_03_SYX_ALL {
            let (name, patch) = crate::preset::parse_syx_patch(syx).expect("syx parse");
            let mut l = vec![0.0; 1024];
            let mut r = vec![0.0; 1024];
            engine.process(
                &patch,
                &[Event::NoteOn {
                    note: 60,
                    velocity: 0.9,
                }],
                &mut l,
                &mut r,
            );
            let energy = rms(&l) + rms(&r);
            assert!(
                energy > 0.001,
                "{}: expected audio output, got rms {}",
                name,
                energy
            );
            for s in l.iter().chain(r.iter()) {
                assert!(s.is_finite(), "{}: sample is not finite", name);
            }
        }
    }

    #[test]
    fn sound_by_sound_cycle_acoustic_analysis() {
        let mut engine = Engine::new(48_000.0);
        engine.set_bank(crate::wavetable::Bank::generate(777));

        for syx in crate::preset::PEAK_DIST_03_SYX_ALL {
            engine.reset();
            let (name, patch) = crate::preset::parse_syx_patch(syx).expect("syx parse");

            // Warm-up 2048 samples for attack and filter settle
            let mut l_warm = vec![0.0; 2048];
            let mut r_warm = vec![0.0; 2048];
            engine.process(
                &patch,
                &[Event::NoteOn {
                    note: 60,
                    velocity: 0.95,
                }],
                &mut l_warm,
                &mut r_warm,
            );

            // Render 4096 samples of sustained tone
            let mut l = vec![0.0; 4096];
            let mut r = vec![0.0; 4096];
            engine.process(&patch, &[], &mut l, &mut r);

            let l_rms = rms(&l);
            let r_rms = rms(&r);
            assert!(
                l_rms > 0.005 && r_rms > 0.005,
                "{name}: audio energy too low (L: {l_rms:.4}, R: {r_rms:.4})"
            );

            // Verify bounded samples and absence of non-finite values
            for s in l.iter().chain(r.iter()) {
                assert!(s.is_finite(), "{name}: non-finite sample detected");
                assert!(s.abs() <= 1.25, "{name}: sample out of bounds: {s}");
            }

            // Verify that audio waveform is rich in dynamics and not static silence
            let mut zero_crossings = 0;
            for w in l.windows(2) {
                if (w[0] <= 0.0 && w[1] > 0.0) || (w[0] >= 0.0 && w[1] < 0.0) {
                    zero_crossings += 1;
                }
            }
            assert!(
                zero_crossings >= 4,
                "{name}: insufficient zero crossings ({zero_crossings})"
            );
        }
    }

    #[test]
    fn ota_ladder_self_oscillates_at_high_resonance() {
        let mut filter = OtaFilter::default();
        let sr = 48_000.0;
        let fc = 1000.0;
        let q = 1.0; // Self-oscillation

        // Excite with a single impulse
        let mut out = Vec::with_capacity(4096);
        for i in 0..4096 {
            let inp = if i == 0 { 0.5 } else { 0.0 };
            let s = filter.tick(inp, fc, q, 0, 2, sr);
            out.push(s);
        }

        // The tail (last 1024 samples) must maintain self-oscillation
        let tail = &out[out.len() - 1024..];
        let tail_rms = rms(tail);
        assert!(
            tail_rms > 0.05,
            "4-pole OTA filter failed to sustain self-oscillation (rms: {tail_rms})"
        );
        for s in tail {
            assert!(s.is_finite() && s.abs() <= 1.5);
        }
    }

    #[test]
    fn oxford_triple_saw_enriches_sawtooth_density() {
        let mut engine = Engine::new(48_000.0);
        engine.set_bank(crate::wavetable::Bank::generate(1));

        let mut patch_single = Patch::default();
        patch_single.set(P::Osc1Shape, 2.0); // Saw
        patch_single.set(P::Osc1ShapeAmt, 0.0); // Single raw saw
        patch_single.set(P::Osc1Level, 1.0);
        patch_single.set(P::Cutoff, 20_000.0);
        patch_single.set(P::Reso, 0.0);

        let mut patch_triple = patch_single.clone();
        patch_triple.set(P::Osc1ShapeAmt, 1.0); // Full Oxford Triple-Saw

        let mut l1 = vec![0.0; 4096];
        let mut r1 = vec![0.0; 4096];
        engine.reset();
        engine.process(
            &patch_single,
            &[Event::NoteOn {
                note: 60,
                velocity: 1.0,
            }],
            &mut l1,
            &mut r1,
        );

        let mut l2 = vec![0.0; 4096];
        let mut r2 = vec![0.0; 4096];
        engine.reset();
        engine.process(
            &patch_triple,
            &[Event::NoteOn {
                note: 60,
                velocity: 1.0,
            }],
            &mut l2,
            &mut r2,
        );

        assert!(rms(&l1) > 0.01);
        assert!(rms(&l2) > 0.01);
        // Triple-saw has phase cancellation and detuning that differs from pure single saw
        let diff: f32 = l1.iter().zip(l2.iter()).map(|(a, b)| (a - b).abs()).sum();
        assert!(
            diff > 10.0,
            "Triple saw did not alter waveform: diff={diff}"
        );
    }

    #[test]
    fn output_stays_bounded_and_finite_at_extremes() {
        let mut engine = Engine::new(48_000.0);
        engine.set_bank(Bank::generate(9));
        let mut patch = Patch::default();
        patch.set(P::Cutoff, 20.0);
        patch.set(P::Reso, 1.0);
        patch.set(P::Drive, 1.0);
        patch.set(P::Dist, 1.0);
        patch.set(P::DelayFeedback, 0.95);
        patch.set(P::DelayMix, 1.0);
        patch.set(P::ReverbMix, 1.0);
        patch.set(P::Volume, 6.0);
        patch.set(P::FilterEnvAmt, -1.0);
        for note in [36, 48, 60, 72, 84] {
            engine.process(
                &patch,
                &[Event::NoteOn {
                    note,
                    velocity: 1.0,
                }],
                &mut vec![0.0; 64],
                &mut vec![0.0; 64],
            );
        }
        for _ in 0..40 {
            let (l, r) = block(&mut engine, &patch, 2048);
            for s in l.iter().chain(r.iter()) {
                assert!(s.is_finite(), "non-finite sample");
                assert!(s.abs() <= 1.2001, "sample {s} out of bounds");
            }
        }
    }

    #[test]
    fn pitch_bend_moves_the_oscillator() {
        let mut engine = Engine::new(48_000.0);
        engine.set_bank(Bank::generate(1));
        let mut patch = Patch::default();
        patch.set(P::AmpAttack, 1.0);
        patch.set(P::AmpDecay, 100.0);
        patch.set(P::AmpSustain, 1.0);
        patch.set(P::Osc1Level, 1.0);
        patch.set(P::Osc1Shape, 0.0);
        patch.set(P::Cutoff, 18_000.0);
        patch.set(P::Reso, 0.0);

        let mut measure = |events: &[Event]| -> f32 {
            engine.reset();
            engine.process(
                &patch,
                &[Event::NoteOn {
                    note: 69,
                    velocity: 1.0,
                }],
                &mut vec![0.0; 512],
                &mut vec![0.0; 512],
            );
            let mut l = vec![0.0; 48_000];
            let mut r = vec![0.0; 48_000];
            engine.process(&patch, events, &mut l, &mut r);
            let mut crossings = 0;
            for w in l.windows(2) {
                if w[0] <= 0.0 && w[1] > 0.0 {
                    crossings += 1;
                }
            }
            crossings as f32
        };

        let base = measure(&[]);
        let bent = measure(&[Event::PitchBend { normalized: 1.0 }]);
        assert!(
            bent > base * 1.05,
            "bend did not raise pitch: {base} vs {bent}"
        );
    }

    #[test]
    fn every_factory_preset_renders_finite_audio() {
        let mut engine = Engine::new(48_000.0);
        engine.set_bank(Bank::generate(1));
        for preset in crate::preset::FACTORY {
            engine.reset();
            let patch = preset.patch();
            engine.process(
                &patch,
                &[Event::NoteOn {
                    note: 60,
                    velocity: 0.9,
                }],
                &mut vec![0.0; 64],
                &mut vec![0.0; 64],
            );
            let (l, r) = block(&mut engine, &patch, 96_000);
            let energy = rms(&l);
            assert!(
                energy.is_finite() && energy > 1e-4,
                "{} rendered silence (rms {energy})",
                preset.name
            );
            assert!(
                l.iter().chain(r.iter()).all(|s| s.is_finite()),
                "{} produced a non-finite sample",
                preset.name
            );
        }
    }

    #[test]
    fn hold_keeps_the_envelope_at_full_before_decay() {
        let mut env = Adsr::new(48_000.0);
        env.set_times(0.001, 0.10, 0.001, 0.0, 0.05);
        env.gate_on();
        // 50 ms in: attack is done and the 100 ms hold is still running.
        for _ in 0..2_400 {
            env.next();
        }
        assert!(env.level > 0.99, "hold level {}", env.level);
        // 150 ms in: the hold has ended and the 1 ms decay has reached zero.
        for _ in 0..4_800 {
            env.next();
        }
        assert!(env.level < 0.05, "decay after hold: {}", env.level);
    }

    #[test]
    fn sample_and_hold_lfo_stays_on_one_value_per_cycle() {
        let mut lfo = Lfo::new(7);
        lfo.retrigger(7);
        let mut samples = Vec::with_capacity(24_000);
        for _ in 0..24_000 {
            samples.push(lfo.advance(2.0, 4, 48_000.0, 0.0));
        }
        // 2 Hz at 48 kHz: one step every 24,000 samples, so this window holds
        // a single value.
        let first = samples[0];
        assert!(samples.iter().all(|s| (*s - first).abs() < 1e-6));
    }

    #[test]
    fn render_is_faster_than_real_time() {
        let mut engine = Engine::new(48_000.0);
        engine.set_bank(Bank::generate(5));
        let mut patch = Patch::default();
        patch.set(P::Polyphony, 8.0);
        for note in 48..56 {
            engine.process(
                &patch,
                &[Event::NoteOn {
                    note,
                    velocity: 0.8,
                }],
                &mut [0.0; 32],
                &mut [0.0; 32],
            );
        }
        let frames = 48_000 * 2;
        let start = Instant::now();
        let _ = block(&mut engine, &patch, frames);
        let elapsed = start.elapsed().as_secs_f32();
        // Two seconds of audio in well under two seconds: a comfortable margin
        // for debug builds on shared CI runners.
        assert!(elapsed < 1.5, "8 voices took {elapsed:.3}s for 2s of audio");
    }
}
