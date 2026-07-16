//! Windowless conformance for retained select-trigger end ellipsis.

use stern_core::{
    ComponentState, Primitive, Rect, SemanticValue, TextPrimitive, UiInput, UiMemory,
    default_dark_theme,
};
use stern_text::{TextLayoutStore, TextOverflow};
use stern_widgets::{
    DropdownItem, DropdownItemId, DropdownModel, SelectFieldConfig, SelectFieldOutput, Ui,
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
