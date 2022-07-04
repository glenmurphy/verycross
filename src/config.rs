use fltk::{app, button::Button, frame::Frame, prelude::*, window::Window};
use tokio::sync::mpsc::{UnboundedSender, unbounded_channel, UnboundedReceiver};
use winit::event_loop::EventLoopProxy;

use crate::interface::InterfaceMessage;

pub struct ConfigInterface {
    app_tx: UnboundedSender<AppMessage>,
    ui_tx: app::Sender<AppControl>,
}

#[allow(unused)]
impl ConfigInterface {
    pub fn open(&mut self) {
        self.app_tx.send(AppMessage::Open).unwrap();
    }
    pub fn close(&mut self) {
        self.ui_tx.send(AppControl::Close);
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
    // Sent from the app thread
    ShowButton,
    HideButton,
    Closed,

    // Sent from ConfigInterface
    Open
}

fn app_loop(
    config_tx: UnboundedSender<AppMessage>,
    ui_tx: app::Sender<AppControl>,
    ui_rx: app::Receiver<AppControl>,
    open_rx: &mut UnboundedReceiver<()>) {
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
            match ui_rx.recv() {
                Some(AppControl::ShowButton) => config_tx.send(AppMessage::ShowButton).unwrap(),
                Some(AppControl::HideButton) => config_tx.send(AppMessage::HideButton).unwrap(),
                Some(AppControl::Close) => break,
                _ => {}
            }
        }

        app.quit();
        config_tx.send(AppMessage::Closed).unwrap();
    }
}

pub fn new(main_loop: EventLoopProxy<InterfaceMessage>) -> ConfigInterface {
    // Used within the app, and by the control_loop to feed control into the app
    let (ui_tx, ui_rx) = fltk::app::channel::<AppControl>();

    // Used by the app to send messages to the control_loop
    let (app_tx, mut app_rx) = unbounded_channel::<AppMessage>();

    // Clone for the Config Interface to send messages to the control loop
    let control_tx = app_tx.clone();

    // Used by control_loop to signal the app to open
    let (open_tx, mut open_rx) = unbounded_channel::<()>();
    
    // We use a non-async thread so we're not blocking sends on the config_tx channels
    std::thread::spawn(move || {
        app_loop(app_tx, ui_tx, ui_rx, &mut open_rx);
    });

    // Main control loop
    tokio::spawn(async move {
        let mut open = false;
        loop {
            let msg = app_rx.recv().await.unwrap();
            match msg {
                AppMessage::ShowButton => main_loop.send_event(InterfaceMessage::Show).unwrap(),
                AppMessage::HideButton => main_loop.send_event(InterfaceMessage::Hide).unwrap(),
                AppMessage::Closed => open = false,

                // Open comes from the config_interface, but we broker it through here because
                // this is where we can keep track of open states
                AppMessage::Open if !open => {
                    open_tx.send(()).unwrap();
                    open = true;
                }
                _ => {}
            }
        }
    });

    ConfigInterface {
        app_tx : control_tx,
        ui_tx,
    }
}
