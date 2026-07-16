//! Windowless widget integration for retained text layout generations and churn.

use stern_core::{
    Brush, Color, ComponentState, FrameContext, Key, KeyEvent, KeyState, Modifiers, MouseButton,
    PhysicalSize, Point, Primitive, Rect, ScaleFactor, SemanticValue, Size, TextInputEvent,
    TextLayoutId, TextPrimitive, TimeInfo, Ui as CoreUi, UiInput, UiInputEvent, UiMemory, Vec2,
    ViewportInfo, WidgetId, default_dark_theme,
};
use stern_text::{TextEditState, TextFeatureSet, TextLayoutKey, TextLayoutStore, TextStyle};
use stern_widgets::{NumericScrubInputConfig, Ui, VectorScrubInputConfig};

const FIELD_RECT: Rect = Rect::new(0.0, 0.0, 160.0, 24.0);

fn frame_context() -> FrameContext {
    FrameContext::new(
        ViewportInfo::new(
            Size::new(320.0, 180.0),
            PhysicalSize::ZERO,
            ScaleFactor::ONE,
        ),
        UiInput::default(),
        TimeInfo::default(),
    )
}

fn canonical(events: impl IntoIterator<Item = UiInputEvent>) -> UiInput {
    let mut input = UiInput::default();
    for event in events {
        input.push_event(event);
    }
    input
}

fn press(x: f32, y: f32) -> UiInputEvent {
    UiInputEvent::PointerButton {
        button: MouseButton::Primary,
        down: true,
        click_count: 1,
        position: Some(Point::new(x, y)),
    }
}

fn moved(x: f32, y: f32, delta_x: f32) -> UiInputEvent {
    UiInputEvent::PointerMoved {
        position: Point::new(x, y),
        delta: Vec2::new(delta_x, 0.0),
    }
}

fn field_id() -> WidgetId {
    WidgetId::from_key("root").child("number")
}

fn retained_ids(output: &stern_core::FrameOutput) -> Vec<u64> {
    let mut ids = output
        .primitives
        .iter()
        .filter_map(|primitive| match primitive {
            Primitive::Text(text) => text.layout.map(TextLayoutId::raw),
            _ => None,
        })
        .collect::<Vec<_>>();
    ids.sort_unstable();
    ids
}

fn assert_tabular_retention(store: &TextLayoutStore, output: &stern_core::FrameOutput) {
    assert!(!store.is_empty(), "numeric component must retain text");
    assert!(store.layouts().all(|entry| {
        entry.key.style.features == TextFeatureSet::TABULAR_NUMBERS
            && entry.key.style.family == "Inter"
    }));
    assert_retained_ids_match(store, output);
}

fn assert_retained_ids_match(store: &TextLayoutStore, output: &stern_core::FrameOutput) {
    let mut expected = store
        .layouts()
        .map(|entry| entry.id.raw())
        .collect::<Vec<_>>();
    expected.sort_unstable();
    assert_eq!(retained_ids(output), expected);
}

fn numeric_width(text: &str) -> f32 {
    let theme = default_dark_theme();
    let input = UiInput::default();
    let mut memory = UiMemory::new();
    let mut store = TextLayoutStore::new();
    let mut state = TextEditState::new(text);
    let mut ui = Ui::new(&input, &mut memory, &theme).with_text_layouts(&mut store);
    let _ = ui.numeric_input("number", FIELD_RECT, &mut state, false);
    let output = ui.finish_output();
    assert_tabular_retention(&store, &output);
    store
        .layouts()
        .next()
        .expect("numeric layout")
        .layout
        .size
        .width
}

fn generic_width(text: &str) -> f32 {
    let theme = default_dark_theme();
    let input = UiInput::default();
    let mut memory = UiMemory::new();
    let mut store = TextLayoutStore::new();
    let mut state = TextEditState::new(text);
    let mut ui = Ui::new(&input, &mut memory, &theme).with_text_layouts(&mut store);
    let _ = ui.text_field("text", FIELD_RECT, &mut state, false);
    let output = ui.finish_output();
    assert_eq!(retained_ids(&output).len(), 1);
    let entry = store.layouts().next().expect("generic layout");
    assert_eq!(entry.key.style.features, TextFeatureSet::NONE);
    entry.layout.size.width
}

