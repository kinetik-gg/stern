#[allow(unused_imports)]
use super::{
    BTreeMap, CollectionProjection, ItemId, ListLayout, Range, Rect, Selection, TableColumn,
    TableColumnConstraints, TableLayout, TreeExpansion, TreeItem, TreeLayout, TreeModel, TreeRow,
    VirtualRangeRequest, VirtualWindow, VirtualWindowRequest, assert_approx,
    assert_empty_finite_window, assert_range, assert_rect_finite, assert_window_finite, id,
    request, table_constraints, table_layout, tree_model, tree_rows, virtual_range, virtual_window,
};

fn assert_composed_list_window_is_bounded() {
    let list_count = 50_000;
    let list = ListLayout::new(12.0);
    let list_window = list.virtual_window(list_count, 1234.0, 60.0, 2);

    assert_window_finite(&list_window);
    assert_range(list_window.visible_range, 102..108);
    assert_range(list_window.materialized_range.clone(), 100..110);
    assert!(list_window.materialized_range.len() < list_count);

    let list_rects =
        list.visible_row_rects(Rect::new(0.0, 0.0, 240.0, 60.0), list_count, 1234.0, 2);
    assert_eq!(list_rects.len(), 10);
    assert_eq!(list_rects[0].index, 100);
    assert_eq!(list_rects[9].index, 109);
    assert_approx(list_rects[0].rect.y, -34.0);
    for row in &list_rects {
        assert_rect_finite(row.rect);
    }
}

fn assert_composed_table_window_is_bounded_and_constrained() {
    let table_count = 100_000;
    let table = TableLayout {
        columns: vec![
            TableColumn {
                id: id(10),
                header: "Name".to_owned(),
                width: 40.0,
            },
            TableColumn {
                id: id(20),
                header: "Status".to_owned(),
                width: 500.0,
            },
            TableColumn {
                id: id(30),
                header: "Kind".to_owned(),
                width: 10.0,
            },
        ],
        header_height: 20.0,
        row_height: 10.0,
        sort: None,
    };
    let constraints = table_constraints([
        (id(10), TableColumnConstraints::new(50.0, 80.0)),
        (id(20), TableColumnConstraints::new(120.0, 160.0)),
        (id(30), TableColumnConstraints::new(24.0, 32.0)),
    ]);
    let table_bounds = Rect::new(10.0, 5.0, 400.0, 70.0);
    let table_window = table.body_virtual_window(table_count, 4567.0, table_bounds.height, 1);

    assert_window_finite(&table_window);
    assert_range(table_window.visible_range, 456..462);
    assert_range(table_window.materialized_range.clone(), 455..463);
    assert!(table_window.materialized_range.len() < table_count);
    assert_approx(table.total_width_with_constraints(&constraints), 234.0);

    let headers = table.header_cells_with_constraints(table_bounds, &constraints);
    assert_eq!(headers.len(), 3);
    assert_approx(headers[0].rect.width, 50.0);
    assert_approx(headers[1].rect.x, 60.0);
    assert_approx(headers[1].rect.width, 160.0);
    assert_approx(headers[2].rect.x, 220.0);
    assert_approx(headers[2].rect.width, 24.0);
    for header in &headers {
        assert_rect_finite(header.rect);
    }

    let cells = table.visible_body_cells_with_constraints(
        table_bounds,
        table_count,
        4567.0,
        1,
        &constraints,
    );
    assert_eq!(cells.len(), 24);
    assert_eq!(cells[0].row, 455);
    assert_eq!(cells[0].column_id, id(10));
    assert_eq!(cells[2].column_id, id(30));
    assert_eq!(cells[23].row, 462);
    assert_eq!(cells[23].column_id, id(30));
    assert_approx(cells[0].rect.y, 8.0);
    assert_approx(cells[0].rect.width, 50.0);
    assert_approx(cells[1].rect.x, 60.0);
    assert_approx(cells[1].rect.width, 160.0);
    assert_approx(cells[2].rect.x, 220.0);
    assert_approx(cells[2].rect.width, 24.0);
    for cell in &cells {
        assert_rect_finite(cell.rect);
    }
}

