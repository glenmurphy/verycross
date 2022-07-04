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
struct TrayRunner {
    tray_tx: UnboundedSender<TrayMessage>,
    control_rx: UnboundedReceiver<TrayControl>,
}

impl TrayRunner {
    async fn run(&mut self) {
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

    pub fn new(tray_tx: UnboundedSender<TrayMessage>, control_rx: UnboundedReceiver<TrayControl>) -> Self {
        TrayRunner {
            tray_tx,
            control_rx,
        }
    }
}

pub fn start() -> TrayInterface {
    let (tray_tx, tray_rx) = unbounded_channel::<TrayMessage>();
    let (control_tx, control_rx) = unbounded_channel::<TrayControl>();

    tokio::spawn(async move {
        TrayRunner::new(tray_tx, control_rx).run().await;
    });

    TrayInterface {
        control_tx,
        tray_rx,
    }
}