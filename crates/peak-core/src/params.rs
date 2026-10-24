//! The parameter set, shared by the DSP engine, the plugin wrapper and the panel.
//!
//! Every parameter is stored as a real-world value (Hz, ms, semitones). The UI
//! works in normalised 0..1 space via [`Patch::normalised`] so a knob drag is
//! independent of the parameter's taper.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Unit {
    Hz,
    Rate,
    Ms,
    Percent,
    Semitones,
    Cents,
    Db,
    Index,
    Choice(&'static [&'static str]),
}

#[derive(Clone, Copy, Debug)]
pub struct ParamSpec {
    pub id: &'static str,
    pub label: &'static str,
    pub min: f32,
    pub max: f32,
    pub default: f32,
    pub unit: Unit,
    /// Logarithmic taper for frequency and time controls.
    pub log: bool,
    pub section: Section,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Section {
    Osc,
    Mixer,
    Filter,
    AmpEnv,
    FilterEnv,
    Lfo,
    Fx,
    Voice,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(usize)]
pub enum P {
    // Oscillators 1-3
    Osc1Level,
    Osc1Shape,
    Osc1Tune,
    Osc1Fine,
    Osc2Level,
    Osc2Shape,
    Osc2Tune,
    Osc2Fine,
    Osc3Level,
    Osc3Shape,
    Osc3Tune,
    Osc3Fine,
    // Mixer
    NoiseLevel,
    RingLevel,
    // Filter
    Cutoff,
    Reso,
    Drive,
    FilterEnvAmt,
    Keytrack,
    FilterType,
    // Amp envelope
    AmpAttack,
    AmpDecay,
    AmpSustain,
    AmpRelease,
    // Filter envelope
    FilterAttack,
    FilterDecay,
    FilterSustain,
    FilterRelease,
    // LFO 1
    Lfo1Rate,
    Lfo1Shape,
    Lfo1Depth,
    Lfo1Dest,
    // LFO 2
    Lfo2Rate,
    Lfo2Shape,
    Lfo2Depth,
    Lfo2Dest,
    // FX
    Dist,
    Chorus,
    DelayTime,
    DelayFeedback,
    DelayMix,
    ReverbSize,
    ReverbMix,
    // Voice
    Glide,
    Polyphony,
    BendRange,
    Volume,
    // Peak fidelity pass. Appended so v1.0.0 preset arrays keep their layout.
    Osc1ShapeAmt,
    Osc2ShapeAmt,
    Osc3ShapeAmt,
    Osc3FilterMod,
    AmpHold,
    FilterHold,
    Lfo1Fade,
    Lfo2Fade,
    Fm12,
    Fm23,
    Drift,
    Diverge,
    NoiseTone,
}

pub const WAVE_NAMES: &[&str] = &[
    "SINE", "TRI", "SAW", "SQUARE", "PULSE 25", "PULSE 12", "HARMONIC", "FORMANT", "USER A",
    "USER B", "USER C", "USER D",
];
// Keep the first four entries in place: v1.0.0 presets store the index.
pub const FILTER_NAMES: &[&str] = &["LP 12", "LP 24", "BP 12", "HP 24", "BP 24", "HP 12"];
pub const LFO_SHAPES: &[&str] = &["SINE", "TRI", "SAW", "SQUARE", "S&H"];
pub const LFO_DESTS: &[&str] = &["PITCH", "FILTER", "SHAPE", "AMP"];
pub const SHAPE_NAMES: &[&str] = WAVE_NAMES;

impl P {
    pub const COUNT: usize = 60;

    pub const ALL: [P; Self::COUNT] = [
        P::Osc1Level,
        P::Osc1Shape,
        P::Osc1Tune,
        P::Osc1Fine,
        P::Osc2Level,
        P::Osc2Shape,
        P::Osc2Tune,
        P::Osc2Fine,
        P::Osc3Level,
        P::Osc3Shape,
        P::Osc3Tune,
        P::Osc3Fine,
        P::NoiseLevel,
        P::RingLevel,
        P::Cutoff,
        P::Reso,
        P::Drive,
        P::FilterEnvAmt,
        P::Keytrack,
        P::FilterType,
        P::AmpAttack,
        P::AmpDecay,
        P::AmpSustain,
        P::AmpRelease,
        P::FilterAttack,
        P::FilterDecay,
        P::FilterSustain,
        P::FilterRelease,
        P::Lfo1Rate,
        P::Lfo1Shape,
        P::Lfo1Depth,
        P::Lfo1Dest,
        P::Lfo2Rate,
        P::Lfo2Shape,
        P::Lfo2Depth,
        P::Lfo2Dest,
        P::Dist,
        P::Chorus,
        P::DelayTime,
        P::DelayFeedback,
        P::DelayMix,
        P::ReverbSize,
        P::ReverbMix,
        P::Glide,
        P::Polyphony,
        P::BendRange,
        P::Volume,
        P::Osc1ShapeAmt,
        P::Osc2ShapeAmt,
        P::Osc3ShapeAmt,
        P::Osc3FilterMod,
        P::AmpHold,
        P::FilterHold,
        P::Lfo1Fade,
        P::Lfo2Fade,
        P::Fm12,
        P::Fm23,
        P::Drift,
        P::Diverge,
        P::NoiseTone,
    ];

