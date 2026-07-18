//! Windowless conformance for retained virtual-table body-cell end ellipsis.

#![allow(clippy::too_many_lines)]

use std::{fs, path::Path, time::Duration};

use stern_core::{
    FrameContext, FrameOutput, Key, KeyEvent, KeyState, KeyboardInput, Modifiers, MouseButton,
    PhysicalSize, Point, PointerButtonState, PointerInput, PointerOrder, Primitive, Rect,
    ScaleFactor, SemanticRole, Size, TextPrimitive, TimeInfo, Transform, UiInput, UiInputEvent,
    UiMemory, Vec2, ViewportInfo, WidgetId, default_dark_theme,
};
use stern_text::{TextFeatureSet, TextLayoutStore, TextOverflow};
use stern_widgets::{
    CollectionProjectedItem, CollectionProjection, ItemId, SortDirection, TableColumn,
    TableColumnConstraints, TableLayout, TableSort, Ui, VirtualTableConfig, VirtualTableOutput,
    VirtualTableRow, VirtualTableSelection, VirtualTableSelectionMode, VirtualTableTarget,
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

fn exact_text<'a>(frame: &'a FrameOutput, source: &str) -> &'a TextPrimitive {
    frame
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            Primitive::Text(text) if text.text == source => Some(text),
            _ => None,
        })
        .unwrap_or_else(|| panic!("missing text primitive for {source:?}"))
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

fn pointer_input(point: Point, pressed: bool, released: bool) -> UiInput {
    UiInput {
        pointer: PointerInput {
            position: Some(point),
            primary: PointerButtonState::new(pressed, pressed, released),
            ..PointerInput::default()
        },
        ..UiInput::default()
    }
}

fn drag_input(point: Point, pressed: bool, delta_x: f32) -> UiInput {
    let mut input = UiInput::default();
    if pressed {
        input.push_event(UiInputEvent::PointerButton {
            button: MouseButton::Primary,
            down: true,
            click_count: 1,
            position: Some(point),
        });
    } else {
        input.pointer.primary = PointerButtonState::new(true, false, false);
        input.push_event(UiInputEvent::PointerMoved {
            position: point,
            delta: Vec2::new(delta_x, 0.0),
        });
    }
    input
}

fn key_input(key: Key) -> UiInput {
    UiInput {
        keyboard: KeyboardInput {
            modifiers: Modifiers::default(),
            events: vec![KeyEvent::new(
                key,
                KeyState::Pressed,
                Modifiers::default(),
                false,
            )],
        },
        ..UiInput::default()
    }
}

fn assert_header_visible_policy(store: &TextLayoutStore, frame: &FrameOutput, source: &str) {
    let text = exact_text(frame, source);
    let stored = store
        .stored_layout(text.layout.expect("generic retained header layout"))
        .expect("resident generic retained header layout");
    assert_eq!(stored.key.text, source);
    assert_eq!(stored.key.width_bits, 0.0_f32.to_bits());
    assert_eq!(stored.key.overflow, TextOverflow::Visible);
}

fn primitives_without_layout_ids(frame: &FrameOutput) -> Vec<Primitive> {
    let mut primitives = frame.primitives.clone();
    for primitive in &mut primitives {
        if let Primitive::Text(text) = primitive {
            text.layout = None;
        }
    }
    primitives
}

fn assert_layout_only_delta(retained: &Run, layoutless: &Run) {
    assert_eq!(retained.root, layoutless.root);
    assert_eq!(retained.output, layoutless.output);
    assert_eq!(retained.callbacks, layoutless.callbacks);
    assert_eq!(
        primitives_without_layout_ids(&retained.frame),
        primitives_without_layout_ids(&layoutless.frame)
    );
    assert_eq!(retained.frame.semantics, layoutless.frame.semantics);
    assert_eq!(retained.frame.repaint, layoutless.frame.repaint);
    assert_eq!(retained.frame.actions, layoutless.frame.actions);
    assert_eq!(
        retained.frame.platform_requests,
        layoutless.frame.platform_requests
    );
    assert_eq!(retained.frame.warnings, layoutless.frame.warnings);
}

