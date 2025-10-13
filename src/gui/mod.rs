mod components;
use eframe::Frame;
use egui::Context;

use crate::emulator::cartridge::Cartridge;
use crate::emulator::DMGBus;
use crate::emulator::{Emulator, RunType, SCREEN_HEIGHT, SCREEN_WIDTH};
use crate::gui::components::emu_screen::EmuScreen;

pub struct EmulatorGui {
    emulator: Emulator<DMGBus>,
    emu_screen: EmuScreen,
    run_type: RunType,
    show_debug_screen: bool,
}

impl EmulatorGui {
    pub fn new(emulator: Emulator<DMGBus>) -> Self {
        let run_type = emulator.run_type().clone();
        Self {
            emulator,
            emu_screen: EmuScreen::new(SCREEN_WIDTH, SCREEN_HEIGHT),
            run_type,
            show_debug_screen: false,
        }
    }
}

impl eframe::App for EmulatorGui {
    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        self.emu_screen
            .update_texture(&self.emulator.tick().unwrap().rgb(), ctx);

        let response = egui::CentralPanel::default().show(ctx, |ui| {
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
                        self.emulator = Emulator::<DMGBus>::new();
                        self.emulator.set_run_type(self.run_type);
                        self.emulator.load_rom(cartridge).unwrap();
                    }
                })
            });
            self.emu_screen.ui(ui);
        });

        ctx.request_repaint();
    }
}

// For deubug menu later
/*
let before = self.run_type;
egui::ComboBox::from_label("Select Run Mode")
    .selected_text(format!("{}", self.run_type))
    .show_ui(ui, |ui| {
        ui.selectable_value(&mut self.run_type, RunType::Paused, "Pause");
        ui.selectable_value(&mut self.run_type, RunType::Frame, "Run");
        ui.selectable_value(
            &mut self.run_type,
            RunType::Instr,
            "Instruction Step",
        );
    });
if self.run_type != before {
    self.emulator.set_run_type(self.run_type);
}
*/
