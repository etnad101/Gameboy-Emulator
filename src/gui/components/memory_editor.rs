use std::mem;

pub struct MemoryEditor {
    bytes_per_row: usize,
    frame_size: usize,
    current_frame: usize,
    max_frames: usize,
}

impl MemoryEditor {
    pub fn new(bytes_per_row: usize, mem_size: usize, frame_size: usize) -> Self {
        Self {
            bytes_per_row,
            frame_size,
            current_frame: 0,
            max_frames: mem_size.div_ceil(frame_size),
        }
    }

    fn increment_frame(&mut self) {
        self.current_frame += 1;
        if self.current_frame >= self.max_frames {
            self.current_frame = 0
        }
    }

    fn decrement_frame(&mut self) {
        self.current_frame = self.current_frame.wrapping_sub(1);
        if self.current_frame > self.max_frames {
            self.current_frame = self.max_frames - 1;
        }
    }

    pub fn ui(
        &mut self,
        ui: &mut egui::Ui,
        read: impl Fn(u16) -> u8,
        mut write: impl FnMut(u16, u8),
    ) {
        let frame_start = self.current_frame * self.frame_size;
        let frame_end = frame_start + self.frame_size - 1;
        ui.horizontal(|ui| {
            ui.label(format!("Frame: {}", self.current_frame));
            ui.label(format!(
                "Start: {:#06X}, End: {:#06X}",
                frame_start, frame_end
            ));

            if ui.button("Prev").clicked() {
                self.decrement_frame();
            }
            if ui.button("Next").clicked() {
                self.increment_frame();
            }
        });
        egui::ScrollArea::vertical().show(ui, |ui| {
            for row in 0..(self.frame_size / self.bytes_per_row) {
                ui.horizontal(|ui| {
                    let offset = row * self.bytes_per_row;
                    let addr = frame_start + offset;
                    ui.label(format!("|{:#06X}|", addr));
                    for col in 0..self.bytes_per_row {
                        let addr = addr + col;
                        let byte = read(addr as u16);
                        let mut byte_str = format!("{:02X}", byte);
                        let edit_box = egui::TextEdit::singleline(&mut byte_str)
                            .desired_width(15.0)
                            .margin(0.0)
                            .char_limit(2)
                            .frame(false);
                        ui.add(edit_box);
                    }
                });
            }
        });
    }
}
