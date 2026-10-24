//! Central MMIO Bus Dispatcher routing hardware register accesses.

use super::display::OledDisplay;
use super::gpio::GpioDevice;
use super::pwr::PwrDevice;
use super::rcc::RccDevice;
use super::spi::SpiDevice;
use super::systick::SysTickDevice;
use super::traits::MmioDevice;
use super::uart::UartDevice;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// A recorded MMIO transaction for tracing and UI displays.
#[derive(Clone, Debug)]
pub struct MmioLogEntry {
    pub is_write: bool,
    pub address: u64,
    pub size: usize,
    pub value: u32,
    pub peripheral: &'static str,
}

pub struct MmioBus {
    pub rcc: RccDevice,
    pub pwr: PwrDevice,
    pub uart4: UartDevice,
    pub usart6: UartDevice,
    pub gpioa: GpioDevice,
    pub gpiob: GpioDevice,
    pub gpioc: GpioDevice,
    pub gpiod: GpioDevice,
    pub gpioe: GpioDevice,
    pub spi1: SpiDevice,
    pub spi2: SpiDevice,
    pub spi3: SpiDevice,
    pub spi4: SpiDevice,
    pub systick: SysTickDevice,
    pub display: Arc<Mutex<OledDisplay>>,
    pub tx_log: Arc<Mutex<Vec<u8>>>,
    pub gpio_odr: Arc<Mutex<u32>>,
    /// Fallback storage for unmodelled registers.
    fallback_regs: HashMap<u64, u32>,
    /// Ring buffer of recent MMIO accesses.
    pub access_log: Arc<Mutex<Vec<MmioLogEntry>>>,
    pub log_enabled: bool,
}

impl MmioBus {
    pub fn new(print_uart: bool) -> Self {
        let display = Arc::new(Mutex::new(OledDisplay::new()));
        let tx_log = Arc::new(Mutex::new(Vec::new()));
        let gpio_odr = Arc::new(Mutex::new(0));

        Self {
            rcc: RccDevice::new(),
            pwr: PwrDevice::new(),
            uart4: UartDevice::new("UART4", Arc::clone(&tx_log), print_uart),
            usart6: UartDevice::new("USART6", Arc::clone(&tx_log), print_uart),
            gpioa: GpioDevice::new("GPIOA", Arc::clone(&gpio_odr)),
            gpiob: GpioDevice::new("GPIOB", Arc::clone(&gpio_odr)),
            gpioc: GpioDevice::new("GPIOC", Arc::clone(&gpio_odr)),
            gpiod: GpioDevice::new("GPIOD", Arc::clone(&gpio_odr)),
            gpioe: GpioDevice::new("GPIOE", Arc::clone(&gpio_odr)),
            spi1: SpiDevice::new("SPI1", Some(Arc::clone(&display))),
            spi2: SpiDevice::new("SPI2", None),
            spi3: SpiDevice::new("SPI3", None),
            spi4: SpiDevice::new("SPI4", None),
            systick: SysTickDevice::new(),
            display,
            tx_log,
            gpio_odr,
            fallback_regs: HashMap::new(),
            access_log: Arc::new(Mutex::new(Vec::with_capacity(1024))),
            log_enabled: true,
        }
    }

