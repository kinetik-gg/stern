//! Vello render translation conformance tests.

use kinetik_ui_core::{
    Brush, ClipId, Color, CornerRadius, ImageId, ImagePrimitive, LayerId, Point, Primitive, Rect,
    RectPrimitive, Size, TextLayoutId, TextPrimitive, TextureId, TexturePrimitive, Transform, Vec2,
};
use kinetik_ui_vello::{
    ImageAtlasRegion, ImageResource, RenderDiagnostic, RenderImage, RenderImageSampling,
    RenderResources, TextureResource, render_translation_snapshot, translate_primitives,
};

fn tiny_image() -> RenderImage {
    RenderImage::rgba8(
        2,
        2,
        vec![
            255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255,
        ],
    )
    .expect("valid tiny image")
}

#[test]
fn render_translation_conformance_preserves_primitive_order_and_context() {
    let missing_layout = TextLayoutId::from_raw(77);
    let missing_image = ImageId::from_raw(88);
    let primitives = vec![
        Primitive::LayerBegin {
            id: LayerId::from_raw(3),
        },
        Primitive::ClipBegin {
            id: ClipId::from_raw(4),
            rect: Rect::new(0.0, 0.0, 40.0, 24.0),
        },
        Primitive::TransformBegin(Transform::translation(Vec2::new(2.5, 3.5))),
        Primitive::Rect(RectPrimitive {
            rect: Rect::new(1.0, 2.0, 10.0, 6.0),
            fill: Some(Brush::Solid(Color::WHITE)),
            stroke: None,
            radius: CornerRadius::all(1.5),
        }),
        Primitive::Text(TextPrimitive {
            layout: Some(missing_layout),
            origin: Point::new(4.0, 14.0),
            text: "Hi".to_owned(),
            family: "monospace".to_owned(),
            size: 12.0,
            line_height: 17.0,
            brush: Brush::Solid(Color::BLACK),
        }),
        Primitive::Image(ImagePrimitive {
            image: missing_image,
            rect: Rect::new(16.0, 4.0, 8.0, 8.0),
            tint: Some(Color::rgba(0.25, 0.5, 0.75, 1.0)),
        }),
        Primitive::TransformEnd,
        Primitive::ClipEnd {
            id: ClipId::from_raw(4),
        },
        Primitive::LayerEnd {
            id: LayerId::from_raw(3),
        },
        Primitive::Texture(TexturePrimitive {
            texture: TextureId::from_raw(9),
            rect: Rect::new(0.0, 30.0, 16.0, 16.0),
            source_size: Size::new(2.0, 2.0),
        }),
    ];

    let translation = translate_primitives(&primitives, &RenderResources::new());

    assert_eq!(
        render_translation_snapshot(&translation),
        "commands:\n  0: layer=3 transform=[1.000, 0.000, 0.000, 1.000, 2.500, 3.500] clips=[{rect=(0.000, 0.000, 40.000, 24.000) transform=[1.000, 0.000, 0.000, 1.000, 0.000, 0.000]}] rect rect=(1.000, 2.000, 10.000, 6.000) fill=rgba(1.000, 1.000, 1.000, 1.000) stroke=none radius=(1.500, 1.500, 1.500, 1.500)\n  1: layer=3 transform=[1.000, 0.000, 0.000, 1.000, 2.500, 3.500] clips=[{rect=(0.000, 0.000, 40.000, 24.000) transform=[1.000, 0.000, 0.000, 1.000, 0.000, 0.000]}] text layout=77 origin=(4.000, 14.000) family=\"monospace\" size=12.000 line_height=17.000 color=rgba(0.000, 0.000, 0.000, 1.000) text=\"Hi\"\n  2: layer=3 transform=[1.000, 0.000, 0.000, 1.000, 2.500, 3.500] clips=[{rect=(0.000, 0.000, 40.000, 24.000) transform=[1.000, 0.000, 0.000, 1.000, 0.000, 0.000]}] image#88 rect=(16.000, 4.000, 8.000, 8.000) tint=rgba(0.250, 0.500, 0.750, 1.000)\n  3: layer=0 transform=[1.000, 0.000, 0.000, 1.000, 0.000, 0.000] clips=[] texture#9 rect=(0.000, 30.000, 16.000, 16.000) source_size=2.000x2.000\ndiagnostics:\n  missing_text_layout#77\n  missing_image#88\n  missing_texture#9"
    );
}

