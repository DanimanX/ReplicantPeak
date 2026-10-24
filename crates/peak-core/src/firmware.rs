//! Novation Peak firmware (`.syx`) validation.
//!
//! The Peak firmware is delivered as a stream of SysEx messages with Novation's
//! manufacturer id (`00 20 29`) and a product byte that identifies the payload:
//!
//! * `0x71` — main firmware
//! * `0x7C` — FPGA bitstream
//!
//! ReplicantPeak does not execute or decode the firmware. The image is used as
//! (1) a gate: the plugin refuses to make sound until a genuine Peak image is
//! linked, (2) a fingerprint against the known-build table below, and (3) a
//! deterministic seed for the user wavetables. Nothing is uploaded anywhere.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// SysEx start-of-exclusive.
const SOX: u8 = 0xF0;
/// SysEx end-of-exclusive.
const EOX: u8 = 0xF7;
/// Novation's three-byte manufacturer id.
const NOVATION_ID: [u8; 3] = [0x00, 0x20, 0x29];
/// Firmware payload product byte.
const PRODUCT_FIRMWARE: u8 = 0x71;
/// FPGA bitstream product byte.
const PRODUCT_FPGA: u8 = 0x7C;
/// A Peak image is a few thousand messages; anything tiny is not a firmware.
const MIN_MESSAGES: usize = 64;

/// Which half of the Peak update package an image belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FwKind {
    Firmware,
    Fpga,
}

impl FwKind {
    pub fn label(self) -> &'static str {
        match self {
            FwKind::Firmware => "FIRMWARE",
            FwKind::Fpga => "FPGA",
        }
    }
}

/// A build Novation has published, identified by its SHA-256.
///
/// The hashes were computed from the files served by the Components download
/// endpoint on 2026-10-23. They let the plugin recognise an image offline
/// instead of trusting a filename.
pub struct KnownBuild {
    pub sha256: &'static str,
    pub kind: FwKind,
    pub version: &'static str,
    pub build: u32,
}

pub const KNOWN_BUILDS: &[KnownBuild] = &[
    KnownBuild {
        sha256: "b9df02687aafb90629ddf8db3e91e05b8d33ff9ef431bfcc9e761fbd2d51f3e7",
        kind: FwKind::Firmware,
        version: "1.2",
        build: 352,
    },
    KnownBuild {
        sha256: "f2b12373cdef564d3e7a6c429e9438cabe366354a3417a7f74a8816a97a5c2e1",
        kind: FwKind::Firmware,
        version: "1.2.1",
        build: 363,
    },
    KnownBuild {
        sha256: "f863219761f91e597360adbecbfa40e5f14f518fbd7ef7781c1e0a371f63e827",
        kind: FwKind::Firmware,
        version: "1.3",
        build: 397,
    },
    KnownBuild {
        sha256: "d14bd7dc927f5d48f289ef30fe71919c93c3c388b5f49b6bf023748246ed17bc",
        kind: FwKind::Firmware,
        version: "1.3.1",
        build: 399,
    },
    KnownBuild {
        sha256: "06b038a0e9608fa9d83094f9d8c9a36b07a0c740b131d8a462029b50175a8285",
        kind: FwKind::Firmware,
        version: "2.0",
        build: 440,
    },
    KnownBuild {
        sha256: "96b72f75b319aca4caa40b3d55a017c5a9e65755556d1d02b4a0163b3285e99e",
        kind: FwKind::Firmware,
        version: "2.1",
        build: 471,
    },
    KnownBuild {
        sha256: "f9d50b4268a5c612b71d4cafd45dc01a02ecdf0902aa6e31784ad4d1bc4de26e",
        kind: FwKind::Firmware,
        version: "2.2",
        build: 2108,
    },
    KnownBuild {
        sha256: "fbf1b5b9a4a4aa47cf0cb134b7284d9fc7653ddc71a8d563314c5672833f7ed7",
        kind: FwKind::Fpga,
        version: "1.2",
        build: 353,
    },
    KnownBuild {
        sha256: "c0270f6089c6d257db51b12e000b69ef5f00a29fdce8e32c5e543797e7e132ff",
        kind: FwKind::Fpga,
        version: "2.0",
        build: 439,
    },
    KnownBuild {
        sha256: "317d48663036feb5bce51fc8c5448c618f0cf8673c4bef2e2a4b1631ad18924d",
        kind: FwKind::Fpga,
        version: "2.1",
        build: 469,
    },
];

