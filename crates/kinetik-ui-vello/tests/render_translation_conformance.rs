//! Vello render translation conformance tests.

use kinetik_ui_core::{
    Brush, ClipId, Color, CornerRadius, GradientStop, ImageId, ImagePrimitive, LayerId,
    LinePrimitive, LinearGradient, PathElement, PathPrimitive, Point, Primitive, Rect,
    RectPrimitive, ShadowPrimitive, Size, Stroke, TextLayoutId, TextPrimitive, TextureId,
    TexturePrimitive, Transform, Vec2,
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

const NESTED_CONTEXT_SNAPSHOT: &str = "commands:\n  0: layer=13 transform=[2.000, 0.000, 0.000, 2.000, 4.000, 6.000] clips=[{rect=(0.000, 0.000, 100.000, 80.000) transform=[1.000, 0.000, 0.000, 1.000, 0.000, 0.000]}, {rect=(5.000, 6.000, 50.000, 40.000) transform=[1.000, 0.000, 0.000, 1.000, 1.000, 2.000]}] shadow rect=(2.000, 3.000, 12.000, 8.000) offset=(1.500, 2.500) blur=4.000 spread=1.000 radius=3.000 color=rgba(0.000, 0.000, 0.000, 0.250)\n  1: layer=13 transform=[2.000, 0.000, 0.000, 2.000, 4.000, 6.000] clips=[{rect=(0.000, 0.000, 100.000, 80.000) transform=[1.000, 0.000, 0.000, 1.000, 0.000, 0.000]}, {rect=(5.000, 6.000, 50.000, 40.000) transform=[1.000, 0.000, 0.000, 1.000, 1.000, 2.000]}] line from=(0.000, 0.000) to=(10.000, 5.000) stroke=2.000 rgba(0.750, 0.500, 0.250, 1.000)\n  2: layer=13 transform=[2.000, 0.000, 0.000, 2.000, 4.000, 6.000] clips=[{rect=(0.000, 0.000, 100.000, 80.000) transform=[1.000, 0.000, 0.000, 1.000, 0.000, 0.000]}, {rect=(5.000, 6.000, 50.000, 40.000) transform=[1.000, 0.000, 0.000, 1.000, 1.000, 2.000]}] path elements=[M(0.000, 0.000), L(8.000, 0.000), Q(10.000, 2.000; 8.000, 4.000), C(6.000, 6.000; 2.000, 6.000; 0.000, 4.000), Z] fill=linear(0.000,0.000)-(12.000,0.000)[rgba(1.000, 0.000, 0.000, 1.000)@0.000,rgba(0.000, 0.000, 1.000, 1.000)@1.000] stroke=1.500 rgba(0.100, 0.200, 0.300, 0.400)\n  3: layer=12 transform=[1.000, 0.000, 0.000, 1.000, 1.000, 2.000] clips=[{rect=(0.000, 0.000, 100.000, 80.000) transform=[1.000, 0.000, 0.000, 1.000, 0.000, 0.000]}] rect rect=(20.000, 10.000, 24.000, 12.000) fill=none stroke=1.000 linear(0.000,0.000)-(0.000,10.000)[rgba(1.000, 1.000, 1.000, 1.000)@0.000,rgba(0.000, 0.000, 0.000, 1.000)@1.000] radius=(0.000, 0.000, 0.000, 0.000)\ndiagnostics:";

fn red_to_blue_gradient() -> Brush {
    Brush::LinearGradient(
        LinearGradient::new(
            Point::new(0.0, 0.0),
            Point::new(12.0, 0.0),
            &[
                GradientStop::new(0.0, Color::rgba(1.0, 0.0, 0.0, 1.0)),
                GradientStop::new(1.0, Color::rgba(0.0, 0.0, 1.0, 1.0)),
            ],
        )
        .expect("valid gradient"),
    )
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
fn render_translation_conformance_preserves_nested_context_geometry_and_brushes() {
    let primitives = vec![
        Primitive::LayerBegin {
            id: LayerId::from_raw(12),
        },
        Primitive::ClipBegin {
            id: ClipId::from_raw(21),
            rect: Rect::new(0.0, 0.0, 100.0, 80.0),
        },
        Primitive::TransformBegin(Transform::translation(Vec2::new(1.0, 2.0))),
        Primitive::LayerBegin {
            id: LayerId::from_raw(13),
        },
        Primitive::ClipBegin {
            id: ClipId::from_raw(22),
            rect: Rect::new(5.0, 6.0, 50.0, 40.0),
        },
        Primitive::TransformBegin(Transform {
            m11: 2.0,
            m12: 0.0,
            m21: 0.0,
            m22: 2.0,
            dx: 3.0,
            dy: 4.0,
        }),
        Primitive::Shadow(ShadowPrimitive::new(
            Rect::new(2.0, 3.0, 12.0, 8.0),
            Vec2::new(1.5, 2.5),
            4.0,
            1.0,
            3.0,
            Color::rgba(0.0, 0.0, 0.0, 0.25),
        )),
        Primitive::Line(LinePrimitive {
            from: Point::new(0.0, 0.0),
            to: Point::new(10.0, 5.0),
            stroke: Stroke::new(2.0, Brush::Solid(Color::rgba(0.75, 0.5, 0.25, 1.0))),
        }),
        Primitive::Path(PathPrimitive::new(
            [
                PathElement::MoveTo(Point::new(0.0, 0.0)),
                PathElement::LineTo(Point::new(8.0, 0.0)),
                PathElement::QuadTo {
                    ctrl: Point::new(10.0, 2.0),
                    to: Point::new(8.0, 4.0),
                },
                PathElement::CubicTo {
                    ctrl1: Point::new(6.0, 6.0),
                    ctrl2: Point::new(2.0, 6.0),
                    to: Point::new(0.0, 4.0),
                },
                PathElement::Close,
            ],
            Some(red_to_blue_gradient()),
            Some(Stroke::new(
                1.5,
                Brush::Solid(Color::rgba(0.1, 0.2, 0.3, 0.4)),
            )),
        )),
        Primitive::TransformEnd,
        Primitive::ClipEnd {
            id: ClipId::from_raw(22),
        },
        Primitive::LayerEnd {
            id: LayerId::from_raw(13),
        },
        Primitive::Rect(RectPrimitive {
            rect: Rect::new(20.0, 10.0, 24.0, 12.0),
            fill: None,
            stroke: Some(Stroke::new(
                1.0,
                Brush::LinearGradient(LinearGradient::between(
                    Point::new(0.0, 0.0),
                    Point::new(0.0, 10.0),
                    Color::WHITE,
                    Color::BLACK,
                )),
            )),
            radius: CornerRadius::all(0.0),
        }),
        Primitive::TransformEnd,
        Primitive::ClipEnd {
            id: ClipId::from_raw(21),
        },
        Primitive::LayerEnd {
            id: LayerId::from_raw(12),
        },
    ];

    let translation = translate_primitives(&primitives, &RenderResources::new());

    assert_eq!(
        render_translation_snapshot(&translation),
        NESTED_CONTEXT_SNAPSHOT
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
    assert_eq!(
        render_translation_snapshot(&translation),
        "commands:\n  0: layer=0 transform=[1.000, 0.000, 0.000, 1.000, 0.000, 0.000] clips=[] text layout=6 origin=(0.000, 10.000) family=\"sans-serif\" size=12.000 line_height=16.000 color=rgba(1.000, 1.000, 1.000, 1.000) text=\"Missing layout\"\n  1: layer=0 transform=[1.000, 0.000, 0.000, 1.000, 0.000, 0.000] clips=[] text layout=none origin=(0.000, 30.000) family=\"sans-serif\" size=12.000 line_height=16.000 color=rgba(1.000, 1.000, 1.000, 1.000) text=\"Fallback text\"\n  2: layer=0 transform=[1.000, 0.000, 0.000, 1.000, 0.000, 0.000] clips=[] image#1 rect=(0.000, 0.000, 8.000, 8.000) tint=none\n  3: layer=0 transform=[1.000, 0.000, 0.000, 1.000, 0.000, 0.000] clips=[] image#2 rect=(10.000, 0.000, 8.000, 8.000) tint=none\n  4: layer=0 transform=[1.000, 0.000, 0.000, 1.000, 0.000, 0.000] clips=[] texture#3 rect=(20.000, 0.000, 8.000, 8.000) source_size=2.000x2.000\n  5: layer=0 transform=[1.000, 0.000, 0.000, 1.000, 0.000, 0.000] clips=[] texture#4 rect=(30.000, 0.000, 8.000, 8.000) source_size=2.000x2.000\ndiagnostics:\n  missing_text_layout#6\n  missing_image#1\n  missing_image_pixels#2\n  missing_texture#3\n  missing_texture_snapshot#4"
    );
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
