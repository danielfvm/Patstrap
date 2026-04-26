use std::{
    collections::HashMap,
    fmt::Display,
    fs::File,
    path::PathBuf,
    sync::{LazyLock, Mutex, MutexGuard},
};

use serde::{Deserialize, Serialize};
use strum_macros::EnumIter;

use crate::haptic::Haptic;

#[derive(PartialEq, Clone, Copy, EnumIter, Serialize, Deserialize)]
pub enum Header {
    Welcome,
    Devices,
    Settings,
}

impl Display for Header {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Header::Welcome => f.write_str("Welcome"),
            Header::Devices => f.write_str("Device List"),
            Header::Settings => f.write_str("Settings"),
        }
    }
}

#[derive(PartialEq, Clone, Copy, EnumIter, Serialize, Deserialize)]
pub enum Mode {
    OSC,
    Unity,
}

impl Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Mode::OSC => f.write_str("VRChat (OSC)"),
            Mode::Unity => f.write_str("Unity"),
        }
    }
}

#[derive(Serialize, Deserialize, Default)]
pub struct DeviceConfig {
    pub auto_connect: bool,
    pub haptics: HashMap<u8, Haptic>,
}

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub devices: HashMap<String,  DeviceConfig>,
    pub mode: Mode,
    pub auto_start: bool,
    pub header: Header,
    pub unity_osc_port: u16,
}

static CFG: LazyLock<Mutex<Config>> = LazyLock::new(|| Mutex::new(Config::default()));

impl Config {
    /// THIS IS DANGEROUS, CAN CAUSE DEADLOCKS!
    pub fn instance<'a>() -> MutexGuard<'a, Config> {
        CFG.lock().unwrap()
    }

    pub fn load(path: PathBuf) -> std::io::Result<()> {
        let file = File::open(path)?;
        let text = std::io::read_to_string(file)?;
        let config = serde_json::from_str::<Config>(text.as_str())?;

        *Config::instance() = config;

        Ok(())
    }

    pub fn save(path: PathBuf) -> std::io::Result<()> {
        std::fs::write(
            path,
            serde_json::to_string_pretty(&*Config::instance()).unwrap(),
        )
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            devices: HashMap::new(),
            mode: Mode::OSC,
            header: Header::Welcome,
            auto_start: false,
            unity_osc_port: 5123,
        }
    }
}
