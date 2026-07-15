//! Public prepared-outliner MVP conformance tests.

use std::time::Duration;

use stern_core::{
    ActionContext, ActionDescriptor, ActionId, ActionSource, FrameContext, FrameOutput, Key,
    KeyEvent, KeyState, KeyboardInput, Modifiers, PhysicalSize, Point, PointerButtonState,
    PointerInput, PointerOrder, PointerTarget, Primitive, RadiusScale, Rect, RepaintRequest,
    Response, ScaleFactor, SemanticRole, Size, StrokeScale, Theme, TimeInfo, UiInput, UiMemory,
    Vec2, ViewportInfo, WidgetId, default_dark_theme,
};
use stern_widgets::outliner::{
    OutlinerConfig, OutlinerOutput, OutlinerRequest, OutlinerSelectionMode, OutlinerState,
};
use stern_widgets::{
    CollectionContextTarget, InlineEditCancelReason, InlineEditCommitReason, InlineEditRequest,
    ItemId, OutlinerDropZoneKind, OutlinerItem, OutlinerModel, OutlinerRowFlags, OutlinerRowZones,
    Ui,
};

const BOUNDS: Rect = Rect::new(0.0, 0.0, 240.0, 80.0);
const LOWER: Rect = Rect::new(0.0, 0.0, 280.0, 120.0);

fn id(raw: u64) -> ItemId {
    ItemId::from_raw(raw)
}

fn roots(raw_ids: impl IntoIterator<Item = u64>) -> OutlinerModel {
    OutlinerModel::new(
        raw_ids
            .into_iter()
            .map(|raw| OutlinerItem::new(id(raw), format!("Item {raw}")))
            .collect::<Vec<_>>(),
    )
}

fn nested_model() -> OutlinerModel {
    OutlinerModel::new(vec![
        OutlinerItem::new(id(10), "World").with_has_children(true),
        OutlinerItem::new(id(11), "Camera").with_parent(id(10)),
        OutlinerItem::new(id(12), "Light").with_parent(id(10)),
        OutlinerItem::new(id(20), "Interface"),
    ])
}

