//! Memory map definitions for the Peak STM32F4-based microcontroller.

/// Base address of on-chip Flash memory.
pub const FLASH_BASE: u64 = 0x0800_0000;
/// Size of Flash memory to map (512 KB, page-aligned).
pub const FLASH_SIZE: usize = 512 * 1024;

/// Base address of on-chip SRAM.
pub const SRAM_BASE: u64 = 0x2000_0000;
/// Size of SRAM to map (256 KB, accommodating 192 KB RAM + page alignment).
pub const SRAM_SIZE: usize = 256 * 1024;

/// Base address of APB1 peripheral space.
pub const APB1_BASE: u64 = 0x4000_0000;
pub const APB1_SIZE: usize = 0x10000; // 64 KB

/// Base address of APB2 peripheral space.
pub const APB2_BASE: u64 = 0x4001_0000;
pub const APB2_SIZE: usize = 0x10000; // 64 KB

/// Base address of AHB1 peripheral space (GPIO, RCC, DMA).
pub const AHB1_BASE: u64 = 0x4002_0000;
pub const AHB1_SIZE: usize = 0x10000; // 64 KB

/// Base address of Peripheral Bit-Band alias region (32 MB).
pub const BITBAND_PERIPH_BASE: u64 = 0x4200_0000;
pub const BITBAND_PERIPH_SIZE: usize = 32 * 1024 * 1024;

/// Base address of SRAM Bit-Band alias region (32 MB).
pub const BITBAND_SRAM_BASE: u64 = 0x2200_0000;
pub const BITBAND_SRAM_SIZE: usize = 32 * 1024 * 1024;

/// Base address of Cortex-M internal peripherals (SysTick, NVIC, SCB).
pub const CORTEX_M_BASE: u64 = 0xE000_0000;
pub const CORTEX_M_SIZE: usize = 0x10000; // 64 KB

/// Specific peripheral addresses.
pub mod peripherals {
    pub const TIM2_BASE: u64 = 0x4000_0000;
    pub const UART4_BASE: u64 = 0x4000_4C00;
    pub const I2C1_BASE: u64 = 0x4000_5400;
    pub const PWR_BASE: u64 = 0x4000_7000;

    pub const USART1_BASE: u64 = 0x4001_1000;
    pub const USART6_BASE: u64 = 0x4001_1400;
    pub const ADC1_BASE: u64 = 0x4001_2000;
    pub const SPI1_BASE: u64 = 0x4001_3000;
    pub const SPI4_BASE: u64 = 0x4001_3400;
    pub const SYSCFG_BASE: u64 = 0x4001_3800;

    pub const GPIOA_BASE: u64 = 0x4002_0000;
    pub const GPIOB_BASE: u64 = 0x4002_0400;
    pub const GPIOC_BASE: u64 = 0x4002_0800;
    pub const GPIOD_BASE: u64 = 0x4002_0C00;
    pub const GPIOE_BASE: u64 = 0x4002_1000;
    pub const RCC_BASE: u64 = 0x4002_3800;
    pub const DMA2_BASE: u64 = 0x4002_6400;

    pub const SYSTICK_BASE: u64 = 0xE000_E010;
    pub const NVIC_BASE: u64 = 0xE000_E100;
    pub const SCB_BASE: u64 = 0xE000_ED00;
}
