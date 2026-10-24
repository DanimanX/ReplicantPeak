//! Presets.
//!
//! A preset is a named [`Patch`]. The factory bank lives in this file so a
//! fresh install has something to play; user presets are individual JSON files
//! in a per-user directory, next to the plugin rather than inside the project,
//! so saving a patch never touches the repository.
//!
//! Peak's own factory patches are *not* here and cannot be: they ship as a
//! separate bank download from Novation Components, and the firmware `.syx`
//! this engine links contains no patch records at all (see the README).

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::params::{Patch, P};

/// A factory preset: a name and only the values that differ from the init
/// patch, so adding a parameter later leaves old presets valid.
pub struct Preset {
    pub name: &'static str,
    pub values: &'static [(P, f32)],
}

impl Preset {
    /// Expand this preset into a complete patch.
    pub fn patch(&self) -> Patch {
        let mut patch = Patch::default();
        patch.name = self.name.to_string();
        for (param, value) in self.values {
            patch.set(*param, *value);
        }
        patch
    }
}

impl std::fmt::Debug for Preset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Preset")
            .field("name", &self.name)
            .field("values", &self.values.len())
            .finish()
    }
}

/// The presets that ship with the plugin.
///
/// Voiced against the reference build: these are patches for *this* engine, not
/// transcriptions of the Peak's factory bank.
pub const FACTORY: &[Preset] = &[
    Preset {
        name: "INIT PATCH",
        values: &[],
    },
    Preset {
        name: "PEAK BASS",
        values: &[
            (P::Osc1Level, 0.80),
            (P::Osc1Shape, 2.0),
            (P::Osc1Tune, -12.0),
            (P::Osc2Level, 0.45),
            (P::Osc2Shape, 3.0),
            (P::Osc2Fine, -8.0),
            (P::Cutoff, 520.0),
            (P::Reso, 0.22),
            (P::Drive, 0.42),
            (P::FilterEnvAmt, 0.45),
            (P::FilterType, 1.0),
            (P::AmpAttack, 2.0),
            (P::AmpDecay, 380.0),
            (P::AmpSustain, 0.62),
            (P::AmpRelease, 180.0),
            (P::FilterAttack, 1.0),
            (P::FilterDecay, 260.0),
            (P::FilterSustain, 0.20),
            (P::FilterRelease, 200.0),
            (P::ReverbMix, 0.06),
            (P::Volume, -4.0),
        ],
    },
    Preset {
        name: "WIDE SAW STACK",
        values: &[
            (P::Osc1Level, 0.62),
            (P::Osc1Shape, 2.0),
            (P::Osc2Level, 0.58),
            (P::Osc2Shape, 2.0),
            (P::Osc2Fine, 11.0),
            (P::Osc3Level, 0.54),
            (P::Osc3Shape, 2.0),
            (P::Osc3Tune, -12.0),
            (P::Osc3Fine, -9.0),
            (P::Cutoff, 3_400.0),
            (P::Reso, 0.10),
            (P::Keytrack, 0.55),
            (P::AmpAttack, 180.0),
            (P::AmpDecay, 900.0),
            (P::AmpSustain, 0.80),
            (P::AmpRelease, 700.0),
            (P::FilterEnvAmt, 0.28),
            (P::FilterDecay, 1_200.0),
            (P::FilterSustain, 0.45),
            (P::FilterRelease, 900.0),
            (P::Lfo2Rate, 0.22),
            (P::Lfo2Depth, 0.28),
            (P::Lfo2Dest, 1.0),
            (P::Chorus, 0.34),
            (P::ReverbMix, 0.30),
        ],
    },
    Preset {
        name: "GLASS PAD",
        values: &[
            (P::Osc1Level, 0.48),
            (P::Osc1Shape, 6.0),
            (P::Osc2Level, 0.42),
            (P::Osc2Shape, 8.0),
            (P::Osc2Tune, 7.0),
            (P::Osc3Level, 0.30),
            (P::Osc3Shape, 5.0),
            (P::Osc3Tune, 12.0),
            (P::Osc3Fine, 6.0),
            (P::Cutoff, 5_200.0),
            (P::Keytrack, 0.40),
            (P::AmpAttack, 1_400.0),
            (P::AmpDecay, 2_400.0),
            (P::AmpSustain, 0.75),
            (P::AmpRelease, 2_100.0),
            (P::FilterAttack, 1_800.0),
            (P::FilterSustain, 0.55),
            (P::FilterRelease, 2_400.0),
            (P::Lfo1Rate, 0.18),
            (P::Lfo1Depth, 0.32),
            (P::Lfo1Dest, 1.0),
            (P::Chorus, 0.48),
            (P::DelayMix, 0.22),
            (P::ReverbSize, 0.82),
            (P::ReverbMix, 0.42),
        ],
    },
    Preset {
        name: "FM BELL",
        values: &[
            (P::Osc1Level, 0.55),
            (P::Osc1Shape, 0.0),
            (P::Osc2Level, 0.40),
            (P::Osc2Shape, 0.0),
            (P::Osc2Tune, 19.0),
            (P::Osc3Level, 0.14),
            (P::Osc3Shape, 0.0),
            (P::Osc3Tune, 24.0),
            (P::RingLevel, 0.30),
            (P::Cutoff, 8_000.0),
            (P::AmpAttack, 1.0),
            (P::AmpDecay, 1_600.0),
            (P::AmpSustain, 0.0),
            (P::AmpRelease, 900.0),
            (P::FilterEnvAmt, 0.55),
            (P::FilterDecay, 700.0),
            (P::FilterSustain, 0.0),
            (P::DelayMix, 0.24),
            (P::ReverbMix, 0.34),
        ],
    },
    Preset {
        name: "RING METAL",
        values: &[
            (P::Osc1Level, 0.60),
            (P::Osc1Shape, 3.0),
            (P::Osc2Level, 0.52),
            (P::Osc2Shape, 3.0),
            (P::Osc2Tune, 7.0),
            (P::Osc2Fine, 34.0),
            (P::RingLevel, 0.62),
            (P::Cutoff, 1_900.0),
            (P::Reso, 0.34),
            (P::Drive, 0.28),
            (P::FilterType, 1.0),
            (P::AmpAttack, 4.0),
            (P::AmpDecay, 800.0),
            (P::AmpSustain, 0.42),
            (P::AmpRelease, 420.0),
            (P::Dist, 0.22),
            (P::DelayMix, 0.20),
            (P::ReverbMix, 0.16),
        ],
    },
    Preset {
        name: "ACID LEAD",
        values: &[
            (P::Osc1Level, 0.78),
            (P::Osc1Shape, 2.0),
            (P::Cutoff, 420.0),
            (P::Reso, 0.68),
            (P::Drive, 0.52),
            (P::FilterEnvAmt, 0.72),
            (P::FilterType, 1.0),
            (P::Keytrack, 0.45),
            (P::Glide, 60.0),
            (P::Polyphony, 1.0),
            (P::AmpAttack, 1.0),
            (P::AmpDecay, 300.0),
            (P::AmpSustain, 0.70),
            (P::AmpRelease, 120.0),
            (P::FilterAttack, 1.0),
            (P::FilterDecay, 220.0),
            (P::FilterSustain, 0.12),
            (P::FilterRelease, 180.0),
            (P::Dist, 0.38),
            (P::DelayMix, 0.18),
        ],
    },
    Preset {
        name: "SUB PLUCK",
        values: &[
            (P::Osc1Level, 0.70),
            (P::Osc1Shape, 0.0),
            (P::Osc1Tune, -12.0),
            (P::Osc2Level, 0.30),
            (P::Osc2Shape, 4.0),
            (P::Cutoff, 900.0),
            (P::Reso, 0.18),
            (P::FilterEnvAmt, 0.60),
            (P::AmpAttack, 1.0),
            (P::AmpDecay, 240.0),
            (P::AmpSustain, 0.0),
            (P::AmpRelease, 160.0),
            (P::FilterDecay, 180.0),
            (P::FilterSustain, 0.0),
            (P::Chorus, 0.0),
            (P::Volume, -3.0),
        ],
    },
    Preset {
        name: "NOISE SWEEP",
        values: &[
            (P::Osc1Level, 0.22),
            (P::Osc1Shape, 2.0),
            (P::NoiseLevel, 0.55),
            (P::Cutoff, 1_200.0),
            (P::Reso, 0.42),
            (P::FilterEnvAmt, -0.85),
            (P::FilterType, 2.0),
            (P::AmpAttack, 2_200.0),
            (P::AmpSustain, 0.85),
            (P::AmpRelease, 1_600.0),
            (P::FilterAttack, 2_400.0),
            (P::FilterSustain, 0.10),
            (P::FilterRelease, 1_800.0),
            (P::Lfo1Rate, 4.60),
            (P::Lfo1Depth, 0.18),
            (P::Lfo1Dest, 2.0),
            (P::ReverbMix, 0.45),
        ],
    },
    Preset {
        name: "WARM CHORDS",
        values: &[
            (P::Osc1Level, 0.50),
            (P::Osc1Shape, 7.0),
            (P::Osc2Level, 0.44),
            (P::Osc2Shape, 3.0),
            (P::Osc2Fine, 6.0),
            (P::Osc3Level, 0.26),
            (P::Osc3Shape, 0.0),
            (P::Osc3Tune, -12.0),
            (P::Cutoff, 2_600.0),
            (P::Reso, 0.14),
            (P::Keytrack, 0.35),
            (P::AmpAttack, 420.0),
            (P::AmpDecay, 1_400.0),
            (P::AmpSustain, 0.78),
            (P::AmpRelease, 1_100.0),
            (P::Chorus, 0.52),
            (P::ReverbSize, 0.68),
            (P::ReverbMix, 0.28),
            (P::Polyphony, 16.0),
        ],
    },
    Preset {
        name: "TAPE KEYS",
        values: &[
            (P::Osc1Level, 0.52),
            (P::Osc1Shape, 1.0),
            (P::Osc2Level, 0.40),
            (P::Osc2Shape, 2.0),
            (P::Osc2Fine, -5.0),
            (P::Cutoff, 3_200.0),
            (P::Keytrack, 0.50),
            (P::AmpAttack, 6.0),
            (P::AmpDecay, 900.0),
            (P::AmpSustain, 0.48),
            (P::AmpRelease, 520.0),
            (P::Drive, 0.24),
            (P::Chorus, 0.28),
            (P::ReverbMix, 0.20),
        ],
    },
    Preset {
        name: "HOLLOW FIFTHS",
        values: &[
            (P::Osc1Level, 0.52),
            (P::Osc1Shape, 5.0),
            (P::Osc2Level, 0.44),
            (P::Osc2Shape, 5.0),
            (P::Osc2Tune, 7.0),
            (P::Osc3Level, 0.30),
            (P::Osc3Shape, 2.0),
            (P::Osc3Tune, 12.0),
            (P::Cutoff, 2_100.0),
            (P::Reso, 0.26),
            (P::AmpAttack, 640.0),
            (P::AmpDecay, 1_800.0),
            (P::AmpSustain, 0.70),
            (P::AmpRelease, 1_500.0),
            (P::Lfo2Rate, 0.12),
            (P::Lfo2Depth, 0.20),
            (P::Lfo2Dest, 0.0),
            (P::ReverbSize, 0.74),
            (P::ReverbMix, 0.36),
        ],
    },
    Preset {
        name: "NEON SAW",
        values: &[
            (P::Osc1Level, 0.62),
            (P::Osc1Shape, 2.0),
            (P::Osc1ShapeAmt, 0.55),
            (P::Osc2Level, 0.58),
            (P::Osc2Shape, 2.0),
            (P::Osc2ShapeAmt, -0.35),
            (P::Osc2Fine, 11.0),
            (P::Osc3Level, 0.50),
            (P::Osc3Shape, 2.0),
            (P::Osc3Tune, -12.0),
            (P::Osc3Fine, -9.0),
            (P::Cutoff, 4_200.0),
            (P::Reso, 0.16),
            (P::Drive, 0.22),
            (P::Keytrack, 0.70),
            (P::Drift, 0.22),
            (P::Diverge, 0.18),
            (P::AmpAttack, 120.0),
            (P::AmpDecay, 1_100.0),
            (P::AmpSustain, 0.82),
            (P::AmpRelease, 900.0),
            (P::Lfo1Rate, 0.16),
            (P::Lfo1Depth, 0.22),
            (P::Lfo1Dest, 1.0),
            (P::Chorus, 0.30),
            (P::ReverbMix, 0.24),
        ],
    },
    Preset {
        name: "FM GLASS",
        values: &[
            (P::Osc1Level, 0.48),
            (P::Osc1Shape, 0.0),
            (P::Osc2Level, 0.18),
            (P::Osc2Shape, 0.0),
            (P::Osc2Tune, 7.0),
            (P::Fm12, 0.46),
            (P::Osc3Level, 0.10),
            (P::Osc3Shape, 0.0),
            (P::Osc3Tune, 19.0),
            (P::Fm23, 0.30),
            (P::Cutoff, 9_500.0),
            (P::FilterType, 0.0),
            (P::AmpAttack, 2.0),
            (P::AmpDecay, 1_800.0),
            (P::AmpSustain, 0.18),
            (P::AmpRelease, 1_400.0),
            (P::FilterEnvAmt, 0.42),
            (P::FilterDecay, 900.0),
            (P::FilterSustain, 0.08),
            (P::DelayMix, 0.30),
            (P::DelayFeedback, 0.42),
            (P::ReverbSize, 0.86),
            (P::ReverbMix, 0.38),
        ],
    },
    Preset {
        name: "PULSE WIDTH",
        values: &[
            (P::Osc1Level, 0.70),
            (P::Osc1Shape, 4.0),
            (P::Osc1ShapeAmt, 0.35),
            (P::Osc2Level, 0.42),
            (P::Osc2Shape, 4.0),
            (P::Osc2ShapeAmt, -0.30),
            (P::Osc2Fine, 6.0),
            (P::Cutoff, 3_800.0),
            (P::Reso, 0.20),
            (P::Keytrack, 0.55),
            (P::AmpAttack, 8.0),
            (P::AmpDecay, 700.0),
            (P::AmpSustain, 0.74),
            (P::AmpRelease, 420.0),
            (P::Lfo1Rate, 0.42),
            (P::Lfo1Shape, 1.0),
            (P::Lfo1Depth, 0.55),
            (P::Lfo1Dest, 2.0),
            (P::Chorus, 0.24),
            (P::ReverbMix, 0.14),
        ],
    },
    Preset {
        name: "BANDPASS SCREAM",
        values: &[
            (P::Osc1Level, 0.55),
            (P::Osc1Shape, 2.0),
            (P::Osc1ShapeAmt, 0.72),
            (P::Osc2Level, 0.40),
            (P::Osc2Shape, 2.0),
            (P::Osc2Fine, -13.0),
            (P::Osc3Level, 0.30),
            (P::Osc3Shape, 6.0),
            (P::Osc3Tune, -12.0),
            (P::Cutoff, 1_100.0),
            (P::Reso, 0.74),
            (P::Drive, 0.58),
            (P::FilterType, 4.0),
            (P::FilterEnvAmt, 0.62),
            (P::Osc3FilterMod, 0.28),
            (P::AmpAttack, 2.0),
            (P::AmpDecay, 500.0),
            (P::AmpSustain, 0.55),
            (P::AmpRelease, 260.0),
            (P::FilterAttack, 1.0),
            (P::FilterDecay, 320.0),
            (P::FilterSustain, 0.10),
            (P::Dist, 0.34),
            (P::DelayMix, 0.24),
            (P::ReverbMix, 0.18),
        ],
    },
    Preset {
        name: "S&H VOID",
        values: &[
            (P::Osc1Level, 0.44),
            (P::Osc1Shape, 6.0),
            (P::Osc1ShapeAmt, -0.45),
            (P::Osc2Level, 0.32),
            (P::Osc2Shape, 7.0),
            (P::Osc2Tune, 7.0),
            (P::Cutoff, 2_400.0),
            (P::Reso, 0.38),
            (P::FilterType, 0.0),
            (P::AmpAttack, 900.0),
            (P::AmpDecay, 2_600.0),
            (P::AmpSustain, 0.66),
            (P::AmpRelease, 2_400.0),
            (P::Lfo1Rate, 3.20),
            (P::Lfo1Shape, 4.0),
            (P::Lfo1Depth, 0.62),
            (P::Lfo1Dest, 1.0),
            (P::Lfo2Rate, 0.11),
            (P::Lfo2Depth, 0.20),
            (P::Lfo2Dest, 2.0),
            (P::Chorus, 0.36),
            (P::DelayMix, 0.28),
            (P::DelayFeedback, 0.48),
            (P::ReverbSize, 0.92),
            (P::ReverbMix, 0.46),
        ],
    },
    Preset {
        name: "HOLD ORGAN",
        values: &[
            (P::Osc1Level, 0.52),
            (P::Osc1Shape, 6.0),
            (P::Osc1ShapeAmt, 0.10),
            (P::Osc2Level, 0.38),
            (P::Osc2Shape, 0.0),
            (P::Osc2Tune, 12.0),
            (P::Osc3Level, 0.30),
            (P::Osc3Shape, 6.0),
            (P::Osc3Tune, 19.0),
            (P::Cutoff, 6_200.0),
            (P::Keytrack, 0.80),
            (P::AmpAttack, 30.0),
            (P::AmpHold, 500.0),
            (P::AmpDecay, 1_200.0),
            (P::AmpSustain, 0.62),
            (P::AmpRelease, 1_600.0),
            (P::FilterAttack, 20.0),
            (P::FilterHold, 350.0),
            (P::FilterDecay, 1_400.0),
            (P::FilterSustain, 0.40),
            (P::FilterRelease, 1_200.0),
            (P::Chorus, 0.34),
            (P::ReverbSize, 0.70),
            (P::ReverbMix, 0.26),
        ],
    },
    Preset {
        name: "DRIFT KEYS",
        values: &[
            (P::Osc1Level, 0.54),
            (P::Osc1Shape, 2.0),
            (P::Osc1ShapeAmt, -0.25),
            (P::Osc2Level, 0.46),
            (P::Osc2Shape, 2.0),
            (P::Osc2Fine, 8.0),
            (P::Osc3Level, 0.30),
            (P::Osc3Shape, 1.0),
            (P::Osc3Tune, -12.0),
            (P::Drift, 0.55),
            (P::Diverge, 0.42),
            (P::Cutoff, 3_600.0),
            (P::Reso, 0.18),
            (P::Keytrack, 0.65),
            (P::AmpAttack, 40.0),
            (P::AmpDecay, 1_600.0),
            (P::AmpSustain, 0.72),
            (P::AmpRelease, 1_200.0),
            (P::Chorus, 0.42),
            (P::ReverbSize, 0.64),
            (P::ReverbMix, 0.24),
        ],
    },
];

