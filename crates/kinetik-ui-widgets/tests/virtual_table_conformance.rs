//! Public prepared/painted virtual-table foundation conformance tests.

use std::time::Duration;

use kinetik_ui_core::{
    FrameContext, Modifiers, PhysicalSize, Point, PointerButtonState, PointerInput, PointerOrder,
    PointerTarget, Primitive, Rect, Response, ScaleFactor, SemanticActionKind, SemanticRole, Size,
    TimeInfo, UiInput, UiMemory, Vec2, ViewportInfo, WidgetId, default_dark_theme,
};
use kinetik_ui_widgets::{
    CollectionProjection, ItemId, SortDirection, TableColumn, TableLayout, TableSort, Ui,
    VirtualTableConfig, VirtualTableOutput, VirtualTableRow,
};

const BOUNDS: Rect = Rect::new(0.0, 0.0, 120.0, 80.0);
const LOWER: Rect = Rect::new(0.0, 0.0, 180.0, 120.0);

fn id(raw: u64) -> ItemId {
    ItemId::from_raw(raw)
}

fn projection(raw_ids: &[u64]) -> CollectionProjection {
    CollectionProjection::from_source_ids(&raw_ids.iter().copied().map(id).collect::<Vec<_>>())
}

fn layout(sort: Option<TableSort>) -> TableLayout {
    TableLayout {
        columns: vec![
            TableColumn::new(id(10), "Name", 80.0),
            TableColumn::new(id(20), "Kind", 80.0),
            TableColumn::new(id(30), "Size", 80.0),
        ],
        header_height: 20.0,
        row_height: 20.0,
        sort,
    }
}

