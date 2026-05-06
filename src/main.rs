//#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

mod ble;
mod config;
mod device;
mod haptic;
mod service;
mod protocol;
mod wifi;

// https://www.egui.rs/#demo
use std::fmt::Display;
use clap::Parser;
use eframe::egui::{self, Color32, Id, Link, RichText};
use log::warn;
use std::error::Error;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::{
    env,
    path::PathBuf,
};
use strum::IntoEnumIterator;
use tokio::select;
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;

use crate::ble::BleConnection;
use crate::config::{Config, Header, Mode};
use crate::device::{ConnectionStatus, Device};
use crate::service::{OSCService, OSCUnity};
use crate::wifi::WifiConnection;

pub const LIGHT_RED: Color32 = Color32::LIGHT_RED;
pub const LIGHT_GREEN: Color32 = Color32::from_rgb(90, 180, 130);

#[derive(Parser)]
struct Cli {
    /// The config file that is used to save settings
    #[arg(short('c'), long("config"), default_value = "config.json")]
    config_path: PathBuf,
}

#[tokio::main]
async fn main() -> Result<(), eframe::Error> {
    env_logger::init();

    let args = Cli::parse();

    if let Err(err) = Config::load(args.config_path.clone()) {
        warn!("Loading config failed: {err}");
    }

    eframe::run_native(
        &format!("Patstrap {}", env!("CARGO_PKG_VERSION")),
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([560.0, 700.0])
                .with_maximize_button(false)
                .with_resizable(false),
            ..eframe::NativeOptions::default()
        },
        Box::new(|_| Ok(Box::new(Patstrap::new(args.config_path.clone())))),
    )?;

    if let Err(err) = Config::save(args.config_path) {
        warn!("Saving config failed: {err}");
    }

    Ok(())
}

#[derive(Debug)]
enum PatstrapError {
    NoBluetoothAdapter,
}

impl Display for PatstrapError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoBluetoothAdapter => f.write_str("No Bluetooth Adapter found"),
        }
    }
}

impl Error for PatstrapError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self)
    }
}

struct Patstrap {
    path: PathBuf,
    devices: Arc<Mutex<Vec<Device>>>,
    app_status: Arc<Mutex<bool>>,
    token: CancellationToken,
    temp_port_number: String,
}

impl Patstrap {
    fn new(path: PathBuf) -> Self {
        let devices = Arc::new(Mutex::new(Vec::new()));

        let devices_copy = devices.clone();
        tokio::spawn(async move {
            if let Err(err) = BleConnection::discover(devices_copy).await {
                warn!("{:?}", err);
            }
        });

        let devices_copy = devices.clone();
        tokio::spawn(async move {
            if let Err(err) = WifiConnection::discover(devices_copy).await {
                warn!("{:?}", err);
            }
        });

        let app_status = Arc::new(Mutex::new(false));
        let app_status_c = app_status.clone();
        let devices_c = devices.clone();
        let token = CancellationToken::new();
        let token_c = token.clone();
        tokio::spawn(async move {
            let _ = OSCUnity::new(devices_c.clone(), app_status_c.clone()).await;

            if let Err(err) = OSCService::new(devices_c, token_c, app_status_c).await {
                warn!("{:?}", err);
            }
        });


        let devices_copy = devices.clone();
        let token_cpy = token.clone();
        tokio::spawn(async move {
            loop {
                select! {
                    _ = token_cpy.cancelled() => break,
                    _ = sleep(Duration::from_millis(100)) => {
                        for device in devices_copy.lock().unwrap().iter_mut() {
                            device.update();
                        }
                    }
                }
            }
        });

        Self {
            token,
            path,
            app_status,
            devices,
            temp_port_number: String::new(),
        }
    }
}

impl Drop for Patstrap {
    fn drop(&mut self) {
        self.token.cancel();
    }
}

impl eframe::App for Patstrap {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(format!("{} connection", Config::instance().mode));
                ui.label(
                    RichText::new("⏺")
                        .color(self.app_status.lock().unwrap().then_some(LIGHT_GREEN).unwrap_or(LIGHT_RED)),
                );

                ui.add_space(20.0);

                let strap_status = self.devices.lock()
                    .unwrap()
                    .iter()
                    .any(|device| device.status() == ConnectionStatus::Connected);

