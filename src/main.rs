// Hide the console on windows release builds
#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder, window::Window, platform::windows::{WindowBuilderExtWindows, WindowExtWindows},
    dpi::{LogicalSize, PhysicalPosition},
};
use winit_blit::{NativeFormat, PixelBufferTyped};
use keyboard::Key;

mod tray;

/// Makes the window transparent to events
/// derived from
/// https://github.com/rust-windowing/winit/pull/2232
/// https://github.com/maroider/overlay/blob/1a80a30d6ef8e6b2fe6fa273a2cc2b472a3b2e51/src/os.rs
fn make_overlay(window: &Window) {
    use winapi::{
        shared::windef::HWND__,
        um::winuser::{
            GetWindowLongPtrW, SetWindowLongPtrW, GWL_EXSTYLE, WS_EX_TRANSPARENT, WS_EX_LAYERED, WS_EX_TOOLWINDOW
        },
    };
    let hwnd = window.hwnd() as *mut HWND__;
    unsafe {
        let window_styles: isize = match { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) } {
            0 => panic!("GetWindowLongPtrW returned 0"),
            ptr => ptr,
        };
        SetWindowLongPtrW(
            hwnd,
            GWL_EXSTYLE,
            window_styles | WS_EX_TRANSPARENT as isize | WS_EX_LAYERED as isize | WS_EX_TOOLWINDOW as isize,
        );
    }
}

fn center_window(window: &Window) {
    let monitor_size = window.primary_monitor().unwrap().size();
    let window_size = window.inner_size();
    let x = (monitor_size.width - window_size.width) / 2;
    let y = (monitor_size.height - window_size.height) / 2;
    window.set_outer_position(PhysicalPosition::new(x, y));
}

fn fill_window(window: &Window) {
    // For better drawing/PNG loading, see the example code linked at:
    // https://github.com/rust-windowing/winit/issues/2109
    let (width, height): (u32, u32) = window.inner_size().into();
    let mut buffer =
        PixelBufferTyped::<NativeFormat>::new_supported(width, height, &window);

    let green = NativeFormat::new(0, 255, 0, 127);
    for (_, row) in buffer.rows_mut().enumerate() {
        for (_, pixel) in row.into_iter().enumerate() {
            *pixel = green;
        }
    }

    buffer.blit(&window).unwrap();
}

#[derive(Debug, Clone, Copy)]
enum WindowControl {
    Show,
    Hide,
    Quit,
}

#[tokio::main]
async fn main() {
    let width: u32 = 2;
    let height: u32 = 2;

    let event_loop = EventLoop::<WindowControl>::with_user_event();

    let core = WindowBuilder::new()
        .with_visible(false)
        .build(&event_loop)
        .unwrap();
    let window = WindowBuilder::new()
        .with_owner_window(core.hwnd() as _)
        .with_inner_size(LogicalSize::new(width, height))
        .with_visible(true)
        .with_decorations(false)
        .with_transparent(true)
        .with_always_on_top(true)
        .build(&event_loop)
        .unwrap();
    window.set_enable(false);

    center_window(&window);
    make_overlay(&window);

    let proxy = event_loop.create_proxy();
    let mut tray = tray::start();    
    tokio::spawn(async move {
        let mut key_rx = keyboard::listen();
        let mut showing = true;

        loop {
            tokio::select! {
                Some((code, down)) = key_rx.recv() => {
                    println!("key: {}, {}", code, down);
                    if code == Key::F10 as u32 && down{
                        if showing {
                            proxy.send_event(WindowControl::Hide).unwrap();
                            tray.off();
                            showing = false;
                        } else {
                            proxy.send_event(WindowControl::Show).unwrap();
                            tray.on();
                            showing = true;
                        }
                    }
                }
                Some(msg) = tray.recv() => {
                    match msg {
                        tray::TrayMessage::Show => {
                            proxy.send_event(WindowControl::Show).unwrap();
                            tray.on();
                            showing = true;
                        }
                        tray::TrayMessage::Hide => {
                            proxy.send_event(WindowControl::Hide).unwrap();
                            tray.off();
                            showing = false;
                        }
                        tray::TrayMessage::Quit => {
                            tray.quit();
                            proxy.send_event(WindowControl::Quit).unwrap();
                            showing = false;
                        }
                    }
                }
            }
            
        }
    });

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::UserEvent( WindowControl::Show ) => {
                window.set_visible(true);
            },
            Event::UserEvent( WindowControl::Hide ) => {
                window.set_visible(false);
            },
            Event::UserEvent( WindowControl::Quit ) => {
                *control_flow = ControlFlow::Exit;
            },
            Event::WindowEvent { event: WindowEvent::CloseRequested, window_id, } if window_id == window.id() => {
                *control_flow = ControlFlow::Exit;
            },
            Event::RedrawRequested(window_id) if window_id == window.id() => {
                fill_window(&window);
            },
            Event::WindowEvent { window_id, event: WindowEvent::ScaleFactorChanged {..} } if window_id == window.id() => {
                center_window(&window);
            }
            _ => (),
        }
    });
}