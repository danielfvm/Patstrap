use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use async_trait::async_trait;
use btleplug::api::{Characteristic, ValueNotification};
use eframe::egui::{self, Color32, RichText};
use futures_util::Stream;
use log::{info, warn};
use tokio::{
    net::tcp::{OwnedReadHalf, OwnedWriteHalf},
    sync::{
        mpsc::{self, Receiver, Sender},
        watch,
    },
    task::JoinHandle,
    time::sleep,
};
use tokio_util::sync::CancellationToken;

use std::pin::Pin;

use crate::{config::DeviceConfig, haptic::Haptic, protocol::{CommandClient, CommandServer, Reader}, Config, LIGHT_GREEN, LIGHT_RED};

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
}

#[async_trait]
pub trait Connection: Send + Sync {
    type Ctx;

    async fn connect(&self) -> anyhow::Result<Self::Ctx>;
    async fn read(&self, ctx: &mut Self::Ctx) -> anyhow::Result<Vec<u8>>;
    async fn write(&self, ctx: &mut Self::Ctx, data: &[u8]) -> anyhow::Result<usize>;
    async fn disconnect(&self, ctx: Self::Ctx) -> anyhow::Result<()>;

    fn name(&self) -> String;
    fn addr(&self) -> String;
    fn id(&self) -> String;
}

#[derive(Debug)]
enum ConnectionState {
    Connected {
        token: CancellationToken,
        _handle: JoinHandle<()>,
        tx: Sender<CommandServer>,
        rx: Receiver<CommandClient>,
    },
    Disconnected,
}

pub enum ConnectionCtx {
    Ble {
        characteristic: Characteristic,
        notifications: Pin<Box<dyn Stream<Item = ValueNotification> + Send>>,
    },
    Wifi {
        reader: OwnedReadHalf,
        writer: OwnedWriteHalf,
    },
}

pub struct Device {
    connection: Arc<Box<dyn Connection<Ctx = ConnectionCtx>>>,
    state: Arc<Mutex<ConnectionState>>,
    battery: Option<f32>,
    connected: bool,
}

impl Device {
    pub fn new(connection: impl Connection<Ctx = ConnectionCtx> + 'static + Send) -> Self {
        Self {
            connection: Arc::new(Box::new(connection)),
            state: Arc::new(Mutex::new(ConnectionState::Disconnected)),
            battery: None,
            connected: false,
        }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        egui::Frame::default()
            .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
            .inner_margin(10.0)
            .corner_radius(5.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Name");
                    ui.label(&self.name());
                });
                ui.horizontal(|ui| {
                    ui.label("Address");
                    ui.label(
                        RichText::new(self.addr())
                            .italics()
                            .color(Color32::DARK_GRAY),
                    );
                });

                ui.horizontal(|ui| {
                    ui.label("Battery");
                    if let Some(battery) = self.battery {
                        ui.add(
                            egui::ProgressBar::new(battery)
                                .show_percentage()
                                .fill(LIGHT_RED.lerp_to_gamma(LIGHT_GREEN, battery))
                                .desired_width(100.0),
                        );
                    } else {
                        ui.add(
                            egui::ProgressBar::new(1.0)
                                .text("Unknown")
                                .fill(Color32::GRAY)
                                .desired_width(100.0),
                        );
                    }
                });

                let status = self.status();
                if status == ConnectionStatus::Connected {
                    ui.horizontal(|ui| {
                        ui.label("Auto connect");

                        let mut cfg = Config::instance();
                        ui.checkbox(&mut self.get_cfg(&mut cfg).auto_connect, "");
                    });
                }

                let mut cfg = Config::instance();
                let cfg = self.get_cfg(&mut cfg);

                if cfg.haptics.len() > 0 {
                    ui.collapsing("Configure Haptics", |ui| {
                        for haptic in cfg.haptics.values_mut() {
                            haptic.ui(ui, self);
                        }
                    });
                }

