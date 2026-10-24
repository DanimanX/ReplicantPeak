//! ARM Cortex-M SysTick Timer stub.

use super::traits::MmioDevice;

pub struct SysTickDevice {
    ctrl: u32,
    load: u32,
    val: u32,
    calib: u32,
    ticks: u64,
}

impl Default for SysTickDevice {
    fn default() -> Self {
        Self::new()
    }
}

impl SysTickDevice {
    pub fn new() -> Self {
        Self {
            ctrl: 0x07, // ENABLE, TICKINT, CLKSOURCE
            load: 168_000, // 1 ms @ 168 MHz
            val: 0,
            calib: 168_000,
            ticks: 0,
        }
    }

    pub fn tick(&mut self) -> bool {
        self.ticks += 1;
        if self.val == 0 {
            self.val = self.load;
            self.ctrl |= 1 << 16; // COUNTFLAG
            (self.ctrl & 0x02) != 0 // Trigger interrupt if TICKINT enabled
        } else {
            self.val = self.val.saturating_sub(1);
            false
        }
    }
}

impl MmioDevice for SysTickDevice {
    fn name(&self) -> &'static str {
        "SysTick"
    }

    fn read(&mut self, offset: u32, _size: usize) -> u32 {
        match offset {
            0x00 => {
                let res = self.ctrl;
                self.ctrl &= !(1 << 16); // Reading CTRL clears COUNTFLAG
                res
            }
            0x04 => self.load,
            0x08 => {
                self.val = self.val.wrapping_sub(100);
                self.val
            }
            0x0C => self.calib,
            _ => 0,
        }
    }

    fn write(&mut self, offset: u32, _size: usize, value: u32) {
        match offset {
            0x00 => self.ctrl = value,
            0x04 => self.load = value,
            0x08 => self.val = 0, // Writing any value clears VAL
            _ => {}
        }
    }
}
