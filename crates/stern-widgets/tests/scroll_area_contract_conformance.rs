//! Public scroll-area extent, clamping, staging, and ownership conformance.

use stern_core::{
    ClipId, FrameOutput, LayoutItem, Measurement, Point, PointerButtonState, PointerInput,
    PointerOrder, PointerTarget, Primitive, Rect, RepaintRequest, Response, Size, SizeRule,
    Transform, UiInput, UiMemory, Vec2, WidgetId, clamp_scroll_offset, default_dark_theme,
    inspect_primitives, max_scroll_offset,
};
use stern_widgets::{ScrollAreaOutput, Ui};

const WHEEL_VIEWPORT: Rect = Rect::new(0.0, 0.0, 80.0, 20.0);
const WHEEL_OLD_PANEL: Rect = Rect::new(0.0, 10.0, 30.0, 10.0);
const WHEEL_NEXT_PANEL: Rect = Rect::new(40.0, 30.0, 40.0, 10.0);

const OUTER_RECT: Rect = Rect::new(0.0, 0.0, 100.0, 80.0);
const INNER_RECT: Rect = Rect::new(20.0, 40.0, 60.0, 50.0);
const CLIPPED_RECT: Rect = Rect::new(30.0, 100.0, 20.0, 20.0);
const ONSCREEN_RECT: Rect = Rect::new(30.0, 60.0, 20.0, 20.0);

fn fixed_item(width: f32, height: f32) -> LayoutItem {
    LayoutItem::new(
        SizeRule::Fixed(width),
        SizeRule::Fixed(height),
        Measurement::default(),
    )
}

fn wheel_input(delta: Vec2) -> UiInput {
    UiInput {
        pointer: PointerInput {
            position: Some(Point::new(10.0, 10.0)),
            wheel_delta: delta,
            ..PointerInput::default()
        },
        window_focused: true,
        ..UiInput::default()
    }
}

fn click_input(position: Point) -> UiInput {
    UiInput {
        pointer: PointerInput {
            position: Some(position),
            primary: PointerButtonState::new(false, true, true),
            ..PointerInput::default()
        },
        window_focused: true,
        ..UiInput::default()
    }
}

fn wheel_frame(memory: &mut UiMemory, input: &UiInput) -> (ScrollAreaOutput<Vec2>, FrameOutput) {
    let theme = default_dark_theme();
    let area_id = WidgetId::from_key("root").child("wheel-area");
    let mut ui = Ui::new(input, memory, &theme);
    let frame_offset = ui.memory().scroll_offset(area_id);
    ui.resolve_pointer_targets(|plan| {
        plan.target(PointerTarget::wheel_only(
            area_id,
            WHEEL_VIEWPORT,
            PointerOrder::new(10),
        ));
    })
    .expect("unique wheel target");
    let area = ui.scroll_area(
        "wheel-area",
        WHEEL_VIEWPORT,
        Size::new(80.0, 80.0),
        false,
        |ui, offset| {
            assert_eq!(offset, frame_offset);
            ui.panel_keyed("old-panel", WHEEL_OLD_PANEL);
            ui.panel_keyed("next-panel", WHEEL_NEXT_PANEL);
            offset
        },
    );
    let frame = ui.finish_output();
    (area, frame)
}

type NestedOutput = ScrollAreaOutput<ScrollAreaOutput<(Vec2, Vec2, Response, Response)>>;

fn nested_frame(memory: &mut UiMemory, input: &UiInput) -> (NestedOutput, FrameOutput) {
    let theme = default_dark_theme();
    let mut ui = Ui::new(input, memory, &theme);
    let nested = ui.scroll_area(
        "nested-outer",
        OUTER_RECT,
        Size::new(200.0, 200.0),
        false,
        |ui, outer_offset| {
            ui.scroll_area(
                "nested-inner",
                INNER_RECT,
                Size::new(120.0, 150.0),
                false,
                |ui, inner_offset| {
                    let clipped = ui.button("clipped", CLIPPED_RECT, "Clipped", false);
                    let onscreen = ui.button("onscreen", ONSCREEN_RECT, "Onscreen", false);
                    (outer_offset, inner_offset, clipped, onscreen)
                },
            )
        },
    );
    let frame = ui.finish_output();
    (nested, frame)
}

