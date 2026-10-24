//! ARM Cortex-M Unicorn-based CPU emulator.

use crate::loader::FirmwareBinary;
use crate::memory::*;
use crate::mmio::MmioBus;
use unicorn_engine::unicorn_const::{Arch, HookType, MemType, Mode, Prot};
use unicorn_engine::{RegisterARM, Unicorn};

/// Context shared with Unicorn hooks.
pub struct CpuContext {
    pub mmio: MmioBus,
    pub instructions_executed: u64,
}

pub struct Emulator {
    uc: Unicorn<'static, CpuContext>,
    pub binary: FirmwareBinary,
}

impl Emulator {
    pub fn new(binary: FirmwareBinary, print_uart: bool) -> Result<Self, String> {
        let context = CpuContext {
            mmio: MmioBus::new(print_uart),
            instructions_executed: 0,
        };

        let mut uc = Unicorn::new_with_data(Arch::ARM, Mode::THUMB, context)
            .map_err(|e| format!("Failed to initialize Unicorn engine: {e:?}"))?;

        // 1. Map Boot Alias (0x00000000) for Cortex-M vector table alias
        uc.mem_map(0x0000_0000, 0x10000, Prot::ALL)
            .map_err(|e| format!("Failed to map Boot Alias at 0x0: {e:?}"))?;
        let _ = uc.mem_write(0x0000_0000, &(binary.initial_sp as u32).to_le_bytes());
        let _ = uc.mem_write(0x0000_0004, &(binary.entry_point as u32 | 1).to_le_bytes());

        // 2. Map Flash memory (0x08000000)
        uc.mem_map(FLASH_BASE, FLASH_SIZE as u64, Prot::ALL)
            .map_err(|e| format!("Failed to map Flash: {e:?}"))?;
        uc.mem_write(binary.base_addr, &binary.data)
            .map_err(|e| format!("Failed to write Flash data: {e:?}"))?;

        // 3. Map SRAM memory (0x20000000)
        uc.mem_map(SRAM_BASE, SRAM_SIZE as u64, Prot::ALL)
            .map_err(|e| format!("Failed to map SRAM: {e:?}"))?;

        // 4. Map MMIO peripheral spaces (APB1, APB2, AHB1, Cortex-M System)
        uc.mem_map(APB1_BASE, APB1_SIZE as u64, Prot::ALL)
            .map_err(|e| format!("Failed to map APB1: {e:?}"))?;
        uc.mem_map(APB2_BASE, APB2_SIZE as u64, Prot::ALL)
            .map_err(|e| format!("Failed to map APB2: {e:?}"))?;
        uc.mem_map(AHB1_BASE, AHB1_SIZE as u64, Prot::ALL)
            .map_err(|e| format!("Failed to map AHB1: {e:?}"))?;
        uc.mem_map(CORTEX_M_BASE, CORTEX_M_SIZE as u64, Prot::ALL)
            .map_err(|e| format!("Failed to map Cortex-M space: {e:?}"))?;

        // 5. Map Bit-Band Alias regions (32 MB each)
        uc.mem_map(BITBAND_PERIPH_BASE, BITBAND_PERIPH_SIZE as u64, Prot::ALL)
            .map_err(|e| format!("Failed to map Peripheral Bit-Band: {e:?}"))?;
        uc.mem_map(BITBAND_SRAM_BASE, BITBAND_SRAM_SIZE as u64, Prot::ALL)
            .map_err(|e| format!("Failed to map SRAM Bit-Band: {e:?}"))?;

        // 6. Install MMIO Hooks
        // APB1 & APB2 & AHB1 (0x40000000 .. 0x40030000)
        uc.add_mem_hook(
            HookType::MEM_READ | HookType::MEM_WRITE,
            0x4000_0000,
            0x4003_0000,
            mmio_hook,
        )
        .map_err(|e| format!("Failed to add MMIO hook: {e:?}"))?;

        // Bit-Band region (0x42000000 .. 0x44000000)
        uc.add_mem_hook(
            HookType::MEM_READ | HookType::MEM_WRITE,
            BITBAND_PERIPH_BASE,
            BITBAND_PERIPH_BASE + BITBAND_PERIPH_SIZE as u64,
            bitband_hook,
        )
        .map_err(|e| format!("Failed to add Bit-Band hook: {e:?}"))?;

        // Cortex-M Internal space (0xE0000000 .. 0xE0010000)
        uc.add_mem_hook(
            HookType::MEM_READ | HookType::MEM_WRITE,
            0xE000_0000,
            0xE001_0000,
            mmio_hook,
        )
        .map_err(|e| format!("Failed to add Cortex-M MMIO hook: {e:?}"))?;

        // 7. Code hook for tracking instructions
        uc.add_code_hook(0, 0xFFFF_FFFF, code_hook)
            .map_err(|e| format!("Failed to add code hook: {e:?}"))?;

        // 6. Initialize CPU Registers
        let entry_thumb = binary.entry_point | 1;
        uc.reg_write(RegisterARM::SP, binary.initial_sp)
            .map_err(|e| format!("Failed to set SP: {e:?}"))?;
        uc.reg_write(RegisterARM::PC, entry_thumb)
            .map_err(|e| format!("Failed to set PC: {e:?}"))?;
        uc.reg_write(RegisterARM::LR, 0xFFFF_FFFF)
            .map_err(|e| format!("Failed to set LR: {e:?}"))?;
        // Set Thumb bit (bit 24) in xPSR
        uc.reg_write(RegisterARM::XPSR, 0x0100_0000)
            .map_err(|e| format!("Failed to set xPSR: {e:?}"))?;

        Ok(Self { uc, binary })
    }

