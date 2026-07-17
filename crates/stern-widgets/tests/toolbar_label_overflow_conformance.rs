//! Windowless conformance for retained chrome-toolbar label end ellipsis.

use std::time::Duration;

use stern_core::{
    ActionContext, ActionDescriptor, FrameContext, FrameOutput, PhysicalSize, Point, PointerOrder,
    Primitive, Rect, ScaleFactor, Size, TextPrimitive, TimeInfo, UiInput, UiMemory, ViewportInfo,
    WidgetId, default_dark_theme,
};
use stern_text::{TextFeatureSet, TextLayoutStore, TextOverflow};
use stern_widgets::{
    ChromeScene, ChromeSceneConfig, ChromeSceneItemKey, ChromeSceneOutput, MenuBar, StatusBar,
    TabStrip, Toolbar, ToolbarGroup, ToolbarGroupId, Ui,
};

const GROUP: ToolbarGroupId = ToolbarGroupId::from_raw(41);
const ROOT: WidgetId = WidgetId::from_raw(0x724);
const BOUNDS: Rect = Rect::new(7.0, 11.0, 480.0, 28.0);

struct Run {
    output: ChromeSceneOutput,
    frame: FrameOutput,
    row_ids: Vec<WidgetId>,
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

fn key(action: &ActionDescriptor) -> ChromeSceneItemKey {
    ChromeSceneItemKey::Toolbar {
        group: GROUP,
        action: action.id.clone(),
    }
}

fn run_toolbar(
    store: Option<&mut TextLayoutStore>,
    memory: &mut UiMemory,
    bounds: Rect,
    actions: &[ActionDescriptor],
    widths: &[f32],
    input: UiInput,
) -> Run {
    assert_eq!(actions.len(), widths.len());
    let menu = MenuBar::new();
    let toolbar = Toolbar::from_groups([ToolbarGroup::from_actions(
        GROUP,
        "Conformance",
        actions.iter().cloned(),
    )]);
    let tabs = TabStrip::new();
    let status = StatusBar::new();
    let config = ChromeSceneConfig::new(
        ROOT,
        Rect::ZERO,
        bounds,
        Rect::ZERO,
        Rect::ZERO,
        ActionContext::Editor,
    )
    .with_overflow_trigger_width(20.0)
    .with_widths(
        actions
            .iter()
            .zip(widths)
            .map(|(action, width)| (key(action), *width)),
    );
    let scene = ChromeScene::new(config, &menu, &toolbar, &tabs, &status);
    let row_ids = actions
        .iter()
        .map(|action| scene.item_widget_id(&key(action)))
        .collect();
    let theme = default_dark_theme();
    let mut ui = Ui::begin_frame(context(input), memory, &theme);
    if let Some(store) = store {
        ui = ui.with_text_layouts(store);
    }
    ui.resolve_pointer_targets(|plan| {
        scene.declare_pointer_targets(plan, PointerOrder::new(100));
    })
    .expect("valid toolbar pointer plan");
    let output = ui.chrome_scene(&scene);
    Run {
        output,
        frame: ui.finish_output(),
        row_ids,
    }
}

fn toolbar_text<'a>(frame: &'a FrameOutput, source: &str) -> &'a TextPrimitive {
    frame
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            Primitive::Text(text) if text.text == source => Some(text),
            _ => None,
        })
        .unwrap_or_else(|| panic!("missing toolbar text {source:?}"))
}

fn marker_count(store: &TextLayoutStore, text: &TextPrimitive) -> usize {
    store
        .stored_layout(text.layout.expect("registered toolbar label"))
        .expect("resident toolbar label")
        .layout
        .runs
        .iter()
        .flat_map(|run| &run.glyphs)
        .filter(|glyph| glyph.elided)
        .count()
}