    pub fn index(self) -> usize {
        self as usize
    }

    pub fn spec(self) -> ParamSpec {
        use Section::*;
        use Unit::*;
        let (id, label, min, max, default, unit, log, section) = match self {
            P::Osc1Level => ("osc1_level", "LEVEL", 0.0, 1.0, 0.72, Percent, false, Osc),
            P::Osc1Shape => (
                "osc1_shape",
                "WAVE",
                0.0,
                11.0,
                2.0,
                Choice(WAVE_NAMES),
                false,
                Osc,
            ),
            P::Osc1Tune => (
                "osc1_tune",
                "COARSE",
                -24.0,
                24.0,
                0.0,
                Semitones,
                false,
                Osc,
            ),
            P::Osc1Fine => ("osc1_fine", "FINE", -100.0, 100.0, 0.0, Cents, false, Osc),
            P::Osc2Level => ("osc2_level", "LEVEL", 0.0, 1.0, 0.0, Percent, false, Osc),
            P::Osc2Shape => (
                "osc2_shape",
                "WAVE",
                0.0,
                11.0,
                2.0,
                Choice(WAVE_NAMES),
                false,
                Osc,
            ),
            P::Osc2Tune => (
                "osc2_tune",
                "COARSE",
                -24.0,
                24.0,
                0.0,
                Semitones,
                false,
                Osc,
            ),
            P::Osc2Fine => ("osc2_fine", "FINE", -100.0, 100.0, 7.0, Cents, false, Osc),
            P::Osc3Level => ("osc3_level", "LEVEL", 0.0, 1.0, 0.0, Percent, false, Osc),
            P::Osc3Shape => (
                "osc3_shape",
                "WAVE",
                0.0,
                11.0,
                2.0,
                Choice(WAVE_NAMES),
                false,
                Osc,
            ),
            P::Osc3Tune => (
                "osc3_tune",
                "COARSE",
                -24.0,
                24.0,
                -12.0,
                Semitones,
                false,
                Osc,
            ),
            P::Osc3Fine => ("osc3_fine", "FINE", -100.0, 100.0, 0.0, Cents, false, Osc),
            P::NoiseLevel => ("noise_level", "NOISE", 0.0, 1.0, 0.0, Percent, false, Mixer),
            P::RingLevel => ("ring_level", "RING", 0.0, 1.0, 0.0, Percent, false, Mixer),
            P::Cutoff => (
                "cutoff", "CUTOFF", 20.0, 18_000.0, 6_400.0, Hz, true, Filter,
            ),
            P::Reso => ("reso", "RESO", 0.0, 1.0, 0.12, Percent, false, Filter),
            P::Drive => ("drive", "DRIVE", 0.0, 1.0, 0.18, Percent, false, Filter),
            P::FilterEnvAmt => (
                "filter_env",
                "ENV AMT",
                -1.0,
                1.0,
                0.30,
                Percent,
                false,
                Filter,
            ),
            P::Keytrack => (
                "keytrack", "KEYTRACK", 0.0, 1.0, 0.30, Percent, false, Filter,
            ),
            P::FilterType => (
                "filter_type",
                "TYPE",
                0.0,
                5.0,
                1.0,
                Choice(FILTER_NAMES),
                false,
                Filter,
            ),
            P::AmpAttack => ("amp_attack", "ATTACK", 1.0, 20_000.0, 4.0, Ms, true, AmpEnv),
            P::AmpDecay => ("amp_decay", "DECAY", 5.0, 25_000.0, 620.0, Ms, true, AmpEnv),
            P::AmpSustain => (
                "amp_sustain",
                "SUSTAIN",
                0.0,
                1.0,
                0.85,
                Percent,
                false,
                AmpEnv,
            ),
            P::AmpRelease => (
                "amp_release",
                "RELEASE",
                5.0,
                25_000.0,
                340.0,
                Ms,
                true,
                AmpEnv,
            ),
            P::FilterAttack => (
                "filter_attack",
                "ATTACK",
                1.0,
                20_000.0,
                3.0,
                Ms,
                true,
                FilterEnv,
            ),
            P::FilterDecay => (
                "filter_decay",
                "DECAY",
                5.0,
                25_000.0,
                480.0,
                Ms,
                true,
                FilterEnv,
            ),
            P::FilterSustain => (
                "filter_sustain",
                "SUSTAIN",
                0.0,
                1.0,
                0.25,
                Percent,
                false,
                FilterEnv,
            ),
            P::FilterRelease => (
                "filter_release",
                "RELEASE",
                5.0,
                25_000.0,
                260.0,
                Ms,
                true,
                FilterEnv,
            ),
            P::Lfo1Rate => ("lfo1_rate", "RATE", 0.05, 30.0, 1.2, Rate, true, Lfo),
            P::Lfo1Shape => (
                "lfo1_shape",
                "SHAPE",
                0.0,
                3.0,
                0.0,
                Choice(LFO_SHAPES),
                false,
                Lfo,
            ),
            P::Lfo1Depth => ("lfo1_depth", "DEPTH", 0.0, 1.0, 0.0, Percent, false, Lfo),
            P::Lfo1Dest => (
                "lfo1_dest",
                "DEST",
                0.0,
                3.0,
                0.0,
                Choice(LFO_DESTS),
                false,
                Lfo,
            ),
            P::Lfo2Rate => ("lfo2_rate", "RATE", 0.05, 30.0, 0.35, Rate, true, Lfo),
            P::Lfo2Shape => (
                "lfo2_shape",
                "SHAPE",
                0.0,
                3.0,
                1.0,
                Choice(LFO_SHAPES),
                false,
                Lfo,
            ),
            P::Lfo2Depth => ("lfo2_depth", "DEPTH", 0.0, 1.0, 0.0, Percent, false, Lfo),
            P::Lfo2Dest => (
                "lfo2_dest",
                "DEST",
                0.0,
                3.0,
                1.0,
                Choice(LFO_DESTS),
                false,
                Lfo,
            ),
            P::Dist => ("dist", "DRIVE", 0.0, 1.0, 0.0, Percent, false, Fx),
            P::Chorus => ("chorus", "CHORUS", 0.0, 1.0, 0.12, Percent, false, Fx),
            P::DelayTime => ("delay_time", "TIME", 20.0, 1_500.0, 330.0, Ms, true, Fx),
            P::DelayFeedback => ("delay_fb", "FEEDBACK", 0.0, 0.95, 0.34, Percent, false, Fx),
            P::DelayMix => ("delay_mix", "DELAY", 0.0, 1.0, 0.14, Percent, false, Fx),
            P::ReverbSize => ("reverb_size", "SIZE", 0.0, 1.0, 0.55, Percent, false, Fx),
            P::ReverbMix => ("reverb_mix", "REVERB", 0.0, 1.0, 0.18, Percent, false, Fx),
            P::Glide => ("glide", "GLIDE", 0.0, 2_000.0, 0.0, Ms, true, Voice),
            P::Polyphony => ("polyphony", "VOICES", 1.0, 16.0, 8.0, Index, false, Voice),
            P::BendRange => (
                "bend_range",
                "BEND",
                0.0,
                24.0,
                2.0,
                Semitones,
                false,
                Voice,
            ),
            P::Volume => ("volume", "MASTER", -36.0, 6.0, -3.0, Db, false, Voice),
            P::Osc1ShapeAmt => (
                "osc1_shape_amt",
                "SHAPE",
                -1.0,
                1.0,
                0.0,
                Percent,
                false,
                Osc,
            ),
            P::Osc2ShapeAmt => (
                "osc2_shape_amt",
                "SHAPE",
                -1.0,
                1.0,
                0.0,
                Percent,
                false,
                Osc,
            ),
            P::Osc3ShapeAmt => (
                "osc3_shape_amt",
                "SHAPE",
                -1.0,
                1.0,
                0.0,
                Percent,
                false,
                Osc,
            ),
            P::Osc3FilterMod => (
                "osc3_filter_mod",
                "OSC3 MOD",
                -1.0,
                1.0,
                0.0,
                Percent,
                false,
                Filter,
            ),
            P::AmpHold => ("amp_hold", "HOLD", 0.0, 5_000.0, 0.0, Ms, false, AmpEnv),
            P::FilterHold => (
                "filter_hold",
                "HOLD",
                0.0,
                5_000.0,
                0.0,
                Ms,
                false,
                FilterEnv,
            ),
            P::Lfo1Fade => ("lfo1_fade", "FADE", 0.0, 5_000.0, 0.0, Ms, false, Lfo),
            P::Lfo2Fade => ("lfo2_fade", "FADE", 0.0, 5_000.0, 0.0, Ms, false, Lfo),
            P::Fm12 => ("fm12", "FM 1>2", 0.0, 1.0, 0.0, Percent, false, Osc),
            P::Fm23 => ("fm23", "FM 2>3", 0.0, 1.0, 0.0, Percent, false, Osc),
            P::Drift => ("drift", "DRIFT", 0.0, 1.0, 0.06, Percent, false, Voice),
            P::Diverge => ("diverge", "DIVERGE", 0.0, 1.0, 0.04, Percent, false, Voice),
            P::NoiseTone => (
                "noise_tone",
                "NOISE LP",
                0.0,
                1.0,
                1.0,
                Percent,
                false,
                Mixer,
            ),
        };
        ParamSpec {
            id,
            label,
            min,
            max,
            default,
            unit,
            log,
            section,
        }
    }

