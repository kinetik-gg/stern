//! Windowless conformance for retained virtual-table body-cell end ellipsis.

use std::time::Duration;

use stern_core::{
    FrameContext, FrameOutput, PhysicalSize, PointerOrder, Primitive, Rect, ScaleFactor,
    SemanticRole, Size, TextPrimitive, TimeInfo, Transform, UiInput, UiMemory, Vec2, ViewportInfo,
    WidgetId, default_dark_theme,
};
use stern_text::{TextFeatureSet, TextLayoutStore, TextOverflow};
use stern_widgets::{
    CollectionProjectedItem, CollectionProjection, ItemId, TableColumn, TableColumnConstraints,
    TableLayout, Ui, VirtualTableConfig, VirtualTableOutput, VirtualTableRow,
    VirtualTableSelection, VirtualTableSelectionMode,
};

const BOUNDS: Rect = Rect::new(7.0, 11.0, 320.0, 88.0);

fn id(raw: u64) -> ItemId {
    ItemId::from_raw(raw)
}

fn projection(raw_ids: &[u64]) -> CollectionProjection {
    CollectionProjection::from_source_ids(&raw_ids.iter().copied().map(id).collect::<Vec<_>>())
}

fn config(
    bounds: Rect,
    widths: impl IntoIterator<Item = f32>,
    mode: VirtualTableSelectionMode,
) -> VirtualTableConfig {
    let columns = widths
        .into_iter()
        .enumerate()
        .map(|(index, width)| {
            TableColumn::new(
                id(10 + u64::try_from(index).expect("fixture column index")),
                format!("Header {index}"),
                width,
            )
        })
        .collect();
    VirtualTableConfig::new(
        bounds,
        TableLayout {
            columns,
            header_height: 20.0,
            row_height: 20.0,
            sort: None,
        },
    )
    .label("Retained cell overflow fixture")
    .overscan(0)
    .selection_mode(mode)
    .resizable(false)
}

fn context(input: UiInput) -> FrameContext {
    FrameContext::new(
        ViewportInfo::new(
            Size::new(640.0, 360.0),
            PhysicalSize::new(640, 360),
            ScaleFactor::ONE,
        ),
        input,
        TimeInfo::new(Duration::from_millis(500), Duration::from_millis(16), 1),
    )
}

struct Run {
    root: WidgetId,
    output: VirtualTableOutput,
    callbacks: Vec<ItemId>,
    frame: FrameOutput,
}

fn run_table(
    store: Option<&mut TextLayoutStore>,
    projection: &CollectionProjection,
    config: VirtualTableConfig,
    selection: &mut VirtualTableSelection,
    memory: &mut UiMemory,
    input: UiInput,
    mut row: impl FnMut(CollectionProjectedItem) -> VirtualTableRow,
) -> Run {
    let theme = default_dark_theme();
    let mut ui = Ui::begin_frame(context(input), memory, &theme);
    if let Some(store) = store {
        ui = ui.with_text_layouts(store);
    }
    let table = ui
        .prepare_virtual_table("retained-cell-table", config, projection)
        .expect("valid retained table fixture");
    let root = table.widget_id();
    ui.resolve_pointer_targets(|plan| {
        table.declare_pointer_targets(plan, PointerOrder::new(100));
    })
    .expect("valid retained table pointer plan");
    let mut callbacks = Vec::new();
    let output = ui.virtual_table(&table, selection, |item| {
        callbacks.push(item.id);
        row(item)
    });
    Run {
        root,
        output,
        callbacks,
        frame: ui.finish_output(),
    }
}

fn body_texts<'a>(frame: &'a FrameOutput, source: &str) -> Vec<&'a TextPrimitive> {
    frame
        .primitives
        .iter()
        .filter_map(|primitive| match primitive {
            Primitive::Text(text) if text.text == source => Some(text),
            _ => None,
        })
        .collect()
}

fn body_semantics<'a>(frame: &'a FrameOutput, source: &str) -> Vec<&'a stern_core::SemanticNode> {
    frame
        .semantics
        .nodes()
        .iter()
        .filter(|node| node.role == SemanticRole::Cell && node.label.as_deref() == Some(source))
        .collect()
}

fn marker_count(store: &TextLayoutStore, text: &TextPrimitive) -> usize {
    store
        .stored_layout(text.layout.expect("registered body-cell label"))
        .expect("resident body-cell layout")
        .layout
        .runs
        .iter()
        .flat_map(|run| &run.glyphs)
        .filter(|glyph| glyph.elided)
        .count()
}

