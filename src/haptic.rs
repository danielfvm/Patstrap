use eframe::egui::{self};
use serde::{Deserialize, Serialize};

use crate::{device::Device, protocol::CommandServer};

#[derive(Debug, Serialize, Deserialize)]
pub struct Haptic {
    pub name: String,
    pub vrc_parameter: String,
    pub strength: f32,
    pub channel: u8,
}

impl Haptic {
    pub fn new(name: String, channel: u8) -> Self {
        Self {
            vrc_parameter: format!("/avatar/parameters/pat_{}", name.to_lowercase()),
            name,
            channel,
            strength: 1.0,
        }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, device: &Device) {
        egui::Frame::default()
            .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
            .inner_margin(10.0)
            .corner_radius(5.0)
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.label("Channel");
                    ui.label(self.channel.to_string());
                });

                ui.horizontal(|ui| {
                    ui.label("Name");
                    ui.text_edit_singleline(&mut self.name);
                });

                ui.horizontal(|ui| {
                    ui.label("VRC Parameter");
                    ui.text_edit_singleline(&mut self.vrc_parameter);
                });

                ui.horizontal(|ui| {
                    ui.label("Strength");
                    ui.add(egui::Slider::new(&mut self.strength, 0.0..=1.0));
                });

                if ui.button("Test Haptic").clicked() {
                    device.exec(CommandServer::Haptic {
                        channel: self.channel,
                        strength: (self.strength * 255.0) as u8,
                        duration: 1000,
                    });
                }
            });
    }
}
