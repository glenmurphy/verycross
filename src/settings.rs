use std::sync::RwLock;
use state::Storage;
use tokio::sync::broadcast;

pub static SETTINGS: Storage<RwLock<Settings>> = Storage::new();

#[derive(Clone, Debug)]
pub struct Settings {
    pub crosshair : usize,
    tx : broadcast::Sender<()>,
}

pub fn init() {
    let (tx, rx) = broadcast::channel::<()>(4);
    SETTINGS.set(RwLock::new(Settings {
        crosshair : 0,
        tx,
    }));
}

pub fn get() -> std::sync::RwLockReadGuard<'static, Settings, > {
    SETTINGS.get().read().unwrap()
}

fn get_mut() -> std::sync::RwLockWriteGuard<'static, Settings, > {
    SETTINGS.get().write().unwrap()
}

pub fn set_crosshair(n: usize) {
    get_mut().crosshair = n;
    notify();
}

pub fn notify() {
    SETTINGS.get().write().unwrap().tx.send(()).unwrap();
}

pub fn subscribe() -> broadcast::Receiver<()> {
    get().tx.subscribe()
}