fn collect_rust_sources(root: &Path, current: &Path, output: &mut Vec<(String, String)>) {
    for entry in fs::read_dir(current).expect("read production source directory") {
        let path = entry.expect("read production source entry").path();
        if path.is_dir() {
            collect_rust_sources(root, &path, output);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            let relative = path
                .strip_prefix(root)
                .expect("production source remains under manifest root")
                .to_string_lossy()
                .replace('\\', "/");
            output.push((
                relative,
                fs::read_to_string(path).expect("read UTF-8 production Rust source"),
            ));
        }
    }
}

fn production_rust_sources() -> Vec<(String, String)> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut sources = Vec::new();
    collect_rust_sources(root, &root.join("src"), &mut sources);
    sources.sort_by(|left, right| left.0.cmp(&right.0));
    sources
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
    let first_clamped_text = body_texts(&clamped_a.frame, source)[0];
    let same_effective_width_text = body_texts(&clamped_b.frame, source)[0];
    assert_eq!(first_clamped_text.layout, Some(first_ids[0]));
    assert_eq!(same_effective_width_text.layout, Some(first_ids[0]));
    assert_eq!(
        body_semantics(&clamped_a.frame, source)[0]
            .bounds
            .width
            .to_bits(),
        80.0_f32.to_bits()
    );
    assert_eq!(
        body_semantics(&clamped_b.frame, source)[0]
            .bounds
            .width
            .to_bits(),
        80.0_f32.to_bits()
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

#[test]
fn interaction_states_preserve_body_label_identity_and_layout_only_primitive_delta() {
    let source = "Complete stateful table body-cell source remains retained";
    let items = projection(&[1]);
    let table_config = || config(BOUNDS, [119.3], VirtualTableSelectionMode::Cell);
    let mut store = TextLayoutStore::new();

    let mut seed_memory = UiMemory::new();
    let mut seed_selection = VirtualTableSelection::new();
    let seed = run_table(
        Some(&mut store),
        &items,
        table_config(),
        &mut seed_selection,
        &mut seed_memory,
        UiInput::default(),
        |_| VirtualTableRow::new([source]),
    );
    let expected_id = body_texts(&seed.frame, source)[0]
        .layout
        .expect("seed stateful body-cell identity");
    let target = seed.output.selection_responses[0].response;
    let point = target.rect.center();
    let accounting = (
        store.len(),
        store.retained_payload_bytes(),
        store.change_cursor(),
    );

    for input in [
        UiInput::default(),
        pointer_input(point, false, false),
        pointer_input(point, true, false),
    ] {
        let mut retained_memory = UiMemory::new();
        let mut retained_selection = VirtualTableSelection::new();
        let retained = run_table(
            Some(&mut store),
            &items,
            table_config(),
            &mut retained_selection,
            &mut retained_memory,
            input.clone(),
            |_| VirtualTableRow::new([source]),
        );
        let mut layoutless_memory = UiMemory::new();
        let mut layoutless_selection = VirtualTableSelection::new();
        let layoutless = run_table(
            None,
            &items,
            table_config(),
            &mut layoutless_selection,
            &mut layoutless_memory,
            input,
            |_| VirtualTableRow::new([source]),
        );
        assert_eq!(
            body_texts(&retained.frame, source)[0].layout,
            Some(expected_id)
        );
        assert_layout_only_delta(&retained, &layoutless);
    }

    let mut retained_memory = UiMemory::new();
    retained_memory.focus(target.id);
    let mut retained_selection = VirtualTableSelection::new();
    let focused = run_table(
        Some(&mut store),
        &items,
        table_config(),
        &mut retained_selection,
        &mut retained_memory,
        UiInput::default(),
        |_| VirtualTableRow::new([source]),
    );
    let mut layoutless_memory = UiMemory::new();
    layoutless_memory.focus(target.id);
    let mut layoutless_selection = VirtualTableSelection::new();
    let focused_layoutless = run_table(
        None,
        &items,
        table_config(),
        &mut layoutless_selection,
        &mut layoutless_memory,
        UiInput::default(),
        |_| VirtualTableRow::new([source]),
    );
    assert!(focused.output.selection_responses[0].response.state.focused);
    assert_eq!(
        body_texts(&focused.frame, source)[0].layout,
        Some(expected_id)
    );
    assert_layout_only_delta(&focused, &focused_layoutless);

    let mut retained_memory = UiMemory::new();
    let mut retained_selection = VirtualTableSelection::new();
    let _ = run_table(
        Some(&mut store),
        &items,
        table_config(),
        &mut retained_selection,
        &mut retained_memory,
        pointer_input(point, true, false),
        |_| VirtualTableRow::new([source]),
    );
    let selected = run_table(
        Some(&mut store),
        &items,
        table_config(),
        &mut retained_selection,
        &mut retained_memory,
        pointer_input(point, false, true),
        |_| VirtualTableRow::new([source]),
    );
    let mut layoutless_memory = UiMemory::new();
    let mut layoutless_selection = VirtualTableSelection::new();
    let _ = run_table(
        None,
        &items,
        table_config(),
        &mut layoutless_selection,
        &mut layoutless_memory,
        pointer_input(point, true, false),
        |_| VirtualTableRow::new([source]),
    );
    let selected_layoutless = run_table(
        None,
        &items,
        table_config(),
        &mut layoutless_selection,
        &mut layoutless_memory,
        pointer_input(point, false, true),
        |_| VirtualTableRow::new([source]),
    );
    assert!(
        selected.output.selection_responses[0]
            .response
            .state
            .selected
    );
    assert_eq!(
        body_texts(&selected.frame, source)[0].layout,
        Some(expected_id)
    );
    assert_layout_only_delta(&selected, &selected_layoutless);

    let mut retained_memory = UiMemory::new();
    let mut retained_selection = VirtualTableSelection::new();
    let disabled = run_table(
        Some(&mut store),
        &items,
        table_config().disabled(true),
        &mut retained_selection,
        &mut retained_memory,
        pointer_input(point, true, false),
        |_| VirtualTableRow::new([source]),
    );
    let mut layoutless_memory = UiMemory::new();
    let mut layoutless_selection = VirtualTableSelection::new();
    let disabled_layoutless = run_table(
        None,
        &items,
        table_config().disabled(true),
        &mut layoutless_selection,
        &mut layoutless_memory,
        pointer_input(point, true, false),
        |_| VirtualTableRow::new([source]),
    );
    assert!(
        disabled.output.selection_responses[0]
            .response
            .state
            .disabled
    );
    assert_eq!(
        body_texts(&disabled.frame, source)[0].layout,
        Some(expected_id)
    );
    assert_layout_only_delta(&disabled, &disabled_layoutless);
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
fn headers_remain_complete_source_visible_consumers_through_focus_sort_narrow_and_resize() {
    let body = "Complete body-cell source stays separate from header policy";
    let items = projection(&[1]);
    let mut store = TextLayoutStore::new();
    let mut memory = UiMemory::new();
    let mut selection = VirtualTableSelection::new();
    let idle = run_table(
        Some(&mut store),
        &items,
        config(BOUNDS, [119.3], VirtualTableSelectionMode::Cell),
        &mut selection,
        &mut memory,
        UiInput::default(),
        |_| VirtualTableRow::new([body]),
    );
    assert_header_visible_policy(&store, &idle.frame, "Header 0");
    let body_text = exact_text(&idle.frame, body);
    let body_layout = store
        .stored_layout(body_text.layout.expect("retained body-cell layout"))
        .expect("resident retained body-cell layout");
    assert_eq!(body_layout.key.overflow, TextOverflow::EndEllipsis);

    let header_id = idle.output.headers[0].response.id;
    memory.focus(header_id);
    let focused = run_table(
        Some(&mut store),
        &items,
        config(BOUNDS, [119.3], VirtualTableSelectionMode::Cell),
        &mut selection,
        &mut memory,
        UiInput::default(),
        |_| VirtualTableRow::new([body]),
    );
    assert!(focused.output.headers[0].response.state.focused);
    assert_header_visible_policy(&store, &focused.frame, "Header 0");

    memory.clear_focus();
    let mut sorted_config = config(BOUNDS, [119.3], VirtualTableSelectionMode::Cell);
    sorted_config.layout.sort = Some(TableSort {
        column: id(10),
        direction: SortDirection::Ascending,
    });
    let sorted = run_table(
        Some(&mut store),
        &items,
        sorted_config,
        &mut selection,
        &mut memory,
        UiInput::default(),
        |_| VirtualTableRow::new([body]),
    );
    assert_header_visible_policy(&store, &sorted.frame, "Header 0 ↑");
    assert_eq!(sorted.output.sort_requested, None);

    let narrow = run_table(
        Some(&mut store),
        &items,
        config(BOUNDS, [1.0], VirtualTableSelectionMode::Cell),
        &mut selection,
        &mut memory,
        UiInput::default(),
        |_| VirtualTableRow::new([body]),
    );
    assert_header_visible_policy(&store, &narrow.frame, "Header 0");

    let resize_config = config(BOUNDS, [119.3], VirtualTableSelectionMode::Cell).resizable(true);
    let resize_seed = run_table(
        Some(&mut store),
        &items,
        resize_config.clone(),
        &mut selection,
        &mut memory,
        UiInput::default(),
        |_| VirtualTableRow::new([body]),
    );
    let handle = resize_seed.output.headers[0]
        .resize_response
        .expect("resize handle response");
    let point = handle.rect.center();
    let pressed = run_table(
        Some(&mut store),
        &items,
        resize_config.clone(),
        &mut selection,
        &mut memory,
        drag_input(point, true, 0.0),
        |_| VirtualTableRow::new([body]),
    );
    assert!(
        pressed.output.headers[0]
            .resize_response
            .expect("pressed resize response")
            .state
            .pressed
    );
    assert_header_visible_policy(&store, &pressed.frame, "Header 0");
    let moved = run_table(
        Some(&mut store),
        &items,
        resize_config,
        &mut selection,
        &mut memory,
        drag_input(Point::new(point.x + 12.0, point.y), false, 12.0),
        |_| VirtualTableRow::new([body]),
    );
    assert_eq!(
        moved.output.resize_requested.map(|request| request.column),
        Some(id(10))
    );
    assert_eq!(
        moved.output.resize_requested.map(|request| request.delta),
        Some(12.0)
    );
    assert_eq!(moved.output.sort_requested, None);
    assert_header_visible_policy(&store, &moved.frame, "Header 0");
}

#[test]
fn projection_reorder_and_both_navigation_modes_preserve_stable_semantic_identity() {
    for mode in [
        VirtualTableSelectionMode::Row,
        VirtualTableSelectionMode::Cell,
    ] {
        let source = projection(&[1, 2, 3]);
        let mut store = TextLayoutStore::new();
        let mut memory = UiMemory::new();
        let mut selection = VirtualTableSelection::new();
        let table_config = || config(BOUNDS, [80.0, 80.0], mode);
        let row = |item: CollectionProjectedItem| {
            VirtualTableRow::new([
                format!("Row {} first", item.id.raw()),
                format!("Row {} second", item.id.raw()),
            ])
        };
        let seed = run_table(
            Some(&mut store),
            &source,
            table_config(),
            &mut selection,
            &mut memory,
            UiInput::default(),
            row,
        );
        let first = seed.output.selection_responses[0].response;
        let point = first.rect.center();
        let _ = run_table(
            Some(&mut store),
            &source,
            table_config(),
            &mut selection,
            &mut memory,
            pointer_input(point, true, false),
            row,
        );
        let selected = run_table(
            Some(&mut store),
            &source,
            table_config(),
            &mut selection,
            &mut memory,
            pointer_input(point, false, true),
            row,
        );
        assert!(
            selected.output.selection_responses[0]
                .response
                .state
                .selected
        );

        let moved_down = run_table(
            Some(&mut store),
            &source,
            table_config(),
            &mut selection,
            &mut memory,
            key_input(Key::ArrowDown),
            row,
        );
        let expected_after_down = match mode {
            VirtualTableSelectionMode::Row => VirtualTableTarget::Row(id(2)),
            VirtualTableSelectionMode::Cell => VirtualTableTarget::Cell {
                row: id(2),
                column: id(10),
            },
        };
        assert_eq!(selection.target(), Some(expected_after_down));
        assert_eq!(
            moved_down.output.cursor_target.map(|cursor| cursor.target),
            Some(expected_after_down)
        );

        let expected_final = if mode == VirtualTableSelectionMode::Cell {
            let moved_right = run_table(
                Some(&mut store),
                &source,
                table_config(),
                &mut selection,
                &mut memory,
                key_input(Key::ArrowRight),
                row,
            );
            let target = VirtualTableTarget::Cell {
                row: id(2),
                column: id(11),
            };
            assert_eq!(
                moved_right.output.cursor_target.map(|cursor| cursor.target),
                Some(target)
            );
            target
        } else {
            expected_after_down
        };
        let final_label = if mode == VirtualTableSelectionMode::Cell {
            "Row 2 second"
        } else {
            "Row 2 first"
        };
        let semantic_before = body_semantics(&moved_down.frame, final_label)
            .first()
            .map(|node| node.id)
            .expect("semantic identity before projection reorder");

        let reordered_projection = projection(&[3, 2, 1]);
        let reordered = run_table(
            Some(&mut store),
            &reordered_projection,
            table_config(),
            &mut selection,
            &mut memory,
            UiInput::default(),
            row,
        );
        assert_eq!(selection.target(), Some(expected_final));
        let semantic_after = body_semantics(&reordered.frame, final_label);
        assert_eq!(semantic_after.len(), 1);
        assert_eq!(semantic_after[0].id, semantic_before);
        assert_eq!(semantic_after[0].label.as_deref(), Some(final_label));
        assert!(reordered.frame.actions.is_empty());
    }
}

#[test]
fn missing_cells_keep_empty_explicit_policy_and_extra_cells_remain_unpainted() {
    let items = projection(&[1]);
    let mut store = TextLayoutStore::new();
    let mut memory = UiMemory::new();
    let mut selection = VirtualTableSelection::new();
    let missing = run_table(
        Some(&mut store),
        &items,
        config(BOUNDS, [80.0, 80.0], VirtualTableSelectionMode::Cell),
        &mut selection,
        &mut memory,
        UiInput::default(),
        |_| VirtualTableRow::new(["Only caller-owned cell"]),
    );
    assert_eq!(
        body_semantics(&missing.frame, "Only caller-owned cell").len(),
        1
    );
    let empty_texts = body_texts(&missing.frame, "");
    let empty_semantics = body_semantics(&missing.frame, "");
    assert_eq!(empty_texts.len(), 1);
    assert_eq!(empty_semantics.len(), 1);
    let empty_layout = store
        .stored_layout(empty_texts[0].layout.expect("explicit missing-cell policy"))
        .expect("resident missing-cell policy");
    assert_eq!(empty_layout.key.text, "");
    assert_eq!(empty_layout.key.overflow, TextOverflow::EndEllipsis);
    assert!(!empty_layout.layout.is_elided());
    assert_eq!(marker_count(&store, empty_texts[0]), 0);

    let extra = "Caller extra cell must remain unpainted";
    let extra_run = run_table(
        Some(&mut store),
        &items,
        config(BOUNDS, [80.0, 80.0], VirtualTableSelectionMode::Cell),
        &mut selection,
        &mut memory,
        UiInput::default(),
        |_| VirtualTableRow::new(["First", "Second", extra]),
    );
    assert!(body_texts(&extra_run.frame, extra).is_empty());
    assert!(body_semantics(&extra_run.frame, extra).is_empty());
    assert!(store.layouts().all(|entry| entry.key.text != extra));
    assert_eq!(body_semantics(&extra_run.frame, "First").len(), 1);
    assert_eq!(body_semantics(&extra_run.frame, "Second").len(), 1);
}

#[test]
fn hundred_thousand_rows_keep_exact_bounded_materialization_and_layout_registration() {
    let raw_ids = (0..100_000).collect::<Vec<_>>();
    let items = projection(&raw_ids);
    let mut store = TextLayoutStore::new();
    let mut memory = UiMemory::new();
    let mut selection = VirtualTableSelection::new();
    let run = run_table(
        Some(&mut store),
        &items,
        config(BOUNDS, [80.0, 80.0, 80.0], VirtualTableSelectionMode::Cell),
        &mut selection,
        &mut memory,
        UiInput::default(),
        |item| {
            VirtualTableRow::new([
                format!("Row {} first", item.id.raw()),
                format!("Row {} second", item.id.raw()),
                format!("Row {} third", item.id.raw()),
            ])
        },
    );

    assert_eq!(run.output.window.body.visible_range, 0..4);
    assert_eq!(run.output.window.body.materialized_range, 0..5);
    assert_eq!(run.callbacks, [id(0), id(1), id(2), id(3), id(4)]);
    assert_eq!(run.output.rows.len(), 5);
    assert_eq!(
        run.frame
            .primitives
            .iter()
            .filter(|primitive| matches!(primitive, Primitive::Text(_)))
            .count(),
        18
    );
    assert_eq!(store.len(), 18);
    assert_eq!(
        store
            .layouts()
            .filter(|entry| entry.key.overflow == TextOverflow::EndEllipsis)
            .count(),
        15
    );
    assert_eq!(
        store
            .layouts()
            .filter(|entry| entry.key.overflow == TextOverflow::Visible)
            .count(),
        3
    );
    assert!(
        store
            .layouts()
            .all(|entry| !entry.key.text.starts_with("Row 99999 "))
    );
    for raw in 0..5 {
        for suffix in ["first", "second", "third"] {
            let source = format!("Row {raw} {suffix}");
            assert_eq!(body_texts(&run.frame, &source).len(), 1);
            assert_eq!(
                body_semantics(&run.frame, &source).len(),
                usize::from(raw < 4)
            );
        }
    }
}

#[test]
fn production_call_graph_limits_explicit_adoption_to_virtual_table_body_cells() {
    let sources = production_rust_sources();
    assert!(!sources.is_empty());
    let overflow_adopters = sources
        .iter()
        .filter_map(|(path, source)| {
            let count = source
                .matches("with_overflow(TextOverflow::EndEllipsis)")
                .count();
            (count > 0).then_some((path.as_str(), count))
        })
        .collect::<Vec<_>>();
    assert_eq!(
        overflow_adopters,
        vec![
            ("src/components/selector_fields.rs", 1),
            ("src/ui/basic_controls.rs", 2),
            ("src/ui/chrome.rs", 1),
            ("src/ui/property_grid.rs", 1),
            ("src/ui/virtual_table.rs", 1),
        ]
    );

    let virtual_table = sources
        .iter()
        .find(|(path, _)| path == "src/ui/virtual_table.rs")
        .map(|(_, source)| source)
        .expect("virtual-table production source");
    assert_eq!(
        virtual_table
            .matches("self.paint_virtual_table_text(rect, &label, recipe.foreground);")
            .count(),
        1
    );
    assert_eq!(
        virtual_table
            .matches("self.paint_virtual_table_body_text(rect, label, recipe.foreground);")
            .count(),
        1
    );
    assert_eq!(
        virtual_table
            .matches("fn paint_virtual_table_text(")
            .count(),
        1
    );
    assert_eq!(
        virtual_table
            .matches("fn paint_virtual_table_body_text(")
            .count(),
        1
    );
    assert_eq!(
        virtual_table
            .matches("self.virtual_table_text_primitive(rect, label, color)")
            .count(),
        2
    );
    assert_eq!(
        virtual_table
            .matches("self.primitive(Primitive::Text(text));")
            .count(),
        2
    );
    let header_boundary = virtual_table
        .split_once("fn paint_virtual_table_header(")
        .and_then(|(_, rest)| rest.split_once("fn paint_virtual_table_resize_handle("))
        .map(|(header, _)| header)
        .expect("bounded header painter source");
    assert!(header_boundary.contains("self.paint_virtual_table_text("));
    assert!(!header_boundary.contains("TextOverflow::EndEllipsis"));
    let body_boundary = virtual_table
        .split_once("fn paint_virtual_table_body_text(")
        .and_then(|(_, rest)| rest.split_once("fn virtual_table_text_primitive("))
        .map(|(body, _)| body)
        .expect("bounded body painter source");
    assert_eq!(
        body_boundary
            .matches(".with_overflow(TextOverflow::EndEllipsis)")
            .count(),
        1
    );
    assert!(body_boundary.contains("let raw_span = rect.width - padding_x * 2.0_f32;"));
    assert!(body_boundary.contains("let label_width = raw_span.max(0.0_f32);"));
    assert!(body_boundary.contains("self.primitive(Primitive::Text(text));"));
}