#[test]
fn exact_projected_width_matrix_preserves_formula_bits_and_positive_endpoint_equality() {
    let theme = default_dark_theme();
    assert_eq!(theme.controls.padding_x.to_bits(), 8.0_f32.to_bits());
    let cases = [
        (119.3_f32, 0x42CE_999A_u32),
        (80.0_f32, 0x4280_0000_u32),
        (16.0_f32, 0.0_f32.to_bits()),
        (15.999_f32, 0.0_f32.to_bits()),
        (1.0_f32, 0.0_f32.to_bits()),
    ];

    for (row_width, expected_bits) in cases {
        let action = ActionDescriptor::new("toolbar.width", "Exact toolbar label width");
        let mut store = TextLayoutStore::new();
        let mut memory = UiMemory::new();
        let run = run_toolbar(
            Some(&mut store),
            &mut memory,
            BOUNDS,
            &[action],
            &[row_width],
            UiInput::default(),
        );
        let text = toolbar_text(&run.frame, "Exact toolbar label width");
        let stored = store
            .stored_layout(text.layout.expect("explicit toolbar label layout"))
            .expect("resident toolbar label layout");
        let rect = run.output.responses[0].rect;
        let padding_x = theme.controls.padding_x;
        let raw_span = rect.width - padding_x * 2.0_f32;
        let label_width = raw_span.max(0.0_f32);

        assert_eq!(rect.width.to_bits(), row_width.to_bits());
        assert_eq!(stored.key.width_bits, label_width.to_bits());
        assert_eq!(stored.key.width_bits, expected_bits);
        assert_eq!(stored.key.overflow, TextOverflow::EndEllipsis);
        assert_eq!(stored.key.style.features, TextFeatureSet::NONE);
        assert!(!stored.key.wrap);
        if label_width.is_finite() && label_width > 0.0 {
            assert_eq!(
                (text.origin.x + label_width).to_bits(),
                (rect.max_x() - padding_x).to_bits()
            );
        } else {
            assert_eq!(label_width.to_bits(), 0.0_f32.to_bits());
        }
    }
}

#[test]
fn long_fitting_and_empty_labels_preserve_complete_source_and_explicit_policy() {
    let cases = [
        (
            "Complete chrome toolbar source remains intact while its retained presentation elides",
            true,
        ),
        ("Fit", false),
        ("", false),
    ];

    for (source, should_elide) in cases {
        let action = ActionDescriptor::new("toolbar.source", source);
        let mut store = TextLayoutStore::new();
        let mut memory = UiMemory::new();
        let run = run_toolbar(
            Some(&mut store),
            &mut memory,
            BOUNDS,
            &[action.clone()],
            &[80.0],
            UiInput::default(),
        );
        let text = toolbar_text(&run.frame, source);
        let id = text.layout.expect("explicit toolbar label identity");
        let stored = store
            .stored_layout(id)
            .expect("resident toolbar label identity");

        assert_eq!(text.text, source);
        assert_eq!(stored.key.text, source);
        assert_eq!(stored.key.style.family, text.family);
        assert_eq!(stored.key.style.size_bits, text.size.to_bits());
        assert_eq!(
            stored.key.style.line_height_bits,
            text.line_height.to_bits()
        );
        assert_eq!(stored.key.overflow, TextOverflow::EndEllipsis);
        assert_eq!(stored.layout.is_elided(), should_elide);
        assert_eq!(marker_count(&store, text), usize::from(should_elide));
        assert_eq!(action.label, source);
        let semantic = run
            .frame
            .semantics
            .get(run.row_ids[0])
            .expect("complete toolbar semantic");
        assert_eq!(semantic.label.as_deref(), Some(source));
        assert!(
            semantic
                .actions
                .iter()
                .any(|entry| entry.action_id.as_ref() == Some(&action.id))
        );
        assert!(run.frame.actions.is_empty());
        assert!(run.frame.warnings.is_empty());
    }
}

