use crate::settings;
use fltk::{app, button::Button, enums::Color, frame::Frame, prelude::*, window::Window};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

pub struct ConfigInterface {
    open_tx: UnboundedSender<()>,
    config_rx: UnboundedReceiver<ConfigMessage>,
    control_tx: app::Sender<Event>,
}

#[allow(unused)]
impl ConfigInterface {
    pub fn open(&mut self) {
        self.open_tx.send(());
    }

    pub fn close(&mut self) {
        println!("Sending close");
        self.control_tx.send(Event::Close);
    }

    pub fn settings_changed(&mut self) {
        self.control_tx.send(Event::SettingsChanged);
    }

    pub async fn recv(&mut self) -> Option<ConfigMessage> {
        tokio::macros::support::poll_fn(|cx| self.config_rx.poll_recv(cx)).await
    }
}

#[derive(Copy, Clone, Debug)]
enum Event {
    // Stuff from inside the app
    ShowButton,
    HideButton,
    SetCrossButton(usize),
    CloseButton,

    // Stuff from outside the app
    Close,
    SettingsChanged,
}

#[derive(Copy, Clone, Debug)]
pub enum ConfigMessage {
    // Sent from the app thread
    ShowCross,
    HideCross,
    SetCross(usize),
    ConfigClosed,
}

fn create_crosshair_buttons(control_tx: app::Sender<Event>) -> Vec<Button> {
    let mut c0 = Button::new(50, 50, 80, 30, "Cross 0");
    c0.emit(control_tx, Event::SetCrossButton(0));

    let mut c1 = Button::new(50, 90, 80, 30, "Cross 1");
    c1.emit(control_tx, Event::SetCrossButton(1));

    let mut c2 = Button::new(50, 130, 80, 30, "Cross 2");
    c2.emit(control_tx, Event::SetCrossButton(2));

    vec![c0, c1, c2]
}

fn update_crosshair_buttons(buttons: &mut Vec<Button>, index: usize) {
    for i in 0..buttons.len() {
        buttons[i].set_color(if i == index { Color::Red } else { Color::White });
    }
}

/// We use a lot of channels because fltk's channels cannot block, which we need
/// for the opening code, but fltk's wait() loop won't trigger on channels
/// other than its own.
fn app_loop(
    config_tx: UnboundedSender<ConfigMessage>,
    event_tx: app::Sender<Event>,
    event_rx: app::Receiver<Event>,
    open_rx: &mut UnboundedReceiver<()>,
) {
    loop {
        open_rx.blocking_recv();

        let app = app::App::default();

        let mut wind = Window::new(100, 100, 400, 300, "Verycross Configuration");
        let mut frame = Frame::new(0, 0, 400, 200, "");
        frame.set_label("hello");

        let mut buttons = create_crosshair_buttons(event_tx.clone());
        update_crosshair_buttons(&mut buttons, settings::get().crosshair);

        Button::new(50, 210, 60, 30, "Hide").emit(event_tx, Event::HideButton);
        Button::new(150, 210, 60, 30, "Show").emit(event_tx, Event::ShowButton);
        Button::new(250, 210, 60, 30, "Close").emit(event_tx, Event::CloseButton);

        wind.end();
        wind.show();

        while app.wait() {
            match event_rx.recv() {
                // Stuff from the app
                Some(Event::ShowButton) => config_tx.send(ConfigMessage::ShowCross).unwrap(),
                Some(Event::HideButton) => config_tx.send(ConfigMessage::HideCross).unwrap(),
                Some(Event::SetCrossButton(n)) => {
                    config_tx.send(ConfigMessage::SetCross(n)).unwrap()
                }
                Some(Event::CloseButton) => break,

                // Stuff from outside the app
                Some(Event::Close) => break,
                Some(Event::SettingsChanged) => {
                    update_crosshair_buttons(&mut buttons, settings::get().crosshair);
                    app.redraw();
                }
                _ => {}
            }
        }

        app.quit();
        config_tx.send(ConfigMessage::ConfigClosed).unwrap();
    }
}

pub fn new() -> ConfigInterface {
    // The app's main event handling mechanism; used internally by buttons,
    // but also to feed content into the app
    let (event_tx, event_rx) = fltk::app::channel::<Event>();

    // Used by the app to send messages outwards
    let (config_tx, config_rx) = unbounded_channel::<ConfigMessage>();

    // Used to signal the app to open
    let (open_tx, mut open_rx) = unbounded_channel::<()>();

    // The app is not async and blocks on its own event loop
    tokio::task::spawn_blocking(move || {
        app_loop(config_tx, event_tx, event_rx, &mut open_rx);
    });

    ConfigInterface {
        open_tx,
        config_rx,
        control_tx: event_tx,
    }
}
