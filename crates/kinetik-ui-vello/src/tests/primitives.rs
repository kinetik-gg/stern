use super::common::{assert_approx, resources};
use crate::{RenderCommandKind, RenderDiagnostic, RenderResources, translate_primitives};
use kinetik_ui_core::render::TexturePrimitive;
use kinetik_ui_core::{
    Brush, ClipId, Color, CornerRadius, GradientStop, LinePrimitive, LinearGradient, PathElement,
    PathPrimitive, Point, Primitive, Rect, RectPrimitive, ShadowPrimitive, Size, Stroke, TextureId,
    Transform, Vec2,
};

#[test]
fn translates_rectangles_and_lines_in_order() {
    let primitives = vec![
        Primitive::Rect(RectPrimitive {
            rect: Rect::new(0.0, 0.0, 10.0, 10.0),
            fill: Some(Brush::Solid(Color::WHITE)),
            stroke: Some(Stroke::new(1.0, Brush::Solid(Color::BLACK))),
            radius: CornerRadius::all(0.0),
        }),
        Primitive::Line(LinePrimitive {
            from: Point::new(0.0, 0.0),
            to: Point::new(10.0, 10.0),
            stroke: Stroke::new(1.0, Brush::Solid(Color::WHITE)),
        }),
    ];

    let translation = translate_primitives(&primitives, &RenderResources::new());

    assert!(matches!(
        translation.commands[0].kind,
        RenderCommandKind::Rect { .. }
    ));
    assert!(matches!(
        translation.commands[1].kind,
        RenderCommandKind::Line { .. }
    ));
}

#[test]
fn translates_paths_in_order() {
    let primitives = vec![Primitive::Path(PathPrimitive::new(
        vec![
            PathElement::MoveTo(Point::new(0.0, 0.0)),
            PathElement::LineTo(Point::new(10.0, 0.0)),
            PathElement::QuadTo {
                ctrl: Point::new(12.0, 4.0),
                to: Point::new(10.0, 8.0),
            },
            PathElement::CubicTo {
                ctrl1: Point::new(8.0, 10.0),
                ctrl2: Point::new(2.0, 10.0),
                to: Point::new(0.0, 8.0),
            },
            PathElement::Close,
        ],
        Some(Brush::Solid(Color::WHITE)),
        Some(Stroke::new(1.0, Brush::Solid(Color::BLACK))),
    ))];

    let translation = translate_primitives(&primitives, &RenderResources::new());

    assert!(translation.diagnostics.is_empty());
    let RenderCommandKind::Path {
        elements,
        fill,
        stroke,
    } = &translation.commands[0].kind
    else {
        panic!("expected path command");
    };
    assert_eq!(elements.len(), 5);
    assert_eq!(*fill, Some(Brush::Solid(Color::WHITE)));
    assert_eq!(*stroke, Some(Stroke::new(1.0, Brush::Solid(Color::BLACK))));
}

#[test]
fn translates_linear_gradient_brushes() {
    let gradient = LinearGradient::from_colors(
        Point::new(0.0, 0.0),
        Point::new(20.0, 0.0),
        &[Color::BLACK, Color::rgb(0.5, 0.5, 0.5), Color::WHITE],
    )
    .expect("valid gradient");
    let primitives = vec![Primitive::Rect(RectPrimitive {
        rect: Rect::new(0.0, 0.0, 20.0, 12.0),
        fill: Some(Brush::LinearGradient(gradient)),
        stroke: Some(Stroke::new(1.0, Brush::LinearGradient(gradient))),
        radius: CornerRadius::all(2.0),
    })];

    let translation = translate_primitives(&primitives, &RenderResources::new());

    assert!(translation.diagnostics.is_empty());
    let RenderCommandKind::Rect { fill, stroke, .. } = &translation.commands[0].kind else {
        panic!("expected rect command");
    };
    assert_eq!(*fill, Some(Brush::LinearGradient(gradient)));
    assert_eq!(
        *stroke,
        Some(Stroke::new(1.0, Brush::LinearGradient(gradient)))
    );
}

#[test]
fn translates_shadows_in_order() {
    let shadow = ShadowPrimitive::new(
        Rect::new(2.0, 4.0, 20.0, 12.0),
        Vec2::new(1.0, 3.0),
        8.0,
        2.0,
        5.0,
        Color::rgba(0.0, 0.0, 0.0, 0.35),
    );
    let primitives = vec![Primitive::Shadow(shadow)];

    let translation = translate_primitives(&primitives, &RenderResources::new());

    assert!(translation.diagnostics.is_empty());
    let RenderCommandKind::Shadow {
        rect,
        offset,
        blur_radius,
        spread,
        radius,
        color,
    } = &translation.commands[0].kind
    else {
        panic!("expected shadow command");
    };
    assert_eq!(*rect, shadow.rect);
    assert_eq!(*offset, shadow.offset);
    assert_approx(*blur_radius, 8.0);
    assert_approx(*spread, 2.0);
    assert_approx(*radius, 5.0);
    assert_eq!(*color, shadow.color);
}

