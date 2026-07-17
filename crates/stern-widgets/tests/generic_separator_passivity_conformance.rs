//! Public generic-separator passivity conformance.

use stern_core::{FrameOutput, Rect, UiInput, UiMemory, default_dark_theme};
use stern_widgets::{Ui, separator};

const SEPARATOR: Rect = Rect::new(40.0, 0.0, 30.0, 20.0);

fn separator_only(rect: Rect) -> (FrameOutput, UiMemory) {
    let theme = default_dark_theme();
    let input = UiInput::default();
    let mut memory = UiMemory::new();
    let mut ui = Ui::new(&input, &mut memory, &theme);
    ui.separator(rect);
    (ui.finish_output(), memory)
}

fn assert_separator_only(rect: Rect) {
    let theme = default_dark_theme();
    let (frame, memory) = separator_only(rect);

    assert_eq!(frame.primitives, [separator(rect, &theme)]);
    assert!(frame.semantics.nodes().is_empty());
    assert!(frame.semantics.focus_order().is_empty());
    assert!(frame.actions.is_empty());
    assert!(frame.platform_requests.is_empty());
    assert!(frame.warnings.is_empty());
    assert_eq!(memory.focused(), None);
    assert_eq!(memory.hovered(), None);
    assert_eq!(memory.active(), None);
    assert_eq!(memory.pressed(), None);
    assert_eq!(memory.pointer_capture(), None);
}

#[test]
fn ui_separator_emits_only_passive_presentation() {
    assert_separator_only(SEPARATOR);
}

#[test]
fn ui_separator_passivity_holds_for_fractional_and_zero_bounds() {
    for rect in [
        Rect::new(10.25, 20.5, 30.75, 0.5),
        Rect::new(10.0, 20.0, 0.0, 12.0),
        Rect::new(10.0, 20.0, 12.0, 0.0),
    ] {
        assert_separator_only(rect);
    }
}
