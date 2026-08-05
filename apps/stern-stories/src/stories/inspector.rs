//! Inspector composition story: panel with property-grid rows.

use stern::core::Rect;
use stern::text::TextEditState;
use stern::widgets::inspector::PropertyGridConfig;
use stern::widgets::{
    DropdownItem, DropdownItemId, DropdownModel, ItemId, PropertyGridRow, SelectFieldConfig, Ui,
};

use crate::story::{Story, StoryKind};

const SECTION: ItemId = ItemId::from_raw(1);
const NAME: ItemId = ItemId::from_raw(2);
const KIND: ItemId = ItemId::from_raw(3);
const VISIBLE: ItemId = ItemId::from_raw(4);
const OPACITY: ItemId = ItemId::from_raw(5);

/// Inspector-with-rows: a property grid composed inside a panel, with mixed
/// value widgets. This is the composition rung where the #941 row-clipping
/// defects live.
#[must_use]
pub fn with_rows() -> Story {
    Story {
        id: "inspector/with-rows",
        title: "Inspector panel with property-grid rows",
        kind: StoryKind::Composition,
        compose,
    }
}

fn compose(ui: &mut Ui<'_>, rect: Rect) {
    ui.panel(rect);
    let bounds = rect.inset(16.0);
    ui.label(
        Rect::new(bounds.x, bounds.y, bounds.width, 16.0),
        "Inspector",
    );
    let grid_bounds = Rect::new(
        bounds.x,
        bounds.y + 24.0,
        bounds.width,
        (bounds.height - 24.0).max(0.0),
    );

    let rows = vec![
        PropertyGridRow::section(SECTION, "Selection"),
        PropertyGridRow::property(NAME, "Name", 0).with_resettable(true, true),
        PropertyGridRow::property(KIND, "Kind", 0).with_resettable(true, false),
        PropertyGridRow::property(VISIBLE, "Visible", 0),
        PropertyGridRow::property(OPACITY, "Opacity", 0).with_resettable(true, true),
    ];
    let kinds = DropdownModel::from_items([
        DropdownItem::new(DropdownItemId::from_raw(1), "Raster"),
        DropdownItem::new(DropdownItemId::from_raw(2), "Vector"),
    ]);

    let mut name = TextEditState::new("Crate atlas 04");
    let mut opacity = TextEditState::new("0.85");
    let mut visible = true;
    let _ = ui.property_grid(
        "story-inspector",
        grid_bounds,
        &rows,
        PropertyGridConfig::default(),
        |ui, cell| match cell.row.id {
            NAME => {
                let _ = ui.text_field("name", cell.value_rect, &mut name, false);
            }
            KIND => {
                let _ = ui.select_field(
                    "kind",
                    cell.value_rect,
                    "Raster",
                    &kinds,
                    SelectFieldConfig::new("Select kind"),
                );
            }
            VISIBLE => {
                let _ = ui.checkbox_value_with_label(
                    "visible",
                    cell.value_rect,
                    "Visible",
                    &mut visible,
                    false,
                );
            }
            OPACITY => {
                let _ = ui.numeric_input("opacity", cell.value_rect, &mut opacity, false);
            }
            _ => {}
        },
    );
}
