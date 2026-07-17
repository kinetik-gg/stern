//! Windowless conformance for retained property-grid section typography.

use stern_core::{
    FontWeightScale, FontWeightToken, FrameOutput, Primitive, Rect, SemanticRole, TextLayoutId,
    TextPrimitive, TextRoleMetrics, Theme, UiInput, UiMemory, default_dark_theme,
};
use stern_text::{TextFeatureSet, TextLayoutStore, TextOverflow, fonts};
use stern_widgets::{
    ItemId, Ui,
    inspector::{PropertyGridConfig, PropertyGridOutput, PropertyGridRow, PropertyGridRowStatus},
};

const BOUNDS: Rect = Rect::new(10.0, 20.0, 360.0, 140.0);

fn retained_grid(
    store: &mut TextLayoutStore,
    memory: &mut UiMemory,
    rows: &[PropertyGridRow],
    bounds: Rect,
    theme: &Theme,
) -> (PropertyGridOutput<()>, FrameOutput) {
    let input = UiInput::default();
    let mut ui = Ui::new(&input, memory, theme).with_text_layouts(store);
    let output = ui
        .property_grid(
            "grid",
            bounds,
            rows,
            PropertyGridConfig::default().with_overscan(0),
            |_, _| (),
        )
        .expect("valid property rows");
    (output, ui.finish_output())
}

fn layoutless_grid(
    memory: &mut UiMemory,
    rows: &[PropertyGridRow],
    bounds: Rect,
    theme: &Theme,
) -> (PropertyGridOutput<()>, FrameOutput) {
    let input = UiInput::default();
    let mut ui = Ui::new(&input, memory, theme);
    let output = ui
        .property_grid(
            "grid",
            bounds,
            rows,
            PropertyGridConfig::default().with_overscan(0),
            |_, _| (),
        )
        .expect("valid property rows");
    (output, ui.finish_output())
}

fn text<'a>(frame: &'a FrameOutput, source: &str) -> &'a TextPrimitive {
    frame
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            Primitive::Text(text) if text.text == source => Some(text),
            _ => None,
        })
        .expect("text primitive")
}

fn layout_id(frame: &FrameOutput, source: &str) -> TextLayoutId {
    text(frame, source)
        .layout
        .expect("registered text layout identity")
}

