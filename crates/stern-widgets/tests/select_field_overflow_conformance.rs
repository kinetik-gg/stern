//! Windowless conformance for retained select-trigger end ellipsis.

use stern_core::{
    ComponentState, FrameOutput, Point, PointerButtonState, PointerInput, Primitive, Rect,
    SemanticValue, TextLayoutId, TextPrimitive, UiInput, UiMemory, WidgetId, default_dark_theme,
};
use stern_text::{TextLayoutStore, TextOverflow};
use stern_widgets::{
    DropdownItem, DropdownItemId, DropdownModel, SelectFieldConfig, SelectFieldOutput, Ui,
    select_field,
};

const FIELD: Rect = Rect::new(7.0, 11.0, 124.0, 24.0);
const ITEM_ID: DropdownItemId = DropdownItemId::from_raw(41);

fn selected_model(label: &str) -> DropdownModel {
    let mut model = DropdownModel::from_items([DropdownItem::new(ITEM_ID, label)]);
    assert!(model.set_selected_id(ITEM_ID));
    model
}

fn value_text(output: &SelectFieldOutput) -> &TextPrimitive {
    output
        .widget
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            Primitive::Text(text) if text.text != "v" && text.text != "^" => Some(text),
            _ => None,
        })
        .expect("select value text")
}

fn disclosure_text(output: &SelectFieldOutput) -> &TextPrimitive {
    output
        .widget
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            Primitive::Text(text) if text.text == "v" || text.text == "^" => Some(text),
            _ => None,
        })
        .expect("separate disclosure text")
}

fn expected_text_width(rect: Rect, padding_x: f32) -> f32 {
    let disclosure_width = 16.0_f32.min(rect.width.max(0.0));
    (rect.width - padding_x * 2.0 - disclosure_width).max(0.0)
}

fn retained_frame(
    store: &mut TextLayoutStore,
    memory: &mut UiMemory,
    model: &DropdownModel,
    rect: Rect,
    config: SelectFieldConfig,
) -> (SelectFieldOutput, FrameOutput) {
    let input = UiInput::default();
    retained_frame_with_input(store, memory, model, rect, config, &input)
}

fn retained_frame_with_input(
    store: &mut TextLayoutStore,
    memory: &mut UiMemory,
    model: &DropdownModel,
    rect: Rect,
    config: SelectFieldConfig,
    input: &UiInput,
) -> (SelectFieldOutput, FrameOutput) {
    let theme = default_dark_theme();
    let mut ui = Ui::new(input, memory, &theme).with_text_layouts(store);
    let output = ui.select_field("retained", rect, "Material", model, config);
    let frame = ui.finish_output();
    (output, frame)
}

fn pointer_input(pressed: bool, down: bool, released: bool) -> UiInput {
    UiInput {
        pointer: PointerInput {
            position: Some(Point::new(FIELD.x + 4.0, FIELD.y + 4.0)),
            primary: PointerButtonState::new(pressed, down, released),
            ..PointerInput::default()
        },
        ..UiInput::default()
    }
}

fn final_value_layout(frame: &FrameOutput, source: &str) -> Option<TextLayoutId> {
    frame
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            Primitive::Text(text) if text.text == source => text.layout,
            _ => None,
        })
}

fn final_disclosure_layout(frame: &FrameOutput, disclosure: &str) -> Option<TextLayoutId> {
    frame
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            Primitive::Text(text) if text.text == disclosure => text.layout,
            _ => None,
        })
}

