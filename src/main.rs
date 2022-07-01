// Hide the console on windows release builds
#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

use keyboard::Key;
use single_instance::SingleInstance;
use winapi::{
    shared::windef::HWND__,
    um::winuser::{
        GetForegroundWindow, GetWindowLongPtrW, SetForegroundWindow,
        SetWindowLongPtrW, SetWindowPos, GWL_EXSTYLE, HWND_TOPMOST, SWP_NOMOVE, SWP_NOSIZE,
        WS_EX_LAYERED, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT,
    },
};
use winit::{
    dpi::{LogicalSize, PhysicalPosition},
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    platform::windows::{WindowBuilderExtWindows, WindowExtWindows},
    window::Window,
    window::WindowBuilder,
};
use winit_blit::{NativeFormat, PixelBufferTyped, BGRA};

mod tray;

/// Makes the window transparent to events
/// derived from
/// https://github.com/rust-windowing/winit/pull/2232
/// https://github.com/maroider/overlay/blob/1a80a30d6ef8e6b2fe6fa273a2cc2b472a3b2e51/src/os.rs
fn make_overlay(window: &Window) {
    let hwnd = window.hwnd() as *mut HWND__;
    unsafe {
        let window_styles: isize = match { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) } {
            0 => panic!("GetWindowLongPtrW returned 0"),
            ptr => ptr,
        };
        if SetWindowLongPtrW(
            hwnd,
            GWL_EXSTYLE,
            window_styles
                | WS_EX_TRANSPARENT as isize
                | WS_EX_LAYERED as isize
                | WS_EX_TOOLWINDOW as isize
                | WS_EX_TOPMOST as isize,
        ) == 0
        {
            panic!("SetWindowLongPtrW returned 0");
        };
    }
}

fn set_topmost(window: &Window) {
    unsafe {
        let hwnd = window.hwnd() as *mut HWND__;
        SetWindowPos(hwnd, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE);
    }
}

fn center_window(window: &Window) {
    let monitor_size = window.primary_monitor().unwrap().size();
    let window_size = window.inner_size();
    let x = (monitor_size.width - window_size.width) / 2;
    let y = (monitor_size.height - window_size.height) / 2;
    window.set_outer_position(PhysicalPosition::new(x, y));
}

struct Image {
    width: usize,
    height: usize,
    buffer: Vec<BGRA>,
}

fn fill_window(image: &Image, window: &Window) {
    let (width, height): (u32, u32) = window.inner_size().into();
    let mut buffer = PixelBufferTyped::<NativeFormat>::new_supported(width, height, &window);

    for (y, row) in buffer.rows_mut().enumerate() {
        for (x, pixel) in row.into_iter().enumerate() {
            *pixel = image.buffer[(y * width as usize + x)];
        }
    }

    buffer.blit(&window).unwrap();
}

fn load_image(bytes: &[u8]) -> Image {
    let decoder = png::Decoder::new(bytes);

    let mut reader = decoder.read_info().unwrap();
    let mut buf = vec![0; reader.output_buffer_size()];
    reader.next_frame(&mut buf).unwrap();

    let width = reader.info().width as usize;
    let height = reader.info().height as usize;

    let mut buffer: Vec<BGRA> = vec![BGRA { b : 0, g : 0, r : 0, a : 0 }; width * height];
    for y in 0..height {
        for x in 0..width {
            let i = (y * width + x) * 4;
            let (r, g, b, a) = (buf[i + 0], buf[i + 1], buf[i + 2], buf[i + 3]);
            buffer[y * width + x] = BGRA { b, g, r, a };
        }
    }

    Image {
        width,
        height,
        buffer,
    }
}

fn show_window(window: &Window) {
    center_window(&window);
    make_overlay(&window);
    set_topmost(window);
}

fn hide_window(window: &Window) {    
    // set_visible occasionally steals focus, so we send the window
    // offscreen instead
    window.set_outer_position(PhysicalPosition::new(-1000, -1000));
}

fn error_dialog(message: &str) {
    println!("Showing error: {}", message);
    use std::ffi::CString;
    let lp_text = CString::new(message).unwrap();
    let lp_caption = CString::new("Error").unwrap();
    unsafe {
        winapi::um::winuser::MessageBoxA(
            GetForegroundWindow(),
            lp_text.as_ptr(),
            lp_caption.as_ptr(),
            0,
        );
    }
}

#[derive(Debug, Clone, Copy)]
enum WindowControl {
    Show,
    Hide,
    Jiggle,
    Quit,
}

#[tokio::main]
async fn main() {
    let instance = SingleInstance::new("verycross").unwrap();
    if !instance.is_single() {
        error_dialog("Verycross is already running");
        return;
    }

    // Get foreground window so we can restore focus later
    let previous_focus = unsafe { GetForegroundWindow() };

    let crosshair = load_image(include_bytes!("../assets/crosshair.png"));

    let event_loop = EventLoop::<WindowControl>::with_user_event();

    let core = WindowBuilder::new()
        .with_visible(false)
        .build(&event_loop)
        .unwrap();
    let window = WindowBuilder::new()
        .with_owner_window(core.hwnd() as _)
        .with_inner_size(LogicalSize::new(
            crosshair.width as u32,
            crosshair.height as u32,
        ))
        .with_decorations(false)
        .with_transparent(true)
        .with_visible(true)
        .with_always_on_top(true)
        .build(&event_loop)
        .unwrap();
    window.set_enable(false);

    center_window(&window);
    show_window(&window);

    let mut tray = tray::start();
    let mut key_rx = keyboard::listen();
    
    let proxy = event_loop.create_proxy();
    tokio::spawn(async move {
        // Sometimes fullscreen apps will put themselves over the window, 
        // so this puts the window back on top once a second
        loop {
            std::thread::sleep(std::time::Duration::from_secs(1));
            proxy.send_event(WindowControl::Jiggle).unwrap();
        }
    });

    let proxy = event_loop.create_proxy();
    tokio::spawn(async move {
        let mut showing = true;
        loop {
            tokio::select! {
                Some((code, down)) = key_rx.recv() => {
                    if code == Key::F10 as u32 && down {
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

    // Restore focus to the previously focused window
    unsafe {
        SetForegroundWindow(previous_focus);
    }

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::UserEvent(WindowControl::Show) => {
                show_window(&window);
            }
            Event::UserEvent(WindowControl::Hide) => {
                hide_window(&window);
            }
            Event::UserEvent(WindowControl::Jiggle) => {
                set_topmost(&window);
            }
            Event::UserEvent(WindowControl::Quit) => {
                *control_flow = ControlFlow::Exit;
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                window_id,
            } if window_id == window.id() => {
                *control_flow = ControlFlow::Exit;
            }
            Event::RedrawRequested(window_id) if window_id == window.id() => {
                fill_window(&crosshair, &window);
            }
            Event::WindowEvent {
                window_id,
                event: WindowEvent::ScaleFactorChanged { .. },
            } if window_id == window.id() => {
                center_window(&window);
            }
            _ => (),
        }
    });
}
