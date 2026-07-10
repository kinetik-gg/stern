//! Widget-facade ownership reconciliation conformance.

use kinetik_ui_core::{RepaintRequest, UiInput, UiMemory, WidgetId, default_dark_theme};
use kinetik_ui_widgets::Ui;

#[test]
fn widget_facade_explicit_registration_drives_finish_output_reconciliation() {
    let owner = WidgetId::from_key("custom-derived-owner");
    let theme = default_dark_theme();
    let input = UiInput::default();
    let mut memory = UiMemory::new();
    memory.capture_pointer(owner);
    memory.activate(owner);
    memory.press(owner);
    memory.start_drag(owner);

    let mut present = Ui::new(&input, &mut memory, &theme);
    assert_eq!(present.register_id(owner), owner);
    let present_output = present.finish_output();
    assert_eq!(memory.pointer_capture(), Some(owner));
    assert_eq!(present_output.repaint, RepaintRequest::None);
    assert!(present_output.warnings.is_empty());

    let removed = Ui::new(&input, &mut memory, &theme);
    let removed_output = removed.finish_output();
    assert_eq!(memory.pointer_capture(), None);
    assert_eq!(memory.active(), None);
    assert_eq!(memory.pressed(), None);
    assert_eq!(memory.drag_source(), None);
    assert_eq!(removed_output.repaint, RepaintRequest::NextFrame);
}