#[test]
fn long_selected_value_uses_exact_retained_end_ellipsis_without_changing_source() {
    let source = "Complete selected material identity remains byte exact while its trigger presentation elides";
    let model = selected_model(source);
    let theme = default_dark_theme();
    let input = UiInput::default();
    let mut memory = UiMemory::new();
    let mut store = TextLayoutStore::new();
    let mut ui = Ui::new(&input, &mut memory, &theme).with_text_layouts(&mut store);

    let output = ui.select_field(
        "material",
        FIELD,
        "Material",
        &model,
        SelectFieldConfig::new("Choose material"),
    );
    let frame = ui.finish_output();
    let value = value_text(&output);
    let id = value.layout.expect("explicit retained value layout");
    let stored = store.stored_layout(id).expect("resident value layout");
    let recipe = theme.text_field(ComponentState {
        selected: true,
        ..ComponentState::default()
    });
    let expected_width = expected_text_width(FIELD, recipe.padding_x);

    assert_eq!(stored.key.width_bits, expected_width.to_bits());
    assert_eq!(stored.key.text, source);
    assert_eq!(stored.key.style.family, value.family);
    assert_eq!(stored.key.style.size_bits, value.size.to_bits());
    assert_eq!(
        stored.key.style.line_height_bits,
        value.line_height.to_bits()
    );
    assert!(!stored.key.wrap);
    assert_eq!(stored.key.overflow, TextOverflow::EndEllipsis);
    assert!(stored.layout.is_elided());
    assert_eq!(
        stored
            .layout
            .runs
            .iter()
            .flat_map(|run| &run.glyphs)
            .filter(|glyph| glyph.elided)
            .count(),
        1
    );
    assert_eq!(value.text, source);
    assert_eq!(output.presentation.label, source);
    assert_eq!(output.presentation.selected_id, Some(ITEM_ID));
    assert!(!output.presentation.placeholder);
    assert_eq!(
        output.widget.semantics[0].description.as_deref(),
        Some(source)
    );
    assert_eq!(
        output.widget.semantics[0].state.value,
        Some(SemanticValue::Text(source.to_owned()))
    );
    assert!(output.widget.semantics[0].state.selected);
    assert_eq!(disclosure_text(&output).text, "v");
    assert_eq!(disclosure_text(&output).layout, None);
    assert!(frame.warnings.is_empty());
}

#[test]
fn fitting_selected_value_keeps_explicit_policy_without_elision() {
    let source = "Fit";
    let model = selected_model(source);
    let theme = default_dark_theme();
    let input = UiInput::default();
    let mut memory = UiMemory::new();
    let mut store = TextLayoutStore::new();
    let mut ui = Ui::new(&input, &mut memory, &theme).with_text_layouts(&mut store);

    let output = ui.select_field(
        "fitting",
        FIELD,
        "Material",
        &model,
        SelectFieldConfig::new("Choose material"),
    );
    let _ = ui.finish_output();
    let value = value_text(&output);
    let stored = store
        .stored_layout(value.layout.expect("explicit retained value layout"))
        .expect("resident fitting value layout");

    assert_eq!(stored.key.text, source);
    assert_eq!(stored.key.overflow, TextOverflow::EndEllipsis);
    assert!(!stored.layout.is_elided());
    assert!(
        stored
            .layout
            .runs
            .iter()
            .flat_map(|run| &run.glyphs)
            .all(|glyph| !glyph.elided)
    );
    assert_eq!(value.text, source);
    assert_eq!(output.presentation.label, source);
}

#[test]
fn public_low_level_select_field_remains_layoutless_and_complete_source() {
    let source = "The direct compatibility helper retains this complete selected value";
    let model = selected_model(source);
    let theme = default_dark_theme();
    let output = select_field(
        WidgetId::from_key("direct-select"),
        FIELD,
        "Material",
        &model,
        SelectFieldConfig::new("Choose material"),
        &UiInput::default(),
        &mut UiMemory::new(),
        &theme,
    );
    let value = value_text(&output);

    assert_eq!(value.layout, None);
    assert_eq!(value.text, source);
    assert_eq!(output.presentation.label, source);
    assert_eq!(output.presentation.selected_id, Some(ITEM_ID));
    assert_eq!(
        output.widget.semantics[0].description.as_deref(),
        Some(source)
    );
    assert_eq!(
        output.widget.semantics[0].state.value,
        Some(SemanticValue::Text(source.to_owned()))
    );
    assert_eq!(disclosure_text(&output).layout, None);
}