#[test]
fn retained_numeric_components_are_tabular_across_access_states() {
    let theme = default_dark_theme();
    let input = UiInput::default();

    for disabled in [false, true] {
        let mut memory = UiMemory::new();
        let mut store = TextLayoutStore::new();
        let mut state = TextEditState::new("20486357");
        let mut ui = Ui::new(&input, &mut memory, &theme).with_text_layouts(&mut store);
        let _ = ui.numeric_input("number", FIELD_RECT, &mut state, disabled);
        let output = ui.finish_output();

        assert_eq!(state.text, "20486357");
        assert!(output.semantics.nodes().iter().any(|node| {
            matches!(node.state.value, Some(SemanticValue::Text(ref value)) if value == "20486357")
        }));
        assert_tabular_retention(&store, &output);
    }

    for config in [
        NumericScrubInputConfig::new(1.0),
        NumericScrubInputConfig::new(1.0).read_only(true),
        NumericScrubInputConfig::new(1.0).disabled(true),
    ] {
        let mut memory = UiMemory::new();
        let mut store = TextLayoutStore::new();
        let mut value = 20_486_356.0;
        let mut state = TextEditState::new("20486357");
        let mut ui = Ui::new(&input, &mut memory, &theme).with_text_layouts(&mut store);
        let output = ui.numeric_scrub_input("number", FIELD_RECT, &mut value, &mut state, config);
        let frame = ui.finish_output();

        assert_eq!(state.text, "20486357");
        assert!(!output.scrubbed);
        assert_tabular_retention(&store, &frame);
    }

    for config in [
        VectorScrubInputConfig::new(NumericScrubInputConfig::new(1.0)),
        VectorScrubInputConfig::new(NumericScrubInputConfig::new(1.0)).read_only(true),
        VectorScrubInputConfig::new(NumericScrubInputConfig::new(1.0)).disabled(true),
    ] {
        let mut memory = UiMemory::new();
        let mut store = TextLayoutStore::new();
        let mut values = [1.0, 2.0];
        let mut states = [
            TextEditState::new("11111111"),
            TextEditState::new("20486357"),
        ];
        let mut ui = Ui::new(&input, &mut memory, &theme).with_text_layouts(&mut store);
        let _ = ui.vector2_scrub_input(
            "vector",
            Rect::new(0.0, 0.0, 320.0, 24.0),
            "Position",
            &mut values,
            &mut states,
            config,
        );
        let frame = ui.finish_output();

        assert_eq!(states[0].text, "11111111");
        assert_eq!(states[1].text, "20486357");
        for state in &states {
            let matching = store
                .layouts()
                .filter(|entry| entry.key.text == state.text)
                .collect::<Vec<_>>();
            assert_eq!(matching.len(), 1);
            assert_eq!(
                matching[0].key.style.features,
                TextFeatureSet::TABULAR_NUMBERS
            );
        }
        assert_eq!(
            store
                .layouts()
                .filter(|entry| entry.key.style.features == TextFeatureSet::NONE)
                .count(),
            2,
            "vector axis labels remain generic text"
        );
        assert_retained_ids_match(&store, &frame);
    }
}

#[test]
fn retained_numeric_widths_are_equal_while_generic_text_remains_proportional() {
    let numeric = ["11111111", "20486357", "99999999"].map(numeric_width);
    let generic = ["11111111", "20486357", "99999999"].map(generic_width);

    assert!(
        numeric
            .windows(2)
            .all(|pair| (pair[0] - pair[1]).abs() <= 0.001),
        "tabular numeric widths diverged: {numeric:?}"
    );
    assert!(
        generic
            .windows(2)
            .any(|pair| (pair[0] - pair[1]).abs() > 0.001),
        "generic proportional control unexpectedly matched: {generic:?}"
    );
}

