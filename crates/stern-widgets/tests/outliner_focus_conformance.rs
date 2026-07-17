//! Outliner inward-focus ownership and composition conformance tests.

#![allow(clippy::float_cmp)]

use std::time::Duration;

use stern_core::{
    ActionDescriptor, Brush, Color, ComponentState, FrameContext, Key, KeyEvent, KeyState,
    KeyboardInput, Modifiers, PathElement, PhysicalSize, Point, PointerButtonState, PointerInput,
    PointerOrder, Primitive, Rect, RepaintRequest, ScaleFactor, SemanticActionKind, SemanticNode,
    Size, TimeInfo, UiInput, UiMemory, Vec2, ViewportInfo, WidgetId, default_dark_theme,
};
use stern_widgets::outliner::{
    OutlinerConfig, OutlinerOutput, OutlinerRequest, OutlinerSelectionMode, OutlinerState,
};
use stern_widgets::{
    CollectionContextTarget, InlineEditCancelReason, InlineEditCommitReason, InlineEditRequest,
    ItemId, OutlinerItem, OutlinerModel, OutlinerRowFlags, OutlinerRowZones, Ui,
};

const BOUNDS: Rect = Rect::new(10.25, 20.5, 260.0, 120.0);

fn id(raw: u64) -> ItemId {
    ItemId::from_raw(raw)
}

fn config(bounds: Rect) -> OutlinerConfig {
    OutlinerConfig::new(bounds, 24.0, 16.0)
        .label("Scene hierarchy")
        .overscan(1)
        .selection_mode(OutlinerSelectionMode::Multiple)
}