    /// Execute up to `count` instructions.
    pub fn step(&mut self, count: usize) -> Result<(), String> {
        let pc = self.uc.reg_read(RegisterARM::PC).unwrap_or(0);
        let pc_thumb = pc | 1;
        self.uc
            .emu_start(pc_thumb, 0xFFFF_FFFF, 0, count)
            .map_err(|e| format!("Emulation error at PC 0x{pc:08x}: {e:?}"))
    }

    /// Read all main registers.
    pub fn dump_registers(&self) -> String {
        let r0 = self.uc.reg_read(RegisterARM::R0).unwrap_or(0);
        let r1 = self.uc.reg_read(RegisterARM::R1).unwrap_or(0);
        let r2 = self.uc.reg_read(RegisterARM::R2).unwrap_or(0);
        let r3 = self.uc.reg_read(RegisterARM::R3).unwrap_or(0);
        let r4 = self.uc.reg_read(RegisterARM::R4).unwrap_or(0);
        let r5 = self.uc.reg_read(RegisterARM::R5).unwrap_or(0);
        let r6 = self.uc.reg_read(RegisterARM::R6).unwrap_or(0);
        let r7 = self.uc.reg_read(RegisterARM::R7).unwrap_or(0);
        let r8 = self.uc.reg_read(RegisterARM::R8).unwrap_or(0);
        let r9 = self.uc.reg_read(RegisterARM::R9).unwrap_or(0);
        let r10 = self.uc.reg_read(RegisterARM::R10).unwrap_or(0);
        let r11 = self.uc.reg_read(RegisterARM::R11).unwrap_or(0);
        let r12 = self.uc.reg_read(RegisterARM::R12).unwrap_or(0);
        let sp = self.uc.reg_read(RegisterARM::SP).unwrap_or(0);
        let lr = self.uc.reg_read(RegisterARM::LR).unwrap_or(0);
        let pc = self.uc.reg_read(RegisterARM::PC).unwrap_or(0);
        let xpsr = self.uc.reg_read(RegisterARM::XPSR).unwrap_or(0);
        let count = self.uc.get_data().instructions_executed;

        format!(
            "Instructions: {count}\n\
             PC:   0x{pc:08X}   LR: 0x{lr:08X}   SP: 0x{sp:08X}   xPSR: 0x{xpsr:08X}\n\
             R0:   0x{r0:08X}   R1: 0x{r1:08X}   R2: 0x{r2:08X}   R3:   0x{r3:08X}\n\
             R4:   0x{r4:08X}   R5: 0x{r5:08X}   R6: 0x{r6:08X}   R7:   0x{r7:08X}\n\
             R8:   0x{r8:08X}   R9: 0x{r9:08X}   R10:0x{r10:08X}  R11:  0x{r11:08X}  R12: 0x{r12:08X}"
        )
    }

