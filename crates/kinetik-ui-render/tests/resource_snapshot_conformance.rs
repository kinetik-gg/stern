//! Backend-neutral resource snapshot conformance tests.

use std::sync::Arc;

use kinetik_ui_core::{ImageId, Rect, Size, TextLayoutId, TextureId};
use kinetik_ui_render::{
    ImageAtlasRegion, ImageResource, RenderImage, RenderImageSampling, RenderResources,
    TextLayoutResource, TextureResource,
};
use kinetik_ui_text::{ShapedTextLayout, TextLayoutKey, TextStyle};

fn empty_layout(width: f32, height: f32, line_count: usize) -> Arc<ShapedTextLayout> {
    Arc::new(ShapedTextLayout {
        size: Size::new(width, height),
        line_count,
        lines: Vec::new(),
        runs: Vec::new(),
    })
}

#[test]
fn resource_snapshot_conformance_sorts_resources_by_handle() {
    let mut resources = RenderResources::new();

    resources.register_texture(TextureResource {
        id: TextureId::from_raw(40),
        size: Size::new(32.0, 16.0),
        sampling: RenderImageSampling::HighQuality,
        snapshot: None,
    });
    resources.register_image(ImageResource {
        id: ImageId::from_raw(9),
        size: Size::new(8.0, 8.0),
        sampling: RenderImageSampling::Smooth,
        pixels: None,
        atlas_region: None,
    });
    resources.register_text_layout(TextLayoutResource {
        id: TextLayoutId::from_raw(12),
        key: TextLayoutKey::new(
            "Later",
            TextStyle::new("sans-serif", 12.0, 16.0),
            200.0,
            false,
        ),
        layout: empty_layout(30.0, 16.0, 1),
    });
    resources.register_texture(TextureResource {
        id: TextureId::from_raw(4),
        size: Size::new(4.0, 2.0),
        sampling: RenderImageSampling::Pixelated,
        snapshot: Some(RenderImage::rgba8(4, 2, vec![255; 32]).expect("valid texture snapshot")),
    });
    resources.register_image(ImageResource {
        id: ImageId::from_raw(1),
        size: Size::new(2.0, 1.0),
        sampling: RenderImageSampling::UiIcon,
        pixels: Some(RenderImage::rgba8(2, 1, vec![128; 8]).expect("valid image pixels")),
        atlas_region: None,
    });
    resources.register_text_layout(TextLayoutResource {
        id: TextLayoutId::from_raw(3),
        key: TextLayoutKey::new(
            "First",
            TextStyle::new("sans-serif", 12.0, 16.0),
            200.0,
            false,
        ),
        layout: empty_layout(10.0, 16.0, 1),
    });

    assert_eq!(
        resources.snapshot().to_text(),
        "resources:\n  image#1 size=2.000x1.000 sampling=ui_icon pixels=true atlas=none\n  image#9 size=8.000x8.000 sampling=smooth pixels=false atlas=none\n  texture#4 size=4.000x2.000 sampling=pixelated snapshot=true\n  texture#40 size=32.000x16.000 sampling=high_quality snapshot=false\n  text_layout#3 size=10.000x16.000 lines=1 glyphs=0\n  text_layout#12 size=30.000x16.000 lines=1 glyphs=0"
    );
}

#[test]
fn resource_snapshot_conformance_omits_raw_payloads_and_backend_objects() {
    let mut resources = RenderResources::new();

    resources.register_image(ImageResource {
        id: ImageId::from_raw(5),
        size: Size::new(f32::NAN, -0.0),
        sampling: RenderImageSampling::Pixelated,
        pixels: Some(RenderImage::rgba8(1, 1, vec![1, 2, 3, 4]).expect("valid image pixels")),
        atlas_region: Some(ImageAtlasRegion {
            atlas: ImageId::from_raw(2),
            source: Rect::new(1.0, 2.0, f32::INFINITY, -0.0),
        }),
    });
    resources.register_texture(TextureResource {
        id: TextureId::from_raw(6),
        size: Size::new(-0.0, f32::NEG_INFINITY),
        sampling: RenderImageSampling::Smooth,
        snapshot: Some(RenderImage::rgba8(1, 1, vec![5, 6, 7, 8]).expect("valid snapshot")),
    });
    resources.register_text_layout(TextLayoutResource {
        id: TextLayoutId::from_raw(7),
        key: TextLayoutKey::new(
            "Bytes stay out of snapshots",
            TextStyle::new("sans-serif", 12.0, 16.0),
            200.0,
            false,
        ),
        layout: empty_layout(f32::INFINITY, -0.0, 2),
    });

    let snapshot = resources.snapshot().to_text();

    assert_eq!(
        snapshot,
        "resources:\n  image#5 size=0.000x0.000 sampling=pixelated pixels=true atlas=2:(1.000,2.000,0.000,0.000)\n  texture#6 size=0.000x0.000 sampling=smooth snapshot=true\n  text_layout#7 size=0.000x0.000 lines=2 glyphs=0"
    );
    assert!(!snapshot.contains("1, 2, 3, 4"));
    assert!(!snapshot.contains("5, 6, 7, 8"));
    assert!(!snapshot.contains("RenderImage"));
    assert!(!snapshot.contains("Arc"));
}
