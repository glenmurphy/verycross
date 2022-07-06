use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::sync::RwLock;
use tokio::sync::broadcast;

lazy_static! {
    static ref SETTINGS: RwLock<Settings> = RwLock::new(load());
    static ref SETTINGS_CHANNEL: broadcast::Sender<()> = broadcast::channel::<()>(4).0;
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct Settings {
    pub crosshair: usize,
}

impl Settings {
    pub fn default() -> Settings {
        Settings { 
            crosshair: 0
        }
    }

    pub fn coerce(&mut self) {
        if self.crosshair > 2 {
            println!("Settings: coercing crosshair to 0");
            self.crosshair = 0;
        }
    }
}

fn get_config_dir() -> std::path::PathBuf {
    let project_dir = ProjectDirs::from("", "", "verycross").unwrap();
    project_dir.config_dir().to_path_buf()
}

fn get_config_filepath() -> std::path::PathBuf {
    let filename = if cfg!(debug_assertions) { "settings.debug.json" } else { "settings.json" };
    get_config_dir().join(filename)
}

fn load() -> Settings {
    if let Ok(data) = std::fs::read_to_string(get_config_filepath()) {
        if let Ok(mut settings) = serde_json::from_str::<Settings>(&data) {
            println!("Loaded settings from disk");
            settings.coerce();
            return settings;
        }
    }

    println!("No saved settings; using defaults");
    Settings::default()
}

fn save() {
    if std::fs::metadata(get_config_filepath()).is_err() {
        if std::fs::create_dir_all(get_config_dir()).is_ok() {
            println!("Created save directory");
        } else {
            println!("Error creating save directory");
            return;
        }
    }

    let mut settings = *get_mut();
    settings.coerce();
    let serialized = serde_json::to_string(&settings).unwrap();
    if std::fs::write(get_config_filepath(), serialized).is_ok() {
        println!("Saved settings to disk");
    } else {
        println!("Error saving settings to disk");
    }
}

fn get_mut() -> std::sync::RwLockWriteGuard<'static, Settings> {
    SETTINGS.write().unwrap()
}

fn updated() {
    save();
    SETTINGS_CHANNEL.send(()).unwrap();
}

pub fn subscribe() -> broadcast::Receiver<()> {
    SETTINGS_CHANNEL.subscribe()
}

pub fn get() -> std::sync::RwLockReadGuard<'static, Settings> {
    SETTINGS.read().unwrap()
}

pub fn set_crosshair(n: usize) {
    get_mut().crosshair = n;
    updated();
}
