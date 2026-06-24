//! Fixed-extent collection virtualization conformance tests.

mod collections_virtualization_conformance {
    use std::collections::BTreeMap;
    use std::ops::Range;

    use kinetik_ui_core::Rect;
    use kinetik_ui_widgets::{
        ItemId, ListLayout, Selection, TableColumn, TableColumnConstraints, TableLayout,
        TreeLayout, TreeRow, VirtualRangeRequest, VirtualWindow, VirtualWindowRequest,
        virtual_range, virtual_window,
    };

    fn request(
        item_count: usize,
        scroll_offset: f32,
        viewport_extent: f32,
        item_extent: f32,
        overscan: usize,
    ) -> VirtualWindowRequest {
        VirtualWindowRequest {
            item_count,
            scroll_offset,
            viewport_extent,
            item_extent,
            overscan,
        }
    }

    fn assert_window_finite(window: &VirtualWindow) {
        assert!(window.content_extent.is_finite(), "{window:?}");
        assert!(window.max_scroll_offset.is_finite(), "{window:?}");
        assert!(window.clamped_scroll_offset.is_finite(), "{window:?}");
        assert!(window.content_extent >= 0.0, "{window:?}");
        assert!(window.max_scroll_offset >= 0.0, "{window:?}");
        assert!(window.clamped_scroll_offset >= 0.0, "{window:?}");
    }

