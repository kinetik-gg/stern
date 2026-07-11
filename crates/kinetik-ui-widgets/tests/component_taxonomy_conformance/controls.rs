use super::{
    ComponentCategory, ComponentConformanceStatus, DropdownCloseReason, DropdownItemId,
    DropdownModel, DropdownOverlay, Key, NumericScrubInputConfig, OverlayId, OverlayStack, Point,
    PointerButtonState, PointerInput, PopoverPlacement, RadioGroupChoice, Rect, SemanticActionKind,
    SemanticRole, SemanticValue, Size, SliderStep, TextEditState, Ui, UiInput, UiMemory, Vec2,
    WidgetId, assert_entry, classify_numeric_input_draft, default_dark_theme, item, numeric_input,
    numeric_scrub_input, pressed_key, slider_with_step,
};
use kinetik_ui_core::{MouseButton, UiInputEvent};

#[test]
fn stage2_dropdown_experimental_status_is_backed_by_public_model_and_lifecycle() {
    let mut model = DropdownModel::from_items([
        item(1, "Source"),
        item(2, "Composite").with_enabled(false),
        item(3, "Output"),
    ]);

    assert_eq!(model.highlight_first(), Some(DropdownItemId::from_raw(1)));
    assert_eq!(model.highlight_next(), Some(DropdownItemId::from_raw(3)));
    assert_eq!(
        model.select_highlighted(),
        Some(DropdownItemId::from_raw(3))
    );
    assert_eq!(
        model.selected_item().map(|item| item.label.as_str()),
        Some("Output")
    );
    assert!(!model.set_selected_id(DropdownItemId::from_raw(2)));

    model.replace_items([item(1, "Source")]);
    assert_eq!(model.selected_id(), None);

    let trigger = WidgetId::from_key("dropdown-trigger");
    let mut stack = OverlayStack::new();
    let mut dropdown = DropdownOverlay::anchored(
        OverlayId::from_raw(4),
        trigger,
        DropdownModel::from_items([item(1, "Source"), item(2, "Output")]),
        Rect::new(20.0, 20.0, 120.0, 24.0),
        Size::new(160.0, 72.0),
        PopoverPlacement::Below,
        4.0,
        true,
        Rect::new(0.0, 0.0, 320.0, 240.0),
        kinetik_ui_widgets::OverlayDismissal::OutsideClickOrEscape,
    );

    dropdown.open_in(&mut stack);
    assert_eq!(
        stack.top().map(|entry| entry.id),
        Some(OverlayId::from_raw(4))
    );
    let closed = dropdown
        .dismiss_in(&mut stack, Some(Point::new(2.0, 2.0)), false)
        .expect("outside click closes dropdown");
    assert_eq!(closed.reason, DropdownCloseReason::OutsideClick);
    assert_eq!(closed.focus_return, trigger);

    dropdown.open_in(&mut stack);
    let selected = dropdown
        .select_and_close(DropdownItemId::from_raw(2), &mut stack)
        .expect("enabled selection closes dropdown");
    assert_eq!(
        selected.reason,
        DropdownCloseReason::Selection(DropdownItemId::from_raw(2))
    );
    assert_eq!(selected.selected_id, Some(DropdownItemId::from_raw(2)));
    assert_eq!(
        dropdown.model.selected_id(),
        Some(DropdownItemId::from_raw(2))
    );
}
#[test]
fn stage2_slider_and_numeric_input_statuses_are_backed_by_public_contracts() {
    let theme = default_dark_theme();
    let slider_id = WidgetId::from_key("stage2-slider");
    let mut memory = UiMemory::new();
    memory.focus(slider_id);
    let mut value = 0.5;
    let slider = slider_with_step(
        slider_id,
        Rect::new(0.0, 0.0, 160.0, 20.0),
        &mut value,
        0.0..=1.0,
        SliderStep::new(0.25).with_page_step(0.5),
        &pressed_key(Key::ArrowRight),
        &mut memory,
        &theme,
        false,
    );

    assert!((value - 0.75).abs() < f32::EPSILON);
    let slider_semantics = slider
        .semantics
        .iter()
        .find(|node| node.role == SemanticRole::Slider)
        .expect("slider semantics");
    assert!(
        slider_semantics
            .actions
            .iter()
            .any(|action| action.kind == SemanticActionKind::SetValue)
    );

    assert_eq!(classify_numeric_input_draft("42.5").value(), Some(42.5));
    assert!(!classify_numeric_input_draft("42 px").is_acceptable());
    assert!(classify_numeric_input_draft("  ").is_empty());

    let numeric_id = WidgetId::from_key("stage2-number");
    let mut numeric_memory = UiMemory::new();
    numeric_memory.focus(numeric_id);
    numeric_memory.set_text_input_owner(numeric_id);
    let mut state = TextEditState::new("12.5");
    let numeric = numeric_input(
        numeric_id,
        Rect::new(0.0, 28.0, 120.0, 24.0),
        &mut state,
        &pressed_key(Key::Enter),
        &mut numeric_memory,
        &theme,
        false,
    );

    assert_eq!(numeric.value, Some(12.5));
    assert!(numeric.valid);
    assert!(numeric.policy.commit_requested);
    assert!(!numeric.policy.revert_requested);

    assert_numeric_scrub_contract(&theme);
}

