//! SPI stub for STM32F4 (SPI1 and SPI4).

use super::display::OledDisplay;
use super::traits::MmioDevice;
use std::sync::{Arc, Mutex};

pub struct SpiDevice {
    name: &'static str,
    cr1: u32,
    cr2: u32,
    sr: u32,
    dr_rx: u32,
    display: Option<Arc<Mutex<OledDisplay>>>,
    last_tx: Arc<Mutex<Vec<u8>>>,
}

impl SpiDevice {
    pub fn new(name: &'static str, display: Option<Arc<Mutex<OledDisplay>>>) -> Self {
        Self {
            name,
            cr1: 0,
            cr2: 0,
            // TXE (bit 1) = 1, RXNE (bit 0) = 1
            sr: (1 << 1) | (1 << 0),
            dr_rx: 0xFF,
            display,
            last_tx: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn last_transfers(&self) -> Vec<u8> {
        self.last_tx.lock().map(|t| t.clone()).unwrap_or_default()
    }
}

impl MmioDevice for SpiDevice {
    fn name(&self) -> &'static str {
        self.name
    }

    fn read(&mut self, offset: u32, _size: usize) -> u32 {
        match offset {
            0x00 => self.cr1,
            0x04 => self.cr2,
            0x08 => (1 << 1) | (1 << 0), // TXE and RXNE ready
            0x0C => self.dr_rx,
            _ => 0,
        }
    }

    fn write(&mut self, offset: u32, _size: usize, value: u32) {
        match offset {
            0x00 => self.cr1 = value,
            0x04 => self.cr2 = value,
            0x08 => self.sr = value,
            0x0C => {
                let byte = (value & 0xFF) as u8;
                if let Ok(mut tx) = self.last_tx.lock() {
                    if tx.len() > 256 {
                        tx.clear();
                    }
                    tx.push(byte);
                }
                // Forward to display if wired
                if let Some(disp) = &self.display {
                    if let Ok(mut d) = disp.lock() {
                        d.write_byte(true, byte);
                    }
                }
            }
            _ => {}
        }
    }
}
