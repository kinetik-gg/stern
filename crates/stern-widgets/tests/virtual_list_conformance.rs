//! Public fixed-height virtual-list composition conformance tests.

#![allow(clippy::float_cmp)]

use std::time::Duration;

use stern_core::{
    Brush, Color, FrameContext, Key, KeyEvent, KeyState, KeyboardInput, Modifiers, PathElement,
    PhysicalSize, Point, PointerButtonState, PointerInput, PointerOrder, PointerTarget, Primitive,
    Rect, RepaintRequest, Response, ScaleFactor, SemanticRole, Size, TimeInfo, Transform, UiInput,
    UiMemory, Vec2, ViewportInfo, WidgetId, default_dark_theme,
};
use stern_widgets::{
    CollectionCursor, CollectionProjection, ItemId, Selection, Ui, VirtualListConfig,
    VirtualListOutput, VirtualListRow, VirtualListSelectionMode,
};

const BOUNDS: Rect = Rect::new(0.0, 0.0, 120.0, 60.0);
const LOWER: Rect = Rect::new(0.0, 0.0, 160.0, 100.0);

fn id(raw: u64) -> ItemId {
    ItemId::from_raw(raw)
}

fn projection(raw_ids: &[u64]) -> CollectionProjection {
    CollectionProjection::from_source_ids(&raw_ids.iter().copied().map(id).collect::<Vec<_>>())
}

fn config() -> VirtualListConfig {
    VirtualListConfig::new(BOUNDS, 20.0)
        .label("Assets")
        .overscan(1)
        .selection_mode(VirtualListSelectionMode::Multiple)
}

fn context(input: UiInput) -> FrameContext {
    FrameContext::new(
        ViewportInfo::new(
            Size::new(320.0, 240.0),
            PhysicalSize::new(320, 240),
            ScaleFactor::ONE,
        ),
        input,
        TimeInfo::new(Duration::from_millis(500), Duration::from_millis(16), 1),
    )
}

fn pointer_input(x: f32, y: f32, pressed: bool, released: bool, modifiers: Modifiers) -> UiInput {
    UiInput {
        pointer: PointerInput {
            position: Some(Point::new(x, y)),
            primary: PointerButtonState::new(pressed, pressed, released),
            ..PointerInput::default()
        },
        keyboard: KeyboardInput {
            modifiers,
            events: Vec::new(),
        },
        ..UiInput::default()
    }
}

fn wheel_input(delta_y: f32) -> UiInput {
    UiInput {
        pointer: PointerInput {
            position: Some(Point::new(10.0, 10.0)),
            wheel_delta: Vec2::new(0.0, delta_y),
            ..PointerInput::default()
        },
        ..UiInput::default()
    }
}

fn key_input(key: Key, modifiers: Modifiers, repeat: bool) -> UiInput {
    UiInput {
        keyboard: KeyboardInput {
            modifiers,
            events: vec![KeyEvent::new(key, KeyState::Pressed, modifiers, repeat)],
        },
        ..UiInput::default()
    }
}

struct Run {
    list_id: WidgetId,
    lower: Option<Response>,
    output: VirtualListOutput,
    callbacks: Vec<ItemId>,
    frame: stern_core::FrameOutput,
}

fn run_frame(
    projection: &CollectionProjection,
    config: VirtualListConfig,
    cursor: &mut CollectionCursor,
    selection: &mut Selection,
    memory: &mut UiMemory,
    input: UiInput,
    lower: bool,
) -> Run {
    let theme = default_dark_theme();
    let mut ui = Ui::begin_frame(context(input), memory, &theme);
    let list = ui
        .prepare_virtual_list("list", config, projection)
        .expect("valid list");
    let list_id = list.widget_id();
    let lower_id = ui.make_id("lower");
    ui.resolve_pointer_targets(|plan| {
        if lower {
            plan.target(PointerTarget::new(lower_id, LOWER, PointerOrder::new(10)));
        }
        list.declare_pointer_targets(plan, PointerOrder::new(100));
    })
    .expect("valid shared pointer plan");
    let lower_response = lower.then(|| ui.pressable("lower", LOWER, false));
    let mut callbacks = Vec::new();
    let output = ui.virtual_list(&list, cursor, selection, |item| {
        callbacks.push(item.id);
        VirtualListRow::new(format!("Row {}", item.id.raw()))
    });
    let frame = ui.finish_output();
    Run {
        list_id,
        lower: lower_response,
        output,
        callbacks,
        frame,
    }
}

