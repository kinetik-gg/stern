//! Captures the real public `DemoApp` Edit and Graph workspaces through Vello.

use std::{path::PathBuf, sync::mpsc};

use stern::core::{
    FrameContext, PhysicalSize, Point, PointerButtonState, PointerInput, ScaleFactor, SemanticRole,
    Size, TimeInfo, UiInput, ViewportInfo,
};
use stern::render::RenderFrameInput;
use stern::render_vello::VelloRenderer;
use stern::vello_winit::{
    AaConfig, AaSupport, RenderContext, RenderParams, Renderer, RendererOptions, Scene, peniko,
    wgpu,
};
use stern_demo::{DemoApp, DemoWorkspace};

const LOGICAL_WIDTH: f32 = 960.0;
const LOGICAL_HEIGHT: f32 = 640.0;
const SCALES: [(f64, &str, u32, u32); 4] = [
    (1.0, "1.00x", 960, 640),
    (1.25, "1.25x", 1_200, 800),
    (1.5, "1.50x", 1_440, 960),
    (2.0, "2.00x", 1_920, 1_280),
];

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = output_dir()?;
    std::fs::create_dir_all(&output)?;

    let mut context = RenderContext::new();
    let device_id = pollster::block_on(context.device(None)).ok_or("no compatible GPU")?;
    let handle = &context.devices[device_id];
    let adapter = handle.adapter().get_info();
    if adapter.backend != wgpu::Backend::Dx12 {
        return Err(format!("expected Dx12 adapter, observed {:?}", adapter.backend).into());
    }
    let mut gpu_renderer = Renderer::new(
        &handle.device,
        RendererOptions {
            use_cpu: false,
            antialiasing_support: AaSupport::area_only(),
            ..RendererOptions::default()
        },
    )?;

    for (workspace, workspace_name) in [
        (DemoWorkspace::Edit, "edit"),
        (DemoWorkspace::Graph, "graph"),
    ] {
        for (scale, scale_label, width, height) in SCALES {
            let mut app = DemoApp::new();
            let output_frame = frame_for_workspace(&mut app, workspace, scale, width, height)?;
            let resources = app.render_resources();
            let mut toolkit_renderer = VelloRenderer::new();
            let encoded = toolkit_renderer.submit_frame(RenderFrameInput {
                viewport: viewport(scale, width, height),
                primitives: &output_frame.primitives,
                resources: &resources,
            });
            if !encoded.diagnostics.is_empty() {
                return Err(format!(
                    "renderer diagnostics for {workspace_name}/{scale_label}: {:?}",
                    encoded.diagnostics
                )
                .into());
            }
            let (pixels, padded_row_bytes) = readback_vello_scene(
                &handle.device,
                &handle.queue,
                &mut gpu_renderer,
                toolkit_renderer.scene(),
                width,
                height,
            )?;
            let directory = output.join(workspace_name);
            std::fs::create_dir_all(&directory)?;
            std::fs::write(directory.join(format!("{scale_label}.rgba")), &pixels)?;
            if padded_row_bytes < width * 4 {
                return Err("invalid GPU row alignment".into());
            }
        }
    }

    println!(
        "STERN_DEMO_VELLO_METADATA={{\"renderer\":\"Vello\",\"backend\":\"{:?}\",\"adapter\":\"{}\",\"vendor\":{},\"device\":{},\"driver\":\"{}\",\"driver_info\":\"{}\",\"device_type\":\"{:?}\",\"texture_format\":\"Rgba8Unorm\",\"antialiasing\":\"Area\"}}",
        adapter.backend,
        escape_json(&adapter.name),
        adapter.vendor,
        adapter.device,
        escape_json(&adapter.driver),
        escape_json(&adapter.driver_info),
        adapter.device_type,
    );
    Ok(())
}

fn output_dir() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let mut args = std::env::args_os().skip(1);
    if args.next().as_deref() != Some(std::ffi::OsStr::new("--output")) {
        return Err("usage: capture_vello_workspaces --output <directory>".into());
    }
    let output = args.next().ok_or("missing output directory")?;
    if args.next().is_some() {
        return Err("unexpected argument".into());
    }
    Ok(output.into())
}

fn frame_for_workspace(
    app: &mut DemoApp,
    workspace: DemoWorkspace,
    scale: f64,
    width: u32,
    height: u32,
) -> Result<stern::core::FrameOutput, Box<dyn std::error::Error>> {
    if workspace == DemoWorkspace::Graph {
        let initial = app.frame(frame_context(UiInput::default(), scale, width, height));
        let point = initial
            .semantics
            .nodes()
            .iter()
            .find(|node| {
                node.role == SemanticRole::IconButton
                    && node.label.as_deref() == Some("Graph Workspace")
            })
            .ok_or("Graph Workspace public semantic action is missing")?
            .bounds
            .center();
        let _ = app.frame(frame_context(
            pointer_input(point, true, true, false),
            scale,
            width,
            height,
        ));
        let _ = app.frame(frame_context(
            pointer_input(point, false, false, true),
            scale,
            width,
            height,
        ));
        if app.workspace() != DemoWorkspace::Graph {
            return Err("public Graph workspace action did not dispatch".into());
        }
    }
    Ok(app.frame(frame_context(UiInput::default(), scale, width, height)))
}

fn frame_context(input: UiInput, scale: f64, width: u32, height: u32) -> FrameContext {
    FrameContext::new(viewport(scale, width, height), input, TimeInfo::default())
}

fn viewport(scale: f64, width: u32, height: u32) -> ViewportInfo {
    ViewportInfo::new(
        Size::new(LOGICAL_WIDTH, LOGICAL_HEIGHT),
        PhysicalSize::new(width, height),
        ScaleFactor::new(scale),
    )
}

fn pointer_input(point: Point, down: bool, pressed: bool, released: bool) -> UiInput {
    UiInput {
        pointer: PointerInput {
            position: Some(point),
            primary: PointerButtonState::new(down, pressed, released),
            ..PointerInput::default()
        },
        ..UiInput::default()
    }
}

fn readback_vello_scene(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    renderer: &mut Renderer,
    scene: &Scene,
    width: u32,
    height: u32,
) -> Result<(Vec<u8>, u32), Box<dyn std::error::Error>> {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Stern demo Vello evidence target"),
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
    renderer.render_to_texture(
        device,
        queue,
        scene,
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
        label: Some("Stern demo Vello evidence readback"),
        size: buffer_size,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Stern demo Vello evidence copy"),
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

const fn align_up(value: u32, alignment: u32) -> u32 {
    value.div_ceil(alignment) * alignment
}

fn escape_json(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}
