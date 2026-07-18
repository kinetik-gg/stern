#![allow(clippy::float_cmp)]

use crate::{
    ImageDataCache, RenderCommandKind, RenderDiagnostic, RenderResources, encode_scene,
    translate_primitives, vello_fill, vello_stroke,
};
use stern_core::{
    Brush, Color, FillRule, IconGraphic, IconId, IconLayer, IconPath, IconPrimitive, IconStroke,
    PathData, PathElement, PathPrimitive, Point, Primitive, Rect, StaticIcon, Stroke, StrokeCap,
    StrokeJoin, Transform, Vec2,
};
use stern_text::TextLayoutStore;
use vello::{
    Scene,
    kurbo::{Cap, Join},
    peniko::Fill,
};

static FILL_ELEMENTS: [PathElement; 4] = [
    PathElement::MoveTo(Point::new(0.0, 0.0)),
    PathElement::LineTo(Point::new(256.0, 0.0)),
    PathElement::LineTo(Point::new(256.0, 256.0)),
    PathElement::Close,
];
static STROKE_ELEMENTS: [PathElement; 2] = [
    PathElement::MoveTo(Point::new(0.0, 128.0)),
    PathElement::LineTo(Point::new(256.0, 128.0)),
];
static ICON_PATHS: [IconPath; 3] = [
    IconPath::new(&FILL_ELEMENTS, Some(FillRule::EvenOdd), None, 0.5),
    IconPath::new(
        &STROKE_ELEMENTS,
        None,
        Some(IconStroke::new(16.0, StrokeCap::Round, StrokeJoin::Bevel)),
        1.0,
    ),
    IconPath::new(
        &FILL_ELEMENTS,
        Some(FillRule::NonZero),
        Some(IconStroke::new(8.0, StrokeCap::Square, StrokeJoin::Round)),
        1.0,
    ),
];
static ICON_LAYERS: [IconLayer; 1] = [IconLayer::new(&ICON_PATHS, 0.5)];
static GRAPHIC: IconGraphic = IconGraphic::new(Rect::new(0.0, 0.0, 256.0, 256.0), &ICON_LAYERS);
static EMPTY_GRAPHIC: IconGraphic = IconGraphic::new(Rect::new(0.0, 0.0, 256.0, 256.0), &[]);
static INVALID_GRAPHIC: IconGraphic =
    IconGraphic::new(Rect::new(0.0, 0.0, 0.0, f32::NAN), &ICON_LAYERS);
static EMPTY_PATH_LAYER: [IconLayer; 1] = [IconLayer::new(&[], 1.0)];
static EMPTY_PATH_GRAPHIC: IconGraphic =
    IconGraphic::new(Rect::new(0.0, 0.0, 256.0, 256.0), &EMPTY_PATH_LAYER);
static EMPTY_ELEMENTS_PATHS: [IconPath; 1] =
    [IconPath::new(&[], Some(FillRule::NonZero), None, 1.0)];
static EMPTY_ELEMENTS_LAYER: [IconLayer; 1] = [IconLayer::new(&EMPTY_ELEMENTS_PATHS, 1.0)];
static EMPTY_ELEMENTS_GRAPHIC: IconGraphic =
    IconGraphic::new(Rect::new(0.0, 0.0, 256.0, 256.0), &EMPTY_ELEMENTS_LAYER);
static INVALID_STROKE_PATHS: [IconPath; 1] = [IconPath::new(
    &STROKE_ELEMENTS,
    None,
    Some(IconStroke::new(
        f32::NAN,
        StrokeCap::Butt,
        StrokeJoin::Miter,
    )),
    1.0,
)];
static INVALID_STROKE_LAYER: [IconLayer; 1] = [IconLayer::new(&INVALID_STROKE_PATHS, 1.0)];
static INVALID_STROKE_GRAPHIC: IconGraphic =
    IconGraphic::new(Rect::new(0.0, 0.0, 256.0, 256.0), &INVALID_STROKE_LAYER);
static INVALID_OPACITY_PATHS: [IconPath; 1] = [IconPath::new(
    &FILL_ELEMENTS,
    Some(FillRule::NonZero),
    None,
    2.0,
)];
static INVALID_OPACITY_LAYER: [IconLayer; 1] = [IconLayer::new(&INVALID_OPACITY_PATHS, f32::NAN)];
static INVALID_OPACITY_GRAPHIC: IconGraphic =
    IconGraphic::new(Rect::new(0.0, 0.0, 256.0, 256.0), &INVALID_OPACITY_LAYER);

