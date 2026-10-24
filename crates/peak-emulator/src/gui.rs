//! Graphical interface featuring the Novation Peak LED / OLED hardware panel and CPU monitor.

use crate::cpu::Emulator;
use crate::loader::FirmwareBinary;
use crate::mmio::{DISPLAY_HEIGHT, DISPLAY_WIDTH};
use eframe::egui::{self, Color32, FontId, Pos2, Rect, Stroke, Vec2};
use std::sync::Arc;
use std::time::Instant;

pub fn launch_gui(binary: FirmwareBinary) -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1120.0, 760.0])
            .with_title("Novation Peak ARM Cortex-M Hardware Emulator"),
        ..Default::default()
    };

    eframe::run_native(
        "peak-emulator",
        options,
        Box::new(move |cc| {
            install_theme(&cc.egui_ctx);
            Ok(Box::new(EmulatorApp::new(binary)))
        }),
    )
}

struct EmulatorApp {
    emulator: Option<Emulator>,
    binary: FirmwareBinary,
    running: bool,
    steps_per_frame: usize,
    _last_frame_time: Instant,
    // Voice LEDs states (1 to 8)
    voice_leds: [bool; 8],
    animate1_led: bool,
    animate2_led: bool,
    filter_24db_led: bool,
    _active_octave: i8,
    error_msg: Option<String>,
}

impl EmulatorApp {
    fn new(binary: FirmwareBinary) -> Self {
        let emu = match Emulator::new(binary.clone(), false) {
            Ok(e) => Some(e),
            Err(err) => {
                eprintln!("Emulator init error: {err}");
                None
            }
        };

        Self {
            emulator: emu,
            binary,
            running: false,
            steps_per_frame: 5000,
            _last_frame_time: Instant::now(),
            voice_leds: [true, false, false, false, false, false, false, false],
            animate1_led: false,
            animate2_led: false,
            filter_24db_led: true,
            _active_octave: 0,
            error_msg: None,
        }
    }

    fn step_cpu(&mut self, steps: usize) {
        if let Some(emu) = &mut self.emulator {
            if let Err(e) = emu.step(steps) {
                self.error_msg = Some(e);
                self.running = false;
            } else {
                self.error_msg = None;
                // Update simulated voice activity based on instruction count / GPIO
                let count = emu.get_instructions_count();
                let active = ((count / 2000) % 8) as usize;
                for i in 0..8 {
                    self.voice_leds[i] = (i <= active) && (count > 0);
                }
            }
        }
    }
}

impl eframe::App for EmulatorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.running {
            self.step_cpu(self.steps_per_frame);
            ctx.request_repaint();
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.spacing_mut().item_spacing = Vec2::new(8.0, 8.0);

            // Title Bar
            ui.horizontal(|ui| {
                ui.heading(
                    egui::RichText::new("NOVATION PEAK HARDWARE EMULATOR")
                        .color(Color32::from_rgb(0, 220, 255))
                        .strong(),
                );
                ui.separator();
                ui.label(
                    egui::RichText::new("STM32F429 ARM CORTEX-M4 \u{00b7} FIRMWARE v2.2")
                        .color(Color32::from_rgb(180, 190, 200)),
                );
            });

            ui.add_space(4.0);

            // Top Section: The Novation Peak LED & OLED Hardware Front Panel
            self.draw_novation_peak_panel(ui);

            ui.add_space(8.0);

            // Bottom Section: CPU Inspector, Registers, Serial UART, and MMIO Log
            ui.columns(2, |cols| {
                // Left Column: CPU Controls and Registers
                cols[0].vertical(|ui| {
                    self.draw_cpu_controls(ui);
                    ui.add_space(6.0);
                    self.draw_registers_view(ui);
                });

                // Right Column: Serial Console and MMIO Trace
                cols[1].vertical(|ui| {
                    self.draw_uart_console(ui);
                    ui.add_space(6.0);
                    self.draw_mmio_log(ui);
                });
            });
        });
    }
}

