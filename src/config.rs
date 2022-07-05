use fltk::{app, button::Button, frame::Frame, prelude::*, window::Window};
use tokio::sync::mpsc::{UnboundedSender, unbounded_channel, UnboundedReceiver};

pub struct ConfigInterface {
    open_tx: UnboundedSender<()>,
    config_rx: UnboundedReceiver<ConfigMessage>,
    control_tx: app::Sender<UIEvent>,
}

#[allow(unused)]
impl ConfigInterface {
    pub fn open(&mut self) {
        self.open_tx.send(());
    }
    pub fn close(&mut self) {
        println!("Sending close");
        self.control_tx.send(UIEvent::Close);
    }
    pub async fn recv(&mut self) -> Option<ConfigMessage> {
        tokio::macros::support::poll_fn(|cx| self.config_rx.poll_recv(cx)).await
    }
}

#[derive(Copy, Clone, Debug)]
enum UIEvent {
    ShowButton,
    HideButton,
    CrossButton(usize),
    Close
}

#[derive(Copy, Clone, Debug)]
pub enum ConfigMessage {
    // Sent from the app thread
    ShowCross,
    HideCross,
    SetCross(usize),
    ConfigClosed,
}

/// We use a lot of channels because fltk's channels have no blocking option, which
/// we need for the opening code, but fltk's wait() loop won't trigger on channels
/// other than its own.
fn app_loop(
    config_tx: UnboundedSender<ConfigMessage>,
    control_tx: app::Sender<UIEvent>,
    control_rx: app::Receiver<UIEvent>,
    open_rx: &mut UnboundedReceiver<()>) {
    
    loop {
        open_rx.blocking_recv();

        let app = app::App::default();

        let mut wind = Window::new(100, 100, 400, 300, "Verycross Configuration");
        let mut frame = Frame::new(0, 0, 400, 200, "");
        frame.set_label("hello");

        let mut cross0_but = Button::new(50, 50, 80, 30, "Cross 1");
        let mut cross1_but = Button::new(50, 90, 80, 30, "Cross 2");

        let mut hide_but = Button::new(50, 210, 60, 30, "Hide");
        let mut show_but = Button::new(150, 210, 60, 30, "Show");
        let mut close_but = Button::new(250, 210, 60, 30, "Close");

        wind.end();
        wind.show();

        cross0_but.emit(control_tx, UIEvent::CrossButton(0));
        cross1_but.emit(control_tx, UIEvent::CrossButton(1));
        show_but.emit(control_tx, UIEvent::ShowButton);
        hide_but.emit(control_tx, UIEvent::HideButton);
        close_but.emit(control_tx, UIEvent::Close);

        while app.wait() {
            match control_rx.recv() {
                Some(UIEvent::ShowButton) => config_tx.send(ConfigMessage::ShowCross).unwrap(),
                Some(UIEvent::HideButton) => config_tx.send(ConfigMessage::HideCross).unwrap(),
                Some(UIEvent::CrossButton(n)) => config_tx.send(ConfigMessage::SetCross(n)).unwrap(),
                Some(UIEvent::Close) => break,
                _ => {}
            }
        }

        app.quit();
        config_tx.send(ConfigMessage::ConfigClosed).unwrap();
    }
}

pub fn new() -> ConfigInterface {
    // Used within the app, and by the control_loop to feed events into the app
    let (control_tx, control_rx) = fltk::app::channel::<UIEvent>();

    // Used by the app to send messages to the control_loop
    let (config_tx, config_rx) = unbounded_channel::<ConfigMessage>();

    // Used by control_loop to signal the app to open
    let (open_tx, mut open_rx) = unbounded_channel::<()>();
    
    // We use a non-async thread so we're not blocking sends on the config_tx channels
    std::thread::spawn(move || {
        app_loop(config_tx, control_tx, control_rx, &mut open_rx);
    });

    ConfigInterface {
        open_tx,
        config_rx,
        control_tx,
    }
}
