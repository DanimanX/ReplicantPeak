//! Host-facing parameters.
//!
//! Generated from the same `P` table the engine and the panel use, so a range
//! can never drift between what the UI draws and what the host automates. The
//! `parameter_ranges_match_core` test in `lib.rs` enforces that.

use std::sync::{Arc, Mutex};

use nih_plug::prelude::*;
use nih_plug_egui::EguiState;
use peak_core::params::{format, ParamSpec, Unit, P};
use peak_ui::{FirmwareStamp, PANEL_H, PANEL_W};

/// Persisted firmware link: what was validated last time this session ran.
pub type FirmwareStore = Arc<Mutex<Option<FirmwareStamp>>>;

#[derive(Params)]
pub struct PeakParams {
    #[persist = "editor-state"]
    pub editor_state: Arc<EguiState>,

    #[persist = "firmware"]
    pub firmware: FirmwareStore,

    #[id = "osc1_level"]
    pub osc1_level: FloatParam,
    #[id = "osc1_shape"]
    pub osc1_shape: FloatParam,
    #[id = "osc1_tune"]
    pub osc1_tune: FloatParam,
    #[id = "osc1_fine"]
    pub osc1_fine: FloatParam,
    #[id = "osc2_level"]
    pub osc2_level: FloatParam,
    #[id = "osc2_shape"]
    pub osc2_shape: FloatParam,
    #[id = "osc2_tune"]
    pub osc2_tune: FloatParam,
    #[id = "osc2_fine"]
    pub osc2_fine: FloatParam,
    #[id = "osc3_level"]
    pub osc3_level: FloatParam,
    #[id = "osc3_shape"]
    pub osc3_shape: FloatParam,
    #[id = "osc3_tune"]
    pub osc3_tune: FloatParam,
    #[id = "osc3_fine"]
    pub osc3_fine: FloatParam,
    #[id = "noise_level"]
    pub noise_level: FloatParam,
    #[id = "ring_level"]
    pub ring_level: FloatParam,
    #[id = "cutoff"]
    pub cutoff: FloatParam,
    #[id = "reso"]
    pub reso: FloatParam,
    #[id = "drive"]
    pub drive: FloatParam,
    #[id = "filter_env"]
    pub filter_env: FloatParam,
    #[id = "keytrack"]
    pub keytrack: FloatParam,
    #[id = "filter_type"]
    pub filter_type: FloatParam,
    #[id = "amp_attack"]
    pub amp_attack: FloatParam,
    #[id = "amp_decay"]
    pub amp_decay: FloatParam,
    #[id = "amp_sustain"]
    pub amp_sustain: FloatParam,
    #[id = "amp_release"]
    pub amp_release: FloatParam,
    #[id = "filter_attack"]
    pub filter_attack: FloatParam,
    #[id = "filter_decay"]
    pub filter_decay: FloatParam,
    #[id = "filter_sustain"]
    pub filter_sustain: FloatParam,
    #[id = "filter_release"]
    pub filter_release: FloatParam,
    #[id = "lfo1_rate"]
    pub lfo1_rate: FloatParam,
    #[id = "lfo1_shape"]
    pub lfo1_shape: FloatParam,
    #[id = "lfo1_depth"]
    pub lfo1_depth: FloatParam,
    #[id = "lfo1_dest"]
    pub lfo1_dest: FloatParam,
    #[id = "lfo2_rate"]
    pub lfo2_rate: FloatParam,
    #[id = "lfo2_shape"]
    pub lfo2_shape: FloatParam,
    #[id = "lfo2_depth"]
    pub lfo2_depth: FloatParam,
    #[id = "lfo2_dest"]
    pub lfo2_dest: FloatParam,
    #[id = "dist"]
    pub dist: FloatParam,
    #[id = "chorus"]
    pub chorus: FloatParam,
    #[id = "delay_time"]
    pub delay_time: FloatParam,
    #[id = "delay_fb"]
    pub delay_fb: FloatParam,
    #[id = "delay_mix"]
    pub delay_mix: FloatParam,
    #[id = "reverb_size"]
    pub reverb_size: FloatParam,
    #[id = "reverb_mix"]
    pub reverb_mix: FloatParam,
    #[id = "glide"]
    pub glide: FloatParam,
    #[id = "polyphony"]
    pub polyphony: FloatParam,
    #[id = "bend_range"]
    pub bend_range: FloatParam,
    #[id = "volume"]
    pub volume: FloatParam,
}

