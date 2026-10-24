//! USART/UART stub capturing firmware serial console logs.

use super::traits::MmioDevice;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

pub struct UartDevice {
    name: &'static str,
    sr: u32,
    cr1: u32,
    cr2: u32,
    cr3: u32,
    brr: u32,
    rx_queue: VecDeque<u8>,
    tx_log: Arc<Mutex<Vec<u8>>>,
    print_stdout: bool,
}

impl UartDevice {
    pub fn new(name: &'static str, tx_log: Arc<Mutex<Vec<u8>>>, print_stdout: bool) -> Self {
        Self {
            name,
            // TXE (bit 7) = 1, TC (bit 6) = 1
            sr: (1 << 7) | (1 << 6),
            cr1: 0,
            cr2: 0,
            cr3: 0,
            brr: 0,
            rx_queue: VecDeque::new(),
            tx_log,
            print_stdout,
        }
    }

    pub fn inject_rx(&mut self, byte: u8) {
        self.rx_queue.push_back(byte);
        self.sr |= 1 << 5; // RXNE = 1
    }
}

impl MmioDevice for UartDevice {
    fn name(&self) -> &'static str {
        self.name
    }

    fn read(&mut self, offset: u32, _size: usize) -> u32 {
        match offset {
            0x00 => {
                let mut status = (1 << 7) | (1 << 6); // TXE and TC always ready
                if !self.rx_queue.is_empty() {
                    status |= 1 << 5; // RXNE
                }
                status
            }
            0x04 => {
                // DR read
                if let Some(b) = self.rx_queue.pop_front() {
                    if self.rx_queue.is_empty() {
                        self.sr &= !(1 << 5);
                    }
                    b as u32
                } else {
                    0
                }
            }
            0x08 => self.brr,
            0x0C => self.cr1,
            0x10 => self.cr2,
            0x14 => self.cr3,
            _ => 0,
        }
    }

    fn write(&mut self, offset: u32, _size: usize, value: u32) {
        match offset {
            0x00 => self.sr = value,
            0x04 => {
                let byte = (value & 0xFF) as u8;
                if let Ok(mut log) = self.tx_log.lock() {
                    log.push(byte);
                }
                if self.print_stdout {
                    let char_out = if byte == b'\r' || byte == b'\n' || (0x20..0x7F).contains(&byte) {
                        byte as char
                    } else {
                        '.'
                    };
                    print!("{char_out}");
                    use std::io::Write;
                    let _ = std::io::stdout().flush();
                }
            }
            0x08 => self.brr = value,
            0x0C => self.cr1 = value,
            0x10 => self.cr2 = value,
            0x14 => self.cr3 = value,
            _ => {}
        }
    }
}