#[test]
fn narrow_spans_and_paragraph_sources_keep_registered_full_source_policy() {
    for row_width in [16.0_f32, 15.999, 1.0] {
        let source = "Complete narrow toolbar source remains visible";
        let action = ActionDescriptor::new("toolbar.narrow", source);
        let mut store = TextLayoutStore::new();
        let mut memory = UiMemory::new();
        let run = run_toolbar(
            Some(&mut store),
            &mut memory,
            BOUNDS,
            &[action],
            &[row_width],
            UiInput::default(),
        );
        let text = toolbar_text(&run.frame, source);
        let stored = store
            .stored_layout(text.layout.expect("registered narrow toolbar policy"))
            .expect("resident narrow toolbar policy");

        assert_eq!(stored.key.width_bits, 0.0_f32.to_bits());
        assert_eq!(stored.key.overflow, TextOverflow::EndEllipsis);
        assert_eq!(stored.key.text, source);
        assert!(!stored.layout.is_elided());
        assert_eq!(marker_count(&store, text), 0);
        assert_eq!(text.text, source);
        assert_eq!(
            run.frame
                .semantics
                .get(run.row_ids[0])
                .unwrap()
                .label
                .as_deref(),
            Some(source)
        );
    }

    for source in [
        "First complete line\nSecond complete line",
        "First complete line\r\nSecond complete line",
        "First complete paragraph\u{2029}Second complete paragraph",
    ] {
        let action = ActionDescriptor::new("toolbar.paragraph", source);
        let mut store = TextLayoutStore::new();
        let mut memory = UiMemory::new();
        let run = run_toolbar(
            Some(&mut store),
            &mut memory,
            BOUNDS,
            &[action],
            &[80.0],
            UiInput::default(),
        );
        let text = toolbar_text(&run.frame, source);
        let stored = store
            .stored_layout(text.layout.expect("registered paragraph toolbar policy"))
            .expect("resident paragraph toolbar policy");

        assert_eq!(stored.key.text, source);
        assert_eq!(stored.key.overflow, TextOverflow::EndEllipsis);
        assert!(!stored.layout.is_elided());
        assert_eq!(marker_count(&store, text), 0);
        assert_eq!(text.text, source);
        assert_eq!(
            run.frame
                .semantics
                .get(run.row_ids[0])
                .unwrap()
                .label
                .as_deref(),
            Some(source)
        );
    }
}

#[test]
fn rejected_and_layoutless_labels_preserve_complete_source_without_identity_leaks() {
    const RETAINED_PAYLOAD_CEILING: usize = 32 * 1024 * 1024;

    let mut store = TextLayoutStore::new();
    let mut memory = UiMemory::new();
    let warm = ActionDescriptor::new("toolbar.warm", "Warm retained toolbar label");
    let _ = run_toolbar(
        Some(&mut store),
        &mut memory,
        BOUNDS,
        &[warm],
        &[80.0],
        UiInput::default(),
    );
    let accounting = (
        store.len(),
        store.retained_payload_bytes(),
        store.change_cursor(),
    );

    let source = "x".repeat(RETAINED_PAYLOAD_CEILING + 1);
    let rejected = ActionDescriptor::new("toolbar.rejected", &source);
    let run = run_toolbar(
        Some(&mut store),
        &mut memory,
        BOUNDS,
        &[rejected],
        &[80.0],
        UiInput::default(),
    );
    let text = toolbar_text(&run.frame, &source);

    assert_eq!(text.layout, None);
    assert_eq!(text.text, source);
    assert_eq!(
        (
            store.len(),
            store.retained_payload_bytes(),
            store.change_cursor()
        ),
        accounting
    );
    assert_eq!(
        run.frame
            .semantics
            .get(run.row_ids[0])
            .unwrap()
            .label
            .as_deref(),
        Some(source.as_str())
    );
    assert!(run.frame.actions.is_empty());
    assert!(run.frame.warnings.is_empty());

    let layoutless_source = "Layoutless complete chrome toolbar source";
    let layoutless = ActionDescriptor::new("toolbar.layoutless", layoutless_source);
    let mut layoutless_memory = UiMemory::new();
    let run = run_toolbar(
        None,
        &mut layoutless_memory,
        BOUNDS,
        &[layoutless],
        &[80.0],
        UiInput::default(),
    );
    assert_eq!(toolbar_text(&run.frame, layoutless_source).layout, None);
    assert_eq!(
        run.frame
            .semantics
            .get(run.row_ids[0])
            .unwrap()
            .label
            .as_deref(),
        Some(layoutless_source)
    );
}