    pub fn from_index(i: usize) -> Option<P> {
        P::ALL.get(i).copied()
    }

    pub fn from_id(id: &str) -> Option<P> {
        P::ALL.iter().copied().find(|p| p.spec().id == id)
    }
}

/// A parameter value in 0..1 space.
pub fn to_norm(spec: ParamSpec, value: f32) -> f32 {
    let v = value.clamp(spec.min, spec.max);
    if spec.log {
        let (lo, hi) = (spec.min.max(1e-4).ln(), spec.max.ln());
        ((v.ln() - lo) / (hi - lo)).clamp(0.0, 1.0)
    } else {
        ((v - spec.min) / (spec.max - spec.min)).clamp(0.0, 1.0)
    }
}

pub fn from_norm(spec: ParamSpec, norm: f32) -> f32 {
    let n = norm.clamp(0.0, 1.0);
    let value = if spec.log {
        let (lo, hi) = (spec.min.max(1e-4).ln(), spec.max.ln());
        (lo + n * (hi - lo)).exp()
    } else {
        spec.min + n * (spec.max - spec.min)
    };
    value.clamp(spec.min, spec.max)
}

/// Human readable value, e.g. `6.40 kHz`, `-12.0 st`, `SAW`.
pub fn format(spec: ParamSpec, value: f32) -> String {
    match spec.unit {
        Unit::Hz => {
            if value >= 1_000.0 {
                format!("{:.2} kHz", value / 1_000.0)
            } else {
                format!("{value:.0} Hz")
            }
        }
        Unit::Rate => format!("{value:.2} Hz"),
        Unit::Ms => {
            if value >= 1_000.0 {
                format!("{:.2} s", value / 1_000.0)
            } else {
                format!("{value:.0} ms")
            }
        }
        Unit::Percent => {
            if spec.min < 0.0 {
                format!("{:+.0}%", value * 100.0)
            } else if spec.id == "filter_type" {
                format!("{value:.0}")
            } else {
                format!("{:.0}%", value * 100.0)
            }
        }
        Unit::Semitones => format!("{value:+.1} st"),
        Unit::Cents => format!("{value:+.0} ct"),
        Unit::Db => format!("{value:+.1} dB"),
        Unit::Index => format!("{value:.0}"),
        Unit::Choice(names) => {
            let i = (value.round() as usize).min(names.len().saturating_sub(1));
            names.get(i).copied().unwrap_or("?").to_string()
        }
    }
}

/// The complete patch: one real-world value per parameter.
///
/// `[f32; 60]` cannot derive serde (arrays only go to 32), so this round-trips
/// through a slice and repairs short or long documents on load.
#[derive(Clone, Debug, PartialEq)]
pub struct Patch {
    values: [f32; P::COUNT],
    pub name: String,
}

impl Serialize for Patch {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("Patch", 2)?;
        state.serialize_field("values", &self.values[..])?;
        state.serialize_field("name", &self.name)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for Patch {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(serde::Deserialize)]
        struct Repr {
            #[serde(default)]
            values: Vec<f32>,
            #[serde(default = "default_name")]
            name: String,
        }
        let repr = Repr::deserialize(deserializer)?;
        let mut patch = Patch::default();
        for (slot, value) in patch.values.iter_mut().zip(repr.values.iter()) {
            *slot = *value;
        }
        patch.name = repr.name;
        patch.sanitise();
        Ok(patch)
    }
}