fn config() -> OutlinerConfig {
    OutlinerConfig::new(BOUNDS, 20.0, 16.0)
        .label("Scene hierarchy")
        .overscan(1)
        .selection_mode(OutlinerSelectionMode::Multiple)
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

fn primary_input(
    point: Point,
    down: bool,
    pressed: bool,
    released: bool,
    click_count: u8,
) -> UiInput {
    UiInput {
        pointer: PointerInput {
            position: Some(point),
            primary: PointerButtonState::new(down, pressed, released),
            click_count,
            ..PointerInput::default()
        },
        ..UiInput::default()
    }
}

fn secondary_input(point: Point, down: bool, pressed: bool, released: bool) -> UiInput {
    UiInput {
        pointer: PointerInput {
            position: Some(point),
            secondary: PointerButtonState::new(down, pressed, released),
            ..PointerInput::default()
        },
        ..UiInput::default()
    }
}

fn move_input(point: Point, delta: Vec2) -> UiInput {
    UiInput {
        pointer: PointerInput {
            position: Some(point),
            delta,
            primary: PointerButtonState::new(true, false, false),
            ..PointerInput::default()
        },
        ..UiInput::default()
    }
}

fn key_input(key: Key) -> UiInput {
    key_input_with_modifiers(key, Modifiers::default())
}

fn key_input_with_modifiers(key: Key, modifiers: Modifiers) -> UiInput {
    UiInput {
        keyboard: KeyboardInput {
            modifiers,
            events: vec![KeyEvent::new(key, KeyState::Pressed, modifiers, false)],
        },
        ..UiInput::default()
    }
}

fn typed_input(text: &str) -> UiInput {
    let event = KeyEvent::new(
        Key::Character(text.to_owned()),
        KeyState::Pressed,
        Modifiers::default(),
        false,
    )
    .with_text(text);
    UiInput {
        keyboard: KeyboardInput {
            modifiers: Modifiers::default(),
            events: vec![event],
        },
        ..UiInput::default()
    }
}

fn action(id: &str, label: &str) -> ActionDescriptor {
    ActionDescriptor::new(id, label)
}

struct Run {
    root: WidgetId,
    rows: Vec<OutlinerRowZones>,
    lower: Option<Response>,
    output: OutlinerOutput,
    frame: FrameOutput,
}

#[allow(clippy::too_many_arguments)]
fn run_frame(
    model: &OutlinerModel,
    config: OutlinerConfig,
    state: &mut OutlinerState,
    memory: &mut UiMemory,
    input: UiInput,
    lower: bool,
) -> Run {
    let theme = default_dark_theme();
    run_frame_with_theme(model, config, state, memory, input, lower, &theme)
}

#[allow(clippy::too_many_arguments)]
fn run_frame_with_theme(
    model: &OutlinerModel,
    config: OutlinerConfig,
    state: &mut OutlinerState,
    memory: &mut UiMemory,
    input: UiInput,
    lower: bool,
    theme: &Theme,
) -> Run {
    let mut ui = Ui::begin_frame(context(input), memory, theme);
    let scene = ui
        .prepare_outliner("scene-outliner", config, model, state)
        .expect("valid outliner scene");
    let root = scene.widget_id();
    let rows = scene.rows().to_vec();
    let lower_id = ui.make_id("lower");
    ui.resolve_pointer_targets(|plan| {
        if lower {
            plan.target(PointerTarget::new(lower_id, LOWER, PointerOrder::new(10)));
        }
        scene.declare_pointer_targets(plan, PointerOrder::new(100), state);
    })
    .expect("valid shared pointer plan");
    let lower = lower.then(|| ui.pressable_with_id(lower_id, LOWER, false));
    let output = ui.outliner(&scene, state, |target| match target {
        CollectionContextTarget::Background(_) => vec![action("scene.create", "Create")],
        CollectionContextTarget::Item(_) | CollectionContextTarget::Selection(_) => {
            vec![action("scene.delete", "Delete")]
        }
    });
    let frame = ui.finish_output();
    Run {
        root,
        rows,
        lower,
        output,
        frame,
    }
}

fn click(
    point: Point,
    click_count: u8,
    model: &OutlinerModel,
    state: &mut OutlinerState,
    memory: &mut UiMemory,
) -> Run {
    let _ = run_frame(
        model,
        config(),
        state,
        memory,
        primary_input(point, true, true, false, click_count),
        false,
    );
    run_frame(
        model,
        config(),
        state,
        memory,
        primary_input(point, false, false, true, click_count),
        false,
    )
}

fn start_rename(
    row: usize,
    model: &OutlinerModel,
    state: &mut OutlinerState,
    memory: &mut UiMemory,
) -> Run {
    let idle = run_frame(model, config(), state, memory, UiInput::default(), false);
    let _ = click(idle.rows[row].label_rect.center(), 1, model, state, memory);
    run_frame(
        model,
        config(),
        state,
        memory,
        key_input(Key::Function(2)),
        false,
    )
}

fn context_click(
    point: Point,
    model: &OutlinerModel,
    state: &mut OutlinerState,
    memory: &mut UiMemory,
) -> Run {
    let _ = run_frame(
        model,
        config(),
        state,
        memory,
        secondary_input(point, true, true, false),
        false,
    );
    run_frame(
        model,
        config(),
        state,
        memory,
        secondary_input(point, false, false, true),
        false,
    )
}

fn semantic_center(frame: &FrameOutput, label: &str) -> Point {
    frame
        .semantics
        .nodes()
        .iter()
        .find(|node| node.label.as_deref() == Some(label))
        .unwrap_or_else(|| panic!("missing semantic node {label}"))
        .bounds
        .center()
}

#[test]
fn prepared_scene_freezes_virtual_rows_pointer_routing_paint_and_semantics() {
    let model = roots(0..10_000);
    let mut state = OutlinerState::new();
    let mut memory = UiMemory::new();
    let hovered = run_frame(
        &model,
        config(),
        &mut state,
        &mut memory,
        primary_input(Point::new(100.0, 10.0), false, false, false, 0),
        true,
    );

    assert_eq!(hovered.output.window.visible_range, 0..4);
    assert_eq!(hovered.output.window.materialized_range, 0..6);
    assert_eq!(hovered.rows.len(), 6);
    assert_eq!(hovered.output.responses.len(), 6);
    assert!(
        hovered
            .lower
            .is_some_and(|response| !response.state.hovered)
    );
    assert_eq!(
        hovered
            .frame
            .primitives
            .iter()
            .filter(|primitive| matches!(primitive, Primitive::Text(_)))
            .count(),
        6
    );
    let root = hovered
        .frame
        .semantics
        .get(hovered.root)
        .expect("outliner semantics root");
    assert_eq!(root.role, SemanticRole::List);
    assert_eq!(root.label.as_deref(), Some("Scene hierarchy"));
    assert_eq!(root.children.len(), 4);
    assert_eq!(
        root.children[0],
        hovered.root.child(("outliner-row", 0_u64))
    );
    hovered
        .frame
        .semantics
        .validate()
        .expect("valid prepared semantic tree");
    assert!(hovered.frame.warnings.is_empty());
}

#[test]
fn selection_and_keyboard_navigation_expand_enter_children_and_return_to_parent() {
    let model = nested_model();
    let mut state = OutlinerState::new();
    let mut memory = UiMemory::new();
    let idle = run_frame(
        &model,
        config(),
        &mut state,
        &mut memory,
        UiInput::default(),
        false,
    );
    let world = idle.rows[0].label_rect.center();

    let selected = click(world, 1, &model, &mut state, &mut memory);
    assert!(selected.output.selection_changed);
    assert_eq!(state.selection.selected(), vec![id(10)]);
    assert_eq!(state.cursor.active(), Some(id(10)));
    assert!(memory.is_focused(selected.root.child(("outliner-row", 10_u64))));

    let expanded = run_frame(
        &model,
        config(),
        &mut state,
        &mut memory,
        key_input(Key::ArrowRight),
        false,
    );
    assert!(expanded.output.expansion_changed);
    assert!(state.expansion.is_expanded(id(10)));

    let child = run_frame(
        &model,
        config(),
        &mut state,
        &mut memory,
        key_input(Key::ArrowRight),
        false,
    );
    assert_eq!(state.cursor.active(), Some(id(11)));
    assert_eq!(state.selection.selected(), vec![id(11)]);
    assert!(child.output.selection_changed);

    let sibling = run_frame(
        &model,
        config(),
        &mut state,
        &mut memory,
        key_input(Key::ArrowDown),
        false,
    );
    assert_eq!(state.cursor.active(), Some(id(12)));
    assert_eq!(state.selection.selected(), vec![id(12)]);
    assert_eq!(sibling.frame.repaint, RepaintRequest::NextFrame);

    let parent = run_frame(
        &model,
        config(),
        &mut state,
        &mut memory,
        key_input(Key::ArrowLeft),
        false,
    );
    assert_eq!(state.cursor.active(), Some(id(10)));
    assert_eq!(state.selection.selected(), vec![id(10)]);
    assert!(parent.output.selection_changed);

    let collapsed = run_frame(
        &model,
        config(),
        &mut state,
        &mut memory,
        key_input(Key::ArrowLeft),
        false,
    );
    assert!(collapsed.output.expansion_changed);
    assert!(!state.expansion.is_expanded(id(10)));
}

#[test]
#[allow(clippy::too_many_lines)]
fn f2_and_double_click_inline_rename_emit_begin_draft_commit_or_cancel_without_app_mutation() {
    let model = nested_model();
    let mut state = OutlinerState::new();
    let mut memory = UiMemory::new();
    let idle = run_frame(
        &model,
        config(),
        &mut state,
        &mut memory,
        UiInput::default(),
        false,
    );
    let world = idle.rows[0].label_rect.center();
    let selected = click(world, 1, &model, &mut state, &mut memory);

    let begin = run_frame(
        &model,
        config(),
        &mut state,
        &mut memory,
        key_input(Key::Function(2)),
        false,
    );
    let Some(OutlinerRequest::Rename(InlineEditRequest::Begin(begin_request))) =
        begin.output.requests.first()
    else {
        panic!("F2 must begin inline rename");
    };
    assert_eq!(begin_request.target, id(10));
    assert_eq!(state.rename_target(), Some(id(10)));

    let drafted = run_frame(
        &model,
        config(),
        &mut state,
        &mut memory,
        typed_input("X"),
        false,
    );
    let Some(OutlinerRequest::Rename(InlineEditRequest::DraftEdit(draft))) =
        drafted.output.requests.first()
    else {
        panic!("text input must emit a draft request");
    };
    assert_eq!(draft.target, id(10));
    assert_eq!(draft.draft_text, "WorldX");
    assert_eq!(state.rename_draft(), Some("WorldX"));
    let rename_id = selected.root.child(("inline-edit", 10_u64));
    assert_eq!(
        drafted
            .frame
            .semantics
            .get(rename_id)
            .expect("rename field semantics")
            .role,
        SemanticRole::TextField
    );
    assert!(
        drafted
            .frame
            .semantics
            .get(selected.root)
            .expect("outliner root")
            .children
            .contains(&rename_id)
    );

    let committed = run_frame(
        &model,
        config(),
        &mut state,
        &mut memory,
        key_input(Key::Enter),
        false,
    );
    let Some(OutlinerRequest::Rename(InlineEditRequest::Commit(commit))) =
        committed.output.requests.first()
    else {
        panic!("Enter must commit changed rename");
    };
    assert_eq!(commit.target, id(10));
    assert_eq!(commit.draft_text, "WorldX");
    assert_eq!(commit.reason, InlineEditCommitReason::Enter);
    assert_eq!(state.rename_target(), None);
    assert_eq!(model.item_by_id(id(10)).expect("app item").label, "World");

    let mut cancel_state = OutlinerState::new();
    let mut cancel_memory = UiMemory::new();
    let double = click(world, 2, &model, &mut cancel_state, &mut cancel_memory);
    assert!(matches!(
        double.output.requests.as_slice(),
        [OutlinerRequest::Rename(InlineEditRequest::Begin(begin))] if begin.target == id(10)
    ));
    assert_eq!(cancel_state.rename_target(), Some(id(10)));
    let cancelled = run_frame(
        &model,
        config(),
        &mut cancel_state,
        &mut cancel_memory,
        key_input(Key::Escape),
        false,
    );
    assert!(matches!(
        cancelled.output.requests.as_slice(),
        [OutlinerRequest::Rename(InlineEditRequest::Cancel(cancel))]
            if cancel.target == id(10) && cancel.reason == InlineEditCancelReason::Escape
    ));
    assert_eq!(cancel_state.rename_target(), None);
}

#[test]
#[allow(clippy::too_many_lines)]
fn visibility_and_lock_controls_emit_typed_requests_without_mutating_app_flags() {
    let model = roots([10]);
    let mut state = OutlinerState::new();
    let mut memory = UiMemory::new();
    let strokes = StrokeScale::from_values(0.75, 1.25, 2.5, 3.5, 4.5);
    let mut theme = default_dark_theme()
        .with_radii(RadiusScale::from_values(4.0, 11.0, 23.0, 777.0))
        .with_strokes(strokes);
    theme.border_width = 99.0;
    let idle = run_frame_with_theme(
        &model,
        config(),
        &mut state,
        &mut memory,
        UiInput::default(),
        false,
        &theme,
    );
    let zones = idle.rows[0].clone();
    let focused_id = idle.output.responses[0].row.id;
    memory.focus(focused_id);
    let focused = run_frame_with_theme(
        &model,
        config(),
        &mut state,
        &mut memory,
        UiInput::default(),
        false,
        &theme,
    );
    assert_eq!(idle.rows, focused.rows);
    assert_eq!(idle.frame.primitives.len(), focused.frame.primitives.len());
    assert_eq!(
        idle.output
            .responses
            .iter()
            .map(|response| (response.row.id, response.row.rect))
            .collect::<Vec<_>>(),
        focused
            .output
            .responses
            .iter()
            .map(|response| (response.row.id, response.row.rect))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        idle.frame
            .primitives
            .iter()
            .filter_map(|primitive| match primitive {
                Primitive::Rect(rect) => Some((
                    rect.rect,
                    rect.radius,
                    rect.stroke.map(|stroke| stroke.width),
                )),
                _ => None,
            })
            .collect::<Vec<_>>(),
        focused
            .frame
            .primitives
            .iter()
            .filter_map(|primitive| match primitive {
                Primitive::Rect(rect) => Some((
                    rect.rect,
                    rect.radius,
                    rect.stroke.map(|stroke| stroke.width),
                )),
                _ => None,
            })
            .collect::<Vec<_>>()
    );
    assert_eq!(
        idle.frame
            .semantics
            .nodes()
            .iter()
            .map(|node| (node.id, node.bounds))
            .collect::<Vec<_>>(),
        focused
            .frame
            .semantics
            .nodes()
            .iter()
            .map(|node| (node.id, node.bounds))
            .collect::<Vec<_>>()
    );
    let surface = idle
        .frame
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            Primitive::Rect(rect) if rect.rect == BOUNDS => Some(rect),
            _ => None,
        })
        .expect("outliner structural surface");
    assert_eq!(
        surface.stroke.map(|stroke| stroke.width),
        Some(strokes.hairline)
    );
    let visibility_inset = zones
        .visibility_toggle_rect
        .width
        .min(zones.visibility_toggle_rect.height)
        * 0.25;
    let visibility_icon = Rect::new(
        zones.visibility_toggle_rect.x + visibility_inset,
        zones.visibility_toggle_rect.y + visibility_inset,
        (zones.visibility_toggle_rect.width - visibility_inset * 2.0).max(0.0),
        (zones.visibility_toggle_rect.height - visibility_inset * 2.0).max(0.0),
    );
    let visibility_paint = idle
        .frame
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            Primitive::Rect(rect) if rect.rect == visibility_icon => Some(rect),
            _ => None,
        })
        .expect("visibility icon paint");
    assert_eq!(visibility_paint.radius, theme.radii.full);
    assert_eq!(
        visibility_paint.stroke.map(|stroke| stroke.width),
        Some(strokes.default)
    );

    let lock_width = zones.lock_toggle_rect.width * 0.42;
    let lock_height = zones.lock_toggle_rect.height * 0.34;
    let lock_body = Rect::new(
        zones.lock_toggle_rect.center().x - lock_width * 0.5,
        zones.lock_toggle_rect.center().y,
        lock_width,
        lock_height,
    );
    let lock_paint = idle
        .frame
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            Primitive::Rect(rect) if rect.rect == lock_body => Some(rect),
            _ => None,
        })
        .expect("lock body paint");
    assert_eq!(lock_paint.radius, theme.radii.sm);
    assert_eq!(
        lock_paint.stroke.map(|stroke| stroke.width),
        Some(strokes.default)
    );
    let row_semantics = idle
        .frame
        .semantics
        .nodes()
        .iter()
        .find(|node| node.label.as_deref() == Some("Item 10"))
        .expect("row semantics");
    assert_eq!(row_semantics.bounds, zones.rect);

    let visibility = click(
        zones.visibility_toggle_rect.center(),
        1,
        &model,
        &mut state,
        &mut memory,
    );
    assert!(matches!(
        visibility.output.requests.as_slice(),
        [OutlinerRequest::Visibility(request)]
            if request.target == id(10) && request.visible
    ));
    assert!(state.selection.selected().is_empty());

    let lock = click(
        zones.lock_toggle_rect.center(),
        1,
        &model,
        &mut state,
        &mut memory,
    );
    assert!(matches!(
        lock.output.requests.as_slice(),
        [OutlinerRequest::Lock(request)]
            if request.target == id(10) && !request.locked
    ));
    let flags = model.item_by_id(id(10)).expect("app item").flags;
    assert!(flags.visible);
    assert!(!flags.locked);
}