fn path_bounds(elements: &[PathElement]) -> Rect {
    let mut min_x = f32::INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut max_y = f32::NEG_INFINITY;
    for point in elements.iter().flat_map(|element| match *element {
        PathElement::MoveTo(point) | PathElement::LineTo(point) => vec![point],
        PathElement::QuadTo { ctrl, to } => vec![ctrl, to],
        PathElement::CubicTo { ctrl1, ctrl2, to } => vec![ctrl1, ctrl2, to],
        PathElement::Close => Vec::new(),
    }) {
        min_x = min_x.min(point.x);
        min_y = min_y.min(point.y);
        max_x = max_x.max(point.x);
        max_y = max_y.max(point.y);
    }
    Rect::new(min_x, min_y, max_x - min_x, max_y - min_y)
}

fn assert_virtual_row_focus(frame: &stern_core::FrameOutput, rect: Rect) -> usize {
    let theme = default_dark_theme();
    let base_index = frame
        .primitives
        .iter()
        .position(|primitive| matches!(primitive, Primitive::Rect(base) if base.rect == rect))
        .expect("virtual row base");
    let Primitive::Rect(base) = &frame.primitives[base_index] else {
        unreachable!()
    };
    assert_eq!(base.radius, theme.radii.none);
    assert_eq!(
        base.stroke.expect("neutral row boundary").brush,
        Brush::Solid(theme.colors.border.subtle)
    );
    assert_eq!(
        base.stroke.expect("neutral row boundary").width,
        theme.strokes.hairline
    );
    let expected = theme
        .focus_ring(true)
        .expect("focus recipe")
        .inward_annulus_primitives(rect, base.radius, base.stroke.unwrap().width);
    assert_eq!(frame.primitives[base_index + 1], expected[0]);
    assert_eq!(frame.primitives[base_index + 2], expected[1]);
    for primitive in &frame.primitives[base_index + 1..=base_index + 2] {
        let Primitive::Path(path) = primitive else {
            panic!("virtual-list focus must remain a compound path");
        };
        assert_eq!(path.elements.len(), 20);
        assert_eq!(path.stroke, None);
        assert!(rect.contains_rect(path_bounds(&path.elements)));
    }
    assert!(matches!(
        frame.primitives[base_index + 3],
        Primitive::Text(_)
    ));
    base_index
}

fn linear_channel(channel: f32) -> f32 {
    if channel <= 0.040_45 {
        channel / 12.92
    } else {
        ((channel + 0.055) / 1.055).powf(2.4)
    }
}

fn contrast_ratio(foreground: Color, background: Color) -> f32 {
    let luminance = |color: Color| {
        0.2126 * linear_channel(color.r)
            + 0.7152 * linear_channel(color.g)
            + 0.0722 * linear_channel(color.b)
    };
    let foreground = luminance(foreground);
    let background = luminance(background);
    (foreground.max(background) + 0.05) / (foreground.min(background) + 0.05)
}

fn primitive_without_focus_paths(frame: &stern_core::FrameOutput) -> Vec<Primitive> {
    frame
        .primitives
        .iter()
        .filter(|primitive| !matches!(primitive, Primitive::Path(_)))
        .cloned()
        .collect()
}

#[allow(clippy::cast_precision_loss)]
fn click_row(
    row: usize,
    modifiers: Modifiers,
    projection: &CollectionProjection,
    cursor: &mut CollectionCursor,
    selection: &mut Selection,
    memory: &mut UiMemory,
) -> Run {
    let y = row as f32 * 20.0 + 10.0;
    let _ = run_frame(
        projection,
        config(),
        cursor,
        selection,
        memory,
        pointer_input(10.0, y, true, false, modifiers),
        false,
    );
    run_frame(
        projection,
        config(),
        cursor,
        selection,
        memory,
        pointer_input(10.0, y, false, true, modifiers),
        false,
    )
}

