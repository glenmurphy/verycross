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
use winit_blit::{NativeFormat, PixelBufferTyped, BGRA};
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
        if SetWindowLongPtrW(
            hwnd,
            GWL_EXSTYLE,
            window_styles | WS_EX_TRANSPARENT as isize | WS_EX_LAYERED as isize | WS_EX_TOOLWINDOW as isize,
        ) == 0 {
            panic!("SetWindowLongPtrW returned 0");
        };
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
    let mut buffer =
        PixelBufferTyped::<NativeFormat>::new_supported(width, height, &window);

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
    let mut png_buffer = vec![0; reader.output_buffer_size()];
    reader.next_frame(&mut png_buffer).unwrap();

    let width = reader.info().width as usize;
    let height = reader.info().height as usize;

    let mut buffer: Vec<BGRA> = vec![BGRA { b: 0, g : 255, r : 0, a : 255}; width * height];
    for y in 0..height {
        for x in 0..width {
            let i = (y * width + x) * 4;
            let r = png_buffer[i + 0];
            let g = png_buffer[i + 1];
            let b = png_buffer[i + 2];
            let a = png_buffer[i + 3];

            buffer[y * width + x] = NativeFormat::new(b, g, r, a);
        }
    }

    Image {
        width, height, buffer
    }
}

fn show_window(window: &Window) {
    window.set_visible(true);
    make_overlay(&window);
}

fn hide_window(window: &Window) {
    window.set_visible(false);
}

#[derive(Debug, Clone, Copy)]
enum WindowControl {
    Show,
    Hide,
    Quit,
}

#[tokio::main]
async fn main() {
    let crosshair = load_image(include_bytes!("../assets/crosshair.png"));

    let event_loop = EventLoop::<WindowControl>::with_user_event();

    let core = WindowBuilder::new()
        .with_visible(false)
        .build(&event_loop)
        .unwrap();
    let window = WindowBuilder::new()
        .with_owner_window(core.hwnd() as _)
        .with_inner_size(LogicalSize::new(crosshair.width as u32, crosshair.height as u32))
        .with_decorations(false)
        .with_transparent(true)
        .with_visible(false)
        .with_always_on_top(true)
        .build(&event_loop)
        .unwrap();
    window.set_enable(false);
    
    center_window(&window);
    show_window(&window);

    let proxy = event_loop.create_proxy();
    let mut tray = tray::start();    
    let mut key_rx = keyboard::listen();
    tokio::spawn(async move {
        let mut showing = true;
        loop {
            tokio::select! {
                Some((code, down)) = key_rx.recv() => {
                    println!("key: {}, {}", code, down);
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

    
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::UserEvent( WindowControl::Show ) => {
                show_window(&window);
            },
            Event::UserEvent( WindowControl::Hide ) => {
                hide_window(&window);
            },
            Event::UserEvent( WindowControl::Quit ) => {
                *control_flow = ControlFlow::Exit;
            },
            Event::WindowEvent { event: WindowEvent::CloseRequested, window_id, } if window_id == window.id() => {
                *control_flow = ControlFlow::Exit;
            },
            Event::RedrawRequested(window_id) if window_id == window.id() => {
                println!("redraw");
                fill_window(&crosshair, &window);
            },
            Event::WindowEvent { window_id, event: WindowEvent::ScaleFactorChanged {..} } if window_id == window.id() => {
                center_window(&window);
            }
            _ => (),
        }
    });
}