fn assert_composed_tree_window_cleans_state() {
    let tree = TreeModel::new(vec![
        TreeItem {
            id: id(1),
            parent: None,
            has_children: true,
        },
        TreeItem {
            id: id(2),
            parent: Some(id(1)),
            has_children: true,
        },
        TreeItem {
            id: id(3),
            parent: Some(id(2)),
            has_children: false,
        },
        TreeItem {
            id: id(4),
            parent: Some(id(1)),
            has_children: false,
        },
        TreeItem {
            id: id(5),
            parent: None,
            has_children: true,
        },
        TreeItem {
            id: id(6),
            parent: Some(id(5)),
            has_children: false,
        },
        TreeItem {
            id: id(7),
            parent: None,
            has_children: false,
        },
    ]);
    let mut expansion = TreeExpansion::new();
    expansion.expand(id(1));
    expansion.expand(id(2));
    expansion.expand(id(6));
    expansion.expand(id(99));

    assert!(expansion.retain_model(&tree));
    assert_eq!(expansion.expanded(), vec![id(1), id(2), id(6)]);
    assert!(expansion.retain_visible(&tree));
    assert_eq!(expansion.expanded(), vec![id(1), id(2)]);

    let tree_rows = tree.visible_rows(&expansion);
    assert_eq!(
        tree_rows.iter().map(|row| row.id).collect::<Vec<_>>(),
        vec![id(1), id(2), id(3), id(4), id(5), id(7)]
    );

    let mut selection = Selection::new();
    selection.replace(id(2));
    selection.toggle(id(6));
    selection.toggle(id(99));
    let tree_visible_ids = tree.visible_item_ids(&expansion);

    assert!(selection.retain_visible(&tree_visible_ids));
    assert_eq!(selection.selected(), vec![id(2)]);
    assert_eq!(selection.active, None);
    assert_eq!(selection.anchor(), None);

    let tree_layout = TreeLayout::new(14.0, 10.0);
    let tree_window = tree_layout.virtual_window(tree_rows.len(), 28.0, 28.0, 0);

    assert_window_finite(&tree_window);
    assert_range(tree_window.visible_range, 2..4);
    assert_range(tree_window.materialized_range, 2..5);

    let tree_rects =
        tree_layout.visible_row_rects(Rect::new(20.0, 100.0, 180.0, 28.0), &tree_rows, 28.0, 0);
    assert_eq!(tree_rects.len(), 3);
    assert_eq!(
        tree_rects.iter().map(|row| row.row.id).collect::<Vec<_>>(),
        vec![id(3), id(4), id(5)]
    );
    assert_approx(tree_rects[0].rect.y, 100.0);
    assert_approx(tree_rects[0].content_rect.x, 40.0);
    assert_approx(tree_rects[1].content_rect.x, 30.0);
    assert_approx(tree_rects[2].content_rect.x, 20.0);
    for row in &tree_rects {
        assert_rect_finite(row.rect);
        assert_rect_finite(row.content_rect);
    }
}

#[test]
fn collection_models_compose_bounded_visible_state_without_renderer() {
    assert_composed_list_window_is_bounded();
    assert_composed_table_window_is_bounded_and_constrained();
    assert_composed_tree_window_cleans_state();
}

#[test]
fn adapters_return_empty_finite_windows_for_invalid_extents() {
    let list = ListLayout::new(f32::NAN);
    assert_empty_finite_window(&list.virtual_window(10, 0.0, 30.0, 0));

    let mut table = table_layout();
    table.header_height = 60.0;
    assert_empty_finite_window(&table.body_virtual_window(10, 0.0, 50.0, 0));

    table.header_height = 20.0;
    table.row_height = 0.0;
    assert_empty_finite_window(&table.body_virtual_window(10, 0.0, 50.0, 0));

    let tree = TreeLayout::new(f32::INFINITY, 12.0);
    assert_empty_finite_window(&tree.virtual_window(10, 0.0, 50.0, 0));
}