#[test]
fn ten_thousand_rows_materialize_only_the_bounded_window() {
    let items = projection(&(0..10_000).collect::<Vec<_>>());
    let mut cursor = CollectionCursor::new();
    let mut selection = Selection::new();
    let mut memory = UiMemory::new();

    let run = run_frame(
        &items,
        config(),
        &mut cursor,
        &mut selection,
        &mut memory,
        UiInput::default(),
        false,
    );

    assert_eq!(run.output.window.visible_range, 0..3);
    assert_eq!(run.output.window.materialized_range, 0..5);
    assert_eq!(run.callbacks, vec![id(0), id(1), id(2), id(3), id(4)]);
    assert_eq!(run.output.responses.len(), 5);
    assert_eq!(
        run.frame
            .primitives
            .iter()
            .filter(|primitive| matches!(primitive, Primitive::Text(_)))
            .count(),
        5
    );
    let root = run.frame.semantics.get(run.list_id).expect("list root");
    assert_eq!(root.role, SemanticRole::List);
    assert_eq!(root.children.len(), 3);
}

#[test]
fn focused_first_middle_and_last_virtual_rows_add_only_exact_inward_annuli() {
    let items = projection(&[0, 1, 2, 3, 4]);
    for (target, target_y) in [(0_u64, 0.0_f32), (2, 40.0), (4, 80.0)] {
        for selected in [false, true] {
            let mut unfocused_cursor = CollectionCursor::new();
            let mut unfocused_selection = Selection::new();
            if selected {
                unfocused_selection.replace(id(target));
            }
            let mut unfocused_memory = UiMemory::new();
            let unfocused = run_frame(
                &items,
                config(),
                &mut unfocused_cursor,
                &mut unfocused_selection,
                &mut unfocused_memory,
                UiInput::default(),
                false,
            );

            let mut focused_cursor = CollectionCursor::new();
            let mut focused_selection = Selection::new();
            if selected {
                focused_selection.replace(id(target));
            }
            let mut focused_memory = UiMemory::new();
            focused_memory.focus(unfocused.list_id.child(("virtual-list-row", target)));
            let focused = run_frame(
                &items,
                config(),
                &mut focused_cursor,
                &mut focused_selection,
                &mut focused_memory,
                UiInput::default(),
                false,
            );

            assert_eq!(focused.output.window, unfocused.output.window);
            assert_eq!(focused.callbacks, unfocused.callbacks);
            assert_eq!(focused.frame.repaint, unfocused.frame.repaint);
            assert_eq!(focused.output.cursor_target, unfocused.output.cursor_target);
            assert_eq!(
                focused.output.selection_changed,
                unfocused.output.selection_changed
            );
            assert_eq!(
                focused
                    .output
                    .responses
                    .iter()
                    .map(|item| (item.id, item.response.id, item.response.rect))
                    .collect::<Vec<_>>(),
                unfocused
                    .output
                    .responses
                    .iter()
                    .map(|item| (item.id, item.response.id, item.response.rect))
                    .collect::<Vec<_>>()
            );
            assert_eq!(
                focused
                    .frame
                    .semantics
                    .nodes()
                    .iter()
                    .map(|node| (node.id, node.bounds, node.label.clone()))
                    .collect::<Vec<_>>(),
                unfocused
                    .frame
                    .semantics
                    .nodes()
                    .iter()
                    .map(|node| (node.id, node.bounds, node.label.clone()))
                    .collect::<Vec<_>>()
            );
            assert_eq!(
                primitive_without_focus_paths(&focused.frame),
                unfocused.frame.primitives
            );
            assert_eq!(
                focused
                    .frame
                    .primitives
                    .iter()
                    .filter(|primitive| matches!(primitive, Primitive::Path(_)))
                    .count(),
                2
            );
            let rect = Rect::new(0.0, target_y, 120.0, 20.0);
            assert_virtual_row_focus(&focused.frame, rect);
        }
    }
}