/// One entry in a [`Bank`]: either a factory preset or a user preset on disk.
#[derive(Clone, Debug)]
pub struct Entry {
    pub name: String,
    pub patch: Patch,
    pub factory: bool,
    /// Where the JSON lives, for user presets.
    pub path: Option<PathBuf>,
}

/// The preset list a host shows: the factory bank followed by user presets.
#[derive(Clone, Debug)]
pub struct Bank {
    entries: Vec<Entry>,
    dir: Option<PathBuf>,
}

impl Default for Bank {
    fn default() -> Self {
        Self::new()
    }
}

impl Bank {
    /// Factory presets only, with the default user preset directory.
    pub fn new() -> Self {
        Self::with_dir(preset_dir())
    }

    pub fn with_dir(dir: Option<PathBuf>) -> Self {
        let mut entries: Vec<Entry> = FACTORY
            .iter()
            .map(|preset| Entry {
                name: preset.name.to_string(),
                patch: preset.patch(),
                factory: true,
                path: None,
            })
            .collect();
        if let Some(dir) = &dir {
            entries.extend(read_dir(dir));
        }
        Self { entries, dir }
    }

    pub fn names(&self) -> Vec<String> {
        self.entries.iter().map(|e| e.name.clone()).collect()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn get(&self, index: usize) -> Option<&Entry> {
        self.entries.get(index)
    }

    pub fn index_of(&self, name: &str) -> Option<usize> {
        self.entries.iter().position(|e| e.name == name)
    }

    pub fn dir(&self) -> Option<&Path> {
        self.dir.as_deref()
    }

    /// Replace the user half of the bank with what is on disk now.
    pub fn reload(&mut self) {
        let factory = self
            .entries
            .iter()
            .filter(|e| e.factory)
            .cloned()
            .collect::<Vec<_>>();
        let user = self.dir.as_deref().map(read_dir).unwrap_or_default();
        self.entries = factory;
        self.entries.extend(user);
    }

    /// Write `patch` as a user preset and add it to the list. Returns the index
    /// of the new entry.
    ///
    /// The name is sanitised into a filename and de-duplicated against the
    /// existing entries so storing twice never overwrites a different patch.
    pub fn store(&mut self, name: &str, patch: &Patch) -> io::Result<usize> {
        let dir = self.dir.clone().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "no user preset directory for this platform",
            )
        })?;
        fs::create_dir_all(&dir)?;

        let mut name = name.trim().to_string();
        if name.is_empty() {
            name = "USER PATCH".to_string();
        }
        if name.chars().count() > 24 {
            name = name.chars().take(24).collect();
        }
        let mut candidate = name.clone();
        let mut n = 2;
        while self.entries.iter().any(|e| e.name == candidate) {
            candidate = format!("{name} {n}");
            n += 1;
        }

        let mut stored = patch.clone();
        stored.name = candidate.clone();
        stored.sanitise();

        let path = dir.join(format!("{}.json", file_stem(&candidate)));
        let entry = StoredPreset {
            name: candidate.clone(),
            patch: stored.clone(),
        };
        fs::write(&path, entry.to_json().as_bytes())?;

        self.entries.push(Entry {
            name: candidate,
            patch: stored,
            factory: false,
            path: Some(path),
        });
        Ok(self.entries.len() - 1)
    }
}

