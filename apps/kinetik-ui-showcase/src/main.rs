//! Windowed Kinetik UI showcase entry point.

mod live;

use kinetik_ui::core::Size;
use kinetik_ui_showcase::{
    app::ShowcaseApp,
    raster::{rasterize, write_bmp},
};

const DEFAULT_WIDTH: usize = 1440;
const DEFAULT_HEIGHT: usize = 900;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args().collect::<Vec<_>>();
    if args.iter().any(|arg| arg == "--list") {
        for scenario in kinetik_ui_showcase::all_scenarios() {
            println!(
                "{}: {} primitives",
                scenario.name,
                scenario.primitives.len()
            );
        }
        return Ok(());
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
        write_bmp(&frame, path)?;
        return Ok(());
    }

    live::run(page_arg(&args).and_then(ShowcaseApp::page_from_name))?;
    Ok(())
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

fn size_from_pixels(width: usize, height: usize) -> Size {
    Size::new(pixel_to_f32(width), pixel_to_f32(height))
}

fn pixel_to_f32(value: usize) -> f32 {
    let value = u16::try_from(value).unwrap_or(u16::MAX);
    f32::from(value)
}