impl EmulatorApp {
    /// Render the authentic Novation Peak LED and OLED hardware panel.
    fn draw_novation_peak_panel(&mut self, ui: &mut egui::Ui) {
        let panel_rect = ui.available_rect_before_wrap();
        let height = 240.0_f32;
        let rect = Rect::from_min_size(panel_rect.min, Vec2::new(panel_rect.width(), height));

        // 1. Panel Background (Peak textured brushed dark metal)
        ui.painter().rect_filled(rect, 4.0_f32, Color32::from_rgb(18, 20, 24));
        ui.painter().rect_stroke(
            rect,
            4.0_f32,
            Stroke::new(1.5_f32, Color32::from_rgb(45, 52, 60)),
            egui::StrokeKind::Inside,
        );

        // Header strip
        let header_rect = Rect::from_min_size(rect.min, Vec2::new(rect.width(), 26.0_f32));
        ui.painter().rect_filled(header_rect, 4.0_f32, Color32::from_rgb(26, 30, 36));
        ui.painter().text(
            Pos2::new(header_rect.left() + 14.0_f32, header_rect.center().y),
            egui::Align2::LEFT_CENTER,
            "NOVATION PEAK // FRONT PANEL DISPLAY & LED INDICATORS",
            FontId::monospace(11.0_f32),
            Color32::from_rgb(130, 150, 170),
        );

        // 2. Central 256x64 OLED Display Screen
        let display_scale = 2.0_f32;
        let oled_w = DISPLAY_WIDTH as f32 * display_scale; // 512px
        let oled_h = DISPLAY_HEIGHT as f32 * display_scale; // 128px
        let oled_x = rect.left() + 20.0_f32;
        let oled_y = rect.top() + 40.0_f32;
        let oled_rect = Rect::from_min_size(Pos2::new(oled_x, oled_y), Vec2::new(oled_w, oled_h));

        // Screen Bezel & Recessed Housing
        let bezel_rect = oled_rect.expand(6.0_f32);
        ui.painter().rect_filled(bezel_rect, 3.0_f32, Color32::from_rgb(8, 10, 12));
        ui.painter().rect_stroke(
            bezel_rect,
            3.0_f32,
            Stroke::new(1.0_f32, Color32::from_rgb(60, 68, 76)),
            egui::StrokeKind::Inside,
        );

        // Render OLED Display Pixels
        ui.painter().rect_filled(oled_rect, 0.0_f32, Color32::from_rgb(2, 6, 8));
        self.render_oled_pixels(ui.painter(), oled_rect);

        // Scanlines overlay for authentic retro look
        for y_step in (0..(oled_h as usize)).step_by(4) {
            let y = oled_rect.top() + y_step as f32;
            ui.painter().line_segment(
                [Pos2::new(oled_rect.left(), y), Pos2::new(oled_rect.right(), y)],
                Stroke::new(1.0_f32, Color32::from_rgba_premultiplied(0, 0, 0, 35)),
            );
        }

        // 3. Peak 8 Voice LEDs Block
        let voice_section_x = oled_rect.right() + 28.0_f32;
        let voice_section_y = oled_rect.top();
        self.draw_voice_leds_panel(ui.painter(), Pos2::new(voice_section_x, voice_section_y));

        // 4. Peak Hardware Status LEDs Block (Animate, Filter, Octave)
        let status_section_x = voice_section_x + 220.0_f32;
        self.draw_hardware_status_leds(ui.painter(), Pos2::new(status_section_x, voice_section_y));

        // 5. Hardware Navigation Buttons below display
        let buttons_y = oled_rect.bottom() + 14.0_f32;
        let btn_labels = ["< PAGE", "PAGE >", "PATCH", "EXIT", "INIT", "COMPARE"];
        let mut btn_x = oled_rect.left();
        for label in btn_labels {
            let btn_rect = Rect::from_min_size(Pos2::new(btn_x, buttons_y), Vec2::new(76.0_f32, 24.0_f32));
            let response = ui.allocate_rect(btn_rect, egui::Sense::click());
            let is_hovered = response.hovered();
            let is_clicked = response.clicked();

            let bg = if is_clicked {
                Color32::from_rgb(0, 160, 200)
            } else if is_hovered {
                Color32::from_rgb(48, 56, 68)
            } else {
                Color32::from_rgb(32, 38, 46)
            };

            ui.painter().rect_filled(btn_rect, 2.0_f32, bg);
            ui.painter().rect_stroke(
                btn_rect,
                2.0_f32,
                Stroke::new(1.0_f32, Color32::from_rgb(70, 80, 95)),
                egui::StrokeKind::Inside,
            );
            ui.painter().text(
                btn_rect.center(),
                egui::Align2::CENTER_CENTER,
                label,
                FontId::monospace(9.5_f32),
                if is_clicked { Color32::BLACK } else { Color32::from_rgb(200, 215, 230) },
            );

            if is_clicked {
                if let Some(emu) = &mut self.emulator {
                    emu.context_mut().mmio.gpioa.set_pin_input(0, false);
                }
            }

            btn_x += 84.0_f32;
        }

        ui.allocate_space(Vec2::new(rect.width(), height));
    }

