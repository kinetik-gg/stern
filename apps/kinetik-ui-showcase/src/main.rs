//! Windowed Kinetik UI showcase entry point.

use kinetik_ui_showcase::{
    editor_shell,
    raster::{rasterize, write_bmp},
};
use minifb::{Key, Window, WindowOptions};

const WIDTH: usize = 1440;
const HEIGHT: usize = 900;

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

    let scenario = editor_shell();
    let frame = rasterize(&scenario.primitives, WIDTH, HEIGHT);
    if let Some(path) = render_once_path(&args) {
        write_bmp(&frame, path).expect("write showcase bmp");
        return;
    }
    let mut window = Window::new(
        "Kinetik UI Showcase",
        WIDTH,
        HEIGHT,
        WindowOptions {
            resize: true,
            ..WindowOptions::default()
        },
    )
    .expect("create showcase window");
    window.set_target_fps(60);

    while window.is_open() && !window.is_key_down(Key::Escape) {
        window
            .update_with_buffer(&frame.pixels, frame.width, frame.height)
            .expect("present showcase frame");
    }
}

fn render_once_path(args: &[String]) -> Option<&str> {
    args.windows(2)
        .find_map(|window| (window[0] == "--render-once").then_some(window[1].as_str()))
}
