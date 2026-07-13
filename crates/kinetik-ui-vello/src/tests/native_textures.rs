use kinetik_ui_core::{
    Brush, ClipId, Color, CornerRadius, PhysicalSize, Primitive, Rect, RectPrimitive, ScaleFactor,
    Size, TextureId, TexturePrimitive, Transform, Vec2, ViewportInfo,
};
use kinetik_ui_render::{
    RenderDiagnostic, RenderFrameInput, RenderImage, RenderImageSampling, RenderResources,
    TextureResource,
};
use vello::peniko::{Blob, ImageAlphaType, ImageData, ImageFormat};

use crate::{
    RenderCommandKind, VelloNativeTextureRegistry, VelloNativeTextureScope, VelloRenderer,
    translation::translate_primitives_with_native,
};

fn test_image(width: u32, height: u32, value: u8) -> ImageData {
    let byte_count = usize::try_from(width)
        .ok()
        .and_then(|width| {
            usize::try_from(height)
                .ok()
                .and_then(|height| width.checked_mul(height))
        })
        .and_then(|pixels| pixels.checked_mul(4))
        .expect("small test image");
    ImageData {
        data: Blob::from(vec![value; byte_count]),
        format: ImageFormat::Rgba8,
        alpha_type: ImageAlphaType::Alpha,
        width,
        height,
    }
}

fn viewport() -> ViewportInfo {
    ViewportInfo::new(
        Size::new(64.0, 64.0),
        PhysicalSize::new(64, 64),
        ScaleFactor::ONE,
    )
}

fn texture_primitive(texture: TextureId) -> Primitive {
    Primitive::Texture(TexturePrimitive {
        texture,
        rect: Rect::new(4.0, 6.0, 16.0, 12.0),
        source_size: Size::new(2.0, 2.0),
    })
}

fn resources(texture: TextureId, snapshot: Option<RenderImage>) -> RenderResources {
    let mut resources = RenderResources::new();
    resources.register_texture(TextureResource {
        id: texture,
        size: Size::new(2.0, 2.0),
        sampling: RenderImageSampling::Pixelated,
        snapshot,
    });
    resources
}

fn native_registry(
    texture: TextureId,
    extent: [u32; 2],
    sampling: RenderImageSampling,
) -> (VelloNativeTextureScope, VelloNativeTextureRegistry) {
    let scope = VelloNativeTextureScope::new().expect("test lower scope");
    let mut registry = VelloNativeTextureRegistry::new(&scope);
    assert!(registry.install_test_native_texture(
        &scope,
        texture,
        test_image(extent[0], extent[1], 91),
        extent,
        sampling,
    ));
    (scope, registry)
}

#[test]
fn native_texture_without_snapshot_resolves_and_suppresses_missing_diagnostic() {
    let texture = TextureId::from_raw(701);
    let resources = resources(texture, None);
    let primitives = vec![texture_primitive(texture)];
    let (scope, registry) = native_registry(texture, [2, 2], RenderImageSampling::Pixelated);
    let mut renderer = VelloRenderer::new();
    let output = renderer.submit_frame_with_native_textures(
        RenderFrameInput {
            viewport: viewport(),
            primitives: &primitives,
            resources: &resources,
        },
        &registry,
        &scope,
    );

    assert!(output.diagnostics.is_empty());
    assert_eq!(renderer.scene().encoding().resources.patches.len(), 1);
}

#[test]
fn native_texture_wins_over_compatible_cpu_snapshot() {
    let texture = TextureId::from_raw(702);
    let snapshot = RenderImage::rgba8(2, 2, vec![7; 16]).expect("valid snapshot");
    let resources = resources(texture, Some(snapshot));
    let primitives = vec![texture_primitive(texture)];
    let (scope, registry) = native_registry(texture, [2, 2], RenderImageSampling::Pixelated);
    let mut renderer = VelloRenderer::new();
    let output = renderer.submit_frame_with_native_textures(
        RenderFrameInput {
            viewport: viewport(),
            primitives: &primitives,
            resources: &resources,
        },
        &registry,
        &scope,
    );
    assert!(output.diagnostics.is_empty());
    assert_eq!(renderer.scene().encoding().resources.patches.len(), 1);

    let (mismatch_scope, mismatch_registry) =
        native_registry(texture, [3, 2], RenderImageSampling::Pixelated);
    let translation = translate_primitives_with_native(
        &primitives,
        &resources,
        Some((&mismatch_registry, &mismatch_scope)),
    );
    assert!(
        translation
            .diagnostics
            .contains(&RenderDiagnostic::InvalidGeometry(
                "native_texture_metadata",
            ))
    );
}

