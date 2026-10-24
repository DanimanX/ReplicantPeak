//! MMIO device interface trait.

/// Trait implemented by virtual MMIO peripherals.
pub trait MmioDevice: Send + Sync {
    /// Return the name of the peripheral for debugging/logging.
    fn name(&self) -> &'static str;

    /// Read an MMIO register at the given byte offset from peripheral base.
    /// `size` is in bytes (1, 2, or 4).
    fn read(&mut self, offset: u32, size: usize) -> u32;

    /// Write an MMIO register at the given byte offset from peripheral base.
    /// `size` is in bytes (1, 2, or 4).
    fn write(&mut self, offset: u32, size: usize, value: u32);
}
