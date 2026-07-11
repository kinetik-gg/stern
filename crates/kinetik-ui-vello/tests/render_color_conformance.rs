//! Public Vello color, alpha, and tint conformance tests.

#![allow(clippy::float_cmp)]

use kinetik_ui_core::{
    Brush, Color, CornerRadius, ImageId, ImagePrimitive, LinearGradient, PhysicalSize, Point,
    Primitive, Rect, RectPrimitive, ScaleFactor, ShadowPrimitive, Size, TextPrimitive, TextureId,
    TexturePrimitive, Vec2, ViewportInfo,
};
use kinetik_ui_vello::{
    ImageResource, RenderFrameInput, RenderImage, RenderImageAlpha, RenderImageFormat,
    RenderImageSampling, RenderResources, TextureResource, VelloRenderer,
};
use vello::peniko::color::ColorSpaceTag;

#[test]
#[allow(clippy::too_many_lines)]
fn submit_frame_preserves_srgb_colors_stops_and_image_resources() {
    let image = ImageId::from_raw(1);
    let texture = TextureId::from_raw(2);
    let mut resources = RenderResources::new();
    resources.register_image(ImageResource {
        id: image,
        size: Size::new(1.0, 1.0),
        sampling: RenderImageSampling::Smooth,
        pixels: Some(RenderImage::rgba8(1, 1, vec![64, 80, 96, 128]).expect("valid image")),
        atlas_region: None,
    });
    resources.register_texture(TextureResource {
        id: texture,
        size: Size::new(1.0, 1.0),
        sampling: RenderImageSampling::Pixelated,
        snapshot: Some(
            RenderImage::new(
                1,
                1,
                vec![32, 16, 8, 64],
                RenderImageFormat::Bgra8,
                RenderImageAlpha::Premultiplied,
            )
            .expect("valid texture snapshot"),
        ),
    });

    let gradient = LinearGradient::between(
        Point::new(0.0, 0.0),
        Point::new(20.0, 0.0),
        Color::rgba(1.0, 0.25, 0.0, 0.25),
        Color::rgba(0.0, 0.5, 1.0, 0.75),
    );
    let primitives = vec![
        Primitive::Rect(RectPrimitive {
            rect: Rect::new(0.0, 0.0, 10.0, 10.0),
            fill: Some(Brush::Solid(Color::rgba(0.25, 0.5, 0.75, 0.5))),
            stroke: None,
            radius: CornerRadius::all(0.0),
        }),
        Primitive::Rect(RectPrimitive {
            rect: Rect::new(12.0, 0.0, 20.0, 10.0),
            fill: Some(Brush::LinearGradient(gradient)),
            stroke: None,
            radius: CornerRadius::all(0.0),
        }),
        Primitive::Text(TextPrimitive {
            layout: None,
            origin: Point::new(0.0, 28.0),
            text: "sRGB".to_owned(),
            family: "sans-serif".to_owned(),
            size: 12.0,
            line_height: 16.0,
            brush: Brush::Solid(Color::rgba(0.8, 0.7, 0.6, 0.9)),
        }),
        Primitive::Shadow(ShadowPrimitive::new(
            Rect::new(0.0, 32.0, 12.0, 8.0),
            Vec2::new(1.0, 1.0),
            2.0,
            0.0,
            1.0,
            Color::rgba(0.1, 0.2, 0.3, 0.4),
        )),
        Primitive::Image(ImagePrimitive {
            image,
            rect: Rect::new(16.0, 32.0, 8.0, 8.0),
            tint: Some(Color::rgba(
                64.0 / 255.0,
                128.0 / 255.0,
                192.0 / 255.0,
                135.0 / 255.0,
            )),
        }),
        Primitive::Texture(TexturePrimitive {
            texture,
            rect: Rect::new(28.0, 32.0, 8.0, 8.0),
            source_size: Size::new(1.0, 1.0),
        }),
    ];

    let mut renderer = VelloRenderer::new();
    let output = renderer.submit_frame(RenderFrameInput {
        viewport: ViewportInfo::new(
            Size::new(64.0, 64.0),
            PhysicalSize::new(64, 64),
            ScaleFactor::ONE,
        ),
        primitives: &primitives,
        resources: &resources,
    });

    assert!(output.diagnostics.is_empty());
    let encoding = renderer.scene().encoding();
    assert!(!encoding.is_empty());
    assert!(encoding.draw_data.contains(&0x8060_4020));
    assert!(encoding.resources.patches.len() >= 2);
    assert!(!encoding.resources.glyph_runs.is_empty());

    let stops = &encoding.resources.color_stops;
    assert_eq!(stops.len(), 2);
    assert_eq!(stops[0].offset, 0.0);
    assert_eq!(stops[0].color.cs, ColorSpaceTag::Srgb);
    assert_eq!(stops[0].color.components, [1.0, 0.25, 0.0, 0.25]);
    assert_eq!(stops[1].offset, 1.0);
    assert_eq!(stops[1].color.cs, ColorSpaceTag::Srgb);
    assert_eq!(stops[1].color.components, [0.0, 0.5, 1.0, 0.75]);
}