fn transforms(frame: &FrameOutput) -> Vec<Transform> {
    frame
        .primitives
        .iter()
        .filter_map(|primitive| match primitive {
            Primitive::TransformBegin(transform) => Some(*transform),
            _ => None,
        })
        .collect()
}

#[test]
fn exact_prepared_cell_width_matrix_preserves_formula_bits_and_pinned_endpoints() {
    let theme = default_dark_theme();
    assert_eq!(theme.controls.padding_x.to_bits(), 8.0_f32.to_bits());
    let cases = [
        (119.3_f32, 0x42CE_999A_u32, true),
        (80.0_f32, 0x4280_0000_u32, true),
        (16.0_f32, 0.0_f32.to_bits(), false),
        (15.999_f32, 0.0_f32.to_bits(), false),
        (1.0_f32, 0.0_f32.to_bits(), false),
    ];

    for (column_width, expected_bits, assert_endpoint) in cases {
        let source = format!("Exact prepared cell width {column_width:?}");
        let matrix_bounds = Rect::new(0.0, BOUNDS.y, BOUNDS.width, BOUNDS.height);
        let items = projection(&[1]);
        let mut store = TextLayoutStore::new();
        let mut memory = UiMemory::new();
        let mut selection = VirtualTableSelection::new();
        let run = run_table(
            Some(&mut store),
            &items,
            config(
                matrix_bounds,
                [column_width],
                VirtualTableSelectionMode::Cell,
            ),
            &mut selection,
            &mut memory,
            UiInput::default(),
            |_| VirtualTableRow::new([source.clone()]),
        );
        let texts = body_texts(&run.frame, &source);
        let semantics = body_semantics(&run.frame, &source);
        assert_eq!(texts.len(), 1);
        assert_eq!(semantics.len(), 1);
        let text = texts[0];
        let cell = semantics[0].bounds;
        assert_eq!(cell.width.to_bits(), column_width.to_bits());
        let stored = store
            .stored_layout(text.layout.expect("explicit cell layout"))
            .expect("resident explicit cell layout");
        let padding_x = theme.controls.padding_x;
        let raw_span = cell.width - padding_x * 2.0_f32;
        let label_width = raw_span.max(0.0_f32);
        assert_eq!(stored.key.width_bits, label_width.to_bits());
        assert_eq!(stored.key.width_bits, expected_bits);
        assert_eq!(stored.key.overflow, TextOverflow::EndEllipsis);
        assert_eq!(stored.key.text, source);
        if assert_endpoint {
            assert_eq!(
                (text.origin.x + label_width).to_bits(),
                (cell.max_x() - padding_x).to_bits()
            );
        } else {
            assert_eq!(label_width.to_bits(), 0.0_f32.to_bits());
        }
    }
}

#[test]
fn long_body_cells_in_both_selection_modes_keep_complete_source_and_one_marker() {
    let source = "Complete virtual-table body-cell source remains intact while presentation elides";

    for mode in [
        VirtualTableSelectionMode::Row,
        VirtualTableSelectionMode::Cell,
    ] {
        let items = projection(&[1]);
        let mut store = TextLayoutStore::new();
        let mut memory = UiMemory::new();
        let mut selection = VirtualTableSelection::new();
        let run = run_table(
            Some(&mut store),
            &items,
            config(BOUNDS, [80.0], mode),
            &mut selection,
            &mut memory,
            UiInput::default(),
            |_| VirtualTableRow::new([source]),
        );
        let texts = body_texts(&run.frame, source);
        let semantics = body_semantics(&run.frame, source);
        assert_eq!(texts.len(), 1);
        assert_eq!(semantics.len(), 1);
        let text = texts[0];
        let stored = store
            .stored_layout(text.layout.expect("explicit body-cell layout"))
            .expect("resident body-cell layout");

        assert_eq!(stored.key.text, source);
        assert_eq!(stored.key.style.family, text.family);
        assert_eq!(stored.key.style.size_bits, text.size.to_bits());
        assert_eq!(
            stored.key.style.line_height_bits,
            text.line_height.to_bits()
        );
        assert_eq!(stored.key.style.features, TextFeatureSet::NONE);
        assert!(!stored.key.wrap);
        assert_eq!(stored.key.overflow, TextOverflow::EndEllipsis);
        assert!(stored.layout.is_elided());
        assert_eq!(marker_count(&store, text), 1);
        assert_eq!(text.text, source);
        assert_eq!(semantics[0].label.as_deref(), Some(source));
        assert_eq!(selection.target(), None);
        assert_eq!(run.output.sort_requested, None);
        assert_eq!(run.output.resize_requested, None);
    }
}

