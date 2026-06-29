//! Windowless drag/drop and context-action conformance for collection widgets.

use kinetik_ui_core::{ActionDescriptor, ActionId, Point, Rect, Size};
use kinetik_ui_widgets::{
    AssetBrowserDropTargetKind, AssetBrowserItem, AssetBrowserLayout, AssetBrowserModel,
    AssetBrowserViewMode, CollectionContextTarget, GridColumns, GridLayout, ItemId, ListLayout,
    OutlinerDropZoneKind, OutlinerItem, OutlinerLayout, OutlinerModel, OutlinerRowFlags, Selection,
    TreeExpansion, collection_context_actions, outliner_context_target_at,
};

fn id(raw: u64) -> ItemId {
    ItemId::from_raw(raw)
}

fn outliner_rows(disabled_target: bool) -> Vec<kinetik_ui_widgets::OutlinerRowZones> {
    let mut disabled = OutlinerRowFlags::new();
    disabled.disabled = disabled_target;
    let rows = OutlinerModel::new(vec![
        OutlinerItem::new(id(10), "World"),
        OutlinerItem::new(id(20), "Camera"),
        OutlinerItem::new(id(30), "Disabled").with_flags(disabled),
    ])
    .visible_rows(&TreeExpansion::new());

    OutlinerLayout::new(20.0, 12.0).visible_row_zones(
        Rect::new(0.0, 0.0, 240.0, 120.0),
        &rows,
        0.0,
        0,
    )
}

fn asset_layout() -> AssetBrowserLayout {
    AssetBrowserLayout::new(
        AssetBrowserViewMode::Grid,
        GridLayout {
            columns: GridColumns::Adaptive { min_width: 92.0 },
            item_size: Size::new(88.0, 74.0),
            gap: 6.0,
        },
        ListLayout::new(28.0),
    )
}

fn asset_model() -> AssetBrowserModel {
    AssetBrowserModel::new(vec![
        AssetBrowserItem::new(id(10), "Studio HDRI", "image"),
        AssetBrowserItem::new(id(20), "Concrete", "material"),
        AssetBrowserItem::new(id(30), "Camera Rig", "prefab"),
        AssetBrowserItem::new(id(40), "Locked", "scene").disabled(true),
        AssetBrowserItem::new(id(50), "Terrain", "mesh"),
    ])
}

fn action(id: &str, label: &str) -> ActionDescriptor {
    ActionDescriptor::new(id, label)
}

#[test]
fn drag_source_identity_is_stable_and_selection_aware() {
    let rows = outliner_rows(false);
    let mut selection = Selection::new();
    selection.replace(id(20));
    selection.toggle(id(10));

    let selected_drag = rows[1]
        .row
        .drag_source(&selection)
        .expect("selected drag source");
    assert_eq!(selected_drag.source, id(20));
    assert_eq!(selected_drag.items, vec![id(10), id(20)]);

    let unselected_drag = rows[2]
        .row
        .drag_source(&selection)
        .expect("unselected drag source");
    assert_eq!(unselected_drag.source, id(30));
    assert_eq!(unselected_drag.items, vec![id(30)]);
}

#[test]
fn outliner_drop_zone_calculation_is_deterministic_and_rejects_self_drop() {
    let rows = outliner_rows(false);
    let source = rows[0]
        .row
        .drag_source(&Selection::new())
        .expect("source drag");
    let target = &rows[1];
    let x = target.rect.x + 4.0;

    assert_eq!(
        target
            .drop_target(Point::new(x, target.rect.y + 1.0), &source)
            .expect("before drop")
            .zone,
        OutlinerDropZoneKind::Before
    );
    assert_eq!(
        target
            .drop_target(Point::new(x, target.rect.y + 10.0), &source)
            .expect("inside drop")
            .zone,
        OutlinerDropZoneKind::Inside
    );
    assert_eq!(
        target
            .drop_target(Point::new(x, target.rect.y + 19.0), &source)
            .expect("after drop")
            .zone,
        OutlinerDropZoneKind::After
    );

    let self_source = rows[1]
        .row
        .drag_source(&Selection::new())
        .expect("self drag");
    assert!(
        target
            .drop_target(Point::new(x, target.rect.y + 10.0), &self_source)
            .is_none()
    );
}