#[test]
fn default_section_uses_exact_title_metrics_weight_and_selected_inter_face() {
    let section_source = "Transform";
    let property_source = "Opacity";
    let rows = [
        PropertyGridRow::section(ItemId::from_raw(1), section_source),
        PropertyGridRow::property(ItemId::from_raw(2), property_source, 0)
            .with_help_text("Opacity help")
            .with_status(PropertyGridRowStatus::error("Opacity error")),
    ];
    let theme = default_dark_theme();
    let mut store = TextLayoutStore::new();
    let mut memory = UiMemory::new();
    let (output, frame) = retained_grid(&mut store, &mut memory, &rows, BOUNDS, &theme);

    assert_eq!(output.visible_rows.len(), 2);
    assert_eq!(output.values.len(), 1);
    assert!(output.intents.is_empty());

    let section = text(&frame, section_source);
    assert_eq!(section.family, "Inter");
    assert_eq!(section.size.to_bits(), 14.0_f32.to_bits());
    assert_eq!(section.line_height.to_bits(), 19.0_f32.to_bits());
    assert_eq!(
        section.origin.x.to_bits(),
        (output.visible_rows[0].label_rect.x + 8.0).to_bits()
    );
    assert_eq!(
        section.origin.y.to_bits(),
        (output.visible_rows[0].label_rect.y + 20.0).to_bits()
    );

    let section_id = section.layout.expect("retained section layout");
    let retained = store
        .stored_layout(section_id)
        .expect("resident section layout");
    assert_eq!(retained.key.text.as_bytes(), section_source.as_bytes());
    assert_eq!(retained.key.style.family, section.family);
    assert_eq!(retained.key.style.size().to_bits(), section.size.to_bits());
    assert_eq!(
        retained.key.style.line_height().to_bits(),
        section.line_height.to_bits()
    );
    assert_eq!(retained.key.style.weight, 600);
    assert_eq!(retained.key.style.features, TextFeatureSet::NONE);
    assert_eq!(retained.key.width_bits, 0.0_f32.to_bits());
    assert!(!retained.key.wrap);
    assert_eq!(retained.key.overflow, TextOverflow::Visible);
    assert!(!retained.layout.is_empty());
    assert!(!retained.layout.is_elided());
    assert_eq!(retained.layout.lines.first().expect("line").text_start, 0);
    assert_eq!(
        retained.layout.lines.last().expect("line").text_end,
        section_source.len()
    );
    assert!(retained.layout.runs.iter().all(|run| {
        run.font.data.data() == fonts::INTER_VARIABLE && run.normalized_coords == [0, 5_898]
    }));

    let property = text(&frame, property_source);
    let help = text(&frame, "?");
    let status = text(&frame, "x");
    for primitive in [property, help, status] {
        assert_eq!(primitive.family, "Inter");
        assert_eq!(primitive.size.to_bits(), 12.0_f32.to_bits());
        assert_eq!(primitive.line_height.to_bits(), 16.0_f32.to_bits());
        let stored = store
            .stored_layout(primitive.layout.expect("retained label text"))
            .expect("resident label text");
        assert_eq!(stored.key.style.weight, 400);
        assert_eq!(stored.key.style.features, TextFeatureSet::NONE);
    }
    assert_eq!(
        store
            .stored_layout(property.layout.expect("property layout"))
            .expect("property entry")
            .key
            .overflow,
        TextOverflow::EndEllipsis
    );
    assert!(frame.semantics.nodes().iter().any(|node| {
        node.role == SemanticRole::Label && node.label.as_deref() == Some(section_source)
    }));
    assert!(frame.semantics.nodes().iter().any(|node| {
        node.role == SemanticRole::Row && node.label.as_deref() == Some(property_source)
    }));
}

#[test]
fn customized_typography_drives_title_family_metrics_and_semibold_request() {
    let source = "Custom semantic section";
    let mut typography = default_dark_theme().typography;
    typography.families.ui = "Space Grotesk";
    typography.families.brand = "Inter";
    typography.title = TextRoleMetrics::new(17.0, 23.0);
    typography.weights = FontWeightScale::new(410, 450, 500, 750);
    let theme = default_dark_theme().with_typography(typography);
    assert_eq!(theme.typography.weights.get(FontWeightToken::Semibold), 500);

    let mut store = TextLayoutStore::new();
    let mut memory = UiMemory::new();
    let (output, frame) = retained_grid(
        &mut store,
        &mut memory,
        &[PropertyGridRow::section(ItemId::from_raw(11), source)],
        BOUNDS,
        &theme,
    );
    let section = text(&frame, source);
    assert_eq!(section.family, "Space Grotesk");
    assert_ne!(section.family, theme.typography.families.brand);
    assert_eq!(section.size.to_bits(), 17.0_f32.to_bits());
    assert_eq!(section.line_height.to_bits(), 23.0_f32.to_bits());
    assert_eq!(
        section.origin.y.to_bits(),
        (output.visible_rows[0].label_rect.y + 21.5).to_bits()
    );

    let retained = store
        .stored_layout(layout_id(&frame, source))
        .expect("custom retained section");
    assert_eq!(retained.key.style.family, "Space Grotesk");
    assert_eq!(retained.key.style.size().to_bits(), 17.0_f32.to_bits());
    assert_eq!(
        retained.key.style.line_height().to_bits(),
        23.0_f32.to_bits()
    );
    assert_eq!(retained.key.style.weight, 500);
    assert!(retained.layout.runs.iter().all(|run| {
        run.font.data.data() == fonts::SPACE_GROTESK_VARIABLE && run.normalized_coords == [10_650]
    }));
}