fn context(input: UiInput) -> FrameContext {
    FrameContext::new(
        ViewportInfo::new(
            Size::new(360.0, 260.0),
            PhysicalSize::new(360, 260),
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
    UiInput {
        keyboard: KeyboardInput {
            modifiers: Modifiers::default(),
            events: vec![KeyEvent::new(
                key,
                KeyState::Pressed,
                Modifiers::default(),
                false,
            )],
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

#[derive(Debug)]
struct Run {
    root: WidgetId,
    outside: WidgetId,
    rows: Vec<OutlinerRowZones>,
    output: OutlinerOutput,
    frame: stern_core::FrameOutput,
}

fn run_frame(
    model: &OutlinerModel,
    cfg: OutlinerConfig,
    state: &mut OutlinerState,
    memory: &mut UiMemory,
    input: UiInput,
) -> Run {
    let theme = default_dark_theme();
    let mut ui = Ui::begin_frame(context(input), memory, &theme);
    let scene = ui
        .prepare_outliner("focus-outliner", cfg, model, state)
        .expect("valid outliner scene");
    let root = scene.widget_id();
    let outside = ui.make_id("outside-focus");
    ui.register_id(outside);
    let rows = scene.rows().to_vec();
    ui.resolve_pointer_targets(|plan| {
        scene.declare_pointer_targets(plan, PointerOrder::new(100), state);
    })
    .expect("valid shared pointer plan");
    let output = ui.outliner(&scene, state, |target| match target {
        CollectionContextTarget::Background(_) => {
            vec![ActionDescriptor::new("scene.create", "Create")]
        }
        CollectionContextTarget::Item(_) | CollectionContextTarget::Selection(_) => {
            vec![ActionDescriptor::new("scene.delete", "Delete")]
        }
    });
    let frame = ui.finish_output();
    Run {
        root,
        outside,
        rows,
        output,
        frame,
    }
}

fn click(
    point: Point,
    click_count: u8,
    model: &OutlinerModel,
    cfg: OutlinerConfig,
    state: &mut OutlinerState,
    memory: &mut UiMemory,
) -> Run {
    let _ = run_frame(
        model,
        cfg.clone(),
        state,
        memory,
        primary_input(point, true, true, false, click_count),
    );
    run_frame(
        model,
        cfg,
        state,
        memory,
        primary_input(point, false, false, true, click_count),
    )
}

fn context_click(
    point: Point,
    model: &OutlinerModel,
    cfg: OutlinerConfig,
    state: &mut OutlinerState,
    memory: &mut UiMemory,
) -> Run {
    let _ = run_frame(
        model,
        cfg.clone(),
        state,
        memory,
        secondary_input(point, true, true, false),
    );
    run_frame(
        model,
        cfg,
        state,
        memory,
        secondary_input(point, false, false, true),
    )
}

fn row_response(run: &Run, target: ItemId) -> stern_widgets::outliner::OutlinerRowResponse {
    *run.output
        .responses
        .iter()
        .find(|response| response.item == target)
        .unwrap_or_else(|| panic!("missing response for row {}", target.raw()))
}

fn assert_isolated_nested_click(run: &Run, target: ItemId, target_kind: &str) {
    let response = row_response(run, target);
    assert!(!response.row.clicked);
    assert_eq!(
        response.disclosure.expect("disclosure response").clicked,
        target_kind == "disclosure"
    );
    assert_eq!(
        response.visibility.expect("visibility response").clicked,
        target_kind == "visibility"
    );
    assert_eq!(
        response.lock.expect("lock response").clicked,
        target_kind == "lock"
    );
    assert!(!run.output.selection_changed);
    assert_eq!(run.output.activated, None);
    assert_eq!(run.output.context_opened, None);
}

fn row_zones(run: &Run, target: ItemId) -> &OutlinerRowZones {
    run.rows
        .iter()
        .find(|zones| zones.row.id == target)
        .unwrap_or_else(|| panic!("missing geometry for row {}", target.raw()))
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

fn visibility_icon(zones: &OutlinerRowZones) -> Rect {
    let inset = zones
        .visibility_toggle_rect
        .width
        .min(zones.visibility_toggle_rect.height)
        * 0.25;
    Rect::new(
        zones.visibility_toggle_rect.x + inset,
        zones.visibility_toggle_rect.y + inset,
        (zones.visibility_toggle_rect.width - inset * 2.0).max(0.0),
        (zones.visibility_toggle_rect.height - inset * 2.0).max(0.0),
    )
}

fn lock_body(zones: &OutlinerRowZones) -> Rect {
    let width = zones.lock_toggle_rect.width * 0.42;
    let height = zones.lock_toggle_rect.height * 0.34;
    Rect::new(
        zones.lock_toggle_rect.center().x - width * 0.5,
        zones.lock_toggle_rect.center().y,
        width,
        height,
    )
}

#[allow(clippy::too_many_lines)]
fn assert_row_focus(run: &Run, target: ItemId) -> [Primitive; 2] {
    let theme = default_dark_theme();
    let zones = row_zones(run, target);
    let response = row_response(run, target);
    assert!(response.row.state.focused);
    assert!(zones.row.flags.can_request_selection());
    assert!(!response.row.state.disabled);
    let hovered = response.row.state.hovered
        || response
            .disclosure
            .is_some_and(|nested| nested.state.hovered)
        || response
            .visibility
            .is_some_and(|nested| nested.state.hovered)
        || response.lock.is_some_and(|nested| nested.state.hovered);
    let pressed = response.row.state.pressed
        || response
            .disclosure
            .is_some_and(|nested| nested.state.pressed)
        || response
            .visibility
            .is_some_and(|nested| nested.state.pressed)
        || response.lock.is_some_and(|nested| nested.state.pressed);
    let state = ComponentState {
        hovered,
        pressed,
        focused: true,
        disabled: false,
        selected: response.row.state.selected,
    };
    let recipe = theme.row(state);
    let base_index = run
        .frame
        .primitives
        .iter()
        .position(|primitive| matches!(primitive, Primitive::Rect(base) if base.rect == zones.rect))
        .expect("outliner row base");
    let Primitive::Rect(base) = &run.frame.primitives[base_index] else {
        unreachable!()
    };
    assert_eq!(base.fill, Some(recipe.background));
    assert_eq!(base.stroke, Some(recipe.border));
    assert_eq!(base.radius, recipe.radius);

    let expected = theme
        .focus_ring(true)
        .expect("focus recipe")
        .inward_annulus_primitives(zones.rect, recipe.radius, recipe.border.width);
    assert_eq!(run.frame.primitives[base_index + 1], expected[0]);
    assert_eq!(run.frame.primitives[base_index + 2], expected[1]);
    for primitive in &run.frame.primitives[base_index + 1..=base_index + 2] {
        let Primitive::Path(path) = primitive else {
            panic!("focus band must be a path")
        };
        assert!(path.fill.is_some());
        assert!(path.stroke.is_none());
        assert!(path.elements.iter().all(|element| match *element {
            PathElement::MoveTo(point) | PathElement::LineTo(point) =>
                point.x.is_finite() && point.y.is_finite(),
            PathElement::QuadTo { ctrl, to } => {
                ctrl.x.is_finite() && ctrl.y.is_finite() && to.x.is_finite() && to.y.is_finite()
            }
            PathElement::CubicTo { ctrl1, ctrl2, to } => {
                ctrl1.x.is_finite()
                    && ctrl1.y.is_finite()
                    && ctrl2.x.is_finite()
                    && ctrl2.y.is_finite()
                    && to.x.is_finite()
                    && to.y.is_finite()
            }
            PathElement::Close => true,
        }));
        let bounds = path_bounds(&path.elements);
        assert!(bounds.x >= zones.rect.x);
        assert!(bounds.y >= zones.rect.y);
        assert!(bounds.max_x() <= zones.rect.max_x());
        assert!(bounds.max_y() <= zones.rect.max_y());
    }

    let mut content = base_index + 3;
    if zones.row.has_children {
        assert!(matches!(run.frame.primitives[content], Primitive::Line(_)));
        assert!(matches!(
            run.frame.primitives[content + 1],
            Primitive::Line(_)
        ));
        content += 2;
    }
    if zones.row.flags.visibility_toggle_available {
        assert!(matches!(
            run.frame.primitives[content],
            Primitive::Rect(icon) if icon.rect == visibility_icon(zones)
        ));
        content += 1;
        if !zones.row.flags.visible {
            assert!(matches!(run.frame.primitives[content], Primitive::Line(_)));
            content += 1;
        }
    }
    if zones.row.flags.lock_toggle_available {
        assert!(matches!(
            run.frame.primitives[content],
            Primitive::Rect(icon) if icon.rect == lock_body(zones)
        ));
        assert!(
            run.frame.primitives[content + 1..=content + 3]
                .iter()
                .all(|primitive| matches!(primitive, Primitive::Line(_)))
        );
        content += 4;
    }
    assert!(matches!(
        run.frame.primitives[content],
        Primitive::Text(ref text) if text.text == zones.row.label
    ));
    assert!(
        run.frame
            .primitives
            .iter()
            .all(|primitive| !matches!(primitive, Primitive::TransformBegin { .. }))
    );

    [
        run.frame.primitives[base_index + 1].clone(),
        run.frame.primitives[base_index + 2].clone(),
    ]
}

fn assert_no_row_annuli(run: &Run, target: ItemId) {
    let base_index = run
        .frame
        .primitives
        .iter()
        .position(|primitive| {
            matches!(primitive, Primitive::Rect(base) if base.rect == row_zones(run, target).rect)
        })
        .expect("outliner row base");
    assert!(!matches!(
        run.frame.primitives.get(base_index + 1),
        Some(Primitive::Path(_))
    ));
}

fn primitives_without_paths(run: &Run) -> Vec<Primitive> {
    run.frame
        .primitives
        .iter()
        .filter(|primitive| !matches!(primitive, Primitive::Path(_)))
        .cloned()
        .collect()
}

fn output_without_focus(mut output: OutlinerOutput) -> OutlinerOutput {
    for response in &mut output.responses {
        response.row.state.focused = false;
    }
    output
}

fn semantics_without_focus(run: &Run) -> Vec<SemanticNode> {
    run.frame
        .semantics
        .nodes()
        .iter()
        .cloned()
        .map(|mut node| {
            node.state.focused = false;
            node
        })
        .collect()
}

fn state_for(expanded: bool, selected: bool) -> OutlinerState {
    let mut state = OutlinerState::new();
    if expanded {
        state.expansion.expand(id(1));
    }
    if selected {
        state.selection.replace(id(1));
    }
    state
}

fn configured_item(flags: OutlinerRowFlags, has_children: bool) -> OutlinerModel {
    OutlinerModel::new(vec![
        OutlinerItem::new(id(1), "Configured row")
            .with_has_children(has_children)
            .with_flags(flags),
    ])
}

fn owned_model(flags: OutlinerRowFlags) -> OutlinerModel {
    OutlinerModel::new(vec![
        OutlinerItem::new(id(1), "Owned row")
            .with_has_children(true)
            .with_flags(flags),
        OutlinerItem::new(id(2), "Child row").with_parent(id(1)),
    ])
}

#[test]
#[allow(clippy::too_many_lines)]
fn selected_and_unselected_branch_leaf_control_matrices_add_only_exact_owned_annuli() {
    for has_children in [false, true] {
        for expanded in if has_children {
            [false, true].as_slice()
        } else {
            [false].as_slice()
        } {
            for visibility_available in [false, true] {
                for visible in [false, true] {
                    for lock_available in [false, true] {
                        for locked in [false, true] {
                            for selected in [false, true] {
                                let mut flags = OutlinerRowFlags::new();
                                flags.visibility_toggle_available = visibility_available;
                                flags.visible = visible;
                                flags.lock_toggle_available = lock_available;
                                flags.locked = locked;
                                let model = configured_item(flags, has_children);
                                let cfg = config(BOUNDS);
                                let mut unfocused_state = state_for(*expanded, selected);
                                let unfocused = run_frame(
                                    &model,
                                    cfg.clone(),
                                    &mut unfocused_state,
                                    &mut UiMemory::new(),
                                    UiInput::default(),
                                );
                                let mut focused_state = state_for(*expanded, selected);
                                let mut focused_memory = UiMemory::new();
                                focused_memory.focus(unfocused.root.child(("outliner-row", 1_u64)));
                                let focused = run_frame(
                                    &model,
                                    cfg,
                                    &mut focused_state,
                                    &mut focused_memory,
                                    UiInput::default(),
                                );

                                assert_eq!(focused.rows, unfocused.rows);
                                assert_eq!(focused.frame.repaint, unfocused.frame.repaint);
                                assert_eq!(
                                    output_without_focus(focused.output.clone()),
                                    unfocused.output
                                );
                                assert_eq!(
                                    primitives_without_paths(&focused),
                                    unfocused.frame.primitives
                                );
                                assert_eq!(
                                    semantics_without_focus(&focused),
                                    unfocused.frame.semantics.nodes()
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
                                assert_eq!(
                                    row_response(&focused, id(1)).row.state.selected,
                                    selected
                                );
                                assert_row_focus(&focused, id(1));
                            }
                        }
                    }
                }
            }
        }
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn row_nested_drag_and_completed_control_transactions_preserve_owned_focus() {
    let flags = OutlinerRowFlags::new();
    let model = owned_model(flags);
    let cfg = config(BOUNDS);

    for target_kind in ["row", "disclosure", "visibility", "lock"] {
        let mut state = OutlinerState::new();
        let mut memory = UiMemory::new();
        let seed = run_frame(
            &model,
            cfg.clone(),
            &mut state,
            &mut memory,
            UiInput::default(),
        );
        let selected = click(
            row_zones(&seed, id(1)).label_rect.center(),
            1,
            &model,
            cfg.clone(),
            &mut state,
            &mut memory,
        );
        let expected = assert_row_focus(&selected, id(1));
        let zones = row_zones(&selected, id(1));
        let point = match target_kind {
            "row" => zones.label_rect.center(),
            "disclosure" => zones.disclosure_rect.center(),
            "visibility" => zones.visibility_toggle_rect.center(),
            "lock" => zones.lock_toggle_rect.center(),
            _ => unreachable!(),
        };
        let hovered = run_frame(
            &model,
            cfg.clone(),
            &mut state,
            &mut memory,
            primary_input(point, false, false, false, 0),
        );
        assert_eq!(assert_row_focus(&hovered, id(1)), expected);
        let pressed = run_frame(
            &model,
            cfg.clone(),
            &mut state,
            &mut memory,
            primary_input(point, true, true, false, 1),
        );
        assert_eq!(assert_row_focus(&pressed, id(1)), expected);
    }

    for target_kind in ["disclosure", "visibility", "lock"] {
        let mut state = OutlinerState::new();
        let mut memory = UiMemory::new();
        let seed = run_frame(
            &model,
            cfg.clone(),
            &mut state,
            &mut memory,
            UiInput::default(),
        );
        let selected = click(
            row_zones(&seed, id(1)).label_rect.center(),
            1,
            &model,
            cfg.clone(),
            &mut state,
            &mut memory,
        );
        let before_paths = assert_row_focus(&selected, id(1));
        let row_id = selected.root.child(("outliner-row", 1_u64));
        assert_eq!(memory.focused(), Some(row_id));
        assert_eq!(state.cursor.active(), Some(id(1)));
        assert_eq!(state.selection.selected(), vec![id(1)]);
        let selected_semantic = selected.frame.semantics.get(row_id).expect("row semantic");
        assert!(selected_semantic.state.focused);
        assert!(selected_semantic.state.selected);
        let zones = row_zones(&selected, id(1));
        let point = match target_kind {
            "disclosure" => zones.disclosure_rect.center(),
            "visibility" => zones.visibility_toggle_rect.center(),
            "lock" => zones.lock_toggle_rect.center(),
            _ => unreachable!(),
        };
        let completed = click(point, 1, &model, cfg.clone(), &mut state, &mut memory);
        assert_isolated_nested_click(&completed, id(1), target_kind);
        assert_eq!(state.cursor.active(), Some(id(1)));
        assert_eq!(state.selection.selected(), vec![id(1)]);
        assert_eq!(assert_row_focus(&completed, id(1)), before_paths);
        match target_kind {
            "disclosure" => {
                assert!(completed.output.expansion_changed);
                assert!(completed.output.requests.is_empty());
                assert!(state.expansion.is_expanded(id(1)));
                let following = run_frame(
                    &model,
                    cfg.clone(),
                    &mut state,
                    &mut memory,
                    UiInput::default(),
                );
                assert!(row_zones(&following, id(1)).row.expanded);
                assert_eq!(assert_row_focus(&following, id(1)), before_paths);
            }
            "visibility" => {
                assert!(!completed.output.expansion_changed);
                assert!(matches!(
                    completed.output.requests.as_slice(),
                    [OutlinerRequest::Visibility(request)]
                        if request.target == id(1) && request.visible
                ));
                assert!(!state.expansion.is_expanded(id(1)));
                let mut updated_flags = flags;
                updated_flags.visible = false;
                let updated = owned_model(updated_flags);
                let following = run_frame(
                    &updated,
                    cfg.clone(),
                    &mut state,
                    &mut memory,
                    UiInput::default(),
                );
                assert!(!row_zones(&following, id(1)).row.flags.visible);
                assert_eq!(assert_row_focus(&following, id(1)), before_paths);
            }
            "lock" => {
                assert!(!completed.output.expansion_changed);
                assert!(matches!(
                    completed.output.requests.as_slice(),
                    [OutlinerRequest::Lock(request)]
                        if request.target == id(1) && !request.locked
                ));
                assert!(!state.expansion.is_expanded(id(1)));
                let mut updated_flags = flags;
                updated_flags.locked = true;
                let updated = owned_model(updated_flags);
                let following = run_frame(
                    &updated,
                    cfg.clone(),
                    &mut state,
                    &mut memory,
                    UiInput::default(),
                );
                assert!(row_zones(&following, id(1)).row.flags.locked);
                assert_eq!(assert_row_focus(&following, id(1)), before_paths);
            }
            _ => unreachable!(),
        }
    }

    for target_kind in ["disclosure", "visibility", "lock"] {
        let mut state = OutlinerState::new();
        let mut memory = UiMemory::new();
        let seed = run_frame(
            &model,
            cfg.clone(),
            &mut state,
            &mut memory,
            UiInput::default(),
        );
        let row_id = seed.root.child(("outliner-row", 1_u64));
        assert_eq!(memory.focused(), None);
        assert_eq!(state.cursor.active(), None);
        assert!(state.selection.selected().is_empty());
        let seed_semantic = seed.frame.semantics.get(row_id).expect("row semantic");
        assert!(!seed_semantic.state.focused);
        assert!(!seed_semantic.state.selected);

        let zones = row_zones(&seed, id(1));
        let point = match target_kind {
            "disclosure" => zones.disclosure_rect.center(),
            "visibility" => zones.visibility_toggle_rect.center(),
            "lock" => zones.lock_toggle_rect.center(),
            _ => unreachable!(),
        };
        let completed = click(point, 1, &model, cfg.clone(), &mut state, &mut memory);
        assert_isolated_nested_click(&completed, id(1), target_kind);
        assert_eq!(memory.focused(), None);
        assert_eq!(state.cursor.active(), None);
        assert!(state.selection.selected().is_empty());
        assert!(!row_response(&completed, id(1)).row.state.focused);
        assert_no_row_annuli(&completed, id(1));
        let semantic = completed.frame.semantics.get(row_id).expect("row semantic");
        assert!(!semantic.state.focused);
        assert!(!semantic.state.selected);

        match target_kind {
            "disclosure" => {
                assert!(completed.output.expansion_changed);
                assert!(completed.output.requests.is_empty());
                assert!(state.expansion.is_expanded(id(1)));
            }
            "visibility" => {
                assert!(!completed.output.expansion_changed);
                assert!(matches!(
                    completed.output.requests.as_slice(),
                    [OutlinerRequest::Visibility(request)]
                        if request.target == id(1) && request.visible
                ));
                assert!(!state.expansion.is_expanded(id(1)));
            }
            "lock" => {
                assert!(!completed.output.expansion_changed);
                assert!(matches!(
                    completed.output.requests.as_slice(),
                    [OutlinerRequest::Lock(request)]
                        if request.target == id(1) && !request.locked
                ));
                assert!(!state.expansion.is_expanded(id(1)));
            }
            _ => unreachable!(),
        }
    }

    let mut drag_state = OutlinerState::new();
    let mut drag_memory = UiMemory::new();
    let seed = run_frame(
        &model,
        cfg.clone(),
        &mut drag_state,
        &mut drag_memory,
        UiInput::default(),
    );
    let row_point = row_zones(&seed, id(1)).label_rect.center();
    let selected = click(
        row_point,
        1,
        &model,
        cfg.clone(),
        &mut drag_state,
        &mut drag_memory,
    );
    let expected = assert_row_focus(&selected, id(1));
    let _ = run_frame(
        &model,
        cfg.clone(),
        &mut drag_state,
        &mut drag_memory,
        primary_input(row_point, true, true, false, 1),
    );
    let dragged = run_frame(
        &model,
        cfg,
        &mut drag_state,
        &mut drag_memory,
        move_input(
            Point::new(row_point.x + 12.0, row_point.y),
            Vec2::new(12.0, 0.0),
        ),
    );
    assert!(row_response(&dragged, id(1)).row.dragged);
    assert_eq!(drag_state.selection.selected(), vec![id(1)]);
    assert_eq!(assert_row_focus(&dragged, id(1)), expected);
}

#[test]
#[allow(clippy::too_many_lines)]
fn nested_drop_background_and_overlay_focus_never_become_row_focus() {
    let model = owned_model(OutlinerRowFlags::new());
    let cfg = config(BOUNDS);
    let mut seed_state = OutlinerState::new();
    let seed = run_frame(
        &model,
        cfg.clone(),
        &mut seed_state,
        &mut UiMemory::new(),
        UiInput::default(),
    );
    let row_id = seed.root.child(("outliner-row", 1_u64));
    for owner in [
        row_id.child("disclosure"),
        row_id.child("visibility"),
        row_id.child("lock"),
        row_id.child("drop"),
        seed.root.child("background"),
    ] {
        let mut state = OutlinerState::new();
        state.selection.replace(id(1));
        let mut memory = UiMemory::new();
        memory.focus(owner);
        let run = run_frame(
            &model,
            cfg.clone(),
            &mut state,
            &mut memory,
            UiInput::default(),
        );
        assert!(!row_response(&run, id(1)).row.state.focused);
        assert_no_row_annuli(&run, id(1));
        let semantic = run.frame.semantics.get(row_id).expect("row semantic");
        assert!(!semantic.state.focused);
        assert!(semantic.state.selected);
    }

    let mut context_state = OutlinerState::new();
    let mut context_memory = UiMemory::new();
    let selected = click(
        row_zones(&seed, id(1)).label_rect.center(),
        1,
        &model,
        cfg.clone(),
        &mut context_state,
        &mut context_memory,
    );
    let expected = assert_row_focus(&selected, id(1));
    let opened = context_click(
        row_zones(&selected, id(1)).context_rect.center(),
        &model,
        cfg.clone(),
        &mut context_state,
        &mut context_memory,
    );
    assert_eq!(assert_row_focus(&opened, id(1)), expected);
    assert_eq!(context_memory.focused(), Some(row_id));
    let menu = run_frame(
        &model,
        cfg.clone(),
        &mut context_state,
        &mut context_memory,
        UiInput::default(),
    );
    assert_eq!(assert_row_focus(&menu, id(1)), expected);
    let overlay_row = menu
        .frame
        .semantics
        .nodes()
        .iter()
        .find(|node| node.label.as_deref() == Some("Delete"))
        .expect("context overlay row")
        .id;
    context_memory.focus(overlay_row);
    let overlay_focused = run_frame(
        &model,
        cfg,
        &mut context_state,
        &mut context_memory,
        UiInput::default(),
    );
    assert!(!row_response(&overlay_focused, id(1)).row.state.focused);
    assert_no_row_annuli(&overlay_focused, id(1));
    assert!(
        !overlay_focused
            .frame
            .semantics
            .get(row_id)
            .expect("row semantic")
            .state
            .focused
    );
}

#[test]
fn context_escape_dismissal_restores_outliner_trigger_focus_without_mutating_selection() {
    let model = owned_model(OutlinerRowFlags::new());
    let cfg = config(BOUNDS);
    let mut state = OutlinerState::new();
    let mut memory = UiMemory::new();
    let idle = run_frame(
        &model,
        cfg.clone(),
        &mut state,
        &mut memory,
        UiInput::default(),
    );
    let trigger_point = row_zones(&idle, id(1)).label_rect.center();
    let selected = click(
        trigger_point,
        1,
        &model,
        cfg.clone(),
        &mut state,
        &mut memory,
    );
    let trigger = selected.root.child(("outliner-row", 1_u64));
    let expected_cursor = state.cursor.active();
    let expected_selection = state.selection.selected();
    assert_eq!(expected_cursor, Some(id(1)));
    assert_eq!(expected_selection, vec![id(1)]);
    assert_eq!(memory.focused(), Some(trigger));

    let opened = context_click(
        row_zones(&selected, id(1)).context_rect.center(),
        &model,
        cfg.clone(),
        &mut state,
        &mut memory,
    );
    assert_eq!(
        opened.output.context_opened,
        state.context_target().cloned()
    );
    assert_eq!(memory.focused(), Some(trigger));
    let menu = run_frame(
        &model,
        cfg.clone(),
        &mut state,
        &mut memory,
        UiInput::default(),
    );
    let action = menu
        .frame
        .semantics
        .nodes()
        .iter()
        .find(|node| node.label.as_deref() == Some("Delete"))
        .expect("selected outliner context action")
        .id;
    memory.focus(action);
    assert_eq!(memory.focused(), Some(action));

    let dismissed = run_frame(
        &model,
        cfg.clone(),
        &mut state,
        &mut memory,
        key_input(Key::Escape),
    );
    assert_eq!(state.context_target(), None);
    assert_eq!(memory.focused(), Some(trigger));
    assert_eq!(state.cursor.active(), expected_cursor);
    assert_eq!(state.selection.selected(), expected_selection);
    assert!(dismissed.output.requests.is_empty());
    assert!(dismissed.frame.actions.is_empty());
    assert_eq!(dismissed.frame.repaint, RepaintRequest::NextFrame);

    let settled = run_frame(&model, cfg, &mut state, &mut memory, UiInput::default());
    assert_eq!(memory.focused(), Some(trigger));
    assert_eq!(state.cursor.active(), expected_cursor);
    assert_eq!(state.selection.selected(), expected_selection);
    assert!(
        settled
            .frame
            .semantics
            .nodes()
            .iter()
            .all(|node| node.label.as_deref() != Some("Delete"))
    );
}

fn row_label_is_painted(run: &Run, target: ItemId, label: &str) -> bool {
    let theme = default_dark_theme();
    let zones = row_zones(run, target);
    let font = theme.font(stern_core::TextRole::Label);
    let extra = (zones.label_rect.height - font.line_height).max(0.0) * 0.5;
    let origin = Point::new(
        zones.label_rect.x + theme.controls.padding_x,
        zones.label_rect.y + extra + font.size,
    );
    run.frame.primitives.iter().any(|primitive| {
        matches!(primitive, Primitive::Text(text) if text.text == label && text.origin == origin)
    })
}

fn start_rename(
    model: &OutlinerModel,
    cfg: OutlinerConfig,
    state: &mut OutlinerState,
    memory: &mut UiMemory,
) -> (Run, Run) {
    let seed = run_frame(model, cfg.clone(), state, memory, UiInput::default());
    let selected = click(
        row_zones(&seed, id(1)).label_rect.center(),
        1,
        model,
        cfg.clone(),
        state,
        memory,
    );
    assert_row_focus(&selected, id(1));
    let begin = run_frame(model, cfg, state, memory, key_input(Key::Function(2)));
    assert!(matches!(
        begin.output.requests.as_slice(),
        [OutlinerRequest::Rename(InlineEditRequest::Begin(request))]
            if request.target == id(1)
    ));
    (selected, begin)
}

#[test]
#[allow(clippy::too_many_lines)]
fn rename_transfers_focus_omits_row_label_and_restores_annuli_after_terminal_frames() {
    let model = OutlinerModel::new(vec![OutlinerItem::new(id(1), "Editable row")]);
    let cfg = config(BOUNDS);

    let mut commit_state = OutlinerState::new();
    let mut commit_memory = UiMemory::new();
    let (_, begin) = start_rename(&model, cfg.clone(), &mut commit_state, &mut commit_memory);
    assert_no_row_annuli(&begin, id(1));
    assert!(row_label_is_painted(&begin, id(1), "Editable row"));
    let editing = run_frame(
        &model,
        cfg.clone(),
        &mut commit_state,
        &mut commit_memory,
        UiInput::default(),
    );
    assert_no_row_annuli(&editing, id(1));
    assert!(!row_label_is_painted(&editing, id(1), "Editable row"));
    assert!(
        editing
            .frame
            .semantics
            .get(editing.root.child(("outliner-row", 1_u64)))
            .is_none()
    );
    let typed = run_frame(
        &model,
        cfg.clone(),
        &mut commit_state,
        &mut commit_memory,
        typed_input(" renamed"),
    );
    assert!(matches!(
        typed.output.requests.as_slice(),
        [OutlinerRequest::Rename(InlineEditRequest::DraftEdit(request))]
            if request.target == id(1)
    ));
    let committed = run_frame(
        &model,
        cfg.clone(),
        &mut commit_state,
        &mut commit_memory,
        key_input(Key::Enter),
    );
    assert!(matches!(
        committed.output.requests.as_slice(),
        [OutlinerRequest::Rename(InlineEditRequest::Commit(request))]
            if request.target == id(1) && request.reason == InlineEditCommitReason::Enter
    ));
    assert_no_row_annuli(&committed, id(1));
    let commit_restored = run_frame(
        &model,
        cfg.clone(),
        &mut commit_state,
        &mut commit_memory,
        UiInput::default(),
    );
    assert_row_focus(&commit_restored, id(1));

    let mut cancel_state = OutlinerState::new();
    let mut cancel_memory = UiMemory::new();
    let (_, _) = start_rename(&model, cfg.clone(), &mut cancel_state, &mut cancel_memory);
    let _ = run_frame(
        &model,
        cfg.clone(),
        &mut cancel_state,
        &mut cancel_memory,
        UiInput::default(),
    );
    let cancelled = run_frame(
        &model,
        cfg.clone(),
        &mut cancel_state,
        &mut cancel_memory,
        key_input(Key::Escape),
    );
    assert!(matches!(
        cancelled.output.requests.as_slice(),
        [OutlinerRequest::Rename(InlineEditRequest::Cancel(request))]
            if request.target == id(1) && request.reason == InlineEditCancelReason::Escape
    ));
    assert_no_row_annuli(&cancelled, id(1));
    let cancel_restored = run_frame(
        &model,
        cfg.clone(),
        &mut cancel_state,
        &mut cancel_memory,
        UiInput::default(),
    );
    assert_row_focus(&cancel_restored, id(1));

    let mut loss_state = OutlinerState::new();
    let mut loss_memory = UiMemory::new();
    let (_, _) = start_rename(&model, cfg.clone(), &mut loss_state, &mut loss_memory);
    let editing = run_frame(
        &model,
        cfg.clone(),
        &mut loss_state,
        &mut loss_memory,
        typed_input(" changed"),
    );
    loss_memory.focus(editing.outside);
    let focus_lost = run_frame(
        &model,
        cfg.clone(),
        &mut loss_state,
        &mut loss_memory,
        UiInput::default(),
    );
    assert!(matches!(
        focus_lost.output.requests.as_slice(),
        [OutlinerRequest::Rename(InlineEditRequest::Commit(request))]
            if request.target == id(1) && request.reason == InlineEditCommitReason::FocusLost
    ));
    assert_no_row_annuli(&focus_lost, id(1));
    let loss_restored = run_frame(
        &model,
        cfg,
        &mut loss_state,
        &mut loss_memory,
        UiInput::default(),
    );
    assert_row_focus(&loss_restored, id(1));
}

fn roots(count: usize) -> OutlinerModel {
    OutlinerModel::new(
        (0..count)
            .map(|index| OutlinerItem::new(id(index as u64), format!("Row {index}")))
            .collect::<Vec<_>>(),
    )
}

#[test]
#[allow(clippy::too_many_lines)]
fn ten_thousand_fractionally_scrolled_rows_preserve_clip_window_and_focus_geometry() {
    let model = roots(10_000);
    let cfg = config(BOUNDS);
    let mut seed_state = OutlinerState::new();
    let seed = run_frame(
        &model,
        cfg.clone(),
        &mut seed_state,
        &mut UiMemory::new(),
        UiInput::default(),
    );
    let mut probe_state = OutlinerState::new();
    let mut probe_memory = UiMemory::new();
    probe_memory.set_scroll_offset(seed.root, Vec2::new(0.0, 12.5));
    let probe = run_frame(
        &model,
        cfg.clone(),
        &mut probe_state,
        &mut probe_memory,
        UiInput::default(),
    );
    assert_eq!(probe.output.window.clamped_scroll_offset, 12.5);
    assert_eq!(probe.output.window.content_extent, 240_000.0);
    assert_eq!(
        probe.rows.len(),
        probe.output.window.materialized_range.len()
    );
    assert!(probe.rows[0].rect.y < BOUNDS.y);
    assert!(probe.rows[0].rect.max_y() > BOUNDS.y);
    assert!(
        probe
            .rows
            .iter()
            .any(|row| row.rect.y < BOUNDS.max_y() && row.rect.max_y() > BOUNDS.max_y())
    );
    assert!(
        probe
            .rows
            .last()
            .is_some_and(|row| row.rect.y >= BOUNDS.max_y())
    );

    let targets = [
        probe.rows[0].row.id,
        probe.rows[probe.rows.len() / 2].row.id,
        probe.rows[probe.rows.len() - 1].row.id,
    ];
    for target in targets {
        let mut unfocused_state = OutlinerState::new();
        unfocused_state.selection.replace(target);
        let mut unfocused_memory = UiMemory::new();
        unfocused_memory.set_scroll_offset(probe.root, Vec2::new(0.0, 12.5));
        let unfocused = run_frame(
            &model,
            cfg.clone(),
            &mut unfocused_state,
            &mut unfocused_memory,
            UiInput::default(),
        );
        let mut focused_state = OutlinerState::new();
        focused_state.selection.replace(target);
        let mut focused_memory = UiMemory::new();
        focused_memory.set_scroll_offset(probe.root, Vec2::new(0.0, 12.5));
        focused_memory.focus(probe.root.child(("outliner-row", target.raw())));
        let focused = run_frame(
            &model,
            cfg.clone(),
            &mut focused_state,
            &mut focused_memory,
            UiInput::default(),
        );

        assert_eq!(focused.rows, unfocused.rows);
        assert_eq!(focused.output.window, probe.output.window);
        assert_eq!(unfocused.output.window, probe.output.window);
        assert_eq!(
            output_without_focus(focused.output.clone()),
            unfocused.output
        );
        assert_eq!(
            primitives_without_paths(&focused),
            unfocused.frame.primitives
        );
        assert_eq!(
            semantics_without_focus(&focused),
            unfocused.frame.semantics.nodes()
        );
        assert_row_focus(&focused, target);
        let clip_begin = focused
            .frame
            .primitives
            .iter()
            .position(|primitive| {
                matches!(primitive, Primitive::ClipBegin { rect, .. } if *rect == BOUNDS)
            })
            .expect("outliner clip begins");
        let clip_end = focused
            .frame
            .primitives
            .iter()
            .position(|primitive| matches!(primitive, Primitive::ClipEnd { .. }))
            .expect("outliner clip ends");
        let base = focused
            .frame
            .primitives
            .iter()
            .position(|primitive| {
                matches!(primitive, Primitive::Rect(rect) if rect.rect == row_zones(&focused, target).rect)
            })
            .expect("focused row base");
        assert!(clip_begin < base && base + 2 < clip_end);
        assert!(
            focused
                .frame
                .primitives
                .iter()
                .all(|primitive| !matches!(primitive, Primitive::TransformBegin { .. }))
        );
    }

    for row in probe.rows.iter().filter(|row| {
        row.rect
            .intersection(BOUNDS)
            .is_some_and(|intersection| intersection.width > 0.0 && intersection.height > 0.0)
    }) {
        let semantic = probe
            .frame
            .semantics
            .get(probe.root.child(("outliner-row", row.row.id.raw())))
            .expect("visible row semantic");
        assert_eq!(
            semantic.bounds,
            row.rect.intersection(BOUNDS).expect("visible intersection")
        );
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn disabled_non_selectable_and_read_only_rows_preserve_exact_semantic_eligibility() {
    let enabled_flags = OutlinerRowFlags::new();
    let mut disabled_flags = OutlinerRowFlags::new();
    disabled_flags.disabled = true;
    let mut non_selectable_flags = OutlinerRowFlags::new();
    non_selectable_flags.selectable = false;
    let mut read_only_flags = OutlinerRowFlags::new();
    read_only_flags.read_only = true;
    let model = OutlinerModel::new(vec![
        OutlinerItem::new(id(1), "Enabled")
            .with_has_children(true)
            .with_flags(enabled_flags),
        OutlinerItem::new(id(2), "Disabled")
            .with_has_children(true)
            .with_flags(disabled_flags),
        OutlinerItem::new(id(3), "Non selectable")
            .with_has_children(true)
            .with_flags(non_selectable_flags),
        OutlinerItem::new(id(4), "Read only")
            .with_has_children(true)
            .with_flags(read_only_flags),
    ]);
    let cfg = config(BOUNDS);
    let mut state = OutlinerState::new();
    let mut memory = UiMemory::new();
    let seed = run_frame(
        &model,
        cfg.clone(),
        &mut state,
        &mut memory,
        UiInput::default(),
    );
    let enabled = click(
        row_zones(&seed, id(1)).label_rect.center(),
        1,
        &model,
        cfg.clone(),
        &mut state,
        &mut memory,
    );
    let expected_cursor = state.cursor.clone();
    let expected_selection = state.selection.clone();
    let mut expected_semantics = enabled.frame.semantics.nodes().to_vec();
    expected_semantics[0].state.disabled = true;
    for semantic in expected_semantics.iter_mut().skip(1) {
        semantic.state.disabled = true;
        semantic.focusable = false;
        semantic.actions.clear();
    }
    let globally_disabled = run_frame(
        &model,
        cfg.clone().disabled(true),
        &mut state,
        &mut memory,
        UiInput::default(),
    );
    assert_eq!(state.cursor, expected_cursor);
    assert_eq!(state.selection, expected_selection);
    assert_eq!(globally_disabled.rows, enabled.rows);
    assert_eq!(globally_disabled.output.window, enabled.output.window);
    assert_eq!(
        globally_disabled.frame.semantics.nodes(),
        expected_semantics
    );
    assert!(globally_disabled.output.requests.is_empty());
    assert!(
        globally_disabled
            .output
            .responses
            .iter()
            .all(|response| response.row.state.disabled)
    );
    assert!(row_response(&globally_disabled, id(1)).row.state.focused);
    assert_eq!(
        globally_disabled
            .frame
            .primitives
            .iter()
            .filter(|primitive| matches!(primitive, Primitive::Path(_)))
            .count(),
        0
    );

    let mut disabled_state = OutlinerState::new();
    disabled_state.selection.replace(id(2));
    let mut disabled_memory = UiMemory::new();
    disabled_memory.focus(seed.root.child(("outliner-row", 2_u64)));
    let disabled = run_frame(
        &model,
        cfg.clone(),
        &mut disabled_state,
        &mut disabled_memory,
        UiInput::default(),
    );
    assert!(row_response(&disabled, id(2)).row.state.focused);
    assert!(row_response(&disabled, id(2)).row.state.disabled);
    assert_no_row_annuli(&disabled, id(2));
    let semantic = disabled
        .frame
        .semantics
        .get(seed.root.child(("outliner-row", 2_u64)))
        .expect("disabled row semantic");
    assert!(semantic.state.disabled);
    assert!(semantic.state.focused);
    assert!(semantic.state.selected);
    assert!(!semantic.focusable);
    assert!(semantic.actions.is_empty());

    let mut non_selectable_state = OutlinerState::new();
    non_selectable_state.selection.replace(id(3));
    let mut non_selectable_memory = UiMemory::new();
    non_selectable_memory.focus(seed.root.child(("outliner-row", 3_u64)));
    let non_selectable = run_frame(
        &model,
        cfg.clone(),
        &mut non_selectable_state,
        &mut non_selectable_memory,
        UiInput::default(),
    );
    let response = row_response(&non_selectable, id(3));
    assert!(response.row.state.focused);
    assert!(!response.row.state.disabled);
    assert!(response.row.state.selected);
    assert_no_row_annuli(&non_selectable, id(3));
    let semantic = non_selectable
        .frame
        .semantics
        .get(seed.root.child(("outliner-row", 3_u64)))
        .expect("non-selectable row semantic");
    assert!(!semantic.focusable);
    assert!(semantic.state.focused);
    assert!(semantic.state.selected);
    assert!(semantic.actions.iter().all(|action| !matches!(
        action.kind,
        SemanticActionKind::Focus | SemanticActionKind::Invoke
    )));

    let mut read_only_state = OutlinerState::new();
    let mut read_only_memory = UiMemory::new();
    let read_only = click(
        row_zones(&seed, id(4)).label_rect.center(),
        1,
        &model,
        cfg.clone(),
        &mut read_only_state,
        &mut read_only_memory,
    );
    assert_eq!(read_only_state.cursor.active(), Some(id(4)));
    assert_eq!(read_only_state.selection.selected(), vec![id(4)]);
    assert_row_focus(&read_only, id(4));
    let response = row_response(&read_only, id(4));
    assert!(response.visibility.is_none());
    assert!(response.lock.is_none());
    let semantic = read_only
        .frame
        .semantics
        .get(seed.root.child(("outliner-row", 4_u64)))
        .expect("read-only row semantic");
    assert!(semantic.focusable);
    assert!(semantic.state.focused);
    assert!(semantic.state.selected);
    assert!(
        semantic
            .actions
            .iter()
            .any(|action| action.kind == SemanticActionKind::Focus)
    );
    assert!(
        semantic
            .actions
            .iter()
            .any(|action| action.kind == SemanticActionKind::Invoke)
    );
    assert!(semantic.actions.iter().all(|action| !matches!(
        &action.kind,
        SemanticActionKind::Custom(name)
            if matches!(name.as_str(), "rename" | "toggle-visibility" | "toggle-lock")
    )));
    assert!(
        row_zones(&read_only, id(4))
            .row
            .drag_source(&read_only_state.selection)
            .is_some()
    );
    let read_only_f2 = run_frame(
        &model,
        cfg,
        &mut read_only_state,
        &mut read_only_memory,
        key_input(Key::Function(2)),
    );
    assert!(read_only_f2.output.requests.is_empty());
    assert_row_focus(&read_only_f2, id(4));
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct RowContentColors {
    disclosure: Option<Color>,
    visibility: Option<Color>,
    lock_stroke: Option<Color>,
    lock_fill: Option<Color>,
    label: Option<Color>,
}

fn solid_color(brush: Brush) -> Color {
    match brush {
        Brush::Solid(color) => color,
        Brush::LinearGradient(_) => panic!("outliner content uses solid brushes"),
    }
}

fn row_content_colors(run: &Run, target: ItemId) -> RowContentColors {
    let zones = row_zones(run, target);
    let base = run
        .frame
        .primitives
        .iter()
        .position(|primitive| matches!(primitive, Primitive::Rect(rect) if rect.rect == zones.rect))
        .expect("row base");
    let mut index = base + 1;
    while matches!(run.frame.primitives.get(index), Some(Primitive::Path(_))) {
        index += 1;
    }
    let disclosure = zones.row.has_children.then(|| {
        let Primitive::Line(line) = run.frame.primitives[index] else {
            panic!("disclosure line")
        };
        index += 2;
        solid_color(line.stroke.brush)
    });
    let visibility = zones.row.flags.visibility_toggle_available.then(|| {
        let Primitive::Rect(icon) = run.frame.primitives[index] else {
            panic!("visibility icon")
        };
        index += 1;
        if !zones.row.flags.visible {
            index += 1;
        }
        solid_color(icon.stroke.expect("visibility stroke").brush)
    });
    let (lock_stroke, lock_fill) = if zones.row.flags.lock_toggle_available {
        let Primitive::Rect(icon) = run.frame.primitives[index] else {
            panic!("lock icon")
        };
        index += 4;
        (
            Some(solid_color(icon.stroke.expect("lock stroke").brush)),
            icon.fill.map(solid_color),
        )
    } else {
        (None, None)
    };
    let label = match run.frame.primitives.get(index) {
        Some(Primitive::Text(text)) if text.text == zones.row.label => {
            Some(solid_color(text.brush))
        }
        _ => None,
    };
    RowContentColors {
        disclosure,
        visibility,
        lock_stroke,
        lock_fill,
        label,
    }
}

fn linear_channel(channel: f32) -> f32 {
    if channel <= 0.04045 {
        channel / 12.92
    } else {
        ((channel + 0.055) / 1.055).powf(2.4)
    }
}

fn srgb_channel(channel: f32) -> f32 {
    if channel <= 0.003_130_8 {
        channel * 12.92
    } else {
        1.055 * channel.powf(1.0 / 2.4) - 0.055
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

fn blend_channel_space(foreground: Color, background: Color) -> Color {
    let alpha = foreground.a;
    Color::rgba(
        foreground.r * alpha + background.r * (1.0 - alpha),
        foreground.g * alpha + background.g * (1.0 - alpha),
        foreground.b * alpha + background.b * (1.0 - alpha),
        1.0,
    )
}

fn blend_linear_light(foreground: Color, background: Color) -> Color {
    let alpha = foreground.a;
    Color::rgba(
        srgb_channel(
            linear_channel(foreground.r) * alpha + linear_channel(background.r) * (1.0 - alpha),
        ),
        srgb_channel(
            linear_channel(foreground.g) * alpha + linear_channel(background.g) * (1.0 - alpha),
        ),
        srgb_channel(
            linear_channel(foreground.b) * alpha + linear_channel(background.b) * (1.0 - alpha),
        ),
        1.0,
    )
}

fn assert_selected_content_colors(run: &Run, expected_visibility_alpha: f32, locked: bool) {
    let theme = default_dark_theme();
    let foreground = theme.colors.selection.foreground;
    let colors = row_content_colors(run, id(1));
    assert_eq!(colors.disclosure, Some(foreground));
    assert_eq!(
        colors.visibility,
        Some(foreground.with_alpha(expected_visibility_alpha))
    );
    assert_eq!(
        colors.lock_stroke,
        Some(foreground.with_alpha(if locked { 1.0 } else { 0.55 }))
    );
    assert_eq!(colors.lock_fill, locked.then_some(foreground));
    assert_eq!(colors.label, Some(foreground));
}

#[test]
#[allow(clippy::too_many_lines)]
fn selected_content_discloses_named_white_exception_and_separate_alpha_nonconformities() {
    let theme = default_dark_theme();
    let background = theme.colors.selection.background;
    let foreground = theme.colors.selection.foreground;
    let full_ratio = contrast_ratio(foreground, background);
    assert!((3.52..3.54).contains(&full_ratio));
    assert!(full_ratio < 4.5);

    let mut full_flags = OutlinerRowFlags::new();
    full_flags.visible = true;
    full_flags.locked = true;
    let full_model = owned_model(full_flags);
    let cfg = config(BOUNDS);
    for (focus, expanded, point_kind, pressed) in [
        (false, false, "none", false),
        (true, false, "none", false),
        (true, true, "row", false),
        (true, true, "row", true),
        (true, true, "disclosure", false),
        (true, true, "disclosure", true),
        (true, true, "visibility", false),
        (true, true, "visibility", true),
        (true, true, "lock", false),
        (true, true, "lock", true),
    ] {
        let mut state = OutlinerState::new();
        state.selection.replace(id(1));
        if expanded {
            state.expansion.expand(id(1));
        }
        let mut memory = UiMemory::new();
        let seed = run_frame(
            &full_model,
            cfg.clone(),
            &mut state,
            &mut memory,
            UiInput::default(),
        );
        if focus {
            memory.focus(seed.root.child(("outliner-row", 1_u64)));
        }
        let zones = row_zones(&seed, id(1));
        let point = match point_kind {
            "row" => zones.label_rect.center(),
            "disclosure" => zones.disclosure_rect.center(),
            "visibility" => zones.visibility_toggle_rect.center(),
            "lock" => zones.lock_toggle_rect.center(),
            _ => Point::new(-100.0, -100.0),
        };
        let run = run_frame(
            &full_model,
            cfg.clone(),
            &mut state,
            &mut memory,
            if point_kind == "none" {
                UiInput::default()
            } else {
                primary_input(point, pressed, pressed, false, u8::from(pressed))
            },
        );
        assert_selected_content_colors(&run, 1.0, true);
        if focus {
            assert_row_focus(&run, id(1));
        } else {
            assert_no_row_annuli(&run, id(1));
        }
    }

    let mut alpha_flags = OutlinerRowFlags::new();
    alpha_flags.visible = false;
    alpha_flags.locked = false;
    let alpha_model = owned_model(alpha_flags);
    let mut alpha_state = OutlinerState::new();
    alpha_state.selection.replace(id(1));
    let alpha_seed = run_frame(
        &alpha_model,
        cfg.clone(),
        &mut alpha_state,
        &mut UiMemory::new(),
        UiInput::default(),
    );
    let colors = row_content_colors(&alpha_seed, id(1));
    assert_selected_content_colors(&alpha_seed, 0.5, false);
    let hidden = colors.visibility.expect("hidden visibility color");
    let unlocked = colors.lock_stroke.expect("unlocked lock color");
    let hidden_channel = contrast_ratio(blend_channel_space(hidden, background), background);
    let hidden_linear = contrast_ratio(blend_linear_light(hidden, background), background);
    let unlocked_channel = contrast_ratio(blend_channel_space(unlocked, background), background);
    let unlocked_linear = contrast_ratio(blend_linear_light(unlocked, background), background);
    assert!((1.90..1.92).contains(&hidden_channel));
    assert!((2.25..2.28).contains(&hidden_linear));
    assert!((2.02..2.05).contains(&unlocked_channel));
    assert!((2.38..2.41).contains(&unlocked_linear));
    assert!(hidden_channel < 3.0 && hidden_linear < 3.0);
    assert!(unlocked_channel < 3.0 && unlocked_linear < 3.0);

    let mut drag_state = OutlinerState::new();
    let mut drag_memory = UiMemory::new();
    let drag_seed = run_frame(
        &full_model,
        cfg.clone(),
        &mut drag_state,
        &mut drag_memory,
        UiInput::default(),
    );
    let row_point = row_zones(&drag_seed, id(1)).label_rect.center();
    let _ = click(
        row_point,
        1,
        &full_model,
        cfg.clone(),
        &mut drag_state,
        &mut drag_memory,
    );
    let _ = run_frame(
        &full_model,
        cfg.clone(),
        &mut drag_state,
        &mut drag_memory,
        primary_input(row_point, true, true, false, 1),
    );
    let dragged = run_frame(
        &full_model,
        cfg.clone(),
        &mut drag_state,
        &mut drag_memory,
        move_input(
            Point::new(row_point.x + 12.0, row_point.y),
            Vec2::new(12.0, 0.0),
        ),
    );
    assert!(row_response(&dragged, id(1)).row.dragged);
    assert_selected_content_colors(&dragged, 1.0, true);
    assert_row_focus(&dragged, id(1));

    let mut rename_state = OutlinerState::new();
    let mut rename_memory = UiMemory::new();
    let (_, _) = start_rename(
        &full_model,
        cfg.clone(),
        &mut rename_state,
        &mut rename_memory,
    );
    let editing = run_frame(
        &full_model,
        cfg.clone(),
        &mut rename_state,
        &mut rename_memory,
        UiInput::default(),
    );
    let editing_colors = row_content_colors(&editing, id(1));
    assert_eq!(editing_colors.label, None);
    assert_eq!(editing_colors.disclosure, Some(foreground));
    assert_eq!(editing_colors.visibility, Some(foreground));
    assert_eq!(editing_colors.lock_fill, Some(foreground));
    assert_no_row_annuli(&editing, id(1));

    let mut context_state = OutlinerState::new();
    let mut context_memory = UiMemory::new();
    let seed = run_frame(
        &full_model,
        cfg.clone(),
        &mut context_state,
        &mut context_memory,
        UiInput::default(),
    );
    let selected = click(
        row_zones(&seed, id(1)).label_rect.center(),
        1,
        &full_model,
        cfg.clone(),
        &mut context_state,
        &mut context_memory,
    );
    let context = context_click(
        row_zones(&selected, id(1)).context_rect.center(),
        &full_model,
        cfg,
        &mut context_state,
        &mut context_memory,
    );
    assert_selected_content_colors(&context, 1.0, true);
    assert_row_focus(&context, id(1));
}
