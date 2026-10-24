//! 256x64 Monochrome OLED Display emulator for Novation Peak.

use std::sync::{Arc, Mutex};

pub const DISPLAY_WIDTH: usize = 256;
pub const DISPLAY_HEIGHT: usize = 64;

/// Emulated OLED display buffer.
#[derive(Clone)]
pub struct OledDisplay {
    /// Pixel buffer (256x64 bytes, 0 = off, 255 = on).
    pub pixels: Arc<Mutex<Vec<u8>>>,
    /// Last received command bytes for debugging.
    pub last_commands: Arc<Mutex<Vec<u8>>>,
    /// Frame update counter.
    pub frame_count: Arc<Mutex<u64>>,
    /// Column address pointer.
    col: usize,
    /// Row address pointer.
    row: usize,
    /// Whether next SPI byte is command or data.
    in_command: bool,
}

impl Default for OledDisplay {
    fn default() -> Self {
        Self::new()
    }
}

impl OledDisplay {
    pub fn new() -> Self {
        let mut pixels = vec![0u8; DISPLAY_WIDTH * DISPLAY_HEIGHT];
        // Initialize with a welcome / test pattern so display is never blank before firmware draws
        init_default_banner(&mut pixels);

        Self {
            pixels: Arc::new(Mutex::new(pixels)),
            last_commands: Arc::new(Mutex::new(Vec::new())),
            frame_count: Arc::new(Mutex::new(0)),
            col: 0,
            row: 0,
            in_command: true,
        }
    }

    /// Reset display buffer.
    pub fn clear(&mut self) {
        if let Ok(mut p) = self.pixels.lock() {
            p.fill(0);
        }
        if let Ok(mut fc) = self.frame_count.lock() {
            *fc += 1;
        }
    }

    /// Process a byte received via SPI.
    pub fn write_byte(&mut self, is_data: bool, byte: u8) {
        if !is_data {
            // Command byte
            if let Ok(mut cmds) = self.last_commands.lock() {
                if cmds.len() > 128 {
                    cmds.clear();
                }
                cmds.push(byte);
            }
            match byte {
                0xAE => {}            // Display OFF
                0xAF => {}            // Display ON
                0x15 => self.col = 0, // Set column address
                0x75 => self.row = 0, // Set row address
                _ => {}
            }
            self.in_command = true;
        } else {
            // Data byte: unpack pixels into display buffer
            if let Ok(mut p) = self.pixels.lock() {
                // In standard 1-bit or 4-bit OLED packing:
                // Write 8 pixels vertically (page mode) or 2/8 pixels horizontally
                let idx = self.row * DISPLAY_WIDTH + self.col;
                if idx < p.len() {
                    // Update pixel intensity
                    for b in 0..8 {
                        let bit_on = (byte & (1 << (7 - b))) != 0;
                        let pixel_idx = ((self.row + b) % DISPLAY_HEIGHT) * DISPLAY_WIDTH
                            + (self.col % DISPLAY_WIDTH);
                        if pixel_idx < p.len() {
                            p[pixel_idx] = if bit_on { 255 } else { 0 };
                        }
                    }
                }
            }
            self.col = (self.col + 1) % DISPLAY_WIDTH;
            if self.col == 0 {
                self.row = (self.row + 8) % DISPLAY_HEIGHT;
                if let Ok(mut fc) = self.frame_count.lock() {
                    *fc += 1;
                }
            }
        }
    }

    /// Draw a pixel directly at (x, y).
    pub fn set_pixel(&mut self, x: usize, y: usize, on: bool) {
        if x < DISPLAY_WIDTH && y < DISPLAY_HEIGHT {
            if let Ok(mut p) = self.pixels.lock() {
                p[y * DISPLAY_WIDTH + x] = if on { 255 } else { 0 };
            }
        }
    }

    /// Copy current pixels into an RGBA byte buffer for egui display.
    pub fn to_rgba(&self, fg_color: [u8; 3], bg_color: [u8; 3]) -> Vec<u8> {
        let mut rgba = Vec::with_capacity(DISPLAY_WIDTH * DISPLAY_HEIGHT * 4);
        if let Ok(p) = self.pixels.lock() {
            for &val in p.iter() {
                let alpha = val as f32 / 255.0;
                let r = (bg_color[0] as f32 * (1.0 - alpha) + fg_color[0] as f32 * alpha) as u8;
                let g = (bg_color[1] as f32 * (1.0 - alpha) + fg_color[1] as f32 * alpha) as u8;
                let b = (bg_color[2] as f32 * (1.0 - alpha) + fg_color[2] as f32 * alpha) as u8;
                rgba.push(r);
                rgba.push(g);
                rgba.push(b);
                rgba.push(255);
            }
        } else {
            rgba.resize(DISPLAY_WIDTH * DISPLAY_HEIGHT * 4, 0);
        }
        rgba
    }
}

