//! Windowed Kinetik UI showcase entry point.

mod live;

use std::fmt;
use std::sync::mpsc;

use kinetik_ui::{
    core::{PhysicalSize, ScaleFactor, Size, ViewportInfo},
    render::{RenderDiagnostic, RenderFrameInput},
    render_vello::VelloRenderer,
};
use kinetik_ui_showcase::{
    app::{ShowcaseApp, ShowcasePage},
    artifacts::{ReviewDumpRequest, dump_review_artifacts},
    raster::{Pixel, RasterFrame, write_bmp},
};
use vello::{
    AaConfig, RenderParams, Renderer, RendererOptions,
    peniko::Color as VelloColor,
    util::RenderContext,
    wgpu::{
        BufferDescriptor, BufferUsages, COPY_BYTES_PER_ROW_ALIGNMENT, CommandEncoderDescriptor,
        Device, Extent3d, MapMode, Origin3d, PollType, Queue, TexelCopyBufferInfo,
        TexelCopyBufferLayout, TexelCopyTextureInfo, Texture, TextureAspect, TextureDescriptor,
        TextureDimension, TextureFormat, TextureUsages, TextureView, TextureViewDescriptor,
    },
};

const DEFAULT_WIDTH: usize = 1440;
const DEFAULT_HEIGHT: usize = 900;

