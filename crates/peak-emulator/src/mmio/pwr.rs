//! Power Control (PWR) stub for STM32F4.

use super::traits::MmioDevice;

pub struct PwrDevice {
    cr: u32,
    csr: u32,
}

impl Default for PwrDevice {
    fn default() -> Self {
        Self::new()
    }
}

impl PwrDevice {
    pub fn new() -> Self {
        Self {
            cr: 0x0000_C000,
            // VOSRDY (bit 14) and BRR (Backup Regulator Ready, bit 3)
            csr: (1 << 14) | (1 << 3),
        }
    }
}

impl MmioDevice for PwrDevice {
    fn name(&self) -> &'static str {
        "PWR"
    }

    fn read(&mut self, offset: u32, _size: usize) -> u32 {
        match offset {
            0x00 => self.cr,
            // Always return VOSRDY (bit 14) and BRR (bit 3) ready
            0x04 => self.csr | (1 << 14) | (1 << 3),
            _ => 0,
        }
    }

    fn write(&mut self, offset: u32, _size: usize, value: u32) {
        match offset {
            0x00 => self.cr = value,
            0x04 => self.csr = value,
            _ => {}
        }
    }
}
