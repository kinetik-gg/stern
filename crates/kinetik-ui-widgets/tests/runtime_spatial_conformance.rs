//! Widget runtime spatial and virtualization conformance coverage.

use kinetik_ui_core::{
    PlatformRequest, Point, PointerButtonState, PointerInput, Rect, Size, UiInput, UiMemory, Vec2,
    WidgetId, default_dark_theme, inspect_primitives,
};
use kinetik_ui_text::TextEditState;
use kinetik_ui_widgets::{
    ItemId, NumericScrubInputConfig, PathFieldConfig, PropertyGridLayout, PropertyGridRow,
    TreeLayout, TreeRow, Ui, VectorComponentLayout, VectorScrubInputConfig,
    vector3_component_rects,
};

fn assert_close(actual: f32, expected: f32) {
    assert!((actual - expected).abs() < 1.0e-4, "{actual} != {expected}");
}

#[test]
fn nested_scroll_scopes_align_paint_hit_semantics_and_local_response_geometry() {
    let root = WidgetId::from_key("root");
    let outer_id = root.child("outer");
    let outer_scope = root.child(("scroll_area_content", outer_id.raw()));
    let inner_id = outer_scope.child("inner");
    let mut memory = UiMemory::new();
    memory.set_scroll_offset(outer_id, Vec2::new(10.0, 20.0));
    memory.set_scroll_offset(inner_id, Vec2::new(5.0, 7.0));
    let input = UiInput {
        pointer: PointerInput {
            position: Some(Point::new(16.0, 34.0)),
            primary: PointerButtonState::new(false, true, true),
            ..PointerInput::default()
        },
        ..UiInput::default()
    };
    let theme = default_dark_theme();
    let outer_rect = Rect::new(0.0, 0.0, 100.0, 80.0);
    let inner_rect = Rect::new(20.0, 40.0, 60.0, 50.0);
    let child_rect = Rect::new(30.0, 60.0, 20.0, 20.0);

    let mut ui = Ui::new(&input, &mut memory, &theme);
    let nested = ui.scroll_area(
        "outer",
        outer_rect,
        Size::new(200.0, 200.0),
        false,
        |ui, outer_offset| {
            assert_eq!(outer_offset, Vec2::new(10.0, 20.0));
            ui.scroll_area(
                "inner",
                inner_rect,
                Size::new(120.0, 120.0),
                false,
                |ui, inner_offset| {
                    assert_eq!(inner_offset, Vec2::new(5.0, 7.0));
                    ui.button("child", child_rect, "Nested child", false)
                },
            )
        },
    );
    let output = ui.finish_output();
    let response = nested.inner.inner;

    assert!(response.clicked);
    assert_eq!(response.rect, child_rect);
    assert!(output.warnings.is_empty());
    let expected_screen = Rect::new(15.0, 33.0, 20.0, 20.0);
    let semantic = output
        .semantics
        .nodes()
        .iter()
        .find(|node| node.label.as_deref() == Some("Nested child"))
        .expect("nested button semantic node");
    assert_eq!(semantic.bounds, expected_screen);
    assert!(
        inspect_primitives(&output.primitives)
            .iter()
            .any(|row| row.bounds == Some(expected_screen))
    );
}

#[test]
fn runtime_scroll_clip_makes_offscreen_child_inert_and_non_focusable() {
    let root = WidgetId::from_key("root");
    let scroll_id = root.child("scroll");
    let mut memory = UiMemory::new();
    memory.set_scroll_offset(scroll_id, Vec2::new(0.0, 50.0));
    let input = UiInput {
        pointer: PointerInput {
            position: Some(Point::new(5.0, 5.0)),
            primary: PointerButtonState::new(false, true, true),
            ..PointerInput::default()
        },
        ..UiInput::default()
    };
    let theme = default_dark_theme();
    let mut ui = Ui::new(&input, &mut memory, &theme);

    let output = ui.scroll_area(
        "scroll",
        Rect::new(0.0, 0.0, 40.0, 40.0),
        Size::new(40.0, 100.0),
        false,
        |ui, offset| {
            assert_eq!(offset, Vec2::new(0.0, 50.0));
            ui.button(
                "offscreen",
                Rect::new(0.0, 0.0, 20.0, 20.0),
                "Offscreen",
                false,
            )
        },
    );
    let frame = ui.finish_output();

    assert!(!output.inner.clicked);
    let semantic = frame
        .semantics
        .nodes()
        .iter()
        .find(|node| node.label.as_deref() == Some("Offscreen"))
        .expect("retained semantic node");
    assert_eq!(semantic.bounds, Rect::ZERO);
    assert!(!semantic.focusable);
    assert!(!frame.semantics.focus_order().contains(&semantic.id));
}