#[test]
fn long_placeholder_uses_retained_policy_without_becoming_selected() {
    let placeholder =
        "Complete placeholder source remains semantic placeholder text even when it must elide";
    let model = DropdownModel::from_items([DropdownItem::new(ITEM_ID, "Available value")]);
    let theme = default_dark_theme();
    let input = UiInput::default();
    let mut memory = UiMemory::new();
    let mut store = TextLayoutStore::new();
    let mut ui = Ui::new(&input, &mut memory, &theme).with_text_layouts(&mut store);

    let output = ui.select_field(
        "placeholder",
        FIELD,
        "Material",
        &model,
        SelectFieldConfig::new(placeholder),
    );
    let _ = ui.finish_output();
    let value = value_text(&output);
    let stored = store
        .stored_layout(value.layout.expect("explicit placeholder layout"))
        .expect("resident placeholder layout");

    assert_eq!(stored.key.text, placeholder);
    assert_eq!(stored.key.overflow, TextOverflow::EndEllipsis);
    assert!(stored.layout.is_elided());
    assert_eq!(value.text, placeholder);
    assert_eq!(output.presentation.label, placeholder);
    assert_eq!(output.presentation.selected_id, None);
    assert!(output.presentation.placeholder);
    assert!(!output.widget.semantics[0].state.selected);
    assert_eq!(
        output.widget.semantics[0].description.as_deref(),
        Some(placeholder)
    );
    assert_eq!(
        output.widget.semantics[0].state.value,
        Some(SemanticValue::Text(placeholder.to_owned()))
    );
}

#[test]
fn open_disabled_and_read_only_states_preserve_value_identity_and_disclosure_isolation() {
    let source = "Complete selected source survives every select trigger presentation state";
    let model = selected_model(source);

    for (config, open, disabled, read_only) in [
        (SelectFieldConfig::new("Choose"), false, false, false),
        (
            SelectFieldConfig::new("Choose").open(true),
            true,
            false,
            false,
        ),
        (
            SelectFieldConfig::new("Choose").disabled(true),
            false,
            true,
            false,
        ),
        (
            SelectFieldConfig::new("Choose").read_only(true),
            false,
            true,
            true,
        ),
    ] {
        let theme = default_dark_theme();
        let input = UiInput::default();
        let mut memory = UiMemory::new();
        let mut store = TextLayoutStore::new();
        let mut ui = Ui::new(&input, &mut memory, &theme).with_text_layouts(&mut store);
        let output = ui.select_field("state", FIELD, "Material", &model, config);
        let _ = ui.finish_output();
        let value = value_text(&output);
        let stored = store
            .stored_layout(value.layout.expect("explicit state value layout"))
            .expect("resident state value layout");
        let semantic = &output.widget.semantics[0];

        assert_eq!(stored.key.text, source);
        assert_eq!(stored.key.overflow, TextOverflow::EndEllipsis);
        assert_eq!(value.text, source);
        assert_eq!(output.presentation.label, source);
        assert_eq!(output.presentation.selected_id, Some(ITEM_ID));
        assert!(!output.presentation.placeholder);
        assert_eq!(output.presentation.open, open);
        assert_eq!(output.presentation.disabled, disabled);
        assert_eq!(output.read_only, read_only);
        assert_eq!(output.response.state.disabled, disabled);
        assert!(!output.open_requested);
        assert_eq!(semantic.description.as_deref(), Some(source));
        assert_eq!(
            semantic.state.value,
            Some(SemanticValue::Text(source.to_owned()))
        );
        assert!(semantic.state.selected);
        assert_eq!(semantic.state.disabled, disabled);
        assert_eq!(semantic.state.expanded, Some(open));
        assert_eq!(disclosure_text(&output).text, if open { "^" } else { "v" });
        assert_eq!(disclosure_text(&output).layout, None);
    }
}