impl Default for PeakParams {
    fn default() -> Self {
        Self {
            editor_state: EguiState::from_size(PANEL_W as u32, PANEL_H as u32),
            firmware: Arc::new(Mutex::new(None)),
            osc1_level: make_param(P::Osc1Level),
            osc1_shape: make_param(P::Osc1Shape),
            osc1_tune: make_param(P::Osc1Tune),
            osc1_fine: make_param(P::Osc1Fine),
            osc2_level: make_param(P::Osc2Level),
            osc2_shape: make_param(P::Osc2Shape),
            osc2_tune: make_param(P::Osc2Tune),
            osc2_fine: make_param(P::Osc2Fine),
            osc3_level: make_param(P::Osc3Level),
            osc3_shape: make_param(P::Osc3Shape),
            osc3_tune: make_param(P::Osc3Tune),
            osc3_fine: make_param(P::Osc3Fine),
            noise_level: make_param(P::NoiseLevel),
            ring_level: make_param(P::RingLevel),
            cutoff: make_param(P::Cutoff),
            reso: make_param(P::Reso),
            drive: make_param(P::Drive),
            filter_env: make_param(P::FilterEnvAmt),
            keytrack: make_param(P::Keytrack),
            filter_type: make_param(P::FilterType),
            amp_attack: make_param(P::AmpAttack),
            amp_decay: make_param(P::AmpDecay),
            amp_sustain: make_param(P::AmpSustain),
            amp_release: make_param(P::AmpRelease),
            filter_attack: make_param(P::FilterAttack),
            filter_decay: make_param(P::FilterDecay),
            filter_sustain: make_param(P::FilterSustain),
            filter_release: make_param(P::FilterRelease),
            lfo1_rate: make_param(P::Lfo1Rate),
            lfo1_shape: make_param(P::Lfo1Shape),
            lfo1_depth: make_param(P::Lfo1Depth),
            lfo1_dest: make_param(P::Lfo1Dest),
            lfo2_rate: make_param(P::Lfo2Rate),
            lfo2_shape: make_param(P::Lfo2Shape),
            lfo2_depth: make_param(P::Lfo2Depth),
            lfo2_dest: make_param(P::Lfo2Dest),
            dist: make_param(P::Dist),
            chorus: make_param(P::Chorus),
            delay_time: make_param(P::DelayTime),
            delay_fb: make_param(P::DelayFeedback),
            delay_mix: make_param(P::DelayMix),
            reverb_size: make_param(P::ReverbSize),
            reverb_mix: make_param(P::ReverbMix),
            glide: make_param(P::Glide),
            polyphony: make_param(P::Polyphony),
            bend_range: make_param(P::BendRange),
            volume: make_param(P::Volume),
        }
    }
}

impl PeakParams {
    pub fn float(&self, p: P) -> &FloatParam {
        match p {
            P::Osc1Level => &self.osc1_level,
            P::Osc1Shape => &self.osc1_shape,
            P::Osc1Tune => &self.osc1_tune,
            P::Osc1Fine => &self.osc1_fine,
            P::Osc2Level => &self.osc2_level,
            P::Osc2Shape => &self.osc2_shape,
            P::Osc2Tune => &self.osc2_tune,
            P::Osc2Fine => &self.osc2_fine,
            P::Osc3Level => &self.osc3_level,
            P::Osc3Shape => &self.osc3_shape,
            P::Osc3Tune => &self.osc3_tune,
            P::Osc3Fine => &self.osc3_fine,
            P::NoiseLevel => &self.noise_level,
            P::RingLevel => &self.ring_level,
            P::Cutoff => &self.cutoff,
            P::Reso => &self.reso,
            P::Drive => &self.drive,
            P::FilterEnvAmt => &self.filter_env,
            P::Keytrack => &self.keytrack,
            P::FilterType => &self.filter_type,
            P::AmpAttack => &self.amp_attack,
            P::AmpDecay => &self.amp_decay,
            P::AmpSustain => &self.amp_sustain,
            P::AmpRelease => &self.amp_release,
            P::FilterAttack => &self.filter_attack,
            P::FilterDecay => &self.filter_decay,
            P::FilterSustain => &self.filter_sustain,
            P::FilterRelease => &self.filter_release,
            P::Lfo1Rate => &self.lfo1_rate,
            P::Lfo1Shape => &self.lfo1_shape,
            P::Lfo1Depth => &self.lfo1_depth,
            P::Lfo1Dest => &self.lfo1_dest,
            P::Lfo2Rate => &self.lfo2_rate,
            P::Lfo2Shape => &self.lfo2_shape,
            P::Lfo2Depth => &self.lfo2_depth,
            P::Lfo2Dest => &self.lfo2_dest,
            P::Dist => &self.dist,
            P::Chorus => &self.chorus,
            P::DelayTime => &self.delay_time,
            P::DelayFeedback => &self.delay_fb,
            P::DelayMix => &self.delay_mix,
            P::ReverbSize => &self.reverb_size,
            P::ReverbMix => &self.reverb_mix,
            P::Glide => &self.glide,
            P::Polyphony => &self.polyphony,
            P::BendRange => &self.bend_range,
            P::Volume => &self.volume,
        }
    }