/// A user preset on disk.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StoredPreset {
    pub name: String,
    pub patch: Patch,
}

impl StoredPreset {
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    pub fn from_json(text: &str) -> Result<Self, serde_json::Error> {
        let mut preset: Self = serde_json::from_str(text)?;
        preset.patch.sanitise();
        if preset.name.trim().is_empty() {
            preset.name = "USER PATCH".to_string();
        }
        Ok(preset)
    }
}

fn read_dir(dir: &Path) -> Vec<Entry> {
    let Ok(read) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut files: Vec<PathBuf> = read
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|ext| ext == "json"))
        .collect();
    files.sort();

    let mut entries = Vec::new();
    for path in files {
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        // A corrupt preset must not take the whole bank down.
        let Ok(preset) = StoredPreset::from_json(&text) else {
            continue;
        };
        entries.push(Entry {
            name: preset.name,
            patch: preset.patch,
            factory: false,
            path: Some(path),
        });
    }
    entries
}

/// Where user presets live, following each platform's convention.
pub fn preset_dir() -> Option<PathBuf> {
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let base = if cfg!(target_os = "windows") {
        std::env::var_os("APPDATA").map(PathBuf::from)
    } else if cfg!(target_os = "macos") {
        home.map(|h| h.join("Library/Application Support"))
    } else {
        std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| home.map(|h| h.join(".local/share")))
    };
    base.map(|b| b.join("ReplicantPeak").join("presets"))
}

