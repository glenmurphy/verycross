use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use state::Storage;
use std::sync::RwLock;
use tokio::sync::broadcast;

static SETTINGS: Storage<RwLock<Settings>> = Storage::new();
static SETTINGS_CHANNEL: Storage<broadcast::Sender<()>> = Storage::new();

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct Settings {
    pub crosshair: usize,
}

fn get_config_dir() -> std::path::PathBuf {
    let project_dir = ProjectDirs::from("", "", "verycross").unwrap();
    project_dir.config_dir().to_path_buf()
}

fn get_config_filepath() -> std::path::PathBuf {
    get_config_dir().join("settings.json")
}

fn load() -> Settings {
    if let Ok(data) = std::fs::read_to_string(get_config_filepath()) {
        if let Ok(settings) = serde_json::from_str::<Settings>(&data) {
            println!("Loaded settings from disk");
            return settings;
        }
    }

    println!("No saved settings; using defaults");
    Settings { crosshair: 0 }
}

fn save() {
    if std::fs::metadata(get_config_filepath()).is_err() {
        if std::fs::create_dir_all(get_config_dir()).is_ok() {
            println!("Created save directory");
        } else {
            println!("Error creating save directory");
        }
    }

    let settings = *get();
    let serialized = serde_json::to_string(&settings).unwrap();
    if std::fs::write(get_config_filepath(), serialized).is_ok() {
        println!("Saved settings to disk");
    } else {
        println!("Error saving settings to disk");
    }
}

pub fn init() {
    let (tx, _rx) = broadcast::channel::<()>(4);
    SETTINGS_CHANNEL.set(tx);
    SETTINGS.set(RwLock::new(load()));
}

fn get_mut() -> std::sync::RwLockWriteGuard<'static, Settings> {
    SETTINGS.get().write().unwrap()
}

fn updated() {
    save();
    SETTINGS_CHANNEL.get().send(()).unwrap();
}

pub fn subscribe() -> broadcast::Receiver<()> {
    SETTINGS_CHANNEL.get().subscribe()
}

pub fn get() -> std::sync::RwLockReadGuard<'static, Settings> {
    SETTINGS.get().read().unwrap()
}

pub fn set_crosshair(n: usize) {
    get_mut().crosshair = n;
    // TODO: save to disk

    updated();
}
