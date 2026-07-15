//! Public conformance evidence for owned virtual-table Cell focus annuli.

#![allow(clippy::cast_precision_loss, clippy::float_cmp, clippy::too_many_lines)]

use std::time::Duration;

use stern_core::{
    Brush, Color, ComponentState, FrameContext, Key, KeyEvent, KeyState, KeyboardInput, Modifiers,
    PathElement, PhysicalSize, Point, PointerButtonState, PointerInput, PointerOrder, Primitive,
    Rect, RepaintRequest, ScaleFactor, SemanticNode, SemanticRole, Size, TimeInfo, Transform,
    UiInput, UiMemory, Vec2, ViewportInfo, WidgetId, default_dark_theme,
};
use stern_widgets::{
    CollectionProjection, ItemId, SortDirection, TableColumn, TableLayout, TableSort, Ui,
    VirtualTableConfig, VirtualTableOutput, VirtualTableRow, VirtualTableSelection,
    VirtualTableSelectionMode, VirtualTableTarget,
};

const BOUNDS: Rect = Rect::new(3.25, 7.75, 240.0, 84.0);

fn id(raw: u64) -> ItemId {
    ItemId::from_raw(raw)
}

fn projection(count: u64) -> CollectionProjection {
    CollectionProjection::from_source_ids(&(1..=count).map(id).collect::<Vec<_>>())
}

fn columns(order: impl IntoIterator<Item = u64>) -> Vec<TableColumn> {
    order
        .into_iter()
        .map(|raw| {
            let label = match raw {
                10 => "Name",
                20 => "Kind",
                30 => "Size",
                _ => unreachable!("test column"),
            };
            TableColumn::new(id(raw), label, 80.0)
        })
        .collect()
}

fn table_config(
    bounds: Rect,
    order: impl IntoIterator<Item = u64>,
    sort: Option<TableSort>,
    overscan: usize,
) -> VirtualTableConfig {
    VirtualTableConfig::new(
        bounds,
        TableLayout {
            columns: columns(order),
            header_height: 20.25,
            row_height: 20.0,
            sort,
        },
    )
    .label("Assets")
    .overscan(overscan)
    .selection_mode(VirtualTableSelectionMode::Cell)
}

fn config(mode: VirtualTableSelectionMode) -> VirtualTableConfig {
    table_config(BOUNDS, [10, 20, 30], None, 0).selection_mode(mode)
}

fn context(input: UiInput) -> FrameContext {
    FrameContext::new(
        ViewportInfo::new(
            Size::new(320.0, 160.0),
            PhysicalSize::new(320, 160),
            ScaleFactor::ONE,
        ),
        input,
        TimeInfo::new(Duration::from_millis(500), Duration::from_millis(16), 1),
    )
}