#[test]
fn scroll_area_exposes_exact_viewport_extent_offset_and_maximum() {
    let viewport = Rect::new(10.0, 20.0, 80.0, 50.0);
    let content = Size::new(200.0, 140.0);
    let retained = Vec2::new(35.0, 25.0);
    let area_id = WidgetId::from_key("root").child("extent-area");
    let input = UiInput::default();
    let mut memory = UiMemory::new();
    memory.set_scroll_offset(area_id, retained);
    let theme = default_dark_theme();
    let mut ui = Ui::new(&input, &mut memory, &theme);

    let area = ui.scroll_area("extent-area", viewport, content, false, |_ui, offset| {
        (offset, String::from("logical extent retained"), 7_usize)
    });
    let frame = ui.finish_output();

    assert_eq!(area.scroll.response.id, area_id);
    assert_eq!(area.scroll.response.rect, viewport);
    assert_eq!(area.scroll.offset, retained);
    assert_eq!(area.scroll.delta, Vec2::ZERO);
    assert_eq!(area.scroll.max_offset, Vec2::new(120.0, 90.0));
    assert_eq!(area.inner.0, retained);
    assert_eq!(area.inner.1, "logical extent retained");
    assert_eq!(area.inner.2, 7);
    assert_eq!(memory.scroll_offset(area_id), retained);
    assert_eq!(
        frame
            .semantics
            .get(area_id)
            .expect("scroll semantics")
            .bounds,
        viewport
    );
    assert!(frame.warnings.is_empty());
}

#[test]
fn scroll_row_and_column_expose_deterministic_axis_policy() {
    let root = WidgetId::from_key("root");
    let row_id = root.child("row-policy");
    let column_id = root.child("column-policy");
    let generic_id = root.child("two-axis-policy");
    let mut memory = UiMemory::new();
    memory.set_scroll_offset(row_id, Vec2::new(999.0, 999.0));
    memory.set_scroll_offset(column_id, Vec2::new(999.0, 999.0));
    memory.set_scroll_offset(generic_id, Vec2::new(13.0, 17.0));
    let input = UiInput::default();
    let theme = default_dark_theme();
    let items = [fixed_item(30.0, 30.0), fixed_item(30.0, 30.0)];
    let mut ui = Ui::new(&input, &mut memory, &theme);

    let row = ui.scroll_row(
        "row-policy",
        Rect::new(0.0, 0.0, 40.0, 40.0),
        &items,
        4.0,
        false,
        |_ui, _, rect| rect,
    );
    let column = ui.scroll_column(
        "column-policy",
        Rect::new(60.0, 0.0, 40.0, 40.0),
        &items,
        4.0,
        false,
        |_ui, _, rect| rect,
    );
    let generic = ui.scroll_area(
        "two-axis-policy",
        Rect::new(120.0, 0.0, 40.0, 40.0),
        Size::new(100.0, 90.0),
        false,
        |_ui, offset| offset,
    );
    let frame = ui.finish_output();

    assert_eq!(row.scroll.offset, Vec2::new(24.0, 0.0));
    assert_eq!(row.scroll.max_offset, Vec2::new(24.0, 0.0));
    assert_eq!(row.inner[1], Rect::new(34.0, 0.0, 30.0, 30.0));
    assert_eq!(column.scroll.offset, Vec2::new(0.0, 24.0));
    assert_eq!(column.scroll.max_offset, Vec2::new(0.0, 24.0));
    assert_eq!(column.inner[1], Rect::new(60.0, 34.0, 30.0, 30.0));
    assert_eq!(generic.scroll.offset, Vec2::new(13.0, 17.0));
    assert_eq!(generic.scroll.max_offset, Vec2::new(60.0, 50.0));
    assert_eq!(generic.inner, Vec2::new(13.0, 17.0));
    assert_eq!(memory.scroll_offset(row_id), Vec2::new(24.0, 0.0));
    assert_eq!(memory.scroll_offset(column_id), Vec2::new(0.0, 24.0));
    assert_eq!(memory.scroll_offset(generic_id), Vec2::new(13.0, 17.0));
    assert!(frame.warnings.is_empty());
}

