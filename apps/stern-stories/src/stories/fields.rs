//! Field and slider state sheet.

use stern::core::Rect;
use stern::text::TextEditState;
use stern::widgets::{DropdownItem, DropdownItemId, DropdownModel, SelectFieldConfig, Ui};

use crate::story::{Story, StoryKind};

/// Fields sheet: text field, search field, numeric input, select field, and
/// sliders in their static states.
#[must_use]
pub fn sheet() -> Story {
    Story {
        id: "fields/sheet",
        title: "Fields and sliders state sheet",
        kind: StoryKind::Component,
        compose,
    }
}

const ROW_HEIGHT: f32 = 24.0;
const ROW_GAP: f32 = 12.0;

#[allow(clippy::too_many_lines)]
fn compose(ui: &mut Ui<'_>, rect: Rect) {
    ui.panel(rect);
    let x = rect.x + 16.0;
    let width = (rect.width - 32.0).max(0.0);
    let mut y = rect.y + 16.0;
    let mut row = |height: f32| {
        let rect = Rect::new(x, y, width, height);
        y += height + ROW_GAP;
        rect
    };

    ui.label(row(16.0), "Text field");
    let mut name = TextEditState::new("Crate atlas 04");
    let _ = ui.text_field("text-field", row(ROW_HEIGHT), &mut name, false);

    ui.label(row(16.0), "Search field");
    let mut search = TextEditState::new("granite");
    let _ = ui.search_field("search-field", row(ROW_HEIGHT), &mut search, false);

    ui.label(row(16.0), "Numeric input");
    let mut opacity = TextEditState::new("0.85");
    let _ = ui.numeric_input("numeric-input", row(ROW_HEIGHT), &mut opacity, false);

    ui.label(row(16.0), "Select field");
    let kinds = DropdownModel::from_items([
        DropdownItem::new(DropdownItemId::from_raw(1), "Raster"),
        DropdownItem::new(DropdownItemId::from_raw(2), "Vector"),
        DropdownItem::new(DropdownItemId::from_raw(3), "Adjustment"),
    ]);
    let _ = ui.select_field(
        "select-field",
        row(ROW_HEIGHT),
        "Raster",
        &kinds,
        SelectFieldConfig::new("Select kind"),
    );

    ui.label(row(16.0), "Sliders");
    let mut level = 0.4_f32;
    let _ = ui.slider("slider", row(ROW_HEIGHT), &mut level, 0.0..=1.0, false);
    let mut exposure = 1.6_f32;
    let _ = ui.slider_with_label(
        "slider-labeled",
        row(ROW_HEIGHT),
        "Exposure",
        &mut exposure,
        0.0..=4.0,
        false,
    );
    let mut disabled_value = 0.7_f32;
    let _ = ui.slider(
        "slider-disabled",
        row(ROW_HEIGHT),
        &mut disabled_value,
        0.0..=1.0,
        true,
    );
}
