use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tray_item::{IconSource, TrayItem};

#[allow(unused)]
#[derive(Copy, Clone)]
pub enum TrayMessage {
    Show,
    Hide,
    Config,
    Quit,
}

#[derive(Debug)]
#[allow(unused)]
enum TrayControl {
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
    pub fn on(&self) {
        self.control_tx.send(TrayControl::On).unwrap()
    }
    pub fn off(&self) {
        self.control_tx.send(TrayControl::Off).unwrap();
    }
    pub fn quit(&self) {
        self.control_tx.send(TrayControl::Quit).unwrap();
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

fn emitter(tx: &UnboundedSender<TrayMessage>, msg: TrayMessage) -> impl Fn() {
    let tx_clone = tx.clone();
    move || { let _ = tx_clone.send(msg); }
}

impl TrayRunner {
    async fn run(&mut self) {
        let mut tray = TrayItem::new("Verycross", IconSource::Resource("tray-on")).unwrap();
        tray.add_label("Verycross").unwrap();

        tray.add_menu_item("Show", emitter(&self.tray_tx, TrayMessage::Show)).unwrap();
        tray.add_menu_item("Hide", emitter(&self.tray_tx, TrayMessage::Hide)).unwrap();
        tray.inner_mut().add_separator().unwrap(); // windows only
        tray.add_menu_item("Config", emitter(&self.tray_tx, TrayMessage::Config)).unwrap();
        tray.add_menu_item("Quit", emitter(&self.tray_tx, TrayMessage::Quit)).unwrap();

        while let Some(v) = self.control_rx.recv().await {
            match v {
                TrayControl::On => tray.set_icon(IconSource::Resource("tray-on")).unwrap(),
                TrayControl::Off => tray.set_icon(IconSource::Resource("tray-off")).unwrap(),
                TrayControl::Quit => {
                    let _ = tray.inner_mut().quit();
                    let _ = tray.inner_mut().shutdown();
                    break;
                }
            }
        }
    }

    pub fn new(
        tray_tx: UnboundedSender<TrayMessage>,
        control_rx: UnboundedReceiver<TrayControl>,
    ) -> Self {
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
