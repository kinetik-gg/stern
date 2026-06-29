//! Asset browser grid/list contract conformance tests.

mod asset_browser_conformance {
    use kinetik_ui_core::Size;
    use kinetik_ui_core::{ImageId, Rect, SemanticActionKind, SemanticRole, WidgetId};
    use kinetik_ui_widgets::{
        AssetBrowserItem, AssetBrowserLayout, AssetBrowserModel, AssetBrowserModelError,
        AssetBrowserSelectionOperation, AssetBrowserViewMode, AssetIconFallback, ComponentCategory,
        ComponentConformanceStatus, GridColumns, GridLayout, ItemId, ListLayout, Selection,
        asset_browser_item_widget_id, asset_browser_semantics, component_metadata,
    };

    fn id(raw: u64) -> ItemId {
        ItemId::from_raw(raw)
    }

    fn item(raw: u64, name: &str, kind: &str) -> AssetBrowserItem {
        AssetBrowserItem::new(id(raw), name, kind)
    }

    fn model() -> AssetBrowserModel {
        AssetBrowserModel::new(vec![
            item(10, "Studio HDRI", "image")
                .with_thumbnail(ImageId::from_raw(100))
                .with_tags(["hdr", "lighting"]),
            item(20, "Concrete", "material")
                .with_fallback(AssetIconFallback::new("material", "MAT"))
                .with_tags(["surface", "rough"]),
            item(30, "Camera Rig", "prefab").with_tags(["scene", "camera"]),
            item(40, "Locked", "scene").disabled(true),
            item(50, "Terrain", "mesh"),
        ])
    }

    fn layout(view_mode: AssetBrowserViewMode) -> AssetBrowserLayout {
        AssetBrowserLayout::new(
            view_mode,
            GridLayout {
                columns: GridColumns::Adaptive { min_width: 92.0 },
                item_size: Size::new(88.0, 74.0),
                gap: 6.0,
            },
            ListLayout::new(28.0),
        )
    }

    fn assert_rect_finite(rect: Rect) {
        assert!(rect.x.is_finite(), "{rect:?}");
        assert!(rect.y.is_finite(), "{rect:?}");
        assert!(rect.width.is_finite(), "{rect:?}");
        assert!(rect.height.is_finite(), "{rect:?}");
        assert!(rect.width >= 0.0, "{rect:?}");
        assert!(rect.height >= 0.0, "{rect:?}");
    }