#[test]
fn hot_translation_and_disabled_frames_reuse_section_identity() {
    let source = "Stable section identity";
    let row = PropertyGridRow::section(ItemId::from_raw(21), source);
    let theme = default_dark_theme();
    let mut store = TextLayoutStore::new();
    let mut memory = UiMemory::new();
    let (_, first) = retained_grid(
        &mut store,
        &mut memory,
        std::slice::from_ref(&row),
        BOUNDS,
        &theme,
    );
    let stable_id = layout_id(&first, source);
    let stable_origin = text(&first, source).origin;
    let stable_brush = text(&first, source).brush;
    let accounting = (
        store.len(),
        store.retained_payload_bytes(),
        store.change_cursor(),
    );

    for _ in 0..4 {
        let (_, hot) = retained_grid(
            &mut store,
            &mut memory,
            std::slice::from_ref(&row),
            BOUNDS,
            &theme,
        );
        assert_eq!(layout_id(&hot, source), stable_id);
        assert_eq!(
            (
                store.len(),
                store.retained_payload_bytes(),
                store.change_cursor()
            ),
            accounting
        );
    }

    let translated_bounds = Rect::new(47.25, 91.5, BOUNDS.width, BOUNDS.height);
    let (_, translated) = retained_grid(
        &mut store,
        &mut memory,
        std::slice::from_ref(&row),
        translated_bounds,
        &theme,
    );
    assert_eq!(layout_id(&translated, source), stable_id);
    assert_ne!(text(&translated, source).origin, stable_origin);
    assert_eq!(
        (
            store.len(),
            store.retained_payload_bytes(),
            store.change_cursor()
        ),
        accounting
    );

    let disabled_row = row.clone().with_disabled(true);
    let (_, disabled) = retained_grid(&mut store, &mut memory, &[disabled_row], BOUNDS, &theme);
    assert_eq!(layout_id(&disabled, source), stable_id);
    assert_ne!(text(&disabled, source).brush, stable_brush);
    assert!(
        disabled
            .semantics
            .nodes()
            .iter()
            .any(|node| { node.label.as_deref() == Some(source) && node.state.disabled })
    );
    assert_eq!(
        (
            store.len(),
            store.retained_payload_bytes(),
            store.change_cursor()
        ),
        accounting
    );

    let changed_source = "Changed section source";
    let (_, changed) = retained_grid(
        &mut store,
        &mut memory,
        &[PropertyGridRow::section(
            ItemId::from_raw(21),
            changed_source,
        )],
        BOUNDS,
        &theme,
    );
    assert_ne!(layout_id(&changed, changed_source), stable_id);

    let mut changed_typography = theme.typography;
    changed_typography.title = TextRoleMetrics::new(15.0, 20.0);
    let changed_theme = theme.with_typography(changed_typography);
    let (_, restyled) = retained_grid(&mut store, &mut memory, &[row], BOUNDS, &changed_theme);
    assert_ne!(layout_id(&restyled, source), stable_id);
}

#[test]
fn layoutless_section_preserves_complete_title_geometry_and_semantics() {
    let source = "Layoutless complete section";
    let theme = default_dark_theme();
    let mut memory = UiMemory::new();
    let (output, frame) = layoutless_grid(
        &mut memory,
        &[PropertyGridRow::section(ItemId::from_raw(31), source)],
        BOUNDS,
        &theme,
    );

    assert!(output.values.is_empty());
    assert!(output.intents.is_empty());
    let section = text(&frame, source);
    assert_eq!(section.layout, None);
    assert_eq!(section.text.as_bytes(), source.as_bytes());
    assert_eq!(section.family, "Inter");
    assert_eq!(section.size.to_bits(), 14.0_f32.to_bits());
    assert_eq!(section.line_height.to_bits(), 19.0_f32.to_bits());
    assert_eq!(
        section.origin.x.to_bits(),
        (output.visible_rows[0].label_rect.x + 8.0).to_bits()
    );
    assert!(
        frame.semantics.nodes().iter().any(|node| {
            node.role == SemanticRole::Label && node.label.as_deref() == Some(source)
        })
    );
}