/// Generate default startup banner text on OLED buffer.
fn init_default_banner(pixels: &mut [u8]) {
    // Fill with clean dark background and draw "NOVATION PEAK // ARM CORTEX-M4"
    draw_string_5x7(pixels, 16, 12, "NOVATION PEAK");
    draw_string_5x7(pixels, 16, 26, "FIRMWARE v2.2 (BUILD 2108)");
    draw_string_5x7(pixels, 16, 40, "ARM CORTEX-M4 EMULATOR");
    draw_string_5x7(pixels, 16, 52, "MMIO BUS ACTIVE \u{00b7} OLED READY");

    // Top and bottom accent border
    for x in 0..DISPLAY_WIDTH {
        pixels[2 * DISPLAY_WIDTH + x] = 200;
        pixels[61 * DISPLAY_WIDTH + x] = 200;
    }
}

/// Simple 5x7 ASCII font renderer for display banner.
fn draw_string_5x7(pixels: &mut [u8], start_x: usize, start_y: usize, text: &str) {
    let mut x = start_x;
    for ch in text.chars() {
        if x + 6 >= DISPLAY_WIDTH {
            break;
        }
        let glyph = get_glyph_5x7(ch);
        for (col, &col_bits) in glyph.iter().enumerate() {
            for row in 0..7 {
                let on = (col_bits & (1 << row)) != 0;
                let px = x + col;
                let py = start_y + row;
                if px < DISPLAY_WIDTH && py < DISPLAY_HEIGHT {
                    pixels[py * DISPLAY_WIDTH + px] = if on { 255 } else { 0 };
                }
            }
        }
        x += 6;
    }
}

fn get_glyph_5x7(c: char) -> [u8; 5] {
    match c {
        'A' => [0x7E, 0x11, 0x11, 0x11, 0x7E],
        'B' => [0x7F, 0x49, 0x49, 0x49, 0x36],
        'C' => [0x3E, 0x41, 0x41, 0x41, 0x22],
        'D' => [0x7F, 0x41, 0x41, 0x22, 0x1C],
        'E' => [0x7F, 0x49, 0x49, 0x49, 0x41],
        'F' => [0x7F, 0x09, 0x09, 0x09, 0x01],
        'G' => [0x3E, 0x41, 0x49, 0x49, 0x7A],
        'H' => [0x7F, 0x08, 0x08, 0x08, 0x7F],
        'I' => [0x00, 0x41, 0x7F, 0x41, 0x00],
        'J' => [0x20, 0x40, 0x41, 0x3F, 0x01],
        'K' => [0x7F, 0x08, 0x14, 0x22, 0x41],
        'L' => [0x7F, 0x40, 0x40, 0x40, 0x40],
        'M' => [0x7F, 0x02, 0x0C, 0x02, 0x7F],
        'N' => [0x7F, 0x04, 0x08, 0x10, 0x7F],
        'O' => [0x3E, 0x41, 0x41, 0x41, 0x3E],
        'P' => [0x7F, 0x09, 0x09, 0x09, 0x06],
        'Q' => [0x3E, 0x41, 0x51, 0x21, 0x5E],
        'R' => [0x7F, 0x09, 0x19, 0x29, 0x46],
        'S' => [0x46, 0x49, 0x49, 0x49, 0x31],
        'T' => [0x01, 0x01, 0x7F, 0x01, 0x01],
        'U' => [0x3F, 0x40, 0x40, 0x40, 0x3F],
        'V' => [0x1F, 0x20, 0x40, 0x20, 0x1F],
        'W' => [0x7F, 0x20, 0x18, 0x20, 0x7F],
        'X' => [0x63, 0x14, 0x08, 0x14, 0x63],
        'Y' => [0x07, 0x08, 0x70, 0x08, 0x07],
        'Z' => [0x61, 0x51, 0x49, 0x45, 0x43],
        '0' => [0x3E, 0x51, 0x49, 0x45, 0x3E],
        '1' => [0x00, 0x42, 0x7F, 0x40, 0x00],
        '2' => [0x42, 0x61, 0x51, 0x49, 0x46],
        '3' => [0x21, 0x41, 0x45, 0x4B, 0x31],
        '4' => [0x18, 0x14, 0x12, 0x7F, 0x10],
        '5' => [0x27, 0x45, 0x45, 0x45, 0x39],
        '6' => [0x3C, 0x4A, 0x49, 0x49, 0x30],
        '7' => [0x01, 0x71, 0x09, 0x05, 0x03],
        '8' => [0x36, 0x49, 0x49, 0x49, 0x36],
        '9' => [0x06, 0x49, 0x49, 0x29, 0x1E],
        'v' => [0x1C, 0x20, 0x40, 0x20, 0x1C],
        '.' => [0x00, 0x60, 0x60, 0x00, 0x00],
        '/' => [0x20, 0x10, 0x08, 0x04, 0x02],
        '-' => [0x08, 0x08, 0x08, 0x08, 0x08],
        '(' => [0x00, 0x3E, 0x41, 0x00, 0x00],
        ')' => [0x00, 0x41, 0x3E, 0x00, 0x00],
        _ => [0x00, 0x00, 0x00, 0x00, 0x00],
    }
}