#[test]
fn render_translation_conformance_reports_recoverable_missing_resource_paths() {
    let mut resources = RenderResources::new();
    resources.register_image(ImageResource {
        id: ImageId::from_raw(2),
        size: Size::new(64.0, 64.0),
        sampling: RenderImageSampling::Pixelated,
        pixels: None,
        atlas_region: None,
    });
    resources.register_texture(TextureResource {
        id: TextureId::from_raw(4),
        size: Size::new(2.0, 2.0),
        sampling: RenderImageSampling::Pixelated,
        snapshot: None,
    });
    let primitives = vec![
        Primitive::Text(TextPrimitive {
            layout: Some(TextLayoutId::from_raw(6)),
            origin: Point::new(0.0, 10.0),
            text: "Missing layout".to_owned(),
            family: "sans-serif".to_owned(),
            size: 12.0,
            line_height: 16.0,
            brush: Brush::Solid(Color::WHITE),
        }),
        Primitive::Text(TextPrimitive {
            layout: None,
            origin: Point::new(0.0, 30.0),
            text: "Fallback text".to_owned(),
            family: "sans-serif".to_owned(),
            size: 12.0,
            line_height: 16.0,
            brush: Brush::Solid(Color::WHITE),
        }),
        Primitive::Image(ImagePrimitive {
            image: ImageId::from_raw(1),
            rect: Rect::new(0.0, 0.0, 8.0, 8.0),
            tint: None,
        }),
        Primitive::Image(ImagePrimitive {
            image: ImageId::from_raw(2),
            rect: Rect::new(10.0, 0.0, 8.0, 8.0),
            tint: None,
        }),
        Primitive::Texture(TexturePrimitive {
            texture: TextureId::from_raw(3),
            rect: Rect::new(20.0, 0.0, 8.0, 8.0),
            source_size: Size::new(2.0, 2.0),
        }),
        Primitive::Texture(TexturePrimitive {
            texture: TextureId::from_raw(4),
            rect: Rect::new(30.0, 0.0, 8.0, 8.0),
            source_size: Size::new(2.0, 2.0),
        }),
    ];

    let translation = translate_primitives(&primitives, &resources);

    assert_eq!(
        translation.diagnostics,
        vec![
            RenderDiagnostic::MissingTextLayout(TextLayoutId::from_raw(6)),
            RenderDiagnostic::MissingImage(ImageId::from_raw(1)),
            RenderDiagnostic::MissingImagePixels(ImageId::from_raw(2)),
            RenderDiagnostic::MissingTexture(TextureId::from_raw(3)),
            RenderDiagnostic::MissingTextureSnapshot(TextureId::from_raw(4)),
        ]
    );
    assert_eq!(translation.commands.len(), 6);
}

#[test]
fn render_translation_conformance_drops_invalid_resource_source_geometry() {
    let mut resources = RenderResources::new();
    resources.register_texture(TextureResource {
        id: TextureId::from_raw(1),
        size: Size::new(2.0, 2.0),
        sampling: RenderImageSampling::Pixelated,
        snapshot: Some(RenderImage::rgba8(1, 1, vec![255; 4]).expect("valid snapshot")),
    });
    resources.register_image(ImageResource {
        id: ImageId::from_raw(2),
        size: Size::new(2.0, 2.0),
        sampling: RenderImageSampling::Pixelated,
        pixels: Some(tiny_image()),
        atlas_region: None,
    });
    resources.register_image(ImageResource {
        id: ImageId::from_raw(3),
        size: Size::new(1.0, 1.0),
        sampling: RenderImageSampling::Pixelated,
        pixels: None,
        atlas_region: Some(ImageAtlasRegion {
            atlas: ImageId::from_raw(2),
            source: Rect::new(1.0, 1.0, 4.0, 4.0),
        }),
    });
    let primitives = vec![
        Primitive::Texture(TexturePrimitive {
            texture: TextureId::from_raw(1),
            rect: Rect::new(0.0, 0.0, 8.0, 8.0),
            source_size: Size::new(f32::NAN, 2.0),
        }),
        Primitive::Texture(TexturePrimitive {
            texture: TextureId::from_raw(1),
            rect: Rect::new(10.0, 0.0, 8.0, 8.0),
            source_size: Size::new(2.0, 2.0),
        }),
        Primitive::Image(ImagePrimitive {
            image: ImageId::from_raw(3),
            rect: Rect::new(20.0, 0.0, 8.0, 8.0),
            tint: None,
        }),
    ];

    let translation = translate_primitives(&primitives, &resources);

    assert_eq!(
        render_translation_snapshot(&translation),
        "commands:\n  0: layer=0 transform=[1.000, 0.000, 0.000, 1.000, 0.000, 0.000] clips=[] image#3 rect=(20.000, 0.000, 8.000, 8.000) tint=none\ndiagnostics:\n  invalid_geometry:texture_source_size\n  invalid_geometry:texture_snapshot_size\n  invalid_geometry:image_atlas_source"
    );
}
