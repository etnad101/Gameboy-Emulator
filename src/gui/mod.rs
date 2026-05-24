mod components;
use std::cell::RefCell;

use eframe::Frame;
use egui::Context;

use crate::emulator::cartridge::Cartridge;
use crate::emulator::DMGBus;
use crate::emulator::{Emulator, RunType, SCREEN_HEIGHT, SCREEN_WIDTH};
use crate::gui::components::{emu_screen::EmuScreen, memory_editor::MemoryEditor};
use crate::Palette;

pub struct EmulatorGui {
    emulator: Emulator<DMGBus>,
    emu_screen: EmuScreen,
    memory_editor: MemoryEditor,
    run_type: RunType,
    show_debug_screen: bool,
}

impl EmulatorGui {
    pub fn new(emulator: Emulator<DMGBus>) -> Self {
        let run_type = emulator.run_type();
        let memory_editor = MemoryEditor::new(16, 0x10000, 0x100);
        Self {
            emulator,
            emu_screen: EmuScreen::new(SCREEN_WIDTH, SCREEN_HEIGHT),
            memory_editor,
            run_type,
            show_debug_screen: false,
        }
    }
}

impl eframe::App for EmulatorGui {
    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        self.emu_screen
            .update_texture(&self.emulator.tick().unwrap().rgb(), ctx);

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("Menu", |ui| {
                    if ui.button("Run").clicked() {
                        self.emulator.set_run_type(RunType::Frame);
                    }
                    if ui.button("Pause").clicked() {
                        self.emulator.set_run_type(RunType::Paused);
                    }
                    if ui.button("Debug Mode").clicked() {
                        self.show_debug_screen = true;
                    }
                });
                ui.menu_button("File", |ui| {
                    if ui.button("Select Rom...").clicked() {
                        let path = rfd::FileDialog::new()
                            .set_directory("~")
                            .pick_file()
                            .unwrap();
                        let cartridge = Cartridge::from(path.to_str().unwrap()).unwrap();
                        let flags = self.emulator.debug_ctx().get_flags();
                        self.emulator = Emulator::<DMGBus>::new()
                            .with_debug_flags(flags)
                            .with_rom(cartridge)
                            .unwrap();
                        self.emulator.set_run_type(self.run_type);
                    }
                    if ui.button("Dump Memory").clicked() {
                        self.emulator.debug_ctx_mut().dump_logs();
                    }
                });
            });
            ui.separator();
            ui.horizontal(|ui| {
                self.emu_screen.ui(ui);

                if self.show_debug_screen {
                    ui.separator();
                }
                ui.vertical(|ui| {
                    if self.show_debug_screen {
                        ui.horizontal(|ui| {
                            ui.label("Debugging screen");
                            if ui.button("Close").clicked() {
                                self.show_debug_screen = false;
                            }
                        });
                        if ui.button("Step").clicked() {
                            self.emulator.set_run_type(RunType::Instr);
                            self.emu_screen
                                .update_texture(&self.emulator.tick().unwrap().rgb(), ctx);
                        }
                        let debug = RefCell::new(self.emulator.debug_ctx_mut());
                        self.memory_editor.ui(
                            ui,
                            |addr| debug.borrow().raw_read(addr),
                            |addr, value| debug.borrow_mut().raw_write(addr, value),
                        );
                        let call_log = self.emulator.debug_ctx().build_call_log();
                        if let Some(log) = call_log {
                            ui.label(log);
                        }
                    }
                });
            });
        });

        ctx.request_repaint();
    }
}
