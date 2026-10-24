//! Hardware emulator for Novation Peak microcontroller firmware.

pub mod cpu;
pub mod gui;
pub mod loader;
pub mod memory;
pub mod mmio;

pub use cpu::Emulator;
pub use loader::FirmwareBinary;
pub use memory::*;
pub use mmio::{MmioBus, OledDisplay};