/// The Peak build this engine is voiced against.
pub const REFERENCE_VERSION: &str = "2.2";
pub const REFERENCE_BUILD: u32 = 2108;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FirmwareError {
    Empty,
    /// The first byte is not SysEx start-of-exclusive.
    NotSysex,
    /// A data byte had the high bit set, a message never terminated, or a new
    /// message started before the previous one ended.
    BrokenFraming {
        offset: usize,
    },
    /// SysEx from a manufacturer other than Novation.
    ForeignManufacturer([u8; 3]),
    /// A Novation product id this build does not handle.
    NotAPeakPayload(u8),
    /// Structurally valid but too short to be a Peak update image.
    TooShort {
        messages: usize,
    },
}

impl std::fmt::Display for FirmwareError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FirmwareError::Empty => write!(f, "file is empty"),
            FirmwareError::NotSysex => write!(f, "not a SysEx file (missing F0 at byte 0)"),
            FirmwareError::BrokenFraming { offset } => {
                write!(f, "broken SysEx framing at byte {offset}")
            }
            FirmwareError::ForeignManufacturer(id) => write!(
                f,
                "manufacturer id {:02X} {:02X} {:02X} is not Novation",
                id[0], id[1], id[2]
            ),
            FirmwareError::NotAPeakPayload(p) => {
                write!(f, "product byte 0x{p:02X} is not Peak firmware/FPGA")
            }
            FirmwareError::TooShort { messages } => {
                write!(f, "only {messages} SysEx messages: not a Peak image")
            }
        }
    }
}

impl std::error::Error for FirmwareError {}

/// Result of validating an image: enough to gate the plugin and label the UI.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FirmwareImage {
    pub sha256: [u8; 32],
    pub kind: FwKind,
    pub product: u8,
    pub messages: usize,
    pub bytes: usize,
    /// Index into [`KNOWN_BUILDS`], when the hash is recognised.
    pub known: Option<usize>,
    /// Deterministic seed for the user wavetables.
    pub seed: u64,
}

impl FirmwareImage {
    pub fn known_build(&self) -> Option<&'static KnownBuild> {
        self.known.map(|i| &KNOWN_BUILDS[i])
    }

    pub fn sha_hex(&self) -> String {
        let mut s = String::with_capacity(64);
        for b in self.sha256 {
            s.push_str(&format!("{b:02x}"));
        }
        s
    }

    pub fn sha_short(&self) -> String {
        self.sha_hex()[..12].to_string()
    }

    /// Build identity for the panel: `v2.2 · 2108` when recognised, otherwise
    /// an explicit "unverified" marker so the UI never overstates what it knows.
    pub fn version_label(&self) -> String {
        match self.known_build() {
            Some(k) => format!("v{} \u{00b7} {}", k.version, k.build),
            None => format!("UNVERIFIED \u{00b7} {}", &self.sha_short()[..8]),
        }
    }

    pub fn is_reference_build(&self) -> bool {
        matches!(self.known_build(), Some(k) if k.kind == FwKind::Firmware
            && k.version == REFERENCE_VERSION && k.build == REFERENCE_BUILD)
    }
}

