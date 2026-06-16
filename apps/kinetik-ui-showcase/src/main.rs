//! Windowed Kinetik UI showcase entry point.

use kinetik_ui::core::{Point, Size};
use kinetik_ui_showcase::{
    app::{ShowcaseApp, ShowcaseInput},
    raster::{rasterize, write_bmp},
};
use minifb::{Key, KeyRepeat, MouseButton, MouseMode, Scale, ScaleMode, Window, WindowOptions};

const DEFAULT_WIDTH: usize = 1440;
const DEFAULT_HEIGHT: usize = 900;

fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    if args.iter().any(|arg| arg == "--list") {
        for scenario in kinetik_ui_showcase::all_scenarios() {
            println!(
                "{}: {} primitives",
                scenario.name,
                scenario.primitives.len()
            );
        }
        return;
    }

    if let Some(path) = render_once_path(&args) {
        let width = usize_arg(&args, "--width").unwrap_or(DEFAULT_WIDTH);
        let height = usize_arg(&args, "--height").unwrap_or(DEFAULT_HEIGHT);
        let mut app = ShowcaseApp::new();
        app.set_viewport_size(size_from_pixels(width, height));
        if let Some(page) = page_arg(&args).and_then(ShowcaseApp::page_from_name) {
            app.set_page(page);
        }
        let frame = rasterize(&app.primitives(), width, height);
        write_bmp(&frame, path).expect("write showcase bmp");
        return;
    }
    let mut window = Window::new(
        "Kinetik UI Showcase",
        DEFAULT_WIDTH,
        DEFAULT_HEIGHT,
        WindowOptions {
            resize: true,
            scale: Scale::X1,
            scale_mode: ScaleMode::UpperLeft,
            ..WindowOptions::default()
        },
    )
    .expect("create showcase window");
    window.set_background_color(12, 12, 13);
    window.set_target_fps(60);
    let mut app = ShowcaseApp::new();

    while window.is_open() && !window.is_key_down(Key::Escape) {
        let (width, height) = window.get_size();
        let width = width.max(1);
        let height = height.max(1);
        let input = window_input(&window, width, height);
        app.update(&input);
        let primitives = app.primitives();
        let frame = rasterize(&primitives, width, height);
        window
            .update_with_buffer(&frame.pixels, frame.width, frame.height)
            .expect("present showcase frame");
    }
}

fn render_once_path(args: &[String]) -> Option<&str> {
    args.windows(2)
        .find_map(|window| (window[0] == "--render-once").then_some(window[1].as_str()))
}

fn page_arg(args: &[String]) -> Option<&str> {
    args.windows(2)
        .find_map(|window| (window[0] == "--page").then_some(window[1].as_str()))
}

fn usize_arg(args: &[String], name: &str) -> Option<usize> {
    args.windows(2)
        .find_map(|window| (window[0] == name).then(|| window[1].parse().ok()))
        .flatten()
}

fn window_input(window: &Window, width: usize, height: usize) -> ShowcaseInput {
    let mouse = window
        .get_mouse_pos(MouseMode::Clamp)
        .map(|(x, y)| Point::new(x, y));
    let keys = window.get_keys_pressed(KeyRepeat::Yes);
    ShowcaseInput {
        mouse,
        viewport_size: Some(size_from_pixels(width, height)),
        mouse_down: window.get_mouse_down(MouseButton::Left),
        typed: keys.iter().filter_map(|key| key_to_char(*key)).collect(),
        backspace: keys.contains(&Key::Backspace),
        enter: keys.contains(&Key::Enter),
    }
}

fn size_from_pixels(width: usize, height: usize) -> Size {
    Size::new(pixel_to_f32(width), pixel_to_f32(height))
}

fn pixel_to_f32(value: usize) -> f32 {
    let value = u16::try_from(value).unwrap_or(u16::MAX);
    f32::from(value)
}

fn key_to_char(key: Key) -> Option<char> {
    Some(match key {
        Key::A => 'a',
        Key::B => 'b',
        Key::C => 'c',
        Key::D => 'd',
        Key::E => 'e',
        Key::F => 'f',
        Key::G => 'g',
        Key::H => 'h',
        Key::I => 'i',
        Key::J => 'j',
        Key::K => 'k',
        Key::L => 'l',
        Key::M => 'm',
        Key::N => 'n',
        Key::O => 'o',
        Key::P => 'p',
        Key::Q => 'q',
        Key::R => 'r',
        Key::S => 's',
        Key::T => 't',
        Key::U => 'u',
        Key::V => 'v',
        Key::W => 'w',
        Key::X => 'x',
        Key::Y => 'y',
        Key::Z => 'z',
        Key::Key0 | Key::NumPad0 => '0',
        Key::Key1 | Key::NumPad1 => '1',
        Key::Key2 | Key::NumPad2 => '2',
        Key::Key3 | Key::NumPad3 => '3',
        Key::Key4 | Key::NumPad4 => '4',
        Key::Key5 | Key::NumPad5 => '5',
        Key::Key6 | Key::NumPad6 => '6',
        Key::Key7 | Key::NumPad7 => '7',
        Key::Key8 | Key::NumPad8 => '8',
        Key::Key9 | Key::NumPad9 => '9',
        Key::Space => ' ',
        Key::Period | Key::NumPadDot => '.',
        Key::Minus | Key::NumPadMinus => '-',
        _ => return None,
    })
}