fn config(sort: Option<TableSort>) -> VirtualTableConfig {
    VirtualTableConfig::new(BOUNDS, layout(sort))
        .label("Assets")
        .overscan(1)
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

fn pointer_input(x: f32, y: f32, pressed: bool, released: bool) -> UiInput {
    UiInput {
        pointer: PointerInput {
            position: Some(Point::new(x, y)),
            primary: PointerButtonState::new(pressed, pressed, released),
            ..PointerInput::default()
        },
        keyboard: kinetik_ui_core::KeyboardInput {
            modifiers: Modifiers::default(),
            events: Vec::new(),
        },
        ..UiInput::default()
    }
}

fn wheel_input(x: f32, y: f32) -> UiInput {
    UiInput {
        pointer: PointerInput {
            position: Some(Point::new(100.0, 40.0)),
            wheel_delta: Vec2::new(x, y),
            ..PointerInput::default()
        },
        ..UiInput::default()
    }
}

struct Run {
    table_id: WidgetId,
    lower: Option<Response>,
    output: VirtualTableOutput,
    callbacks: Vec<ItemId>,
    frame: kinetik_ui_core::FrameOutput,
}

fn run_frame(
    projection: &CollectionProjection,
    config: VirtualTableConfig,
    memory: &mut UiMemory,
    input: UiInput,
    lower: bool,
) -> Run {
    let theme = default_dark_theme();
    let mut ui = Ui::begin_frame(context(input), memory, &theme);
    let table = ui
        .prepare_virtual_table("table", config, projection)
        .expect("valid table");
    let table_id = table.widget_id();
    let lower_id = ui.make_id("lower");
    ui.resolve_pointer_targets(|plan| {
        if lower {
            plan.target(PointerTarget::new(lower_id, LOWER, PointerOrder::new(10)));
        }
        table.declare_pointer_targets(plan, PointerOrder::new(100));
    })
    .expect("valid shared pointer plan");
    let lower_response = lower.then(|| ui.pressable("lower", LOWER, false));
    let mut callbacks = Vec::new();
    let output = ui.virtual_table(&table, |item| {
        callbacks.push(item.id);
        VirtualTableRow::new([
            format!("Row {} name", item.id.raw()),
            format!("Row {} kind", item.id.raw()),
            format!("Row {} size", item.id.raw()),
        ])
    });
    let frame = ui.finish_output();
    Run {
        table_id,
        lower: lower_response,
        output,
        callbacks,
        frame,
    }
}

fn click_header(
    x: f32,
    projection: &CollectionProjection,
    config: VirtualTableConfig,
    memory: &mut UiMemory,
    lower: bool,
) -> Run {
    let _ = run_frame(
        projection,
        config.clone(),
        memory,
        pointer_input(x, 10.0, true, false),
        lower,
    );
    run_frame(
        projection,
        config,
        memory,
        pointer_input(x, 10.0, false, true),
        lower,
    )
}

#[test]
fn hundred_thousand_rows_materialize_bounded_cells_and_semantics() {
    let rows = (0..100_000).collect::<Vec<_>>();
    let items = projection(&rows);
    let mut memory = UiMemory::new();
    let run = run_frame(&items, config(None), &mut memory, UiInput::default(), false);

    assert_eq!(run.output.window.body.visible_range, 0..3);
    assert_eq!(run.output.window.body.materialized_range, 0..5);
    assert_eq!(run.callbacks, vec![id(0), id(1), id(2), id(3), id(4)]);
    assert_eq!(run.output.rows.len(), 5);
    assert_eq!(
        run.frame
            .primitives
            .iter()
            .filter(|primitive| matches!(primitive, Primitive::Text(_)))
            .count(),
        18
    );

    let root = run.frame.semantics.get(run.table_id).expect("table root");
    assert_eq!(root.role, SemanticRole::Table);
    assert_eq!(root.children.len(), 4);
    let header_row = run
        .frame
        .semantics
        .get(run.table_id.child("virtual-table-header-row"))
        .expect("header row");
    assert_eq!(header_row.role, SemanticRole::Row);
    assert_eq!(header_row.children.len(), 2);
    let first_row = run
        .frame
        .semantics
        .get(run.table_id.child(("virtual-table-row", 0_u64)))
        .expect("first row");
    assert_eq!(first_row.role, SemanticRole::Row);
    assert_eq!(first_row.children.len(), 2);
    assert_eq!(
        run.frame
            .semantics
            .get(run.table_id.child(("virtual-table-cell", 0_u64, 10_u64)))
            .expect("first cell")
            .role,
        SemanticRole::Cell
    );
}

#[test]
fn two_axis_wheel_freezes_geometry_and_keeps_header_vertical_position() {
    let items = projection(&(0..20).collect::<Vec<_>>());
    let mut memory = UiMemory::new();
    let current = run_frame(
        &items,
        config(None),
        &mut memory,
        wheel_input(-30.0, -20.0),
        false,
    );
    assert_eq!(current.output.window.offset, Vec2::ZERO);
    assert_eq!(current.output.scroll.offset.x.to_bits(), 30.0_f32.to_bits());
    assert_eq!(current.output.scroll.offset.y.to_bits(), 20.0_f32.to_bits());

    let next = run_frame(&items, config(None), &mut memory, UiInput::default(), false);
    assert_eq!(next.output.window.offset.x.to_bits(), 30.0_f32.to_bits());
    assert_eq!(next.output.window.offset.y.to_bits(), 20.0_f32.to_bits());
    assert_eq!(next.output.window.body.visible_range, 1..4);

    let header = next
        .frame
        .semantics
        .get(next.table_id.child(("virtual-table-header", 10_u64)))
        .expect("partially visible header");
    let cell = next
        .frame
        .semantics
        .get(next.table_id.child(("virtual-table-cell", 1_u64, 10_u64)))
        .expect("first visible body cell");
    assert_eq!(header.bounds.x.to_bits(), 0.0_f32.to_bits());
    assert_eq!(header.bounds.width.to_bits(), 50.0_f32.to_bits());
    assert_eq!(header.bounds.y.to_bits(), 0.0_f32.to_bits());
    assert_eq!(cell.bounds.x.to_bits(), 0.0_f32.to_bits());
    assert_eq!(cell.bounds.width.to_bits(), 50.0_f32.to_bits());
    assert_eq!(cell.bounds.y.to_bits(), 20.0_f32.to_bits());
}

#[test]
fn header_click_emits_sort_intent_without_reordering_projection() {
    let items = projection(&[1, 2, 3]);
    let mut memory = UiMemory::new();
    let ascending = click_header(40.0, &items, config(None), &mut memory, true);
    assert!(ascending.lower.is_some_and(|response| !response.clicked));
    assert_eq!(
        ascending.output.sort_requested,
        Some(TableSort {
            column: id(10),
            direction: SortDirection::Ascending,
        })
    );
    assert_eq!(ascending.callbacks, vec![id(1), id(2), id(3)]);
    let header = ascending
        .frame
        .semantics
        .get(ascending.table_id.child(("virtual-table-header", 10_u64)))
        .expect("sortable header");
    assert!(
        header
            .actions
            .iter()
            .any(|action| action.kind == SemanticActionKind::Invoke)
    );

    let current = TableSort {
        column: id(10),
        direction: SortDirection::Ascending,
    };
    let descending = click_header(40.0, &items, config(Some(current)), &mut memory, false);
    assert_eq!(
        descending.output.sort_requested,
        Some(TableSort {
            column: id(10),
            direction: SortDirection::Descending,
        })
    );
    let other = click_header(
        100.0,
        &items,
        config(descending.output.sort_requested),
        &mut memory,
        false,
    );
    assert_eq!(
        other.output.sort_requested,
        Some(TableSort {
            column: id(20),
            direction: SortDirection::Ascending,
        })
    );
}

#[test]
fn stable_header_row_and_cell_ids_survive_projection_reorder() {
    let original = projection(&[1, 2, 3]);
    let reordered = projection(&[3, 1, 2]);
    let mut memory = UiMemory::new();
    let first = run_frame(
        &original,
        config(None),
        &mut memory,
        UiInput::default(),
        false,
    );
    let row = first.table_id.child(("virtual-table-row", 1_u64));
    let cell = first.table_id.child(("virtual-table-cell", 1_u64, 10_u64));
    let header = first.table_id.child(("virtual-table-header", 10_u64));
    assert!(first.frame.semantics.get(row).is_some());
    assert!(first.frame.semantics.get(cell).is_some());
    assert!(first.frame.semantics.get(header).is_some());

    let second = run_frame(
        &reordered,
        config(None),
        &mut memory,
        UiInput::default(),
        false,
    );
    assert!(second.frame.semantics.get(row).is_some());
    assert!(second.frame.semantics.get(cell).is_some());
    assert!(second.frame.semantics.get(header).is_some());
    let root = second.frame.semantics.get(second.table_id).expect("root");
    assert_eq!(
        root.children[1],
        second.table_id.child(("virtual-table-row", 3_u64))
    );
    assert_eq!(root.children[2], row);
}

#[test]
fn empty_disabled_and_invalid_tables_are_inert_or_rejected() {
    let empty = projection(&[]);
    let mut memory = UiMemory::new();
    let empty_run = run_frame(&empty, config(None), &mut memory, UiInput::default(), false);
    assert!(empty_run.callbacks.is_empty());
    assert!(empty_run.output.rows.is_empty());
    assert_eq!(
        empty_run
            .frame
            .semantics
            .get(empty_run.table_id)
            .expect("empty table")
            .children,
        vec![empty_run.table_id.child("virtual-table-header-row")]
    );

    let items = projection(&[1]);
    let disabled = config(None).disabled(true);
    let _ = run_frame(
        &items,
        disabled.clone(),
        &mut memory,
        pointer_input(40.0, 10.0, true, false),
        true,
    );
    let released = run_frame(
        &items,
        disabled,
        &mut memory,
        pointer_input(40.0, 10.0, false, true),
        true,
    );
    assert!(released.lower.is_some_and(|response| !response.clicked));
    assert_eq!(released.output.sort_requested, None);
    let disabled_wheel = run_frame(
        &items,
        config(None).disabled(true),
        &mut memory,
        wheel_input(-30.0, -20.0),
        false,
    );
    assert_eq!(disabled_wheel.output.scroll.delta, Vec2::ZERO);

    let theme = default_dark_theme();
    let mut invalid_memory = UiMemory::new();
    let ui = Ui::begin_frame(context(UiInput::default()), &mut invalid_memory, &theme);
    assert!(
        ui.prepare_virtual_table(
            "bad-bounds",
            VirtualTableConfig::new(Rect::new(0.0, 0.0, f32::NAN, 80.0), layout(None),),
            &items,
        )
        .is_none()
    );
    let mut bad_header = layout(None);
    bad_header.header_height = 0.0;
    assert!(
        ui.prepare_virtual_table(
            "bad-header",
            VirtualTableConfig::new(BOUNDS, bad_header),
            &items,
        )
        .is_none()
    );
    let mut bad_row = layout(None);
    bad_row.row_height = 0.0;
    assert!(
        ui.prepare_virtual_table("bad-row", VirtualTableConfig::new(BOUNDS, bad_row), &items,)
            .is_none()
    );
    let mut duplicate = layout(None);
    duplicate.columns[1].id = duplicate.columns[0].id;
    assert!(
        ui.prepare_virtual_table(
            "duplicate",
            VirtualTableConfig::new(BOUNDS, duplicate),
            &items,
        )
        .is_none()
    );
    let mut zero_width = layout(None);
    zero_width.columns[0].width = 0.0;
    assert!(
        ui.prepare_virtual_table(
            "zero-width",
            VirtualTableConfig::new(BOUNDS, zero_width),
            &items,
        )
        .is_none()
    );
}