#[test]
#[allow(clippy::too_many_lines)]
fn row_and_background_context_menus_preserve_targets_and_match_the_action_queue() {
    let model = roots([10]);
    let mut state = OutlinerState::new();
    let mut memory = UiMemory::new();
    let idle = run_frame(
        &model,
        config(),
        &mut state,
        &mut memory,
        UiInput::default(),
        false,
    );
    let row = idle.rows[0].label_rect.center();
    let selected = click(row, 1, &model, &mut state, &mut memory);
    let opened = context_click(row, &model, &mut state, &mut memory);
    let selection_target =
        CollectionContextTarget::selection([id(10)]).expect("selection context target");
    assert_eq!(opened.output.context_opened, Some(selection_target.clone()));
    assert_eq!(state.context_target(), Some(&selection_target));

    let menu = run_frame(
        &model,
        config(),
        &mut state,
        &mut memory,
        UiInput::default(),
        false,
    );
    let delete = semantic_center(&menu.frame, "Delete");
    let _ = run_frame(
        &model,
        config(),
        &mut state,
        &mut memory,
        primary_input(delete, true, true, false, 1),
        false,
    );
    let mut invoked = run_frame(
        &model,
        config(),
        &mut state,
        &mut memory,
        primary_input(delete, false, false, true, 1),
        false,
    );
    let Some(OutlinerRequest::Context(request)) = invoked.output.requests.first() else {
        panic!("context menu must emit a typed action request");
    };
    let invocation = invoked
        .frame
        .actions
        .pop_front()
        .expect("matching frame action");
    assert_eq!(request.action_id, ActionId::new("scene.delete"));
    assert_eq!(request.target, selection_target);
    assert_eq!(request.target_ids, vec![id(10)]);
    assert_eq!(invocation.action_id, request.action_id);
    assert_eq!(invocation.source, ActionSource::Menu);
    assert_eq!(invocation.context, ActionContext::Widget(selected.root));
    assert!(invoked.frame.actions.is_empty());
    assert_eq!(state.context_target(), None);

    let background = Point::new(200.0, 60.0);
    let opened = context_click(background, &model, &mut state, &mut memory);
    assert_eq!(
        opened.output.context_opened,
        Some(CollectionContextTarget::background())
    );
    let menu = run_frame(
        &model,
        config(),
        &mut state,
        &mut memory,
        UiInput::default(),
        false,
    );
    let create = semantic_center(&menu.frame, "Create");
    let _ = run_frame(
        &model,
        config(),
        &mut state,
        &mut memory,
        primary_input(create, true, true, false, 1),
        false,
    );
    let mut invoked = run_frame(
        &model,
        config(),
        &mut state,
        &mut memory,
        primary_input(create, false, false, true, 1),
        false,
    );
    let Some(OutlinerRequest::Context(request)) = invoked.output.requests.first() else {
        panic!("background menu must emit a typed action request");
    };
    let invocation = invoked
        .frame
        .actions
        .pop_front()
        .expect("matching background frame action");
    assert_eq!(request.action_id, ActionId::new("scene.create"));
    assert_eq!(request.target, CollectionContextTarget::background());
    assert!(request.target_ids.is_empty());
    assert_eq!(invocation.action_id, request.action_id);
    assert_eq!(invocation.source, ActionSource::Menu);
    assert_eq!(invocation.context, ActionContext::Widget(selected.root));
}