#[test]
fn sanitizes_linear_gradient_stops_before_encoding() {
    let gradient = LinearGradient::new(
        Point::new(0.0, 0.0),
        Point::new(20.0, 0.0),
        &[
            GradientStop::new(1.0, Color::WHITE),
            GradientStop::new(f32::NAN, Color::rgba(f32::NAN, 0.25, 0.5, 1.0)),
            GradientStop::new(-0.25, Color::BLACK),
        ],
    )
    .expect("valid stop count");
    let primitives = vec![Primitive::Rect(RectPrimitive {
        rect: Rect::new(0.0, 0.0, 20.0, 12.0),
        fill: Some(Brush::LinearGradient(gradient)),
        stroke: None,
        radius: CornerRadius::all(0.0),
    })];

    let translation = translate_primitives(&primitives, &RenderResources::new());

    assert_eq!(
        translation.diagnostics,
        vec![
            RenderDiagnostic::InvalidGeometry("rect_fill"),
            RenderDiagnostic::InvalidGeometry("rect_fill"),
            RenderDiagnostic::InvalidGeometry("rect_fill"),
        ]
    );
    let RenderCommandKind::Rect {
        fill: Some(Brush::LinearGradient(gradient)),
        ..
    } = &translation.commands[0].kind
    else {
        panic!("expected sanitized gradient fill");
    };
    assert_approx(gradient.stops()[0].offset, 0.0);
    assert_approx(gradient.stops()[1].offset, 0.0);
    assert_approx(gradient.stops()[2].offset, 1.0);
    assert_eq!(gradient.stops()[0].color, Color::rgba(0.0, 0.25, 0.5, 1.0));
}

#[test]
fn invalid_linear_gradient_endpoint_falls_back_to_solid_brush() {
    let gradient = LinearGradient::between(
        Point::new(f32::NAN, 0.0),
        Point::new(20.0, 0.0),
        Color::WHITE,
        Color::BLACK,
    );
    let primitives = vec![Primitive::Rect(RectPrimitive {
        rect: Rect::new(0.0, 0.0, 20.0, 12.0),
        fill: Some(Brush::LinearGradient(gradient)),
        stroke: None,
        radius: CornerRadius::all(0.0),
    })];

    let translation = translate_primitives(&primitives, &RenderResources::new());

    assert_eq!(
        translation.diagnostics,
        vec![RenderDiagnostic::InvalidGeometry("rect_fill")]
    );
    let RenderCommandKind::Rect {
        fill: Some(Brush::Solid(color)),
        ..
    } = &translation.commands[0].kind
    else {
        panic!("expected solid fallback");
    };
    assert_eq!(*color, Color::WHITE);
}

#[test]
fn invalid_shadow_geometry_is_diagnosed_and_sanitized() {
    let primitives = vec![Primitive::Shadow(ShadowPrimitive::new(
        Rect::new(f32::NAN, 2.0, 20.0, 12.0),
        Vec2::new(f32::NAN, 3.0),
        -4.0,
        f32::NAN,
        -2.0,
        Color::rgba(f32::NAN, 0.0, 0.0, 0.25),
    ))];

    let translation = translate_primitives(&primitives, &RenderResources::new());

    assert_eq!(
        translation.diagnostics,
        vec![
            RenderDiagnostic::InvalidGeometry("shadow"),
            RenderDiagnostic::InvalidGeometry("shadow_offset"),
            RenderDiagnostic::InvalidGeometry("shadow_blur"),
            RenderDiagnostic::InvalidGeometry("shadow_spread"),
            RenderDiagnostic::InvalidGeometry("shadow_radius"),
            RenderDiagnostic::InvalidGeometry("shadow_color"),
        ]
    );
    let RenderCommandKind::Shadow {
        rect,
        offset,
        blur_radius,
        spread,
        radius,
        color,
    } = &translation.commands[0].kind
    else {
        panic!("expected sanitized shadow");
    };
    assert_approx(rect.x, 0.0);
    assert_eq!(*offset, Vec2::new(0.0, 3.0));
    assert_approx(*blur_radius, 0.0);
    assert_approx(*spread, 0.0);
    assert_approx(*radius, 0.0);
    assert_approx(color.r, 0.0);
}

