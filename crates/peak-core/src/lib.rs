//! ReplicantPeak core: firmware validation, wavetable bank and DSP engine.
//!
//! This crate has no GUI or plugin-framework dependency, so the engine can be
//! unit tested and benchmarked without a host.

pub mod dsp;
pub mod firmware;
pub mod params;
pub mod preset;
pub mod wavetable;

pub use dsp::{Engine, Event};
pub use firmware::{
    clear_last_firmware_path, clear_last_firmware_path_if_matches, config_dir,
    load_last_firmware_path, parse as parse_firmware, read_last_firmware_path_unverified,
    save_last_firmware_path, FirmwareError, FirmwareImage, FwKind,
};
pub use params::{Patch, P};
pub use preset::Bank as PresetBank;
pub use wavetable::Bank;

use std::sync::Arc;

/// Everything that follows from a validated image: the fingerprint the UI shows
/// and the bank the engine plays.
pub struct LinkedFirmware {
    pub image: FirmwareImage,
    pub bank: Arc<Bank>,
    /// Where the image was read from, kept so a session can relink on open.
    pub path: Option<String>,
}

/// Validate bytes, then build the bank. Intended for a worker thread: the
/// additive synthesis costs a few tens of milliseconds.
pub fn link_firmware(bytes: &[u8], path: Option<String>) -> Result<LinkedFirmware, FirmwareError> {
    let image = firmware::parse(bytes)?;
    let bank = Bank::generate(image.seed);
    Ok(LinkedFirmware { image, bank, path })
}

#[cfg(test)]
mod integration {
    use super::*;

    #[test]
    fn linking_a_synthetic_image_builds_a_bank() {
        let mut bytes = Vec::new();
        for i in 0..128u32 {
            bytes.extend_from_slice(&[
                0xF0,
                0x00,
                0x20,
                0x29,
                0x00,
                0x71,
                0x00,
                0x7E,
                (i % 128) as u8,
                0xF7,
            ]);
        }
        let linked = link_firmware(&bytes, Some("synthetic.syx".into())).unwrap();
        assert_eq!(linked.bank.len(), wavetable::TABLES);
        assert_eq!(linked.bank.seed(), linked.image.seed);
        assert_eq!(linked.path.as_deref(), Some("synthetic.syx"));
    }

    #[test]
    fn linking_rejects_non_firmware() {
        assert!(link_firmware(b"not a sysex file", None).is_err());
    }
}