#[test]
fn empty_all_disabled_and_missing_selection_keep_placeholder_model_contracts() {
    let mut missing = selected_model("Removed selection");
    missing.replace_items([DropdownItem::new(
        DropdownItemId::from_raw(99),
        "Remaining enabled item",
    )]);
    let cases = [
        (
            "empty",
            DropdownModel::new(),
            "Complete empty-model placeholder remains available to semantics",
            true,
        ),
        (
            "all-disabled",
            DropdownModel::from_items([
                DropdownItem::new(DropdownItemId::from_raw(71), "Disabled A").with_enabled(false),
                DropdownItem::new(DropdownItemId::from_raw(72), "Disabled B").with_enabled(false),
            ]),
            "Complete all-disabled placeholder remains available to semantics",
            true,
        ),
        (
            "missing-selection",
            missing,
            "Complete missing-selection placeholder remains available to semantics",
            false,
        ),
    ];

    for (key, model, placeholder, disabled) in cases {
        let model_before = model.clone();
        let theme = default_dark_theme();
        let input = UiInput::default();
        let mut memory = UiMemory::new();
        let mut store = TextLayoutStore::new();
        let mut ui = Ui::new(&input, &mut memory, &theme).with_text_layouts(&mut store);
        let output = ui.select_field(
            key,
            FIELD,
            "Material",
            &model,
            SelectFieldConfig::new(placeholder),
        );
        let _ = ui.finish_output();
        let value = value_text(&output);
        let stored = store
            .stored_layout(value.layout.expect("explicit placeholder layout"))
            .expect("resident placeholder layout");
        let semantic = &output.widget.semantics[0];

        assert_eq!(model, model_before, "{key}");
        assert_eq!(model.selected_id(), None, "{key}");
        assert_eq!(stored.key.text, placeholder, "{key}");
        assert_eq!(stored.key.overflow, TextOverflow::EndEllipsis, "{key}");
        assert_eq!(value.text, placeholder, "{key}");
        assert_eq!(output.presentation.label, placeholder, "{key}");
        assert_eq!(output.presentation.selected_id, None, "{key}");
        assert!(output.presentation.placeholder, "{key}");
        assert_eq!(output.presentation.disabled, disabled, "{key}");
        assert_eq!(output.response.state.disabled, disabled, "{key}");
        assert!(!output.response.clicked, "{key}");
        assert!(!output.response.keyboard_activated, "{key}");
        assert!(!output.open_requested, "{key}");
        assert!(!semantic.state.selected, "{key}");
        assert_eq!(semantic.state.disabled, disabled, "{key}");
        assert_eq!(semantic.description.as_deref(), Some(placeholder), "{key}");
        assert_eq!(
            semantic.state.value,
            Some(SemanticValue::Text(placeholder.to_owned())),
            "{key}"
        );
    }
}

#[test]
fn identical_hot_frames_reuse_value_id_and_retained_accounting() {
    let source = "Stable selected source remains retained across identical hot frames";
    let model = selected_model(source);
    let mut store = TextLayoutStore::new();
    let mut memory = UiMemory::new();
    let (first, first_frame) = retained_frame(
        &mut store,
        &mut memory,
        &model,
        FIELD,
        SelectFieldConfig::new("Choose"),
    );
    let first_id = value_text(&first).layout.expect("first retained value ID");
    assert_eq!(final_value_layout(&first_frame, source), Some(first_id));
    let first_len = store.len();
    let first_bytes = store.retained_payload_bytes();
    let first_cursor = store.change_cursor();

    for _ in 0..4 {
        let (output, frame) = retained_frame(
            &mut store,
            &mut memory,
            &model,
            FIELD,
            SelectFieldConfig::new("Choose"),
        );
        assert_eq!(value_text(&output).layout, Some(first_id));
        assert_eq!(final_value_layout(&frame, source), Some(first_id));
        assert_eq!(store.len(), first_len);
        assert_eq!(store.retained_payload_bytes(), first_bytes);
        assert_eq!(store.change_cursor(), first_cursor);
    }
}

