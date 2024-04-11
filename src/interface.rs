use crate::config;
use crate::settings;
use crate::tray;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::mpsc::UnboundedSender;
use winit::event_loop::EventLoopProxy;
use winky::{Key, Event};

#[derive(Debug, Clone, Copy)]
pub enum InterfaceMessage {
    ShowCross,
    HideCross,
    SetCross(usize),
    Jiggle,
    Quit,
}

struct InterfaceRunner {
    showing: bool,
    tray: tray::TrayInterface,
    config: config::ConfigInterface,
    config_open: bool,
    key_rx: UnboundedReceiver<Event>,
    main_rx: UnboundedReceiver<InterfaceControl>,
    event_proxy: EventLoopProxy<InterfaceMessage>,
}

impl InterfaceRunner {
    fn new(
        event_proxy: EventLoopProxy<InterfaceMessage>,
        main_rx: UnboundedReceiver<InterfaceControl>,
    ) -> InterfaceRunner {
        InterfaceRunner {
            showing: true,
            tray: tray::start(),
            config: config::new(),
            config_open: false,
            key_rx: winky::listen(),
            main_rx,
            event_proxy,
        }
    }

    fn show_cross(&mut self) {
        self.event_proxy
            .send_event(InterfaceMessage::ShowCross)
            .unwrap();
        self.tray.on();
        self.showing = true;
    }

    fn hide_cross(&mut self) {
        self.event_proxy
            .send_event(InterfaceMessage::HideCross)
            .unwrap();
        self.tray.off();
        self.showing = false;
    }

    fn set_cross(&mut self, n: usize) {
        self.event_proxy
            .send_event(InterfaceMessage::SetCross(n))
            .unwrap();
    }

    fn toggle_cross(&mut self) {
        if self.showing {
            self.hide_cross()
        } else {
            self.show_cross()
        }
    }

    fn toggle_config(&mut self) {
        // Toggling doesn't really make UI sense, but we have it here so we can test
        // our ability to communicate with the interior of the fltk window.
        if self.config_open {
            self.config.close();
            self.config_open = false;
        } else {
            self.config.open();
            self.config_open = true;
        }
    }

    fn quit(&mut self) {
        self.event_proxy.send_event(InterfaceMessage::Quit).unwrap();
        self.tray.quit();
        self.showing = false;
    }

    async fn listen(&mut self) {
        let mut settings_rx = settings::subscribe();
        loop {
            tokio::select! {
                Some(key_event) = self.key_rx.recv() => {
                    match key_event {
                        Event::Keyboard(Key::ScrollLock, true) => self.toggle_cross(),
                        _ => {}
                    }
                },
                Some(msg) = self.tray.recv() => {
                    match msg {
                        tray::TrayMessage::Show => self.show_cross(),
                        tray::TrayMessage::Hide => self.hide_cross(),
                        tray::TrayMessage::Config => self.toggle_config(),
                        tray::TrayMessage::Quit => self.quit(),
                    }
                },
                Some(msg) = self.config.recv() => {
                    match msg {
                        config::ConfigMessage::ShowCross => self.show_cross(),
                        config::ConfigMessage::HideCross => self.hide_cross(),
                        config::ConfigMessage::SetCross(x) => self.set_cross(x),
                        config::ConfigMessage::ConfigClosed => self.config_open = false,
                    }
                },
                Some(msg) = self.main_rx.recv() => {
                    match msg {
                        InterfaceControl::Quit => self.quit(),
                    }
                },
                Ok(_) = settings_rx.recv() => {
                    self.config.settings_changed();
                }
            }
        }
    }
}

enum InterfaceControl {
    Quit,
}

pub struct Interface {
    main_tx: UnboundedSender<InterfaceControl>,
}

impl Interface {
    pub fn quit(&mut self) {
        let _ = self.main_tx.send(InterfaceControl::Quit);
    }
}

pub fn start(event_proxy: EventLoopProxy<InterfaceMessage>) -> Interface {
    let (main_tx, main_rx) = tokio::sync::mpsc::unbounded_channel();
    tokio::spawn(async move {
        InterfaceRunner::new(event_proxy, main_rx).listen().await;
    });

    Interface { main_tx }
}
