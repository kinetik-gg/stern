//! Backend-neutral image color and alpha conformance tests.

use kinetik_ui_core::{ImageId, Size, TextureId};
use kinetik_ui_render::{
    ImageResource, RenderImage, RenderImageAlpha, RenderImageFormat, RenderImageSampling,
    RenderResources, TextureResource,
};

#[test]
fn convenience_constructors_default_to_straight_srgb_bytes() {
    let rgba = RenderImage::rgba8(1, 1, vec![1, 2, 3, 4]).expect("valid RGBA image");
    assert_eq!(rgba.format, RenderImageFormat::Rgba8);
    assert_eq!(rgba.alpha, RenderImageAlpha::Alpha);
    assert_eq!(rgba.data.as_ref(), &[1, 2, 3, 4]);

    let bgra = RenderImage::bgra8(1, 1, vec![3, 2, 1, 4]).expect("valid BGRA image");
    assert_eq!(bgra.format, RenderImageFormat::Bgra8);
    assert_eq!(bgra.alpha, RenderImageAlpha::Alpha);
    assert_eq!(bgra.data.as_ref(), &[3, 2, 1, 4]);
}

#[test]
fn explicit_premultiplied_metadata_and_payload_survive_construction() {
    let image = RenderImage::new(
        2,
        1,
        vec![32, 16, 8, 64, 128, 64, 32, 128],
        RenderImageFormat::Rgba8,
        RenderImageAlpha::Premultiplied,
    )
    .expect("valid premultiplied image");

    assert_eq!(image.width, 2);
    assert_eq!(image.height, 1);
    assert_eq!(image.format, RenderImageFormat::Rgba8);
    assert_eq!(image.alpha, RenderImageAlpha::Premultiplied);
    assert_eq!(image.data.as_ref(), &[32, 16, 8, 64, 128, 64, 32, 128]);
}

#[test]
fn public_resource_snapshot_grammar_remains_payload_free() {
    let mut resources = RenderResources::new();
    resources.register_texture(TextureResource {
        id: TextureId::from_raw(9),
        size: Size::new(1.0, 1.0),
        sampling: RenderImageSampling::Smooth,
        snapshot: Some(
            RenderImage::new(
                1,
                1,
                vec![8, 16, 32, 64],
                RenderImageFormat::Bgra8,
                RenderImageAlpha::Premultiplied,
            )
            .expect("valid texture snapshot"),
        ),
    });
    resources.register_image(ImageResource {
        id: ImageId::from_raw(4),
        size: Size::new(1.0, 1.0),
        sampling: RenderImageSampling::UiIcon,
        pixels: Some(RenderImage::rgba8(1, 1, vec![1, 2, 3, 4]).expect("valid image")),
        atlas_region: None,
    });

    let text = resources.snapshot().to_text();
    assert_eq!(
        text,
        "resources:\n  image#4 size=1.000x1.000 sampling=ui_icon pixels=true atlas=none\n  texture#9 size=1.000x1.000 sampling=smooth snapshot=true"
    );
    assert!(!text.contains("rgba8"));
    assert!(!text.contains("bgra8"));
    assert!(!text.contains("premultiplied"));
    assert!(!text.contains("1, 2, 3, 4"));
}