    /// Read an MMIO register at the specified address.
    pub fn read(&mut self, address: u64, size: usize) -> u32 {
        let (val, name) = match address {
            0x4002_3800..=0x4002_3BFF => (self.rcc.read((address - 0x4002_3800) as u32, size), "RCC"),
            0x4000_7000..=0x4000_73FF => (self.pwr.read((address - 0x4000_7000) as u32, size), "PWR"),
            0x4000_4C00..=0x4000_4FFF => (self.uart4.read((address - 0x4000_4C00) as u32, size), "UART4"),
            0x4001_1400..=0x4001_17FF => (self.usart6.read((address - 0x4001_1400) as u32, size), "USART6"),
            0x4001_1000..=0x4001_13FF => (self.usart6.read((address - 0x4001_1000) as u32, size), "USART1"),
            0x4002_0000..=0x4002_03FF => (self.gpioa.read((address - 0x4002_0000) as u32, size), "GPIOA"),
            0x4002_0400..=0x4002_07FF => (self.gpiob.read((address - 0x4002_0400) as u32, size), "GPIOB"),
            0x4002_0800..=0x4002_0BFF => (self.gpioc.read((address - 0x4002_0800) as u32, size), "GPIOC"),
            0x4002_0C00..=0x4002_0FFF => (self.gpiod.read((address - 0x4002_0C00) as u32, size), "GPIOD"),
            0x4002_1000..=0x4002_13FF => (self.gpioe.read((address - 0x4002_1000) as u32, size), "GPIOE"),
            0x4000_3800..=0x4000_3BFF => (self.spi2.read((address - 0x4000_3800) as u32, size), "SPI2"),
            0x4000_3C00..=0x4000_3FFF => (self.spi3.read((address - 0x4000_3C00) as u32, size), "SPI3"),
            0x4001_3000..=0x4001_33FF => (self.spi1.read((address - 0x4001_3000) as u32, size), "SPI1"),
            0x4001_3400..=0x4001_37FF => (self.spi4.read((address - 0x4001_3400) as u32, size), "SPI4"),
            0xE000_E010..=0xE000_E01F => (self.systick.read((address - 0xE000_E010) as u32, size), "SysTick"),
            0xE000_ED00..=0xE000_ED8F => {
                // System Control Block (e.g. VTOR at 0xE000ED08)
                let val = *self.fallback_regs.get(&address).unwrap_or(&0x0800_0000);
                (val, "SCB")
            }
            _ => {
                let val = *self.fallback_regs.get(&address).unwrap_or(&0);
                (val, "MMIO_UNKNOWN")
            }
        };

        if self.log_enabled {
            self.record_log(false, address, size, val, name);
        }
        val
    }

    /// Write an MMIO register at the specified address.
    pub fn write(&mut self, address: u64, size: usize, value: u32) {
        let name = match address {
            0x4002_3800..=0x4002_3BFF => {
                self.rcc.write((address - 0x4002_3800) as u32, size, value);
                "RCC"
            }
            0x4000_7000..=0x4000_73FF => {
                self.pwr.write((address - 0x4000_7000) as u32, size, value);
                "PWR"
            }
            0x4000_4C00..=0x4000_4FFF => {
                self.uart4.write((address - 0x4000_4C00) as u32, size, value);
                "UART4"
            }
            0x4001_1400..=0x4001_17FF => {
                self.usart6.write((address - 0x4001_1400) as u32, size, value);
                "USART6"
            }
            0x4001_1000..=0x4001_13FF => {
                self.usart6.write((address - 0x4001_1000) as u32, size, value);
                "USART1"
            }
            0x4002_0000..=0x4002_03FF => {
                self.gpioa.write((address - 0x4002_0000) as u32, size, value);
                "GPIOA"
            }
            0x4002_0400..=0x4002_07FF => {
                self.gpiob.write((address - 0x4002_0400) as u32, size, value);
                "GPIOB"
            }
            0x4002_0800..=0x4002_0BFF => {
                self.gpioc.write((address - 0x4002_0800) as u32, size, value);
                "GPIOC"
            }
            0x4002_0C00..=0x4002_0FFF => {
                self.gpiod.write((address - 0x4002_0C00) as u32, size, value);
                "GPIOD"
            }
            0x4002_1000..=0x4002_13FF => {
                self.gpioe.write((address - 0x4002_1000) as u32, size, value);
                "GPIOE"
            }
            0x4000_3800..=0x4000_3BFF => {
                self.spi2.write((address - 0x4000_3800) as u32, size, value);
                "SPI2"
            }
            0x4000_3C00..=0x4000_3FFF => {
                self.spi3.write((address - 0x4000_3C00) as u32, size, value);
                "SPI3"
            }
            0x4001_3000..=0x4001_33FF => {
                self.spi1.write((address - 0x4001_3000) as u32, size, value);
                "SPI1"
            }
            0x4001_3400..=0x4001_37FF => {
                self.spi4.write((address - 0x4001_3400) as u32, size, value);
                "SPI4"
            }
            0xE000_E010..=0xE000_E01F => {
                self.systick.write((address - 0xE000_E010) as u32, size, value);
                "SysTick"
            }
            _ => {
                self.fallback_regs.insert(address, value);
                "MMIO_UNKNOWN"
            }
        };

        if self.log_enabled {
            self.record_log(true, address, size, value, name);
        }
    }

    fn record_log(&self, is_write: bool, address: u64, size: usize, value: u32, peripheral: &'static str) {
        if let Ok(mut log) = self.access_log.lock() {
            if log.len() >= 500 {
                log.remove(0);
            }
            log.push(MmioLogEntry {
                is_write,
                address,
                size,
                value,
                peripheral,
            });
        }
    }
}
