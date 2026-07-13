use std::sync::mpsc;

use kinetik_ui_core::{
    PhysicalSize, Primitive, Rect, ScaleFactor, Size, TextureId, TexturePrimitive, ViewportInfo,
};
use kinetik_ui_render::{
    RenderFrameInput, RenderImage, RenderImageSampling, RenderResources, TextureResource,
};
use kinetik_ui_vello::{VelloNativeTextureRegistry, VelloNativeTextureScope};
use vello::{
    AaConfig, AaSupport, RenderParams, Renderer, RendererOptions,
    peniko::Color as VelloColor,
    util::RenderContext,
    wgpu::{
        Backend, BufferDescriptor, BufferUsages, CommandEncoderDescriptor, Extent3d, MapMode,
        Origin3d, PollType, TexelCopyBufferInfo, TexelCopyBufferLayout, TexelCopyTextureInfo,
        Texture, TextureAspect, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
        TextureViewDescriptor,
    },
};

use super::{GpuState, VelloWindowPresenter};
use crate::{
    VelloNativeTextureUpdateOutcome, VelloPresenterConfig, device::DeviceInbox, lifecycle::Extent,
};

const WIDTH: u32 = 8;
const HEIGHT: u32 = 8;
const BASE: [u8; 4] = [32, 64, 128, 255];

#[test]
#[ignore = "explicit real-GPU gate; run with WGPU_BACKEND=dx12"]
fn same_device_native_texture_pixels_cover_update_replace_lifetime_and_remove() {
    pollster::block_on(async {
        let (mut presenter, scope) = headless_presenter().await;
        let texture_id = TextureId::from_raw(58_200);
        let resource = TextureResource {
            id: texture_id,
            size: Size::new(1.0, 1.0),
            sampling: RenderImageSampling::Pixelated,
            snapshot: Some(
                RenderImage::rgba8(1, 1, vec![255, 0, 255, 255]).expect("valid one-pixel fallback"),
            ),
        };
        let mut resources = RenderResources::new();
        resources.register_texture(resource.clone());
        let primitives = [Primitive::Texture(TexturePrimitive {
            texture: texture_id,
            rect: Rect::new(2.0, 2.0, 4.0, 4.0),
            source_size: Size::new(1.0, 1.0),
        })];
        let target = create_target(&presenter);

        let source = create_source(&mut presenter, &scope, [224, 128, 64, 128]);
        let registration = presenter
            .register_native_texture(&scope, &resource, &source, 1)
            .expect("same-device native registration");
        assert_pixel_near(
            render_center(&mut presenter, &resources, &primitives, &target),
            [128, 96, 96, 255],
        );

        write_source(&mut presenter, &scope, &source, [32, 224, 96, 64]);
        assert_eq!(
            presenter
                .update_native_texture(&registration, 2)
                .expect("dirty update"),
            VelloNativeTextureUpdateOutcome::MarkedDirty
        );
        assert_pixel_near(
            render_center(&mut presenter, &resources, &primitives, &target),
            [32, 104, 120, 255],
        );

        let replacement = create_source(&mut presenter, &scope, [240, 16, 208, 192]);
        let registration = presenter
            .replace_native_texture(&registration, &resource, &replacement, 3)
            .expect("native replacement");
        drop(replacement);
        drop(source);
        assert_pixel_near(
            render_center(&mut presenter, &resources, &primitives, &target),
            [189, 28, 188, 255],
        );

        presenter
            .remove_native_texture(&registration)
            .expect("native removal");
        assert_pixel_near(
            render_center(&mut presenter, &resources, &primitives, &target),
            [255, 0, 255, 255],
        );
    });
}

async fn headless_presenter() -> (VelloWindowPresenter, crate::PresenterDeviceScope) {
    let mut context = RenderContext::new();
    let dev_id = context
        .device(None)
        .await
        .expect("REND-04B requires a compatible DX12 adapter");
    let device_handle = &context.devices[dev_id];
    assert_eq!(
        device_handle.adapter().get_info().backend,
        Backend::Dx12,
        "run this gate with WGPU_BACKEND=dx12"
    );
    let renderer = Renderer::new(
        &device_handle.device,
        RendererOptions {
            antialiasing_support: AaSupport::area_only(),
            ..RendererOptions::default()
        },
    )
    .expect("create real Vello renderer");
    let native_scope = VelloNativeTextureScope::new().expect("native texture scope");
    let native_registry = VelloNativeTextureRegistry::new(&native_scope);

    let mut presenter = VelloWindowPresenter::new(
        VelloPresenterConfig::new().with_antialiasing_method(AaConfig::Area),
    )
    .expect("headless presenter");
    let (scope, _test_sender) = presenter
        .install_test_device(Extent {
            width: WIDTH,
            height: HEIGHT,
        })
        .expect("install attached test lifecycle");
    let inbox = DeviceInbox::install(&device_handle.device, scope.clone());
    presenter.gpu = Some(GpuState {
        renderer,
        context,
        dev_id,
        native_registry,
        native_scope,
    });
    presenter.inbox = Some(inbox);
    (presenter, scope)
}

