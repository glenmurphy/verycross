use std::sync::RwLock;
use state::Storage;
use tokio::sync::broadcast;

static SETTINGS: Storage<RwLock<Settings>> = Storage::new();
static SETTINGS_CHANNEL: Storage<broadcast::Sender<()>> = Storage::new();

#[derive(Copy, Clone, Debug)]
pub struct Settings {
    pub crosshair : usize,
}

pub fn init() {
    let (tx, _rx) = broadcast::channel::<()>(4);
    // TODO: load from disk
    SETTINGS.set(RwLock::new(
        Settings {
            crosshair : 1,
        }
    ));
    SETTINGS_CHANNEL.set(tx);
}

fn get_mut() -> std::sync::RwLockWriteGuard<'static, Settings, > {
    SETTINGS.get().write().unwrap()
}

fn notify() {
    SETTINGS_CHANNEL.get().send(()).unwrap();
}

pub fn subscribe() -> broadcast::Receiver<()> {
    SETTINGS_CHANNEL.get().subscribe()
}

pub fn get() -> std::sync::RwLockReadGuard<'static, Settings, > {
    SETTINGS.get().read().unwrap()
}

pub fn set_crosshair(n: usize) {
    get_mut().crosshair = n;
    // TODO: save to disk
    notify();
}