pub(crate) fn showcase_antialiasing_method() -> AaConfig {
    AaConfig::Msaa16
}

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

        let frame = pollster::block_on(render_once_vello_frame(
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

fn submit_render_once_to_vello(
    app: &ShowcaseApp,
    width: usize,
    height: usize,
    scale_factor: f64,
) -> Result<(VelloRenderer, ViewportInfo), RenderOnceVelloError> {
    let viewport = render_once_viewport(app, width, height, scale_factor)?;
    let resources = app.render_resources();
    let mut renderer = VelloRenderer::new();
    let output = renderer.submit_frame(RenderFrameInput {
        viewport,
        primitives: &app.output().primitives,
        resources,
    });

    if output.diagnostics.is_empty() {
        Ok((renderer, viewport))
    } else {
        Err(RenderOnceVelloError::Diagnostics(output.diagnostics))
    }
}

async fn render_once_vello_frame(
    app: &ShowcaseApp,
    width: usize,
    height: usize,
    scale_factor: f64,
) -> Result<RasterFrame, RenderOnceVelloError> {
    let (toolkit, viewport) = submit_render_once_to_vello(app, width, height, scale_factor)?;
    let width = viewport.physical_size.width;
    let height = viewport.physical_size.height;
    let mut context = RenderContext::new();
    let device_id = context
        .device(None)
        .await
        .ok_or(RenderOnceVelloError::NoCompatibleDevice)?;
    let device_handle = &context.devices[device_id];
    let (texture, texture_view) = create_render_once_texture(&device_handle.device, width, height);
    render_scene_to_texture(
        &device_handle.device,
        &device_handle.queue,
        toolkit.scene(),
        &texture_view,
        width,
        height,
    )?;
    read_texture_to_frame(
        &device_handle.device,
        &device_handle.queue,
        &texture,
        width,
        height,
    )
}

fn create_render_once_texture(device: &Device, width: u32, height: u32) -> (Texture, TextureView) {
    let texture = device.create_texture(&TextureDescriptor {
        label: Some("kinetik-ui-showcase-render-once"),
        size: Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TextureFormat::Rgba8Unorm,
        usage: TextureUsages::COPY_SRC
            | TextureUsages::STORAGE_BINDING
            | TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let texture_view = texture.create_view(&TextureViewDescriptor::default());
    (texture, texture_view)
}

fn render_scene_to_texture(
    device: &Device,
    queue: &Queue,
    scene: &vello::Scene,
    texture_view: &TextureView,
    width: u32,
    height: u32,
) -> Result<(), RenderOnceVelloError> {
    let mut renderer = Renderer::new(device, RendererOptions::default())?;
    renderer.render_to_texture(
        device,
        queue,
        scene,
        texture_view,
        &RenderParams {
            base_color: VelloColor::from_rgb8(11, 12, 13),
            width,
            height,
            antialiasing_method: showcase_antialiasing_method(),
        },
    )?;
    Ok(())
}

fn read_texture_to_frame(
    device: &Device,
    queue: &Queue,
    texture: &Texture,
    width: u32,
    height: u32,
) -> Result<RasterFrame, RenderOnceVelloError> {
    let bytes_per_pixel = 4_u32;
    let unpadded_bytes_per_row = width.saturating_mul(bytes_per_pixel);
    let padded_bytes_per_row = align_to(unpadded_bytes_per_row, COPY_BYTES_PER_ROW_ALIGNMENT);
    let buffer_size = u64::from(padded_bytes_per_row).saturating_mul(u64::from(height));
    let buffer = device.create_buffer(&BufferDescriptor {
        label: Some("kinetik-ui-showcase-render-once-readback"),
        size: buffer_size,
        usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
        label: Some("kinetik-ui-showcase-render-once-copy"),
    });
    encoder.copy_texture_to_buffer(
        TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: Origin3d::ZERO,
            aspect: TextureAspect::All,
        },
        TexelCopyBufferInfo {
            buffer: &buffer,
            layout: TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded_bytes_per_row),
                rows_per_image: Some(height),
            },
        },
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    queue.submit([encoder.finish()]);

    let buffer_slice = buffer.slice(..);
    let (sender, receiver) = mpsc::channel();
    buffer_slice.map_async(MapMode::Read, move |result| {
        let _ = sender.send(result);
    });
    device
        .poll(PollType::wait_indefinitely())
        .map_err(|error| RenderOnceVelloError::Readback(error.to_string()))?;
    receiver
        .recv()
        .map_err(|error| RenderOnceVelloError::Readback(error.to_string()))?
        .map_err(|error| RenderOnceVelloError::Readback(error.to_string()))?;

    let mapped = buffer_slice.get_mapped_range();
    let width = usize::try_from(width).unwrap_or(usize::MAX);
    let height = usize::try_from(height).unwrap_or(usize::MAX);
    let padded_bytes_per_row = usize::try_from(padded_bytes_per_row).unwrap_or(usize::MAX);
    let unpadded_bytes_per_row = usize::try_from(unpadded_bytes_per_row).unwrap_or(usize::MAX);
    let pixels = read_rgb_pixels(
        &mapped,
        width,
        height,
        padded_bytes_per_row,
        unpadded_bytes_per_row,
    );
    drop(mapped);
    buffer.unmap();

    Ok(RasterFrame {
        width,
        height,
        pixels,
    })
}

fn read_rgb_pixels(
    mapped: &[u8],
    width: usize,
    height: usize,
    padded_bytes_per_row: usize,
    unpadded_bytes_per_row: usize,
) -> Vec<Pixel> {
    let mut pixels = Vec::<Pixel>::with_capacity(width.saturating_mul(height));
    for row in mapped.chunks(padded_bytes_per_row).take(height) {
        for rgba in row[..unpadded_bytes_per_row].chunks_exact(4) {
            pixels
                .push((u32::from(rgba[0]) << 16) | (u32::from(rgba[1]) << 8) | u32::from(rgba[2]));
        }
    }
    pixels
}

fn render_once_viewport(
    app: &ShowcaseApp,
    width: usize,
    height: usize,
    scale_factor: f64,
) -> Result<ViewportInfo, RenderOnceVelloError> {
    let scale_factor = ScaleFactor::new(scale_factor);
    if !scale_factor.is_valid() {
        return Err(RenderOnceVelloError::InvalidScaleFactor);
    }

    Ok(ViewportInfo::new(
        app.viewport_size(),
        PhysicalSize::new(pixel_to_u32(width), pixel_to_u32(height)),
        scale_factor,
    ))
}

#[cfg(test)]
fn logical_size_from_pixels(width: usize, height: usize, scale_factor: f64) -> Size {
    let scale_factor = ScaleFactor::new(scale_factor);
    if scale_factor.is_valid() {
        scale_factor
            .physical_size_to_logical(PhysicalSize::new(pixel_to_u32(width), pixel_to_u32(height)))
    } else {
        Size::new(pixel_to_f32(width), pixel_to_f32(height))
    }
}

fn pixel_to_f32(value: usize) -> f32 {
    let value = u16::try_from(value).unwrap_or(u16::MAX);
    f32::from(value)
}

fn pixel_to_u32(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

fn align_to(value: u32, alignment: u32) -> u32 {
    value.div_ceil(alignment) * alignment
}

#[derive(Debug)]
enum RenderOnceVelloError {
    InvalidScaleFactor,
    NoCompatibleDevice,
    Diagnostics(Vec<RenderDiagnostic>),
    Render(vello::Error),
    Readback(String),
}

impl fmt::Display for RenderOnceVelloError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidScaleFactor => write!(formatter, "invalid render-once scale factor"),
            Self::NoCompatibleDevice => write!(formatter, "no compatible Vello render device"),
            Self::Diagnostics(diagnostics) => {
                write!(formatter, "render-once Vello diagnostics: {diagnostics:?}")
            }
            Self::Render(error) => write!(formatter, "render-once Vello render failed: {error}"),
            Self::Readback(error) => write!(formatter, "render-once readback failed: {error}"),
        }
    }
}

impl std::error::Error for RenderOnceVelloError {}

impl From<vello::Error> for RenderOnceVelloError {
    fn from(error: vello::Error) -> Self {
        Self::Render(error)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        PageArgError, ShowcasePage, Size, align_to, dump_review_artifacts_label, f64_arg,
        logical_size_from_pixels, page_arg, render_once_target, render_once_viewport,
        showcase_antialiasing_method, showcase_page_list, submit_render_once_to_vello, usize_arg,
    };
    use kinetik_ui::core::Primitive;
    use kinetik_ui_showcase::app::ShowcaseApp;
    use vello::AaConfig;

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
    fn render_once_viewport_uses_scaled_logical_size() {
        let mut app = ShowcaseApp::new();
        app.set_viewport_size(logical_size_from_pixels(1440, 900, 1.25));

        let viewport = render_once_viewport(&app, 1440, 900, 1.25).expect("viewport");

        assert_eq!(viewport.physical_size.width, 1440);
        assert_eq!(viewport.physical_size.height, 900);
        assert_eq!(viewport.logical_size, app.viewport_size());
        assert!((viewport.scale_factor.value() - 1.25).abs() < f64::EPSILON);
    }

