use winit::event_loop::EventLoopProxy;
use winky::Key;
use crate::{tray};

#[derive(Debug, Clone, Copy)]
pub enum InterfaceMessage {
    Show,
    Hide,
    Jiggle,
    Quit,
}

pub fn start(proxy: EventLoopProxy<InterfaceMessage>) {
    let mut tray = tray::start();
    let mut key_rx = winky::listen();
    tokio::spawn(async move {
        let mut showing = true;
        loop {
            tokio::select! {
                Some((code, down)) = key_rx.recv() => {
                    if code == Key::ScrollLock as u32 && down {
                        if showing {
                            proxy.send_event(InterfaceMessage::Hide).unwrap();
                            tray.off();
                            showing = false;
                        } else {
                            proxy.send_event(InterfaceMessage::Show).unwrap();
                            tray.on();
                            showing = true;
                        }
                    }
                }
                Some(msg) = tray.recv() => {
                    match msg {
                        tray::TrayMessage::Show => {
                            proxy.send_event(InterfaceMessage::Show).unwrap();
                            tray.on();
                            showing = true;
                        }
                        tray::TrayMessage::Hide => {
                            proxy.send_event(InterfaceMessage::Hide).unwrap();
                            tray.off();
                            showing = false;
                        }
                        tray::TrayMessage::Quit => {
                            tray.quit();
                            proxy.send_event(InterfaceMessage::Quit).unwrap();
                            showing = false;
                        }
                    }
                }
            }
        }
    });
}