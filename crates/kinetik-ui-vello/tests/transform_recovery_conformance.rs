//! Vello transform-scope recovery conformance tests.

use kinetik_ui_core::{
    Brush, ClipId, Color, CornerRadius, Primitive, Rect, RectPrimitive, Transform, Vec2,
};
use kinetik_ui_vello::{
    RenderDiagnostic, RenderResources, render_translation_snapshot, translate_primitives,
};

fn marker(x: f32) -> Primitive {
    Primitive::Rect(RectPrimitive {
        rect: Rect::new(x, 0.0, 1.0, 1.0),
        fill: Some(Brush::Solid(Color::WHITE)),
        stroke: None,
        radius: CornerRadius::all(0.0),
    })
}

fn invalid_transform() -> Transform {
    Transform {
        m11: f32::INFINITY,
        ..Transform::IDENTITY
    }
}

#[test]
fn rejected_inner_transform_preserves_outer_scope_balance() {
    let outer = Transform::translation(Vec2::new(2.0, 3.0));
    let clip = ClipId::from_raw(1);
    let primitives = vec![
        Primitive::TransformBegin(outer),
        Primitive::TransformBegin(invalid_transform()),
        Primitive::ClipBegin {
            id: clip,
            rect: Rect::new(0.0, 0.0, 5.0, 5.0),
        },
        marker(1.0),
        Primitive::ClipEnd { id: clip },
        Primitive::TransformEnd,
        marker(2.0),
        Primitive::TransformEnd,
        marker(3.0),
    ];

    let translation = translate_primitives(&primitives, &RenderResources::new());

    assert_eq!(
        translation.diagnostics,
        vec![RenderDiagnostic::InvalidGeometry("transform")]
    );
    assert_eq!(translation.commands[0].transform, outer);
    assert_eq!(translation.commands[1].transform, outer);
    assert_eq!(translation.commands[2].transform, Transform::IDENTITY);
    assert_eq!(
        render_translation_snapshot(&translation),
        "commands:\n  0: layer=0 transform=[1.000, 0.000, 0.000, 1.000, 2.000, 3.000] clips=[{rect=(0.000, 0.000, 5.000, 5.000) transform=[1.000, 0.000, 0.000, 1.000, 2.000, 3.000]}] rect rect=(1.000, 0.000, 1.000, 1.000) fill=rgba(1.000, 1.000, 1.000, 1.000) stroke=none radius=(0.000, 0.000, 0.000, 0.000)\n  1: layer=0 transform=[1.000, 0.000, 0.000, 1.000, 2.000, 3.000] clips=[] rect rect=(2.000, 0.000, 1.000, 1.000) fill=rgba(1.000, 1.000, 1.000, 1.000) stroke=none radius=(0.000, 0.000, 0.000, 0.000)\n  2: layer=0 transform=[1.000, 0.000, 0.000, 1.000, 0.000, 0.000] clips=[] rect rect=(3.000, 0.000, 1.000, 1.000) fill=rgba(1.000, 1.000, 1.000, 1.000) stroke=none radius=(0.000, 0.000, 0.000, 0.000)\ndiagnostics:\n  invalid_geometry:transform",
    );
}

#[test]
fn nested_rejections_allow_a_valid_descendant_and_restore_lifo() {
    let outer = Transform::translation(Vec2::new(10.0, 20.0));
    let descendant = Transform::translation(Vec2::new(3.0, 4.0));
    let primitives = vec![
        Primitive::TransformBegin(outer),
        Primitive::TransformBegin(Transform {
            dx: f32::NAN,
            ..Transform::IDENTITY
        }),
        Primitive::TransformBegin(invalid_transform()),
        Primitive::TransformBegin(descendant),
        marker(1.0),
        Primitive::TransformEnd,
        marker(2.0),
        Primitive::TransformEnd,
        marker(3.0),
        Primitive::TransformEnd,
        marker(4.0),
        Primitive::TransformEnd,
        marker(5.0),
    ];

    let translation = translate_primitives(&primitives, &RenderResources::new());

    assert_eq!(
        translation.diagnostics,
        vec![
            RenderDiagnostic::InvalidGeometry("transform"),
            RenderDiagnostic::InvalidGeometry("transform"),
        ]
    );
    assert_eq!(
        translation.commands[0].transform,
        Transform::translation(Vec2::new(13.0, 24.0))
    );
    assert_eq!(translation.commands[1].transform, outer);
    assert_eq!(translation.commands[2].transform, outer);
    assert_eq!(translation.commands[3].transform, outer);
    assert_eq!(translation.commands[4].transform, Transform::IDENTITY);
}

#[test]
fn composition_overflow_is_rejected_but_balanced() {
    let outer = Transform::scale(Vec2::new(f32::MAX, 1.0));
    let primitives = vec![
        Primitive::TransformBegin(outer),
        Primitive::TransformBegin(Transform::scale(Vec2::new(2.0, 1.0))),
        marker(1.0),
        Primitive::TransformEnd,
        marker(2.0),
        Primitive::TransformEnd,
        marker(3.0),
    ];

    let translation = translate_primitives(&primitives, &RenderResources::new());

    assert_eq!(
        translation.diagnostics,
        vec![RenderDiagnostic::InvalidGeometry("transform")]
    );
    assert_eq!(translation.commands[0].transform, outer);
    assert_eq!(translation.commands[1].transform, outer);
    assert_eq!(translation.commands[2].transform, Transform::IDENTITY);
}

#[test]
fn unclosed_rejected_scope_reports_begin_then_eof_diagnostics() {
    let primitives = vec![Primitive::TransformBegin(invalid_transform()), marker(1.0)];

    let translation = translate_primitives(&primitives, &RenderResources::new());

    assert_eq!(
        translation.diagnostics,
        vec![
            RenderDiagnostic::InvalidGeometry("transform"),
            RenderDiagnostic::InvalidGeometry("transform_stack"),
        ]
    );
    assert_eq!(translation.commands[0].transform, Transform::IDENTITY);
}

#[test]
fn unmatched_end_diagnoses_once_and_keeps_identity() {
    let primitives = vec![Primitive::TransformEnd, marker(1.0)];

    let translation = translate_primitives(&primitives, &RenderResources::new());

    assert_eq!(
        translation.diagnostics,
        vec![RenderDiagnostic::InvalidGeometry("transform_stack")]
    );
    assert_eq!(translation.commands[0].transform, Transform::IDENTITY);
}