#[test]
fn virtual_selected_rows_enumerate_the_white_on_blue_contrast_exception() {
    let theme = default_dark_theme();
    assert_eq!(
        theme.colors.selection.background,
        Color::rgb8(0x0C, 0x8C, 0xE9)
    );
    assert_eq!(theme.colors.selection.foreground, Color::WHITE);
    let ratio = contrast_ratio(
        theme.colors.selection.foreground,
        theme.colors.selection.background,
    );
    assert!((ratio - 3.53).abs() < 0.01);
    assert!(
        ratio < 4.5,
        "known exception is not AA normal-text compliance"
    );

    let items = projection(&[0, 1, 2]);
    let seed = run_frame(
        &items,
        config(),
        &mut CollectionCursor::new(),
        &mut Selection::new(),
        &mut UiMemory::new(),
        UiInput::default(),
        false,
    );
    for (name, input, focused) in [
        ("selected-only", UiInput::default(), false),
        (
            "selected-hovered",
            pointer_input(10.0, 10.0, false, false, Modifiers::default()),
            false,
        ),
        (
            "selected-pressed",
            pointer_input(10.0, 10.0, true, false, Modifiers::default()),
            false,
        ),
        ("selected-focused", UiInput::default(), true),
        (
            "selected-focused-hovered",
            pointer_input(10.0, 10.0, false, false, Modifiers::default()),
            true,
        ),
    ] {
        let mut cursor = CollectionCursor::new();
        let mut selection = Selection::new();
        selection.replace(id(0));
        let mut memory = UiMemory::new();
        if focused {
            memory.focus(seed.list_id.child(("virtual-list-row", 0_u64)));
        }
        let run = run_frame(
            &items,
            config(),
            &mut cursor,
            &mut selection,
            &mut memory,
            input,
            false,
        );
        let base = run
            .frame
            .primitives
            .iter()
            .find_map(|primitive| match primitive {
                Primitive::Rect(base) if base.rect == Rect::new(0.0, 0.0, 120.0, 20.0) => {
                    Some(base)
                }
                _ => None,
            })
            .expect("selected row base");
        assert_eq!(
            base.fill,
            Some(Brush::Solid(theme.colors.selection.background)),
            "{name}"
        );
        assert_eq!(
            base.stroke.expect("neutral row boundary").brush,
            Brush::Solid(theme.colors.border.subtle),
            "{name}"
        );
        let text = run
            .frame
            .primitives
            .iter()
            .find_map(|primitive| match primitive {
                Primitive::Text(text) if text.text == "Row 0" => Some(text),
                _ => None,
            })
            .expect("selected row text");
        assert_eq!(
            text.brush,
            Brush::Solid(theme.colors.selection.foreground),
            "{name}"
        );
    }
}

#[test]
fn disabled_virtual_rows_suppress_focus_annuli_and_remain_non_focusable() {
    let items = projection(&[0, 1]);
    let mut seed_cursor = CollectionCursor::new();
    let mut seed_selection = Selection::new();
    let seed = run_frame(
        &items,
        config(),
        &mut seed_cursor,
        &mut seed_selection,
        &mut UiMemory::new(),
        UiInput::default(),
        false,
    );
    let row_id = seed.list_id.child(("virtual-list-row", 0_u64));
    let mut memory = UiMemory::new();
    memory.focus(row_id);
    let disabled = run_frame(
        &items,
        config().disabled(true),
        &mut CollectionCursor::new(),
        &mut Selection::new(),
        &mut memory,
        UiInput::default(),
        false,
    );
    assert!(disabled.output.responses[0].response.state.focused);
    assert!(
        disabled
            .frame
            .primitives
            .iter()
            .all(|primitive| !matches!(primitive, Primitive::Path(_)))
    );
    let semantics = disabled.frame.semantics.get(row_id).expect("disabled row");
    assert!(!semantics.focusable);
    assert!(semantics.state.disabled);
}

