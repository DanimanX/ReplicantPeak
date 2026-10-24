//! Peripheral bus and MMIO stubs for STM32F4 microcontroller.

pub mod bus;
pub mod display;
pub mod gpio;
pub mod pwr;
pub mod rcc;
pub mod spi;
pub mod systick;
pub mod traits;
pub mod uart;

pub use bus::{MmioBus, MmioLogEntry};
pub use display::{OledDisplay, DISPLAY_HEIGHT, DISPLAY_WIDTH};
pub use traits::MmioDevice;