    fn assert_close(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() < f32::EPSILON,
            "expected {actual} to equal {expected}"
        );
    }

    #[test]
    fn taxonomy_reports_asset_browser_as_partial_collection_contract() {
        let metadata = component_metadata("AssetBrowser").expect("asset browser metadata");

        assert_eq!(metadata.slug, "asset-browser");
        assert_eq!(metadata.category, ComponentCategory::Collection);
        assert_eq!(metadata.status, ComponentConformanceStatus::Partial);
    }

    #[test]
    fn grid_layout_resolves_deterministic_materialized_items() {
        let model = model();
        let bounds = Rect::new(10.0, 20.0, 300.0, 90.0);
        let mut selection = Selection::new();
        selection.replace(id(20));
        let layout = layout(AssetBrowserViewMode::Grid).with_overscan(1);

        let first = layout.resolve(bounds, &model, 0.0, &selection, Some(id(30)));
        let second = layout.resolve(bounds, &model, 0.0, &selection, Some(id(30)));

        assert_eq!(first, second);
        assert_eq!(first.view_mode, AssetBrowserViewMode::Grid);
        assert_eq!(first.columns, 3);
        assert_eq!(first.visible_range, 0..5);
        assert_eq!(first.materialized_range, 0..5);
        assert_eq!(
            first.materialized_item_ids(),
            vec![id(10), id(20), id(30), id(40), id(50)]
        );
        assert!(first.items[1].item.state.selected);
        assert!(first.items[2].item.state.hovered);
        assert!(first.items[3].item.state.disabled);
        for item in &first.items {
            assert_rect_finite(item.rect);
            assert_rect_finite(item.preview_rect);
            assert_rect_finite(item.name_rect);
            assert_rect_finite(item.kind_rect);
        }
    }

    #[test]
    fn adaptive_grid_columns_are_deterministic_and_finite() {
        let model = model();
        let selection = Selection::new();
        let layout = AssetBrowserLayout::new(
            AssetBrowserViewMode::Grid,
            GridLayout {
                columns: GridColumns::Adaptive {
                    min_width: f32::NAN,
                },
                item_size: Size::new(64.0, 64.0),
                gap: f32::NEG_INFINITY,
            },
            ListLayout::new(28.0),
        );

        let resolved = layout.resolve(
            Rect::new(f32::NAN, f32::INFINITY, f32::INFINITY, 130.0),
            &model,
            f32::INFINITY,
            &selection,
            None,
        );

        assert_eq!(resolved.columns, 1);
        assert_close(resolved.scroll_offset, 0.0);
        assert_eq!(resolved.materialized_range, 0..4);
        for item in &resolved.items {
            assert_rect_finite(item.rect);
        }
    }

    #[test]
    fn list_row_rectangles_are_stable() {
        let model = model();
        let selection = Selection::new();
        let layout = layout(AssetBrowserViewMode::List);

        let rows = layout.resolve(
            Rect::new(5.0, 7.0, 180.0, 56.0),
            &model,
            28.0,
            &selection,
            None,
        );

        assert_eq!(rows.view_mode, AssetBrowserViewMode::List);
        assert_eq!(rows.columns, 1);
        assert_eq!(rows.visible_range, 1..3);
        assert_eq!(rows.materialized_range, 1..4);
        assert_eq!(rows.items[0].item.id, id(20));
        assert_eq!(rows.items[0].rect, Rect::new(5.0, 7.0, 180.0, 28.0));
        assert_eq!(rows.items[1].rect, Rect::new(5.0, 35.0, 180.0, 28.0));
    }

    #[test]
    fn thumbnail_handle_and_fallback_metadata_are_preserved() {
        let model = model();
        let resolved = layout(AssetBrowserViewMode::Grid).resolve(
            Rect::new(0.0, 0.0, 300.0, 100.0),
            &model,
            0.0,
            &Selection::new(),
            None,
        );

        assert_eq!(
            resolved.items[0].item.thumbnail,
            Some(ImageId::from_raw(100))
        );
        assert_eq!(resolved.items[1].item.thumbnail, None);
        assert_eq!(
            resolved.items[1].item.fallback,
            AssetIconFallback::new("material", "MAT")
        );
    }

    #[test]
    fn multi_select_identity_is_stable_across_reorder() {
        let mut selection = Selection::new();
        selection.replace(id(20));
        selection.toggle(id(50));

        let reordered = AssetBrowserModel::new(vec![
            item(50, "Terrain", "mesh"),
            item(10, "Studio HDRI", "image"),
            item(20, "Concrete", "material"),
        ]);
        let resolved = layout(AssetBrowserViewMode::List).resolve(
            Rect::new(0.0, 0.0, 200.0, 90.0),
            &reordered,
            0.0,
            &selection,
            None,
        );

        assert_eq!(selection.selected(), vec![id(20), id(50)]);
        assert_eq!(
            resolved
                .items
                .iter()
                .filter(|item| item.item.state.selected)
                .map(|item| item.item.id)
                .collect::<Vec<_>>(),
            vec![id(50), id(20)]
        );
    }

    #[test]
    fn tags_are_exposed_for_later_filtering() {
        let model = model();
        let resolved = layout(AssetBrowserViewMode::Grid).resolve(
            Rect::new(0.0, 0.0, 300.0, 100.0),
            &model,
            0.0,
            &Selection::new(),
            None,
        );

        assert_eq!(
            model.item_ids(),
            vec![id(10), id(20), id(30), id(40), id(50)]
        );
        assert_eq!(
            resolved.items[0].item.tags,
            vec!["hdr".to_owned(), "lighting".to_owned()]
        );
        assert_eq!(
            model.item_by_id(id(20)).map(|item| item.tags.as_slice()),
            Some(["surface".to_owned(), "rough".to_owned()].as_slice())
        );
    }

    #[test]
    fn duplicate_asset_ids_are_diagnosed_deterministically() {
        let duplicate = AssetBrowserModel::new(vec![
            item(1, "First", "image"),
            item(1, "Second", "material"),
        ]);

        assert_eq!(
            duplicate.validate(),
            Err(AssetBrowserModelError::DuplicateItemId { id: id(1) })
        );
    }

    #[test]
    fn disabled_items_do_not_emit_selection_requests() {
        let resolved = layout(AssetBrowserViewMode::List).resolve(
            Rect::new(0.0, 0.0, 200.0, 160.0),
            &model(),
            0.0,
            &Selection::new(),
            None,
        );

        assert_eq!(
            resolved.items[0]
                .item
                .selection_request(AssetBrowserSelectionOperation::Replace)
                .map(|request| request.target),
            Some(id(10))
        );
        assert!(
            resolved.items[3]
                .item
                .selection_request(AssetBrowserSelectionOperation::Replace)
                .is_none()
        );
    }

    #[test]
    fn non_finite_layout_inputs_sanitize_deterministically() {
        let model = model();
        let invalid_grid = AssetBrowserLayout::new(
            AssetBrowserViewMode::Grid,
            GridLayout {
                columns: GridColumns::Fixed(2),
                item_size: Size::new(f32::NAN, 74.0),
                gap: 6.0,
            },
            ListLayout::new(28.0),
        );
        let empty = invalid_grid.resolve(
            Rect::new(0.0, 0.0, 300.0, 100.0),
            &model,
            0.0,
            &Selection::new(),
            None,
        );
        assert!(empty.items.is_empty());
        assert_eq!(empty.materialized_range, 0..0);

        let list = AssetBrowserLayout::new(
            AssetBrowserViewMode::List,
            GridLayout {
                columns: GridColumns::Fixed(1),
                item_size: Size::new(88.0, 74.0),
                gap: 6.0,
            },
            ListLayout::new(28.0),
        );
        let rows = list.resolve(
            Rect::new(f32::NAN, f32::NEG_INFINITY, f32::INFINITY, 60.0),
            &model,
            f32::INFINITY,
            &Selection::new(),
            None,
        );
        assert_eq!(rows.visible_range, 0..3);
        for item in &rows.items {
            assert_rect_finite(item.rect);
        }
    }

    #[test]
    fn semantics_preserve_view_mode_selection_and_disabled_state() {
        let mut selection = Selection::new();
        selection.replace(id(20));
        let resolved = layout(AssetBrowserViewMode::Grid).resolve(
            Rect::new(0.0, 0.0, 300.0, 100.0),
            &model(),
            0.0,
            &selection,
            None,
        );
        let root = WidgetId::from_key("assets");
        let semantics =
            asset_browser_semantics(root, Rect::new(0.0, 0.0, 300.0, 100.0), &resolved, "Assets");

        assert_eq!(semantics[0].role, SemanticRole::Grid);
        assert_eq!(
            semantics[0].children,
            resolved
                .items
                .iter()
                .map(|item| asset_browser_item_widget_id(root, item.item.id))
                .collect::<Vec<_>>()
        );

        let selected = semantics
            .iter()
            .find(|node| node.id == asset_browser_item_widget_id(root, id(20)))
            .expect("selected item semantics");
        assert!(selected.state.selected);
        assert!(
            selected
                .actions
                .iter()
                .any(|action| action.kind == SemanticActionKind::Invoke)
        );

        let disabled = semantics
            .iter()
            .find(|node| node.id == asset_browser_item_widget_id(root, id(40)))
            .expect("disabled item semantics");
        assert!(disabled.state.disabled);
        assert!(
            disabled
                .actions
                .iter()
                .all(|action| action.kind != SemanticActionKind::Invoke)
        );
    }
}