#[test]
fn fractional_scroll_keeps_logical_focus_contained_under_the_existing_clip_transform() {
    let items = projection(&(0..20).collect::<Vec<_>>());
    let mut cursor = CollectionCursor::new();
    let mut selection = Selection::new();
    let mut memory = UiMemory::new();
    let seed = run_frame(
        &items,
        config(),
        &mut cursor,
        &mut selection,
        &mut memory,
        UiInput::default(),
        false,
    );
    memory.focus(seed.list_id.child(("virtual-list-row", 0_u64)));
    let _ = run_frame(
        &items,
        config(),
        &mut cursor,
        &mut selection,
        &mut memory,
        wheel_input(-10.5),
        false,
    );
    let scrolled = run_frame(
        &items,
        config(),
        &mut cursor,
        &mut selection,
        &mut memory,
        UiInput::default(),
        false,
    );
    assert_eq!(
        scrolled.output.window.clamped_scroll_offset.to_bits(),
        10.5_f32.to_bits()
    );
    assert!(matches!(
        scrolled.frame.primitives[1],
        Primitive::ClipBegin { rect, .. } if rect == BOUNDS
    ));
    assert_eq!(
        scrolled.frame.primitives[2],
        Primitive::TransformBegin(Transform::translation(Vec2::new(0.0, -10.5)))
    );
    assert!(matches!(
        scrolled.frame.primitives[scrolled.frame.primitives.len() - 2],
        Primitive::TransformEnd
    ));
    assert!(matches!(
        scrolled.frame.primitives[scrolled.frame.primitives.len() - 1],
        Primitive::ClipEnd { .. }
    ));
    assert_virtual_row_focus(&scrolled.frame, Rect::new(0.0, 0.0, 120.0, 20.0));
}

#[test]
fn wheel_scroll_changes_the_next_frame_window_without_moving_current_geometry() {
    let items = projection(&(0..20).collect::<Vec<_>>());
    let mut cursor = CollectionCursor::new();
    let mut selection = Selection::new();
    let mut memory = UiMemory::new();

    let current = run_frame(
        &items,
        config(),
        &mut cursor,
        &mut selection,
        &mut memory,
        wheel_input(-40.0),
        false,
    );
    assert_eq!(current.output.window.visible_range, 0..3);
    assert_eq!(current.output.scroll.offset.y.to_bits(), 40.0_f32.to_bits());
    assert_eq!(current.callbacks[0], id(0));

    let next = run_frame(
        &items,
        config(),
        &mut cursor,
        &mut selection,
        &mut memory,
        UiInput::default(),
        false,
    );
    assert_eq!(next.output.window.visible_range, 2..5);
    assert_eq!(next.callbacks[0], id(1));
}

#[test]
fn focused_idle_frames_do_not_repaint_or_undo_manual_wheel_scroll() {
    let items = projection(&(0..20).collect::<Vec<_>>());
    let mut cursor = CollectionCursor::new();
    let mut selection = Selection::new();
    let mut memory = UiMemory::new();

    click_row(
        0,
        Modifiers::default(),
        &items,
        &mut cursor,
        &mut selection,
        &mut memory,
    );
    let idle = run_frame(
        &items,
        config(),
        &mut cursor,
        &mut selection,
        &mut memory,
        UiInput::default(),
        false,
    );
    assert_eq!(idle.frame.repaint, RepaintRequest::None);

    let wheel = run_frame(
        &items,
        config(),
        &mut cursor,
        &mut selection,
        &mut memory,
        wheel_input(-40.0),
        false,
    );
    assert_eq!(wheel.frame.repaint, RepaintRequest::NextFrame);
    let scrolled = run_frame(
        &items,
        config(),
        &mut cursor,
        &mut selection,
        &mut memory,
        UiInput::default(),
        false,
    );
    assert_eq!(scrolled.output.window.visible_range, 2..5);
    assert_eq!(scrolled.frame.repaint, RepaintRequest::None);
    assert!(memory.is_focused(scrolled.list_id.child(("virtual-list-row", 0_u64))));
}

