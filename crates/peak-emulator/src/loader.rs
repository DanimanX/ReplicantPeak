//! Firmware loader for Novation Peak SysEx and raw binary files.

use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

/// Flash base address for STM32F4 series (0x08000000).
pub const FLASH_BASE: u64 = 0x0800_0000;
/// Flash memory size to map (typically 512 KB or 1 MB).
pub const FLASH_SIZE: usize = 512 * 1024;
/// SRAM base address (0x20000000).
pub const SRAM_BASE: u64 = 0x2000_0000;
/// SRAM memory size (192 KB for STM32F429).
pub const SRAM_SIZE: usize = 192 * 1024;

/// Result of loading and unpacking a firmware image.
#[derive(Clone, Debug)]
pub struct FirmwareBinary {
    /// Raw binary contents (ARM Cortex-M Thumb-2 code and data).
    pub data: Vec<u8>,
    /// Base address where data should be mapped.
    pub base_addr: u64,
    /// Reset entry point address (Thumb bit set or cleared).
    pub entry_point: u64,
    /// Initial stack pointer.
    pub initial_sp: u64,
    /// Vector table address.
    pub vector_table: u64,
}

impl FirmwareBinary {
    /// Load from a file path (.syx SysEx file or raw .bin).
    pub fn from_file<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let mut file = File::open(path)?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;
        Self::from_bytes(&bytes)
    }

    /// Load from memory bytes (.syx SysEx or raw .bin).
    pub fn from_bytes(bytes: &[u8]) -> io::Result<Self> {
        if bytes.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Firmware file is empty",
            ));
        }

        let raw = if bytes[0] == 0xF0 {
            // SysEx file
            unpack_sysex(bytes)?
        } else {
            bytes.to_vec()
        };

        if raw.len() < 32 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Binary too short",
            ));
        }

        // For Novation Peak:
        // Flash base is 0x08000000.
        // SRAM base is 0x20000000, 192KB top = 0x20030000.
        let initial_sp = 0x2003_0000;
        // As discovered during reverse engineering, CRT startup entry point is 0x080116a4
        let entry_point = 0x0801_16a4;

        Ok(FirmwareBinary {
            data: raw,
            base_addr: FLASH_BASE,
            entry_point,
            initial_sp,
            vector_table: FLASH_BASE,
        })
    }
}

/// Unpack Novation SysEx messages to raw binary.
pub fn unpack_sysex(data: &[u8]) -> io::Result<Vec<u8>> {
    let mut msgs: Vec<&[u8]> = Vec::new();
    let mut start: Option<usize> = None;

    for (i, &b) in data.iter().enumerate() {
        if b == 0xF0 {
            start = Some(i);
        } else if b == 0xF7 {
            if let Some(s) = start.take() {
                msgs.push(&data[s..=i]);
            }
        }
    }

    if msgs.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "No valid SysEx messages found",
        ));
    }

    let mut binary = Vec::with_capacity(256 * 1024);

    for msg in msgs {
        // Message format:
        // [0] = 0xF0
        // [1..4] = Novation ID (0x00, 0x20, 0x29) + device (0x00)
        // [5] = Command byte: 0x72 (UPDATE_WRITE), 0x73 (UPDATE_FINISH)
        // [6..len-1] = 7-bit encoded data payload (37 bytes)
        // [len-1] = 0xF7
        if msg.len() >= 8 && msg[1] == 0x00 && msg[2] == 0x20 && msg[3] == 0x29 {
            let cmd = msg[5];
            if cmd == 0x72 || cmd == 0x73 {
                let payload = &msg[6..msg.len() - 1];
                unpack_7to8(payload, &mut binary);
            }
        }
    }

    if binary.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "No firmware payload found in SysEx",
        ));
    }

    Ok(binary)
}

/// Decode 7-bit MIDI SysEx stream into 8-bit bytes.
pub fn unpack_7to8(packed: &[u8], out: &mut Vec<u8>) {
    let mut bit_buf: u64 = 0;
    let mut bit_count: usize = 0;
    for &b in packed {
        bit_buf = (bit_buf << 7) | ((b & 0x7F) as u64);
        bit_count += 7;
        while bit_count >= 8 {
            bit_count -= 8;
            out.push(((bit_buf >> bit_count) & 0xFF) as u8);
        }
    }
}