/// Walk the SysEx stream, verifying framing, and collect the first message
/// header. Deliberately strict: a corrupted download must be rejected instead of
/// producing a half-valid bank.
fn scan(bytes: &[u8]) -> Result<(usize, [u8; 8]), FirmwareError> {
    if bytes.is_empty() {
        return Err(FirmwareError::Empty);
    }
    if bytes[0] != SOX {
        return Err(FirmwareError::NotSysex);
    }

    let mut messages = 0usize;
    let mut in_message = false;
    let mut first = [0u8; 8];
    let mut header_len = 0usize;

    for (i, &b) in bytes.iter().enumerate() {
        match b {
            SOX => {
                if in_message {
                    return Err(FirmwareError::BrokenFraming { offset: i });
                }
                in_message = true;
                header_len = 0;
            }
            EOX => {
                if !in_message {
                    return Err(FirmwareError::BrokenFraming { offset: i });
                }
                in_message = false;
                messages += 1;
            }
            _ => {
                if !in_message {
                    return Err(FirmwareError::BrokenFraming { offset: i });
                }
                if b & 0x80 != 0 {
                    return Err(FirmwareError::BrokenFraming { offset: i });
                }
                // Only the first message carries the identity header; later
                // messages start with sample data.
                if messages == 0 && header_len < first.len() {
                    first[header_len] = b;
                    header_len += 1;
                }
            }
        }
    }

    if in_message {
        return Err(FirmwareError::BrokenFraming {
            offset: bytes.len(),
        });
    }

    Ok((messages, first))
}

/// Validate a Peak firmware or FPGA image.
pub fn parse(bytes: &[u8]) -> Result<FirmwareImage, FirmwareError> {
    let (messages, header) = scan(bytes)?;
    // header[0..3] is the manufacturer id, header[3] the family and header[4]
    // the product id. The real Peak stream starts `F0 00 20 29 00 71 00 7E`.
    if header[0..3] != NOVATION_ID {
        return Err(FirmwareError::ForeignManufacturer([
            header[0], header[1], header[2],
        ]));
    }
    let product = header[4];
    let kind = match product {
        PRODUCT_FIRMWARE => FwKind::Firmware,
        PRODUCT_FPGA => FwKind::Fpga,
        other => return Err(FirmwareError::NotAPeakPayload(other)),
    };
    if messages < MIN_MESSAGES {
        return Err(FirmwareError::TooShort { messages });
    }

    let digest: [u8; 32] = Sha256::digest(bytes).into();
    let hex: String = digest.iter().map(|b| format!("{b:02x}")).collect();
    let known = KNOWN_BUILDS.iter().position(|b| b.sha256 == hex);

    // Seed: the digest itself, mixed so that nearby builds do not produce
    // similar tables.
    let seed = u64::from_le_bytes(digest[0..8].try_into().unwrap()) ^ 0x9E37_79B9_7F4A_7C15;

    Ok(FirmwareImage {
        sha256: digest,
        kind,
        product,
        messages,
        bytes: bytes.len(),
        known,
        seed,
    })
}

