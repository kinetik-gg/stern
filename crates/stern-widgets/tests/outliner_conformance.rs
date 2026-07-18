//! Outliner tree behavior conformance tests.

mod outliner_conformance {
    use stern_core::{Point, Rect, SemanticActionKind, SemanticRole, SemanticValue, WidgetId};
    use stern_widgets::{
        ItemId, OutlinerLayout, OutlinerModel, OutlinerResourceMetadata, OutlinerRowFlags,
        OutlinerRowZoneKind, OutlinerSelectionOperation, OutlinerVisibilityToggleRequest,
        Selection, TreeExpansion, TreeModelError, outliner_row_widget_id, outliner_semantics,
    };

    fn id(raw: u64) -> ItemId {
        ItemId::from_raw(raw)
    }

    fn item(raw: u64, parent: Option<u64>, label: &str) -> stern_widgets::OutlinerItem {
        let item = stern_widgets::OutlinerItem::new(id(raw), label);
        if let Some(parent) = parent {
            item.with_parent(id(parent))
        } else {
            item
        }
    }

    fn model() -> OutlinerModel {
        OutlinerModel::new(vec![
            item(10, None, "World"),
            item(30, Some(10), "Camera"),
            item(20, Some(10), "Light"),
            item(40, Some(20), "Shadow"),
            item(50, None, "Materials")
                .with_icon(stern_icons_phosphor::regular::CUBE)
                .with_resource(OutlinerResourceMetadata::new("library", "materials")),
        ])
    }

    fn expanded_model_rows() -> Vec<stern_widgets::OutlinerRow> {
        let mut expansion = TreeExpansion::new();
        expansion.expand(id(10));
        expansion.expand(id(20));
        model().visible_rows(&expansion)
    }

    fn assert_rect_finite(rect: Rect) {
        assert!(rect.x.is_finite(), "{rect:?}");
        assert!(rect.y.is_finite(), "{rect:?}");
        assert!(rect.width.is_finite(), "{rect:?}");
        assert!(rect.height.is_finite(), "{rect:?}");
        assert!(rect.width >= 0.0, "{rect:?}");
        assert!(rect.height >= 0.0, "{rect:?}");
    }

    fn assert_row_zones_finite(zones: &stern_widgets::OutlinerRowZones) {
        assert_rect_finite(zones.rect);
        assert_rect_finite(zones.disclosure_rect);
        assert_rect_finite(zones.visibility_toggle_rect);
        assert_rect_finite(zones.lock_toggle_rect);
        assert_rect_finite(zones.label_rect);
        assert_rect_finite(zones.context_rect);
    }

    #[test]
    fn tree_visible_row_order_is_deterministic() {
        let mut expansion = TreeExpansion::new();
        expansion.expand(id(10));

        let rows = model().visible_rows(&expansion);

        assert_eq!(
            rows.iter()
                .map(|row| (row.id, row.parent, row.depth, row.label.as_str()))
                .collect::<Vec<_>>(),
            vec![
                (id(10), None, 0, "World"),
                (id(30), Some(id(10)), 1, "Camera"),
                (id(20), Some(id(10)), 1, "Light"),
                (id(50), None, 0, "Materials"),
            ]
        );
        assert!(rows[0].has_children);
        assert!(rows[0].expanded);
        assert!(!rows[1].has_children);
        assert_eq!(
            rows[3].icon,
            Some(stern_icons_phosphor::regular::CUBE.icon())
        );
        assert_eq!(
            rows[3]
                .resource
                .as_ref()
                .map(|resource| resource.kind.as_str()),
            Some("library")
        );
    }

    #[test]
    fn expansion_preservation_is_deterministic() {
        let model = model();
        let mut expansion = TreeExpansion::new();
        expansion.expand(id(20));
        expansion.expand(id(10));
        expansion.expand(id(999));

        let preserved = model.preserved_expansion(&expansion);
        assert_eq!(preserved.expanded(), vec![id(10), id(20)]);

        let preserved_again = model.preserved_expansion(&preserved);
        assert_eq!(preserved_again, preserved);
    }

    #[test]
    fn row_zone_rectangles_are_stable_and_finite() {
        let rows = expanded_model_rows();
        let layout = OutlinerLayout::new(20.0, 12.0);
        let first = layout.visible_row_zones(Rect::new(10.0, 30.0, 220.0, 80.0), &rows, 0.0, 0);
        let second = layout.visible_row_zones(Rect::new(10.0, 30.0, 220.0, 80.0), &rows, 0.0, 0);

        assert_eq!(first, second);
        assert_eq!(
            first.iter().map(|zones| zones.row.id).collect::<Vec<_>>(),
            vec![id(10), id(30), id(20), id(40), id(50)]
        );
        for zones in &first {
            assert_row_zones_finite(zones);
            assert_eq!(
                zones.hit_zone(Point::new(
                    zones.disclosure_rect.x + 1.0,
                    zones.disclosure_rect.y + 1.0,
                )),
                Some(OutlinerRowZoneKind::Disclosure)
            );
        }
        assert!(first[1].label_rect.x > first[0].label_rect.x);
    }