    /// Copy the host's values into an existing patch. Allocation-free, so the
    /// audio thread can call it every block.
    pub fn write_into(&self, patch: &mut peak_core::Patch) {
        for p in P::ALL {
            patch.set(p, self.float(p).value());
        }
    }

    /// Snapshot the host's parameter values into a plain patch for the engine.
    #[allow(dead_code)]
    pub fn to_patch(&self) -> peak_core::Patch {
        let mut patch = peak_core::Patch::default();
        self.write_into(&mut patch);
        patch
    }
}

/// Logarithmic controls use a skewed range so the knob's midpoint is the
/// geometric mean instead of the arithmetic one.
fn log_factor(spec: ParamSpec) -> f32 {
    let mid = (spec.min.max(1e-4) * spec.max).sqrt();
    let y = ((mid - spec.min) / (spec.max - spec.min)).clamp(1e-4, 0.9999);
    0.5f32.log(y).clamp(0.1, 20.0)
}

fn unit_suffix(unit: Unit) -> &'static str {
    match unit {
        Unit::Hz | Unit::Rate => " Hz",
        Unit::Ms => " ms",
        Unit::Percent => "%",
        Unit::Semitones => " st",
        Unit::Cents => " ct",
        Unit::Db => " dB",
        Unit::Index | Unit::Choice(_) => "",
    }
}

fn param_name(p: P) -> String {
    let spec = p.spec();
    let prefix = match spec.id {
        id if id.starts_with("osc1_") => "OSC 1",
        id if id.starts_with("osc2_") => "OSC 2",
        id if id.starts_with("osc3_") => "OSC 3",
        "noise_level" | "ring_level" => "MIXER",
        "cutoff" | "reso" | "drive" | "filter_env" | "keytrack" | "filter_type" => "FILTER",
        id if id.starts_with("amp_") => "AMP ENV",
        "filter_attack" | "filter_decay" | "filter_sustain" | "filter_release" => "FILTER ENV",
        id if id.starts_with("lfo1_") => "LFO 1",
        id if id.starts_with("lfo2_") => "LFO 2",
        id if id.starts_with("delay_") => "DELAY",
        id if id.starts_with("reverb_") => "REVERB",
        "dist" | "chorus" => "FX",
        "glide" | "polyphony" | "bend_range" => "VOICE",
        _ => "MASTER",
    };
    format!("{prefix} {}", spec.label)
}

fn make_param(p: P) -> FloatParam {
    let spec = p.spec();
    let range = if matches!(spec.unit, Unit::Choice(_)) || spec.unit == Unit::Index {
        FloatRange::Linear {
            min: spec.min,
            max: spec.max,
        }
    } else if spec.log {
        FloatRange::Skewed {
            min: spec.min,
            max: spec.max,
            factor: log_factor(spec),
        }
    } else {
        FloatRange::Linear {
            min: spec.min,
            max: spec.max,
        }
    };

    let mut param = FloatParam::new(param_name(p), spec.default, range)
        .with_unit(unit_suffix(spec.unit))
        .with_value_to_string(Arc::new(move |value| format(spec, value)));

    if matches!(spec.unit, Unit::Choice(_)) || spec.unit == Unit::Index {
        param = param.with_step_size(1.0);
    } else if spec.unit != Unit::Ms && spec.unit != Unit::Hz && spec.unit != Unit::Rate {
        // Continuous controls get a short smoother so fast automation cannot
        // zipper the audio; the engine smooths the harshest ones again.
        param = param.with_smoother(SmoothingStyle::Linear(5.0));
    }
    param
}