#[test]
#[allow(clippy::too_many_lines)]
fn retained_numeric_pointer_geometry_uses_the_tabular_navigation_authority() {
    let theme = default_dark_theme();
    let text = "11119999";
    let recipe = theme.text_field(ComponentState::default());
    let content_width = FIELD_RECT.width - recipe.padding_x * 2.0;
    let style = TextStyle::new(
        recipe.font.family,
        recipe.font.size,
        recipe.font.line_height,
    );
    let mut comparison_store = TextLayoutStore::new();
    let proportional_layout = comparison_store.shape_transient(&TextLayoutKey::new(
        text,
        style.clone(),
        content_width,
        false,
    ));
    let tabular_layout = comparison_store.shape_transient(&TextLayoutKey::new(
        text,
        style.with_features(TextFeatureSet::TABULAR_NUMBERS),
        content_width,
        false,
    ));
    let proportional_navigation = proportional_layout
        .navigation(text)
        .expect("proportional navigation");
    let tabular_navigation = tabular_layout.navigation(text).expect("tabular navigation");
    assert!(
        proportional_navigation
            .caret_stops()
            .iter()
            .chain(tabular_navigation.caret_stops())
            .all(|stop| stop.visual_line == 0),
        "the navigation witness requires one unwrapped visual line"
    );

    let mut decision_boundaries = proportional_navigation
        .caret_stops()
        .windows(2)
        .chain(tabular_navigation.caret_stops().windows(2))
        .map(|stops| (stops[0].x + stops[1].x) * 0.5)
        .collect::<Vec<_>>();
    decision_boundaries.sort_by(f32::total_cmp);
    decision_boundaries.dedup_by(|left, right| left.to_bits() == right.to_bits());
    let first_caret = tabular_navigation
        .caret_stops()
        .first()
        .expect("tabular caret stop")
        .caret;
    let line_rect = tabular_navigation.caret_rect(first_caret);
    let model_y = line_rect.y + line_rect.height * 0.5;
    let (model_x, tabular_hit, proportional_hit) = decision_boundaries
        .windows(2)
        .find_map(|bounds| {
            let x = (bounds[0] + bounds[1]) * 0.5;
            let tabular = tabular_navigation.hit_test_caret(x, model_y);
            let proportional = proportional_navigation.hit_test_caret(x, model_y);
            (tabular.offset != proportional.offset).then_some((x, tabular, proportional))
        })
        .expect("proportional and tabular hit regions must diverge");
    assert!((0.0..content_width).contains(&model_x));

    let anchor = if tabular_hit.offset == 0 {
        text.len()
    } else {
        0
    };
    let click = Point::new(
        FIELD_RECT.x + recipe.padding_x + model_x,
        FIELD_RECT.y + recipe.padding_y + recipe.font.size + model_y,
    );
    let input = canonical([
        UiInputEvent::ModifiersChanged(Modifiers::new(true, false, false, false)),
        press(click.x, click.y),
    ]);
    let mut memory = UiMemory::new();
    let mut state = TextEditState::new(text);
    state.set_caret(anchor);
    let mut store = TextLayoutStore::new();
    let mut ui = Ui::new(&input, &mut memory, &theme).with_text_layouts(&mut store);
    let _ = ui.numeric_input("number", FIELD_RECT, &mut state, false);
    let output = ui.finish_output();

    assert_eq!(state.caret_position(), tabular_hit);
    assert_ne!(state.caret_position().offset, proportional_hit.offset);
    assert_eq!(
        state.selection,
        stern_text::TextSelection::new(anchor, tabular_hit.offset)
    );

    let retained = store.layouts().next().expect("retained numeric layout");
    assert_eq!(store.len(), 1);
    assert_eq!(retained.key.text, text);
    assert_eq!(retained.key.style.features, TextFeatureSet::TABULAR_NUMBERS);
    let emitted_layout = output
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            Primitive::Text(text) => text.layout,
            _ => None,
        })
        .expect("retained numeric text primitive");
    assert_eq!(emitted_layout, retained.id);

    let retained_navigation = retained
        .layout
        .navigation(text)
        .expect("retained numeric navigation");
    let paint_offset = Vec2::new(
        FIELD_RECT.x + recipe.padding_x,
        FIELD_RECT.y + recipe.padding_y + recipe.font.size,
    );
    let expected_selection = retained_navigation
        .selection_rects(state.selection.range())
        .into_iter()
        .map(|rect| rect.translate(paint_offset))
        .collect::<Vec<_>>();
    let painted_selection = output
        .primitives
        .iter()
        .filter_map(|primitive| match primitive {
            Primitive::Rect(rect) if rect.fill == Some(recipe.selection) => Some(rect.rect),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(painted_selection, expected_selection);

    let expected_caret = retained_navigation
        .caret_rect(state.caret_position())
        .translate(paint_offset);
    let painted_caret = output
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            Primitive::Rect(rect) if rect.fill == Some(Brush::Solid(recipe.caret)) => {
                Some(rect.rect)
            }
            _ => None,
        })
        .expect("painted retained caret");
    assert_eq!(painted_caret, expected_caret);
}