#[test]
fn static_icon_translation_preserves_borrowing_styles_transform_and_tint() {
    let tint = Color::rgba(0.2, 0.4, 0.6, 0.8);
    let icon = StaticIcon::new(IconId::from_raw(91), &GRAPHIC);
    let primitives = [
        Primitive::TransformBegin(Transform::translation(Vec2::new(3.0, 4.0))),
        Primitive::Icon(IconPrimitive::new(
            icon,
            Rect::new(10.0, 20.0, 32.0, 32.0),
            tint,
        )),
        Primitive::TransformEnd,
    ];

    let translation = translate_primitives(&primitives, &RenderResources::new());

    assert!(translation.diagnostics.is_empty());
    assert_eq!(translation.commands.len(), 5);
    assert!(matches!(
        translation.commands[0].kind,
        RenderCommandKind::OpacityGroupBegin { opacity: 0.5, .. }
    ));
    assert_eq!(translation.commands[1].transform.m11, 0.125);
    assert_eq!(translation.commands[1].transform.m22, 0.125);
    assert_eq!(translation.commands[1].transform.dx, 13.0);
    assert_eq!(translation.commands[1].transform.dy, 24.0);

    let RenderCommandKind::Path {
        elements,
        fill,
        stroke,
        fill_rule,
        opacity,
    } = &translation.commands[1].kind
    else {
        panic!("expected first icon path");
    };
    assert!(elements.is_static());
    let PathData::Static(elements) = elements else {
        unreachable!()
    };
    assert!(core::ptr::eq(*elements, FILL_ELEMENTS.as_slice()));
    assert_eq!(*fill, Some(Brush::Solid(tint)));
    assert_eq!(*stroke, None);
    assert_eq!(*fill_rule, FillRule::EvenOdd);
    assert_eq!(*opacity, 0.5);

    let RenderCommandKind::Path {
        fill,
        stroke: Some(stroke),
        opacity,
        ..
    } = translation.commands[2].kind
    else {
        panic!("expected stroked icon path");
    };
    assert_eq!(fill, None);
    assert_eq!(stroke.width, 16.0);
    assert_eq!(stroke.brush, Brush::Solid(tint));
    assert_eq!(stroke.cap, StrokeCap::Round);
    assert_eq!(stroke.join, StrokeJoin::Bevel);
    assert_eq!(opacity, 1.0);

    let RenderCommandKind::Path { fill, stroke, .. } = translation.commands[3].kind else {
        panic!("expected fill-and-stroke icon path");
    };
    assert_eq!(fill, Some(Brush::Solid(tint)));
    assert!(stroke.is_some());
    assert!(matches!(
        translation.commands[4].kind,
        RenderCommandKind::OpacityGroupEnd
    ));
}

fn encode(commands: &[crate::RenderCommand]) -> Scene {
    let mut scene = Scene::new();
    let resources = RenderResources::new();
    let mut layouts = TextLayoutStore::new();
    let mut images = ImageDataCache::default();
    encode_scene(
        &mut scene,
        commands,
        &resources,
        &mut layouts,
        &mut images,
        1.0,
    );
    scene
}

#[test]
fn production_scene_encoding_preserves_icon_styles_transform_tint_and_group_opacity() {
    let tint = Color::rgba(0.2, 0.4, 0.6, 0.8);
    let icon = StaticIcon::new(IconId::from_raw(91), &GRAPHIC);
    let translation = translate_primitives(
        &[
            Primitive::TransformBegin(Transform::translation(Vec2::new(3.0, 4.0))),
            Primitive::Icon(IconPrimitive::new(
                icon,
                Rect::new(10.0, 20.0, 32.0, 32.0),
                tint,
            )),
            Primitive::TransformEnd,
        ],
        &RenderResources::new(),
    );

    let scene = encode(&translation.commands);
    let encoding = scene.encoding();

    assert_eq!(encoding.n_open_clips, 0);
    assert_eq!(encoding.n_clips, 2);
    assert!(encoding.n_paths >= 6);
    assert!(!encoding.path_data.is_empty());
    assert_eq!(encoding.draw_tags.first().map(|tag| tag.0), Some(0x49));
    assert_eq!(encoding.draw_tags.last().map(|tag| tag.0), Some(0x21));
    assert!(encoding.draw_data.contains(&0.5_f32.to_bits()));
    assert!(
        encoding
            .styles
            .iter()
            .any(|style| style.flags_and_miter_limit & 0x4000_0000 != 0)
    );
    assert!(encoding.styles.iter().any(|style| {
        style.flags_and_miter_limit & 0x8000_0000 != 0
            && style.flags_and_miter_limit & 0x3000_0000 == 0
            && style.flags_and_miter_limit & 0x0f00_0000 == 0x0a00_0000
    }));
    assert!(encoding.transforms.iter().any(|transform| {
        transform.matrix[0].to_bits() == 0.125_f32.to_bits()
            && transform.translation[0].to_bits() == 13.0_f32.to_bits()
            && transform.translation[1].to_bits() == 24.0_f32.to_bits()
    }));
    let expected_tint = 0xcc_7a_52_29_u32;
    assert!(encoding.draw_data.contains(&expected_tint));
}