#[test]
fn disabled_outliner_targets_are_suppressed() {
    let rows = outliner_rows(true);
    let source = rows[0]
        .row
        .drag_source(&Selection::new())
        .expect("source drag");
    let disabled = &rows[2];
    let point = Point::new(disabled.rect.x + 4.0, disabled.rect.y + 10.0);

    assert!(disabled.row.drag_source(&Selection::new()).is_none());
    assert!(disabled.drop_target(point, &source).is_none());
    assert!(disabled.row.context_target(&Selection::new()).is_none());
    assert!(
        outliner_context_target_at(
            Rect::new(0.0, 0.0, 240.0, 120.0),
            &rows,
            point,
            &Selection::new()
        )
        .is_none()
    );
}

#[test]
fn asset_browser_resolves_item_and_empty_space_drop_targets() {
    let bounds = Rect::new(0.0, 0.0, 300.0, 160.0);
    let result = asset_layout().resolve(bounds, &asset_model(), 0.0, &Selection::new(), None);
    let source = result.items[0]
        .item
        .drag_source(&Selection::new())
        .expect("asset drag source");
    let item_target = &result.items[1];
    let item_point = Point::new(item_target.rect.x + 4.0, item_target.rect.y + 4.0);

    assert_eq!(
        result
            .drop_target_at(bounds, item_point, &source)
            .expect("asset item target")
            .kind,
        AssetBrowserDropTargetKind::Item { target: id(20) }
    );

    let self_point = Point::new(result.items[0].rect.x + 4.0, result.items[0].rect.y + 4.0);
    assert!(result.drop_target_at(bounds, self_point, &source).is_none());

    let disabled_point = Point::new(result.items[3].rect.x + 4.0, result.items[3].rect.y + 4.0);
    assert!(
        result
            .drop_target_at(bounds, disabled_point, &source)
            .is_none()
    );

    assert_eq!(
        result
            .drop_target_at(bounds, Point::new(260.0, 90.0), &source)
            .expect("empty-space target")
            .kind,
        AssetBrowserDropTargetKind::EmptySpace { index: 5 }
    );
}

#[test]
fn context_target_routing_preserves_item_selection_and_background_targets() {
    let mut selection = Selection::new();
    selection.replace(id(20));
    selection.toggle(id(10));
    let rows = outliner_rows(false);
    let bounds = Rect::new(0.0, 0.0, 240.0, 120.0);

    assert_eq!(
        outliner_context_target_at(
            bounds,
            &rows,
            Point::new(rows[1].rect.x + 4.0, rows[1].rect.y + 4.0),
            &selection,
        ),
        Some(CollectionContextTarget::selection([id(10), id(20)]).expect("selection target"))
    );
    assert_eq!(
        outliner_context_target_at(
            bounds,
            &rows,
            Point::new(rows[2].rect.x + 4.0, rows[2].rect.y + 4.0),
            &selection,
        ),
        Some(CollectionContextTarget::item(id(30)))
    );
    assert_eq!(
        outliner_context_target_at(bounds, &rows, Point::new(4.0, 90.0), &selection),
        Some(CollectionContextTarget::background())
    );

    let asset_bounds = Rect::new(0.0, 0.0, 300.0, 160.0);
    let assets = asset_layout().resolve(asset_bounds, &asset_model(), 0.0, &selection, None);
    assert_eq!(
        assets.context_target_at(
            asset_bounds,
            Point::new(assets.items[1].rect.x + 4.0, assets.items[1].rect.y + 4.0),
            &selection,
        ),
        Some(CollectionContextTarget::selection([id(10), id(20)]).expect("selection target"))
    );
    assert_eq!(
        assets.context_target_at(asset_bounds, Point::new(260.0, 90.0), &selection),
        Some(CollectionContextTarget::background())
    );
    assert!(
        assets
            .context_target_at(
                asset_bounds,
                Point::new(assets.items[3].rect.x + 4.0, assets.items[3].rect.y + 4.0),
                &selection,
            )
            .is_none()
    );
}

#[test]
fn context_action_requests_are_metadata_only_and_selection_aware() {
    let target = CollectionContextTarget::selection([id(20), id(10)]).expect("selection target");
    let mut disabled = action("delete", "Delete");
    disabled.state.enabled = false;
    let mut hidden = action("hidden", "Hidden");
    hidden.state.visible = false;

    let actions = collection_context_actions(
        &target,
        [action("rename", "Rename"), disabled.clone(), hidden.clone()],
    );

    assert_eq!(actions.len(), 2);
    let request = actions[0].request().expect("enabled request metadata");
    assert_eq!(request.action_id, ActionId::new("rename"));
    assert_eq!(request.target_ids, vec![id(10), id(20)]);
    assert_eq!(
        request.target,
        CollectionContextTarget::selection([id(10), id(20)]).expect("selection target")
    );
    assert!(actions[1].request().is_none());
    assert_eq!(actions[1].descriptor.id, ActionId::new("delete"));
}
