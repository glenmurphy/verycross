use fltk::{app, button::Button, frame::Frame, prelude::*, window::Window};
use tokio::sync::mpsc::{UnboundedSender, unbounded_channel, UnboundedReceiver};
use winit::event_loop::EventLoopProxy;

use crate::interface::InterfaceMessage;

#[allow(unused)]
#[derive(Copy, Clone, Debug)]
enum Control {
    Open,
    Close,
}

pub struct ConfigInterface {
    control_tx: UnboundedSender<Control>,
}

#[allow(unused)]
impl ConfigInterface {
    pub fn open(&mut self) {
        let _ = self.control_tx.send(Control::Open);
    }
    pub fn close(&mut self) {
        let _ = self.control_tx.send(Control::Close);
    }
}

#[derive(Copy, Clone, Debug)]
enum AppControl {
    ShowButton,
    HideButton,
    Close
}

#[derive(Copy, Clone, Debug)]
enum AppMessage {
    ShowButton,
    HideButton,
    Closed
}

fn app_loop(
    config_tx: UnboundedSender<AppMessage>,
    ui_tx: app::Sender<AppControl>,
    ui_rx: app::Receiver<AppControl>,
    open_rx: &mut UnboundedReceiver<()>) {
    // We use a non-async thread so we're not blocking sends on the config_tx channels
    loop {
        let _ = open_rx.blocking_recv();

        let app = app::App::default();

        let mut wind = Window::new(100, 100, 400, 300, "Verycross Configuration");
        let mut frame = Frame::new(0, 0, 400, 200, "");
        frame.set_tooltip("hello");
                    
        let mut hide_but = Button::new(50, 210, 80, 40, "Hide");
        let mut show_but = Button::new(150, 210, 80, 40, "Show");
        let mut close_but = Button::new(250, 210, 80, 40, "Close");

        wind.end();
        wind.show();

        show_but.emit(ui_tx, AppControl::ShowButton);
        hide_but.emit(ui_tx, AppControl::HideButton);
        close_but.emit(ui_tx, AppControl::Close);

        while app.wait() {
            if let Some(msg) = ui_rx.recv() {
                match msg {
                    AppControl::ShowButton => {
                        println!("showbutton");
                        config_tx.send(AppMessage::ShowButton).unwrap();
                    }
                    AppControl::HideButton => {
                        println!("hidebutton");
                        config_tx.send(AppMessage::HideButton).unwrap();
                    }
                    AppControl::Close => {
                        println!("close button");
                        break;
                    }
                }
            }
        }
        app.quit();

        config_tx.send(AppMessage::Closed).unwrap();
        // Once a message is received on this channel, the loop begins anew.
    }
}

pub fn new(main_loop: EventLoopProxy<InterfaceMessage>) -> ConfigInterface {
    // Used by ConfigInterface to communicate with the control_loop
    let (control_tx, mut control_rx) =  unbounded_channel();

    // Used within the app, and by the control_loop to feed control into the app
    let (ui_tx, ui_rx) = fltk::app::channel::<AppControl>();

    // Used by the app to send messages to the control_loop
    let (app_tx, mut app_rx) = unbounded_channel::<AppMessage>();

    // Single-purpose - used by control_loop to signal the app to open
    let (open_tx, mut open_rx) = unbounded_channel::<()>();
    
    // We use a non-async thread so we're not blocking sends on the config_tx channels
    std::thread::spawn(move || {
        app_loop(app_tx, ui_tx, ui_rx, &mut open_rx);
    });

    // Main control loop - brokers messages between all the channels
    tokio::spawn(async move {
        let mut open = false;
        loop {
            tokio::select! {
                // Messages from the app, passed on to the main thread
                Some(msg) = app_rx.recv() => {
                    match msg {
                        AppMessage::ShowButton => main_loop.send_event(InterfaceMessage::Show).unwrap(),
                        AppMessage::HideButton => main_loop.send_event(InterfaceMessage::Hide).unwrap(),
                        AppMessage::Closed => open = false,
                    }
                },
                // Messages from ConfigInterface, passed on to the app.
                Some(msg) = control_rx.recv() => {
                    match msg {
                        Control::Open if !open => {
                            open_tx.send(()).unwrap();
                            open = true;
                        },
                        Control::Close if open => { 
                            open = false;
                            ui_tx.send(AppControl::Close);
                        },
                        _ => {}
                    }
                }
            }
        }
    });

    ConfigInterface {
        control_tx,
    }
}