    #[test]
    fn render_once_rejects_invalid_scale_factor() {
        let app = ShowcaseApp::new();
        let args = [
            "showcase".to_owned(),
            "--render-once".to_owned(),
            "frame.bmp".to_owned(),
            "--scale".to_owned(),
            "0".to_owned(),
        ];

        assert!(render_once_target(&args).is_err());
        assert!(render_once_viewport(&app, 1440, 900, 0.0).is_err());
        assert!(render_once_viewport(&app, 1440, 900, f64::NAN).is_err());
    }

    #[test]
    fn render_once_prefers_crisp_showcase_antialiasing() {
        assert_eq!(showcase_antialiasing_method(), AaConfig::Msaa16);
    }

    #[test]
    fn render_once_submits_fractional_dpi_text_through_vello() {
        let mut app = ShowcaseApp::new();
        app.set_viewport_size(logical_size_from_pixels(1440, 900, 1.25));

        let (renderer, _) =
            submit_render_once_to_vello(&app, 1440, 900, 1.25).expect("vello submission");
        let encoding = renderer.scene().encoding();
        let glyph_runs = &encoding.resources.glyph_runs;
        let glyphs = &encoding.resources.glyphs;

        assert!(!glyph_runs.is_empty());
        assert!(!glyphs.is_empty());
        assert!(
            glyph_runs.iter().all(|run| run.hint),
            "render-once should use hinted physical text for axis-aligned showcase glyphs"
        );
        assert!(
            glyphs
                .iter()
                .all(|glyph| (glyph.x - glyph.x.round()).abs() <= 0.001),
            "render-once should snap physical glyph x positions"
        );
        assert!(
            glyphs
                .iter()
                .all(|glyph| (glyph.y - glyph.y.round()).abs() <= 0.001),
            "render-once should snap physical glyph baselines"
        );
    }

    #[test]
    fn live_logical_render_once_submits_matching_physical_text_size() {
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
        let mut app = ShowcaseApp::new();
        app.set_viewport_size(target.logical_size);
        let first_text = app
            .output()
            .primitives
            .iter()
            .find_map(|primitive| match primitive {
                Primitive::Text(text) => Some(text),
                _ => None,
            })
            .expect("showcase text primitive");
        let layout = first_text.layout.expect("registered showcase text layout");
        let registered_run = app
            .render_resources()
            .text_layout_resource(layout)
            .and_then(|resource| resource.layout.runs.first())
            .expect("registered showcase glyph run");

        let (renderer, viewport) = submit_render_once_to_vello(
            &app,
            target.physical_width,
            target.physical_height,
            target.scale_factor,
        )
        .expect("vello submission");
        let encoding = renderer.scene().encoding();
        let encoded_run = encoding.resources.glyph_runs.first().expect("glyph run");
        let encoded_glyphs = &encoding.resources.glyphs[encoded_run.glyphs.clone()];

        assert_eq!(viewport.logical_size, Size::new(1440.0, 900.0));
        assert_eq!(viewport.physical_size.width, 1800);
        assert_eq!(viewport.physical_size.height, 1125);
        let physical_width =
            u16::try_from(viewport.physical_size.width).expect("physical width fits u16");
        let physical_height =
            u16::try_from(viewport.physical_size.height).expect("physical height fits u16");
        let scale_x = f32::from(physical_width) / viewport.logical_size.width;
        let scale_y = f32::from(physical_height) / viewport.logical_size.height;
        assert_eq!(scale_x.to_bits(), scale_y.to_bits());

        assert!(encoded_run.hint);
        assert_eq!(encoded_run.glyphs.start, 0);
        assert_eq!(encoded_run.glyphs.end, registered_run.glyphs.len());
        assert_eq!(
            encoded_run.font.data.as_ref(),
            registered_run.font.data.data()
        );
        assert_eq!(encoded_run.font.index, registered_run.font.index);
        assert_eq!(encoded_glyphs.len(), registered_run.glyphs.len());
        assert!(
            encoded_glyphs
                .iter()
                .zip(&registered_run.glyphs)
                .all(|(encoded, registered)| encoded.id == registered.id)
        );
        assert_eq!(
            encoded_run.font_size.to_bits(),
            (registered_run.font_size * scale_x).to_bits()
        );
    }

    #[test]
    fn render_once_readback_rows_align_to_wgpu_copy_pitch() {
        assert_eq!(align_to(0, 256), 0);
        assert_eq!(align_to(4, 256), 256);
        assert_eq!(align_to(1024, 256), 1024);
        assert_eq!(align_to(1025, 256), 1280);
    }

    fn assert_approx_f64(actual: f64, expected: f64) {
        assert!((actual - expected).abs() <= f64::EPSILON);
    }
}
