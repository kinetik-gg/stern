//! Captures the deterministic Stern DPI calibration scene through Vello and a real GPU.

use std::{fs::File, path::PathBuf, sync::mpsc};

use stern_core::{
    Brush, Color, CornerRadius, LinePrimitive, PhysicalSize, Point, Primitive, Rect, RectPrimitive,
    ScaleFactor, Size, Stroke, ViewportInfo, default_dark_theme,
};
use stern_vello::{RenderFrameInput, RenderResources, VelloRenderer};
use vello::wgpu;
use vello::{AaConfig, AaSupport, RenderParams, Renderer, RendererOptions, peniko};

const SCALES: [(f64, &str, u32, u32); 4] = [
    (1.0, "1.00x", 640, 360),
    (1.25, "1.25x", 800, 450),
    (1.5, "1.50x", 960, 540),
    (2.0, "2.00x", 1280, 720),
];

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = output_dir()?;
    std::fs::create_dir_all(&output)?;

    let mut context = vello::util::RenderContext::new();
    let device_id = pollster::block_on(context.device(None)).ok_or("no compatible GPU")?;
    let handle = &context.devices[device_id];
    let info = handle.adapter().get_info();
    if info.backend != wgpu::Backend::Dx12 {
        return Err(format!("expected Dx12 adapter, observed {:?}", info.backend).into());
    }
    let mut gpu_renderer = Renderer::new(
        &handle.device,
        RendererOptions {
            use_cpu: false,
            antialiasing_support: AaSupport::area_only(),
            ..RendererOptions::default()
        },
    )?;

    let mut captures = Vec::new();
    for (scale, label, width, height) in SCALES {
        let (pixels, padded_row_bytes) = render(
            &handle.device,
            &handle.queue,
            &mut gpu_renderer,
            scale,
            width,
            height,
        )?;
        let path = output.join(format!("{label}.png"));
        write_png(&path, width, height, &pixels)?;
        captures.push(format!(
            "{{\"scale\":{scale},\"label\":\"{label}\",\"width\":{width},\"height\":{height},\"unpadded_row_bytes\":{},\"padded_row_bytes\":{padded_row_bytes}}}",
            width * 4
        ));
    }

    println!(
        "STERN_VELLO_METADATA={{\"wgpu_version\":\"29.0.3\",\"vello_version\":\"0.9.0\",\"backend\":\"{:?}\",\"adapter\":\"{}\",\"vendor\":{},\"device\":{},\"driver\":\"{}\",\"driver_info\":\"{}\",\"device_type\":\"{:?}\",\"texture_format\":\"Rgba8Unorm\",\"aa\":\"Area\",\"captures\":[{}]}}",
        info.backend,
        escape_json(&info.name),
        info.vendor,
        info.device,
        escape_json(&info.driver),
        escape_json(&info.driver_info),
        info.device_type,
        captures.join(",")
    );
    Ok(())
}

fn output_dir() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let mut args = std::env::args_os().skip(1);
    if args.next().as_deref() != Some(std::ffi::OsStr::new("--output")) {
        return Err("usage: capture_dpi_evidence --output <directory>".into());
    }
    let output = args.next().ok_or("missing output directory")?;
    if args.next().is_some() {
        return Err("unexpected argument".into());
    }
    Ok(output.into())
}