#[test]
fn fitting_empty_and_layoutless_body_cells_keep_complete_sources() {
    for source in ["Fit", ""] {
        let items = projection(&[1]);
        let mut store = TextLayoutStore::new();
        let mut memory = UiMemory::new();
        let mut selection = VirtualTableSelection::new();
        let run = run_table(
            Some(&mut store),
            &items,
            config(BOUNDS, [119.3], VirtualTableSelectionMode::Cell),
            &mut selection,
            &mut memory,
            UiInput::default(),
            |_| VirtualTableRow::new([source]),
        );
        let texts = body_texts(&run.frame, source);
        let semantics = body_semantics(&run.frame, source);
        assert_eq!(texts.len(), 1);
        assert_eq!(semantics.len(), 1);
        let stored = store
            .stored_layout(texts[0].layout.expect("explicit fitting body-cell policy"))
            .expect("resident fitting body-cell policy");
        assert_eq!(stored.key.text, source);
        assert_eq!(stored.key.overflow, TextOverflow::EndEllipsis);
        assert!(!stored.layout.is_elided());
        assert_eq!(marker_count(&store, texts[0]), 0);
        assert_eq!(texts[0].text, source);
        assert_eq!(semantics[0].label.as_deref(), Some(source));
    }

    let source = "Layoutless table facade keeps the complete body-cell source";
    let items = projection(&[1]);
    let mut memory = UiMemory::new();
    let mut selection = VirtualTableSelection::new();
    let run = run_table(
        None,
        &items,
        config(BOUNDS, [80.0], VirtualTableSelectionMode::Cell),
        &mut selection,
        &mut memory,
        UiInput::default(),
        |_| VirtualTableRow::new([source]),
    );
    let texts = body_texts(&run.frame, source);
    let semantics = body_semantics(&run.frame, source);
    assert_eq!(texts.len(), 1);
    assert_eq!(texts[0].layout, None);
    assert_eq!(texts[0].text, source);
    assert_eq!(semantics.len(), 1);
    assert_eq!(semantics[0].label.as_deref(), Some(source));
}

#[test]
fn narrow_nonpositive_spans_and_paragraphs_keep_registered_full_source_policy() {
    for width in [16.0_f32, 15.999, 1.0] {
        let source = "Complete narrow table body-cell source remains present";
        let items = projection(&[1]);
        let mut store = TextLayoutStore::new();
        let mut memory = UiMemory::new();
        let mut selection = VirtualTableSelection::new();
        let run = run_table(
            Some(&mut store),
            &items,
            config(BOUNDS, [width], VirtualTableSelectionMode::Cell),
            &mut selection,
            &mut memory,
            UiInput::default(),
            |_| VirtualTableRow::new([source]),
        );
        let text = body_texts(&run.frame, source)[0];
        let stored = store
            .stored_layout(text.layout.expect("registered zero-span body-cell policy"))
            .expect("resident zero-span body-cell policy");
        assert_eq!(stored.key.width_bits, 0.0_f32.to_bits());
        assert_eq!(stored.key.overflow, TextOverflow::EndEllipsis);
        assert_eq!(stored.key.text, source);
        assert!(!stored.layout.is_elided());
        assert_eq!(marker_count(&store, text), 0);
        assert_eq!(text.text, source);
        assert_eq!(
            body_semantics(&run.frame, source)[0].label.as_deref(),
            Some(source)
        );
    }

    for source in [
        "First complete line\nSecond complete line",
        "First complete line\r\nSecond complete line",
        "First complete paragraph\u{2029}Second complete paragraph",
    ] {
        let items = projection(&[1]);
        let mut store = TextLayoutStore::new();
        let mut memory = UiMemory::new();
        let mut selection = VirtualTableSelection::new();
        let run = run_table(
            Some(&mut store),
            &items,
            config(BOUNDS, [119.3], VirtualTableSelectionMode::Row),
            &mut selection,
            &mut memory,
            UiInput::default(),
            |_| VirtualTableRow::new([source]),
        );
        let text = body_texts(&run.frame, source)[0];
        let stored = store
            .stored_layout(text.layout.expect("registered paragraph body-cell policy"))
            .expect("resident paragraph body-cell policy");
        assert_eq!(stored.key.text, source);
        assert_eq!(stored.key.overflow, TextOverflow::EndEllipsis);
        assert!(!stored.layout.is_elided());
        assert_eq!(marker_count(&store, text), 0);
        assert_eq!(text.text, source);
        assert_eq!(
            body_semantics(&run.frame, source)[0].label.as_deref(),
            Some(source)
        );
    }
}