#[test]
fn scroll_clamping_sanitizes_invalid_and_oversized_state() {
    let viewport = Size::new(40.0, 30.0);
    let content = Size::new(100.0, 80.0);

    assert_eq!(
        clamp_scroll_offset(Vec2::new(-10.0, -5.0), viewport, content),
        Vec2::ZERO
    );
    assert_eq!(
        clamp_scroll_offset(Vec2::new(f32::NAN, f32::INFINITY), viewport, content,),
        Vec2::ZERO
    );
    assert_eq!(
        clamp_scroll_offset(Vec2::new(999.0, 999.0), viewport, content),
        Vec2::new(60.0, 50.0)
    );
    assert_eq!(
        max_scroll_offset(Size::ZERO, Size::new(25.0, 35.0)),
        Vec2::new(25.0, 35.0)
    );
    assert_eq!(
        max_scroll_offset(Size::new(100.0, 90.0), Size::new(20.0, 30.0)),
        Vec2::ZERO
    );
    assert_eq!(
        max_scroll_offset(
            Size::new(f32::NAN, f32::NEG_INFINITY),
            Size::new(f32::INFINITY, 50.0),
        ),
        Vec2::new(0.0, 50.0)
    );

    let root = WidgetId::from_key("root");
    let zero_id = root.child("zero-viewport");
    let smaller_id = root.child("smaller-content");
    let mut memory = UiMemory::new();
    memory.set_scroll_offset(zero_id, Vec2::new(f32::NAN, f32::INFINITY));
    memory.set_scroll_offset(smaller_id, Vec2::new(999.0, 999.0));
    let input = UiInput::default();
    let theme = default_dark_theme();
    let mut ui = Ui::new(&input, &mut memory, &theme);

    let zero = ui.scroll_area(
        "zero-viewport",
        Rect::new(0.0, 0.0, 0.0, 0.0),
        Size::new(25.0, 35.0),
        false,
        |_ui, offset| offset,
    );
    let smaller = ui.scroll_area(
        "smaller-content",
        Rect::new(10.0, 10.0, 100.0, 90.0),
        Size::new(20.0, 30.0),
        false,
        |_ui, offset| offset,
    );
    let frame = ui.finish_output();

    assert_eq!(zero.scroll.offset, Vec2::ZERO);
    assert_eq!(zero.scroll.max_offset, Vec2::new(25.0, 35.0));
    assert_eq!(zero.inner, Vec2::ZERO);
    assert_eq!(smaller.scroll.offset, Vec2::ZERO);
    assert_eq!(smaller.scroll.max_offset, Vec2::ZERO);
    assert_eq!(smaller.inner, Vec2::ZERO);
    assert_eq!(memory.scroll_offset(zero_id), Vec2::ZERO);
    assert_eq!(memory.scroll_offset(smaller_id), Vec2::ZERO);
    for component in [
        zero.scroll.offset.x,
        zero.scroll.offset.y,
        zero.scroll.max_offset.x,
        zero.scroll.max_offset.y,
        smaller.scroll.offset.x,
        smaller.scroll.offset.y,
    ] {
        assert!(component.is_finite());
    }
    assert!(frame.warnings.is_empty());
}