#[test]
fn list_surface_blocks_lower_input_and_click_selects_with_ordered_semantics() {
    let items = projection(&[1, 2]);
    let mut cursor = CollectionCursor::new();
    let mut selection = Selection::new();
    let mut memory = UiMemory::new();

    let _ = run_frame(
        &items,
        config(),
        &mut cursor,
        &mut selection,
        &mut memory,
        pointer_input(10.0, 10.0, true, false, Modifiers::default()),
        true,
    );
    let released = run_frame(
        &items,
        config(),
        &mut cursor,
        &mut selection,
        &mut memory,
        pointer_input(10.0, 10.0, false, true, Modifiers::default()),
        true,
    );

    assert!(released.lower.is_some_and(|response| !response.clicked));
    assert_eq!(selection.selected(), vec![id(1)]);
    assert_eq!(cursor.active(), Some(id(1)));
    assert!(released.output.selection_changed);
    let root_position = released
        .frame
        .semantics
        .nodes()
        .iter()
        .position(|node| node.id == released.list_id)
        .expect("root position");
    let row_id = released.list_id.child(("virtual-list-row", 1_u64));
    let row_position = released
        .frame
        .semantics
        .nodes()
        .iter()
        .position(|node| node.id == row_id)
        .expect("row position");
    assert!(root_position < row_position);
    let row = released.frame.semantics.get(row_id).expect("row semantics");
    assert!(row.state.selected);
    assert!(row.state.focused);

    let _ = run_frame(
        &items,
        config(),
        &mut cursor,
        &mut selection,
        &mut memory,
        pointer_input(10.0, 50.0, true, false, Modifiers::default()),
        true,
    );
    let empty_release = run_frame(
        &items,
        config(),
        &mut cursor,
        &mut selection,
        &mut memory,
        pointer_input(10.0, 50.0, false, true, Modifiers::default()),
        true,
    );
    assert!(
        empty_release
            .lower
            .is_some_and(|response| !response.clicked)
    );
}

#[test]
fn multiple_selection_supports_toggle_and_range_modifiers() {
    let items = projection(&[1, 2, 3, 4]);
    let mut cursor = CollectionCursor::new();
    let mut selection = Selection::new();
    let mut memory = UiMemory::new();

    click_row(
        0,
        Modifiers::default(),
        &items,
        &mut cursor,
        &mut selection,
        &mut memory,
    );
    click_row(
        2,
        Modifiers::new(false, true, false, false),
        &items,
        &mut cursor,
        &mut selection,
        &mut memory,
    );
    assert_eq!(selection.selected(), vec![id(1), id(3)]);

    click_row(
        0,
        Modifiers::default(),
        &items,
        &mut cursor,
        &mut selection,
        &mut memory,
    );
    click_row(
        2,
        Modifiers::new(true, false, false, false),
        &items,
        &mut cursor,
        &mut selection,
        &mut memory,
    );
    assert_eq!(selection.selected(), vec![id(1), id(2), id(3)]);
}

#[test]
fn keyboard_navigation_selects_focuses_and_reveals_the_target() {
    let items = projection(&(0..20).collect::<Vec<_>>());
    let mut cursor = CollectionCursor::new();
    let mut selection = Selection::new();
    let mut memory = UiMemory::new();
    click_row(
        0,
        Modifiers::default(),
        &items,
        &mut cursor,
        &mut selection,
        &mut memory,
    );

    let navigated = run_frame(
        &items,
        config(),
        &mut cursor,
        &mut selection,
        &mut memory,
        key_input(Key::PageDown, Modifiers::default(), false),
        false,
    );
    assert_eq!(cursor.active(), Some(id(3)));
    assert_eq!(selection.selected(), vec![id(3)]);
    assert_eq!(
        navigated
            .output
            .cursor_target
            .map(|target| target.projected_index),
        Some(3)
    );

    let revealed = run_frame(
        &items,
        config(),
        &mut cursor,
        &mut selection,
        &mut memory,
        UiInput::default(),
        false,
    );
    assert_eq!(
        revealed.output.window.clamped_scroll_offset.to_bits(),
        20.0_f32.to_bits()
    );
    let focused = revealed.list_id.child(("virtual-list-row", 3_u64));
    assert!(memory.is_focused(focused));
}

#[test]
fn enter_and_space_activate_once_and_reject_repeat() {
    let items = projection(&[1, 2, 3]);
    let mut cursor = CollectionCursor::new();
    let mut selection = Selection::new();
    let mut memory = UiMemory::new();
    click_row(
        1,
        Modifiers::default(),
        &items,
        &mut cursor,
        &mut selection,
        &mut memory,
    );

    let enter = run_frame(
        &items,
        config(),
        &mut cursor,
        &mut selection,
        &mut memory,
        key_input(Key::Enter, Modifiers::default(), false),
        false,
    );
    assert_eq!(enter.output.activated, Some(id(2)));
    assert_eq!(
        enter
            .output
            .responses
            .iter()
            .filter(|item| item.response.keyboard_activated)
            .count(),
        1
    );

    let repeated = run_frame(
        &items,
        config(),
        &mut cursor,
        &mut selection,
        &mut memory,
        key_input(Key::Space, Modifiers::default(), true),
        false,
    );
    assert_eq!(repeated.output.activated, None);
    assert!(
        repeated
            .output
            .responses
            .iter()
            .all(|item| !item.response.keyboard_activated)
    );
}