#[test]
fn hierarchy_drag_rejects_descendant_cycles_and_emits_one_valid_drop_request() {
    let model = nested_model();
    let mut state = OutlinerState::new();
    state.expansion.expand(id(10));
    let mut memory = UiMemory::new();
    let idle = run_frame(
        &model,
        config(),
        &mut state,
        &mut memory,
        UiInput::default(),
        false,
    );
    let world = idle.rows[0].label_rect.center();
    let camera = idle.rows[1].label_rect.center();
    let interface = idle.rows[3].label_rect.center();

    let _ = run_frame(
        &model,
        config(),
        &mut state,
        &mut memory,
        primary_input(world, true, true, false, 1),
        false,
    );
    let rejected_preview = run_frame(
        &model,
        config(),
        &mut state,
        &mut memory,
        move_input(camera, Vec2::new(camera.x - world.x, camera.y - world.y)),
        false,
    );
    assert!(state.dragging());
    assert_eq!(rejected_preview.output.drop_preview, None);
    let rejected = run_frame(
        &model,
        config(),
        &mut state,
        &mut memory,
        primary_input(camera, false, false, true, 1),
        false,
    );
    assert!(
        rejected
            .output
            .requests
            .iter()
            .all(|request| !matches!(request, OutlinerRequest::Drop(_)))
    );
    assert!(!state.dragging());

    let _ = run_frame(
        &model,
        config(),
        &mut state,
        &mut memory,
        primary_input(camera, true, true, false, 1),
        false,
    );
    let accepted_preview = run_frame(
        &model,
        config(),
        &mut state,
        &mut memory,
        move_input(
            interface,
            Vec2::new(interface.x - camera.x, interface.y - camera.y),
        ),
        false,
    );
    let preview = accepted_preview
        .output
        .drop_preview
        .expect("cycle-safe drop preview");
    assert_eq!(preview.source.source, id(11));
    assert_eq!(preview.target, id(20));
    assert_eq!(preview.zone, OutlinerDropZoneKind::Inside);

    let accepted = run_frame(
        &model,
        config(),
        &mut state,
        &mut memory,
        primary_input(interface, false, false, true, 1),
        false,
    );
    let Some(OutlinerRequest::Drop(drop)) = accepted.output.requests.first() else {
        panic!("valid hierarchy drop must emit one typed request");
    };
    assert_eq!(drop.source.source, id(11));
    assert_eq!(drop.source.items, vec![id(11)]);
    assert_eq!(drop.target, id(20));
    assert_eq!(drop.zone, OutlinerDropZoneKind::Inside);
    assert!(!state.dragging());
}