#[test]
fn wheel_staging_preserves_one_frame_scroll_snapshot() {
    let area_id = WidgetId::from_key("root").child("wheel-area");
    let mut memory = UiMemory::new();
    memory.set_scroll_offset(area_id, Vec2::new(0.0, 5.0));

    let (staged, staged_frame) = wheel_frame(&mut memory, &wheel_input(Vec2::new(0.0, -20.0)));
    assert_eq!(staged.scroll.response.id, area_id);
    assert_eq!(staged.scroll.offset, Vec2::new(0.0, 25.0));
    assert_eq!(staged.scroll.delta, Vec2::new(0.0, 20.0));
    assert_eq!(staged.inner, Vec2::new(0.0, 5.0));
    assert_eq!(memory.scroll_offset(area_id), Vec2::new(0.0, 25.0));
    assert_eq!(staged_frame.repaint, RepaintRequest::NextFrame);
    let staged_primitives = inspect_primitives(&staged_frame.primitives);
    assert!(
        staged_primitives
            .iter()
            .any(|item| item.bounds == Some(Rect::new(0.0, 5.0, 30.0, 10.0)))
    );
    assert!(
        !staged_primitives
            .iter()
            .any(|item| item.bounds == Some(Rect::new(40.0, 5.0, 40.0, 10.0)))
    );

    let (consumed, consumed_frame) = wheel_frame(&mut memory, &wheel_input(Vec2::ZERO));
    assert_eq!(consumed.scroll.offset, Vec2::new(0.0, 25.0));
    assert_eq!(consumed.scroll.delta, Vec2::ZERO);
    assert_eq!(consumed.inner, Vec2::new(0.0, 25.0));
    assert_eq!(memory.scroll_offset(area_id), Vec2::new(0.0, 25.0));
    assert_eq!(consumed_frame.repaint, RepaintRequest::None);
    let consumed_primitives = inspect_primitives(&consumed_frame.primitives);
    assert!(
        !consumed_primitives
            .iter()
            .any(|item| item.bounds == Some(Rect::new(0.0, -15.0, 30.0, 10.0)))
    );
    assert!(
        consumed_primitives
            .iter()
            .any(|item| item.bounds == Some(Rect::new(40.0, 5.0, 40.0, 10.0)))
    );
    assert!(staged_frame.warnings.is_empty());
    assert!(consumed_frame.warnings.is_empty());
}