    /// Draw the 8 Novation Peak Voice Activity LEDs.
    fn draw_voice_leds_panel(&self, painter: &egui::Painter, pos: Pos2) {
        let block_w = 190.0_f32;
        let block_h = 128.0_f32;
        let block_rect = Rect::from_min_size(pos, Vec2::new(block_w, block_h));

        painter.rect_filled(block_rect, 3.0_f32, Color32::from_rgb(12, 14, 18));
        painter.rect_stroke(
            block_rect,
            3.0_f32,
            Stroke::new(1.0_f32, Color32::from_rgb(40, 48, 56)),
            egui::StrokeKind::Inside,
        );

        // Section Title
        painter.text(
            Pos2::new(block_rect.left() + 10.0_f32, block_rect.top() + 10.0_f32),
            egui::Align2::LEFT_TOP,
            "VOICE ACTIVITY (8x LED)",
            FontId::monospace(9.5_f32),
            Color32::from_rgb(0, 200, 240),
        );

        // Grid of 8 LEDs: 2 rows of 4 LEDs
        let led_radius = 6.5_f32;
        let start_x = block_rect.left() + 26.0_f32;
        let start_y = block_rect.top() + 45.0_f32;
        let x_pitch = 40.0_f32;
        let y_pitch = 40.0_f32;

        for voice_idx in 0..8 {
            let col = voice_idx % 4;
            let row = voice_idx / 4;
            let center = Pos2::new(start_x + col as f32 * x_pitch, start_y + row as f32 * y_pitch);
            let is_lit = self.voice_leds[voice_idx];

            painter.circle_filled(center, led_radius + 2.0_f32, Color32::from_rgb(30, 36, 42));
            painter.circle_stroke(center, led_radius + 2.0_f32, Stroke::new(1.0_f32, Color32::from_rgb(60, 70, 80)));

            if is_lit {
                painter.circle_filled(center, led_radius + 5.0_f32, Color32::from_rgba_premultiplied(0, 220, 255, 60));
                painter.circle_filled(center, led_radius, Color32::from_rgb(100, 240, 255));
                painter.circle_filled(center, led_radius * 0.45_f32, Color32::WHITE);
            } else {
                painter.circle_filled(center, led_radius, Color32::from_rgb(15, 35, 45));
            }

            painter.text(
                Pos2::new(center.x, center.y + led_radius + 4.0_f32),
                egui::Align2::CENTER_TOP,
                format!("{}", voice_idx + 1),
                FontId::monospace(8.5_f32),
                if is_lit { Color32::from_rgb(0, 220, 255) } else { Color32::from_rgb(90, 105, 120) },
            );
        }
    }