#[test]
fn over_budget_source_rejects_custom_and_generic_layouts_without_store_mutation() {
    const RETAINED_PAYLOAD_CEILING: usize = 32 * 1024 * 1024;

    let items = projection(&[1]);
    let mut store = TextLayoutStore::new();
    let mut memory = UiMemory::new();
    let mut selection = VirtualTableSelection::new();
    let warm = run_table(
        Some(&mut store),
        &items,
        config(BOUNDS, [80.0], VirtualTableSelectionMode::Cell),
        &mut selection,
        &mut memory,
        UiInput::default(),
        |_| VirtualTableRow::new(["Warm retained table body-cell label"]),
    );
    assert!(
        body_texts(&warm.frame, "Warm retained table body-cell label")[0]
            .layout
            .is_some()
    );
    let accounting = (
        store.len(),
        store.retained_payload_bytes(),
        store.change_cursor(),
    );

    let source = "x".repeat(RETAINED_PAYLOAD_CEILING + 1);
    let rejected = run_table(
        Some(&mut store),
        &items,
        config(BOUNDS, [80.0], VirtualTableSelectionMode::Cell),
        &mut selection,
        &mut memory,
        UiInput::default(),
        |_| VirtualTableRow::new([source.clone()]),
    );
    let texts = body_texts(&rejected.frame, &source);
    let semantics = body_semantics(&rejected.frame, &source);
    assert_eq!(texts.len(), 1);
    assert_eq!(texts[0].layout, None);
    assert_eq!(texts[0].text, source);
    assert_eq!(semantics.len(), 1);
    assert_eq!(semantics[0].label.as_deref(), Some(source.as_str()));
    assert_eq!(
        (
            store.len(),
            store.retained_payload_bytes(),
            store.change_cursor()
        ),
        accounting
    );
    assert!(store.layouts().all(|entry| entry.key.text != source));
}

#[test]
fn hot_frames_source_width_and_clamped_width_obey_retained_identity_boundaries() {
    let source = "Stable complete table body-cell source across hot frames";
    let items = projection(&[1, 2]);
    let mut store = TextLayoutStore::new();
    let mut memory = UiMemory::new();
    let mut selection = VirtualTableSelection::new();
    let first = run_table(
        Some(&mut store),
        &items,
        config(BOUNDS, [80.0, 119.3], VirtualTableSelectionMode::Cell),
        &mut selection,
        &mut memory,
        UiInput::default(),
        |_| VirtualTableRow::new([source, source]),
    );
    let first_ids = body_texts(&first.frame, source)
        .iter()
        .map(|text| text.layout.expect("initial body-cell identity"))
        .collect::<Vec<_>>();
    assert_eq!(first_ids.len(), 4);
    assert_eq!(first_ids[0], first_ids[2]);
    assert_eq!(first_ids[1], first_ids[3]);
    assert_ne!(first_ids[0], first_ids[1]);
    let accounting = (
        store.len(),
        store.retained_payload_bytes(),
        store.change_cursor(),
    );

    for _ in 0..4 {
        let hot = run_table(
            Some(&mut store),
            &items,
            config(BOUNDS, [80.0, 119.3], VirtualTableSelectionMode::Cell),
            &mut selection,
            &mut memory,
            UiInput::default(),
            |_| VirtualTableRow::new([source, source]),
        );
        let hot_ids = body_texts(&hot.frame, source)
            .iter()
            .map(|text| text.layout.expect("hot body-cell identity"))
            .collect::<Vec<_>>();
        assert_eq!(hot_ids, first_ids);
        assert_eq!(
            (
                store.len(),
                store.retained_payload_bytes(),
                store.change_cursor()
            ),
            accounting
        );
    }

    let changed_source = "Distinct complete table body-cell source";
    let changed = run_table(
        Some(&mut store),
        &projection(&[1]),
        config(BOUNDS, [80.0], VirtualTableSelectionMode::Cell),
        &mut selection,
        &mut memory,
        UiInput::default(),
        |_| VirtualTableRow::new([changed_source]),
    );
    let changed_id = body_texts(&changed.frame, changed_source)[0]
        .layout
        .expect("changed-source body-cell identity");
    assert_ne!(changed_id, first_ids[0]);

    let resized = run_table(
        Some(&mut store),
        &projection(&[1]),
        config(BOUNDS, [100.0], VirtualTableSelectionMode::Cell),
        &mut selection,
        &mut memory,
        UiInput::default(),
        |_| VirtualTableRow::new([source]),
    );
    let resized_id = body_texts(&resized.frame, source)[0]
        .layout
        .expect("resized body-cell identity");
    assert_ne!(resized_id, first_ids[0]);

    let clamped_config = |raw_width| {
        config(BOUNDS, [raw_width], VirtualTableSelectionMode::Cell)
            .column_constraints([(id(10), TableColumnConstraints::new(80.0, 80.0))])
    };
    let clamped_a = run_table(
        Some(&mut store),
        &projection(&[1]),
        clamped_config(160.0),
        &mut selection,
        &mut memory,
        UiInput::default(),
        |_| VirtualTableRow::new([source]),
    );
    let clamped_b = run_table(
        Some(&mut store),
        &projection(&[1]),
        clamped_config(240.0),
        &mut selection,
        &mut memory,
        UiInput::default(),
        |_| VirtualTableRow::new([source]),
    );
    let clamped_a_text = body_texts(&clamped_a.frame, source)[0];
    let clamped_b_text = body_texts(&clamped_b.frame, source)[0];
    assert_eq!(clamped_a_text.layout, Some(first_ids[0]));
    assert_eq!(clamped_b_text.layout, Some(first_ids[0]));
    assert_eq!(
        body_semantics(&clamped_a.frame, source)[0].bounds.width,
        80.0
    );
    assert_eq!(
        body_semantics(&clamped_b.frame, source)[0].bounds.width,
        80.0
    );
}