fn file_stem(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if matches!(ch, ' ' | '-' | '_' | '.') && !out.ends_with('_') {
            out.push('_');
        }
    }
    let trimmed = out.trim_matches('_').to_string();
    if trimmed.is_empty() {
        "user_patch".to_string()
    } else {
        trimmed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn factory_presets_are_valid_and_named() {
        assert!(FACTORY.len() >= 8, "expected a usable factory bank");
        let mut names = Vec::new();
        for preset in FACTORY {
            assert!(!preset.name.trim().is_empty());
            assert!(preset.name.len() <= 24, "{} is too long", preset.name);
            names.push(preset.name);
            let patch = preset.patch();
            assert_eq!(patch.name, preset.name);
            for p in P::ALL {
                let v = patch.get(p);
                assert!(v.is_finite(), "{}: {p:?} is not finite", preset.name);
            }
        }
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), FACTORY.len(), "duplicate factory names");
    }

    #[test]
    fn factory_presets_differ_from_init() {
        // A preset that is byte-identical to the init patch would be a typo.
        let init = Patch::default();
        for preset in FACTORY.iter().filter(|p| p.name != "INIT PATCH") {
            let patch = preset.patch();
            let differs = P::ALL.iter().any(|p| patch.get(*p) != init.get(*p));
            assert!(differs, "{} does not change any parameter", preset.name);
        }
    }

    #[test]
    fn bank_starts_with_the_factory_presets() {
        let bank = Bank::with_dir(None);
        assert_eq!(bank.len(), FACTORY.len());
        assert_eq!(bank.names()[0], "INIT PATCH");
        assert!(bank.get(0).is_some_and(|e| e.factory));
        assert!(bank.get(bank.len()).is_none());
    }

    #[test]
    fn user_presets_round_trip_through_disk() {
        let dir = std::env::temp_dir().join(format!(
            "replicant-peak-bank-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = fs::remove_dir_all(&dir);

        let mut bank = Bank::with_dir(Some(dir.clone()));
        let mut patch = Patch::default();
        patch.set(P::Cutoff, 1234.0);
        patch.set(P::Osc2Level, 0.5);

        let index = bank.store("My Sound", &patch).expect("store");
        assert_eq!(bank.len(), FACTORY.len() + 1);
        assert_eq!(bank.names()[index], "My Sound");
        assert!(!bank.get(index).unwrap().factory);

        // Storing the same name again must not clobber the first one.
        let second = bank.store("My Sound", &patch).expect("store again");
        assert_eq!(bank.names()[second], "My Sound 2");
        assert_eq!(bank.len(), FACTORY.len() + 2);

        // A fresh bank reads both back with the values intact.
        let reloaded = Bank::with_dir(Some(dir.clone()));
        let entry = reloaded.get(FACTORY.len()).expect("reloaded entry");
        assert_eq!(entry.name, "My Sound");
        assert!((entry.patch.get(P::Cutoff) - 1234.0).abs() < 1e-3);
        assert!((entry.patch.get(P::Osc2Level) - 0.5).abs() < 1e-6);

        // A corrupt file is skipped instead of failing the whole bank.
        fs::write(dir.join("zz-broken.json"), b"{ not json").unwrap();
        let with_broken = Bank::with_dir(Some(dir.clone()));
        assert_eq!(with_broken.len(), FACTORY.len() + 2);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn file_names_are_safe() {
        assert_eq!(file_stem("My Sound"), "my_sound");
        assert_eq!(file_stem("   "), "user_patch");
        // Nothing that could escape the preset directory survives.
        for hostile in ["../etc/passwd", "..\\..\\win.ini", "a/b", "C:\\x"] {
            let stem = file_stem(hostile);
            assert!(!stem.contains('/'), "{hostile} -> {stem}");
            assert!(!stem.contains('\\'), "{hostile} -> {stem}");
            assert!(!stem.contains(':'), "{hostile} -> {stem}");
        }
    }
}
