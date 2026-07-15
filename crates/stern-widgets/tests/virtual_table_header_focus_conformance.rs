//! Public conformance evidence for owned virtual-table header focus annuli.

#![allow(clippy::float_cmp, clippy::too_many_lines)]

use std::time::Duration;

use stern_core::{
    Brush, Color, ComponentState, FrameContext, Key, KeyEvent, KeyState, KeyboardInput, Modifiers,
    MouseButton, PathElement, PhysicalSize, Point, PointerButtonState, PointerInput, PointerOrder,
    Primitive, Rect, RepaintRequest, ScaleFactor, SemanticActionKind, SemanticNode, SemanticRole,
    Size, TimeInfo, Transform, UiInput, UiInputEvent, UiMemory, Vec2, ViewportInfo, WidgetId,
    default_dark_theme,
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

fn columns(order: [u64; 3]) -> Vec<TableColumn> {
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

fn config(sort: Option<TableSort>) -> VirtualTableConfig {
    config_with(BOUNDS, sort, [10, 20, 30])
}

fn config_with(bounds: Rect, sort: Option<TableSort>, order: [u64; 3]) -> VirtualTableConfig {
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
    .overscan(0)
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

fn pointer_input(point: Point, pressed: bool) -> UiInput {
    UiInput {
        pointer: PointerInput {
            position: Some(point),
            primary: PointerButtonState::new(pressed, pressed, false),
            ..PointerInput::default()
        },
        ..UiInput::default()
    }
}

fn release_input(point: Point) -> UiInput {
    UiInput {
        pointer: PointerInput {
            position: Some(point),
            primary: PointerButtonState::new(false, false, true),
            ..PointerInput::default()
        },
        ..UiInput::default()
    }
}

fn drag_input(point: Point, down: bool, pressed: bool, released: bool, delta_x: f32) -> UiInput {
    let mut input = UiInput::default();
    if pressed {
        input.push_event(UiInputEvent::PointerButton {
            button: MouseButton::Primary,
            down: true,
            click_count: 1,
            position: Some(point),
        });
    } else if released {
        input.pointer.position = Some(point);
        input.pointer.primary = PointerButtonState::new(true, false, false);
        input.push_event(UiInputEvent::PointerButton {
            button: MouseButton::Primary,
            down: false,
            click_count: 1,
            position: Some(point),
        });
    } else {
        input.pointer.primary = PointerButtonState::new(down, false, false);
        input.push_event(UiInputEvent::PointerMoved {
            position: point,
            delta: Vec2::new(delta_x, 0.0),
        });
    }
    input
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

#[derive(Clone, Copy)]
enum HeaderInteraction {
    Idle,
    Hover,
    Press,
}

impl HeaderInteraction {
    fn input(self, point: Point) -> UiInput {
        match self {
            Self::Idle => UiInput::default(),
            Self::Hover => pointer_input(point, false),
            Self::Press => pointer_input(point, true),
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
        .prepare_virtual_table("header-focus-table", config, projection)
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

fn header_response(run: &Run, column: ItemId) -> stern_core::Response {
    run.output
        .headers
        .iter()
        .find(|header| header.column == column)
        .unwrap_or_else(|| panic!("missing header {}", column.raw()))
        .response
}

fn header_base_index(run: &Run, column: ItemId) -> usize {
    let rect = header_response(run, column).rect;
    run.frame
        .primitives
        .iter()
        .position(|primitive| matches!(primitive, Primitive::Rect(base) if base.rect == rect))
        .expect("header base")
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

fn assert_header_focus(run: &Run, column: ItemId, selected: bool) -> [Primitive; 2] {
    let theme = default_dark_theme();
    let response = header_response(run, column);
    assert!(response.state.focused);
    assert!(!response.state.disabled);
    assert!(!response.state.selected, "sort is painter-only state");
    let state = ComponentState {
        hovered: response.state.hovered,
        pressed: response.state.pressed,
        focused: response.state.focused,
        disabled: response.state.disabled,
        selected,
    };
    let recipe = theme.row(state);
    let base_index = header_base_index(run, column);
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
    for primitive in &run.frame.primitives[base_index + 1..=base_index + 2] {
        let Primitive::Path(path) = primitive else {
            panic!("header focus must be a compound path");
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
    assert!(matches!(
        run.frame.primitives[base_index + 3],
        Primitive::Text(_)
    ));
    expected
}

fn output_without_header_focus(mut output: VirtualTableOutput) -> VirtualTableOutput {
    for header in &mut output.headers {
        header.response.state.focused = false;
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
        output_without_header_focus(focused.output.clone()),
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
    assert_eq!(
        focused
            .frame
            .primitives
            .iter()
            .filter(|primitive| matches!(primitive, Primitive::Path(_)))
            .count(),
        2
    );
}

#[test]
fn every_header_state_matrix_case_adds_only_exact_owned_annuli_when_focused() {
    let items = projection(3);
    let seed = run_frame(
        &items,
        config(None),
        &mut VirtualTableSelection::new(),
        &mut UiMemory::new(),
        UiInput::default(),
    );

    for column in [id(10), id(20), id(30)] {
        let header_id = seed
            .output
            .headers
            .iter()
            .find(|header| header.column == column)
            .expect("seed header")
            .response
            .id;
        let point = header_response(&seed, column).rect.center();
        for selected in [false, true] {
            let sort = selected.then_some(TableSort {
                column,
                direction: SortDirection::Ascending,
            });
            for interaction in [
                HeaderInteraction::Idle,
                HeaderInteraction::Hover,
                HeaderInteraction::Press,
            ] {
                let unfocused = run_frame(
                    &items,
                    config(sort),
                    &mut VirtualTableSelection::new(),
                    &mut UiMemory::new(),
                    interaction.input(point),
                );
                let unfocused_response = header_response(&unfocused, column);
                assert!(!unfocused_response.state.focused);
                assert_eq!(
                    unfocused_response.state.hovered,
                    interaction.expected_hovered()
                );
                assert_eq!(
                    unfocused_response.state.pressed,
                    interaction.expected_pressed()
                );
                assert!(!unfocused_response.state.selected);
                assert_eq!(
                    unfocused
                        .frame
                        .primitives
                        .iter()
                        .filter(|primitive| matches!(primitive, Primitive::Path(_)))
                        .count(),
                    0,
                    "unfocused headers never paint focus paths"
                );
                let state = ComponentState {
                    hovered: unfocused_response.state.hovered,
                    pressed: unfocused_response.state.pressed,
                    focused: false,
                    disabled: false,
                    selected,
                };
                let recipe = default_dark_theme().row(state);
                let base_index = header_base_index(&unfocused, column);
                let Primitive::Rect(base) = &unfocused.frame.primitives[base_index] else {
                    unreachable!()
                };
                assert_eq!(base.rect, unfocused_response.rect);
                assert_eq!(base.fill, Some(recipe.background));
                assert_eq!(base.stroke, Some(recipe.border));
                assert_eq!(base.radius, recipe.radius);
                assert!(matches!(
                    unfocused.frame.primitives[base_index + 1],
                    Primitive::Text(_)
                ));

                let mut focused_memory = UiMemory::new();
                focused_memory.focus(header_id);
                let focused = run_frame(
                    &items,
                    config(sort),
                    &mut VirtualTableSelection::new(),
                    &mut focused_memory,
                    interaction.input(point),
                );
                let focused_response = header_response(&focused, column);
                assert_eq!(
                    focused_response.state.hovered,
                    interaction.expected_hovered()
                );
                assert_eq!(
                    focused_response.state.pressed,
                    interaction.expected_pressed()
                );
                assert_header_focus(&focused, column, selected);
                assert_focus_only_transition(&focused, &unfocused);
            }
        }
    }
}

#[test]
fn focused_annuli_are_identical_across_every_header_interaction_and_sort_state() {
    let items = projection(3);
    let seed = run_frame(
        &items,
        config(None),
        &mut VirtualTableSelection::new(),
        &mut UiMemory::new(),
        UiInput::default(),
    );
    for column in [id(10), id(20), id(30)] {
        let response = header_response(&seed, column);
        let mut reference = None;
        for selected in [false, true] {
            let sort = selected.then_some(TableSort {
                column,
                direction: SortDirection::Ascending,
            });
            for interaction in [
                HeaderInteraction::Idle,
                HeaderInteraction::Hover,
                HeaderInteraction::Press,
            ] {
                let mut memory = UiMemory::new();
                memory.focus(response.id);
                let run = run_frame(
                    &items,
                    config(sort),
                    &mut VirtualTableSelection::new(),
                    &mut memory,
                    interaction.input(response.rect.center()),
                );
                let annuli = assert_header_focus(&run, column, selected);
                if let Some(reference) = &reference {
                    assert_eq!(&annuli, reference);
                } else {
                    reference = Some(annuli);
                }
            }
        }
    }
}

#[test]
fn sort_direction_changes_preserve_header_identity_focus_and_annulus_geometry() {
    let items = projection(1);
    let seed = run_frame(
        &items,
        config(None),
        &mut VirtualTableSelection::new(),
        &mut UiMemory::new(),
        UiInput::default(),
    );
    let column = id(20);
    let header_id = header_response(&seed, column).id;
    let mut memory = UiMemory::new();
    memory.focus(header_id);

    let ascending = run_frame(
        &items,
        config(Some(TableSort {
            column,
            direction: SortDirection::Ascending,
        })),
        &mut VirtualTableSelection::new(),
        &mut memory,
        UiInput::default(),
    );
    let descending = run_frame(
        &items,
        config(Some(TableSort {
            column,
            direction: SortDirection::Descending,
        })),
        &mut VirtualTableSelection::new(),
        &mut memory,
        UiInput::default(),
    );
    assert_eq!(header_response(&ascending, column).id, header_id);
    assert_eq!(header_response(&descending, column).id, header_id);
    assert_eq!(
        assert_header_focus(&ascending, column, true),
        assert_header_focus(&descending, column, true)
    );
}

#[test]
fn modifier_and_repeat_inputs_do_not_change_idle_focus_geometry() {
    let items = projection(1);
    let seed = run_frame(
        &items,
        config(None),
        &mut VirtualTableSelection::new(),
        &mut UiMemory::new(),
        UiInput::default(),
    );
    let column = id(10);
    let mut memory = UiMemory::new();
    memory.focus(header_response(&seed, column).id);
    let idle = run_frame(
        &items,
        config(None),
        &mut VirtualTableSelection::new(),
        &mut memory,
        UiInput::default(),
    );
    let expected = assert_header_focus(&idle, column, false);

    let modified = Modifiers {
        ctrl: true,
        ..Modifiers::default()
    };
    for input in [
        key_input(Key::Enter, Modifiers::default(), true),
        key_input(Key::Space, modified, false),
    ] {
        let run = run_frame(
            &items,
            config(None),
            &mut VirtualTableSelection::new(),
            &mut memory,
            input,
        );
        assert_eq!(run.output.sort_requested, None);
        assert_eq!(assert_header_focus(&run, column, false), expected);
    }
    assert_eq!(memory.scroll_offset(seed.root), Vec2::ZERO);
}

#[test]
fn pointer_and_keyboard_sorting_emit_one_exact_descriptor_without_pointer_focus_transfer() {
    let items = projection(2);
    let table_config = config(None);
    let seed = run_frame(
        &items,
        table_config.clone(),
        &mut VirtualTableSelection::new(),
        &mut UiMemory::new(),
        UiInput::default(),
    );
    let column = id(10);
    let point = header_response(&seed, column).rect.center();

    let mut pointer_memory = UiMemory::new();
    let pressed = run_frame(
        &items,
        table_config.clone(),
        &mut VirtualTableSelection::new(),
        &mut pointer_memory,
        pointer_input(point, true),
    );
    assert!(header_response(&pressed, column).state.pressed);
    let pointer = run_frame(
        &items,
        table_config.clone(),
        &mut VirtualTableSelection::new(),
        &mut pointer_memory,
        release_input(point),
    );
    assert_eq!(
        pointer.output.sort_requested,
        Some(TableSort {
            column,
            direction: SortDirection::Ascending,
        })
    );
    assert_eq!(pointer_memory.focused(), None);
    assert!(
        pointer
            .output
            .headers
            .iter()
            .all(|header| !header.response.state.focused)
    );

    let mut keyboard_memory = UiMemory::new();
    keyboard_memory.focus(header_response(&seed, column).id);
    for key in [Key::Enter, Key::Space] {
        let keyboard = run_frame(
            &items,
            table_config.clone(),
            &mut VirtualTableSelection::new(),
            &mut keyboard_memory,
            key_input(key, Modifiers::default(), false),
        );
        assert_eq!(
            keyboard.output.sort_requested,
            Some(TableSort {
                column,
                direction: SortDirection::Ascending,
            })
        );
        let response = header_response(&keyboard, column);
        assert!(response.clicked);
        assert!(response.keyboard_activated);
        assert!(response.state.focused);
        assert_eq!(keyboard_memory.focused(), Some(response.id));
    }

    let modified = Modifiers {
        shift: true,
        ..Modifiers::default()
    };
    for input in [
        key_input(Key::Enter, Modifiers::default(), true),
        key_input(Key::Space, modified, false),
    ] {
        let inert = run_frame(
            &items,
            table_config.clone(),
            &mut VirtualTableSelection::new(),
            &mut keyboard_memory,
            input,
        );
        assert_eq!(inert.output.sort_requested, None);
        assert_eq!(inert.output.resize_requested, None);
        assert!(!inert.output.selection_changed);
        assert_eq!(inert.output.cursor_target, None);
        assert_eq!(inert.frame.repaint, RepaintRequest::None);
        assert!(inert.frame.actions.is_empty());
        assert!(inert.frame.platform_requests.is_empty());
    }
}

#[test]
fn header_semantics_mirror_owned_focus_press_and_disabled_retention_without_selected_state() {
    let items = projection(1);
    let sort = Some(TableSort {
        column: id(20),
        direction: SortDirection::Ascending,
    });
    let seed = run_frame(
        &items,
        config(sort),
        &mut VirtualTableSelection::new(),
        &mut UiMemory::new(),
        UiInput::default(),
    );
    let column = id(20);
    let header_id = header_response(&seed, column).id;
    let point = header_response(&seed, column).rect.center();
    let mut memory = UiMemory::new();
    memory.focus(header_id);
    let enabled = run_frame(
        &items,
        config(sort),
        &mut VirtualTableSelection::new(),
        &mut memory,
        pointer_input(point, true),
    );
    let response = header_response(&enabled, column);
    assert!(response.state.focused);
    assert!(response.state.pressed);
    assert!(!response.state.selected);
    let semantic = enabled
        .frame
        .semantics
        .get(header_id)
        .expect("enabled header semantics");
    assert_eq!(semantic.role, SemanticRole::Cell);
    assert!(semantic.focusable);
    assert!(semantic.state.focused);
    assert!(semantic.state.pressed);
    assert!(!semantic.state.selected);
    assert_eq!(
        semantic
            .actions
            .iter()
            .map(|action| action.kind.clone())
            .collect::<Vec<_>>(),
        vec![SemanticActionKind::Focus, SemanticActionKind::Invoke]
    );

    let disabled = run_frame(
        &items,
        config(sort).disabled(true),
        &mut VirtualTableSelection::new(),
        &mut memory,
        pointer_input(point, true),
    );
    let response = header_response(&disabled, column);
    assert!(response.state.disabled);
    assert!(response.state.focused, "retained focus remains observable");
    assert!(!response.state.pressed, "disable cancels an owned press");
    assert!(!response.clicked);
    assert!(!response.keyboard_activated);
    assert_eq!(disabled.output.sort_requested, None);
    assert_eq!(disabled.output.resize_requested, None);
    assert!(!disabled.output.selection_changed);
    assert_eq!(disabled.output.cursor_target, None);
    assert_eq!(
        disabled
            .frame
            .primitives
            .iter()
            .filter(|primitive| matches!(primitive, Primitive::Path(_)))
            .count(),
        0
    );
    let semantic = disabled
        .frame
        .semantics
        .get(header_id)
        .expect("disabled retained-focus semantics");
    assert!(semantic.state.disabled);
    assert!(semantic.state.focused);
    assert!(!semantic.state.pressed);
    assert!(!semantic.focusable);
    assert!(semantic.actions.is_empty());

    let no_focus = run_frame(
        &items,
        config(sort).disabled(true),
        &mut VirtualTableSelection::new(),
        &mut UiMemory::new(),
        UiInput::default(),
    );
    let semantic = no_focus
        .frame
        .semantics
        .get(header_id)
        .expect("disabled header semantics");
    assert!(!semantic.state.focused);
    assert!(!semantic.state.pressed);
    assert!(!semantic.focusable);
    assert!(semantic.actions.is_empty());
}

#[test]
fn resize_ownership_isolated_from_header_focus_sort_semantics_and_annuli() {
    let items = projection(10);
    let bounds = Rect::new(BOUNDS.x, BOUNDS.y, 160.0, 64.0);
    let resize_config = config_with(bounds, None, [10, 20, 30]).resizable(true);
    let seed = run_frame(
        &items,
        resize_config.clone(),
        &mut VirtualTableSelection::new(),
        &mut UiMemory::new(),
        UiInput::default(),
    );
    let column = id(10);
    let seed_header = seed
        .output
        .headers
        .iter()
        .find(|header| header.column == column)
        .expect("resizable header");
    let header_id = seed_header.response.id;
    let resize_id = seed_header.resize_response.expect("resize response").id;
    assert_ne!(header_id, resize_id);
    let retained_scroll = Vec2::new(30.0, 40.0);
    let mut memory = UiMemory::new();
    memory.focus(header_id);
    memory.set_scroll_offset(seed.root, retained_scroll);
    let scrolled = run_frame(
        &items,
        resize_config.clone(),
        &mut VirtualTableSelection::new(),
        &mut memory,
        UiInput::default(),
    );
    assert_eq!(scrolled.output.window.offset, retained_scroll);
    assert_eq!(scrolled.output.scroll.offset, retained_scroll);
    let resize = scrolled
        .output
        .headers
        .iter()
        .find(|header| header.column == column)
        .and_then(|header| header.resize_response)
        .expect("scrolled resize response");
    assert_ne!(header_id, resize.id);
    assert_eq!(resize.id, resize_id);
    let point = Point::new(
        resize.rect.center().x - retained_scroll.x,
        resize.rect.center().y,
    );

    let pressed = run_frame(
        &items,
        resize_config.clone(),
        &mut VirtualTableSelection::new(),
        &mut memory,
        drag_input(point, true, true, false, 0.0),
    );
    assert_eq!(pressed.output.sort_requested, None);
    assert_eq!(pressed.output.window.offset, retained_scroll);
    assert_eq!(pressed.output.scroll.offset, retained_scroll);
    assert_eq!(memory.scroll_offset(seed.root), retained_scroll);
    assert!(header_response(&pressed, column).state.focused);
    assert_header_focus(&pressed, column, false);
    let moved_point = Point::new(point.x + 12.0, point.y);
    let moved = run_frame(
        &items,
        resize_config.clone(),
        &mut VirtualTableSelection::new(),
        &mut memory,
        drag_input(moved_point, true, false, false, 12.0),
    );
    assert_eq!(moved.output.sort_requested, None);
    assert_eq!(
        moved.output.resize_requested.map(|request| request.column),
        Some(column)
    );
    assert!(!moved.output.selection_changed);
    assert_eq!(moved.output.cursor_target, None);
    assert_eq!(moved.output.window.offset, retained_scroll);
    assert_eq!(moved.output.scroll.offset, retained_scroll);
    assert_eq!(memory.scroll_offset(seed.root), retained_scroll);
    assert_eq!(memory.focused(), Some(header_id));
    assert_header_focus(&moved, column, false);
    let _ = run_frame(
        &items,
        resize_config.clone(),
        &mut VirtualTableSelection::new(),
        &mut memory,
        drag_input(moved_point, false, false, true, 0.0),
    );

    let mut resize_focused_memory = UiMemory::new();
    resize_focused_memory.focus(resize_id);
    let resize_focused = run_frame(
        &items,
        resize_config,
        &mut VirtualTableSelection::new(),
        &mut resize_focused_memory,
        UiInput::default(),
    );
    assert!(
        resize_focused
            .output
            .headers
            .iter()
            .all(|header| !header.response.state.focused)
    );
    assert_eq!(
        resize_focused
            .frame
            .primitives
            .iter()
            .filter(|primitive| matches!(primitive, Primitive::Path(_)))
            .count(),
        0
    );
    assert!(resize_focused.frame.semantics.get(resize_id).is_none());
    assert!(
        resize_focused
            .frame
            .semantics
            .nodes()
            .iter()
            .all(|node| !node.state.focused)
    );
}

#[test]
fn column_reorder_preserves_semantic_identity_order_content_geometry_and_retained_focus() {
    let items = projection(4);
    let column = id(20);
    let sort = Some(TableSort {
        column,
        direction: SortDirection::Ascending,
    });
    let seed = run_frame(
        &items,
        config(sort),
        &mut VirtualTableSelection::new(),
        &mut UiMemory::new(),
        UiInput::default(),
    );
    let header_id = header_response(&seed, column).id;
    let mut memory = UiMemory::new();
    memory.focus(header_id);
    let original = run_frame(
        &items,
        config(sort),
        &mut VirtualTableSelection::new(),
        &mut memory,
        UiInput::default(),
    );
    let original_semantic = original
        .frame
        .semantics
        .get(header_id)
        .expect("original sorted header semantics")
        .clone();
    let reordered = run_frame(
        &items,
        config_with(BOUNDS, sort, [30, 10, 20]),
        &mut VirtualTableSelection::new(),
        &mut memory,
        UiInput::default(),
    );
    assert_eq!(header_response(&reordered, column).id, header_id);
    assert_eq!(memory.focused(), Some(header_id));
    assert_header_focus(&reordered, column, true);
    let header_row = reordered
        .frame
        .semantics
        .get(reordered.root.child("virtual-table-header-row"))
        .expect("reordered header row semantics");
    assert_eq!(
        header_row.children,
        reordered
            .output
            .headers
            .iter()
            .map(|header| header.response.id)
            .collect::<Vec<_>>()
    );
    assert_eq!(
        header_row.children,
        vec![
            reordered.root.child(("virtual-table-header", 30_u64)),
            reordered.root.child(("virtual-table-header", 10_u64)),
            header_id,
        ]
    );
    let reordered_response = header_response(&reordered, column);
    let reordered_semantic = reordered
        .frame
        .semantics
        .get(header_id)
        .expect("reordered sorted header semantics");
    assert_eq!(reordered_semantic.id, original_semantic.id);
    assert_eq!(reordered_semantic.role, original_semantic.role);
    assert_eq!(reordered_semantic.label, original_semantic.label);
    assert_eq!(reordered_semantic.label.as_deref(), Some("Kind ↑"));
    assert_eq!(
        reordered_semantic.state.value,
        original_semantic.state.value
    );
    assert_eq!(reordered_semantic.bounds, reordered_response.rect);
    assert_eq!(
        reordered_semantic.bounds,
        Rect::new(BOUNDS.x + 160.0, BOUNDS.y, 80.0, 20.25)
    );
    assert!(reordered_semantic.state.focused);
    assert!(!reordered_semantic.state.selected);
}

#[test]
fn row_and_cell_body_selection_never_sort_resize_or_create_header_annuli() {
    let items = projection(4);
    let body_point = Point::new(BOUNDS.x + 10.0, BOUNDS.y + 30.0);
    for (mode, expected_target) in [
        (
            VirtualTableSelectionMode::Row,
            VirtualTableTarget::Row(id(1)),
        ),
        (
            VirtualTableSelectionMode::Cell,
            VirtualTableTarget::Cell {
                row: id(1),
                column: id(10),
            },
        ),
    ] {
        let mut selection = VirtualTableSelection::new();
        let mut body_memory = UiMemory::new();
        let body_config = config(None).selection_mode(mode);
        let _ = run_frame(
            &items,
            body_config.clone(),
            &mut selection,
            &mut body_memory,
            pointer_input(body_point, true),
        );
        let selected = run_frame(
            &items,
            body_config,
            &mut selection,
            &mut body_memory,
            release_input(body_point),
        );
        assert_eq!(selection.target(), Some(expected_target));
        assert!(selected.output.selection_changed);
        assert_eq!(selected.output.sort_requested, None);
        assert_eq!(selected.output.resize_requested, None);
        assert!(selected.output.cursor_target.is_some());
        assert!(
            selected
                .output
                .selection_responses
                .iter()
                .any(|response| response.target == expected_target
                    && response.response.state.selected)
        );
        assert!(
            selected
                .output
                .headers
                .iter()
                .all(|header| !header.response.state.focused)
        );
        assert_eq!(
            selected
                .frame
                .primitives
                .iter()
                .filter(|primitive| matches!(primitive, Primitive::Path(_)))
                .count(),
            0,
            "body selection uses the unchanged body-cell painter"
        );
    }
}

fn assert_header_clip_transform(run: &Run, bounds: Rect, offset_x: f32) {
    let expected_clip = Rect::new(bounds.x, bounds.y, bounds.width, 20.25);
    assert!(matches!(
        run.frame.primitives[1],
        Primitive::ClipBegin { rect, .. } if rect == expected_clip
    ));
    assert_eq!(
        run.frame.primitives[2],
        Primitive::TransformBegin(Transform::translation(Vec2::new(-offset_x, 0.0)))
    );
    let transform_end = run
        .frame
        .primitives
        .iter()
        .position(|primitive| matches!(primitive, Primitive::TransformEnd))
        .expect("header transform end");
    assert!(matches!(
        run.frame.primitives[transform_end + 1],
        Primitive::ClipEnd { .. }
    ));
}

#[test]
fn fractional_scroll_keeps_partial_left_right_and_fully_clipped_focus_in_header_scope() {
    let items = projection(3);
    let bounds = Rect::new(3.25, 7.75, 120.0, 84.0);
    let narrow = config_with(bounds, None, [10, 20, 30]);
    let seed = run_frame(
        &items,
        narrow.clone(),
        &mut VirtualTableSelection::new(),
        &mut UiMemory::new(),
        UiInput::default(),
    );
    let column = id(10);
    let header_id = header_response(&seed, column).id;
    let logical_rect = header_response(&seed, column).rect;
    assert_eq!(logical_rect, Rect::new(bounds.x, bounds.y, 80.0, 20.25));

    let mut memory = UiMemory::new();
    memory.focus(header_id);
    memory.set_scroll_offset(seed.root, Vec2::new(30.25, 0.0));
    let partial = run_frame(
        &items,
        narrow.clone(),
        &mut VirtualTableSelection::new(),
        &mut memory,
        UiInput::default(),
    );
    assert_eq!(partial.output.window.offset.x, 30.25);
    assert_eq!(header_response(&partial, column).rect, logical_rect);
    let expected = assert_header_focus(&partial, column, false);
    assert_header_clip_transform(&partial, bounds, 30.25);

    let right_column = id(30);
    let right_header_id = header_response(&seed, right_column).id;
    let right_logical_rect = header_response(&seed, right_column).rect;
    assert_eq!(
        right_logical_rect,
        Rect::new(bounds.x + 160.0, bounds.y, 80.0, 20.25)
    );
    let mut right_memory = UiMemory::new();
    right_memory.focus(right_header_id);
    right_memory.set_scroll_offset(seed.root, Vec2::new(90.0, 0.0));
    let partial_right = run_frame(
        &items,
        narrow.clone(),
        &mut VirtualTableSelection::new(),
        &mut right_memory,
        UiInput::default(),
    );
    assert_eq!(partial_right.output.window.offset.x, 90.0);
    assert_eq!(
        header_response(&partial_right, right_column).rect,
        right_logical_rect
    );
    assert_header_focus(&partial_right, right_column, false);
    assert_header_clip_transform(&partial_right, bounds, 90.0);
    let right_semantic = partial_right
        .frame
        .semantics
        .get(right_header_id)
        .expect("partially right-clipped header semantics");
    assert_eq!(
        right_semantic.bounds,
        Rect::new(bounds.x + 70.0, bounds.y, 50.0, 20.25)
    );
    assert!(right_semantic.state.focused);

    memory.set_scroll_offset(seed.root, Vec2::new(120.0, 0.0));
    let clipped = run_frame(
        &items,
        narrow,
        &mut VirtualTableSelection::new(),
        &mut memory,
        UiInput::default(),
    );
    assert_eq!(clipped.output.window.offset.x, 120.0);
    assert_eq!(header_response(&clipped, column).rect, logical_rect);
    assert_eq!(assert_header_focus(&clipped, column, false), expected);
    assert_header_clip_transform(&clipped, bounds, 120.0);
    assert!(clipped.frame.semantics.get(header_id).is_none());
}

#[test]
fn ten_thousand_rows_compare_identically_modulo_two_header_paths_and_focus_bits() {
    let items = projection(10_000);
    let table_config = config(None).selection_mode(VirtualTableSelectionMode::Cell);
    let body_point = Point::new(BOUNDS.x + 10.0, BOUNDS.y + 30.0);
    let target = VirtualTableTarget::Cell {
        row: id(1),
        column: id(10),
    };
    let mut selection = VirtualTableSelection::new();
    let mut memory = UiMemory::new();
    let _ = run_frame(
        &items,
        table_config.clone(),
        &mut selection,
        &mut memory,
        pointer_input(body_point, true),
    );
    let selected = run_frame(
        &items,
        table_config.clone(),
        &mut selection,
        &mut memory,
        release_input(body_point),
    );
    assert_eq!(selection.target(), Some(target));
    assert_eq!(
        selected.output.cursor_target.map(|cursor| cursor.target),
        Some(target)
    );
    memory.clear_focus();
    let baseline = run_frame(
        &items,
        table_config.clone(),
        &mut selection,
        &mut memory,
        UiInput::default(),
    );
    assert!(!baseline.callbacks.is_empty());
    assert!(baseline.callbacks.len() < 10_000);
    assert_eq!(
        baseline.output.cursor_target.map(|cursor| cursor.target),
        Some(target)
    );
    assert!(
        baseline
            .output
            .selection_responses
            .iter()
            .any(|response| response.target == target && response.response.state.selected)
    );
    let column = id(30);
    memory.focus(header_response(&baseline, column).id);
    let focused = run_frame(
        &items,
        table_config,
        &mut selection,
        &mut memory,
        UiInput::default(),
    );
    assert_eq!(selection.target(), Some(target));
    assert_eq!(
        focused.output.cursor_target.map(|cursor| cursor.target),
        Some(target)
    );
    assert_eq!(focused.callbacks, baseline.callbacks);
    assert!(
        focused
            .output
            .selection_responses
            .iter()
            .any(|response| response.target == target && response.response.state.selected)
    );
    assert_header_focus(&focused, column, false);
    assert_focus_only_transition(&focused, &baseline);
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
        panic!("expected solid theme brush");
    };
    color
}

fn assert_ratio(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 0.000_01,
        "{actual} != {expected}"
    );
}

fn header_base_color(run: &Run, column: ItemId) -> Color {
    let Primitive::Rect(base) = &run.frame.primitives[header_base_index(run, column)] else {
        unreachable!()
    };
    solid(base.fill.expect("header base fill"))
}

fn header_text_evidence(run: &Run, column: ItemId) -> (String, Color, usize) {
    let base_index = header_base_index(run, column);
    let (index, text) = run
        .frame
        .primitives
        .iter()
        .enumerate()
        .skip(base_index + 1)
        .find_map(|(index, primitive)| match primitive {
            Primitive::Text(text) => Some((index, text)),
            _ => None,
        })
        .expect("header text primitive");
    (text.text.clone(), solid(text.brush), index)
}

fn header_resize_line_color(run: &Run, column: ItemId) -> Color {
    let response = run
        .output
        .headers
        .iter()
        .find(|header| header.column == column)
        .expect("header response");
    let handle = response.resize_response.expect("resize response");
    let (_, _, text_index) = header_text_evidence(run, column);
    let Primitive::Rect(line) = &run.frame.primitives[text_index + 1] else {
        panic!("resize line must follow header text");
    };
    assert_eq!(line.rect.height, handle.rect.height);
    assert_eq!(line.rect.center().x, handle.rect.center().x);
    assert!(line.stroke.is_none());
    solid(line.fill.expect("resize line fill"))
}

fn header_focus_colors(run: &Run, column: ItemId) -> (Color, Color) {
    let base_index = header_base_index(run, column);
    let Primitive::Path(primary) = &run.frame.primitives[base_index + 1] else {
        panic!("primary focus path");
    };
    let Primitive::Path(separator) = &run.frame.primitives[base_index + 2] else {
        panic!("separator focus path");
    };
    (
        solid(primary.fill.expect("primary focus fill")),
        solid(separator.fill.expect("separator focus fill")),
    )
}

#[test]
fn production_header_primitives_inventory_acc005_and_resize_nonconformities() {
    let theme = default_dark_theme();
    let items = projection(2);
    let column = id(20);
    let resizable = config(None).resizable(true);
    let seed = run_frame(
        &items,
        resizable.clone(),
        &mut VirtualTableSelection::new(),
        &mut UiMemory::new(),
        UiInput::default(),
    );
    let seed_response = header_response(&seed, column);
    let point = seed_response.rect.center();
    let mut focused_colors = None;

    for selected in [false, true] {
        let sort = selected.then_some(TableSort {
            column,
            direction: SortDirection::Ascending,
        });
        for focused in [false, true] {
            for interaction in [
                HeaderInteraction::Idle,
                HeaderInteraction::Hover,
                HeaderInteraction::Press,
            ] {
                let mut memory = UiMemory::new();
                if focused {
                    memory.focus(seed_response.id);
                }
                let run = run_frame(
                    &items,
                    config(sort).resizable(true),
                    &mut VirtualTableSelection::new(),
                    &mut memory,
                    interaction.input(point),
                );
                let response = header_response(&run, column);
                let state = ComponentState {
                    hovered: response.state.hovered,
                    pressed: response.state.pressed,
                    focused,
                    disabled: false,
                    selected,
                };
                let recipe = theme.row(state);
                let background = header_base_color(&run, column);
                let (label, label_color, _) = header_text_evidence(&run, column);
                assert_eq!(
                    label,
                    if selected { "Kind ↑" } else { "Kind" },
                    "actual label primitive includes the production sort arrow"
                );
                assert_eq!(background, solid(recipe.background));
                assert_eq!(label_color, recipe.foreground);
                let idle_resize = header_resize_line_color(&run, column);
                assert_eq!(idle_resize, theme.colors.border.subtle);

                if selected {
                    let ratio = contrast_ratio(label_color, background);
                    assert_ratio(ratio, 3.533_269);
                    assert!(ratio < 4.5, "ACC-005 named selected-label exception");
                    assert_ratio(contrast_ratio(idle_resize, background), 4.502_908);
                } else {
                    let ratio = contrast_ratio(idle_resize, background);
                    if matches!(interaction, HeaderInteraction::Idle) {
                        assert_ratio(ratio, 1.237_124);
                        assert!(ratio < 3.0, "idle neutral resize is a nonconformity");
                    }
                }

                if focused {
                    let colors = header_focus_colors(&run, column);
                    if let Some(expected) = focused_colors {
                        assert_eq!(colors, expected);
                    } else {
                        focused_colors = Some(colors);
                    }
                }
            }
        }
    }

    for selected in [false, true] {
        let sort = selected.then_some(TableSort {
            column,
            direction: SortDirection::Ascending,
        });
        let handle = seed
            .output
            .headers
            .iter()
            .find(|header| header.column == column)
            .and_then(|header| header.resize_response)
            .expect("resize handle");
        let active = run_frame(
            &items,
            config(sort).resizable(true),
            &mut VirtualTableSelection::new(),
            &mut UiMemory::new(),
            drag_input(handle.rect.center(), true, true, false, 0.0),
        );
        assert!(!header_response(&active, column).state.pressed);
        let active_resize = header_resize_line_color(&active, column);
        let background = header_base_color(&active, column);
        assert_eq!(active_resize, theme.colors.accent.default);
        if selected {
            let ratio = contrast_ratio(active_resize, background);
            assert_ratio(ratio, 1.0);
            assert!(ratio < 3.0, "active sorted resize is a nonconformity");
        } else {
            assert_ratio(contrast_ratio(active_resize, background), 5.570_656);
        }
    }

    let disabled = run_frame(
        &items,
        config(None).disabled(true).resizable(true),
        &mut VirtualTableSelection::new(),
        &mut UiMemory::new(),
        UiInput::default(),
    );
    let (_, disabled_label, _) = header_text_evidence(&disabled, column);
    let disabled_background = header_base_color(&disabled, column);
    let disabled_ratio = contrast_ratio(disabled_label, disabled_background);
    assert_ratio(disabled_ratio, 3.208_475);
    assert!(disabled_ratio < 4.5, "disabled text is not claimed as AA");

    let (primary, separator) = focused_colors.expect("actual focused header colors");
    let accent = theme.colors.accent.default;
    assert_ratio(contrast_ratio(primary, separator), 8.555_114);
    assert_ratio(contrast_ratio(separator, accent), 5.570_656);
    let direct_indicator_accent = contrast_ratio(primary, accent);
    assert_ratio(direct_indicator_accent, 1.535_746);
    assert!(direct_indicator_accent < 3.0);
    assert!(
        contrast_ratio(separator, accent) >= 3.0,
        "separator is the adjacent boundary"
    );
}