    #[test]
    fn outliner_range_virtualization_matches_full_row_layout_window() {
        let model = model();
        let mut expansion = TreeExpansion::new();
        expansion.expand(id(10));
        expansion.expand(id(20));
        let rows = model.visible_rows(&expansion);
        let layout = OutlinerLayout::new(20.0, 12.0);
        let bounds = Rect::new(10.0, 30.0, 220.0, 40.0);

        let full = layout.visible_row_zones(bounds, &rows, 24.0, 0);
        let range = layout.visible_model_row_zones(bounds, &model, &expansion, 24.0, 0);

        assert_eq!(range, full);
        assert_eq!(
            range.iter().map(|zones| zones.row.id).collect::<Vec<_>>(),
            vec![id(30), id(20), id(40)]
        );
    }

    #[test]
    fn outliner_range_virtualization_preserves_global_indices_and_stable_ids() {
        let model = model();
        let mut expansion = TreeExpansion::new();
        expansion.expand(id(10));
        expansion.expand(id(20));

        let rows = model.visible_rows_in_range(&expansion, 2..5);

        assert_eq!(
            rows.iter()
                .map(|row| (row.row, row.id, row.item_index, row.label.as_str()))
                .collect::<Vec<_>>(),
            vec![
                (2, id(20), 2, "Light"),
                (3, id(40), 3, "Shadow"),
                (4, id(50), 4, "Materials")
            ]
        );
    }

    #[test]
    fn outliner_range_virtualization_invalid_models_return_empty_zones() {
        let duplicate = OutlinerModel::new(vec![item(1, None, "A"), item(1, None, "B")]);
        let layout = OutlinerLayout::new(20.0, 12.0);

        assert!(
            duplicate
                .visible_rows_in_range(&TreeExpansion::new(), 0..2)
                .is_empty()
        );
        assert!(
            layout
                .visible_model_row_zones(
                    Rect::new(0.0, 0.0, 240.0, 80.0),
                    &duplicate,
                    &TreeExpansion::new(),
                    0.0,
                    0,
                )
                .is_empty()
        );
        assert_eq!(
            duplicate.validate(),
            Err(TreeModelError::DuplicateItemId { id: id(1) })
        );
    }

    #[test]
    fn toggle_requests_preserve_target_ids() {
        let mut flags = OutlinerRowFlags::new();
        flags.visible = false;
        flags.locked = true;
        let model = OutlinerModel::new(vec![
            item(7, None, "Layer").with_flags(flags),
            item(8, None, "Lazy parent").with_has_children(true),
        ]);
        let rows = model.visible_rows(&TreeExpansion::new());
        let row = &rows[0];

        assert_eq!(
            row.selection_request(OutlinerSelectionOperation::Replace)
                .map(|request| request.target),
            Some(id(7))
        );
        assert_eq!(
            row.visibility_toggle_request(),
            Some(OutlinerVisibilityToggleRequest {
                target: id(7),
                visible: false,
            })
        );
        assert_eq!(
            row.lock_toggle_request().map(|request| request.target),
            Some(id(7))
        );
        assert_eq!(
            row.expansion_request(true),
            None,
            "leaf rows do not request expansion"
        );
        assert_eq!(
            rows[1].expand_request().map(|request| request.target),
            Some(id(8))
        );
        assert_eq!(
            rows[1]
                .collapse_request()
                .map(|request| (request.target, request.expanded)),
            Some((id(8), false))
        );
    }

    #[test]
    fn disabled_and_read_only_rows_suppress_unavailable_requests() {
        let mut disabled = OutlinerRowFlags::new();
        disabled.disabled = true;
        let mut read_only = OutlinerRowFlags::new();
        read_only.read_only = true;
        let model = OutlinerModel::new(vec![
            item(1, None, "Disabled").with_flags(disabled),
            item(2, None, "Read only").with_flags(read_only),
        ]);
        let rows = model.visible_rows(&TreeExpansion::new());

        assert!(
            rows[0]
                .selection_request(OutlinerSelectionOperation::Replace)
                .is_none()
        );
        assert!(rows[0].visibility_toggle_request().is_none());
        assert!(rows[0].lock_toggle_request().is_none());

        assert!(
            rows[1]
                .selection_request(OutlinerSelectionOperation::Replace)
                .is_some()
        );
        assert!(rows[1].visibility_toggle_request().is_none());
        assert!(rows[1].lock_toggle_request().is_none());
    }