#[test]
fn keyboard_shift_range_skips_disabled_and_non_selectable_rows() {
    let mut disabled = OutlinerRowFlags::new();
    disabled.disabled = true;
    let mut non_selectable = OutlinerRowFlags::new();
    non_selectable.selectable = false;
    let model = OutlinerModel::new(vec![
        OutlinerItem::new(id(1), "First"),
        OutlinerItem::new(id(2), "Disabled").with_flags(disabled),
        OutlinerItem::new(id(3), "Decorative").with_flags(non_selectable),
        OutlinerItem::new(id(4), "Last"),
    ]);
    let mut state = OutlinerState::new();
    let mut memory = UiMemory::new();
    let idle = run_frame(
        &model,
        config(),
        &mut state,
        &mut memory,
        UiInput::default(),
        false,
    );
    let selected = click(
        idle.rows[0].label_rect.center(),
        1,
        &model,
        &mut state,
        &mut memory,
    );
    assert_eq!(state.selection.selected(), vec![id(1)]);

    let ranged = run_frame(
        &model,
        config(),
        &mut state,
        &mut memory,
        key_input_with_modifiers(Key::ArrowDown, Modifiers::new(true, false, false, false)),
        false,
    );

    assert!(ranged.output.selection_changed);
    assert_eq!(state.cursor.active(), Some(id(4)));
    assert_eq!(state.selection.selected(), vec![id(1), id(4)]);
    assert!(memory.is_focused(selected.root.child(("outliner-row", 4_u64))));
}