#[test]
fn shadow_spread_that_erases_rect_is_diagnosed_and_skipped() {
    let primitives = vec![Primitive::Shadow(ShadowPrimitive::new(
        Rect::new(0.0, 0.0, 10.0, 10.0),
        Vec2::ZERO,
        0.0,
        -6.0,
        0.0,
        Color::BLACK,
    ))];

    let translation = translate_primitives(&primitives, &RenderResources::new());

    assert_eq!(
        translation.diagnostics,
        vec![RenderDiagnostic::InvalidGeometry("shadow_spread")]
    );
    assert!(translation.commands.is_empty());
}

#[test]
fn invalid_geometry_is_diagnosed_and_sanitized_before_encoding() {
    let primitives = vec![
        Primitive::Rect(RectPrimitive {
            rect: Rect::new(0.0, 0.0, -10.0, 10.0),
            fill: Some(Brush::Solid(Color::WHITE)),
            stroke: None,
            radius: CornerRadius::all(0.0),
        }),
        Primitive::Line(LinePrimitive {
            from: Point::new(f32::NAN, 0.0),
            to: Point::new(10.0, 10.0),
            stroke: Stroke::new(1.0, Brush::Solid(Color::WHITE)),
        }),
        Primitive::Path(PathPrimitive::new(
            vec![PathElement::MoveTo(Point::new(f32::NAN, 0.0))],
            Some(Brush::Solid(Color::WHITE)),
            None,
        )),
        Primitive::ClipBegin {
            id: ClipId::from_raw(9),
            rect: Rect::new(0.0, 0.0, f32::NAN, 10.0),
        },
        Primitive::TransformBegin(Transform {
            dx: f32::INFINITY,
            ..Transform::IDENTITY
        }),
        Primitive::Rect(RectPrimitive {
            rect: Rect::new(f32::NAN, 2.0, 10.0, 10.0),
            fill: Some(Brush::Solid(Color::rgba(f32::NAN, 0.5, 0.5, 1.0))),
            stroke: Some(Stroke::new(-1.0, Brush::Solid(Color::WHITE))),
            radius: CornerRadius::all(-3.0),
        }),
    ];

    let translation = translate_primitives(&primitives, &RenderResources::new());

    assert_eq!(
        translation.diagnostics,
        vec![
            RenderDiagnostic::InvalidGeometry("rect"),
            RenderDiagnostic::InvalidGeometry("line"),
            RenderDiagnostic::InvalidGeometry("path"),
            RenderDiagnostic::InvalidGeometry("clip"),
            RenderDiagnostic::InvalidGeometry("transform"),
            RenderDiagnostic::InvalidGeometry("rect"),
            RenderDiagnostic::InvalidGeometry("rect_fill"),
            RenderDiagnostic::InvalidGeometry("rect_stroke"),
            RenderDiagnostic::InvalidGeometry("rect_radius"),
            RenderDiagnostic::InvalidGeometry("transform_stack"),
        ]
    );
    assert_eq!(translation.commands.len(), 1);
    assert_eq!(translation.commands[0].transform, Transform::IDENTITY);
    assert!(translation.commands[0].clips.is_empty());
    let RenderCommandKind::Rect {
        rect,
        fill,
        stroke,
        radius,
    } = &translation.commands[0].kind
    else {
        panic!("expected sanitized rect command");
    };
    assert_approx(rect.x, 0.0);
    assert_approx(rect.y, 2.0);
    assert!(stroke.is_none());
    assert_approx(radius.top_left, 0.0);
    let Some(Brush::Solid(color)) = fill else {
        panic!("expected solid fill");
    };
    assert_approx(color.r, 0.0);
    assert_approx(color.g, 0.5);
}

#[test]
fn invalid_empty_paths_are_diagnosed_and_skipped() {
    let primitives = vec![Primitive::Path(PathPrimitive::new(
        Vec::new(),
        Some(Brush::Solid(Color::WHITE)),
        None,
    ))];

    let translation = translate_primitives(&primitives, &RenderResources::new());

    assert_eq!(
        translation.diagnostics,
        vec![RenderDiagnostic::InvalidGeometry("path")]
    );
    assert!(translation.commands.is_empty());
}

#[test]
fn invalid_texture_source_size_is_diagnosed_and_dropped() {
    let primitives = vec![Primitive::Texture(TexturePrimitive {
        texture: TextureId::from_raw(2),
        rect: Rect::new(0.0, 0.0, 10.0, 10.0),
        source_size: Size::new(f32::NAN, 10.0),
    })];

    let translation = translate_primitives(&primitives, &resources());

    assert_eq!(
        translation.diagnostics,
        vec![RenderDiagnostic::InvalidGeometry("texture_source_size")]
    );
    assert!(translation.commands.is_empty());
}