    #[test]
    fn selection_by_id_survives_reorder() {
        let mut selection = Selection::new();
        selection.replace(id(20));

        let reordered = OutlinerModel::new(vec![
            item(50, None, "Materials"),
            item(10, None, "World"),
            item(20, Some(10), "Light"),
            item(30, Some(10), "Camera"),
        ]);
        let mut expansion = TreeExpansion::new();
        expansion.expand(id(10));
        let rows = reordered.visible_rows(&expansion);

        assert_eq!(
            rows.iter().map(|row| row.id).collect::<Vec<_>>(),
            vec![id(50), id(10), id(20), id(30)]
        );
        assert!(selection.contains(id(20)));
        assert_eq!(selection.selected(), vec![id(20)]);
    }

    #[test]
    fn semantics_and_list_metadata_are_stable() {
        let rows = expanded_model_rows();
        let zones = OutlinerLayout::new(20.0, 12.0).visible_row_zones(
            Rect::new(0.0, 0.0, 240.0, 100.0),
            &rows,
            0.0,
            0,
        );
        let mut selection = Selection::new();
        selection.replace(id(20));
        let root = WidgetId::from_key("outliner");

        let semantics = outliner_semantics(
            root,
            Rect::new(0.0, 0.0, 240.0, 100.0),
            &zones,
            &selection,
            "Scene",
        );

        assert_eq!(semantics[0].role, SemanticRole::List);
        assert_eq!(semantics[0].label.as_deref(), Some("Scene"));
        assert_eq!(
            semantics[0].state.value,
            Some(SemanticValue::Text("5 rows".to_owned()))
        );
        assert_eq!(
            semantics[0].children,
            zones
                .iter()
                .map(|zones| outliner_row_widget_id(root, zones.row.id))
                .collect::<Vec<_>>()
        );

        let selected = semantics
            .iter()
            .find(|node| node.id == outliner_row_widget_id(root, id(20)))
            .expect("selected row semantics");
        assert_eq!(selected.role, SemanticRole::ListItem);
        assert_eq!(selected.label.as_deref(), Some("Light"));
        assert!(selected.state.selected);
        assert_eq!(selected.state.expanded, Some(true));
        assert!(
            selected
                .actions
                .iter()
                .any(|action| action.kind == SemanticActionKind::Close)
        );
    }

    #[test]
    fn malformed_hierarchies_are_diagnosed_deterministically() {
        let duplicate = OutlinerModel::new(vec![item(1, None, "A"), item(1, None, "B")]);
        assert_eq!(
            duplicate.validate(),
            Err(TreeModelError::DuplicateItemId { id: id(1) })
        );
        assert!(duplicate.visible_rows(&TreeExpansion::new()).is_empty());

        let missing_parent = OutlinerModel::new(vec![item(2, Some(99), "Orphan")]);
        assert_eq!(
            missing_parent.validate(),
            Err(TreeModelError::UnknownParent {
                id: id(2),
                parent: id(99),
            })
        );
        assert!(
            missing_parent
                .visible_rows(&TreeExpansion::new())
                .is_empty()
        );

        let cycle = OutlinerModel::new(vec![item(1, Some(2), "A"), item(2, Some(1), "B")]);
        assert_eq!(cycle.validate(), Err(TreeModelError::Cycle { id: id(1) }));
        assert!(cycle.visible_rows(&TreeExpansion::new()).is_empty());
    }

    #[test]
    fn invalid_layout_inputs_are_deterministic_and_finite() {
        let rows = expanded_model_rows();
        let invalid_height = OutlinerLayout::new(f32::NAN, 12.0);
        assert!(
            invalid_height
                .visible_row_zones(Rect::new(0.0, 0.0, 100.0, 100.0), &rows, 0.0, 0)
                .is_empty()
        );

        let mut layout = OutlinerLayout::new(20.0, f32::NAN);
        layout.disclosure_width = f32::INFINITY;
        layout.visibility_toggle_width = f32::NAN;
        layout.lock_toggle_width = -10.0;
        layout.gap = f32::NEG_INFINITY;
        let zones = layout.visible_row_zones(
            Rect::new(f32::NAN, f32::INFINITY, f32::INFINITY, 40.0),
            &rows,
            f32::NEG_INFINITY,
            usize::MAX,
        );

        assert_eq!(zones.len(), rows.len());
        for row in &zones {
            assert_row_zones_finite(row);
        }
    }
}
