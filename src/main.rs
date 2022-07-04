// Hide the console on windows release builds
#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]
mod interface;
mod tray;
mod config;

use single_instance::SingleInstance;
use winapi::{
    shared::windef::HWND__,
    um::winuser::{
        GetForegroundWindow, GetWindowLongPtrW, SetForegroundWindow,
        SetWindowLongPtrW, SetWindowPos, GWL_EXSTYLE, HWND_TOPMOST, SWP_NOMOVE, SWP_NOSIZE,
        WS_EX_LAYERED, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT, MessageBoxA,
    },
};
use winit::{
    dpi::{LogicalSize, PhysicalPosition},
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop, EventLoopProxy},
    platform::windows::{WindowBuilderExtWindows, WindowExtWindows},
    window::Window,
    window::WindowBuilder,
};
use winit_blit::{NativeFormat, PixelBufferTyped, BGRA};
use interface::InterfaceMessage;

fn create_window(width: u32, height: u32, event_loop: &EventLoop<InterfaceMessage>) -> (Window, Window) {
    let core = WindowBuilder::new()
        .with_visible(false)
        .build(&event_loop)
        .unwrap();
    let window = WindowBuilder::new()
        .with_owner_window(core.hwnd() as _)
        .with_inner_size(LogicalSize::new(width, height,))
        .with_decorations(false)
        .with_transparent(true)
        .with_visible(true)
        .with_always_on_top(true)
        .build(&event_loop)
        .unwrap();
    window.set_enable(false);
    center_window(&window);
    show_window(&window);
    (core, window)
}

/// Makes the window transparent to events
/// derived from
/// https://github.com/rust-windowing/winit/pull/2232
/// https://github.com/maroider/overlay/blob/1a80a30d6ef8e6b2fe6fa273a2cc2b472a3b2e51/src/os.rs
fn make_overlay(window: &Window) {
    let hwnd = window.hwnd() as *mut HWND__;
    unsafe {
        let window_styles = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        assert!(window_styles != 0);

        let result = SetWindowLongPtrW(
            hwnd,
            GWL_EXSTYLE,
            window_styles
                | WS_EX_TRANSPARENT as isize
                | WS_EX_LAYERED as isize
                | WS_EX_TOOLWINDOW as isize
                | WS_EX_TOPMOST as isize,
        );
        assert!(result != 0);
    }
}

/// Sets the window to be the topmost window; we have to 
/// call this occasionally because other apps will sometimes
/// attempt to do the same.
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
    width: u32,
    height: u32,
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

    let width = reader.info().width;
    let height = reader.info().height;

    let mut buffer: Vec<BGRA> = vec![BGRA { b : 0, g : 0, r : 0, a : 0 }; (width * height) as usize];
    for y in 0..height {
        for x in 0..width {
            let base = (y * width + x) as usize;
            let i = base * 4;
            let (r, g, b, a) = (buf[i + 0], buf[i + 1], buf[i + 2], buf[i + 3]);
            buffer[base] = BGRA { b, g, r, a };
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
    let lp_text = std::ffi::CString::new(message).unwrap();
    let lp_caption = std::ffi::CString::new("Error").unwrap();
    unsafe {
        MessageBoxA(
            GetForegroundWindow(),
            lp_text.as_ptr(),
            lp_caption.as_ptr(),
            0,
        );
    }
}

fn start_jiggler(proxy: EventLoopProxy<InterfaceMessage>, ms: u64) {
    tokio::spawn(async move {
        // Sometimes fullscreen apps will put themselves over the window, 
        // so this puts the window back on top once a second
        loop {
            std::thread::sleep(std::time::Duration::from_millis(ms));
            proxy.send_event(InterfaceMessage::Jiggle).unwrap();
        }
    });
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
    let event_loop = EventLoop::<InterfaceMessage>::with_user_event();
    let (_core, window) = create_window(crosshair.width, crosshair.height, &event_loop);
    start_jiggler(event_loop.create_proxy(), 1000);
    interface::start(event_loop.create_proxy());
    let mut config = config::new(event_loop.create_proxy());

    // Restore focus to the previously focused window
    unsafe { SetForegroundWindow(previous_focus); }

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::UserEvent(InterfaceMessage::Show) => show_window(&window),
            Event::UserEvent(InterfaceMessage::Hide) => hide_window(&window),
            Event::UserEvent(InterfaceMessage::Config) => config.open(),
            Event::UserEvent(InterfaceMessage::Jiggle) => set_topmost(&window),
            Event::UserEvent(InterfaceMessage::Quit) => *control_flow = ControlFlow::Exit,
            Event::RedrawRequested(window_id) if window_id == window.id() => fill_window(&crosshair, &window),
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                window_id,
            } if window_id == window.id() => {
                *control_flow = ControlFlow::Exit;
            },
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
