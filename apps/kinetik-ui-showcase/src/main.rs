//! Windowed Kinetik UI showcase entry point.

mod live;
mod offscreen;

use std::fmt;

use kinetik_ui::core::{PhysicalSize, ScaleFactor, Size};
use kinetik_ui_showcase::{
    app::{ShowcaseApp, ShowcasePage},
    artifacts::{ReviewDumpRequest, dump_review_artifacts},
    raster::write_bmp,
};

use crate::offscreen::RenderOnceVelloError;

const DEFAULT_WIDTH: usize = 1440;
const DEFAULT_HEIGHT: usize = 900;

#[derive(Debug, Clone, Copy, PartialEq)]
struct RenderOnceTarget {
    physical_width: usize,
    physical_height: usize,
    logical_size: Size,
    scale_factor: f64,
}

#[derive(Clone, PartialEq, Eq)]
enum PageArgError {
    MissingValue,
    UnknownValue(String),
}

impl fmt::Debug for PageArgError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, formatter)
    }
}

impl fmt::Display for PageArgError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let expected = ShowcasePage::ALL
            .iter()
            .map(|page| page.slug())
            .collect::<Vec<_>>()
            .join(", ");
        match self {
            Self::MissingValue => write!(
                formatter,
                "--page requires a page value; expected one of: {expected}"
            ),
            Self::UnknownValue(value) => write!(
                formatter,
                "unknown --page value '{value}'; expected one of: {expected}"
            ),
        }
    }
}

impl std::error::Error for PageArgError {}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args().collect::<Vec<_>>();
    if args.iter().any(|arg| arg == "--list") {
        print!("{}", showcase_page_list());
        return Ok(());
    }
    let selected_page = page_arg(&args)?;

    if let Some(label) = dump_review_artifacts_label(&args) {
        let target = render_once_target(&args)?;
        let mut request =
            ReviewDumpRequest::new(label, target.physical_width, target.physical_height)
                .with_logical_size(target.logical_size);
        if let Some(page) = selected_page {
            request = request.with_page(page);
        }

        let dump = dump_review_artifacts(&request)?;
        println!("review artifact dump: {}", dump.directory.display());
        println!("manifest: {}", dump.manifest_path.display());
        for frame in dump.frames {
            println!(
                "{}: {} primitives, {} warnings, {}",
                frame.page_name,
                frame.primitive_count,
                frame.warning_count,
                frame.bmp_path.display()
            );
        }
        return Ok(());
    }

    if let Some(path) = render_once_path(&args) {
        let target = render_once_target(&args)?;
        let mut app = ShowcaseApp::new();
        app.set_viewport_size(target.logical_size);
        if let Some(page) = selected_page {
            app.set_page(page);
        }

        let frame = pollster::block_on(offscreen::render_once_vello_frame(
            &app,
            target.physical_width,
            target.physical_height,
            target.scale_factor,
        ))?;
        write_bmp(&frame, path)?;
        return Ok(());
    }

    live::run(selected_page)?;
    Ok(())
}

fn showcase_page_list() -> String {
    let mut output = ShowcasePage::ALL
        .iter()
        .map(|page| page.slug())
        .collect::<Vec<_>>()
        .join("\n");
    output.push('\n');
    output
}

fn render_once_path(args: &[String]) -> Option<&str> {
    args.windows(2)
        .find_map(|window| (window[0] == "--render-once").then_some(window[1].as_str()))
}

fn dump_review_artifacts_label(args: &[String]) -> Option<&str> {
    args.windows(2)
        .find_map(|window| (window[0] == "--dump-review-artifacts").then_some(window[1].as_str()))
}

fn page_arg(args: &[String]) -> Result<Option<ShowcasePage>, PageArgError> {
    let Some(index) = args.iter().position(|arg| arg == "--page") else {
        return Ok(None);
    };
    let Some(value) = args.get(index + 1).filter(|value| !value.starts_with('-')) else {
        return Err(PageArgError::MissingValue);
    };
    ShowcasePage::parse(value)
        .map(Some)
        .ok_or_else(|| PageArgError::UnknownValue(value.clone()))
}