#[test]
fn static_and_owned_path_commands_encode_equivalent_geometry_without_copying_static_data() {
    let style = Stroke::new(2.0, Brush::Solid(Color::BLACK))
        .with_cap(StrokeCap::Square)
        .with_join(StrokeJoin::Round);
    let static_path = PathPrimitive::from_static(
        &FILL_ELEMENTS,
        Some(Brush::Solid(Color::WHITE)),
        Some(style),
    )
    .with_fill_rule(FillRule::EvenOdd)
    .with_opacity(0.75);
    let owned_path = PathPrimitive::new(
        FILL_ELEMENTS.to_vec(),
        Some(Brush::Solid(Color::WHITE)),
        Some(style),
    )
    .with_fill_rule(FillRule::EvenOdd)
    .with_opacity(0.75);

    let translation = translate_primitives(
        &[Primitive::Path(static_path), Primitive::Path(owned_path)],
        &RenderResources::new(),
    );

    assert!(translation.diagnostics.is_empty());
    assert_eq!(translation.commands[0].kind, translation.commands[1].kind);
    let RenderCommandKind::Path { elements, .. } = &translation.commands[0].kind else {
        unreachable!()
    };
    assert!(elements.is_static());
    let RenderCommandKind::Path { elements, .. } = &translation.commands[1].kind else {
        unreachable!()
    };
    assert!(!elements.is_static());

    let static_scene = encode(&translation.commands[..1]);
    let owned_scene = encode(&translation.commands[1..]);
    let static_encoding = static_scene.encoding();
    let owned_encoding = owned_scene.encoding();
    assert!(static_encoding.path_tags == owned_encoding.path_tags);
    assert_eq!(static_encoding.path_data, owned_encoding.path_data);
    assert!(static_encoding.draw_tags == owned_encoding.draw_tags);
    assert_eq!(static_encoding.draw_data, owned_encoding.draw_data);
    assert_eq!(static_encoding.transforms, owned_encoding.transforms);
    assert_eq!(static_encoding.styles, owned_encoding.styles);
    assert_eq!(static_encoding.n_clips, 2);
    assert_eq!(static_encoding.n_open_clips, 0);
    assert!(static_encoding.draw_data.contains(&0.75_f32.to_bits()));
}

#[test]
fn vello_style_mapping_covers_fill_caps_and_joins() {
    assert_eq!(vello_fill(FillRule::NonZero), Fill::NonZero);
    assert_eq!(vello_fill(FillRule::EvenOdd), Fill::EvenOdd);

    let butt_miter = vello_stroke(
        Stroke::new(2.0, Brush::Solid(Color::WHITE))
            .with_cap(StrokeCap::Butt)
            .with_join(StrokeJoin::Miter),
        1.0,
    );
    assert_eq!(butt_miter.start_cap, Cap::Butt);
    assert_eq!(butt_miter.end_cap, Cap::Butt);
    assert_eq!(butt_miter.join, Join::Miter);

    let round_bevel = vello_stroke(
        Stroke::new(2.0, Brush::Solid(Color::WHITE))
            .with_cap(StrokeCap::Round)
            .with_join(StrokeJoin::Bevel),
        1.0,
    );
    assert_eq!(round_bevel.start_cap, Cap::Round);
    assert_eq!(round_bevel.end_cap, Cap::Round);
    assert_eq!(round_bevel.join, Join::Bevel);

    let square_round = vello_stroke(
        Stroke::new(2.0, Brush::Solid(Color::WHITE))
            .with_cap(StrokeCap::Square)
            .with_join(StrokeJoin::Round),
        1.0,
    );
    assert_eq!(square_round.start_cap, Cap::Square);
    assert_eq!(square_round.join, Join::Round);
}

