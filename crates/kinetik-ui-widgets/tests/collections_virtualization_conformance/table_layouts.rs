#[allow(unused_imports)]
use super::{
    BTreeMap, CollectionProjection, ItemId, ListLayout, Range, Rect, Selection, TableColumn,
    TableColumnConstraints, TableLayout, TreeExpansion, TreeItem, TreeLayout, TreeModel, TreeRow,
    VirtualRangeRequest, VirtualWindow, VirtualWindowRequest, assert_approx,
    assert_empty_finite_window, assert_range, assert_rect_finite, assert_window_finite, id,
    request, table_constraints, table_layout, tree_model, tree_rows, virtual_range, virtual_window,
};

#[test]
fn table_body_adapter_subtracts_header_height_for_rows() {
    let table = table_layout();
    let window = table.body_virtual_window(100, 20.0, 55.0, 1);

    assert_window_finite(&window);
    assert_approx(window.content_extent, 1000.0);
    assert_approx(window.max_scroll_offset, 970.0);
    assert_approx(window.clamped_scroll_offset, 20.0);
    assert_range(window.visible_range, 2..5);
    assert_range(window.materialized_range.clone(), 1..7);
    assert_eq!(
        table.visible_row_range(100, 20.0, 55.0, 1),
        window.materialized_range
    );

    let full_height_window = virtual_window(request(100, 20.0, 55.0, 10.0, 1));
    assert_range(full_height_window.visible_range, 2..8);

    let cells = table.visible_body_cells(Rect::new(0.0, 0.0, 120.0, 55.0), 100, 20.0, 1);
    assert_eq!(cells.len(), 12);
    assert_eq!(cells[0].row, 1);
    assert_eq!(cells[0].column, 0);
    assert_approx(cells[0].rect.y, 15.0);
}

#[test]
fn table_columns_keep_default_unconstrained_width_behavior() {
    let mut table = table_layout();

    assert_approx(table.total_width(), 120.0);
    assert_eq!(table.column_width(id(1)), Some(80.0));

    let headers = table.header_cells(Rect::new(0.0, 0.0, 160.0, 50.0));
    assert_approx(headers[0].rect.width, 80.0);
    assert_approx(headers[1].rect.x, 80.0);
    assert_approx(headers[1].rect.width, 40.0);

    assert!(table.resize_column(id(1), 25.0));
    assert_eq!(table.column_width(id(1)), Some(105.0));
    assert_approx(table.total_width(), 145.0);
}

#[test]
fn table_column_constraints_clamp_and_sanitize_bounds() {
    let inverted = TableColumnConstraints::new(120.0, 80.0).sanitized();
    assert_approx(inverted.min_width, 120.0);
    assert_approx(inverted.max_width, 120.0);
    assert_approx(inverted.clamp_width(90.0), 120.0);

    let non_finite = TableColumnConstraints::new(f32::NAN, f32::INFINITY).sanitized();
    assert_approx(non_finite.min_width, 0.0);
    assert_approx(non_finite.max_width, f32::MAX);
    assert_approx(non_finite.clamp_width(f32::INFINITY), 0.0);

    let table = TableLayout {
        columns: vec![
            TableColumn {
                id: id(10),
                header: "Small".to_owned(),
                width: 40.0,
            },
            TableColumn {
                id: id(20),
                header: "Wide".to_owned(),
                width: 300.0,
            },
        ],
        header_height: 20.0,
        row_height: 10.0,
        sort: None,
    };
    let constraints = table_constraints([
        (id(10), TableColumnConstraints::new(50.0, 100.0)),
        (id(20), TableColumnConstraints::new(200.0, 120.0)),
    ]);

    assert_eq!(
        table.column_width_with_constraints(id(10), &constraints),
        Some(50.0)
    );
    assert_eq!(
        table.column_width_with_constraints(id(20), &constraints),
        Some(200.0)
    );
    assert_approx(table.total_width_with_constraints(&constraints), 250.0);
}

#[test]
fn table_resize_column_uses_item_id_and_clamps_to_constraints() {
    let mut table = table_layout();
    let constraints = table_constraints([
        (id(1), TableColumnConstraints::new(70.0, 90.0)),
        (id(2), TableColumnConstraints::new(10.0, 45.0)),
    ]);

    assert!(table.resize_column_with_constraints(id(2), 20.0, &constraints));
    assert_eq!(
        table.column_width_with_constraints(id(2), &constraints),
        Some(45.0)
    );
    assert_eq!(
        table.column_width_with_constraints(id(1), &constraints),
        Some(80.0)
    );

    assert!(table.resize_column_with_constraints(id(1), 30.0, &constraints));
    assert_eq!(
        table.column_width_with_constraints(id(1), &constraints),
        Some(90.0)
    );

    assert!(!table.resize_column_with_constraints(id(1), 5.0, &constraints));
    assert_eq!(
        table.column_width_with_constraints(id(1), &constraints),
        Some(90.0)
    );

    assert!(table.resize_column_with_constraints(id(1), -80.0, &constraints));
    assert_eq!(
        table.column_width_with_constraints(id(1), &constraints),
        Some(70.0)
    );
}

#[test]
fn table_resize_same_clamped_width_is_noop_when_stored_width_exceeds_constraints() {
    let mut table = TableLayout {
        columns: vec![TableColumn {
            id: id(1),
            header: "Name".to_owned(),
            width: 120.0,
        }],
        header_height: 25.0,
        row_height: 10.0,
        sort: None,
    };
    let constraints = table_constraints([(id(1), TableColumnConstraints::new(50.0, 100.0))]);
    let before = table.clone();

    assert_eq!(
        table.column_width_with_constraints(id(1), &constraints),
        Some(100.0)
    );
    assert!(!table.resize_column_with_constraints(id(1), 0.0, &constraints));
    assert_eq!(table, before);
    assert_eq!(
        table.column_width_with_constraints(id(1), &constraints),
        Some(100.0)
    );
}

#[test]
fn table_resize_unknown_column_id_is_noop() {
    let mut table = table_layout();
    let before = table.clone();
    let constraints = table_constraints([(id(1), TableColumnConstraints::new(70.0, 90.0))]);

    assert!(!table.resize_column_with_constraints(id(99), 20.0, &constraints));
    assert_eq!(table, before);
}

#[test]
fn table_header_and_body_rects_use_clamped_column_widths() {
    let table = table_layout();
    let constraints = table_constraints([
        (id(1), TableColumnConstraints::new(90.0, 120.0)),
        (id(2), TableColumnConstraints::new(10.0, 30.0)),
    ]);

    let headers =
        table.header_cells_with_constraints(Rect::new(10.0, 20.0, 200.0, 80.0), &constraints);
    assert_approx(headers[0].rect.width, 90.0);
    assert_approx(headers[1].rect.x, 100.0);
    assert_approx(headers[1].rect.width, 30.0);

    let cells = table.body_cells_with_constraints(
        Rect::new(10.0, 20.0, 200.0, 80.0),
        2,
        0..1,
        &constraints,
    );
    assert_approx(cells[0].rect.width, 90.0);
    assert_approx(cells[1].rect.x, 100.0);
    assert_approx(cells[1].rect.width, 30.0);

    let visible = table.visible_body_cells_with_constraints(
        Rect::new(10.0, 20.0, 200.0, 55.0),
        100,
        20.0,
        1,
        &constraints,
    );
    assert_approx(visible[0].rect.width, 90.0);
    assert_approx(visible[1].rect.x, 100.0);
    assert_approx(visible[1].rect.width, 30.0);
}