                ui.scope(|ui| {
                    if status == ConnectionStatus::Connecting {
                        ui.disable();
                    }

                    let btn_text = match status {
                        ConnectionStatus::Connected => "Disconnect",
                        ConnectionStatus::Connecting => "Connecting...",
                        ConnectionStatus::Disconnected => "Connect",
                    };

                    if status == ConnectionStatus::Connected {
                        ui.style_mut().visuals.widgets.inactive.fg_stroke.color = Color32::WHITE;
                        ui.style_mut().visuals.widgets.inactive.weak_bg_fill = LIGHT_RED;
                        ui.style_mut().visuals.widgets.hovered.weak_bg_fill = LIGHT_RED;
                        ui.style_mut().visuals.widgets.active.weak_bg_fill = LIGHT_RED;
                    }

                    if ui.button(btn_text).clicked() {
                        match status {
                            ConnectionStatus::Disconnected => self.connect(),
                            ConnectionStatus::Connected => self.disconnect(),
                            _ => (),
                        };
                    }
                });
            });
    }

    fn get_cfg<'a>(&self, cfg: &'a mut Config) -> &'a mut DeviceConfig {
        cfg.devices
            .entry(self.id())
            .or_insert(DeviceConfig::default())
    }

    pub fn update(&mut self) {
        for cmd in self.messages() {
            match cmd {
                CommandClient::Connected => {
                    self.connected = true;
                }
                CommandClient::Disconnect => {
                    *self.state.lock().unwrap() = ConnectionState::Disconnected;

                    self.battery = None;
                    self.connected = false;
                }
                CommandClient::Info { channel, name } => {
                    let mut cfg = Config::instance();
                    let cfg = self.get_cfg(&mut cfg);
                    cfg.haptics
                        .entry(channel)
                        .or_insert(Haptic::new(name, channel));
                }
                CommandClient::Battery { level } => {
                    self.battery = Some((level as f32) / 100.0);
                }
                CommandClient::Invalid => {}
            }
        }
    }

    fn messages(&self) -> Vec<CommandClient> {
        match &mut *self.state.lock().unwrap() {
            ConnectionState::Connected { rx, .. } => {
                let mut cmds = vec![];
                while let Ok(cmd) = rx.try_recv() {
                    cmds.push(cmd);
                }

                cmds
            }
            _ => vec![],
        }
    }

    pub fn haptic_osc(&mut self, osc: &str, strength: u8) {
        let mut cfg = Config::instance();
        let cfg = self.get_cfg(&mut cfg);

        for haptic in cfg.haptics.values() {
            if haptic.osc_parameter == osc {
                self.exec(CommandServer::Haptic {
                    channel: haptic.channel,
                    strength: (strength as f32 * haptic.strength) as u8,
                    duration: 50,
                });
            }
        }
    }

    pub fn status(&self) -> ConnectionStatus {
        match &*self.state.lock().unwrap() {
            ConnectionState::Connected { .. } if self.connected => ConnectionStatus::Connected,
            ConnectionState::Connected { .. } if !self.connected => ConnectionStatus::Connecting,
            _ => ConnectionStatus::Disconnected,
        }
    }

    pub fn id(&self) -> String {
        self.connection.id()
    }

    pub fn name(&self) -> String {
        self.connection.name()
    }

    pub fn addr(&self) -> String {
        self.connection.addr()
    }

    pub fn exec(&self, cmd: CommandServer) {
        if let Ok(state) = self.state.lock() {
            match &*state {
                ConnectionState::Connected { tx, .. } => {
                    if let Err(err) = tx.try_send(cmd) {
                        warn!("Error executing: {err}")
                    }
                }
                _ => {}
            }
        }
    }

    fn disconnect(&mut self) {
        match &*self.state.lock().unwrap() {
            ConnectionState::Connected { token, .. } => token.cancel(),
            _ => (),
        }
    }

    fn connect(&mut self) {
        if self.status() == ConnectionStatus::Connected {
            return;
        }

        let (tx_handle, rx) = mpsc::channel::<CommandClient>(32);
        let (tx, mut rx_handle) = mpsc::channel::<CommandServer>(32);

        let token = CancellationToken::new();
        let connection = self.connection.clone();
        let token_handle = token.clone();
        let state = self.state.clone();

        let id = self.id();
        let handle = tokio::task::spawn(async move {
            let mut ctx = match connection.connect().await {
                Ok(ok) => ok,
                Err(err) => {
                    info!("{id}: Connection failed: {err}");
                    *state.lock().unwrap() = ConnectionState::Disconnected;
                    return;
                }
            };

            info!("{id}: Connected");
            let _ = tx_handle.send(CommandClient::Connected).await;

            let id2 = id.clone();
            let (tx, mut rx) = watch::channel(());
            let token_handle_timeout = token_handle.clone();
            tokio::spawn(async move {
                loop {
                    tokio::select! {
                        _ = sleep(Duration::from_secs(5)) => {
                            token_handle_timeout.cancel();
                            warn!("{id2}: Device timeout");
                            break;
                        }
                        _ = rx.changed() => {
                            continue;
                        }
                    }
                }
            });

            loop {
                tokio::select! {
                    data = connection.read(&mut ctx) => {
                        match data {
                            Ok(data) => {
                                let _ = tx.send(());
                                info!("{id}: Recv: {:?}", &data);
                                let mut reader = Reader::new(&data);
                                while let Some(command) = CommandClient::from_bytes(&mut reader) {
                                    if let Err(err) = tx_handle.send(command).await {
                                        warn!("{id}: {}", err);
                                        break;
                                    }
                                }

                            }
                            Err(err) => {
                                warn!("{id}: {}", err);
                                break;
                            }
                        }
                    }
                    Some(data) = rx_handle.recv() => {
                        info!("Sending: {:?}", data);
                        if let Err(err) = connection.write(&mut ctx, &data.as_bytes()).await {
                            warn!("{id}: {}", err);
                            break;
                        }
                    }
                    _ = token_handle.cancelled() => {
                        break;
                    }
                }
            }

            info!("{id}: Disconnecting...");
            let _ = tx_handle.send(CommandClient::Disconnect).await;
            if let Err(err) = connection.disconnect(ctx).await {
                warn!("{id}: {}", err);
            }
            info!("{id}: Disconnected");
        });

        *self.state.lock().unwrap() = ConnectionState::Connected {
            token,
            _handle: handle,
            tx,
            rx,
        };
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        self.disconnect();
    }
}