#[test]
fn nested_scroll_projects_focused_text_semantics_and_ime_to_one_screen_rect() {
    let root = WidgetId::from_key("root");
    let outer_id = root.child("focus-outer");
    let outer_scope = root.child(("scroll_area_content", outer_id.raw()));
    let inner_id = outer_scope.child("focus-inner");
    let inner_scope = outer_scope.child(("scroll_area_content", inner_id.raw()));
    let field_id = inner_scope.child("field");
    let mut memory = UiMemory::new();
    memory.set_scroll_offset(outer_id, Vec2::new(10.0, 20.0));
    memory.set_scroll_offset(inner_id, Vec2::new(5.0, 7.0));
    memory.focus(field_id);
    let theme = default_dark_theme();
    let mut state = TextEditState::new("Nested text");
    state.set_caret(0);
    let child_rect = Rect::new(30.0, 60.0, 20.0, 20.0);
    let expected_screen = Rect::new(15.0, 33.0, 20.0, 20.0);
    let expected_caret_screen = Rect::new(19.0, 37.0, 1.0, 12.0);
    let input = UiInput::default();
    let mut ui = Ui::new(&input, &mut memory, &theme);

    let response = ui.scroll_area(
        "focus-outer",
        Rect::new(0.0, 0.0, 100.0, 80.0),
        Size::new(200.0, 200.0),
        false,
        |ui, _| {
            ui.scroll_area(
                "focus-inner",
                Rect::new(20.0, 40.0, 60.0, 50.0),
                Size::new(120.0, 120.0),
                false,
                |ui, _| ui.text_field("field", child_rect, &mut state, false),
            )
        },
    );
    let frame = ui.finish_output();

    assert_eq!(
        response.inner.inner.widget.response.expect("response").rect,
        child_rect
    );
    assert_eq!(memory.focused(), Some(field_id));
    assert_eq!(memory.text_input_owner(), Some(field_id));
    assert_eq!(
        frame
            .semantics
            .get(field_id)
            .expect("field semantic")
            .bounds,
        expected_screen
    );
    assert!(
        frame
            .semantics
            .get(field_id)
            .expect("field semantic")
            .state
            .focused
    );
    let ime_rect = frame
        .platform_requests
        .iter()
        .find_map(|request| match request {
            PlatformRequest::StartTextInput { rect: Some(rect) } => Some(*rect),
            _ => None,
        });
    assert_eq!(ime_rect, Some(expected_caret_screen));
}

#[test]
fn nested_scroll_projects_vector_component_caret_once_and_clips_it() {
    let root = WidgetId::from_key("root");
    let scroll_id = root.child("wrapper-scroll");
    let offset = Vec2::new(10.0, 20.0);
    let outer_rect = Rect::new(0.0, 0.0, 160.0, 50.0);
    let vector_rect = Rect::new(20.0, 30.0, 220.0, 24.0);
    let component_rects = vector3_component_rects(vector_rect, VectorComponentLayout::default());
    let local_target = component_rects[0].value_rect.center();
    let screen_target = Point::new(local_target.x - offset.x, local_target.y - offset.y);
    let input = UiInput {
        pointer: PointerInput {
            position: Some(screen_target),
            primary: PointerButtonState::new(false, true, true),
            ..PointerInput::default()
        },
        ..UiInput::default()
    };
    let mut memory = UiMemory::new();
    memory.set_scroll_offset(scroll_id, offset);
    let theme = default_dark_theme();
    let mut values = [1.0, 2.0, 3.0];
    let mut states = [
        TextEditState::new("1"),
        TextEditState::new("2"),
        TextEditState::new("3"),
    ];
    let mut ui = Ui::new(&input, &mut memory, &theme);

    let vector = ui.scroll_area(
        "wrapper-scroll",
        outer_rect,
        Size::new(260.0, 100.0),
        false,
        |ui, retained| {
            assert_eq!(retained, offset);
            ui.vector3_scrub_input(
                "vector",
                vector_rect,
                "Position",
                &mut values,
                &mut states,
                VectorScrubInputConfig::new(NumericScrubInputConfig::default()),
            )
        },
    );
    let frame = ui.finish_output();

    assert!(!vector.inner.components[0].scrubbed);
    let caret = frame
        .platform_requests
        .iter()
        .find_map(|request| match request {
            PlatformRequest::StartTextInput { rect: Some(rect) } => Some(*rect),
            _ => None,
        })
        .expect("nested vector component starts IME");
    let projected_field = Rect::new(
        component_rects[0].value_rect.x - offset.x,
        component_rects[0].value_rect.y - offset.y,
        component_rects[0].value_rect.width,
        component_rects[0].value_rect.height,
    );
    assert_close(caret.width, 1.0);
    assert_ne!(caret, projected_field);
    assert!(outer_rect.contains_point(caret.center()));
    assert!(projected_field.contains_point(caret.center()));
    assert!(frame.warnings.is_empty());
}