fn usize_arg(args: &[String], name: &str) -> Option<usize> {
    args.windows(2)
        .find_map(|window| (window[0] == name).then(|| window[1].parse().ok()))
        .flatten()
}

fn f64_arg(args: &[String], name: &str) -> Option<f64> {
    args.windows(2)
        .find_map(|window| (window[0] == name).then(|| window[1].parse().ok()))
        .flatten()
}

fn render_once_target(args: &[String]) -> Result<RenderOnceTarget, RenderOnceVelloError> {
    let scale_factor = f64_arg(args, "--scale").unwrap_or(1.0);
    let scale = ScaleFactor::new(scale_factor);
    if !scale.is_valid() {
        return Err(RenderOnceVelloError::InvalidScaleFactor);
    }

    if usize_arg(args, "--logical-width").is_some() || usize_arg(args, "--logical-height").is_some()
    {
        let logical_size = Size::new(
            pixel_to_f32(usize_arg(args, "--logical-width").unwrap_or(DEFAULT_WIDTH)),
            pixel_to_f32(usize_arg(args, "--logical-height").unwrap_or(DEFAULT_HEIGHT)),
        );
        let physical_size = scale.logical_size_to_physical(logical_size);
        return Ok(RenderOnceTarget {
            physical_width: usize::try_from(physical_size.width).unwrap_or(usize::MAX),
            physical_height: usize::try_from(physical_size.height).unwrap_or(usize::MAX),
            logical_size,
            scale_factor,
        });
    }

    let physical_width = usize_arg(args, "--width").unwrap_or(DEFAULT_WIDTH);
    let physical_height = usize_arg(args, "--height").unwrap_or(DEFAULT_HEIGHT);
    Ok(RenderOnceTarget {
        physical_width,
        physical_height,
        logical_size: scale.physical_size_to_logical(PhysicalSize::new(
            pixel_to_u32(physical_width),
            pixel_to_u32(physical_height),
        )),
        scale_factor,
    })
}

fn pixel_to_f32(value: usize) -> f32 {
    let value = u16::try_from(value).unwrap_or(u16::MAX);
    f32::from(value)
}

