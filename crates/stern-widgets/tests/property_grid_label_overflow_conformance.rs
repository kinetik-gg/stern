//! Windowless conformance for retained property-label end ellipsis.

use std::collections::BTreeMap;

use stern_core::{
    FrameOutput, PointerInput, Primitive, Rect, SemanticRole, SpacingRole, TextLayoutId,
    TextPrimitive, Theme, UiInput, UiMemory, Vec2, WidgetId, default_dark_theme,
};
use stern_text::{TextLayoutStore, TextOverflow};
use stern_widgets::{
    ItemId, Ui,
    inspector::{
        PropertyGridAccess, PropertyGridConfig, PropertyGridLayout, PropertyGridOutput,
        PropertyGridRow, PropertyGridRowRect, PropertyGridRowStatus,
        property_grid_row_affordance_rects,
    },
};

const BOUNDS: Rect = Rect::new(10.0, 20.0, 360.0, 140.0);

#[derive(Debug, Clone, Copy, PartialEq)]
struct ValueObservation {
    row: ItemId,
    access: PropertyGridAccess,
    geometry: PropertyGridRowRect,
    value_rect: Rect,
    row_widget_id: WidgetId,
    value_widget_id: WidgetId,
}

fn layout(label_width: f32) -> PropertyGridLayout {
    PropertyGridLayout::new(24.0, 26.0, label_width, 6.0, 12.0)
}

fn retained_grid(
    store: &mut TextLayoutStore,
    memory: &mut UiMemory,
    rows: &[PropertyGridRow],
    bounds: Rect,
    config: PropertyGridConfig,
    input: &UiInput,
    theme: &Theme,
) -> (PropertyGridOutput<ValueObservation>, FrameOutput) {
    let mut ui = Ui::new(input, memory, theme).with_text_layouts(store);
    let output = ui
        .property_grid("grid", bounds, rows, config, |_, cell| ValueObservation {
            row: cell.row.id,
            access: cell.access,
            geometry: cell.geometry,
            value_rect: cell.value_rect,
            row_widget_id: cell.row_widget_id(),
            value_widget_id: cell.value_widget_id(),
        })
        .expect("valid property rows");
    (output, ui.finish_output())
}

fn retained_default(
    store: &mut TextLayoutStore,
    memory: &mut UiMemory,
    rows: &[PropertyGridRow],
) -> (PropertyGridOutput<ValueObservation>, FrameOutput) {
    retained_grid(
        store,
        memory,
        rows,
        BOUNDS,
        PropertyGridConfig::default(),
        &UiInput::default(),
        &default_dark_theme(),
    )
}

fn label_text<'a>(frame: &'a FrameOutput, source: &str) -> &'a TextPrimitive {
    frame
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            Primitive::Text(text) if text.text == source => Some(text),
            _ => None,
        })
        .expect("property label primitive")
}

fn marker_count(store: &TextLayoutStore, text: &TextPrimitive) -> usize {
    store
        .stored_layout(text.layout.expect("registered property label"))
        .expect("resident property label")
        .layout
        .runs
        .iter()
        .flat_map(|run| &run.glyphs)
        .filter(|glyph| glyph.elided)
        .count()
}

fn label_id(frame: &FrameOutput, source: &str) -> TextLayoutId {
    label_text(frame, source)
        .layout
        .expect("registered property label identity")
}

