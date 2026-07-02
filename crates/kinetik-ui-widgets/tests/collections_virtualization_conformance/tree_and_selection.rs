#[allow(unused_imports)]
use super::{
    BTreeMap, CollectionProjection, ItemId, ListLayout, Range, Rect, Selection, TableColumn,
    TableColumnConstraints, TableLayout, TreeExpansion, TreeItem, TreeLayout, TreeModel, TreeRow,
    VirtualRangeRequest, VirtualWindow, VirtualWindowRequest, assert_approx,
    assert_empty_finite_window, assert_range, assert_rect_finite, assert_window_finite, id,
    request, table_constraints, table_layout, tree_model, tree_rows, virtual_range, virtual_window,
};

#[test]
fn tree_layout_adapter_preserves_materialized_row_rects() {
    let layout = TreeLayout::new(10.0, 6.0);
    let rows = tree_rows();
    let window = layout.virtual_window(rows.len(), 15.0, 25.0, 1);

    assert_window_finite(&window);
    assert_approx(window.content_extent, 60.0);
    assert_approx(window.max_scroll_offset, 35.0);
    assert_approx(window.clamped_scroll_offset, 15.0);
    assert_range(window.visible_range, 1..4);
    assert_range(window.materialized_range.clone(), 0..6);
    assert_eq!(
        layout.visible_range(rows.len(), 15.0, 25.0, 1),
        window.materialized_range
    );

    let rects = layout.visible_row_rects(Rect::new(10.0, 100.0, 120.0, 25.0), &rows, 15.0, 1);
    assert_eq!(rects.len(), rows.len());
    assert_eq!(rects[0].row.id, ItemId::from_raw(1));
    assert_eq!(rects[2].row.id, ItemId::from_raw(3));
    assert_approx(rects[0].rect.y, 85.0);
    assert_approx(rects[2].content_rect.x, 22.0);
}

#[test]
fn tree_expansion_retain_model_removes_stale_ids() {
    let tree = TreeModel::new(vec![
        TreeItem {
            id: id(1),
            parent: None,
            has_children: false,
        },
        TreeItem {
            id: id(2),
            parent: Some(id(1)),
            has_children: false,
        },
    ]);
    let mut expansion = TreeExpansion::new();
    expansion.expand(id(2));
    expansion.expand(id(99));

    assert!(expansion.retain_model(&tree));
    assert_eq!(expansion.expanded(), vec![id(2)]);
}

#[test]
fn tree_expansion_retain_visible_removes_hidden_descendants() {
    let tree = tree_model();
    let mut expansion = TreeExpansion::new();
    expansion.expand(id(2));

    assert_eq!(tree.visible_item_ids(&expansion), vec![id(1), id(4)]);
    assert!(expansion.retain_visible(&tree));
    assert!(expansion.expanded().is_empty());
}

#[test]
fn tree_expansion_retain_visible_is_deterministic_and_idempotent() {
    let tree = tree_model();
    let mut expansion = TreeExpansion::new();
    expansion.expand(id(3));
    expansion.expand(id(1));
    expansion.expand(id(2));
    expansion.expand(id(99));

    assert_eq!(expansion.expanded(), vec![id(1), id(2), id(3), id(99)]);
    assert!(expansion.retain_visible(&tree));
    assert_eq!(expansion.expanded(), vec![id(1), id(2), id(3)]);

    assert!(!expansion.retain_visible(&tree));
    assert_eq!(expansion.expanded(), vec![id(1), id(2), id(3)]);
}

#[test]
fn unknown_expansion_ids_do_not_affect_tree_visible_rows_before_cleanup() {
    let tree = tree_model();
    let mut expansion = TreeExpansion::new();
    expansion.expand(id(1));
    expansion.expand(id(404));

    assert_eq!(tree.visible_item_ids(&expansion), vec![id(1), id(2), id(4)]);

    assert!(expansion.retain_model(&tree));
    assert_eq!(expansion.expanded(), vec![id(1)]);
    assert_eq!(tree.visible_item_ids(&expansion), vec![id(1), id(2), id(4)]);
}

#[test]
fn invalid_tree_cleanup_clears_expansion_without_panicking() {
    let invalid = TreeModel::new(vec![TreeItem {
        id: id(1),
        parent: Some(id(99)),
        has_children: false,
    }]);
    let mut expansion = TreeExpansion::new();
    expansion.expand(id(1));
    expansion.expand(id(99));

    assert!(expansion.retain_model(&invalid));
    assert!(expansion.expanded().is_empty());
    assert!(!expansion.retain_visible(&invalid));
}

#[test]
fn selection_range_survives_reorder_and_uses_visible_id_order() {
    let mut selection = Selection::new();
    selection.replace(id(20));

    let visible_after_reorder = [id(40), id(10), id(20), id(50), id(30)];
    assert!(selection.select_range(&visible_after_reorder, id(40)));

    assert_eq!(selection.anchor(), Some(id(20)));
    assert_eq!(selection.active, Some(id(40)));
    assert_eq!(selection.selected(), vec![id(10), id(20), id(40)]);
}

#[test]
fn selection_retain_visible_removes_hidden_selection_active_and_anchor() {
    let mut selection = Selection::new();
    selection.replace(id(2));
    selection.toggle(id(4));

    assert!(selection.retain_visible(&[id(1), id(2), id(3)]));

    assert_eq!(selection.selected(), vec![id(2)]);
    assert_eq!(selection.active, None);
    assert_eq!(selection.anchor(), None);
    assert!(!selection.anchor_visible(&[id(1), id(2), id(3)]));
}

#[test]
fn selection_retain_visible_reports_noop_when_everything_remains_visible() {
    let mut selection = Selection::new();
    selection.replace(id(2));
    selection.toggle(id(4));

    assert!(!selection.retain_visible(&[id(4), id(2), id(9)]));

    assert_eq!(selection.selected(), vec![id(2), id(4)]);
    assert_eq!(selection.active, Some(id(4)));
    assert_eq!(selection.anchor(), Some(id(4)));
    assert!(selection.anchor_visible(&[id(4), id(2), id(9)]));
}

#[test]
fn selection_range_failure_preserves_state_when_endpoint_is_missing() {
    let mut selection = Selection::new();
    selection.replace(id(3));
    selection.toggle(id(1));

    assert!(!selection.select_range(&[id(5), id(3), id(9), id(1)], id(99)));

    assert_eq!(selection.selected(), vec![id(1), id(3)]);
    assert_eq!(selection.active, Some(id(1)));
    assert_eq!(selection.anchor(), Some(id(1)));
}

#[test]
fn selection_range_failure_preserves_state_when_anchor_is_missing() {
    let mut selection = Selection::new();
    selection.replace(id(3));

    assert!(!selection.select_range(&[id(5), id(9), id(1)], id(9)));

    assert_eq!(selection.selected(), vec![id(3)]);
    assert_eq!(selection.active, Some(id(3)));
    assert_eq!(selection.anchor(), Some(id(3)));
}