    fn assert_approx(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() < f32::EPSILON,
            "expected {actual} to equal {expected}"
        );
    }

    fn assert_empty_finite_window(window: &VirtualWindow) {
        assert_window_finite(window);
        assert_approx(window.content_extent, 0.0);
        assert_approx(window.max_scroll_offset, 0.0);
        assert_approx(window.clamped_scroll_offset, 0.0);
        assert_eq!(window.visible_range, 0..0);
        assert_eq!(window.materialized_range, 0..0);
    }

    fn assert_range(range: Range<usize>, expected: Range<usize>) {
        assert_eq!(range, expected);
    }

    fn table_layout() -> TableLayout {
        TableLayout {
            columns: vec![
                TableColumn {
                    id: ItemId::from_raw(1),
                    header: "Name".to_owned(),
                    width: 80.0,
                },
                TableColumn {
                    id: ItemId::from_raw(2),
                    header: "State".to_owned(),
                    width: 40.0,
                },
            ],
            header_height: 25.0,
            row_height: 10.0,
            sort: None,
        }
    }

    fn tree_rows() -> Vec<TreeRow> {
        vec![
            TreeRow {
                row: 0,
                item_index: 0,
                id: ItemId::from_raw(1),
                parent: None,
                depth: 0,
                has_children: true,
                expanded: true,
            },
            TreeRow {
                row: 1,
                item_index: 1,
                id: ItemId::from_raw(2),
                parent: Some(ItemId::from_raw(1)),
                depth: 1,
                has_children: true,
                expanded: true,
            },
            TreeRow {
                row: 2,
                item_index: 2,
                id: ItemId::from_raw(3),
                parent: Some(ItemId::from_raw(2)),
                depth: 2,
                has_children: false,
                expanded: false,
            },
            TreeRow {
                row: 3,
                item_index: 3,
                id: ItemId::from_raw(4),
                parent: None,
                depth: 0,
                has_children: false,
                expanded: false,
            },
            TreeRow {
                row: 4,
                item_index: 4,
                id: ItemId::from_raw(5),
                parent: None,
                depth: 0,
                has_children: false,
                expanded: false,
            },
            TreeRow {
                row: 5,
                item_index: 5,
                id: ItemId::from_raw(6),
                parent: None,
                depth: 0,
                has_children: false,
                expanded: false,
            },
        ]
    }

    fn id(raw: u64) -> ItemId {
        ItemId::from_raw(raw)
    }

    fn table_constraints(
        entries: impl IntoIterator<Item = (ItemId, TableColumnConstraints)>,
    ) -> BTreeMap<ItemId, TableColumnConstraints> {
        entries.into_iter().collect()
    }

    #[test]
    fn virtual_window_distinguishes_visible_and_materialized_ranges() {
        let window = virtual_window(request(100, 50.0, 35.0, 10.0, 2));

        assert_window_finite(&window);
        assert_approx(window.content_extent, 1000.0);
        assert_approx(window.max_scroll_offset, 965.0);
        assert_approx(window.clamped_scroll_offset, 50.0);
        assert_range(window.visible_range, 5..9);
        assert_range(window.materialized_range, 3..12);
    }

    #[test]
    fn virtual_window_clamps_negative_and_overscrolled_offsets() {
        let negative = virtual_window(request(10, -25.0, 25.0, 10.0, 1));
        assert_approx(negative.clamped_scroll_offset, 0.0);
        assert_range(negative.visible_range, 0..3);
        assert_range(negative.materialized_range, 0..5);

        let overscrolled = virtual_window(request(10, 500.0, 30.0, 10.0, 1));
        assert_approx(overscrolled.max_scroll_offset, 70.0);
        assert_approx(overscrolled.clamped_scroll_offset, 70.0);
        assert_range(overscrolled.visible_range, 7..10);
        assert_range(overscrolled.materialized_range, 6..10);
    }

    #[test]
    fn virtual_window_handles_empty_invalid_and_nonfinite_inputs() {
        assert_empty_finite_window(&virtual_window(request(0, 0.0, 100.0, 20.0, 1)));

        for item_extent in [0.0, -1.0, f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            assert_empty_finite_window(&virtual_window(request(10, 0.0, 100.0, item_extent, 1)));
        }

        for viewport_extent in [0.0, -1.0, f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            assert_empty_finite_window(&virtual_window(request(10, 0.0, viewport_extent, 20.0, 1)));
        }

        for scroll_offset in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            let window = virtual_window(request(10, scroll_offset, 25.0, 10.0, 0));
            assert_window_finite(&window);
            assert_approx(window.clamped_scroll_offset, 0.0);
            assert_range(window.visible_range, 0..3);
            assert_range(window.materialized_range, 0..4);
        }
    }

    #[test]
    fn virtual_window_handles_exact_row_boundaries_and_partial_rows() {
        let exact = virtual_window(request(20, 10.0, 30.0, 10.0, 0));
        assert_range(exact.visible_range, 1..4);
        assert_range(exact.materialized_range, 1..5);

        let partial_scroll = virtual_window(request(20, 15.0, 30.0, 10.0, 0));
        assert_range(partial_scroll.visible_range, 1..5);
        assert_range(partial_scroll.materialized_range, 1..5);

        let partial_viewport = virtual_window(request(20, 0.0, 35.0, 10.0, 0));
        assert_range(partial_viewport.visible_range, 0..4);
        assert_range(partial_viewport.materialized_range, 0..5);
    }

    #[test]
    fn virtual_window_bounds_extreme_overscan_to_item_count() {
        let window = virtual_window(request(6, 20.0, 20.0, 10.0, usize::MAX));

        assert_window_finite(&window);
        assert_range(window.visible_range, 2..4);
        assert_range(window.materialized_range, 0..6);
    }

    #[test]
    fn virtual_window_bounds_huge_finite_viewport_materialized_range() {
        let window = virtual_window(request(4, 0.0, f32::MAX, 1.0, 0));

        assert_window_finite(&window);
        assert_range(window.visible_range, 0..4);
        assert_range(window.materialized_range, 0..4);
    }

    #[test]
    fn virtual_window_keeps_content_and_scroll_extents_finite_for_large_counts() {
        let window = virtual_window(request(usize::MAX, f32::MAX, 1.0, 1.0e30, 1));

        assert_window_finite(&window);
        assert!(window.content_extent > 1.0e38);
        assert!(window.max_scroll_offset > 1.0e38);
        assert!(window.clamped_scroll_offset > 1.0e38);
        assert!(window.visible_range.start <= window.visible_range.end);
        assert!(window.materialized_range.start <= window.materialized_range.end);
    }

    #[test]
    fn virtual_range_still_returns_materialized_range_for_compatibility() {
        let cases = [
            request(100, 50.0, 40.0, 10.0, 2),
            request(100, 5000.0, 40.0, 10.0, 0),
            request(20, 10.0, 30.0, 10.0, 0),
            request(6, -200.0, 20.0, 10.0, usize::MAX),
        ];

        for request in cases {
            let range_request = VirtualRangeRequest {
                item_count: request.item_count,
                scroll_offset: request.scroll_offset,
                viewport_extent: request.viewport_extent,
                item_extent: request.item_extent,
                overscan: request.overscan,
            };
            assert_eq!(
                virtual_range(range_request),
                virtual_window(request).materialized_range
            );
        }
    }

    #[test]
    fn list_layout_adapter_exposes_strict_and_materialized_ranges() {
        let list = ListLayout::new(10.0);
        let window = list.virtual_window(100, 50.0, 35.0, 2);

        assert_window_finite(&window);
        assert_approx(window.content_extent, 1000.0);
        assert_approx(window.max_scroll_offset, 965.0);
        assert_approx(window.clamped_scroll_offset, 50.0);
        assert_range(window.visible_range, 5..9);
        assert_range(window.materialized_range.clone(), 3..12);
        assert_eq!(
            list.visible_range(100, 50.0, 35.0, 2),
            window.materialized_range
        );

        let rects = list.visible_row_rects(Rect::new(0.0, 0.0, 120.0, 35.0), 100, 50.0, 2);
        assert_eq!(rects.len(), 9);
        assert_eq!(rects[0].index, 3);
        assert_eq!(rects[8].index, 11);
        assert_approx(rects[0].rect.y, -20.0);
    }

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
}