    /// Draw other Peak hardware status LEDs (Animate, Filter Slope, Octaves).
    fn draw_hardware_status_leds(&mut self, painter: &egui::Painter, pos: Pos2) {
        let block_w = 260.0_f32;
        let block_h = 128.0_f32;
        let block_rect = Rect::from_min_size(pos, Vec2::new(block_w, block_h));

        painter.rect_filled(block_rect, 3.0_f32, Color32::from_rgb(12, 14, 18));
        painter.rect_stroke(
            block_rect,
            3.0_f32,
            Stroke::new(1.0_f32, Color32::from_rgb(40, 48, 56)),
            egui::StrokeKind::Inside,
        );

        painter.text(
            Pos2::new(block_rect.left() + 10.0_f32, block_rect.top() + 10.0_f32),
            egui::Align2::LEFT_TOP,
            "HARDWARE STATUS LEDS",
            FontId::monospace(9.5_f32),
            Color32::from_rgb(255, 170, 0),
        );

        let items = [
            ("ANIMATE 1", self.animate1_led, Color32::from_rgb(255, 80, 80)),
            ("ANIMATE 2", self.animate2_led, Color32::from_rgb(255, 80, 80)),
            ("FILTER 24dB", self.filter_24db_led, Color32::from_rgb(255, 200, 0)),
            ("ARP ON", self.running, Color32::from_rgb(0, 255, 120)),
        ];

        let mut y = block_rect.top() + 34.0_f32;
        for (label, active, color) in items {
            let led_center = Pos2::new(block_rect.left() + 20.0_f32, y);
            painter.circle_filled(led_center, 6.0_f32, Color32::from_rgb(25, 30, 35));
            painter.circle_stroke(led_center, 6.0_f32, Stroke::new(1.0_f32, Color32::from_rgb(55, 65, 75)));

            if active {
                painter.circle_filled(led_center, 9.0_f32, Color32::from_rgba_premultiplied(color.r(), color.g(), color.b(), 60));
                painter.circle_filled(led_center, 4.5_f32, color);
            } else {
                painter.circle_filled(led_center, 4.5_f32, Color32::from_rgb(30, 25, 25));
            }

            painter.text(
                Pos2::new(led_center.x + 14.0_f32, led_center.y),
                egui::Align2::LEFT_CENTER,
                label,
                FontId::monospace(9.0_f32),
                if active { Color32::WHITE } else { Color32::from_rgb(110, 120, 130) },
            );

            y += 22.0_f32;
        }
    }

    /// Render pixels from the OLED display buffer.
    fn render_oled_pixels(&self, painter: &egui::Painter, oled_rect: Rect) {
        if let Some(emu) = &self.emulator {
            let pixels_opt: Option<Vec<u8>> = {
                let display = Arc::clone(&emu.context().mmio.display);
                let x = if let Ok(disp) = display.lock() {
                    disp.pixels.lock().ok().map(|p| p.clone())
                } else {
                    None
                };
                x
            };

            if let Some(pixels) = pixels_opt {
                let pixel_w = oled_rect.width() / DISPLAY_WIDTH as f32;
                let pixel_h = oled_rect.height() / DISPLAY_HEIGHT as f32;

                for y in 0..DISPLAY_HEIGHT {
                    for x in 0..DISPLAY_WIDTH {
                        let val = pixels[y * DISPLAY_WIDTH + x];
                        if val > 16 {
                            let px = oled_rect.left() + x as f32 * pixel_w;
                            let py = oled_rect.top() + y as f32 * pixel_h;
                            let rect = Rect::from_min_size(Pos2::new(px, py), Vec2::new(pixel_w, pixel_h));

                            let alpha = val as f32 / 255.0_f32;
                            let color = Color32::from_rgba_premultiplied(
                                (20.0_f32 * alpha) as u8,
                                (220.0_f32 * alpha) as u8,
                                (255.0_f32 * alpha) as u8,
                                val,
                            );
                            painter.rect_filled(rect, 0.0_f32, color);
                        }
                    }
                }
            }
        }
    }