#[test]
fn nested_scroll_projects_search_numeric_and_path_child_carets_once() {
    let root = WidgetId::from_key("root");
    let outer_rect = Rect::new(0.0, 0.0, 100.0, 40.0);
    let field_rect = Rect::new(20.0, 20.0, 140.0, 24.0);
    let offset = Vec2::new(10.0, 8.0);
    let theme = default_dark_theme();

    for kind in ["search", "numeric", "path"] {
        let scroll_key = format!("{kind}-scroll");
        let scroll_id = root.child(&scroll_key);
        let scope = root.child(("scroll_area_content", scroll_id.raw()));
        let field_id = if kind == "path" {
            scope.child(kind).child("text")
        } else {
            scope.child(kind)
        };
        let mut memory = UiMemory::new();
        memory.set_scroll_offset(scroll_id, offset);
        memory.focus(field_id);
        let input = UiInput::default();
        let mut state = TextEditState::new("long wrapper text");
        state.set_caret(0);
        let mut ui = Ui::new(&input, &mut memory, &theme);
        ui.scroll_area(
            &scroll_key,
            outer_rect,
            Size::new(180.0, 80.0),
            false,
            |ui, retained| {
                assert_eq!(retained, offset);
                match kind {
                    "search" => {
                        let _ = ui.search_field(kind, field_rect, &mut state, false);
                    }
                    "numeric" => {
                        let _ = ui.numeric_input(kind, field_rect, &mut state, false);
                    }
                    "path" => {
                        let _ = ui.path_field(
                            kind,
                            field_rect,
                            "Source",
                            &mut state,
                            PathFieldConfig::new().browse(false),
                        );
                    }
                    _ => unreachable!(),
                }
            },
        );
        let frame = ui.finish_output();
        assert_eq!(memory.focused(), Some(field_id));
        let carets = frame
            .platform_requests
            .iter()
            .filter_map(|request| match request {
                PlatformRequest::StartTextInput { rect: Some(rect) } => Some(*rect),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(carets.len(), 1, "{kind}");
        let caret = carets[0];
        let projected_field = Rect::new(
            field_rect.x - offset.x,
            field_rect.y - offset.y,
            field_rect.width,
            field_rect.height,
        );
        assert_close(caret.width, 1.0);
        assert_ne!(caret, projected_field);
        assert!(outer_rect.contains_point(caret.center()));
        assert!(projected_field.contains_point(caret.center()));
        assert!(frame.warnings.is_empty());
    }
}

#[test]
fn runtime_scroll_emission_uses_one_offset_for_fractional_overscan_and_mixed_rows() {
    let root = WidgetId::from_key("root");
    let tree_scroll_id = root.child("tree-runtime");
    let inspector_scroll_id = root.child("inspector-runtime");
    let mut memory = UiMemory::new();
    memory.set_scroll_offset(tree_scroll_id, Vec2::new(0.0, 25.5));
    memory.set_scroll_offset(inspector_scroll_id, Vec2::new(0.0, 999.0));
    let theme = default_dark_theme();
    let tree_bounds = Rect::new(0.0, 0.0, 100.0, 25.0);
    let tree_rows = (0_usize..10)
        .map(|row| TreeRow {
            row,
            item_index: row,
            id: ItemId::from_raw(u64::try_from(row).expect("small row index") + 100),
            parent: None,
            depth: row % 2,
            has_children: false,
            expanded: false,
        })
        .collect::<Vec<_>>();
    let tree = TreeLayout::new(10.0, 8.0);
    let inspector_bounds = Rect::new(120.0, 0.0, 100.0, 30.0);
    let inspector_rows = vec![
        PropertyGridRow::section(ItemId::from_raw(200), "A"),
        PropertyGridRow::property(ItemId::from_raw(201), "A1", 0),
        PropertyGridRow::property(ItemId::from_raw(202), "A2", 0),
        PropertyGridRow::property(ItemId::from_raw(203), "A3", 0),
        PropertyGridRow::property(ItemId::from_raw(204), "A4", 0),
        PropertyGridRow::section(ItemId::from_raw(205), "B"),
        PropertyGridRow::property(ItemId::from_raw(206), "B1", 0),
        PropertyGridRow::property(ItemId::from_raw(207), "B2", 0),
        PropertyGridRow::property(ItemId::from_raw(208), "B3", 0),
        PropertyGridRow::property(ItemId::from_raw(209), "B4", 0),
    ];
    let inspector = PropertyGridLayout::new(10.0, 15.0, 45.0, 4.0, 8.0);
    let input = UiInput::default();
    let mut ui = Ui::new(&input, &mut memory, &theme);

    let tree_output = ui.scroll_area(
        "tree-runtime",
        tree_bounds,
        Size::new(tree_bounds.width, tree.content_height(tree_rows.len())),
        false,
        |ui, offset| {
            let rows = tree.visible_row_rects_content(tree_bounds, &tree_rows, offset.y, 1);
            for row in &rows {
                ui.panel_keyed(("tree-emitted", row.row.row), row.rect);
            }
            rows
        },
    );
    let inspector_output = ui.scroll_area(
        "inspector-runtime",
        inspector_bounds,
        Size::new(
            inspector_bounds.width,
            inspector.content_height(&inspector_rows),
        ),
        false,
        |ui, offset| {
            let rows =
                inspector.visible_row_rects_content(inspector_bounds, &inspector_rows, offset.y, 1);
            for row in &rows {
                ui.panel_keyed(("inspector-emitted", row.index), row.rect);
            }
            rows
        },
    );
    let frame = ui.finish_output();
    let inspected = inspect_primitives(&frame.primitives);

    assert_close(tree_output.scroll.offset.y, 25.5);
    assert!(tree_output.inner.first().expect("tree rows").row.row < 2);
    assert!(inspected.iter().any(|row| {
        row.bounds
            .is_some_and(|bounds| bounds == Rect::new(0.0, 4.5, 100.0, 10.0))
    }));
    assert_close(inspector_output.scroll.offset.y, 80.0);
    assert!(inspector_output.inner.iter().any(|row| row.index == 9));
    assert!(inspected.iter().any(|row| {
        row.bounds
            .is_some_and(|bounds| bounds == Rect::new(120.0, 20.0, 100.0, 10.0))
    }));

    let range_rows = &tree_rows[1..8];
    let viewport = tree.visible_row_rects_in_range(tree_bounds, 10, range_rows, 25.5, 1);
    let content = tree.visible_row_rects_in_range_content(tree_bounds, 10, range_rows, 25.5, 1);
    assert_eq!(viewport.len(), content.len());
    for (viewport, content) in viewport.iter().zip(&content) {
        assert_eq!(viewport.row.row, content.row.row);
        assert_close(viewport.rect.y, content.rect.y - 25.5);
    }
}

#[test]
fn tree_and_inspector_content_materialization_uses_scroll_only_for_range_selection() {
    let bounds = Rect::new(10.0, 100.0, 120.0, 20.0);
    let tree_rows = (0_usize..10)
        .map(|row| TreeRow {
            row,
            item_index: row,
            id: ItemId::from_raw(u64::try_from(row).expect("small row index") + 1),
            parent: None,
            depth: 0,
            has_children: false,
            expanded: false,
        })
        .collect::<Vec<_>>();
    let tree = TreeLayout::new(10.0, 12.0);
    let viewport_tree = tree.visible_row_rects(bounds, &tree_rows, 30.0, 0);
    let content_tree = tree.visible_row_rects_content(bounds, &tree_rows, 30.0, 0);

    assert_eq!(viewport_tree[0].row.row, 3);
    assert_close(viewport_tree[0].rect.y, 100.0);
    assert_eq!(content_tree[0].row.row, 3);
    assert_close(content_tree[0].rect.y, 130.0);
    assert_close(
        content_tree[0].rect.translate(Vec2::new(0.0, -30.0)).y,
        100.0,
    );

    let inspector_rows = (0_usize..10)
        .map(|row| {
            PropertyGridRow::property(
                ItemId::from_raw(u64::try_from(row).expect("small row index") + 20),
                format!("Row {row}"),
                0,
            )
        })
        .collect::<Vec<_>>();
    let inspector = PropertyGridLayout::new(10.0, 10.0, 50.0, 4.0, 8.0);
    let viewport_inspector = inspector.visible_row_rects(bounds, &inspector_rows, 30.0, 0);
    let content_inspector = inspector.visible_row_rects_content(bounds, &inspector_rows, 30.0, 0);

    assert_eq!(viewport_inspector[0].index, 3);
    assert_close(viewport_inspector[0].rect.y, 100.0);
    assert_eq!(content_inspector[0].index, 3);
    assert_close(content_inspector[0].rect.y, 130.0);
    assert_close(
        content_inspector[0].rect.translate(Vec2::new(0.0, -30.0)).y,
        100.0,
    );
}
