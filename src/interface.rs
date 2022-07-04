use tokio::sync::mpsc::UnboundedReceiver;
use winit::event_loop::EventLoopProxy;
use winky::Key;
use crate::tray;

#[derive(Debug, Clone, Copy)]
pub enum InterfaceMessage {
    Show,
    Hide,
    Config,
    Jiggle,
    Quit,
}

struct Interface {
    showing : bool,
    tray : tray::TrayInterface,
    key_rx : UnboundedReceiver<(Key, bool)>,
    event_proxy : EventLoopProxy<InterfaceMessage>
}

impl Interface {
    fn new(event_proxy: EventLoopProxy<InterfaceMessage>) -> Interface {
        Interface {
            showing : true,
            tray : tray::start(),
            key_rx : winky::listen(),
            event_proxy
        }
    }

    fn show(&mut self) {
        self.event_proxy.send_event(InterfaceMessage::Show).unwrap();
        self.tray.on();
        self.showing = true;
    }

    fn hide(&mut self) {
        self.event_proxy.send_event(InterfaceMessage::Hide).unwrap();
        self.tray.off();
        self.showing = false;
    }

    fn config(&mut self) {
        self.event_proxy.send_event(InterfaceMessage::Config).unwrap();
    }

    fn toggle(&mut self) {
        if self.showing { self.hide() } else { self.show() }
    }

    fn quit(&mut self) {
        self.event_proxy.send_event(InterfaceMessage::Quit).unwrap();
        self.tray.quit();
        self.showing = false;
    }

    async fn listen(&mut self) {
        loop {
            tokio::select! {
                Some(key_event) = self.key_rx.recv() => {
                    match key_event {
                        (Key::ScrollLock, true) => self.toggle(),
                        _ => {}
                    }
                },
                Some(msg) = self.tray.recv() => {
                    match msg {
                        tray::TrayMessage::Show => self.show(),
                        tray::TrayMessage::Hide => self.hide(),
                        tray::TrayMessage::Config => self.config(),
                        tray::TrayMessage::Quit => self.quit(),
                    }
                }
            }
        }
    }
}

pub fn start(event_proxy: EventLoopProxy<InterfaceMessage>) {
    tokio::spawn(async move {
        Interface::new(event_proxy).listen().await;
    });
}