#[test]
fn stable_ids_and_focus_repair_survive_reorder_and_removal() {
    let first = projection(&[1, 2, 3]);
    let reordered = projection(&[3, 2, 1]);
    let removed = projection(&[1, 3]);
    let mut cursor = CollectionCursor::new();
    let mut selection = Selection::new();
    let mut memory = UiMemory::new();
    let clicked = click_row(
        1,
        Modifiers::default(),
        &first,
        &mut cursor,
        &mut selection,
        &mut memory,
    );
    let stable_id = clicked.list_id.child(("virtual-list-row", 2_u64));

    let reordered_run = run_frame(
        &reordered,
        config(),
        &mut cursor,
        &mut selection,
        &mut memory,
        UiInput::default(),
        false,
    );
    assert_eq!(
        reordered_run.list_id.child(("virtual-list-row", 2_u64)),
        stable_id
    );
    assert!(memory.is_focused(stable_id));

    let removed_run = run_frame(
        &removed,
        config(),
        &mut cursor,
        &mut selection,
        &mut memory,
        UiInput::default(),
        false,
    );
    assert_eq!(cursor.active(), Some(id(3)));
    let repaired = removed_run.list_id.child(("virtual-list-row", 3_u64));
    assert!(memory.is_focused(repaired));
    assert!(!memory.is_focused(stable_id));
    assert!(removed_run.frame.semantics.get(stable_id).is_none());
}

#[test]
fn invalid_geometry_is_rejected_and_disabled_or_empty_lists_are_inert() {
    let items = projection(&[1, 2]);
    let theme = default_dark_theme();
    let mut invalid_memory = UiMemory::new();
    let ui = Ui::begin_frame(context(UiInput::default()), &mut invalid_memory, &theme);
    assert!(
        ui.prepare_virtual_list(
            "invalid",
            VirtualListConfig::new(Rect::new(f32::NAN, 0.0, 100.0, 20.0), 20.0),
            &items,
        )
        .is_none()
    );
    assert!(
        ui.prepare_virtual_list(
            "empty-bounds",
            VirtualListConfig::new(Rect::ZERO, 20.0),
            &items,
        )
        .is_none()
    );
    assert!(
        ui.prepare_virtual_list(
            "invalid-row",
            VirtualListConfig::new(BOUNDS, f32::INFINITY),
            &items,
        )
        .is_none()
    );

    let empty = CollectionProjection::empty();
    let mut cursor = CollectionCursor::new();
    let mut selection = Selection::new();
    let mut empty_memory = UiMemory::new();
    let empty_run = run_frame(
        &empty,
        config(),
        &mut cursor,
        &mut selection,
        &mut empty_memory,
        UiInput::default(),
        false,
    );
    assert!(empty_run.callbacks.is_empty());
    assert!(empty_run.output.responses.is_empty());
    assert_eq!(
        empty_run
            .frame
            .semantics
            .get(empty_run.list_id)
            .expect("empty list semantics")
            .role,
        SemanticRole::List
    );

    let mut disabled_memory = UiMemory::new();
    let disabled_config = config().disabled(true);
    let _ = run_frame(
        &items,
        disabled_config.clone(),
        &mut cursor,
        &mut selection,
        &mut disabled_memory,
        pointer_input(10.0, 10.0, true, false, Modifiers::default()),
        false,
    );
    let disabled = run_frame(
        &items,
        disabled_config,
        &mut cursor,
        &mut selection,
        &mut disabled_memory,
        pointer_input(10.0, 10.0, false, true, Modifiers::default()),
        false,
    );
    assert!(selection.selected().is_empty());
    assert!(
        disabled
            .output
            .responses
            .iter()
            .all(|item| { item.response.state.disabled && !item.response.clicked })
    );
}