#[test]
#[allow(clippy::too_many_lines)]
fn nested_scroll_areas_keep_independent_extents_clips_and_semantics() {
    let root = WidgetId::from_key("root");
    let outer_id = root.child("nested-outer");
    let outer_scope = root.child(("scroll_area_content", outer_id.raw()));
    let inner_id = outer_scope.child("nested-inner");
    let inner_scope = outer_scope.child(("scroll_area_content", inner_id.raw()));
    let clipped_id = inner_scope.child("clipped");
    let onscreen_id = inner_scope.child("onscreen");
    let outer_offset = Vec2::new(10.0, 20.0);
    let inner_offset = Vec2::new(5.0, 7.0);
    let onscreen_bounds = Rect::new(15.0, 33.0, 20.0, 20.0);
    let clipped_bounds = Rect::new(15.0, 73.0, 20.0, 20.0);
    let mut memory = UiMemory::new();
    memory.set_scroll_offset(outer_id, outer_offset);
    memory.set_scroll_offset(inner_id, inner_offset);
    let input = click_input(Point::new(25.0, 43.0));
    let (nested, frame) = nested_frame(&mut memory, &input);
    assert_eq!(nested.scroll.response.id, outer_id);
    assert_eq!(nested.scroll.offset, outer_offset);
    assert_eq!(nested.scroll.max_offset, Vec2::new(100.0, 120.0));
    assert_eq!(nested.inner.scroll.response.id, inner_id);
    assert_eq!(nested.inner.scroll.offset, inner_offset);
    assert_eq!(nested.inner.scroll.max_offset, Vec2::new(60.0, 100.0));
    assert_eq!(nested.inner.inner.0, outer_offset);
    assert_eq!(nested.inner.inner.1, inner_offset);
    assert_eq!(nested.inner.inner.2.id, clipped_id);
    assert_eq!(nested.inner.inner.3.id, onscreen_id);
    assert!(nested.inner.inner.3.clicked);
    assert_eq!(memory.scroll_offset(outer_id), outer_offset);
    assert_eq!(memory.scroll_offset(inner_id), inner_offset);

    let structural: Vec<_> = frame
        .primitives
        .iter()
        .filter(|primitive| {
            matches!(
                primitive,
                Primitive::ClipBegin { .. }
                    | Primitive::ClipEnd { .. }
                    | Primitive::TransformBegin(_)
                    | Primitive::TransformEnd
            )
        })
        .cloned()
        .collect();
    assert_eq!(
        structural,
        vec![
            Primitive::ClipBegin {
                id: ClipId::from_raw(outer_id.raw()),
                rect: OUTER_RECT,
            },
            Primitive::TransformBegin(Transform::translation(Vec2::new(-10.0, -20.0))),
            Primitive::ClipBegin {
                id: ClipId::from_raw(inner_id.raw()),
                rect: INNER_RECT,
            },
            Primitive::TransformBegin(Transform::translation(Vec2::new(-5.0, -7.0))),
            Primitive::TransformEnd,
            Primitive::ClipEnd {
                id: ClipId::from_raw(inner_id.raw()),
            },
            Primitive::TransformEnd,
            Primitive::ClipEnd {
                id: ClipId::from_raw(outer_id.raw()),
            },
        ]
    );
    let inspected = inspect_primitives(&frame.primitives);
    assert!(
        inspected
            .iter()
            .any(|item| item.bounds == Some(onscreen_bounds))
    );
    assert!(
        !inspected
            .iter()
            .any(|item| item.bounds == Some(clipped_bounds))
    );
    let clipped_semantic = frame.semantics.get(clipped_id).expect("clipped semantics");
    assert_eq!(clipped_semantic.bounds, Rect::ZERO);
    assert!(!clipped_semantic.focusable);
    let onscreen_semantic = frame
        .semantics
        .get(onscreen_id)
        .expect("onscreen semantics");
    assert_eq!(onscreen_semantic.bounds, onscreen_bounds);
    assert!(onscreen_semantic.focusable);
    assert!(frame.warnings.is_empty());

    let clipped_probe = Point::new(25.0, 75.0);
    assert!(clipped_bounds.contains_point(clipped_probe));
    assert!(OUTER_RECT.contains_point(clipped_probe));
    assert!(!Rect::new(10.0, 20.0, 60.0, 50.0).contains_point(clipped_probe));
    let (probe, probe_frame) = nested_frame(&mut memory, &click_input(clipped_probe));
    let probe_clipped = &probe.inner.inner.2;
    assert_eq!(probe.scroll.response.id, outer_id);
    assert_eq!(probe.inner.scroll.response.id, inner_id);
    assert_eq!(probe_clipped.id, clipped_id);
    assert!(!probe_clipped.state.hovered);
    assert!(!probe_clipped.state.pressed);
    assert!(!probe_clipped.clicked);
    assert_eq!(probe.inner.inner.3.id, onscreen_id);
    assert!(probe_frame.actions.is_empty());
    let probe_semantics = &probe_frame.semantics;
    let clipped_node = probe_semantics.get(clipped_id).expect("stable clipped");
    let onscreen_node = probe_semantics.get(onscreen_id).expect("stable onscreen");
    assert_eq!(clipped_node.bounds, Rect::ZERO);
    assert_eq!(onscreen_node.bounds, onscreen_bounds);
    assert!(probe_frame.warnings.is_empty());
}