#[test]
fn hot_translation_source_and_width_obey_retained_identity_boundaries() {
    let source = "Stable complete toolbar source remains retained across hot frames";
    let action = ActionDescriptor::new("toolbar.stable", source);
    let mut store = TextLayoutStore::new();
    let mut memory = UiMemory::new();
    let first = run_toolbar(
        Some(&mut store),
        &mut memory,
        BOUNDS,
        std::slice::from_ref(&action),
        &[80.0],
        UiInput::default(),
    );
    let first_text = toolbar_text(&first.frame, source);
    let first_id = first_text.layout.expect("initial toolbar identity");
    let first_origin = first_text.origin;
    let first_rect = first.output.responses[0].rect;
    let first_width_bits = store
        .stored_layout(first_id)
        .expect("initial toolbar entry")
        .key
        .width_bits;
    let accounting = (
        store.len(),
        store.retained_payload_bytes(),
        store.change_cursor(),
    );

    for _ in 0..4 {
        let hot = run_toolbar(
            Some(&mut store),
            &mut memory,
            BOUNDS,
            std::slice::from_ref(&action),
            &[80.0],
            UiInput::default(),
        );
        assert_eq!(toolbar_text(&hot.frame, source).layout, Some(first_id));
        assert_eq!(
            (
                store.len(),
                store.retained_payload_bytes(),
                store.change_cursor()
            ),
            accounting
        );
    }

    let delta = Point::new(40.0, 20.0);
    let translated_bounds = Rect::new(
        BOUNDS.x + delta.x,
        BOUNDS.y + delta.y,
        BOUNDS.width,
        BOUNDS.height,
    );
    let moved = run_toolbar(
        Some(&mut store),
        &mut memory,
        translated_bounds,
        std::slice::from_ref(&action),
        &[80.0],
        UiInput::default(),
    );
    let moved_text = toolbar_text(&moved.frame, source);
    let moved_rect = moved.output.responses[0].rect;
    assert_eq!(moved_text.layout, Some(first_id));
    assert_eq!(
        (moved_text.origin.x - first_origin.x).to_bits(),
        delta.x.to_bits()
    );
    assert_eq!(
        (moved_text.origin.y - first_origin.y).to_bits(),
        delta.y.to_bits()
    );
    assert_eq!((moved_rect.x - first_rect.x).to_bits(), delta.x.to_bits());
    assert_eq!((moved_rect.y - first_rect.y).to_bits(), delta.y.to_bits());
    assert_eq!(moved_rect.width.to_bits(), first_rect.width.to_bits());
    assert_eq!(moved_rect.height.to_bits(), first_rect.height.to_bits());
    assert_eq!(first.frame.primitives.len(), moved.frame.primitives.len());
    assert_eq!(
        store
            .stored_layout(first_id)
            .expect("translated toolbar entry")
            .key
            .width_bits,
        first_width_bits
    );
    assert_eq!(
        (
            store.len(),
            store.retained_payload_bytes(),
            store.change_cursor()
        ),
        accounting
    );

    let changed_source = "Distinct complete toolbar source receives a distinct retained identity";
    let changed = ActionDescriptor::new("toolbar.stable", changed_source);
    let changed_run = run_toolbar(
        Some(&mut store),
        &mut memory,
        BOUNDS,
        &[changed],
        &[80.0],
        UiInput::default(),
    );
    let changed_id = toolbar_text(&changed_run.frame, changed_source)
        .layout
        .expect("changed-source toolbar identity");
    assert_ne!(changed_id, first_id);

    let resized = run_toolbar(
        Some(&mut store),
        &mut memory,
        BOUNDS,
        &[action],
        &[100.0],
        UiInput::default(),
    );
    let resized_id = toolbar_text(&resized.frame, source)
        .layout
        .expect("resized toolbar identity");
    assert_ne!(resized_id, first_id);
    assert_ne!(resized_id, changed_id);
    assert_ne!(
        store
            .stored_layout(resized_id)
            .expect("resized toolbar entry")
            .key
            .width_bits,
        first_width_bits
    );
}