fn render(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    gpu_renderer: &mut Renderer,
    scale: f64,
    width: u32,
    height: u32,
) -> Result<(Vec<u8>, u32), Box<dyn std::error::Error>> {
    let primitives = scene_primitives();
    let resources = RenderResources::new();
    let mut stern_renderer = VelloRenderer::new();
    let output = stern_renderer.submit_frame(RenderFrameInput {
        viewport: ViewportInfo::new(
            Size::new(640.0, 360.0),
            PhysicalSize::new(width, height),
            ScaleFactor::new(scale),
        ),
        primitives: &primitives,
        resources: &resources,
    });
    if !output.diagnostics.is_empty() {
        return Err(format!("renderer diagnostics: {:?}", output.diagnostics).into());
    }

    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("stern-dpi-009 evidence target"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    gpu_renderer.render_to_texture(
        device,
        queue,
        stern_renderer.scene(),
        &view,
        &RenderParams {
            base_color: peniko::Color::from_rgb8(0x11, 0x11, 0x11),
            width,
            height,
            antialiasing_method: AaConfig::Area,
        },
    )?;

    let tight_row_bytes = width.checked_mul(4).ok_or("row byte overflow")?;
    let padded_row_bytes = align_up(tight_row_bytes, 256);
    let buffer_size = u64::from(padded_row_bytes)
        .checked_mul(u64::from(height))
        .ok_or("buffer size overflow")?;
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("stern-dpi-009 evidence readback"),
        size: buffer_size,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("stern-dpi-009 evidence copy"),
    });
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded_row_bytes),
                rows_per_image: Some(height),
            },
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    queue.submit([encoder.finish()]);

    let slice = buffer.slice(..);
    let (sender, receiver) = mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |result| {
        let _ = sender.send(result);
    });
    device.poll(wgpu::PollType::wait_indefinitely())?;
    receiver.recv()??;
    let mapped = slice.get_mapped_range();
    let tight_len = usize::try_from(tight_row_bytes)?;
    let padded_len = usize::try_from(padded_row_bytes)?;
    let mut pixels = Vec::with_capacity(tight_len * usize::try_from(height)?);
    for row in mapped
        .chunks_exact(padded_len)
        .take(usize::try_from(height)?)
    {
        pixels.extend_from_slice(&row[..tight_len]);
    }
    drop(mapped);
    buffer.unmap();
    Ok((pixels, padded_row_bytes))
}

fn scene_primitives() -> Vec<Primitive> {
    let theme = default_dark_theme();
    let colors = theme.colors;
    vec![
        filled(
            Rect::new(0.0, 0.0, 640.0, 360.0),
            colors.surface.application,
        ),
        rectangle(
            Rect::new(40.0, 32.0, 560.0, 296.0),
            colors.surface.panel_raised,
            colors.border.default,
            theme.strokes.default,
        ),
        rectangle(
            Rect::new(72.0, 76.0, 96.0, 64.0),
            colors.surface.control,
            colors.border.strong,
            theme.strokes.default,
        ),
        rectangle(
            Rect::new(168.0, 76.0, 96.0, 64.0),
            colors.accent.default,
            colors.border.strong,
            theme.strokes.default,
        ),
        rectangle(
            Rect::new(296.0, 76.0, 272.0, 64.0),
            colors.surface.sunken,
            colors.border.subtle,
            theme.strokes.default,
        ),
        line(
            72.0,
            180.0,
            568.0,
            180.0,
            colors.border.default,
            theme.strokes.hairline,
        ),
        line(
            72.0,
            220.0,
            568.0,
            220.0,
            colors.accent.focus,
            theme.strokes.emphasis,
        ),
        filled(
            Rect::new(72.0, 260.0, 160.0, 36.0),
            colors.surface.control_hover,
        ),
        filled(
            Rect::new(232.0, 260.0, 168.0, 36.0),
            colors.surface.control_pressed,
        ),
        filled(Rect::new(400.0, 260.0, 168.0, 36.0), colors.accent.subtle),
    ]
}

fn filled(rect: Rect, color: Color) -> Primitive {
    Primitive::Rect(RectPrimitive {
        rect,
        fill: Some(Brush::Solid(color)),
        stroke: None,
        radius: CornerRadius::all(0.0),
    })
}

fn rectangle(rect: Rect, fill: Color, border: Color, width: f32) -> Primitive {
    Primitive::Rect(RectPrimitive {
        rect,
        fill: Some(Brush::Solid(fill)),
        stroke: Some(Stroke::new(width, Brush::Solid(border))),
        radius: CornerRadius::all(0.0),
    })
}

fn line(x1: f32, y1: f32, x2: f32, y2: f32, color: Color, width: f32) -> Primitive {
    Primitive::Line(LinePrimitive {
        from: Point::new(x1, y1),
        to: Point::new(x2, y2),
        stroke: Stroke::new(width, Brush::Solid(color)),
    })
}

fn write_png(
    path: &std::path::Path,
    width: u32,
    height: u32,
    pixels: &[u8],
) -> Result<(), Box<dyn std::error::Error>> {
    let mut encoder = png::Encoder::new(File::create(path)?, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header()?.write_image_data(pixels)?;
    Ok(())
}

const fn align_up(value: u32, alignment: u32) -> u32 {
    value.div_ceil(alignment) * alignment
}

fn escape_json(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}
