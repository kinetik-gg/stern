//! Collections sheet: sorted virtual table and scrolled virtual list.

use stern::core::{Rect, UiMemory, Vec2, WidgetId};
use stern::widgets::{
    CollectionProjection, ItemId, SortDirection, TableColumn, TableLayout, TableSort, Ui,
    VirtualListConfig, VirtualListRow, VirtualTableConfig, VirtualTableRow,
};

use crate::story::{Story, StoryKind};

/// Collections sheet: a sorted three-column table (caret sort indicators)
/// above a scrolled virtual list (scrollbar thumb).
#[must_use]
pub fn sheet() -> Story {
    Story {
        id: "collections/sheet",
        title: "Collections: sorted table + scrolled list",
        kind: StoryKind::Component,
        compose,
        seed_memory: Some(seed_scroll),
    }
}

const LIST_KEY: &str = "collections-scroll-list";

fn seed_scroll(memory: &mut UiMemory) {
    // Set (not stage) the retained offset: staged offsets commit at frame
    // end and a static render composes exactly one frame.
    memory.set_scroll_offset(
        WidgetId::from_key("root").child(LIST_KEY),
        Vec2::new(0.0, 168.0),
    );
}

fn compose(ui: &mut Ui<'_>, rect: Rect) {
    ui.panel(rect);
    let inset = 16.0;
    let width = (rect.width - inset * 2.0).max(0.0);
    let x = rect.x + inset;

    ui.label(
        Rect::new(x, rect.y + inset, width, 16.0),
        "Virtual table — Name ascending (active caret focus.indicator), others muted affordance",
    );
    let table_bounds = Rect::new(x, rect.y + 40.0, width, 180.0);

    let top = table_bounds.max_y() + 20.0;
    ui.label(
        Rect::new(x, top, width, 16.0),
        "Virtual list — scrolled 168px of 960px content (scrollbar thumb right)",
    );
    let list_bounds = Rect::new(x, top + 24.0, width, 160.0);

    // Both collections share the frame's single pointer-plan pass.
    let table_config = table_config(table_bounds);
    let list_config = VirtualListConfig::new(list_bounds, 24.0).label("Layers");
    let projection =
        CollectionProjection::from_source_ids(&(1..=24).map(ItemId::from_raw).collect::<Vec<_>>());
    let layers =
        CollectionProjection::from_source_ids(&(1..=40).map(ItemId::from_raw).collect::<Vec<_>>());
    let table = ui.prepare_virtual_table("collections-table", table_config, &projection);
    let list = ui.prepare_virtual_list(LIST_KEY, list_config, &layers);
    ui.resolve_pointer_targets(|plan| {
        if let Some(table) = &table {
            table.declare_pointer_targets(plan, stern::core::PointerOrder::new(100));
        }
        if let Some(list) = &list {
            list.declare_pointer_targets(plan, stern::core::PointerOrder::new(200));
        }
    })
    .expect("collections pointer plan");

    if let Some(table) = &table {
        let mut selection = stern::widgets::VirtualTableSelection::new();
        let _ = ui.virtual_table(table, &mut selection, |item| {
            let raw = item.id.raw();
            VirtualTableRow::new([
                format!("Asset {raw:02}"),
                if raw % 3 == 0 {
                    "Raster".to_owned()
                } else {
                    "Vector".to_owned()
                },
                format!("{} KB", raw * 137 % 4096),
            ])
        });
    }
    if let Some(list) = &list {
        let mut cursor = stern::widgets::CollectionCursor::new();
        let mut selection = stern::widgets::Selection::new();
        let _ = ui.virtual_list(list, &mut cursor, &mut selection, |item| {
            VirtualListRow::new(format!("Layer {}", item.id.raw()))
        });
    }
}

fn table_config(bounds: Rect) -> VirtualTableConfig {
    let columns = [
        TableColumn::new(ItemId::from_raw(10), "Name", 150.0),
        TableColumn::new(ItemId::from_raw(20), "Kind", 110.0),
        TableColumn::new(ItemId::from_raw(30), "Size", 90.0),
    ];
    VirtualTableConfig::new(
        bounds,
        TableLayout {
            columns: columns.to_vec(),
            header_height: 24.0,
            row_height: 26.0,
            sort: Some(TableSort {
                column: ItemId::from_raw(10),
                direction: SortDirection::Ascending,
            }),
        },
    )
    .label("Assets")
}