#[test]
fn source_and_width_change_value_identity_while_open_only_changes_disclosure() {
    let source = "Stable selected source for identity transition evidence";
    let model = selected_model(source);
    let mut store = TextLayoutStore::new();
    let mut memory = UiMemory::new();
    let (closed, closed_frame) = retained_frame(
        &mut store,
        &mut memory,
        &model,
        FIELD,
        SelectFieldConfig::new("Choose"),
    );
    let closed_id = value_text(&closed)
        .layout
        .expect("closed retained value ID");
    let closed_disclosure_id =
        final_disclosure_layout(&closed_frame, "v").expect("closed disclosure layout");

    let (open, open_frame) = retained_frame(
        &mut store,
        &mut memory,
        &model,
        FIELD,
        SelectFieldConfig::new("Choose").open(true),
    );
    let open_id = value_text(&open).layout.expect("open retained value ID");
    let open_disclosure_id =
        final_disclosure_layout(&open_frame, "^").expect("open disclosure layout");
    assert_eq!(open_id, closed_id);
    assert_eq!(final_value_layout(&open_frame, source), Some(closed_id));
    assert_ne!(open_disclosure_id, closed_disclosure_id);
    let expected_disclosure_x = (FIELD.max_x() - 16.0).to_bits();
    assert_eq!(
        disclosure_text(&closed).origin.x.to_bits(),
        expected_disclosure_x
    );
    assert_eq!(
        disclosure_text(&open).origin.x.to_bits(),
        expected_disclosure_x
    );

    let changed_source = "A different complete selected source produces distinct identity";
    let changed_model = selected_model(changed_source);
    let (changed, changed_frame) = retained_frame(
        &mut store,
        &mut memory,
        &changed_model,
        FIELD,
        SelectFieldConfig::new("Choose"),
    );
    let changed_id = value_text(&changed)
        .layout
        .expect("changed-source retained value ID");
    assert_ne!(changed_id, closed_id);
    assert_eq!(
        final_value_layout(&changed_frame, changed_source),
        Some(changed_id)
    );

    let wider = Rect::new(FIELD.x, FIELD.y, FIELD.width + 13.25, FIELD.height);
    let (resized, resized_frame) = retained_frame(
        &mut store,
        &mut memory,
        &model,
        wider,
        SelectFieldConfig::new("Choose"),
    );
    let resized_id = value_text(&resized)
        .layout
        .expect("resized retained value ID");
    assert_ne!(resized_id, closed_id);
    assert_eq!(final_value_layout(&resized_frame, source), Some(resized_id));
    assert_eq!(
        store
            .stored_layout(resized_id)
            .expect("resized resident layout")
            .key
            .width_bits,
        expected_text_width(
            wider,
            default_dark_theme()
                .text_field(ComponentState::default())
                .padding_x
        )
        .to_bits()
    );
}

#[test]
fn over_budget_source_rejects_custom_identity_without_store_mutation_or_source_loss() {
    const RETAINED_PAYLOAD_CEILING: usize = 32 * 1024 * 1024;

    let mut store = TextLayoutStore::new();
    let mut memory = UiMemory::new();
    let warm_model = selected_model("Warm the stable disclosure layout");
    let _ = retained_frame(
        &mut store,
        &mut memory,
        &warm_model,
        FIELD,
        SelectFieldConfig::new("Choose"),
    );
    let len_before = store.len();
    let bytes_before = store.retained_payload_bytes();
    let cursor_before = store.change_cursor();

    let source = "x".repeat(RETAINED_PAYLOAD_CEILING + 1);
    let model = selected_model(&source);
    let (output, frame) = retained_frame(
        &mut store,
        &mut memory,
        &model,
        FIELD,
        SelectFieldConfig::new("Choose"),
    );
    let value = value_text(&output);

    assert_eq!(value.layout, None, "custom admission must reject");
    if let Some(final_id) = final_value_layout(&frame, &source) {
        let stored = store
            .stored_layout(final_id)
            .expect("generic attachment may emit only a registered ID");
        assert_eq!(stored.key.text, source);
        assert_eq!(stored.key.overflow, TextOverflow::Visible);
    }
    assert_eq!(store.len(), len_before);
    assert_eq!(store.retained_payload_bytes(), bytes_before);
    assert_eq!(store.change_cursor(), cursor_before);
    assert_eq!(value.text, source);
    assert_eq!(output.presentation.label, source);
    assert_eq!(output.presentation.selected_id, Some(ITEM_ID));
    assert_eq!(
        output.widget.semantics[0].description.as_deref(),
        Some(source.as_str())
    );
    match output.widget.semantics[0].state.value.as_ref() {
        Some(SemanticValue::Text(value)) => assert_eq!(value, &source),
        other => panic!("expected complete semantic text, got {other:?}"),
    }
}