#[test]
fn native_metadata_mismatch_falls_back_with_invalid_geometry() {
    let texture = TextureId::from_raw(703);
    let resources = resources(texture, None);
    let primitives = vec![texture_primitive(texture)];
    let (scope, registry) = native_registry(texture, [4, 2], RenderImageSampling::Smooth);
    let translation =
        translate_primitives_with_native(&primitives, &resources, Some((&registry, &scope)));

    assert!(
        translation
            .diagnostics
            .contains(&RenderDiagnostic::InvalidGeometry(
                "native_texture_metadata",
            ))
    );
    assert!(
        translation
            .diagnostics
            .contains(&RenderDiagnostic::MissingTextureSnapshot(texture,))
    );
}

#[test]
fn native_resolution_preserves_order_transforms_clips_and_overlays() {
    let texture = TextureId::from_raw(704);
    let resources = resources(texture, None);
    let transform = Transform::translation(Vec2::new(3.0, 5.0));
    let clip = ClipId::from_raw(44);
    let primitives = vec![
        Primitive::Rect(RectPrimitive {
            rect: Rect::new(0.0, 0.0, 8.0, 8.0),
            fill: Some(Brush::Solid(Color::rgba(0.2, 0.3, 0.4, 1.0))),
            stroke: None,
            radius: CornerRadius::all(0.0),
        }),
        Primitive::TransformBegin(transform),
        Primitive::ClipBegin {
            id: clip,
            rect: Rect::new(1.0, 2.0, 20.0, 20.0),
        },
        texture_primitive(texture),
        Primitive::ClipEnd { id: clip },
        Primitive::TransformEnd,
        Primitive::Rect(RectPrimitive {
            rect: Rect::new(20.0, 20.0, 8.0, 8.0),
            fill: Some(Brush::Solid(Color::rgba(0.8, 0.2, 0.1, 1.0))),
            stroke: None,
            radius: CornerRadius::all(0.0),
        }),
    ];
    let (scope, registry) = native_registry(texture, [2, 2], RenderImageSampling::Pixelated);
    let translation =
        translate_primitives_with_native(&primitives, &resources, Some((&registry, &scope)));

    assert!(translation.diagnostics.is_empty());
    assert_eq!(translation.commands.len(), 3);
    assert!(matches!(
        translation.commands[0].kind,
        RenderCommandKind::Rect { .. }
    ));
    assert!(matches!(
        translation.commands[1].kind,
        RenderCommandKind::Texture { .. }
    ));
    assert!(matches!(
        translation.commands[2].kind,
        RenderCommandKind::Rect { .. }
    ));
    assert_eq!(translation.commands[1].transform, transform);
    assert_eq!(translation.commands[1].clips.len(), 1);
    assert_eq!(
        translation.commands[1].clips[0].rect,
        Rect::new(1.0, 2.0, 20.0, 20.0)
    );
}

#[test]
fn mismatched_native_scope_never_exposes_cross_renderer_image() {
    let texture = TextureId::from_raw(705);
    let (scope, registry) = native_registry(texture, [2, 2], RenderImageSampling::Pixelated);
    let foreign_scope = VelloNativeTextureScope::new().expect("foreign test lower scope");

    assert!(
        registry
            .resolve_native_texture(&foreign_scope, texture)
            .is_none()
    );
    assert!(
        registry
            .native_texture_metadata(&foreign_scope, texture)
            .is_none()
    );
    assert!(registry.resolve_native_texture(&scope, texture).is_some());
}
