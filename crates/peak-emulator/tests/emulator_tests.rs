#[cfg(test)]
mod tests {
    use peak_emulator::cpu::Emulator;
    use peak_emulator::loader::{unpack_7to8, FirmwareBinary};
    use peak_emulator::mmio::{MmioBus, DISPLAY_HEIGHT, DISPLAY_WIDTH};

    #[test]
    fn test_unpack_7to8_known_pattern() {
        // 37 bytes of 7-bit MIDI data should decode to 32 bytes of 8-bit data
        let mut sample_7bit = [0x55u8; 37];
        // Ensure MSB is 0
        for b in &mut sample_7bit {
            *b &= 0x7F;
        }
        let mut out = Vec::new();
        unpack_7to8(&sample_7bit, &mut out);
        assert_eq!(out.len(), 32);
    }

    fn find_firmware_path() -> Option<&'static str> {
        let candidates = [
            "firmware/peak-firmware-2.2-build2108.syx",
            "../../firmware/peak-firmware-2.2-build2108.syx",
            "../firmware/peak-firmware-2.2-build2108.syx",
        ];
        candidates
            .into_iter()
            .find(|p| std::path::Path::new(p).exists())
    }

    #[test]
    fn test_load_peak_firmware_sysex() {
        let Some(path) = find_firmware_path() else {
            eprintln!("skipping: real firmware not present in CI environment");
            return;
        };
        let binary = FirmwareBinary::from_file(path).expect("Failed to load firmware");
        assert_eq!(binary.data.len(), 249056);
        assert_eq!(binary.base_addr, 0x0800_0000);
        assert_eq!(binary.entry_point, 0x0801_16A4);
        assert_eq!(binary.initial_sp, 0x2003_0000);
    }

    #[test]
    fn test_mmio_rcc_pwr_behavior() {
        let mut bus = MmioBus::new(false);

        // RCC CR: when PLLON and HSEON are written, HSERDY and PLLRDY should be ready
        bus.write(0x4002_3800, 4, (1 << 16) | (1 << 24));
        let cr = bus.read(0x4002_3800, 4);
        assert_ne!(cr & (1 << 17), 0, "HSERDY should be set");
        assert_ne!(cr & (1 << 25), 0, "PLLRDY should be set");

        // PWR CSR: VOSRDY and BRR should be set
        let csr = bus.read(0x4000_7004, 4);
        assert_ne!(csr & (1 << 14), 0, "VOSRDY should be set");
        assert_ne!(csr & (1 << 3), 0, "BRR should be set");
    }

    #[test]
    fn test_mmio_gpio_and_leds() {
        let mut bus = MmioBus::new(false);
        // Test BSRR lower 16 bits (set) and upper 16 bits (reset)
        bus.write(0x4002_0418, 4, 0x0000_0001); // Set pin 0
        assert!(bus.gpiob.get_pin_output(0));

        bus.write(0x4002_0418, 4, 0x0001_0000); // Reset pin 0
        assert!(!bus.gpiob.get_pin_output(0));
    }

    #[test]
    fn test_oled_display_buffer() {
        let bus = MmioBus::new(false);
        if let Ok(mut disp) = bus.display.lock() {
            disp.set_pixel(10, 10, true);
            let rgba = disp.to_rgba([255, 255, 255], [0, 0, 0]);
            assert_eq!(rgba.len(), DISPLAY_WIDTH * DISPLAY_HEIGHT * 4);
        };
    }

    #[test]
    fn test_cpu_execution_synthetic() {
        // Test ARM Cortex-M execution using a synthetic binary
        // Vector table (first 8 bytes) + code:
        // Initial SP: 0x20030000
        // Reset Vector: 0x08000009 (Thumb bit set)
        // Code at 0x08000008:
        // 0x202a: movs r0, #42
        // 0x3001: adds r0, #1
        // 0x4770: bx lr
        let mut code = vec![0u8; 64];
        code[0..4].copy_from_slice(&0x2003_0000u32.to_le_bytes());
        code[4..8].copy_from_slice(&0x0800_0009u32.to_le_bytes());
        code[8..10].copy_from_slice(&0x202Au16.to_le_bytes()); // movs r0, #42
        code[10..12].copy_from_slice(&0x3001u16.to_le_bytes()); // adds r0, #1
        code[12..14].copy_from_slice(&0x4770u16.to_le_bytes()); // bx lr

        let binary = FirmwareBinary {
            data: code,
            base_addr: 0x0800_0000,
            entry_point: 0x0800_0008,
            initial_sp: 0x2003_0000,
            vector_table: 0x0800_0000,
        };

        let mut emu = Emulator::new(binary, false).expect("Failed to initialize emulator");
        emu.step(2).expect("Execution step failed");
        assert_eq!(emu.get_r0(), 43, "R0 should be 42 + 1 = 43");
        assert_eq!(emu.get_instructions_count(), 2);
    }

    #[test]
    fn test_cpu_execution_steps() {
        let Some(path) = find_firmware_path() else {
            eprintln!("skipping: real firmware not present in CI environment");
            return;
        };
        let binary = FirmwareBinary::from_file(path).expect("Failed to load firmware");
        let mut emu = Emulator::new(binary, false).expect("Failed to create emulator");

        // Execute 10,000 instructions
        emu.step(10000).expect("Step failed");
        assert!(emu.get_instructions_count() >= 10000);
        assert_eq!(emu.get_sp(), 0x2003_0000);
    }
}