fn pointer_input(point: Point, pressed: bool, released: bool) -> UiInput {
    UiInput {
        pointer: PointerInput {
            position: Some(point),
            primary: PointerButtonState::new(pressed, pressed, released),
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

#[derive(Clone, Copy)]
enum CellInteraction {
    Idle,
    Hover,
    Press,
}

impl CellInteraction {
    fn input(self, point: Point) -> UiInput {
        match self {
            Self::Idle => UiInput::default(),
            Self::Hover => pointer_input(point, false, false),
            Self::Press => pointer_input(point, true, false),
        }
    }

    const fn expected_hovered(self) -> bool {
        !matches!(self, Self::Idle)
    }

    const fn expected_pressed(self) -> bool {
        matches!(self, Self::Press)
    }
}

struct Run {
    root: WidgetId,
    output: VirtualTableOutput,
    callbacks: Vec<ItemId>,
    frame: stern_core::FrameOutput,
}

fn run_frame(
    projection: &CollectionProjection,
    config: VirtualTableConfig,
    selection: &mut VirtualTableSelection,
    memory: &mut UiMemory,
    input: UiInput,
) -> Run {
    let theme = default_dark_theme();
    let mut ui = Ui::begin_frame(context(input), memory, &theme);
    let table = ui
        .prepare_virtual_table("cell-focus-table", config, projection)
        .expect("valid table");
    let root = table.widget_id();
    ui.resolve_pointer_targets(|plan| {
        table.declare_pointer_targets(plan, PointerOrder::new(100));
    })
    .expect("valid table pointer plan");
    let mut callbacks = Vec::new();
    let output = ui.virtual_table(&table, selection, |item| {
        callbacks.push(item.id);
        VirtualTableRow::new([
            format!("Row {} name", item.id.raw()),
            format!("Row {} kind", item.id.raw()),
            format!("Row {} size", item.id.raw()),
        ])
    });
    Run {
        root,
        output,
        callbacks,
        frame: ui.finish_output(),
    }
}

fn cell_point(row: usize, column: usize) -> Point {
    cell_point_in(BOUNDS, row, column)
}

fn cell_point_in(bounds: Rect, row: usize, column: usize) -> Point {
    Point::new(
        bounds.x + column as f32 * 80.0 + 40.0,
        bounds.y + 20.25 + row as f32 * 20.0 + 10.0,
    )
}

fn cell_target(row: usize, column: usize) -> VirtualTableTarget {
    VirtualTableTarget::Cell {
        row: id(row as u64 + 1),
        column: id([10, 20, 30][column]),
    }
}

fn select_cell(
    projection: &CollectionProjection,
    table_config: VirtualTableConfig,
    selection: &mut VirtualTableSelection,
    memory: &mut UiMemory,
    row: usize,
    column: usize,
) -> Run {
    let point = cell_point(row, column);
    let _ = run_frame(
        projection,
        table_config.clone(),
        selection,
        memory,
        pointer_input(point, true, false),
    );
    run_frame(
        projection,
        table_config,
        selection,
        memory,
        pointer_input(point, false, true),
    )
}

fn select_cell_in(
    projection: &CollectionProjection,
    table_config: VirtualTableConfig,
    selection: &mut VirtualTableSelection,
    memory: &mut UiMemory,
    point: Point,
) -> Run {
    let _ = run_frame(
        projection,
        table_config.clone(),
        selection,
        memory,
        pointer_input(point, true, false),
    );
    run_frame(
        projection,
        table_config,
        selection,
        memory,
        pointer_input(point, false, true),
    )
}

fn selection_response(run: &Run, target: VirtualTableTarget) -> stern_core::Response {
    run.output
        .selection_responses
        .iter()
        .find(|candidate| candidate.target == target)
        .unwrap_or_else(|| panic!("missing response for {target:?}"))
        .response
}

fn cell_base_index(run: &Run, target: VirtualTableTarget) -> usize {
    let response = selection_response(run, target);
    run.frame
        .primitives
        .iter()
        .position(
            |primitive| matches!(primitive, Primitive::Rect(base) if base.rect == response.rect),
        )
        .expect("cell base")
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

fn endpoint(element: &PathElement) -> Option<Point> {
    match *element {
        PathElement::MoveTo(point)
        | PathElement::LineTo(point)
        | PathElement::QuadTo { to: point, .. }
        | PathElement::CubicTo { to: point, .. } => Some(point),
        PathElement::Close => None,
    }
}

fn winding_at(elements: &[PathElement], point: Point) -> i32 {
    let mut winding = 0;
    let mut current = Point::ZERO;
    let mut start = Point::ZERO;
    for element in elements {
        if let PathElement::MoveTo(to) = *element {
            current = to;
            start = to;
            continue;
        }
        let to = if matches!(element, PathElement::Close) {
            start
        } else {
            endpoint(element).expect("drawable path endpoint")
        };
        let cross =
            (to.x - current.x) * (point.y - current.y) - (point.x - current.x) * (to.y - current.y);
        if current.y <= point.y && to.y > point.y && cross > 0.0 {
            winding += 1;
        } else if current.y > point.y && to.y <= point.y && cross < 0.0 {
            winding -= 1;
        }
        current = to;
    }
    winding
}

fn assert_focused_cell(run: &Run, target: VirtualTableTarget) -> [Primitive; 2] {
    let theme = default_dark_theme();
    let response = selection_response(run, target);
    assert!(response.state.focused);
    assert!(!response.state.disabled);
    let state = ComponentState {
        hovered: response.state.hovered,
        pressed: response.state.pressed,
        focused: response.state.focused,
        disabled: response.state.disabled,
        selected: response.state.selected,
    };
    let recipe = theme.row(state);
    let base_index = cell_base_index(run, target);
    let Primitive::Rect(base) = &run.frame.primitives[base_index] else {
        unreachable!()
    };
    assert_eq!(base.rect, response.rect);
    assert_eq!(base.fill, Some(recipe.background));
    assert_eq!(base.stroke, Some(recipe.border));
    assert_eq!(base.radius, recipe.radius);
    let expected = theme
        .focus_ring(true)
        .expect("focus recipe")
        .inward_annulus_primitives(response.rect, recipe.radius, recipe.border.width);
    assert_eq!(run.frame.primitives[base_index + 1], expected[0]);
    assert_eq!(run.frame.primitives[base_index + 2], expected[1]);
    assert!(matches!(
        run.frame.primitives[base_index + 3],
        Primitive::Text(_)
    ));
    for primitive in &run.frame.primitives[base_index + 1..=base_index + 2] {
        let Primitive::Path(path) = primitive else {
            panic!("cell focus must be a compound path");
        };
        assert_eq!(path.elements.len(), 20);
        assert_eq!(path.stroke, None);
        assert_eq!(winding_at(&path.elements, response.rect.center()), 0);
        let bounds = path_bounds(&path.elements);
        assert!(
            [
                bounds.x,
                bounds.y,
                bounds.width,
                bounds.height,
                bounds.max_x(),
                bounds.max_y(),
            ]
            .into_iter()
            .all(f32::is_finite)
        );
        assert!(response.rect.contains_rect(bounds));
    }
    assert_eq!(
        run.frame
            .primitives
            .iter()
            .filter(|primitive| matches!(primitive, Primitive::Path(_)))
            .count(),
        2
    );
    assert_enabled_focus_owner(run, target);
    expected
}

fn assert_enabled_focus_owner(run: &Run, target: VirtualTableTarget) {
    let VirtualTableTarget::Cell { row, column } = target else {
        panic!("Cell mode focus must have a stable cell target")
    };
    let response = selection_response(run, target);
    assert!(!response.state.disabled);
    assert_eq!(
        run.output
            .selection_responses
            .iter()
            .filter(|candidate| candidate.response.state.focused)
            .map(|candidate| candidate.target)
            .collect::<Vec<_>>(),
        vec![target]
    );
    let focused_semantics = run
        .frame
        .semantics
        .nodes()
        .iter()
        .filter(|node| node.state.focused)
        .collect::<Vec<_>>();
    if let Some(cell) = run.frame.semantics.get(response.id) {
        assert_eq!(focused_semantics.len(), 1);
        assert_eq!(focused_semantics[0].id, response.id);
        assert_eq!(cell.role, SemanticRole::Cell);
        assert!(!cell.state.disabled);
        assert!(cell.focusable);
        assert_eq!(cell.state.selected, response.state.selected);
        assert_eq!(cell.state.pressed, response.state.pressed);
        assert!(cell.state.focused);
    } else {
        assert!(
            focused_semantics.is_empty(),
            "a fully clipped owner has no semantic node and never transfers focus"
        );
    }
    if let Some(row) = run
        .frame
        .semantics
        .get(run.root.child(("virtual-table-row", row.raw())))
    {
        assert!(!row.state.focused);
        assert!(!row.focusable);
    }
    for candidate in &run.output.selection_responses {
        if candidate.target != target {
            assert!(!candidate.response.state.focused);
            if let Some(sibling) = run.frame.semantics.get(candidate.response.id) {
                assert!(!sibling.state.focused);
            }
        }
    }
    for header in &run.output.headers {
        assert!(!header.response.state.focused);
    }
    assert_eq!(
        response.id,
        run.root
            .child(("virtual-table-cell", row.raw(), column.raw()))
    );
}

fn output_without_focus(mut output: VirtualTableOutput) -> VirtualTableOutput {
    for response in &mut output.selection_responses {
        response.response.state.focused = false;
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

fn assert_focus_only_transition(focused: &Run, unfocused: &Run) {
    assert_eq!(focused.callbacks, unfocused.callbacks);
    assert_eq!(
        output_without_focus(focused.output.clone()),
        unfocused.output
    );
    assert_eq!(
        focused
            .frame
            .primitives
            .iter()
            .filter(|primitive| !matches!(primitive, Primitive::Path(_)))
            .cloned()
            .collect::<Vec<_>>(),
        unfocused.frame.primitives
    );
    assert_eq!(
        semantics_without_focus(focused),
        unfocused.frame.semantics.nodes()
    );
    assert_eq!(focused.frame.repaint, unfocused.frame.repaint);
    assert_eq!(focused.frame.actions, unfocused.frame.actions);
    assert_eq!(
        focused.frame.platform_requests,
        unfocused.frame.platform_requests
    );
    assert_eq!(focused.frame.warnings, unfocused.frame.warnings);
}

fn assert_cell_focus_transaction(run: &Run, target: VirtualTableTarget, changed: bool) {
    assert_eq!(run.output.selection_changed, changed);
    assert_eq!(run.output.sort_requested, None);
    assert_eq!(run.output.resize_requested, None);
    assert_eq!(
        run.output
            .selection_responses
            .iter()
            .filter(|response| response.response.state.focused)
            .map(|response| response.target)
            .collect::<Vec<_>>(),
        vec![target]
    );
    assert!(run.frame.actions.is_empty());
    assert_focused_cell(run, target);
}

fn assert_body_scope(run: &Run, bounds: Rect, offset: Vec2) -> (usize, usize) {
    let body_clip = Rect::new(
        bounds.x,
        bounds.y + 20.25,
        bounds.width,
        bounds.height - 20.25,
    );
    let clip_begin = run
        .frame
        .primitives
        .iter()
        .position(
            |primitive| matches!(primitive, Primitive::ClipBegin { rect, .. } if *rect == body_clip),
        )
        .expect("body clip begin");
    let Primitive::ClipBegin {
        id: body_clip_id, ..
    } = run.frame.primitives[clip_begin]
    else {
        unreachable!()
    };
    let clip_end = run
        .frame
        .primitives
        .iter()
        .enumerate()
        .skip(clip_begin + 1)
        .find_map(|(index, primitive)| {
            matches!(primitive, Primitive::ClipEnd { id } if *id == body_clip_id).then_some(index)
        })
        .expect("body clip end");
    assert_eq!(
        run.frame.primitives[clip_begin + 1],
        Primitive::TransformBegin(Transform::translation(Vec2::new(-offset.x, -offset.y)))
    );
    assert_eq!(run.frame.primitives[clip_end - 1], Primitive::TransformEnd);
    assert_eq!(
        run.frame.primitives[clip_begin + 1..clip_end]
            .iter()
            .filter(|primitive| matches!(primitive, Primitive::TransformBegin(_)))
            .count(),
        1
    );
    (clip_begin, clip_end)
}

#[test]
fn every_cell_state_matrix_case_adds_only_one_exact_owned_pair_when_focused() {
    let items = projection(3);
    let table_config = config(VirtualTableSelectionMode::Cell);
    for row in 0..3 {
        for column in 0..3 {
            let target = cell_target(row, column);
            for selected in [false, true] {
                for focused in [false, true] {
                    for interaction in [
                        CellInteraction::Idle,
                        CellInteraction::Hover,
                        CellInteraction::Press,
                    ] {
                        let mut selection = VirtualTableSelection::new();
                        let mut memory = UiMemory::new();
                        let seed = run_frame(
                            &items,
                            table_config.clone(),
                            &mut selection,
                            &mut memory,
                            UiInput::default(),
                        );
                        if selected {
                            let _ = select_cell(
                                &items,
                                table_config.clone(),
                                &mut selection,
                                &mut memory,
                                row,
                                column,
                            );
                        }
                        let cell_id = seed.root.child((
                            "virtual-table-cell",
                            row as u64 + 1,
                            [10_u64, 20, 30][column],
                        ));
                        if focused {
                            memory.focus(cell_id);
                        } else {
                            memory.clear_focus();
                        }
                        let run = run_frame(
                            &items,
                            table_config.clone(),
                            &mut selection,
                            &mut memory,
                            interaction.input(cell_point(row, column)),
                        );
                        let response = selection_response(&run, target);
                        assert_eq!(response.state.selected, selected);
                        assert_eq!(response.state.focused, focused);
                        assert_eq!(response.state.hovered, interaction.expected_hovered());
                        assert_eq!(response.state.pressed, interaction.expected_pressed());
                        assert_eq!(run.output.sort_requested, None);
                        assert_eq!(run.output.resize_requested, None);
                        if focused {
                            assert_focused_cell(&run, target);
                        } else {
                            assert_eq!(
                                run.frame
                                    .primitives
                                    .iter()
                                    .filter(|primitive| matches!(primitive, Primitive::Path(_)))
                                    .count(),
                                0
                            );
                            assert!(matches!(
                                run.frame.primitives[cell_base_index(&run, target) + 1],
                                Primitive::Text(_)
                            ));
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn focused_cell_is_exactly_the_unfocused_frame_plus_two_paths_and_focus_bits() {
    let items = projection(3);
    let table_config = config(VirtualTableSelectionMode::Cell);
    let target = cell_target(1, 1);
    let mut selection = VirtualTableSelection::new();
    let mut memory = UiMemory::new();
    let seed = select_cell(
        &items,
        table_config.clone(),
        &mut selection,
        &mut memory,
        1,
        1,
    );
    let cell_id = selection_response(&seed, target).id;
    memory.clear_focus();
    let unfocused = run_frame(
        &items,
        table_config.clone(),
        &mut selection,
        &mut memory,
        UiInput::default(),
    );
    memory.focus(cell_id);
    let focused = run_frame(
        &items,
        table_config,
        &mut selection,
        &mut memory,
        UiInput::default(),
    );
    assert_focused_cell(&focused, target);
    assert_focus_only_transition(&focused, &unfocused);
}

#[test]
fn disabled_cells_are_inert_and_ring_free_with_focus_observable_only_when_retained() {
    let items = projection(3);
    let enabled = config(VirtualTableSelectionMode::Cell);
    for row in 0..3 {
        for column in 0..3 {
            let target = cell_target(row, column);
            let mut seeded_selection = VirtualTableSelection::new();
            let selected = select_cell(
                &items,
                enabled.clone(),
                &mut seeded_selection,
                &mut UiMemory::new(),
                row,
                column,
            );
            let cell_id = selection_response(&selected, target).id;
            for retained_focus in [false, true] {
                let mut selection = seeded_selection.clone();
                let mut memory = UiMemory::new();
                if retained_focus {
                    memory.focus(cell_id);
                }
                let disabled = run_frame(
                    &items,
                    config(VirtualTableSelectionMode::Cell).disabled(true),
                    &mut selection,
                    &mut memory,
                    pointer_input(cell_point(row, column), true, false),
                );
                assert_eq!(selection.target(), Some(target));
                assert_eq!(memory.is_focused(cell_id), retained_focus);
                let response = selection_response(&disabled, target);
                assert_eq!(response.id, cell_id);
                assert_eq!(response.state.focused, retained_focus);
                assert!(response.state.selected);
                assert!(response.state.disabled);
                assert!(!response.state.hovered);
                assert!(!response.state.pressed);
                assert_eq!(
                    disabled
                        .frame
                        .primitives
                        .iter()
                        .filter(|primitive| matches!(primitive, Primitive::Path(_)))
                        .count(),
                    0
                );
                assert!(!disabled.output.selection_changed);
                assert_eq!(disabled.output.sort_requested, None);
                assert_eq!(disabled.output.resize_requested, None);
                assert_eq!(disabled.output.scroll.delta, Vec2::ZERO);
                assert_eq!(disabled.frame.repaint, RepaintRequest::None);
                assert!(disabled.frame.actions.is_empty());
                assert!(disabled.frame.platform_requests.is_empty());

                let semantic = disabled
                    .frame
                    .semantics
                    .get(cell_id)
                    .expect("disabled cell");
                assert_eq!(semantic.role, SemanticRole::Cell);
                assert_eq!(semantic.state.focused, retained_focus);
                assert!(semantic.state.selected);
                assert!(semantic.state.disabled);
                assert!(!semantic.state.pressed);
                assert!(!semantic.focusable);
                assert!(semantic.actions.is_empty());
                let focused_semantics = disabled
                    .frame
                    .semantics
                    .nodes()
                    .iter()
                    .filter(|node| node.state.focused)
                    .collect::<Vec<_>>();
                assert_eq!(focused_semantics.len(), usize::from(retained_focus));
                if retained_focus {
                    assert_eq!(focused_semantics[0].id, cell_id);
                }
                for candidate in &disabled.output.selection_responses {
                    assert!(candidate.response.state.disabled);
                    assert!(!candidate.response.state.pressed);
                    if candidate.target != target {
                        assert!(!candidate.response.state.focused);
                    }
                }
                let VirtualTableTarget::Cell { row, .. } = target else {
                    unreachable!()
                };
                let row_semantic = disabled
                    .frame
                    .semantics
                    .get(disabled.root.child(("virtual-table-row", row.raw())))
                    .expect("disabled row");
                assert!(row_semantic.state.disabled);
                assert!(!row_semantic.state.focused);
                assert!(!row_semantic.focusable);
                assert!(row_semantic.actions.is_empty());
            }
        }
    }
}

#[test]
fn row_mode_retains_row_focus_and_exact_cell_bases_without_any_annuli() {
    let items = projection(3);
    let table_config = config(VirtualTableSelectionMode::Row);
    let mut selection = VirtualTableSelection::new();
    let mut memory = UiMemory::new();
    let point = cell_point(1, 1);
    let _ = run_frame(
        &items,
        table_config.clone(),
        &mut selection,
        &mut memory,
        pointer_input(point, true, false),
    );
    let _ = run_frame(
        &items,
        table_config.clone(),
        &mut selection,
        &mut memory,
        pointer_input(point, false, true),
    );
    let focused = run_frame(
        &items,
        table_config.clone(),
        &mut selection,
        &mut memory,
        UiInput::default(),
    );
    let target = VirtualTableTarget::Row(id(2));
    let row_response = selection_response(&focused, target);
    assert!(row_response.state.focused);
    assert!(row_response.state.selected);
    assert_eq!(
        focused
            .frame
            .primitives
            .iter()
            .filter(|primitive| matches!(primitive, Primitive::Path(_)))
            .count(),
        0
    );
    let row_semantic = focused
        .frame
        .semantics
        .get(row_response.id)
        .expect("focused row");
    assert!(row_semantic.focusable);
    assert!(row_semantic.state.focused);
    for column in [10_u64, 20, 30] {
        let cell = focused
            .frame
            .semantics
            .get(focused.root.child(("virtual-table-cell", 2_u64, column)))
            .expect("row-owned cell");
        assert!(!cell.focusable);
        assert!(!cell.state.focused);
    }
    memory.clear_focus();
    let unfocused = run_frame(
        &items,
        table_config,
        &mut selection,
        &mut memory,
        UiInput::default(),
    );
    assert_focus_only_transition(&focused, &unfocused);
}

#[test]
fn pointer_and_every_cell_navigation_key_move_one_stable_focus_owner_and_reveal() {
    let bounds = Rect::new(3.25, 7.75, 120.0, 84.0);
    let items = projection(20);
    let table_config = table_config(bounds, [10, 20, 30], None, 1);
    let mut selection = VirtualTableSelection::new();
    let mut memory = UiMemory::new();
    let selected = select_cell_in(
        &items,
        table_config.clone(),
        &mut selection,
        &mut memory,
        cell_point_in(bounds, 0, 0),
    );
    let first = VirtualTableTarget::Cell {
        row: id(1),
        column: id(10),
    };
    assert_cell_focus_transaction(&selected, first, true);

    for (key, target, expected_offset) in [
        (
            Key::ArrowRight,
            VirtualTableTarget::Cell {
                row: id(1),
                column: id(20),
            },
            Vec2::new(40.0, 0.0),
        ),
        (
            Key::End,
            VirtualTableTarget::Cell {
                row: id(1),
                column: id(30),
            },
            Vec2::new(120.0, 0.0),
        ),
        (
            Key::ArrowLeft,
            VirtualTableTarget::Cell {
                row: id(1),
                column: id(20),
            },
            Vec2::new(80.0, 0.0),
        ),
        (
            Key::Home,
            VirtualTableTarget::Cell {
                row: id(1),
                column: id(10),
            },
            Vec2::new(0.0, 0.0),
        ),
        (
            Key::ArrowDown,
            VirtualTableTarget::Cell {
                row: id(2),
                column: id(10),
            },
            Vec2::new(0.0, 0.0),
        ),
        (
            Key::PageDown,
            VirtualTableTarget::Cell {
                row: id(5),
                column: id(10),
            },
            Vec2::new(0.0, 36.25),
        ),
        (
            Key::ArrowUp,
            VirtualTableTarget::Cell {
                row: id(4),
                column: id(10),
            },
            Vec2::new(0.0, 36.25),
        ),
        (
            Key::PageUp,
            VirtualTableTarget::Cell {
                row: id(1),
                column: id(10),
            },
            Vec2::new(0.0, 0.0),
        ),
    ] {
        let moved = run_frame(
            &items,
            table_config.clone(),
            &mut selection,
            &mut memory,
            key_input(key),
        );
        assert_eq!(selection.target(), Some(target));
        assert_cell_focus_transaction(&moved, target, true);
        let settled = run_frame(
            &items,
            table_config.clone(),
            &mut selection,
            &mut memory,
            UiInput::default(),
        );
        assert_eq!(settled.output.window.offset, expected_offset);
        assert_cell_focus_transaction(&settled, target, false);
    }
}

#[test]
fn stable_cell_identity_survives_projection_column_and_sort_reorder_then_repairs_removal() {
    let original = CollectionProjection::from_source_ids(&[id(1), id(2), id(3), id(4)]);
    let mut selection = VirtualTableSelection::new();
    let mut memory = UiMemory::new();
    let base_config = table_config(BOUNDS, [10, 20, 30], None, 1);
    let selected = select_cell_in(
        &original,
        base_config,
        &mut selection,
        &mut memory,
        cell_point(1, 1),
    );
    let stable = VirtualTableTarget::Cell {
        row: id(2),
        column: id(20),
    };
    let stable_id = selection_response(&selected, stable).id;
    assert!(memory.is_focused(stable_id));

    let reordered = CollectionProjection::from_source_ids(&[id(4), id(3), id(1), id(2)]);
    let projection_reordered = run_frame(
        &reordered,
        table_config(BOUNDS, [10, 20, 30], None, 1),
        &mut selection,
        &mut memory,
        UiInput::default(),
    );
    assert_eq!(selection.target(), Some(stable));
    assert_eq!(
        selection_response(&projection_reordered, stable).id,
        stable_id
    );
    assert_cell_focus_transaction(&projection_reordered, stable, false);

    let sort = TableSort {
        column: id(30),
        direction: SortDirection::Descending,
    };
    let sorted = run_frame(
        &reordered,
        table_config(BOUNDS, [10, 20, 30], Some(sort), 1),
        &mut selection,
        &mut memory,
        UiInput::default(),
    );
    assert_eq!(selection.target(), Some(stable));
    assert_eq!(selection_response(&sorted, stable).id, stable_id);
    assert_cell_focus_transaction(&sorted, stable, false);

    let columns_reordered = run_frame(
        &reordered,
        table_config(BOUNDS, [20, 30, 10], Some(sort), 1),
        &mut selection,
        &mut memory,
        UiInput::default(),
    );
    assert_eq!(selection.target(), Some(stable));
    assert_eq!(selection_response(&columns_reordered, stable).id, stable_id);
    assert_cell_focus_transaction(&columns_reordered, stable, false);

    let removed_row = CollectionProjection::from_source_ids(&[id(4), id(3), id(1)]);
    let repaired_row = VirtualTableTarget::Cell {
        row: id(1),
        column: id(20),
    };
    let row_repaired = run_frame(
        &removed_row,
        table_config(BOUNDS, [20, 30, 10], Some(sort), 1),
        &mut selection,
        &mut memory,
        UiInput::default(),
    );
    assert_eq!(selection.target(), Some(repaired_row));
    assert!(!memory.is_focused(stable_id));
    assert_cell_focus_transaction(&row_repaired, repaired_row, true);
    assert!(
        row_repaired
            .output
            .selection_responses
            .iter()
            .all(|response| response.target != stable)
    );

    let repaired_column = VirtualTableTarget::Cell {
        row: id(1),
        column: id(30),
    };
    let column_repaired = run_frame(
        &removed_row,
        table_config(BOUNDS, [30, 10], Some(sort), 1),
        &mut selection,
        &mut memory,
        UiInput::default(),
    );
    assert_eq!(selection.target(), Some(repaired_column));
    assert_cell_focus_transaction(&column_repaired, repaired_column, true);
    assert!(
        column_repaired
            .output
            .selection_responses
            .iter()
            .all(|response| response.target != repaired_row)
    );
}

#[test]
fn fractional_two_axis_scroll_clips_one_full_geometry_pair_in_the_exact_body_scope() {
    let items = projection(12);
    let seed_bounds = Rect::new(3.25, 7.75, 240.0, 120.0);
    let bounds = Rect::new(3.25, 7.75, 123.5, 64.0);
    let offset = Vec2::new(30.25, 12.5);
    let scrolled_config = table_config(bounds, [10, 20, 30], None, 1);
    let body_clip = Rect::new(
        bounds.x,
        bounds.y + 20.25,
        bounds.width,
        bounds.height - 20.25,
    );

    for (row, column, visible) in [(0, 0, true), (1, 1, true), (2, 0, true), (3, 2, false)] {
        let mut seeded_selection = VirtualTableSelection::new();
        let seeded = select_cell_in(
            &items,
            table_config(seed_bounds, [10, 20, 30], None, 1),
            &mut seeded_selection,
            &mut UiMemory::new(),
            cell_point_in(seed_bounds, row, column),
        );
        let target = cell_target(row, column);
        let cell_id = seeded.root.child((
            "virtual-table-cell",
            row as u64 + 1,
            [10_u64, 20, 30][column],
        ));

        let mut focused_selection = seeded_selection.clone();
        let mut focused_memory = UiMemory::new();
        focused_memory.set_scroll_offset(seeded.root, offset);
        focused_memory.focus(cell_id);
        let focused = run_frame(
            &items,
            scrolled_config.clone(),
            &mut focused_selection,
            &mut focused_memory,
            UiInput::default(),
        );
        assert_eq!(focused.output.window.offset, offset);
        assert!(
            focused.output.window.body.materialized_range.len()
                > focused.output.window.body.visible_range.len()
        );
        assert_eq!(focused.callbacks.len(), focused.output.rows.len());
        assert_eq!(
            focused.callbacks,
            focused
                .output
                .rows
                .iter()
                .map(|row| row.id)
                .collect::<Vec<_>>()
        );
        assert_cell_focus_transaction(&focused, target, false);
        let response = selection_response(&focused, target);
        let translated = Rect::new(
            response.rect.x - offset.x,
            response.rect.y - offset.y,
            response.rect.width,
            response.rect.height,
        );
        assert_eq!(translated.intersection(body_clip).is_some(), visible);
        match (row, column) {
            (0, 0) => {
                assert!(translated.x < body_clip.x && translated.max_x() > body_clip.x);
                assert!(translated.y < body_clip.y && translated.max_y() > body_clip.y);
            }
            (1, 1) => {
                assert!(translated.x < body_clip.max_x() && translated.max_x() > body_clip.max_x());
            }
            (2, 0) => {
                assert!(translated.y < body_clip.max_y() && translated.max_y() > body_clip.max_y());
            }
            (3, 2) => assert!(translated.intersection(body_clip).is_none()),
            _ => unreachable!(),
        }
        let semantic = focused.frame.semantics.get(cell_id);
        assert_eq!(semantic.is_some(), visible);
        let (clip_begin, clip_end) = assert_body_scope(&focused, bounds, offset);
        let base = cell_base_index(&focused, target);
        assert!(clip_begin < base && base + 3 < clip_end);

        let mut unfocused_selection = seeded_selection;
        let mut unfocused_memory = UiMemory::new();
        unfocused_memory.set_scroll_offset(seeded.root, offset);
        let unfocused = run_frame(
            &items,
            scrolled_config.clone(),
            &mut unfocused_selection,
            &mut unfocused_memory,
            UiInput::default(),
        );
        assert_focus_only_transition(&focused, &unfocused);
    }
}

#[test]
fn ten_and_hundred_thousand_rows_bound_materialization_without_recycled_focus_transfer() {
    let bounds = Rect::new(3.25, 7.75, 123.5, 64.0);
    let table_config = table_config(bounds, [10, 20, 30], None, 2);
    for count in [10_000_u64, 100_000] {
        let items = projection(count);
        let mut selection = VirtualTableSelection::new();
        let mut seed_memory = UiMemory::new();
        let seed = select_cell_in(
            &items,
            table_config.clone(),
            &mut selection,
            &mut seed_memory,
            cell_point_in(bounds, 0, 1),
        );
        let original = VirtualTableTarget::Cell {
            row: id(1),
            column: id(20),
        };
        let original_id = selection_response(&seed, original).id;
        let far_offset = Vec2::new(30.25, count as f32 * 10.0 + 0.5);
        let mut memory = UiMemory::new();
        memory.set_scroll_offset(seed.root, far_offset);
        memory.focus(original_id);
        let scrolled = run_frame(
            &items,
            table_config.clone(),
            &mut selection,
            &mut memory,
            UiInput::default(),
        );
        assert_eq!(scrolled.output.window.offset, far_offset);
        assert_eq!(selection.target(), Some(original));
        assert!(memory.is_focused(original_id));
        assert_eq!(scrolled.callbacks.len(), scrolled.output.rows.len());
        assert!(scrolled.callbacks.len() <= 8);
        assert_eq!(
            scrolled.callbacks.len(),
            scrolled.output.window.body.materialized_range.len()
        );
        assert!(
            scrolled
                .output
                .selection_responses
                .iter()
                .all(|response| !response.response.state.focused)
        );
        assert!(
            scrolled
                .frame
                .semantics
                .nodes()
                .iter()
                .all(|node| !node.state.focused)
        );
        assert_eq!(
            scrolled
                .frame
                .primitives
                .iter()
                .filter(|primitive| matches!(primitive, Primitive::Path(_)))
                .count(),
            0
        );

        let staged_reveal = run_frame(
            &items,
            table_config.clone(),
            &mut selection,
            &mut memory,
            key_input(Key::ArrowDown),
        );
        let revealed_target = VirtualTableTarget::Cell {
            row: id(2),
            column: id(20),
        };
        assert_eq!(selection.target(), Some(revealed_target));
        assert!(staged_reveal.output.selection_changed);
        assert!(staged_reveal.callbacks.len() <= 8);
        assert_eq!(
            staged_reveal
                .frame
                .primitives
                .iter()
                .filter(|primitive| matches!(primitive, Primitive::Path(_)))
                .count(),
            0
        );

        let revealed = run_frame(
            &items,
            table_config.clone(),
            &mut selection,
            &mut memory,
            UiInput::default(),
        );
        assert_eq!(revealed.output.window.offset, Vec2::new(36.5, 20.0));
        assert!(revealed.callbacks.len() <= 8);
        assert!(revealed.callbacks.contains(&id(2)));
        assert_cell_focus_transaction(&revealed, revealed_target, false);
        assert!(!memory.is_focused(original_id));
    }
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

fn solid(brush: Brush) -> Color {
    let Brush::Solid(color) = brush else {
        panic!("expected solid brush");
    };
    color
}

fn assert_ratio(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 0.000_01,
        "{actual} != {expected}"
    );
}

fn cell_colors(run: &Run, target: VirtualTableTarget) -> (Color, Color, Color) {
    let base_index = cell_base_index(run, target);
    let Primitive::Rect(base) = &run.frame.primitives[base_index] else {
        unreachable!()
    };
    let text = run.frame.primitives[base_index + 1..]
        .iter()
        .find_map(|primitive| match primitive {
            Primitive::Text(text) => Some(text),
            _ => None,
        })
        .expect("cell text");
    (
        solid(base.fill.expect("cell fill")),
        solid(base.stroke.expect("cell border").brush),
        solid(text.brush),
    )
}

#[test]
fn production_cell_primitives_inventory_acc005_and_grid_border_nonconformities() {
    let theme = default_dark_theme();
    let items = projection(2);
    let table_config = config(VirtualTableSelectionMode::Cell);
    let target = cell_target(0, 0);

    let idle = run_frame(
        &items,
        table_config.clone(),
        &mut VirtualTableSelection::new(),
        &mut UiMemory::new(),
        UiInput::default(),
    );
    let (idle_background, idle_border, idle_text) = cell_colors(&idle, target);
    assert_ratio(contrast_ratio(idle_text, idle_background), 16.063_878);
    assert_ratio(contrast_ratio(idle_border, idle_background), 1.237_124);

    let hovered = run_frame(
        &items,
        table_config.clone(),
        &mut VirtualTableSelection::new(),
        &mut UiMemory::new(),
        CellInteraction::Hover.input(cell_point(0, 0)),
    );
    let (hover_background, hover_border, hover_text) = cell_colors(&hovered, target);
    assert_ratio(contrast_ratio(hover_text, hover_background), 13.908_798);
    assert_ratio(contrast_ratio(hover_border, hover_background), 1.071_155);

    let mut selection = VirtualTableSelection::new();
    let mut memory = UiMemory::new();
    let selected = select_cell(
        &items,
        table_config.clone(),
        &mut selection,
        &mut memory,
        0,
        0,
    );
    assert_focused_cell(&selected, target);
    let (selected_background, selected_border, selected_text) = cell_colors(&selected, target);
    assert_ratio(
        contrast_ratio(selected_text, selected_background),
        3.533_269,
    );
    assert_ratio(
        contrast_ratio(selected_border, selected_background),
        4.502_908,
    );

    let disabled = run_frame(
        &items,
        VirtualTableConfig {
            disabled: true,
            ..table_config
        },
        &mut selection,
        &mut memory,
        UiInput::default(),
    );
    let (disabled_background, disabled_border, disabled_text) = cell_colors(&disabled, target);
    assert_ratio(
        contrast_ratio(disabled_text, disabled_background),
        3.208_475,
    );
    assert_ratio(
        contrast_ratio(disabled_border, disabled_background),
        1.157_923,
    );

    let base_index = cell_base_index(&selected, target);
    let Primitive::Path(primary) = &selected.frame.primitives[base_index + 1] else {
        panic!("primary focus path");
    };
    let Primitive::Path(separator) = &selected.frame.primitives[base_index + 2] else {
        panic!("separator focus path");
    };
    let primary = solid(primary.fill.expect("primary focus fill"));
    let separator = solid(separator.fill.expect("separator focus fill"));
    assert_ratio(contrast_ratio(primary, separator), 8.555_114);
    assert_ratio(contrast_ratio(separator, selected_background), 5.570_656);
    assert_ratio(contrast_ratio(primary, selected_background), 1.535_746);
    assert_eq!(selected_background, theme.colors.selection.background);
    assert_eq!(selected_text, theme.colors.selection.foreground);
}
