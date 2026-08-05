//! Primitive / basic-controls state sheet.

use stern::core::Rect;
use stern::widgets::Ui;

use crate::story::{Story, StoryKind};

/// Basic controls sheet: buttons, tabs, list rows, checkboxes, radios,
/// toggles, and separators in their static states.
#[must_use]
pub fn sheet() -> Story {
    Story {
        id: "basic-controls/sheet",
        title: "Basic controls state sheet",
        kind: StoryKind::Component,
        compose,
    }
}

const ROW_HEIGHT: f32 = 24.0;
const ROW_GAP: f32 = 12.0;

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

    ui.label(row(16.0), "Buttons");
    let buttons = row(ROW_HEIGHT);
    let third = (buttons.width / 3.0 - 8.0).max(0.0);
    let _ = ui.button(
        "button-default",
        Rect::new(buttons.x, buttons.y, third, buttons.height),
        "Default",
        false,
    );
    let _ = ui.button(
        "button-disabled",
        Rect::new(buttons.x + third + 12.0, buttons.y, third, buttons.height),
        "Disabled",
        true,
    );
    let _ = ui.tab_button(
        "tab-selected",
        Rect::new(
            buttons.x + 2.0 * (third + 12.0),
            buttons.y,
            third,
            buttons.height,
        ),
        "Selected tab",
        true,
        false,
    );

    ui.label(row(16.0), "Choice controls");
    let choices = row(ROW_HEIGHT);
    let half = (choices.width / 2.0 - 8.0).max(0.0);
    let _ = ui.checkbox_with_label(
        "checkbox-on",
        Rect::new(choices.x, choices.y, half, choices.height),
        "Checked",
        true,
        false,
    );
    let _ = ui.checkbox_with_label(
        "checkbox-off",
        Rect::new(choices.x + half + 16.0, choices.y, half, choices.height),
        "Unchecked",
        false,
        false,
    );
    let radios = row(ROW_HEIGHT);
    let _ = ui.radio_button_with_label(
        "radio-selected",
        Rect::new(radios.x, radios.y, half, radios.height),
        "Selected",
        true,
        false,
    );
    let _ = ui.toggle_with_label(
        "toggle-on",
        Rect::new(radios.x + half + 16.0, radios.y, half, radios.height),
        "Toggle on",
        true,
        false,
    );

    ui.separator(row(1.0));

    ui.label(row(16.0), "List rows");
    let _ = ui.list_row("list-row-selected", row(ROW_HEIGHT), "Selected row", true, false);
    let _ = ui.list_row("list-row-default", row(ROW_HEIGHT), "Default row", false, false);
    let _ = ui.list_row("list-row-disabled", row(ROW_HEIGHT), "Disabled row", false, true);
}