#[test]
fn retained_numeric_hot_frames_reuse_one_feature_bearing_layout() {
    let theme = default_dark_theme();
    let mut memory = UiMemory::new();
    let mut store = TextLayoutStore::new();
    let mut state = TextEditState::new("20486357");
    let mut baseline = None;

    for _ in 0..64 {
        let input = UiInput::default();
        let mut ui = Ui::new(&input, &mut memory, &theme).with_text_layouts(&mut store);
        let _ = ui.numeric_input("number", FIELD_RECT, &mut state, false);
        let output = ui.finish_output();
        assert_tabular_retention(&store, &output);

        let entry = store.layouts().next().expect("retained numeric layout");
        let snapshot = (
            entry.id,
            store.len(),
            store.retained_payload_bytes(),
            store.change_cursor(),
        );
        if let Some(expected) = baseline {
            assert_eq!(snapshot, expected);
        } else {
            baseline = Some(snapshot);
        }
    }
    assert_eq!(state.text, "20486357");
}

#[test]
fn each_text_store_attachment_path_advances_exactly_once() {
    let theme = default_dark_theme();
    let mut memory = UiMemory::new();
    let mut store = TextLayoutStore::new();
    let input = UiInput::default();

    let ui = Ui::new(&input, &mut memory, &theme).with_text_layouts(&mut store);
    let _ = ui.finish_output();
    assert_eq!(store.generation(), 1);

    let ui = Ui::begin_frame_with_text_layouts(frame_context(), &mut memory, &theme, &mut store);
    let _ = ui.finish_output();
    assert_eq!(store.generation(), 2, "delegating begin path advances once");

    let runtime = CoreUi::begin_frame(frame_context(), &mut memory);
    let ui = Ui::from_core_with_text_layouts(runtime, &theme, &mut store);
    let _ = ui.finish_output();
    assert_eq!(store.generation(), 3);
}

#[test]
fn existing_external_layout_id_remains_authoritative() {
    let theme = default_dark_theme();
    let input = UiInput::default();
    for external in [
        TextLayoutId::from_raw(0),
        TextLayoutId::from_raw(0xfeed_beef),
    ] {
        let mut memory = UiMemory::new();
        let mut store = TextLayoutStore::new();
        let mut ui = Ui::new(&input, &mut memory, &theme).with_text_layouts(&mut store);
        ui.primitive(Primitive::Text(TextPrimitive {
            layout: Some(external),
            origin: Point::new(0.0, 16.0),
            text: "externally registered".to_owned(),
            family: "Inter".to_owned(),
            size: 12.0,
            line_height: 16.0,
            brush: Brush::Solid(Color::WHITE),
        }));
        let output = ui.finish_output();

        let emitted = output
            .primitives
            .iter()
            .find_map(|primitive| match primitive {
                Primitive::Text(text) => text.layout,
                _ => None,
            })
            .expect("text layout handle");
        assert_eq!(emitted, external);
        assert!(store.is_empty());
    }
}