fn assert_numeric_scrub_contract(theme: &kinetik_ui_core::Theme) {
    let scrub_id = WidgetId::from_key("stage2-numeric-scrub");
    let mut scrub_memory = UiMemory::new();
    let mut scrub_state = TextEditState::new("2");
    let mut scrub_value = 2.0;
    let scrub_rect = Rect::new(0.0, 56.0, 120.0, 24.0);
    let _ = numeric_scrub_input(
        scrub_id,
        scrub_rect,
        &mut scrub_value,
        &mut scrub_state,
        NumericScrubInputConfig::new(0.5).with_range(0.0, 10.0),
        &UiInput {
            pointer: PointerInput {
                position: Some(Point::new(4.0, 60.0)),
                primary: PointerButtonState::new(true, true, false),
                ..PointerInput::default()
            },
            ..UiInput::default()
        },
        &mut scrub_memory,
        theme,
    );
    let scrub = numeric_scrub_input(
        scrub_id,
        scrub_rect,
        &mut scrub_value,
        &mut scrub_state,
        NumericScrubInputConfig::new(0.5).with_range(0.0, 10.0),
        &UiInput {
            pointer: PointerInput {
                position: Some(Point::new(8.0, 60.0)),
                delta: Vec2::new(4.0, 0.0),
                primary: PointerButtonState::new(true, false, false),
                ..PointerInput::default()
            },
            ..UiInput::default()
        },
        &mut scrub_memory,
        theme,
    );

    assert!(scrub.scrubbed);
    assert!((scrub_value - 4.0).abs() < f32::EPSILON);
    assert!(
        scrub
            .input
            .field
            .widget
            .semantics
            .iter()
            .any(|node| matches!(
                node.state.value,
                Some(SemanticValue::Number { current, min, max })
                    if (current - 4.0).abs() < f32::EPSILON
                        && (min - 0.0).abs() < f32::EPSILON
                        && (max - 10.0).abs() < f32::EPSILON
            ))
    );
}

#[test]
fn numeric_scrub_resolves_one_canonical_domain_drag_lifecycle() {
    let theme = default_dark_theme();
    let id = WidgetId::from_key("canonical-numeric-scrub");
    let rect = Rect::new(0.0, 56.0, 120.0, 24.0);
    let config = NumericScrubInputConfig::new(0.5).with_range(0.0, 10.0);
    let mut memory = UiMemory::new();
    let mut state = TextEditState::new("2");
    let mut value = 2.0;

    memory.begin_frame();
    let mut press = UiInput::default();
    press.push_event(UiInputEvent::PointerButton {
        button: MouseButton::Primary,
        down: true,
        click_count: 1,
        position: Some(Point::new(4.0, 60.0)),
    });
    let _ = numeric_scrub_input(
        id,
        rect,
        &mut value,
        &mut state,
        config,
        &press,
        &mut memory,
        &theme,
    );

    memory.begin_frame();
    let mut moves = UiInput::default();
    moves.pointer.position = Some(Point::new(4.0, 60.0));
    moves.pointer.primary = PointerButtonState::new(true, false, false);
    moves.push_event(UiInputEvent::PointerMoved {
        position: Point::new(6.0, 60.0),
        delta: Vec2::new(2.0, 0.0),
    });
    moves.push_event(UiInputEvent::PointerMoved {
        position: Point::new(8.0, 60.0),
        delta: Vec2::new(2.0, 0.0),
    });
    let crossing = numeric_scrub_input(
        id,
        rect,
        &mut value,
        &mut state,
        config,
        &moves,
        &mut memory,
        &theme,
    );
    assert!(crossing.scrub_response.dragged);
    assert_eq!(crossing.scrub_response.drag_delta, Vec2::new(4.0, 0.0));
    assert!(crossing.scrubbed);
    assert!((value - 4.0).abs() < f32::EPSILON);

    memory.begin_frame();
    let mut release = UiInput::default();
    release.pointer.position = Some(Point::new(8.0, 60.0));
    release.pointer.primary = PointerButtonState::new(true, false, false);
    release.push_event(UiInputEvent::PointerButton {
        button: MouseButton::Primary,
        down: false,
        click_count: 1,
        position: Some(Point::new(8.0, 60.0)),
    });
    let released = numeric_scrub_input(
        id,
        rect,
        &mut value,
        &mut state,
        config,
        &release,
        &mut memory,
        &theme,
    );
    assert!(!released.scrub_response.clicked);
    assert_eq!(memory.released_drag_source(), Some(id));
}

