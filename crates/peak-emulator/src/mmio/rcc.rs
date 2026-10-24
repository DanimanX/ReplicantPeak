//! Reset and Clock Control (RCC) stub for STM32F4.

use super::traits::MmioDevice;

pub struct RccDevice {
    cr: u32,
    pllcfgr: u32,
    cfgr: u32,
    cir: u32,
    ahb1enr: u32,
    ahb2enr: u32,
    apb1enr: u32,
    apb2enr: u32,
}

impl Default for RccDevice {
    fn default() -> Self {
        Self::new()
    }
}

impl RccDevice {
    pub fn new() -> Self {
        Self {
            // HSI ready (bit 1), HSE ready (bit 17), PLL ready (bit 25)
            cr: (1 << 1) | (1 << 17) | (1 << 25),
            pllcfgr: 0x2400_3010,
            cfgr: 0x0000_0008, // SWS = PLL
            cir: 0,
            ahb1enr: 0x0010_0000,
            ahb2enr: 0,
            apb1enr: 0,
            apb2enr: 0,
        }
    }
}

impl MmioDevice for RccDevice {
    fn name(&self) -> &'static str {
        "RCC"
    }

    fn read(&mut self, offset: u32, _size: usize) -> u32 {
        match offset {
            0x00 => self.cr,
            0x04 => self.pllcfgr,
            0x08 => self.cfgr,
            0x0C => self.cir,
            0x30 => self.ahb1enr,
            0x34 => self.ahb2enr,
            0x40 => self.apb1enr,
            0x44 => self.apb2enr,
            _ => 0,
        }
    }

    fn write(&mut self, offset: u32, _size: usize, value: u32) {
        match offset {
            0x00 => {
                // If HSEON (bit 16) is set, set HSERDY (bit 17)
                // If PLLON (bit 24) is set, set PLLRDY (bit 25)
                let mut cr = value;
                if (cr & (1 << 16)) != 0 {
                    cr |= 1 << 17;
                }
                if (cr & (1 << 24)) != 0 {
                    cr |= 1 << 25;
                }
                cr |= 1 << 1; // HSI always ready
                self.cr = cr;
            }
            0x04 => self.pllcfgr = value,
            0x08 => {
                let sw = value & 0x3;
                // Reflect SW into SWS (bits 3:2)
                self.cfgr = (value & !0x0C) | (sw << 2);
            }
            0x0C => self.cir = value,
            0x30 => self.ahb1enr = value,
            0x34 => self.ahb2enr = value,
            0x40 => self.apb1enr = value,
            0x44 => self.apb2enr = value,
            _ => {}
        }
    }
}
