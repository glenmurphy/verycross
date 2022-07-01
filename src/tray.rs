use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tray_item::TrayItem;

#[allow(unused)]
pub enum TrayMessage {
    Show,
    Hide,
    Quit,
}

#[allow(unused)]
pub enum TrayControl {
    On,
    Off,
    Quit,
}

// MacOS tray blocks the main thread so only Windows allows channels to work,
// so we have this interface that will only sending on those channels if we're
// in Windows to prevent the unbounded_channel filling up. This is super daft hax.
#[allow(unused)]
pub struct TrayInterface {
    control_tx: UnboundedSender<TrayControl>,
    tray_rx: UnboundedReceiver<TrayMessage>,
}

impl TrayInterface {
    pub fn send(&self, msg: TrayControl) {
        #[cfg(target_os = "windows")]
        let _ = self.control_tx.send(msg);
    }
    pub fn on(&self) {
        self.send(TrayControl::On);
    }
    pub fn off(&self) {
        self.send(TrayControl::Off);
    }
    pub fn quit(&self) {
        self.send(TrayControl::Quit);
    }

    pub async fn recv(&mut self) -> Option<TrayMessage> {
        tokio::macros::support::poll_fn(|cx| self.tray_rx.poll_recv(cx)).await
    }
}

#[allow(unused)]
pub struct Tray {
    tray_tx: UnboundedSender<TrayMessage>,
    control_rx: UnboundedReceiver<TrayControl>,
}

impl Tray {
    #[cfg(target_os = "windows")]
    async fn run_win(&mut self) {
        let mut tray = TrayItem::new("Verycross", "tray-on").unwrap();
        tray.add_label("Verycross").unwrap();

        let show_tx = self.tray_tx.clone();
        tray.add_menu_item("Show", move || {
            let _ = show_tx.send(TrayMessage::Show);
        })
        .unwrap();

        let hide_tx = self.tray_tx.clone();
        tray.add_menu_item("Hide", move || {
            let _ = hide_tx.send(TrayMessage::Hide);
        })
        .unwrap();

        let quit_tx = self.tray_tx.clone();
        tray.add_menu_item("Quit", move || {
            let _ = quit_tx.send(TrayMessage::Quit);
        })
        .unwrap();

        while let Some(v) = self.control_rx.recv().await {
            match v {
                TrayControl::On => tray.set_icon("tray-on").unwrap(),
                TrayControl::Off => tray.set_icon("tray-off").unwrap(),
                TrayControl::Quit => {
                    let _ = tray.inner_mut().quit();
                    let _ = tray.inner_mut().shutdown();
                    break;
                }
            }
        }
    }

    #[allow(unused)]
    #[cfg(target_os = "macos")]
    fn run_mac(&self) {
        println!("starting mac tray");
        let mut tray = TrayItem::new("Panel", "").unwrap();
        tray.add_label("Panel").unwrap();

        let url = self.url.clone();
        tray.add_menu_item("Show", move || {
            let _ = webbrowser::open(&url);
        })
        .unwrap();

        let inner = tray.inner_mut();
        inner.add_quit_item("Quit");
        inner.display();
    }

    #[allow(unused)]
    pub fn new() -> (Self, TrayInterface) {
        let (tray_tx, mut tray_rx) = unbounded_channel::<TrayMessage>();
        let (control_tx, control_rx) = unbounded_channel::<TrayControl>();
        (
            Tray {
                tray_tx,
                control_rx,
            },
            TrayInterface {
                control_tx,
                tray_rx,
            },
        )
    }

    pub async fn run(&mut self) {
        // TODO: make this cross platform
        // TrayItem seems to have a different API on different platforms
        #[cfg(target_os = "windows")]
        self.run_win().await;

        // Experimental
        #[cfg(target_os = "macos")]
        self.run_mac();
    }
}

pub fn start() -> TrayInterface {
    let (mut tray, tray_interface) = Tray::new();
    tokio::spawn(async move {
        tray.run().await;
    });
    tray_interface
}
