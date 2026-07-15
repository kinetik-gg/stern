//! Public conformance evidence for owned virtual-table header focus annuli.

#![allow(clippy::float_cmp, clippy::too_many_lines)]

use std::time::Duration;

use stern_core::{
    ComponentState, FrameContext, Key, KeyEvent, KeyState, KeyboardInput, Modifiers, PathElement,
    PhysicalSize, Point, PointerButtonState, PointerInput, PointerOrder, Primitive, Rect,
    ScaleFactor, SemanticNode, Size, TimeInfo, UiInput, UiMemory, Vec2, ViewportInfo, WidgetId,
    default_dark_theme,
};
use stern_widgets::{
    CollectionProjection, ItemId, SortDirection, TableColumn, TableLayout, TableSort, Ui,
    VirtualTableConfig, VirtualTableOutput, VirtualTableRow, VirtualTableSelection,
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
    VirtualTableConfig::new(
        BOUNDS,
        TableLayout {
            columns: columns([10, 20, 30]),
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
    root: WidgetId,
    output: VirtualTableOutput,
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
    let output = ui.virtual_table(&table, selection, |item| {
        VirtualTableRow::new([
            format!("Row {} name", item.id.raw()),
            format!("Row {} kind", item.id.raw()),
            format!("Row {} size", item.id.raw()),
        ])
    });
    Run {
        root,
        output,
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
fn first_middle_last_headers_add_only_exact_owned_annuli_across_sort_hover_and_press() {
    let items = projection(3);
    let seed = run_frame(
        &items,
        config(None),
        &mut VirtualTableSelection::new(),
        &mut UiMemory::new(),
        UiInput::default(),
    );

    for (column, sort) in [
        (id(10), None),
        (
            id(20),
            Some(TableSort {
                column: id(20),
                direction: SortDirection::Ascending,
            }),
        ),
        (id(30), None),
    ] {
        let mut unfocused_memory = UiMemory::new();
        let unfocused = run_frame(
            &items,
            config(sort),
            &mut VirtualTableSelection::new(),
            &mut unfocused_memory,
            UiInput::default(),
        );
        assert_eq!(
            unfocused
                .frame
                .primitives
                .iter()
                .filter(|primitive| matches!(primitive, Primitive::Path(_)))
                .count(),
            0,
            "sort alone must not paint focus"
        );

        let header_id = seed
            .output
            .headers
            .iter()
            .find(|header| header.column == column)
            .expect("seed header")
            .response
            .id;
        let mut focused_memory = UiMemory::new();
        focused_memory.focus(header_id);
        let focused = run_frame(
            &items,
            config(sort),
            &mut VirtualTableSelection::new(),
            &mut focused_memory,
            UiInput::default(),
        );
        let expected = assert_header_focus(&focused, column, sort.is_some());
        assert_focus_only_transition(&focused, &unfocused);

        for input in [
            pointer_input(header_response(&focused, column).rect.center(), false),
            pointer_input(header_response(&focused, column).rect.center(), true),
        ] {
            let state_run = run_frame(
                &items,
                config(sort),
                &mut VirtualTableSelection::new(),
                &mut focused_memory,
                input,
            );
            assert_eq!(
                assert_header_focus(&state_run, column, sort.is_some()),
                expected,
                "hover/press may not alter owned annuli"
            );
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

    let mut modified = Modifiers::default();
    modified.ctrl = true;
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