#[test]
fn nonpositive_and_nonfinite_text_geometry_preserves_visible_complete_source() {
    let source = "Complete source remains visible when select geometry is ineligible";
    let model = selected_model(source);

    for width in [0.0, -20.0, f32::NAN, f32::INFINITY] {
        let rect = Rect::new(FIELD.x, FIELD.y, width, FIELD.height);
        let theme = default_dark_theme();
        let recipe = theme.text_field(ComponentState {
            selected: true,
            ..ComponentState::default()
        });
        let expected_width = expected_text_width(rect, recipe.padding_x);
        let mut store = TextLayoutStore::new();
        let mut memory = UiMemory::new();
        let (output, frame) = retained_frame(
            &mut store,
            &mut memory,
            &model,
            rect,
            SelectFieldConfig::new("Choose"),
        );
        let value = value_text(&output);
        let id = value.layout.expect("registered ineligible-policy layout");
        let stored = store
            .stored_layout(id)
            .expect("resident ineligible-policy layout");

        assert_eq!(stored.key.width_bits, expected_width.to_bits());
        assert_eq!(stored.key.overflow, TextOverflow::EndEllipsis);
        assert!(!stored.layout.is_elided());
        assert_eq!(stored.key.text, source);
        assert_eq!(value.text, source);
        assert_eq!(output.presentation.label, source);
        assert_eq!(final_value_layout(&frame, source), Some(id));
        assert_eq!(
            output.widget.semantics[0].description.as_deref(),
            Some(source)
        );
    }
}

#[test]
fn multiline_and_paragraph_separator_labels_remain_visible_and_complete() {
    for source in [
        "First complete line\nSecond complete line",
        "First complete line\r\nSecond complete line",
        "First complete paragraph\u{2029}Second complete paragraph",
    ] {
        let model = selected_model(source);
        let mut store = TextLayoutStore::new();
        let mut memory = UiMemory::new();
        let (output, frame) = retained_frame(
            &mut store,
            &mut memory,
            &model,
            FIELD,
            SelectFieldConfig::new("Choose"),
        );
        let value = value_text(&output);
        let id = value.layout.expect("registered multiline-policy layout");
        let stored = store
            .stored_layout(id)
            .expect("resident multiline-policy layout");

        assert_eq!(stored.key.overflow, TextOverflow::EndEllipsis);
        assert!(!stored.layout.is_elided());
        assert!(
            stored
                .layout
                .runs
                .iter()
                .flat_map(|run| &run.glyphs)
                .all(|glyph| !glyph.elided)
        );
        assert_eq!(stored.key.text, source);
        assert_eq!(value.text, source);
        assert_eq!(output.presentation.label, source);
        assert_eq!(final_value_layout(&frame, source), Some(id));
        assert_eq!(
            output.widget.semantics[0].state.value,
            Some(SemanticValue::Text(source.to_owned()))
        );
    }
}

#[test]
fn unresolved_model_states_preserve_open_request_eligibility() {
    let mut missing = selected_model("Removed selection");
    missing.replace_items([DropdownItem::new(
        DropdownItemId::from_raw(99),
        "Remaining enabled item",
    )]);
    let cases = [
        (DropdownModel::new(), true),
        (
            DropdownModel::from_items([DropdownItem::new(
                DropdownItemId::from_raw(71),
                "Disabled A",
            )
            .with_enabled(false)]),
            true,
        ),
        (missing, false),
    ];

    for (model, disabled) in cases {
        let mut store = TextLayoutStore::new();
        let mut memory = UiMemory::new();
        let _ = retained_frame_with_input(
            &mut store,
            &mut memory,
            &model,
            FIELD,
            SelectFieldConfig::new("Complete unresolved placeholder"),
            &pointer_input(true, true, false),
        );
        let (released, _) = retained_frame_with_input(
            &mut store,
            &mut memory,
            &model,
            FIELD,
            SelectFieldConfig::new("Complete unresolved placeholder"),
            &pointer_input(false, false, true),
        );

        assert_eq!(released.response.state.disabled, disabled);
        assert_eq!(released.response.clicked, !disabled);
        assert_eq!(released.open_requested, !disabled);
        assert_eq!(model.selected_id(), None);
        assert!(released.presentation.placeholder);
        assert!(!released.widget.semantics[0].state.selected);
    }
}