/// Convenience wrapper for the common "read the file, validate it" path.
pub fn parse_file(path: &std::path::Path) -> Result<FirmwareImage, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    parse(&bytes).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn message(payload: &[u8]) -> Vec<u8> {
        let mut v = vec![SOX];
        v.extend_from_slice(payload);
        v.push(EOX);
        v
    }

    fn fake_image(product: u8, count: usize) -> Vec<u8> {
        let mut v = Vec::new();
        for i in 0..count {
            let payload = [
                NOVATION_ID[0],
                NOVATION_ID[1],
                NOVATION_ID[2],
                0x00,
                product,
                0x00,
                0x7E,
                (i % 128) as u8,
                (i / 128 % 128) as u8,
            ];
            v.extend(message(&payload));
        }
        v
    }

    #[test]
    fn rejects_empty_and_non_sysex() {
        assert_eq!(parse(&[]), Err(FirmwareError::Empty));
        assert_eq!(parse(&[0x90, 0x40, 0x7F]), Err(FirmwareError::NotSysex));
    }

    #[test]
    fn rejects_broken_framing() {
        let mut img = fake_image(PRODUCT_FIRMWARE, MIN_MESSAGES);
        // Drop the final EOX.
        img.pop();
        assert!(matches!(
            parse(&img),
            Err(FirmwareError::BrokenFraming { .. })
        ));

        let mut img = fake_image(PRODUCT_FIRMWARE, MIN_MESSAGES);
        // A data byte with the high bit set is illegal in SysEx.
        img[6] = 0x91;
        assert!(matches!(
            parse(&img),
            Err(FirmwareError::BrokenFraming { .. })
        ));

        // Two SOX bytes in a row.
        let mut bad = vec![SOX, SOX];
        bad.extend(fake_image(PRODUCT_FIRMWARE, MIN_MESSAGES));
        assert!(matches!(
            parse(&bad),
            Err(FirmwareError::BrokenFraming { offset: 1 })
        ));
    }

    #[test]
    fn rejects_foreign_manufacturer_and_product() {
        let junk: Vec<u8> = (0..MIN_MESSAGES)
            .flat_map(|_| message(&[0x00, 0x41, 0x29, 0x00, 0x71]))
            .collect();
        assert_eq!(
            parse(&junk),
            Err(FirmwareError::ForeignManufacturer([0x00, 0x41, 0x29]))
        );

        assert_eq!(
            parse(&fake_image(0x3A, MIN_MESSAGES)),
            Err(FirmwareError::NotAPeakPayload(0x3A))
        );
    }

    #[test]
    fn rejects_short_images() {
        assert_eq!(
            parse(&fake_image(PRODUCT_FIRMWARE, 3)),
            Err(FirmwareError::TooShort { messages: 3 })
        );
    }

    #[test]
    fn classifies_firmware_and_fpga() {
        let fw = parse(&fake_image(PRODUCT_FIRMWARE, MIN_MESSAGES)).unwrap();
        assert_eq!(fw.kind, FwKind::Firmware);
        assert_eq!(fw.messages, MIN_MESSAGES);
        assert!(fw.known.is_none(), "synthetic image must not match a build");
        assert!(fw.version_label().starts_with("UNVERIFIED"));

        let fpga = parse(&fake_image(PRODUCT_FPGA, MIN_MESSAGES)).unwrap();
        assert_eq!(fpga.kind, FwKind::Fpga);
    }

    #[test]
    fn seed_is_stable_and_depends_on_content() {
        let a = parse(&fake_image(PRODUCT_FIRMWARE, MIN_MESSAGES)).unwrap();
        let b = parse(&fake_image(PRODUCT_FIRMWARE, MIN_MESSAGES)).unwrap();
        assert_eq!(a.seed, b.seed);

        let mut other = fake_image(PRODUCT_FIRMWARE, MIN_MESSAGES);
        let last = other.len() - 2;
        other[last] = 7;
        let c = parse(&other).unwrap();
        assert_ne!(a.seed, c.seed);
        assert_ne!(a.sha256, c.sha256);
    }

    #[test]
    fn known_build_table_is_well_formed() {
        for b in KNOWN_BUILDS {
            assert_eq!(b.sha256.len(), 64, "{} has a short hash", b.version);
            assert!(
                b.sha256
                    .bytes()
                    .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
                "{} hash is not lowercase hex",
                b.version
            );
        }
        let mut hashes: Vec<_> = KNOWN_BUILDS.iter().map(|b| b.sha256).collect();
        hashes.sort_unstable();
        hashes.dedup();
        assert_eq!(hashes.len(), KNOWN_BUILDS.len(), "duplicate hashes");
    }

    /// The real 2.2 image, when present in the working tree, must be recognised
    /// and reported as the reference build.
    #[test]
    fn recognises_reference_build_when_available() {
        let path = std::path::Path::new("../../firmware/peak-firmware-2.2-build2108.syx");
        let Ok(bytes) = std::fs::read(path) else {
            eprintln!("skipping: {} not present", path.display());
            return;
        };
        let img = parse(&bytes).expect("reference image must validate");
        assert_eq!(img.kind, FwKind::Firmware);
        assert!(img.is_reference_build(), "got {}", img.version_label());
        assert_eq!(img.sha_short(), "f9d50b4268a5");
    }
}