    pub fn get_pc(&self) -> u64 {
        self.uc.reg_read(RegisterARM::PC).unwrap_or(0)
    }

    pub fn get_sp(&self) -> u64 {
        self.uc.reg_read(RegisterARM::SP).unwrap_or(0)
    }

    pub fn get_lr(&self) -> u64 {
        self.uc.reg_read(RegisterARM::LR).unwrap_or(0)
    }

    pub fn get_r0(&self) -> u64 {
        self.uc.reg_read(RegisterARM::R0).unwrap_or(0)
    }

    pub fn get_instructions_count(&self) -> u64 {
        self.uc.get_data().instructions_executed
    }

    pub fn context(&self) -> &CpuContext {
        self.uc.get_data()
    }

    pub fn context_mut(&mut self) -> &mut CpuContext {
        self.uc.get_data_mut()
    }
}

/// Bit-Band alias hook mapping 32MB alias to bit manipulation in peripheral space.
fn bitband_hook(
    uc: &mut Unicorn<'_, CpuContext>,
    mem_type: MemType,
    address: u64,
    size: usize,
    value: i64,
) -> bool {
    let rel = address - BITBAND_PERIPH_BASE;
    let byte_offset = rel / 32;
    let bit_idx = (rel % 32) / 4;
    let target_addr = 0x4000_0000 + (byte_offset & !3);
    let in_word_bit = (byte_offset % 4) * 8 + bit_idx;

    match mem_type {
        MemType::READ => {
            let current = uc.get_data_mut().mmio.read(target_addr, 4);
            let bit_val = (current >> in_word_bit) & 1;
            let bytes = bit_val.to_le_bytes();
            let slice = &bytes[..size.min(4)];
            let _ = uc.mem_write(address, slice);
        }
        MemType::WRITE => {
            let bit_val = (value as u32) & 1;
            let mut current = uc.get_data_mut().mmio.read(target_addr, 4);
            if bit_val == 1 {
                current |= 1 << in_word_bit;
            } else {
                current &= !(1 << in_word_bit);
            }
            uc.get_data_mut().mmio.write(target_addr, 4, current);
        }
        _ => {}
    }
    true
}

/// MMIO interceptor callback for Unicorn.
fn mmio_hook(
    uc: &mut Unicorn<'_, CpuContext>,
    mem_type: MemType,
    address: u64,
    size: usize,
    value: i64,
) -> bool {
    match mem_type {
        MemType::READ => {
            let val = uc.get_data_mut().mmio.read(address, size);
            // Write value to emulated memory before CPU instruction completes the read
            let bytes = val.to_le_bytes();
            let slice = &bytes[..size.min(4)];
            let _ = uc.mem_write(address, slice);
        }
        MemType::WRITE => {
            let val = (value as u64 & 0xFFFF_FFFF) as u32;
            uc.get_data_mut().mmio.write(address, size, val);
        }
        _ => {}
    }
    true
}

/// Code hook for instruction count, timer ticks, and stepping.
fn code_hook(uc: &mut Unicorn<'_, CpuContext>, _address: u64, _size: u32) {
    let ctx = uc.get_data_mut();
    ctx.instructions_executed += 1;
    let count = ctx.instructions_executed;

    // Simulate 1 ms hardware timer tick every 1000 instructions to service delays
    if count % 1000 == 0 {
        let tick_addr = 0x2000_3BA2;
        let mut buf = [0u8; 2];
        if uc.mem_read(tick_addr, &mut buf).is_ok() {
            let val = u16::from_le_bytes(buf).wrapping_add(1);
            let _ = uc.mem_write(tick_addr, &val.to_le_bytes());
        }
    }
}