#[test]
fn over_budget_section_rejection_is_transactional_and_leaks_no_identity() {
    const RETAINED_PAYLOAD_CEILING: usize = 32 * 1024 * 1024;

    let theme = default_dark_theme();
    let mut store = TextLayoutStore::new();
    let mut memory = UiMemory::new();
    let _ = retained_grid(
        &mut store,
        &mut memory,
        &[PropertyGridRow::section(
            ItemId::from_raw(41),
            "Warm retained section",
        )],
        BOUNDS,
        &theme,
    );
    let accounting = (
        store.len(),
        store.retained_payload_bytes(),
        store.change_cursor(),
    );

    let source = "x".repeat(RETAINED_PAYLOAD_CEILING + 1);
    let (output, frame) = retained_grid(
        &mut store,
        &mut memory,
        &[PropertyGridRow::section(ItemId::from_raw(42), &source)],
        BOUNDS,
        &theme,
    );
    let section = text(&frame, &source);
    assert_eq!(section.layout, None);
    assert_eq!(section.text.as_bytes(), source.as_bytes());
    assert_eq!(section.family, "Inter");
    assert_eq!(section.size.to_bits(), 14.0_f32.to_bits());
    assert_eq!(section.line_height.to_bits(), 19.0_f32.to_bits());
    assert_eq!(
        (
            store.len(),
            store.retained_payload_bytes(),
            store.change_cursor()
        ),
        accounting
    );
    assert!(output.values.is_empty());
    assert!(output.intents.is_empty());
    assert!(frame.semantics.nodes().iter().any(|node| {
        node.role == SemanticRole::Label && node.label.as_deref() == Some(source.as_str())
    }));
}

#[test]
fn section_row_preserves_geometry_primitive_order_and_semantic_topology() {
    let source = "Required section remains complete";
    let row = PropertyGridRow::section(ItemId::from_raw(51), source)
        .with_required(true)
        .with_help_text("Section help")
        .with_status(PropertyGridRowStatus::warning("Section warning"));
    let rows = [row];
    let theme = default_dark_theme();
    let mut store = TextLayoutStore::new();
    let mut memory = UiMemory::new();
    let (output, frame) = retained_grid(&mut store, &mut memory, &rows, BOUNDS, &theme);

    assert_eq!(
        output.visible_rows,
        PropertyGridConfig::default()
            .layout
            .visible_row_rects(BOUNDS, &rows, 0.0, 0)
    );
    assert!(output.values.is_empty());
    assert!(output.intents.is_empty());
    assert_eq!(frame.primitives.len(), 7);
    assert!(matches!(frame.primitives[0], Primitive::ClipBegin { .. }));
    assert!(matches!(frame.primitives[1], Primitive::Rect(_)));
    assert!(matches!(frame.primitives[2], Primitive::Rect(_)));
    assert!(matches!(frame.primitives[3], Primitive::Text(ref text) if text.text == source));
    assert!(matches!(frame.primitives[4], Primitive::Text(ref text) if text.text == "?"));
    assert!(matches!(frame.primitives[5], Primitive::Text(ref text) if text.text == "!"));
    assert!(matches!(frame.primitives[6], Primitive::ClipEnd { .. }));
    assert!(!frame.primitives.iter().any(
        |primitive| matches!(primitive, Primitive::Text(text) if text.text == format!("{source} *"))
    ));

    let section_node = frame
        .semantics
        .nodes()
        .iter()
        .find(|node| node.role == SemanticRole::Label && node.label.as_deref() == Some(source))
        .expect("section semantic node");
    assert_eq!(section_node.bounds, output.visible_rows[0].rect);
    assert!(
        section_node
            .description
            .as_deref()
            .expect("section description")
            .contains("Section help")
    );
}