pub fn default_name() -> String {
    "INIT PATCH".to_string()
}

impl Default for Patch {
    fn default() -> Self {
        let mut values = [0.0f32; P::COUNT];
        for p in P::ALL {
            values[p.index()] = p.spec().default;
        }
        Self {
            values,
            name: default_name(),
        }
    }
}

impl Patch {
    pub fn get(&self, p: P) -> f32 {
        self.values[p.index()]
    }

    pub fn set(&mut self, p: P, value: f32) {
        let spec = p.spec();
        self.values[p.index()] = if matches!(spec.unit, Unit::Choice(_)) {
            value.round().clamp(spec.min, spec.max)
        } else {
            value.clamp(spec.min, spec.max)
        };
    }

    pub fn normalised(&self, p: P) -> f32 {
        to_norm(p.spec(), self.get(p))
    }

    pub fn set_normalised(&mut self, p: P, norm: f32) {
        let spec = p.spec();
        let mut v = from_norm(spec, norm);
        if matches!(spec.unit, Unit::Choice(_)) {
            v = v.round();
        }
        self.set(p, v);
    }

    /// Restore a single control to its factory value.
    pub fn reset(&mut self, p: P) {
        self.set(p, p.spec().default);
    }

    /// Clamp and round choice parameters after deserialising untrusted state.
    pub fn sanitise(&mut self) {
        for p in P::ALL {
            let v = self.values[p.index()];
            let v = if v.is_finite() { v } else { p.spec().default };
            self.values[p.index()] = v;
            self.set(p, v);
        }
        if self.name.len() > 24 || self.name.trim().is_empty() {
            self.name = default_name();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_is_complete_and_indexes_match() {
        assert_eq!(P::ALL.len(), P::COUNT);
        for (i, p) in P::ALL.iter().enumerate() {
            assert_eq!(p.index(), i, "{p:?} is out of order");
            assert_eq!(P::from_index(i), Some(*p));
        }
        let mut ids: Vec<_> = P::ALL.iter().map(|p| p.spec().id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), P::COUNT, "duplicate parameter ids");
    }

    #[test]
    fn defaults_are_in_range_and_round_trip() {
        for p in P::ALL {
            let spec = p.spec();
            assert!(
                spec.default >= spec.min && spec.default <= spec.max,
                "{:?} default out of range",
                p
            );
            assert!(spec.min < spec.max, "{:?} has an empty range", p);
            let n = to_norm(spec, spec.default);
            let back = from_norm(spec, n);
            let tol = (spec.max - spec.min) * 1e-3;
            assert!(
                (back - spec.default).abs() <= tol,
                "{p:?}: {back} != {}",
                spec.default
            );
        }
    }

    #[test]
    fn normalised_space_is_clamped() {
        let spec = P::Cutoff.spec();
        assert_eq!(to_norm(spec, 1.0), 0.0);
        assert_eq!(to_norm(spec, 1e9), 1.0);
        assert_eq!(from_norm(spec, -1.0), spec.min);
        assert_eq!(from_norm(spec, 2.0), spec.max);
    }

    #[test]
    fn choice_parameters_snap() {
        let mut patch = Patch::default();
        patch.set_normalised(P::FilterType, 0.9);
        assert_eq!(patch.get(P::FilterType), 5.0);
        assert_eq!(format(P::FilterType.spec(), 5.0), "HP 12");
        // Mid-drag values snap to the nearest option instead of drifting.
        patch.set_normalised(P::FilterType, 0.4);
        assert_eq!(patch.get(P::FilterType), 2.0);
    }

    #[test]
    fn patch_sanitises_garbage() {
        let mut patch = Patch::default();
        patch.values[P::Cutoff.index()] = f32::NAN;
        patch.values[P::Osc1Level.index()] = 1e12;
        patch.name = " ".into();
        patch.sanitise();
        assert!(patch.get(P::Cutoff).is_finite());
        assert_eq!(patch.get(P::Osc1Level), 1.0);
        assert_eq!(patch.name, default_name());
    }

    #[test]
    fn formats_readable_values() {
        assert_eq!(format(P::Cutoff.spec(), 6400.0), "6.40 kHz");
        assert_eq!(format(P::AmpDecay.spec(), 620.0), "620 ms");
        assert_eq!(format(P::AmpRelease.spec(), 1_500.0), "1.50 s");
        assert_eq!(format(P::Volume.spec(), -3.0), "-3.0 dB");
        assert_eq!(format(P::FilterEnvAmt.spec(), 0.3), "+30%");
        assert_eq!(format(P::Osc1Shape.spec(), 2.0), "SAW");
        assert_eq!(format(P::Osc1ShapeAmt.spec(), 0.25), "+25%");
        assert_eq!(format(P::Lfo1Shape.spec(), 4.0), "S&H");
    }
}