fn create_source(
    presenter: &mut VelloWindowPresenter,
    scope: &crate::PresenterDeviceScope,
    rgba: [u8; 4],
) -> Texture {
    presenter
        .with_device(scope, |presenter_device| {
            let texture = presenter_device
                .device()
                .create_texture(&source_descriptor());
            write_rgba(presenter_device.queue(), &texture, rgba);
            texture
        })
        .expect("create same-device producer texture")
}

fn write_source(
    presenter: &mut VelloWindowPresenter,
    scope: &crate::PresenterDeviceScope,
    texture: &Texture,
    rgba: [u8; 4],
) {
    presenter
        .with_device(scope, |presenter_device| {
            write_rgba(presenter_device.queue(), texture, rgba);
        })
        .expect("write same-device producer texture");
}

fn write_rgba(queue: &vello::wgpu::Queue, texture: &Texture, rgba: [u8; 4]) {
    queue.write_texture(
        TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: Origin3d::ZERO,
            aspect: TextureAspect::All,
        },
        &rgba,
        TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(4),
            rows_per_image: Some(1),
        },
        Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
    );
}

fn source_descriptor() -> TextureDescriptor<'static> {
    TextureDescriptor {
        label: Some("kinetik-ui-rend-04b-producer"),
        size: Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TextureFormat::Rgba8Unorm,
        usage: TextureUsages::COPY_SRC | TextureUsages::COPY_DST,
        view_formats: &[],
    }
}

fn create_target(presenter: &VelloWindowPresenter) -> Texture {
    let gpu = presenter.gpu.as_ref().expect("installed GPU");
    gpu.context.devices[gpu.dev_id]
        .device
        .create_texture(&TextureDescriptor {
            label: Some("kinetik-ui-rend-04b-target"),
            size: Extent3d {
                width: WIDTH,
                height: HEIGHT,
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
        })
}

fn render_center(
    presenter: &mut VelloWindowPresenter,
    resources: &RenderResources,
    primitives: &[Primitive],
    target: &Texture,
) -> [u8; 4] {
    let viewport = ViewportInfo::new(
        Size::new(8.0, 8.0),
        PhysicalSize::new(WIDTH, HEIGHT),
        ScaleFactor::ONE,
    );
    let output = {
        let gpu = presenter.gpu.as_ref().expect("installed GPU");
        presenter.toolkit.submit_frame_with_native_textures(
            RenderFrameInput {
                viewport,
                primitives,
                resources,
            },
            &gpu.native_registry,
            &gpu.native_scope,
        )
    };
    assert!(
        output.diagnostics.is_empty(),
        "unexpected render diagnostics: {:?}",
        output.diagnostics
    );

    let target_view = target.create_view(&TextureViewDescriptor::default());
    let scene = presenter.toolkit.scene();
    let gpu = presenter.gpu.as_mut().expect("installed GPU");
    let device_handle = &gpu.context.devices[gpu.dev_id];
    gpu.renderer
        .render_to_texture(
            &device_handle.device,
            &device_handle.queue,
            scene,
            &target_view,
            &RenderParams {
                base_color: VelloColor::from_rgb8(BASE[0], BASE[1], BASE[2]),
                width: WIDTH,
                height: HEIGHT,
                antialiasing_method: AaConfig::Area,
            },
        )
        .expect("render native texture on GPU");
    read_center(&device_handle.device, &device_handle.queue, target)
}

fn read_center(
    device: &vello::wgpu::Device,
    queue: &vello::wgpu::Queue,
    texture: &Texture,
) -> [u8; 4] {
    const PADDED_BYTES_PER_ROW: u32 = 256;
    let buffer = device.create_buffer(&BufferDescriptor {
        label: Some("kinetik-ui-rend-04b-readback"),
        size: u64::from(PADDED_BYTES_PER_ROW) * u64::from(HEIGHT),
        usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
        label: Some("kinetik-ui-rend-04b-copy"),
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
                bytes_per_row: Some(PADDED_BYTES_PER_ROW),
                rows_per_image: Some(HEIGHT),
            },
        },
        Extent3d {
            width: WIDTH,
            height: HEIGHT,
            depth_or_array_layers: 1,
        },
    );
    queue.submit([encoder.finish()]);

    let slice = buffer.slice(..);
    let (sender, receiver) = mpsc::channel();
    slice.map_async(MapMode::Read, move |result| {
        sender.send(result).expect("readback receiver alive");
    });
    device
        .poll(PollType::wait_indefinitely())
        .expect("poll GPU readback");
    receiver
        .recv()
        .expect("readback callback")
        .expect("map readback buffer");
    let mapped = slice.get_mapped_range();
    let center_offset = usize::try_from((HEIGHT / 2) * PADDED_BYTES_PER_ROW + (WIDTH / 2) * 4)
        .expect("small center offset");
    let pixel = mapped[center_offset..center_offset + 4]
        .try_into()
        .expect("one RGBA pixel");
    drop(mapped);
    buffer.unmap();
    pixel
}

fn assert_pixel_near(actual: [u8; 4], expected: [u8; 4]) {
    for channel in 0..3 {
        let actual_channel = actual[channel];
        let expected_channel = expected[channel];
        assert!(
            actual_channel.abs_diff(expected_channel) <= 2,
            "channel {channel}: expected {expected_channel}±2, got {actual_channel} (pixel {actual:?})"
        );
    }
    assert_eq!(
        actual[3], expected[3],
        "alpha mismatch for pixel {actual:?}"
    );
}