fn wheel_input(bounds: Rect, delta_y: f32) -> UiInput {
    UiInput {
        pointer: PointerInput {
            position: Some(bounds.center()),
            wheel_delta: Vec2::new(0.0, delta_y),
            ..PointerInput::default()
        },
        ..UiInput::default()
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn label_width_matrix_preserves_exact_operation_order_and_positive_zero() {
    let cases = [
        (
            "No trailing state reserves no width",
            PropertyGridRow::property(
                ItemId::from_raw(1),
                "No trailing state reserves no width",
                0,
            ),
            0.0_f32,
            0x42E2_999A_u32,
        ),
        (
            "Status reserves its fixed glyph origin",
            PropertyGridRow::property(
                ItemId::from_raw(2),
                "Status reserves its fixed glyph origin",
                0,
            )
            .with_status(PropertyGridRowStatus::warning("Warning")),
            10.0_f32,
            0x42CE_999A_u32,
        ),
        (
            "Help reserves its fixed glyph origin",
            PropertyGridRow::property(
                ItemId::from_raw(3),
                "Help reserves its fixed glyph origin",
                0,
            )
            .with_help_text("Help"),
            22.0_f32,
            0x42B6_999A_u32,
        ),
        (
            "Help wins over status reservation",
            PropertyGridRow::property(ItemId::from_raw(4), "Help wins over status reservation", 0)
                .with_help_text("Help")
                .with_status(PropertyGridRowStatus::error("Error")),
            22.0_f32,
            0x42B6_999A_u32,
        ),
    ];

    for (source, row, reserved_right, expected_bits) in cases {
        let config = PropertyGridConfig::new(layout(119.3)).with_overscan(0);
        let mut store = TextLayoutStore::new();
        let mut memory = UiMemory::new();
        let (output, frame) = retained_grid(
            &mut store,
            &mut memory,
            &[row],
            BOUNDS,
            config,
            &UiInput::default(),
            &default_dark_theme(),
        );
        let geometry = output.visible_rows[0];
        let label = label_text(&frame, source);
        let stored = store
            .stored_layout(label.layout.expect("explicit label layout"))
            .expect("resident label layout");
        let raw_span = (geometry.label_rect.width - 6.0_f32) - reserved_right;
        let expected_width = raw_span.max(0.0_f32);

        assert_eq!(geometry.label_rect.width.to_bits(), 119.3_f32.to_bits());
        assert_eq!(stored.key.width_bits, expected_width.to_bits());
        assert_eq!(stored.key.width_bits, expected_bits);
        assert_eq!(
            (label.origin.x + f32::from_bits(stored.key.width_bits)).to_bits(),
            (geometry.label_rect.max_x() - reserved_right).to_bits()
        );
        assert_eq!(stored.key.overflow, TextOverflow::EndEllipsis);
    }

    for (label_width, row) in [
        (
            5.0_f32,
            PropertyGridRow::property(ItemId::from_raw(11), "Tiny plain label", 0),
        ),
        (
            12.0_f32,
            PropertyGridRow::property(ItemId::from_raw(12), "Tiny status label", 0)
                .with_status(PropertyGridRowStatus::info("Info")),
        ),
        (
            20.0_f32,
            PropertyGridRow::property(ItemId::from_raw(13), "Tiny help label", 0)
                .with_help_text(""),
        ),
    ] {
        let reserved_right = if row.state.help_text.is_some() {
            22.0_f32
        } else if row.state.status.presentation().accented {
            10.0_f32
        } else {
            0.0_f32
        };
        let source = row.label.clone();
        let mut store = TextLayoutStore::new();
        let mut memory = UiMemory::new();
        let (output, frame) = retained_grid(
            &mut store,
            &mut memory,
            &[row],
            BOUNDS,
            PropertyGridConfig::new(layout(label_width)).with_overscan(0),
            &UiInput::default(),
            &default_dark_theme(),
        );
        let geometry = output.visible_rows[0];
        let raw_span = (geometry.label_rect.width - 6.0_f32) - reserved_right;
        assert!(raw_span <= 0.0);
        let label = label_text(&frame, &source);
        let stored = store
            .stored_layout(label.layout.expect("registered zero-width policy"))
            .expect("resident zero-width policy");
        assert_eq!(stored.key.width_bits, raw_span.max(0.0_f32).to_bits());
        assert_eq!(stored.key.width_bits, 0.0_f32.to_bits());
        assert_eq!(stored.key.overflow, TextOverflow::EndEllipsis);
    }
}

#[test]
fn ordinary_required_and_fitting_labels_preserve_complete_sources() {
    let long =
        "Complete ordinary property label source remains intact while its presentation elides";
    let required =
        "Complete required property label source keeps its presentation-only suffix while eliding";
    let rows = [
        PropertyGridRow::property(ItemId::from_raw(21), long, 0),
        PropertyGridRow::property(ItemId::from_raw(22), required, 0).with_required(true),
        PropertyGridRow::property(ItemId::from_raw(23), "Fit", 0),
    ];
    let mut store = TextLayoutStore::new();
    let mut memory = UiMemory::new();
    let (_, frame) = retained_default(&mut store, &mut memory, &rows);

    for (source, semantic, elided) in [
        (long.to_owned(), long, true),
        (format!("{required} *"), required, true),
        ("Fit".to_owned(), "Fit", false),
    ] {
        let label = label_text(&frame, &source);
        let stored = store
            .stored_layout(label.layout.expect("explicit property label layout"))
            .expect("resident property label layout");
        assert_eq!(label.text, source);
        assert_eq!(stored.key.text, source);
        assert_eq!(stored.key.style.family, label.family);
        assert_eq!(stored.key.style.size_bits, label.size.to_bits());
        assert_eq!(
            stored.key.style.line_height_bits,
            label.line_height.to_bits()
        );
        assert!(!stored.key.wrap);
        assert_eq!(stored.key.overflow, TextOverflow::EndEllipsis);
        assert_eq!(stored.layout.is_elided(), elided);
        assert_eq!(marker_count(&store, label), usize::from(elided));
        assert!(frame.semantics.nodes().iter().any(|node| {
            node.role == SemanticRole::Row && node.label.as_deref() == Some(semantic)
        }));
    }
}

#[test]
fn help_and_status_glyphs_keep_exact_reservations_and_visible_layouts() {
    let cases = [
        (
            PropertyGridRow::property(ItemId::from_raw(31), "Help only label source", 0)
                .with_help_text("Helpful detail"),
            22.0_f32,
            &["?"][..],
        ),
        (
            PropertyGridRow::property(ItemId::from_raw(32), "Empty help label source", 0)
                .with_help_text(""),
            22.0_f32,
            &["?"][..],
        ),
        (
            PropertyGridRow::property(ItemId::from_raw(33), "Info label source", 0)
                .with_status(PropertyGridRowStatus::info("Information")),
            10.0_f32,
            &["i"][..],
        ),
        (
            PropertyGridRow::property(ItemId::from_raw(34), "Warning label source", 0)
                .with_status(PropertyGridRowStatus::warning("Warning")),
            10.0_f32,
            &["!"][..],
        ),
        (
            PropertyGridRow::property(ItemId::from_raw(35), "Error label source", 0)
                .with_status(PropertyGridRowStatus::error("Error")),
            10.0_f32,
            &["x"][..],
        ),
        (
            PropertyGridRow::property(ItemId::from_raw(36), "Help and status label source", 0)
                .with_help_text("Help wins")
                .with_status(PropertyGridRowStatus::error("Error remains separate")),
            22.0_f32,
            &["?", "x"][..],
        ),
    ];

    for (row, reserved_right, glyphs) in cases {
        let source = row.label.clone();
        let mut store = TextLayoutStore::new();
        let mut memory = UiMemory::new();
        let (output, frame) = retained_default(&mut store, &mut memory, &[row]);
        let geometry = output.visible_rows[0];
        let label = label_text(&frame, &source);
        let label_id = label.layout.expect("registered property label");
        let stored = store
            .stored_layout(label_id)
            .expect("resident property label");
        assert_eq!(
            stored.key.width_bits,
            ((geometry.label_rect.width - 6.0_f32) - reserved_right)
                .max(0.0_f32)
                .to_bits()
        );
        assert_eq!(stored.key.text, source);
        assert_eq!(stored.key.overflow, TextOverflow::EndEllipsis);

        for glyph in glyphs {
            let primitive = label_text(&frame, glyph);
            let glyph_id = primitive.layout.expect("registered trailing glyph");
            let glyph_layout = store
                .stored_layout(glyph_id)
                .expect("resident trailing glyph");
            assert_ne!(glyph_id, label_id);
            assert_eq!(glyph_layout.key.text, *glyph);
            assert_eq!(glyph_layout.key.overflow, TextOverflow::Visible);
            assert!(!glyph_layout.layout.is_elided());
        }
    }
}

#[test]
fn severity_and_access_brush_changes_preserve_effective_label_identity() {
    let source = "Stable property source keeps identity across presentation-only brush changes";
    let mut store = TextLayoutStore::new();
    let mut memory = UiMemory::new();
    let mut severity_id = None;
    let mut severity_brushes = Vec::new();

    for status in [
        PropertyGridRowStatus::info("Info"),
        PropertyGridRowStatus::warning("Warning"),
        PropertyGridRowStatus::error("Error"),
    ] {
        let row = PropertyGridRow::property(ItemId::from_raw(41), source, 0).with_status(status);
        let (_, frame) = retained_default(&mut store, &mut memory, &[row]);
        let label = label_text(&frame, source);
        let id = label.layout.expect("severity label identity");
        assert_eq!(*severity_id.get_or_insert(id), id);
        severity_brushes.push(label.brush);
    }
    assert_ne!(severity_brushes[0], severity_brushes[1]);
    assert_ne!(severity_brushes[1], severity_brushes[2]);

    let mut access_id = None;
    let mut access_brushes = Vec::new();
    for row in [
        PropertyGridRow::property(ItemId::from_raw(42), source, 0),
        PropertyGridRow::property(ItemId::from_raw(42), source, 0).with_read_only(true),
        PropertyGridRow::property(ItemId::from_raw(42), source, 0).with_disabled(true),
    ] {
        let (_, frame) = retained_default(&mut store, &mut memory, &[row]);
        let label = label_text(&frame, source);
        let id = label.layout.expect("access label identity");
        assert_eq!(*access_id.get_or_insert(id), id);
        access_brushes.push(label.brush);
    }
    assert_eq!(access_brushes[0], access_brushes[1]);
    assert_ne!(access_brushes[1], access_brushes[2]);
}

#[test]
fn translation_and_scroll_change_origins_without_retained_identity_growth() {
    let source = "Translated property source retains exact width and identity";
    let row = PropertyGridRow::property(ItemId::from_raw(51), source, 0);
    let rows = [row.clone()];
    let mut store = TextLayoutStore::new();
    let mut memory = UiMemory::new();
    let (_, first) = retained_default(&mut store, &mut memory, &rows);
    let first_label = label_text(&first, source);
    let first_id = first_label.layout.expect("initial translated identity");
    let width_bits = store
        .stored_layout(first_id)
        .expect("initial translated layout")
        .key
        .width_bits;
    let accounting = (
        store.len(),
        store.retained_payload_bytes(),
        store.change_cursor(),
    );

    let translated = Rect::new(47.25, 91.5, BOUNDS.width, BOUNDS.height);
    let (_, second) = retained_grid(
        &mut store,
        &mut memory,
        &rows,
        translated,
        PropertyGridConfig::default(),
        &UiInput::default(),
        &default_dark_theme(),
    );
    let second_label = label_text(&second, source);
    assert_eq!(second_label.layout, Some(first_id));
    assert_ne!(second_label.origin, first_label.origin);
    assert_eq!(
        store
            .stored_layout(first_id)
            .expect("translated resident layout")
            .key
            .width_bits,
        width_bits
    );
    assert_eq!(
        (
            store.len(),
            store.retained_payload_bytes(),
            store.change_cursor()
        ),
        accounting
    );

    let scroll_bounds = Rect::new(0.0, 0.0, 300.0, 24.0);
    let scroll_config =
        PropertyGridConfig::new(PropertyGridLayout::new(100.0, 26.0, 120.0, 6.0, 12.0))
            .with_overscan(0);
    let mut scroll_store = TextLayoutStore::new();
    let mut scroll_memory = UiMemory::new();
    let (_, unscrolled) = retained_grid(
        &mut scroll_store,
        &mut scroll_memory,
        &rows,
        scroll_bounds,
        scroll_config,
        &UiInput::default(),
        &default_dark_theme(),
    );
    let unscrolled_label = label_text(&unscrolled, source);
    let scroll_id = unscrolled_label.layout.expect("initial scroll identity");
    let scroll_accounting = (
        scroll_store.len(),
        scroll_store.retained_payload_bytes(),
        scroll_store.change_cursor(),
    );
    let (_, scrolled) = retained_grid(
        &mut scroll_store,
        &mut scroll_memory,
        &rows,
        scroll_bounds,
        scroll_config,
        &wheel_input(scroll_bounds, -20.0),
        &default_dark_theme(),
    );
    let scrolled_label = label_text(&scrolled, source);
    assert_eq!(scrolled_label.layout, Some(scroll_id));
    assert_ne!(
        scrolled_label.origin.y.to_bits(),
        unscrolled_label.origin.y.to_bits()
    );
    assert_eq!(
        (
            scroll_store.len(),
            scroll_store.retained_payload_bytes(),
            scroll_store.change_cursor()
        ),
        scroll_accounting
    );
}

#[test]
fn icon_label_gap_customization_does_not_enter_property_label_identity() {
    let source = "Property label width ignores the icon-label spacing role";
    let row = PropertyGridRow::property(ItemId::from_raw(52), source, 0);
    let base_theme = default_dark_theme();
    let mut changed_theme = base_theme;
    changed_theme.spacing.two += 37.0;
    assert_ne!(
        base_theme
            .spacing
            .resolve(SpacingRole::IconLabelGap)
            .to_bits(),
        changed_theme
            .spacing
            .resolve(SpacingRole::IconLabelGap)
            .to_bits()
    );

    let mut store = TextLayoutStore::new();
    let mut memory = UiMemory::new();
    let (_, first) = retained_grid(
        &mut store,
        &mut memory,
        std::slice::from_ref(&row),
        BOUNDS,
        PropertyGridConfig::default(),
        &UiInput::default(),
        &base_theme,
    );
    let id = label_id(&first, source);
    let width_bits = store
        .stored_layout(id)
        .expect("base spacing layout")
        .key
        .width_bits;
    let (_, second) = retained_grid(
        &mut store,
        &mut memory,
        &[row],
        BOUNDS,
        PropertyGridConfig::default(),
        &UiInput::default(),
        &changed_theme,
    );
    assert_eq!(label_id(&second, source), id);
    assert_eq!(
        store
            .stored_layout(id)
            .expect("changed spacing layout")
            .key
            .width_bits,
        width_bits
    );
}

#[test]
fn hot_frames_reuse_identity_and_source_width_or_suffix_changes_do_not() {
    let source = "Stable property source remains retained across identical hot frames";
    let row = PropertyGridRow::property(ItemId::from_raw(61), source, 0);
    let mut store = TextLayoutStore::new();
    let mut memory = UiMemory::new();
    let (_, first) = retained_default(&mut store, &mut memory, std::slice::from_ref(&row));
    let stable_id = label_id(&first, source);
    let stable_accounting = (
        store.len(),
        store.retained_payload_bytes(),
        store.change_cursor(),
    );

    for _ in 0..4 {
        let (_, frame) = retained_default(&mut store, &mut memory, std::slice::from_ref(&row));
        assert_eq!(label_id(&frame, source), stable_id);
        assert_eq!(
            (
                store.len(),
                store.retained_payload_bytes(),
                store.change_cursor()
            ),
            stable_accounting
        );
    }

    let changed_source = "A distinct complete property source has distinct retained identity";
    let (_, changed) = retained_default(
        &mut store,
        &mut memory,
        &[PropertyGridRow::property(
            ItemId::from_raw(61),
            changed_source,
            0,
        )],
    );
    assert_ne!(label_id(&changed, changed_source), stable_id);

    let (_, resized) = retained_grid(
        &mut store,
        &mut memory,
        std::slice::from_ref(&row),
        BOUNDS,
        PropertyGridConfig::new(layout(133.25)),
        &UiInput::default(),
        &default_dark_theme(),
    );
    assert_ne!(label_id(&resized, source), stable_id);

    let required_source = format!("{source} *");
    let (_, required) = retained_default(&mut store, &mut memory, &[row.with_required(true)]);
    assert_ne!(label_id(&required, &required_source), stable_id);
}

#[test]
#[allow(clippy::too_many_lines)]
fn row_access_geometry_callbacks_order_and_semantics_remain_application_owned() {
    let rows = vec![
        PropertyGridRow::property(ItemId::from_raw(71), "Editable", 2),
        PropertyGridRow::property(ItemId::from_raw(72), "Read only", 1).with_read_only(true),
        PropertyGridRow::property(ItemId::from_raw(73), "Disabled", 0).with_disabled(true),
        PropertyGridRow::property(ItemId::from_raw(74), "Required", 0)
            .with_required(true)
            .with_resettable(true, false)
            .with_keyframeable(true, false),
    ];
    let rows_before = rows.clone();
    let config = PropertyGridConfig::default().with_overscan(0);
    let expected_geometry = config.layout.visible_row_rects(BOUNDS, &rows, 0.0, 0);
    let mut store = TextLayoutStore::new();
    let mut memory = UiMemory::new();
    let (output, frame) = retained_grid(
        &mut store,
        &mut memory,
        &rows,
        BOUNDS,
        config,
        &UiInput::default(),
        &default_dark_theme(),
    );

    assert_eq!(rows, rows_before);
    assert_eq!(output.visible_rows, expected_geometry);
    assert_eq!(output.values.len(), rows.len());
    assert!(output.intents.is_empty());
    assert_eq!(
        output
            .values
            .iter()
            .map(|value| value.value.access)
            .collect::<Vec<_>>(),
        vec![
            PropertyGridAccess::Editable,
            PropertyGridAccess::ReadOnly,
            PropertyGridAccess::Disabled,
            PropertyGridAccess::Editable,
        ]
    );
    for (value, geometry) in output.values.iter().zip(&expected_geometry) {
        let row = &rows[geometry.index];
        let expected_value_rect = property_grid_row_affordance_rects(
            row,
            geometry
                .value_rect
                .intersection(BOUNDS)
                .unwrap_or(Rect::ZERO)
                .inset(2.0)
                .max_zero(),
            config.affordances,
        )
        .value_rect;
        assert_eq!(value.row, value.value.row);
        assert_eq!(value.value.geometry, *geometry);
        assert_eq!(value.value.value_rect, expected_value_rect);
        assert_eq!(
            value.value.row_widget_id,
            output.root.child(("property-grid-row", value.row.raw()))
        );
        assert_eq!(
            value.value.value_widget_id,
            value.value.row_widget_id.child("value")
        );
    }
    for (row, geometry) in rows.iter().zip(&expected_geometry) {
        let presentation = if row.state.required {
            format!("{} *", row.label)
        } else {
            row.label.clone()
        };
        assert_eq!(
            label_text(&frame, &presentation).origin.x.to_bits(),
            (geometry.label_rect.x + 6.0_f32).to_bits()
        );
        assert!(frame.semantics.nodes().iter().any(|node| {
            node.id == output.root.child(("property-grid-row", row.id.raw()))
                && node.label.as_deref() == Some(row.label.as_str())
                && node.state.disabled == matches!(row.id.raw(), 73)
        }));
    }

    let original_ids = output
        .values
        .iter()
        .map(|value| {
            (
                value.row,
                (value.value.row_widget_id, value.value.value_widget_id),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let reordered = rows.iter().rev().cloned().collect::<Vec<_>>();
    let mut reorder_memory = UiMemory::new();
    let (reordered_output, _) = retained_grid(
        &mut store,
        &mut reorder_memory,
        &reordered,
        BOUNDS,
        config,
        &UiInput::default(),
        &default_dark_theme(),
    );
    assert_eq!(
        reordered_output
            .values
            .iter()
            .map(|value| (
                value.row,
                (value.value.row_widget_id, value.value.value_widget_id)
            ))
            .collect::<BTreeMap<_, _>>(),
        original_ids
    );
}

#[test]
fn offscreen_rows_do_not_register_layouts_and_sections_keep_explicit_retained_policy() {
    let rows = (0..6)
        .map(|index| {
            PropertyGridRow::property(
                ItemId::from_raw(80 + index),
                format!("Virtual property {index}"),
                0,
            )
        })
        .collect::<Vec<_>>();
    let viewport = Rect::new(0.0, 0.0, 300.0, 24.0);
    let config = PropertyGridConfig::default().with_overscan(0);
    let mut store = TextLayoutStore::new();
    let mut memory = UiMemory::new();
    let (output, frame) = retained_grid(
        &mut store,
        &mut memory,
        &rows,
        viewport,
        config,
        &wheel_input(viewport, -1_000.0),
        &default_dark_theme(),
    );
    assert_eq!(output.values.len(), 1);
    assert_eq!(output.values[0].row, ItemId::from_raw(85));
    assert_eq!(
        label_text(&frame, "Virtual property 5").text,
        "Virtual property 5"
    );
    let retained_sources = store
        .layouts()
        .map(|entry| entry.key.text.as_str())
        .collect::<Vec<_>>();
    assert_eq!(retained_sources, vec!["Virtual property 5"]);

    let section_source = "Long section title remains on its explicit retained visible path";
    let section =
        PropertyGridRow::section(ItemId::from_raw(90), section_source).with_required(true);
    let mut section_store = TextLayoutStore::new();
    let mut section_memory = UiMemory::new();
    let (section_output, section_frame) = retained_default(
        &mut section_store,
        &mut section_memory,
        std::slice::from_ref(&section),
    );
    assert!(section_output.values.is_empty());
    let section_text = label_text(&section_frame, section_source);
    assert_eq!(
        section_text.origin.x.to_bits(),
        (section_output.visible_rows[0].label_rect.x + 8.0_f32).to_bits()
    );
    let section_layout = section_store
        .stored_layout(
            section_text
                .layout
                .expect("explicit retained section layout"),
        )
        .expect("resident section layout");
    assert_eq!(section_layout.key.text, section_source);
    assert_eq!(section_layout.key.overflow, TextOverflow::Visible);

    let input = UiInput::default();
    let theme = default_dark_theme();
    let mut layoutless_memory = UiMemory::new();
    let mut ui = Ui::new(&input, &mut layoutless_memory, &theme);
    let _ = ui
        .property_grid(
            "grid",
            BOUNDS,
            &[section],
            PropertyGridConfig::default(),
            |_, _| (),
        )
        .expect("valid layoutless section");
    let layoutless_frame = ui.finish_output();
    assert_eq!(label_text(&layoutless_frame, section_source).layout, None);
}

#[test]
fn ineligible_widths_and_multiline_sources_keep_registered_full_source_policy() {
    let narrow_source = "Complete narrow property source remains visible";
    let narrow_row = PropertyGridRow::property(ItemId::from_raw(101), narrow_source, 0);
    let mut narrow_store = TextLayoutStore::new();
    let mut narrow_memory = UiMemory::new();
    let (narrow_output, narrow_frame) = retained_grid(
        &mut narrow_store,
        &mut narrow_memory,
        std::slice::from_ref(&narrow_row),
        BOUNDS,
        PropertyGridConfig::new(layout(5.0)).with_overscan(0),
        &UiInput::default(),
        &default_dark_theme(),
    );
    let narrow_label = label_text(&narrow_frame, narrow_source);
    let narrow_layout = narrow_store
        .stored_layout(narrow_label.layout.expect("registered narrow policy"))
        .expect("resident narrow policy");
    assert_eq!(narrow_layout.key.width_bits, 0.0_f32.to_bits());
    assert_eq!(narrow_layout.key.overflow, TextOverflow::EndEllipsis);
    assert_eq!(narrow_layout.key.text, narrow_source);
    assert!(!narrow_layout.layout.is_elided());
    assert_eq!(marker_count(&narrow_store, narrow_label), 0);
    assert!(narrow_frame.semantics.nodes().iter().any(|node| {
        node.label.as_deref() == Some(narrow_source)
            && node.bounds == narrow_output.visible_rows[0].rect
    }));

    for (index, source) in [
        "First complete line\nSecond complete line",
        "First complete line\r\nSecond complete line",
        "First complete paragraph\u{2029}Second complete paragraph",
    ]
    .into_iter()
    .enumerate()
    {
        let row = PropertyGridRow::property(ItemId::from_raw(102 + index as u64), source, 0);
        let mut store = TextLayoutStore::new();
        let mut memory = UiMemory::new();
        let (_, frame) = retained_default(&mut store, &mut memory, &[row]);
        let label = label_text(&frame, source);
        let stored = store
            .stored_layout(label.layout.expect("registered multiline policy"))
            .expect("resident multiline policy");
        assert_eq!(stored.key.text, source);
        assert_eq!(stored.key.overflow, TextOverflow::EndEllipsis);
        assert!(!stored.layout.is_elided());
        assert_eq!(marker_count(&store, label), 0);
        assert!(frame.semantics.nodes().iter().any(|node| {
            node.role == SemanticRole::Row && node.label.as_deref() == Some(source)
        }));
    }
}

#[test]
fn layoutless_ui_and_invalid_outer_bounds_preserve_fail_safe_output() {
    let source = "Complete layoutless property source";
    let row = PropertyGridRow::property(ItemId::from_raw(111), source, 0);
    let input = UiInput::default();
    let theme = default_dark_theme();
    let mut memory = UiMemory::new();
    let mut ui = Ui::new(&input, &mut memory, &theme);
    let output = ui
        .property_grid(
            "grid",
            BOUNDS,
            std::slice::from_ref(&row),
            PropertyGridConfig::default(),
            |_, cell| cell.row.id,
        )
        .expect("valid layoutless property row");
    let frame = ui.finish_output();
    assert_eq!(output.values[0].value, row.id);
    assert_eq!(label_text(&frame, source).layout, None);
    assert_eq!(label_text(&frame, source).text, source);
    assert!(
        frame.semantics.nodes().iter().any(|node| {
            node.role == SemanticRole::Row && node.label.as_deref() == Some(source)
        })
    );

    for bounds in [
        Rect::ZERO,
        Rect::new(0.0, 0.0, -1.0, 20.0),
        Rect::new(f32::NAN, 0.0, 20.0, 20.0),
        Rect::new(0.0, f32::INFINITY, 20.0, 20.0),
    ] {
        let mut store = TextLayoutStore::new();
        let mut memory = UiMemory::new();
        let (output, frame) = retained_grid(
            &mut store,
            &mut memory,
            std::slice::from_ref(&row),
            bounds,
            PropertyGridConfig::default(),
            &UiInput::default(),
            &default_dark_theme(),
        );
        assert!(output.visible_rows.is_empty());
        assert!(output.values.is_empty());
        assert!(output.intents.is_empty());
        assert!(frame.primitives.is_empty());
        assert!(frame.semantics.nodes().is_empty());
        assert!(store.is_empty());
    }
}

#[test]
fn over_budget_property_source_rejects_without_store_mutation_or_identity_leak() {
    const RETAINED_PAYLOAD_CEILING: usize = 32 * 1024 * 1024;

    let mut store = TextLayoutStore::new();
    let mut memory = UiMemory::new();
    let warm = PropertyGridRow::property(ItemId::from_raw(121), "Warm retained property", 0);
    let _ = retained_default(&mut store, &mut memory, &[warm]);
    let accounting = (
        store.len(),
        store.retained_payload_bytes(),
        store.change_cursor(),
    );

    let source = "x".repeat(RETAINED_PAYLOAD_CEILING + 1);
    let row = PropertyGridRow::property(ItemId::from_raw(122), &source, 0);
    let (output, frame) = retained_default(&mut store, &mut memory, &[row]);
    let label = label_text(&frame, &source);

    assert_eq!(label.layout, None);
    assert_eq!(label.text, source);
    assert_eq!(
        (
            store.len(),
            store.retained_payload_bytes(),
            store.change_cursor()
        ),
        accounting
    );
    assert_eq!(output.values.len(), 1);
    assert!(frame.semantics.nodes().iter().any(|node| {
        node.role == SemanticRole::Row && node.label.as_deref() == Some(source.as_str())
    }));
}
