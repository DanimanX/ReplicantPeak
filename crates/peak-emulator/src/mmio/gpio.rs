//! GPIO stub for STM32F4 GPIOA through GPIOE.

use super::traits::MmioDevice;
use std::sync::{Arc, Mutex};

pub struct GpioDevice {
    name: &'static str,
    moder: u32,
    otyper: u32,
    ospeedr: u32,
    pupdr: u32,
    idr: u32,
    odr: Arc<Mutex<u32>>,
    afrl: u32,
    afrh: u32,
}

impl GpioDevice {
    pub fn new(name: &'static str, odr_shared: Arc<Mutex<u32>>) -> Self {
        Self {
            name,
            moder: 0,
            otyper: 0,
            ospeedr: 0,
            pupdr: 0,
            idr: 0xFFFF, // Default pull-up / high
            odr: odr_shared,
            afrl: 0,
            afrh: 0,
        }
    }

    /// Set an input pin state (0 or 1).
    pub fn set_pin_input(&mut self, pin: u8, high: bool) {
        if pin < 16 {
            if high {
                self.idr |= 1 << pin;
            } else {
                self.idr &= !(1 << pin);
            }
        }
    }

    /// Read an output pin state.
    pub fn get_pin_output(&self, pin: u8) -> bool {
        if pin < 16 {
            if let Ok(odr) = self.odr.lock() {
                return (*odr & (1 << pin)) != 0;
            }
        }
        false
    }
}

impl MmioDevice for GpioDevice {
    fn name(&self) -> &'static str {
        self.name
    }

    fn read(&mut self, offset: u32, _size: usize) -> u32 {
        match offset {
            0x00 => self.moder,
            0x04 => self.otyper,
            0x08 => self.ospeedr,
            0x0C => self.pupdr,
            0x10 => self.idr,
            0x14 => {
                if let Ok(odr) = self.odr.lock() {
                    *odr
                } else {
                    0
                }
            }
            0x20 => self.afrl,
            0x24 => self.afrh,
            _ => 0,
        }
    }

    fn write(&mut self, offset: u32, _size: usize, value: u32) {
        match offset {
            0x00 => self.moder = value,
            0x04 => self.otyper = value,
            0x08 => self.ospeedr = value,
            0x0C => self.pupdr = value,
            0x14 => {
                if let Ok(mut odr) = self.odr.lock() {
                    *odr = value & 0xFFFF;
                }
            }
            0x18 => {
                // BSRR: lower 16 bits set, upper 16 bits reset
                let set_mask = value & 0xFFFF;
                let reset_mask = (value >> 16) & 0xFFFF;
                if let Ok(mut odr) = self.odr.lock() {
                    *odr = (*odr | set_mask) & !reset_mask;
                }
            }
            0x20 => self.afrl = value,
            0x24 => self.afrh = value,
            _ => {}
        }
    }
}