                ui.label("Device connection");
                ui.label(
                    RichText::new("⏺").color(
                        strap_status
                            .then_some(LIGHT_GREEN)
                            .unwrap_or(LIGHT_RED),
                    ),
                );

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button("💾").clicked() {
                        if let Err(err) = Config::save(self.path.clone()) {
                            warn!("Saving config failed: {err}");
                        }
                    }
                });
            });

            ui.separator();

            ui.horizontal(|ui| {
                for header in Header::iter() {
                    ui.selectable_value(&mut Config::instance().header, header, header.to_string());
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    match Config::instance().header {
                        Header::Devices => {
                            if ui.small_button("↻").clicked() {
                            }
                        }
                        _ => {}
                    }
                });
            });

            ui.add_space(10.0);

            let header = {
                Config::instance().header.clone()
            };

            match header {
                Header::Welcome => {
                    egui::Frame::default()
                        .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
                        .inner_margin(10.0)
                        .corner_radius(5.0)
                        .show(ui, |ui| {
                            ui.horizontal_wrapped(|ui| {
                                ui.set_width(ui.available_width());
                                ui.heading("# Quickstart");
                                ui.end_row();
                                ui.label("1. Start by going to");
                                if ui.add(Link::new(Header::Settings.to_string())).clicked() {
                                    Config::instance().header = Header::Settings;
                                }
                                ui.label("and select the correct Mode. The App Status indicator should turn green as soon as the connection to the App/Game was established.");
                                ui.end_row();
                                ui.label("2. Then go to");
                                if ui.add(Link::new(Header::Devices.to_string())).clicked() {
                                    Config::instance().header = Header::Settings;
                                }
                                ui.label("and connect to your Hardware.");
                                ui.label("After a successful connection the indicator lamp should turn green.");
                                ui.end_row();
                                ui.label("3. Next configure your Haptics in");
                                /*if ui.add(Link::new(Header::Haptics.to_string())).clicked() {
                                    self.config.header = Header::Haptics;
                                }*/
                                ui.label("The defaults should be fine if you use the setup described in the");
                                ui.hyperlink_to("README.md", "https://github.com/danielfvm/Patstrap/blob/master/README.md");
                                ui.end_row();
                                ui.end_row();
                            });
                        });

                    egui::Frame::default()
                        .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
                        .inner_margin(10.0)
                        .corner_radius(5.0)
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.horizontal_wrapped(|ui| {
                                ui.heading("# Troubleshooting");
                                ui.end_row();
                                ui.label("- If you are using a Bluetooth device, make sure you are paired with the device in your Operating System settings!");
                                ui.end_row();
                                ui.label("- If you are using a WiFi device, make sure your PC and the device are in the same local Network!");
                                ui.end_row();
                                ui.label("- If you have a custom setup (e.g. multiple haptics) you might need to add additional haptics or change the settings.");
                                ui.end_row();
                                ui.end_row();
                            });
                        });

                    egui::Frame::default()
                        .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
                        .inner_margin(10.0)
                        .corner_radius(5.0)
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.horizontal_wrapped(|ui| {
                                ui.heading("# Links");
                                ui.end_row();
                                ui.label("If you have issues, need some help or want to contribute to the project, here some links to get you started:");
                                ui.end_row();
                                ui.hyperlink_to("- GitHub", "https://github.com/danielfvm/Patstrap/");
                                ui.end_row();
                                ui.hyperlink_to("- Discord", "https://discord.gg/QsuHQXECw2");
                            });
                        });
                }
                Header::Devices => {
                    ui.vertical_centered_justified(|ui| {
                        egui::ScrollArea::vertical()
                            .auto_shrink(false)
                            .show(ui, |ui| {
                                for device in self.devices.lock().unwrap().iter_mut() {
                                    device.ui(ui);
                                }

                                ui.add(egui::Spinner::new());
                            });
                    });
                }
                Header::Settings => {
                    ui.horizontal(|ui| {
                        ui.label("Mode");
    
                        let mode = {
                            Config::instance().mode.to_string()
                        };

                        egui::ComboBox::from_id_salt(Id::new("Mode"))
                            .selected_text(mode)
                            .show_ui(ui, |ui| {
                                for mode in Mode::iter() {
                                    ui.selectable_value(
                                        &mut Config::instance().mode,
                                        mode,
                                        mode.to_string(),
                                    );
                                }
                            });
                    });

                    ui.horizontal(|ui| {
                        ui.label("Auto start");
                        ui.checkbox(&mut Config::instance().auto_start, "");
                    });

                    ui.horizontal(|ui| {
                        ui.label("Unity OSC Port");

                        let response = ui.text_edit_singleline(&mut self.temp_port_number); 

                        let submit = response.lost_focus()
                            && (ui.input(|i| i.key_pressed(egui::Key::Enter)) || true);

                        if submit {
                            if let Ok(port) = self.temp_port_number.parse::<u16>() {
                                Config::instance().unity_osc_port = port;
                            }
                        } else if !response.has_focus() {
                            self.temp_port_number = Config::instance().unity_osc_port.to_string();
                        }
                    });
                }
            }
        });
    }
}