#[test]
fn ordinary_label_overflow_help_status_and_rejection_contracts_remain_unchanged() {
    const RETAINED_PAYLOAD_CEILING: usize = 32 * 1024 * 1024;

    let source = "Ordinary property source remains complete while its presentation is elided";
    let row = PropertyGridRow::property(ItemId::from_raw(61), source, 0)
        .with_help_text("Property help")
        .with_status(PropertyGridRowStatus::error("Property error"));
    let bounds = Rect::new(0.0, 0.0, 96.0, 24.0);
    let theme = default_dark_theme();
    let mut store = TextLayoutStore::new();
    let mut memory = UiMemory::new();
    let (output, frame) = retained_grid(
        &mut store,
        &mut memory,
        std::slice::from_ref(&row),
        bounds,
        &theme,
    );

    assert_eq!(output.values.len(), 1);
    assert!(output.intents.is_empty());
    let label = text(&frame, source);
    let retained = store
        .stored_layout(layout_id(&frame, source))
        .expect("ordinary retained label");
    assert_eq!(retained.key.text.as_bytes(), source.as_bytes());
    assert_eq!(retained.key.style.weight, 400);
    assert_eq!(retained.key.style.features, TextFeatureSet::NONE);
    assert_eq!(retained.key.overflow, TextOverflow::EndEllipsis);
    assert!(!retained.key.wrap);
    assert_eq!(
        retained.key.width_bits,
        ((output.visible_rows[0].label_rect.width - 6.0_f32) - 22.0_f32)
            .max(0.0)
            .to_bits()
    );
    assert!(retained.layout.is_elided());
    assert_eq!(
        retained
            .layout
            .runs
            .iter()
            .flat_map(|run| &run.glyphs)
            .filter(|glyph| glyph.elided)
            .count(),
        1
    );
    assert_eq!(label.text.as_bytes(), source.as_bytes());
    for trailing in [text(&frame, "?"), text(&frame, "x")] {
        let stored = store
            .stored_layout(trailing.layout.expect("trailing glyph layout"))
            .expect("trailing glyph entry");
        assert_eq!(stored.key.style.weight, 400);
        assert_eq!(stored.key.overflow, TextOverflow::Visible);
    }
    assert!(matches!(frame.primitives[0], Primitive::ClipBegin { .. }));
    assert!(matches!(frame.primitives[1], Primitive::Rect(_)));
    assert!(matches!(frame.primitives[2], Primitive::Rect(_)));
    assert!(matches!(frame.primitives[3], Primitive::Text(ref text) if text.text == source));
    assert!(matches!(frame.primitives[4], Primitive::Text(ref text) if text.text == "?"));
    assert!(matches!(frame.primitives[5], Primitive::Text(ref text) if text.text == "x"));
    assert!(matches!(frame.primitives[6], Primitive::ClipEnd { .. }));
    assert!(
        frame.semantics.nodes().iter().any(|node| {
            node.role == SemanticRole::Row && node.label.as_deref() == Some(source)
        })
    );

    let rejection_source = "y".repeat(RETAINED_PAYLOAD_CEILING + 1);
    let accounting = (
        store.len(),
        store.retained_payload_bytes(),
        store.change_cursor(),
    );
    let (_, rejected) = retained_grid(
        &mut store,
        &mut memory,
        &[PropertyGridRow::property(
            ItemId::from_raw(62),
            &rejection_source,
            0,
        )],
        bounds,
        &theme,
    );
    assert_eq!(text(&rejected, &rejection_source).layout, None);
    assert_eq!(
        (
            store.len(),
            store.retained_payload_bytes(),
            store.change_cursor()
        ),
        accounting
    );
}
