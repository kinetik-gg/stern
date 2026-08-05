//! Layout-engine seam sheet: the first content-sized controls (RFC 0001 L1).
//!
//! Everything interactive here is composed through `Ui::layout` with builder
//! widgets that measure their own content — the deliberate contrast with the
//! full-width caller-rect controls in `basic-controls/sheet`. Solved rects
//! come from the L0 solver; nothing hand-computes a control width.

use stern::core::{Alignment, Insets, Rect, SizeRule};
use stern::widgets::{Button, Checkbox, Label, RadioButton, Slider, Toggle, Ui};

use crate::story::{Story, StoryKind};

/// Content-sized builder sheet composed through the `Ui::layout` seam.
#[must_use]
pub fn sheet() -> Story {
    Story {
        id: "layout/l1-builders",
        title: "Layout seam — content-sized builders",
        kind: StoryKind::Component,
        compose,
    }
}

fn compose(ui: &mut Ui<'_>, rect: Rect) {
    ui.panel(rect);
    let content = rect.inset(16.0);

    let mut opacity = 0.65;
    let mut card = None;
    let layout = ui.layout(content, |l| {
        l.column(SizeRule::Fill, SizeRule::Fit, 12.0, |l| {
            l.add(Label::new("Buttons size to their labels"));
            l.row(SizeRule::Fill, SizeRule::Fit, 8.0, |l| {
                l.add(Button::new("l1-new", "New"));
                l.add(Button::new("l1-duplicate", "Duplicate Selection"));
                l.add(Button::new("l1-delete", "Delete").disabled(true));
            });
            l.add(Label::new("Choice controls at intrinsic size"));
            l.row(SizeRule::Fill, SizeRule::Fit, 12.0, |l| {
                l.add(Checkbox::new("l1-check", "Checked", true));
                l.add(RadioButton::new("l1-radio", "Selected", true));
                l.add(Toggle::new("l1-toggle", "On", true));
            });
            l.add(Label::new("A Fit card wraps its measured content"));
            card = Some(
                l.padding(SizeRule::Fit, SizeRule::Fit, Insets::all(8.0), |l| {
                    l.column(SizeRule::Fit, SizeRule::Fit, 8.0, |l| {
                        l.add(Button::new("l1-card-a", "Card action"));
                        l.align(
                            SizeRule::Fill,
                            SizeRule::Fit,
                            Alignment::End,
                            Alignment::Start,
                            |l| {
                                l.add(Button::new("l1-card-b", "OK"));
                            },
                        );
                    });
                }),
            );
            l.add(Label::new("A slider fills; its height is intrinsic"));
            l.add(Slider::new("l1-slider", "Opacity", &mut opacity, 0.0..=1.0));
        });
    });

    // The card's solved rect exists before composition, so its panel surface
    // paints behind the content it measured.
    if let Some(card) = card {
        ui.panel(layout.rect(card));
    }
    let _ = layout.compose(ui);
}