fn pixel_to_u32(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use super::{
        PageArgError, ShowcasePage, Size, dump_review_artifacts_label, f64_arg, page_arg,
        render_once_target, showcase_page_list, usize_arg,
    };
    use kinetik_ui_showcase::app::ShowcaseApp;

    #[test]
    fn showcase_cli_list_matches_canonical_page_catalogue() {
        assert_eq!(
            showcase_page_list(),
            "editor\ncomponents\nlayout\nviewport\nsystems\n"
        );
    }

    #[test]
    fn showcase_cli_page_parser_accepts_every_canonical_slug() {
        for page in ShowcasePage::ALL {
            let args = [
                "showcase".to_owned(),
                "--page".to_owned(),
                page.slug().to_owned(),
            ];
            assert_eq!(page_arg(&args), Ok(Some(page)));
        }
    }

    #[test]
    fn showcase_cli_page_parser_rejects_missing_and_unknown_values() {
        let missing = ["showcase".to_owned(), "--page".to_owned()];
        let followed_by_flag = [
            "showcase".to_owned(),
            "--page".to_owned(),
            "--render-once".to_owned(),
            "frame.bmp".to_owned(),
        ];
        let unknown = [
            "showcase".to_owned(),
            "--page".to_owned(),
            "dashboard".to_owned(),
        ];

        assert_eq!(page_arg(&missing), Err(PageArgError::MissingValue));
        assert_eq!(page_arg(&followed_by_flag), Err(PageArgError::MissingValue));
        assert_eq!(
            page_arg(&unknown),
            Err(PageArgError::UnknownValue("dashboard".to_owned()))
        );
        assert_eq!(
            page_arg(&missing).unwrap_err().to_string(),
            "--page requires a page value; expected one of: editor, components, layout, viewport, systems"
        );
        assert_eq!(
            page_arg(&unknown).unwrap_err().to_string(),
            "unknown --page value 'dashboard'; expected one of: editor, components, layout, viewport, systems"
        );
        assert_eq!(
            format!("{:?}", page_arg(&missing).unwrap_err()),
            "--page requires a page value; expected one of: editor, components, layout, viewport, systems"
        );
    }

    #[test]
    fn showcase_cli_page_parser_preserves_default_editor_when_absent() {
        let args = ["showcase".to_owned()];
        let app = ShowcaseApp::new();

        assert_eq!(page_arg(&args), Ok(None));
        assert_eq!(app.page(), ShowcasePage::Editor);
    }

    #[test]
    fn render_once_cli_parses_physical_scale_and_dimensions() {
        let args = [
            "showcase".to_owned(),
            "--render-once".to_owned(),
            "frame.bmp".to_owned(),
            "--width".to_owned(),
            "1440".to_owned(),
            "--height".to_owned(),
            "900".to_owned(),
            "--scale".to_owned(),
            "1.25".to_owned(),
        ];

        assert_eq!(usize_arg(&args, "--width"), Some(1440));
        assert_eq!(usize_arg(&args, "--height"), Some(900));
        assert_eq!(f64_arg(&args, "--scale"), Some(1.25));
    }

    #[test]
    fn dump_review_artifacts_cli_parses_label_without_render_once() {
        let args = [
            "showcase".to_owned(),
            "--dump-review-artifacts".to_owned(),
            "s8-12c".to_owned(),
            "--page".to_owned(),
            "components".to_owned(),
            "--width".to_owned(),
            "320".to_owned(),
            "--height".to_owned(),
            "200".to_owned(),
        ];

        assert_eq!(dump_review_artifacts_label(&args), Some("s8-12c"));
        let target = render_once_target(&args).expect("dump target");

        assert_eq!(target.physical_width, 320);
        assert_eq!(target.physical_height, 200);
        assert_eq!(target.logical_size, Size::new(320.0, 200.0));
    }

    #[test]
    fn render_once_target_defaults_to_physical_dimensions() {
        let args = [
            "showcase".to_owned(),
            "--render-once".to_owned(),
            "frame.bmp".to_owned(),
            "--width".to_owned(),
            "1440".to_owned(),
            "--height".to_owned(),
            "900".to_owned(),
            "--scale".to_owned(),
            "1.25".to_owned(),
        ];

        let target = render_once_target(&args).expect("render-once target");

        assert_eq!(target.physical_width, 1440);
        assert_eq!(target.physical_height, 900);
        assert_eq!(target.logical_size, Size::new(1152.0, 720.0));
        assert_approx_f64(target.scale_factor, 1.25);
    }

    #[test]
    fn render_once_target_accepts_live_logical_dimensions() {
        let args = [
            "showcase".to_owned(),
            "--render-once".to_owned(),
            "frame.bmp".to_owned(),
            "--logical-width".to_owned(),
            "1440".to_owned(),
            "--logical-height".to_owned(),
            "900".to_owned(),
            "--scale".to_owned(),
            "1.25".to_owned(),
        ];

        let target = render_once_target(&args).expect("render-once target");

        assert_eq!(target.physical_width, 1800);
        assert_eq!(target.physical_height, 1125);
        assert_eq!(target.logical_size, Size::new(1440.0, 900.0));
        assert_approx_f64(target.scale_factor, 1.25);
    }

    #[test]
    fn render_once_rejects_invalid_scale_factor() {
        let args = [
            "showcase".to_owned(),
            "--render-once".to_owned(),
            "frame.bmp".to_owned(),
            "--scale".to_owned(),
            "0".to_owned(),
        ];

        assert!(render_once_target(&args).is_err());
    }

    fn assert_approx_f64(actual: f64, expected: f64) {
        assert!((actual - expected).abs() <= f64::EPSILON);
    }
}
