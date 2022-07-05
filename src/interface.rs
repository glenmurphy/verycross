use tokio::sync::mpsc::UnboundedReceiver;
use winit::event_loop::EventLoopProxy;
use winky::Key;
use crate::tray;
use crate::config;

#[derive(Debug, Clone, Copy)]
pub enum InterfaceMessage {
    ShowCross,
    HideCross,
    Jiggle,
    Quit,
}

struct Interface {
    showing : bool,
    tray : tray::TrayInterface,
    config : config::ConfigInterface,
    config_open : bool,
    key_rx : UnboundedReceiver<(Key, bool)>,
    event_proxy : EventLoopProxy<InterfaceMessage>
}

impl Interface {
    fn new(event_proxy: EventLoopProxy<InterfaceMessage>) -> Interface {
        Interface {
            showing : true,
            tray : tray::start(),
            config : config::new(),
            config_open : false,
            key_rx : winky::listen(),
            event_proxy
        }
    }

    fn show(&mut self) {
        self.event_proxy.send_event(InterfaceMessage::ShowCross).unwrap();
        self.tray.on();
        self.showing = true;
    }

    fn hide(&mut self) {
        self.event_proxy.send_event(InterfaceMessage::HideCross).unwrap();
        self.tray.off();
        self.showing = false;
    }

    fn open_config(&mut self) {
        if self.config_open {
            self.config.close();
            self.config_open = false;
        } else {
            self.config.open();
            self.config_open = true;
        }
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
                        tray::TrayMessage::Config => self.open_config(),
                        tray::TrayMessage::Quit => self.quit(),
                    }
                },
                Some(msg) = self.config.recv() => {
                    match msg {
                        config::ConfigMessage::ShowCross => self.show(),
                        config::ConfigMessage::HideCross => self.hide(),
                        config::ConfigMessage::ConfigClosed => self.config_open = false,
                    }
                },
            }
        }
    }
}

pub fn start(event_proxy: EventLoopProxy<InterfaceMessage>) {
    tokio::spawn(async move {
        Interface::new(event_proxy).listen().await;
    });
}