#[test]
#[allow(clippy::too_many_lines)]
fn retained_rename_reconciles_read_only_global_disabled_offscreen_and_removed_targets() {
    let enabled = roots([10]);

    let mut read_only_state = OutlinerState::new();
    let mut read_only_memory = UiMemory::new();
    let _ = start_rename(0, &enabled, &mut read_only_state, &mut read_only_memory);
    let mut read_only_flags = OutlinerRowFlags::new();
    read_only_flags.read_only = true;
    let read_only = OutlinerModel::new(vec![
        OutlinerItem::new(id(10), "Item 10").with_flags(read_only_flags),
    ]);
    let reconciled = run_frame(
        &read_only,
        config(),
        &mut read_only_state,
        &mut read_only_memory,
        UiInput::default(),
        false,
    );
    assert!(matches!(
        reconciled.output.requests.as_slice(),
        [OutlinerRequest::Rename(InlineEditRequest::Cancel(cancel))]
            if cancel.target == id(10) && cancel.reason == InlineEditCancelReason::Explicit
    ));
    assert_eq!(read_only_state.rename_target(), None);

    let mut disabled_state = OutlinerState::new();
    let mut disabled_memory = UiMemory::new();
    let _ = start_rename(0, &enabled, &mut disabled_state, &mut disabled_memory);
    let disabled = run_frame(
        &enabled,
        config().disabled(true),
        &mut disabled_state,
        &mut disabled_memory,
        UiInput::default(),
        false,
    );
    assert!(matches!(
        disabled.output.requests.as_slice(),
        [OutlinerRequest::Rename(InlineEditRequest::Cancel(cancel))]
            if cancel.target == id(10) && cancel.reason == InlineEditCancelReason::Explicit
    ));
    assert_eq!(disabled_state.rename_target(), None);

    let many = roots(0..20);
    let mut offscreen_state = OutlinerState::new();
    let mut offscreen_memory = UiMemory::new();
    let begin = start_rename(0, &many, &mut offscreen_state, &mut offscreen_memory);
    offscreen_memory.set_scroll_offset(begin.root, Vec2::new(0.0, 200.0));
    let offscreen = run_frame(
        &many,
        config(),
        &mut offscreen_state,
        &mut offscreen_memory,
        UiInput::default(),
        false,
    );
    assert!(matches!(
        offscreen.output.requests.as_slice(),
        [OutlinerRequest::Rename(InlineEditRequest::Cancel(cancel))]
            if cancel.target == id(0) && cancel.reason == InlineEditCancelReason::DraftPolicy
    ));
    assert_eq!(offscreen_state.rename_target(), None);
    let revealed = run_frame(
        &many,
        config(),
        &mut offscreen_state,
        &mut offscreen_memory,
        UiInput::default(),
        false,
    );
    assert_eq!(revealed.output.window.visible_range.start, 0);

    let original = roots([1, 2]);
    let mut removed_state = OutlinerState::new();
    let mut removed_memory = UiMemory::new();
    let _ = start_rename(0, &original, &mut removed_state, &mut removed_memory);
    let removed = run_frame(
        &roots([2]),
        config(),
        &mut removed_state,
        &mut removed_memory,
        UiInput::default(),
        false,
    );
    assert!(matches!(
        removed.output.requests.as_slice(),
        [OutlinerRequest::Rename(InlineEditRequest::Cancel(cancel))]
            if cancel.target == id(1) && cancel.reason == InlineEditCancelReason::Explicit
    ));
    assert_eq!(removed_state.rename_target(), None);
}