#[test]
fn translation_and_fractional_horizontal_scroll_preserve_logical_width_and_identity() {
    let source = "Complete translated and horizontally scrolled body-cell source";
    let items = projection(&[1]);
    let mut store = TextLayoutStore::new();
    let mut memory = UiMemory::new();
    let mut selection = VirtualTableSelection::new();
    let seed = run_table(
        Some(&mut store),
        &items,
        config(BOUNDS, [200.0, 200.0], VirtualTableSelectionMode::Cell),
        &mut selection,
        &mut memory,
        UiInput::default(),
        |_| VirtualTableRow::new([source, source]),
    );
    let seed_texts = body_texts(&seed.frame, source);
    let seed_ids = seed_texts
        .iter()
        .map(|text| text.layout.expect("seed body-cell identity"))
        .collect::<Vec<_>>();
    assert_eq!(seed_ids.len(), 2);
    assert_eq!(seed_ids[0], seed_ids[1]);
    let width_bits = store
        .stored_layout(seed_ids[0])
        .expect("seed retained body-cell entry")
        .key
        .width_bits;
    let accounting = (
        store.len(),
        store.retained_payload_bytes(),
        store.change_cursor(),
    );

    let moved_bounds = Rect::new(
        BOUNDS.x + 40.0,
        BOUNDS.y + 20.0,
        BOUNDS.width,
        BOUNDS.height,
    );
    let moved = run_table(
        Some(&mut store),
        &items,
        config(
            moved_bounds,
            [200.0, 200.0],
            VirtualTableSelectionMode::Cell,
        ),
        &mut selection,
        &mut memory,
        UiInput::default(),
        |_| VirtualTableRow::new([source, source]),
    );
    let moved_texts = body_texts(&moved.frame, source);
    assert_eq!(
        moved_texts
            .iter()
            .map(|text| text.layout.expect("moved body-cell identity"))
            .collect::<Vec<_>>(),
        seed_ids
    );
    for (seed_text, moved_text) in seed_texts.iter().zip(moved_texts) {
        assert_eq!(
            (moved_text.origin.x - seed_text.origin.x).to_bits(),
            40.0_f32.to_bits()
        );
        assert_eq!(
            (moved_text.origin.y - seed_text.origin.y).to_bits(),
            20.0_f32.to_bits()
        );
    }
    assert_eq!(
        store
            .stored_layout(seed_ids[0])
            .expect("moved retained body-cell entry")
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

    memory.set_scroll_offset(seed.root, Vec2::new(30.25, 0.0));
    let scrolled = run_table(
        Some(&mut store),
        &items,
        config(BOUNDS, [200.0, 200.0], VirtualTableSelectionMode::Cell),
        &mut selection,
        &mut memory,
        UiInput::default(),
        |_| VirtualTableRow::new([source, source]),
    );
    let scrolled_texts = body_texts(&scrolled.frame, source);
    assert_eq!(scrolled.output.window.offset, Vec2::new(30.25, 0.0));
    assert_eq!(
        scrolled_texts
            .iter()
            .map(|text| text.layout.expect("scrolled body-cell identity"))
            .collect::<Vec<_>>(),
        seed_ids
    );
    assert_eq!(
        scrolled_texts
            .iter()
            .map(|text| text.origin)
            .collect::<Vec<_>>(),
        seed_texts
            .iter()
            .map(|text| text.origin)
            .collect::<Vec<_>>()
    );
    assert_eq!(transforms(&seed.frame).len(), 2);
    assert_eq!(transforms(&scrolled.frame).len(), 2);
    assert_eq!(transforms(&seed.frame)[0], Transform::IDENTITY);
    assert_eq!(transforms(&seed.frame)[1], Transform::IDENTITY);
    assert_eq!(
        transforms(&scrolled.frame)[0],
        Transform::translation(Vec2::new(-30.25, 0.0))
    );
    assert_eq!(
        transforms(&scrolled.frame)[1],
        Transform::translation(Vec2::new(-30.25, 0.0))
    );
    assert_eq!(
        (
            store.len(),
            store.retained_payload_bytes(),
            store.change_cursor()
        ),
        accounting
    );
}

#[test]
fn vertical_scroll_reuses_overlapping_cell_ids_and_preserves_exact_window_contract() {
    let raw_ids = (1..=10).collect::<Vec<_>>();
    let items = projection(&raw_ids);
    let mut store = TextLayoutStore::new();
    let mut memory = UiMemory::new();
    let mut selection = VirtualTableSelection::new();
    let seed = run_table(
        Some(&mut store),
        &items,
        config(BOUNDS, [119.3], VirtualTableSelectionMode::Row),
        &mut selection,
        &mut memory,
        UiInput::default(),
        |item| VirtualTableRow::new([format!("Vertical row {}", item.id.raw())]),
    );
    assert_eq!(seed.output.window.body.visible_range, 0..4);
    assert_eq!(seed.output.window.body.materialized_range, 0..5);
    assert_eq!(seed.callbacks, [id(1), id(2), id(3), id(4), id(5)]);
    assert_eq!(
        seed.output
            .rows
            .iter()
            .map(|row| row.id)
            .collect::<Vec<_>>(),
        seed.callbacks
    );

    let seed_ids = (1..=5)
        .map(|raw| {
            let source = format!("Vertical row {raw}");
            (
                raw,
                body_texts(&seed.frame, &source)[0]
                    .layout
                    .expect("seed vertical body-cell identity"),
            )
        })
        .collect::<Vec<_>>();
    memory.set_scroll_offset(seed.root, Vec2::new(0.0, 20.0));
    let scrolled = run_table(
        Some(&mut store),
        &items,
        config(BOUNDS, [119.3], VirtualTableSelectionMode::Row),
        &mut selection,
        &mut memory,
        UiInput::default(),
        |item| VirtualTableRow::new([format!("Vertical row {}", item.id.raw())]),
    );
    assert_eq!(scrolled.output.window.offset, Vec2::new(0.0, 20.0));
    assert_eq!(scrolled.output.window.body.visible_range, 1..5);
    assert_eq!(scrolled.output.window.body.materialized_range, 1..6);
    assert_eq!(scrolled.callbacks, [id(2), id(3), id(4), id(5), id(6)]);
    assert_eq!(
        scrolled
            .output
            .rows
            .iter()
            .map(|row| row.id)
            .collect::<Vec<_>>(),
        scrolled.callbacks
    );
    for (raw, expected_id) in seed_ids.into_iter().filter(|(raw, _)| *raw >= 2) {
        let source = format!("Vertical row {raw}");
        assert_eq!(
            body_texts(&scrolled.frame, &source)[0].layout,
            Some(expected_id)
        );
        assert_eq!(body_semantics(&scrolled.frame, &source).len(), 1);
    }
    assert_eq!(
        transforms(&scrolled.frame)[1].dy.to_bits(),
        (-20.0_f32).to_bits()
    );
}