#[test]
fn numeric_scrub_ignores_pre_press_motion_and_keeps_below_threshold_focus() {
    let theme = default_dark_theme();
    let id = WidgetId::from_key("causal-numeric-scrub");
    let rect = Rect::new(0.0, 56.0, 120.0, 24.0);
    let config = NumericScrubInputConfig::new(0.5);
    let mut memory = UiMemory::new();
    let mut state = TextEditState::new("2");
    let mut value = 2.0;

    memory.begin_frame();
    let mut press = UiInput::default();
    press.push_event(UiInputEvent::PointerMoved {
        position: Point::new(100.0, 60.0),
        delta: Vec2::new(100.0, 60.0),
    });
    press.push_event(UiInputEvent::PointerMoved {
        position: Point::new(4.0, 60.0),
        delta: Vec2::new(-96.0, 0.0),
    });
    press.push_event(UiInputEvent::PointerButton {
        button: MouseButton::Primary,
        down: true,
        click_count: 1,
        position: Some(Point::new(4.0, 60.0)),
    });
    let pressed = numeric_scrub_input(
        id,
        rect,
        &mut value,
        &mut state,
        config,
        &press,
        &mut memory,
        &theme,
    );
    assert!(!pressed.scrub_response.dragged);
    assert_eq!(memory.drag_source(), None);

    memory.begin_frame();
    let mut release = UiInput::default();
    release.pointer.position = Some(Point::new(4.0, 60.0));
    release.pointer.primary = PointerButtonState::new(true, false, false);
    release.push_event(UiInputEvent::PointerButton {
        button: MouseButton::Primary,
        down: false,
        click_count: 1,
        position: Some(Point::new(4.0, 60.0)),
    });
    let released = numeric_scrub_input(
        id,
        rect,
        &mut value,
        &mut state,
        config,
        &release,
        &mut memory,
        &theme,
    );
    assert!(released.scrub_response.clicked);
    assert!(memory.is_focused(id));
    assert_eq!(memory.released_drag_source(), None);
    assert!((value - 2.0).abs() < f32::EPSILON);
}

#[test]
fn stage2_radio_button_status_is_backed_by_label_target_group_contract() {
    assert_entry(
        "RadioButton",
        ComponentCategory::Input,
        ComponentConformanceStatus::Experimental,
    );

    let theme = default_dark_theme();
    let mut memory = UiMemory::new();
    let mut selected = 99_u8;
    let choices = [
        RadioGroupChoice::new("first", Rect::new(0.0, 0.0, 20.0, 20.0), "First", 1)
            .with_label_rect(Rect::new(24.0, 0.0, 80.0, 20.0)),
        RadioGroupChoice::new("second", Rect::new(0.0, 28.0, 20.0, 20.0), "Second", 2)
            .with_label_rect(Rect::new(24.0, 28.0, 80.0, 20.0))
            .disabled(true),
        RadioGroupChoice::new("third", Rect::new(0.0, 56.0, 20.0, 20.0), "Third", 3)
            .with_label_rect(Rect::new(24.0, 56.0, 80.0, 20.0)),
    ];
    let input = UiInput::default();
    let mut ui = Ui::new(&input, &mut memory, &theme);
    let group = ui.radio_group_value("stage2-radio", &mut selected, &choices);
    let output = ui.finish_output();

    assert_eq!(group.selected, 1);
    assert_eq!(group.selected_index, Some(0));
    assert!(group.changed);
    assert_eq!(
        output
            .semantics
            .nodes()
            .iter()
            .filter(|node| node.role == SemanticRole::RadioButton)
            .count(),
        3
    );
    assert!(output.semantics.nodes().iter().any(|node| {
        node.role == SemanticRole::RadioButton
            && node.label.as_deref() == Some("Second")
            && node.state.disabled
    }));
}