    /// CPU Controls (Play/Pause, Step, Speed).
    fn draw_cpu_controls(&mut self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.heading(egui::RichText::new("CPU CONTROL").font(FontId::monospace(12.0_f32)));
                ui.separator();

                let play_label = if self.running { "⏸ PAUSE" } else { "▶ RUN" };
                if ui.button(egui::RichText::new(play_label).strong()).clicked() {
                    self.running = !self.running;
                }

                if ui.button("STEP (1)").clicked() {
                    self.running = false;
                    self.step_cpu(1);
                }

                if ui.button("STEP (1,000)").clicked() {
                    self.running = false;
                    self.step_cpu(1000);
                }

                if ui.button("STEP (50,000)").clicked() {
                    self.running = false;
                    self.step_cpu(50000);
                }

                if ui.button("🔄 RESET").clicked() {
                    self.running = false;
                    self.emulator = Emulator::new(self.binary.clone(), false).ok();
                }

                ui.separator();
                ui.label("Speed:");
                ui.add(egui::Slider::new(&mut self.steps_per_frame, 500..=50000).logarithmic(true));
            });

            if let Some(err) = &self.error_msg {
                ui.colored_label(Color32::from_rgb(255, 80, 80), format!("Execution fault: {err}"));
            }
        });
    }

    /// Registers view (PC, SP, LR, xPSR, R0-R12).
    fn draw_registers_view(&self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.heading(egui::RichText::new("CPU REGISTERS & STATE").font(FontId::monospace(12.0_f32)));
                if let Some(emu) = &self.emulator {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new(format!("Instructions: {}", emu.get_instructions_count()))
                                .color(Color32::from_rgb(0, 220, 255)),
                        );
                    });
                }
            });

            if let Some(emu) = &self.emulator {
                let dump = emu.dump_registers();
                ui.add(
                    egui::TextEdit::multiline(&mut dump.as_str())
                        .font(FontId::monospace(10.5_f32))
                        .desired_rows(5)
                        .lock_focus(true),
                );
            }
        });
    }

    /// Serial UART output console.
    fn draw_uart_console(&self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.heading(egui::RichText::new("UART SERIAL CONSOLE (USART6 / UART4)").font(FontId::monospace(12.0_f32)));

            let text = if let Some(emu) = &self.emulator {
                let log_bytes: Option<Vec<u8>> = {
                    let log = Arc::clone(&emu.context().mmio.tx_log);
                    log.lock().ok().map(|b| b.clone())
                };
                if let Some(bytes) = log_bytes {
                    String::from_utf8_lossy(&bytes).to_string()
                } else {
                    String::new()
                }
            } else {
                String::new()
            };

            let display_text = if text.is_empty() {
                "[No UART transmission yet. Start CPU execution to observe serial output.]".to_string()
            } else {
                text
            };

            egui::ScrollArea::vertical()
                .max_height(140.0_f32)
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    ui.add(
                        egui::TextEdit::multiline(&mut display_text.as_str())
                            .font(FontId::monospace(10.0_f32))
                            .desired_rows(6)
                            .lock_focus(true),
                    );
                });
        });
    }

    /// MMIO Transaction Log.
    fn draw_mmio_log(&self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.heading(egui::RichText::new("MMIO BUS ACTIVITY LOG").font(FontId::monospace(12.0_f32)));

            let entries = if let Some(emu) = &self.emulator {
                let log_arc = Arc::clone(&emu.context().mmio.access_log);
                log_arc.lock().ok().map(|l| l.clone()).unwrap_or_default()
            } else {
                Vec::new()
            };

            egui::ScrollArea::vertical()
                .max_height(150.0_f32)
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    for entry in entries.iter().rev().take(40) {
                        let op = if entry.is_write { "WRITE" } else { "READ " };
                        let op_color = if entry.is_write {
                            Color32::from_rgb(255, 170, 0)
                        } else {
                            Color32::from_rgb(0, 200, 255)
                        };

                        ui.horizontal(|ui| {
                            ui.colored_label(op_color, op);
                            ui.monospace(format!(
                                "[{:8}] 0x{:08X} ({}b) = 0x{:08X}",
                                entry.peripheral, entry.address, entry.size * 8, entry.value
                            ));
                        });
                    }
                });
        });
    }
}

fn install_theme(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.override_text_color = Some(Color32::from_rgb(220, 230, 240));
    visuals.panel_fill = Color32::from_rgb(14, 16, 20);
    visuals.window_fill = Color32::from_rgb(18, 20, 26);
    ctx.set_visuals(visuals);
}
