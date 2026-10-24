//! CLI and GUI launcher for the Peak firmware emulator.

use clap::Parser;
use peak_emulator::cpu::Emulator;
use peak_emulator::gui::launch_gui;
use peak_emulator::loader::FirmwareBinary;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "emulate", about = "Novation Peak ARM Cortex-M Microcontroller Emulator")]
struct Args {
    /// Path to Novation Peak firmware (.syx or raw .bin)
    #[arg(default_value = "firmware/peak-firmware-2.2-build2108.syx")]
    firmware: PathBuf,

    /// Run in headless mode without graphical UI
    #[arg(long, default_value_t = false)]
    headless: bool,

    /// Number of CPU instructions to execute in headless mode
    #[arg(short, long, default_value_t = 100_000)]
    steps: usize,

    /// Print instruction trace and register dump
    #[arg(short, long, default_value_t = false)]
    trace: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let args = Args::parse();

    println!("============================================================");
    println!("     NOVATION PEAK FIRMWARE EMULATOR (ARM CORTEX-M4)       ");
    println!("============================================================");
    println!("Loading firmware from: {}", args.firmware.display());

    let binary = FirmwareBinary::from_file(&args.firmware)?;
    println!("Unpacked payload size: {} bytes ({:.1} KB)", binary.data.len(), binary.data.len() as f32 / 1024.0);
    println!("Mapped Base Address:   0x{:08X}", binary.base_addr);
    println!("Reset Entry Point:     0x{:08X}", binary.entry_point);
    println!("Initial Stack Pointer: 0x{:08X}", binary.initial_sp);

    if !args.headless {
        println!("Launching GUI with Novation Peak LED & OLED hardware panel...");
        if let Err(e) = launch_gui(binary) {
            eprintln!("Failed to launch GUI: {e}");
        }
        return Ok(());
    }

    println!("\nInitializing CPU core (Unicorn ARM Thumb)...");
    let mut emulator = Emulator::new(binary, true)
        .map_err(|e| format!("Initialization error: {e}"))?;

    println!("Starting execution for {} steps...\n", args.steps);
    let start_time = std::time::Instant::now();

    let step_batch = 10_000.min(args.steps);
    let mut executed = 0;

    while executed < args.steps {
        let count = step_batch.min(args.steps - executed);
        match emulator.step(count) {
            Ok(()) => executed += count,
            Err(e) => {
                println!("\n[!] Execution halted: {e}");
                break;
            }
        }
        if args.trace {
            println!("--- Checkpoint: {} instructions ---", executed);
            println!("{}", emulator.dump_registers());
        }
    }

    let elapsed = start_time.elapsed();
    let instructions = emulator.get_instructions_count();
    let mips = (instructions as f64 / elapsed.as_secs_f64()) / 1_000_000.0;

    println!("\n============================================================");
    println!("EMULATION COMPLETE");
    println!("Total instructions: {}", instructions);
    println!("Elapsed time:       {:.2?}", elapsed);
    println!("Performance:        {:.2} MIPS", mips);
    println!("Final PC:           0x{:08X}", emulator.get_pc());
    println!("Final SP:           0x{:08X}", emulator.get_sp());
    println!("\nFinal Register State:\n{}", emulator.dump_registers());
    println!("============================================================");

    Ok(())
}