#[test]
fn global_disable_closes_context_and_offscreen_drag_ownership() {
    let model = roots(0..20);

    let mut context_state = OutlinerState::new();
    let mut context_memory = UiMemory::new();
    let idle = run_frame(
        &model,
        config(),
        &mut context_state,
        &mut context_memory,
        UiInput::default(),
        false,
    );
    let opened = context_click(
        idle.rows[0].label_rect.center(),
        &model,
        &mut context_state,
        &mut context_memory,
    );
    assert!(opened.output.context_opened.is_some());
    let disabled = run_frame(
        &model,
        config().disabled(true),
        &mut context_state,
        &mut context_memory,
        UiInput::default(),
        false,
    );
    assert_eq!(context_state.context_target(), None);
    assert!(disabled.output.requests.is_empty());
    assert!(disabled.frame.actions.is_empty());
    assert_eq!(disabled.frame.repaint, RepaintRequest::NextFrame);

    let mut drag_state = OutlinerState::new();
    let mut drag_memory = UiMemory::new();
    let idle = run_frame(
        &model,
        config(),
        &mut drag_state,
        &mut drag_memory,
        UiInput::default(),
        false,
    );
    let source = idle.rows[0].label_rect.center();
    let target = idle.rows[1].label_rect.center();
    let _ = run_frame(
        &model,
        config(),
        &mut drag_state,
        &mut drag_memory,
        primary_input(source, true, true, false, 1),
        false,
    );
    let dragging = run_frame(
        &model,
        config(),
        &mut drag_state,
        &mut drag_memory,
        move_input(target, Vec2::new(target.x - source.x, target.y - source.y)),
        false,
    );
    assert!(drag_state.dragging());
    assert!(dragging.output.drop_preview.is_some());
    drag_memory.set_scroll_offset(dragging.root, Vec2::new(0.0, 200.0));
    let reconciled = run_frame(
        &model,
        config(),
        &mut drag_state,
        &mut drag_memory,
        UiInput::default(),
        false,
    );
    assert!(!drag_state.dragging());
    assert_eq!(drag_memory.drag_source(), None);
    assert!(
        reconciled
            .output
            .requests
            .iter()
            .all(|request| !matches!(request, OutlinerRequest::Drop(_)))
    );
    assert_eq!(reconciled.frame.repaint, RepaintRequest::NextFrame);
}

#[test]
fn disabled_or_read_only_affordances_fail_closed() {
    let mut flags = OutlinerRowFlags::new();
    flags.read_only = true;
    let model = OutlinerModel::new(vec![
        OutlinerItem::new(id(1), "Read only").with_flags(flags),
    ]);
    let mut state = OutlinerState::new();
    let mut memory = UiMemory::new();
    let idle = run_frame(
        &model,
        config(),
        &mut state,
        &mut memory,
        UiInput::default(),
        false,
    );
    let zones = idle.rows[0].clone();

    for point in [
        zones.visibility_toggle_rect.center(),
        zones.lock_toggle_rect.center(),
    ] {
        let inert = click(point, 1, &model, &mut state, &mut memory);
        assert!(inert.output.requests.is_empty());
    }
}