#[test]
fn same_generation_saturation_keeps_accepted_ids_resolvable_and_rejections_layoutless() {
    let theme = default_dark_theme();
    let input = UiInput::default();
    let mut memory = UiMemory::new();
    let mut store = TextLayoutStore::new();
    let mut ui = Ui::new(&input, &mut memory, &theme).with_text_layouts(&mut store);
    for index in 0..72 {
        ui.primitive(Primitive::Text(TextPrimitive {
            layout: None,
            origin: Point::new(0.0, 16.0),
            text: format!("saturation-{index}"),
            family: format!("saturation-{index}-{}", "x".repeat(512 * 1024)),
            size: 12.0,
            line_height: 16.0,
            brush: Brush::Solid(Color::WHITE),
        }));
    }
    let output = ui.finish_output();

    let layouts = output
        .primitives
        .iter()
        .filter_map(|primitive| match primitive {
            Primitive::Text(text) => Some(text.layout),
            _ => None,
        })
        .collect::<Vec<_>>();
    let accepted = layouts.iter().flatten().copied().collect::<Vec<_>>();
    let rejected = layouts.iter().filter(|layout| layout.is_none()).count();
    assert!(!accepted.is_empty());
    assert!(
        rejected > 0,
        "same-generation pins must force strict rejection"
    );
    assert_eq!(accepted.len() + rejected, 72);
    assert!(store.retained_payload_bytes() <= 32 * 1024 * 1024);
    for id in accepted {
        assert!(store.layout(id).is_some(), "accepted ID must still resolve");
    }
}

#[test]
fn accepted_edit_retains_only_final_field_geometry() {
    let theme = default_dark_theme();
    let id = field_id();
    let input = canonical([UiInputEvent::Text(TextInputEvent::Commit("X".to_owned()))]);
    let mut memory = UiMemory::new();
    memory.focus(id);
    let mut state = TextEditState::new("0");
    let mut store = TextLayoutStore::new();
    let cursor = store.change_cursor();

    let mut ui = Ui::new(&input, &mut memory, &theme).with_text_layouts(&mut store);
    ui.text_field("number", FIELD_RECT, &mut state, false);
    let output = ui.finish_output();

    assert_eq!(state.text, "0X");
    assert_eq!(
        store.len(),
        1,
        "entry and intermediate states are transient"
    );
    let retained = store.layouts().collect::<Vec<_>>();
    assert_eq!(retained[0].key.text, "0X");
    assert_eq!(
        store
            .changes_since(cursor)
            .iter()
            .map(stern_text::TextLayoutChange::id)
            .collect::<Vec<_>>(),
        [retained[0].id]
    );
    assert!(output.primitives.iter().any(|primitive| {
        matches!(primitive, Primitive::Text(text) if text.layout == Some(retained[0].id))
    }));
}