#[test]
fn invalid_and_empty_icon_graphics_fail_without_commands() {
    let primitives = [
        Primitive::Icon(IconPrimitive::new(
            StaticIcon::new(IconId::from_raw(1), &INVALID_GRAPHIC),
            Rect::new(0.0, 0.0, 16.0, 16.0),
            Color::WHITE,
        )),
        Primitive::Icon(IconPrimitive::new(
            StaticIcon::new(IconId::from_raw(2), &EMPTY_GRAPHIC),
            Rect::new(0.0, 0.0, 16.0, 16.0),
            Color::WHITE,
        )),
    ];

    let translation = translate_primitives(&primitives, &RenderResources::new());

    assert!(translation.commands.is_empty());
    assert_eq!(
        translation.diagnostics,
        vec![
            RenderDiagnostic::InvalidGeometry("icon_view_box"),
            RenderDiagnostic::InvalidGeometry("icon"),
        ]
    );
}

#[test]
#[allow(clippy::too_many_lines)]
fn invalid_icon_inputs_have_deterministic_diagnostics_and_fallbacks() {
    let resources = RenderResources::new();
    let make_icon = |id, graphic, rect, tint| {
        Primitive::Icon(IconPrimitive::new(
            StaticIcon::new(IconId::from_raw(id), graphic),
            rect,
            tint,
        ))
    };

    let invalid_rect = translate_primitives(
        &[make_icon(
            10,
            &GRAPHIC,
            Rect::new(0.0, 0.0, 0.0, 16.0),
            Color::WHITE,
        )],
        &resources,
    );
    assert!(invalid_rect.commands.is_empty());
    assert_eq!(
        invalid_rect.diagnostics,
        vec![RenderDiagnostic::InvalidGeometry("icon")]
    );

    let invalid_transform = translate_primitives(
        &[
            Primitive::TransformBegin(Transform {
                dx: f32::NAN,
                ..Transform::IDENTITY
            }),
            make_icon(11, &GRAPHIC, Rect::new(0.0, 0.0, 16.0, 16.0), Color::WHITE),
            Primitive::TransformEnd,
        ],
        &resources,
    );
    assert_eq!(
        invalid_transform.diagnostics,
        vec![RenderDiagnostic::InvalidGeometry("transform")]
    );
    assert!(
        invalid_transform
            .commands
            .iter()
            .all(|command| command.transform.dx.is_finite())
    );

    for (graphic, context) in [
        (&EMPTY_PATH_GRAPHIC, "icon_layer"),
        (&EMPTY_ELEMENTS_GRAPHIC, "icon_path"),
        (&INVALID_STROKE_GRAPHIC, "icon_stroke"),
    ] {
        let translation = translate_primitives(
            &[make_icon(
                12,
                graphic,
                Rect::new(0.0, 0.0, 16.0, 16.0),
                Color::WHITE,
            )],
            &resources,
        );
        assert!(translation.commands.is_empty());
        assert_eq!(
            translation.diagnostics,
            vec![RenderDiagnostic::InvalidGeometry(context)]
        );
    }

    let invalid_opacity = translate_primitives(
        &[make_icon(
            13,
            &INVALID_OPACITY_GRAPHIC,
            Rect::new(0.0, 0.0, 16.0, 16.0),
            Color::WHITE,
        )],
        &resources,
    );
    assert_eq!(
        invalid_opacity.diagnostics,
        vec![
            RenderDiagnostic::InvalidGeometry("icon_opacity"),
            RenderDiagnostic::InvalidGeometry("icon_opacity"),
        ]
    );
    assert!(matches!(
        invalid_opacity.commands[0].kind,
        RenderCommandKind::OpacityGroupBegin { opacity: 0.0, .. }
    ));
    assert!(matches!(
        invalid_opacity.commands[1].kind,
        RenderCommandKind::Path { opacity: 1.0, .. }
    ));

    let invalid_tint = translate_primitives(
        &[make_icon(
            14,
            &GRAPHIC,
            Rect::new(0.0, 0.0, 16.0, 16.0),
            Color::rgba(f32::NAN, 0.4, 0.6, 1.0),
        )],
        &resources,
    );
    assert_eq!(
        invalid_tint.diagnostics,
        vec![RenderDiagnostic::InvalidGeometry("icon_tint")]
    );
    let RenderCommandKind::Path {
        fill: Some(Brush::Solid(tint)),
        ..
    } = invalid_tint.commands[1].kind
    else {
        panic!("expected sanitized tinted path")
    };
    assert_eq!(tint, Color::rgba(0.0, 0.4, 0.6, 1.0));
}
