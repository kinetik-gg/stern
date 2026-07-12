use crate::{
    ImageAtlasRegion, ImageResource, RenderCommand, RenderImage, RenderImageSampling,
    RenderResources, TextLayoutResource, TextureResource,
};
use kinetik_ui_core::{ImageId, Rect, Size, TextLayoutId, TextureId, Transform};
use kinetik_ui_text::{CosmicTextEngine, TextLayoutKey, TextStyle};

pub(crate) fn resources() -> RenderResources {
    let mut resources = RenderResources::new();
    resources.register_image(ImageResource {
        id: ImageId::from_raw(1),
        size: Size::new(2.0, 2.0),
        sampling: RenderImageSampling::default(),
        pixels: Some(tiny_image()),
        atlas_region: None,
    });
    resources.register_texture(TextureResource {
        id: TextureId::from_raw(2),
        size: Size::new(2.0, 2.0),
        sampling: RenderImageSampling::default(),
        snapshot: Some(tiny_image()),
    });
    resources
}

pub(crate) fn size_only_resources() -> RenderResources {
    let mut resources = RenderResources::new();
    resources.register_image(ImageResource {
        id: ImageId::from_raw(1),
        size: Size::new(64.0, 64.0),
        sampling: RenderImageSampling::default(),
        pixels: None,
        atlas_region: None,
    });
    resources.register_texture(TextureResource {
        id: TextureId::from_raw(2),
        size: Size::new(2.0, 2.0),
        sampling: RenderImageSampling::default(),
        snapshot: None,
    });
    resources
}

pub(crate) fn atlas_resources() -> RenderResources {
    let mut resources = RenderResources::new();
    resources.register_image(ImageResource {
        id: ImageId::from_raw(1),
        size: Size::new(2.0, 2.0),
        sampling: RenderImageSampling::default(),
        pixels: Some(tiny_image()),
        atlas_region: None,
    });
    resources.register_image(ImageResource {
        id: ImageId::from_raw(3),
        size: Size::new(1.0, 1.0),
        sampling: RenderImageSampling::default(),
        pixels: None,
        atlas_region: Some(ImageAtlasRegion {
            atlas: ImageId::from_raw(1),
            source: Rect::new(1.0, 0.0, 1.0, 1.0),
        }),
    });
    resources.register_image(ImageResource {
        id: ImageId::from_raw(4),
        size: Size::new(1.0, 1.0),
        sampling: RenderImageSampling::default(),
        pixels: None,
        atlas_region: Some(ImageAtlasRegion {
            atlas: ImageId::from_raw(1),
            source: Rect::new(0.0, 1.0, 1.0, 1.0),
        }),
    });
    resources
}

pub(crate) fn tiny_image() -> RenderImage {
    RenderImage::rgba8(
        2,
        2,
        vec![
            255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255,
        ],
    )
    .expect("valid tiny image")
}

pub(crate) fn one_pixel_image() -> RenderImage {
    RenderImage::rgba8(1, 1, vec![255, 255, 255, 255]).expect("valid one pixel image")
}

pub(crate) fn text_layout_resource(id: TextLayoutId, text: &str) -> TextLayoutResource {
    let mut engine = CosmicTextEngine::new();
    let key = TextLayoutKey::new(text, TextStyle::new("sans-serif", 12.0, 16.0), 200.0, false);
    let layout = engine.shape_text(&key);
    TextLayoutResource {
        id,
        key,
        layout: std::sync::Arc::new(layout),
    }
}

pub(crate) fn clip_rects(command: &RenderCommand) -> Vec<Rect> {
    command.clips.iter().map(|clip| clip.rect).collect()
}

pub(crate) fn clip_transforms(command: &RenderCommand) -> Vec<Transform> {
    command.clips.iter().map(|clip| clip.transform).collect()
}

pub(crate) fn assert_approx(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < f32::EPSILON,
        "expected {actual} to equal {expected}"
    );
}

pub(crate) fn assert_approx64(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < f64::EPSILON,
        "expected {actual} to equal {expected}"
    );
}