#[test]
#[allow(clippy::too_many_lines)]
fn rejected_unique_scrub_previews_match_a_no_preview_control_for_1000_frames() {
    let theme = default_dark_theme();
    let config = NumericScrubInputConfig::new(1.0);
    let id = field_id();
    let mut preview_store = TextLayoutStore::new();
    let mut control_store = TextLayoutStore::new();
    let mut preview_state = TextEditState::new("0");
    let mut control_state = TextEditState::new("0");
    let mut preview_value = 0.0;
    let mut control_value = 0.0;

    for (store, state, value) in [
        (&mut preview_store, &mut preview_state, &mut preview_value),
        (&mut control_store, &mut control_state, &mut control_value),
    ] {
        let input = UiInput::default();
        let mut memory = UiMemory::new();
        memory.focus(id);
        let mut ui = Ui::new(&input, &mut memory, &theme).with_text_layouts(store);
        ui.numeric_scrub_input("number", FIELD_RECT, value, state, config);
        let _ = ui.finish_output();
    }

    let preview_baseline = preview_store.change_cursor();
    let control_baseline = control_store.change_cursor();
    assert_ne!(preview_baseline, control_baseline);
    let expected_ids = preview_store
        .layouts()
        .map(|entry| entry.id)
        .collect::<Vec<_>>();
    assert_eq!(
        expected_ids,
        control_store
            .layouts()
            .map(|entry| entry.id)
            .collect::<Vec<_>>()
    );

    for index in 0..1_000 {
        let preview_input = canonical([
            press(8.0, 8.0),
            moved(14.0, 8.0, 6.0),
            moved(16.0, 8.0, f32::INFINITY),
            UiInputEvent::Text(TextInputEvent::Commit(format!("preview-{index}"))),
            UiInputEvent::Key(KeyEvent::new(
                Key::ArrowLeft,
                KeyState::Pressed,
                Modifiers::default(),
                false,
            )),
        ]);
        let mut preview_memory = UiMemory::new();
        preview_memory.focus(id);
        let mut preview_ui = Ui::new(&preview_input, &mut preview_memory, &theme)
            .with_text_layouts(&mut preview_store);
        let preview = preview_ui.numeric_scrub_input(
            "number",
            FIELD_RECT,
            &mut preview_value,
            &mut preview_state,
            config,
        );
        let _ = preview_ui.finish_output();
        assert!(preview.scrub_response.dragged);
        assert!(!preview.scrubbed);

        let control_input = UiInput::default();
        let mut control_memory = UiMemory::new();
        control_memory.focus(id);
        let mut control_ui = Ui::new(&control_input, &mut control_memory, &theme)
            .with_text_layouts(&mut control_store);
        let control = control_ui.numeric_scrub_input(
            "number",
            FIELD_RECT,
            &mut control_value,
            &mut control_state,
            config,
        );
        let _ = control_ui.finish_output();
        assert!(!control.scrubbed);

        assert_eq!(preview_state.text, "0");
        assert_eq!(control_state.text, "0");
        assert_eq!(preview_value.to_bits(), 0.0_f32.to_bits());
        assert_eq!(control_value.to_bits(), 0.0_f32.to_bits());
        assert_eq!(preview_store.generation(), control_store.generation());
        assert_eq!(preview_store.len(), control_store.len());
        assert_eq!(
            preview_store.retained_payload_bytes(),
            control_store.retained_payload_bytes()
        );
        assert_eq!(
            preview_store
                .layouts()
                .map(|entry| entry.id)
                .collect::<Vec<_>>(),
            expected_ids
        );
        assert_eq!(preview_store.change_cursor(), preview_baseline);
        assert_eq!(control_store.change_cursor(), control_baseline);
        assert_eq!(
            preview_store.changes_since(preview_baseline).iter().count(),
            0
        );
        assert_eq!(
            control_store.changes_since(control_baseline).iter().count(),
            0
        );
        assert!(
            preview_store
                .changes_since(control_baseline)
                .requires_reset()
        );
        assert!(
            control_store
                .changes_since(preview_baseline)
                .requires_reset()
        );
    }

    let preview_pressure_cursor = preview_store.change_cursor();
    let control_pressure_cursor = control_store.change_cursor();
    let anchor = expected_ids[0];
    for index in 0..72 {
        preview_store.advance_generation();
        control_store.advance_generation();
        assert!(preview_store.touch_layout(anchor));
        assert!(control_store.touch_layout(anchor));
        let family = format!("pressure-{index}-{}", "x".repeat(512 * 1024));
        let request = TextLayoutKey::new(
            format!("p{index}"),
            TextStyle::new(family, 12.0, 16.0),
            80.0,
            false,
        );
        assert_eq!(
            preview_store.try_layout_id(request.clone()),
            control_store.try_layout_id(request)
        );
        assert_eq!(
            preview_store
                .layouts()
                .map(|entry| entry.id)
                .collect::<Vec<_>>(),
            control_store
                .layouts()
                .map(|entry| entry.id)
                .collect::<Vec<_>>()
        );
        assert_eq!(
            preview_store.retained_payload_bytes(),
            control_store.retained_payload_bytes()
        );
    }
    assert!(preview_store.touch_layout(anchor));
    assert_eq!(
        preview_store
            .changes_since(preview_pressure_cursor)
            .iter()
            .map(stern_text::TextLayoutChange::id)
            .collect::<Vec<_>>(),
        control_store
            .changes_since(control_pressure_cursor)
            .iter()
            .map(stern_text::TextLayoutChange::id)
            .collect::<Vec<_>>()
    );
}
