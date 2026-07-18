//! Windowless conformance for retained standard-button label end ellipsis.

use std::{fs, path::Path, time::Duration};

use stern_core::{
    ActionContext, ActionDescriptor, ActionId, ActionSource, CursorShape, FrameOutput, ImageId,
    Key, KeyEvent, KeyState, KeyboardInput, Modifiers, MouseButton, PathElement, PlatformRequest,
    Point, PointerButtonState, PointerInput, Primitive, Rect, RepaintRequest, Response,
    SemanticRole, Shortcut, TextPrimitive, UiInput, UiMemory, WidgetId, default_dark_theme,
};
use stern_text::{TextFeatureSet, TextLayoutStore, TextOverflow};
use stern_widgets::chrome::{SystemFeedbackScene, SystemFeedbackSceneConfig};
use stern_widgets::{
    ChromeScene, ChromeSceneConfig, ChromeSceneItemKey, DiagnosticStrip, FeedbackStack, JobCancel,
    JobList, JobPhase, JobRow, JobRowId, MenuBar, MenuBarMenu, MenuBarMenuId, StatusBar, TabStrip,
    Toolbar, ToolbarGroup, ToolbarGroupId, Ui, button,
};

const BUTTON: Rect = Rect::new(7.0, 11.0, 119.3, 28.0);

fn retained_button(
    store: &mut TextLayoutStore,
    memory: &mut UiMemory,
    rect: Rect,
    source: &str,
    disabled: bool,
    input: &UiInput,
) -> (Response, FrameOutput) {
    let theme = default_dark_theme();
    let mut ui = Ui::new(input, memory, &theme).with_text_layouts(store);
    let response = ui.button("retained-button", rect, source, disabled);
    (response, ui.finish_output())
}

fn button_text<'a>(frame: &'a FrameOutput, source: &str) -> &'a TextPrimitive {
    frame
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            Primitive::Text(text) if text.text == source => Some(text),
            _ => None,
        })
        .expect("standard button label primitive")
}

fn marker_count(store: &TextLayoutStore, text: &TextPrimitive) -> usize {
    store
        .stored_layout(text.layout.expect("registered button label"))
        .expect("resident button label")
        .layout
        .runs
        .iter()
        .flat_map(|run| &run.glyphs)
        .filter(|glyph| glyph.elided)
        .count()
}

fn pointer_transition(down: bool) -> UiInput {
    let mut input = UiInput::default();
    input.pointer.position = Some(BUTTON.center());
    input
        .pointer
        .apply_button_transition(MouseButton::Primary, down);
    input
}

fn assert_rect_bits(left: Rect, right: Rect) {
    assert_eq!(left.x.to_bits(), right.x.to_bits());
    assert_eq!(left.y.to_bits(), right.y.to_bits());
    assert_eq!(left.width.to_bits(), right.width.to_bits());
    assert_eq!(left.height.to_bits(), right.height.to_bits());
}

fn assert_point_translation(source: Point, translated: Point, delta: Point) {
    assert_eq!(translated.x.to_bits(), (source.x + delta.x).to_bits());
    assert_eq!(translated.y.to_bits(), (source.y + delta.y).to_bits());
}

fn assert_path_element_translation(source: &PathElement, translated: &PathElement, delta: Point) {
    match (source, translated) {
        (PathElement::MoveTo(source), PathElement::MoveTo(translated))
        | (PathElement::LineTo(source), PathElement::LineTo(translated)) => {
            assert_point_translation(*source, *translated, delta);
        }
        (
            PathElement::QuadTo {
                ctrl: source_ctrl,
                to: source_to,
            },
            PathElement::QuadTo {
                ctrl: translated_ctrl,
                to: translated_to,
            },
        ) => {
            assert_point_translation(*source_ctrl, *translated_ctrl, delta);
            assert_point_translation(*source_to, *translated_to, delta);
        }
        (
            PathElement::CubicTo {
                ctrl1: source_ctrl1,
                ctrl2: source_ctrl2,
                to: source_to,
            },
            PathElement::CubicTo {
                ctrl1: translated_ctrl1,
                ctrl2: translated_ctrl2,
                to: translated_to,
            },
        ) => {
            assert_point_translation(*source_ctrl1, *translated_ctrl1, delta);
            assert_point_translation(*source_ctrl2, *translated_ctrl2, delta);
            assert_point_translation(*source_to, *translated_to, delta);
        }
        (PathElement::Close, PathElement::Close) => {}
        _ => panic!("translated focus path changed element topology"),
    }
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
fn exact_width_matrix_preserves_formula_bits_and_positive_endpoint_equality() {
    let theme = default_dark_theme();
    assert_eq!(theme.controls.padding_x.to_bits(), 8.0_f32.to_bits());
    let cases = [
        (119.3_f32, 0x42CE_999A_u32),
        (80.0_f32, 0x4280_0000_u32),
        (16.0_f32, 0.0_f32.to_bits()),
        (15.999_f32, 0.0_f32.to_bits()),
        (1.0_f32, 0.0_f32.to_bits()),
    ];

    for (rect_width, expected_bits) in cases {
        let rect = Rect::new(BUTTON.x, BUTTON.y, rect_width, BUTTON.height);
        let mut store = TextLayoutStore::new();
        let mut memory = UiMemory::new();
        let (_, frame) = retained_button(
            &mut store,
            &mut memory,
            rect,
            "Exact button label width",
            false,
            &UiInput::default(),
        );
        let label = button_text(&frame, "Exact button label width");
        let stored = store
            .stored_layout(label.layout.expect("explicit button label layout"))
            .expect("resident button label layout");
        let raw_span = rect.width - theme.controls.padding_x * 2.0_f32;
        let label_width = raw_span.max(0.0_f32);

        assert_eq!(stored.key.width_bits, label_width.to_bits());
        assert_eq!(stored.key.width_bits, expected_bits);
        assert_eq!(stored.key.overflow, TextOverflow::EndEllipsis);
        if label_width.is_finite() && label_width > 0.0 {
            assert_eq!(
                (label.origin.x + label_width).to_bits(),
                (rect.max_x() - theme.controls.padding_x).to_bits()
            );
        }
    }
}

#[test]
fn long_standard_button_registers_complete_source_and_one_end_marker() {
    let source =
        "Complete standard button source remains intact while its retained presentation elides";
    let mut store = TextLayoutStore::new();
    let mut memory = UiMemory::new();
    let (response, frame) = retained_button(
        &mut store,
        &mut memory,
        BUTTON,
        source,
        false,
        &UiInput::default(),
    );
    let label = button_text(&frame, source);
    let id = label.layout.expect("explicit retained button layout");
    let stored = store
        .stored_layout(id)
        .expect("resident retained button layout");

    assert_eq!(stored.key.text, source);
    assert_eq!(stored.key.style.family, label.family);
    assert_eq!(stored.key.style.size_bits, label.size.to_bits());
    assert_eq!(
        stored.key.style.line_height_bits,
        label.line_height.to_bits()
    );
    assert_eq!(stored.key.style.features, TextFeatureSet::NONE);
    assert!(!stored.key.wrap);
    assert_eq!(stored.key.overflow, TextOverflow::EndEllipsis);
    assert!(stored.layout.is_elided());
    assert_eq!(marker_count(&store, label), 1);
    assert_eq!(label.text, source);
    assert_eq!(response.rect, BUTTON);
    assert_eq!(frame.semantics.nodes().len(), 1);
    assert_eq!(frame.semantics.nodes()[0].id, response.id);
    assert_eq!(frame.semantics.nodes()[0].role, SemanticRole::Button);
    assert_eq!(frame.semantics.nodes()[0].label.as_deref(), Some(source));
    assert!(frame.warnings.is_empty());
}

#[test]
fn fitting_empty_layoutless_and_direct_buttons_keep_complete_sources() {
    for source in ["Fit", ""] {
        let mut store = TextLayoutStore::new();
        let mut memory = UiMemory::new();
        let (_, frame) = retained_button(
            &mut store,
            &mut memory,
            BUTTON,
            source,
            false,
            &UiInput::default(),
        );
        let label = button_text(&frame, source);
        let stored = store
            .stored_layout(label.layout.expect("explicit fitting button policy"))
            .expect("resident fitting button policy");
        assert_eq!(stored.key.text, source);
        assert_eq!(stored.key.overflow, TextOverflow::EndEllipsis);
        assert!(!stored.layout.is_elided());
        assert_eq!(marker_count(&store, label), 0);
        assert_eq!(label.text, source);
        assert_eq!(frame.semantics.nodes()[0].label.as_deref(), Some(source));
    }

    let source = "Layoutless retained facade keeps the complete button source";
    let theme = default_dark_theme();
    let input = UiInput::default();
    let mut memory = UiMemory::new();
    let mut ui = Ui::new(&input, &mut memory, &theme);
    let response = ui.button("layoutless", BUTTON, source, false);
    let frame = ui.finish_output();
    assert_eq!(button_text(&frame, source).layout, None);
    assert_eq!(frame.semantics.nodes()[0].label.as_deref(), Some(source));
    assert_eq!(response.rect, BUTTON);

    let direct = button(
        WidgetId::from_key("direct-button"),
        Rect::new(1.0, 2.0, 8.0, 20.0),
        source,
        &UiInput::default(),
        &mut UiMemory::new(),
        &theme,
        false,
    );
    let direct_label = direct
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            Primitive::Text(text) => Some(text),
            _ => None,
        })
        .expect("direct button label");
    assert_eq!(direct_label.text, source);
    assert_eq!(direct_label.layout, None);
    assert_eq!(direct.semantics[0].label.as_deref(), Some(source));
    assert_eq!(direct_label.origin, Point::new(9.0, direct_label.origin.y));
}

#[test]
fn narrow_nonpositive_and_multiline_labels_keep_registered_full_source_policy() {
    for width in [16.0_f32, 15.999, 1.0, 0.0, -20.0] {
        let source = "Complete narrow button source remains visible";
        let rect = Rect::new(BUTTON.x, BUTTON.y, width, BUTTON.height);
        let mut store = TextLayoutStore::new();
        let mut memory = UiMemory::new();
        let (_, frame) = retained_button(
            &mut store,
            &mut memory,
            rect,
            source,
            false,
            &UiInput::default(),
        );
        let label = button_text(&frame, source);
        let stored = store
            .stored_layout(label.layout.expect("registered zero-width button policy"))
            .expect("resident zero-width button policy");

        assert_eq!(stored.key.width_bits, 0.0_f32.to_bits());
        assert_eq!(stored.key.overflow, TextOverflow::EndEllipsis);
        assert_eq!(stored.key.text, source);
        assert!(!stored.layout.is_elided());
        assert_eq!(marker_count(&store, label), 0);
        assert_eq!(label.text, source);
        assert_eq!(frame.semantics.nodes()[0].label.as_deref(), Some(source));
    }

    for source in [
        "First complete line\nSecond complete line",
        "First complete line\r\nSecond complete line",
        "First complete paragraph\u{2029}Second complete paragraph",
    ] {
        let mut store = TextLayoutStore::new();
        let mut memory = UiMemory::new();
        let (_, frame) = retained_button(
            &mut store,
            &mut memory,
            BUTTON,
            source,
            false,
            &UiInput::default(),
        );
        let label = button_text(&frame, source);
        let stored = store
            .stored_layout(label.layout.expect("registered multiline button policy"))
            .expect("resident multiline button policy");

        assert_eq!(stored.key.text, source);
        assert_eq!(stored.key.overflow, TextOverflow::EndEllipsis);
        assert!(!stored.layout.is_elided());
        assert_eq!(marker_count(&store, label), 0);
        assert_eq!(label.text, source);
        assert_eq!(frame.semantics.nodes()[0].label.as_deref(), Some(source));
    }
}

#[test]
fn over_budget_source_rejects_without_store_mutation_or_identity_leak() {
    const RETAINED_PAYLOAD_CEILING: usize = 32 * 1024 * 1024;

    let mut store = TextLayoutStore::new();
    let mut memory = UiMemory::new();
    let _ = retained_button(
        &mut store,
        &mut memory,
        BUTTON,
        "Warm retained button label",
        false,
        &UiInput::default(),
    );
    let accounting = (
        store.len(),
        store.retained_payload_bytes(),
        store.change_cursor(),
    );

    let source = "x".repeat(RETAINED_PAYLOAD_CEILING + 1);
    let (response, frame) = retained_button(
        &mut store,
        &mut memory,
        BUTTON,
        &source,
        false,
        &UiInput::default(),
    );
    let label = button_text(&frame, &source);

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
    assert_eq!(frame.primitives.len(), 2);
    assert_eq!(frame.semantics.nodes().len(), 1);
    assert_eq!(frame.semantics.nodes()[0].id, response.id);
    assert_eq!(
        frame.semantics.nodes()[0].label.as_deref(),
        Some(source.as_str())
    );
    assert!(frame.actions.is_empty());
    assert!(frame.warnings.is_empty());
}

#[test]
#[allow(clippy::too_many_lines)]
fn hot_frames_translation_source_and_width_obey_retained_identity_boundaries() {
    let source = "Stable complete button source remains retained across hot frames";
    let mut store = TextLayoutStore::new();
    let mut memory = UiMemory::new();
    let (_, first) = retained_button(
        &mut store,
        &mut memory,
        BUTTON,
        source,
        false,
        &UiInput::default(),
    );
    let first_label = button_text(&first, source);
    let first_id = first_label
        .layout
        .expect("initial retained button identity");
    let first_origin = first_label.origin;
    let first_width_bits = store
        .stored_layout(first_id)
        .expect("initial retained button entry")
        .key
        .width_bits;
    let accounting = (
        store.len(),
        store.retained_payload_bytes(),
        store.change_cursor(),
    );

    for _ in 0..4 {
        let (_, frame) = retained_button(
            &mut store,
            &mut memory,
            BUTTON,
            source,
            false,
            &UiInput::default(),
        );
        assert_eq!(button_text(&frame, source).layout, Some(first_id));
        assert_eq!(
            (
                store.len(),
                store.retained_payload_bytes(),
                store.change_cursor()
            ),
            accounting
        );
    }

    let translated = Rect::new(
        BUTTON.x + 40.0,
        BUTTON.y + 20.0,
        BUTTON.width,
        BUTTON.height,
    );
    let (_, moved) = retained_button(
        &mut store,
        &mut memory,
        translated,
        source,
        false,
        &UiInput::default(),
    );
    let moved_label = button_text(&moved, source);
    assert_eq!(moved_label.layout, Some(first_id));
    assert_eq!(
        store
            .stored_layout(first_id)
            .expect("translated retained button entry")
            .key
            .width_bits,
        first_width_bits
    );
    assert_eq!(
        (moved_label.origin.x - first_origin.x).to_bits(),
        40.0_f32.to_bits()
    );
    assert_eq!(
        (moved_label.origin.y - first_origin.y).to_bits(),
        20.0_f32.to_bits()
    );
    assert_eq!(
        (
            store.len(),
            store.retained_payload_bytes(),
            store.change_cursor()
        ),
        accounting
    );

    let changed_source = "Distinct complete button source receives distinct retained identity";
    let (_, changed) = retained_button(
        &mut store,
        &mut memory,
        BUTTON,
        changed_source,
        false,
        &UiInput::default(),
    );
    let changed_id = button_text(&changed, changed_source)
        .layout
        .expect("changed-source button identity");
    assert_ne!(changed_id, first_id);

    let wider = Rect::new(BUTTON.x, BUTTON.y, BUTTON.width + 20.0, BUTTON.height);
    let (_, resized) = retained_button(
        &mut store,
        &mut memory,
        wider,
        source,
        false,
        &UiInput::default(),
    );
    let resized_id = button_text(&resized, source)
        .layout
        .expect("resized button identity");
    assert_ne!(resized_id, first_id);
    assert_ne!(resized_id, changed_id);
    assert_ne!(
        store
            .stored_layout(resized_id)
            .expect("resized retained button entry")
            .key
            .width_bits,
        first_width_bits
    );
}

#[test]
#[allow(clippy::too_many_lines)]
fn focused_retained_button_translates_complete_surface_and_focus_geometry_only() {
    let source = "Focused retained button translates without changing label identity";
    let delta = Point::new(40.0, 20.0);
    let translated_rect = Rect::new(
        BUTTON.x + delta.x,
        BUTTON.y + delta.y,
        BUTTON.width,
        BUTTON.height,
    );
    let mut store = TextLayoutStore::new();
    let mut initial_memory = UiMemory::new();
    let (initial_response, _) = retained_button(
        &mut store,
        &mut initial_memory,
        BUTTON,
        source,
        false,
        &UiInput::default(),
    );

    let mut source_memory = UiMemory::new();
    source_memory.focus(initial_response.id);
    let (source_response, source_frame) = retained_button(
        &mut store,
        &mut source_memory,
        BUTTON,
        source,
        false,
        &UiInput::default(),
    );
    let source_label = button_text(&source_frame, source);
    let retained_id = source_label.layout.expect("focused retained label");
    let width_bits = store
        .stored_layout(retained_id)
        .expect("focused retained entry")
        .key
        .width_bits;
    let accounting = (
        store.len(),
        store.retained_payload_bytes(),
        store.change_cursor(),
    );

    let mut translated_memory = UiMemory::new();
    translated_memory.focus(initial_response.id);
    let (translated_response, translated_frame) = retained_button(
        &mut store,
        &mut translated_memory,
        translated_rect,
        source,
        false,
        &UiInput::default(),
    );
    let translated_label = button_text(&translated_frame, source);

    assert!(source_response.state.focused);
    assert!(translated_response.state.focused);
    assert_eq!(source_response.id, translated_response.id);
    assert_point_translation(
        Point::new(source_response.rect.x, source_response.rect.y),
        Point::new(translated_response.rect.x, translated_response.rect.y),
        delta,
    );
    assert_eq!(
        source_response.rect.width.to_bits(),
        translated_response.rect.width.to_bits()
    );
    assert_eq!(
        source_response.rect.height.to_bits(),
        translated_response.rect.height.to_bits()
    );
    assert_eq!(source_frame.primitives.len(), 4);
    assert_eq!(translated_frame.primitives.len(), 4);

    for (source_primitive, translated_primitive) in source_frame
        .primitives
        .iter()
        .zip(&translated_frame.primitives)
    {
        match (source_primitive, translated_primitive) {
            (Primitive::Rect(source), Primitive::Rect(translated)) => {
                assert_point_translation(
                    Point::new(source.rect.x, source.rect.y),
                    Point::new(translated.rect.x, translated.rect.y),
                    delta,
                );
                assert_eq!(source.rect.width.to_bits(), translated.rect.width.to_bits());
                assert_eq!(
                    source.rect.height.to_bits(),
                    translated.rect.height.to_bits()
                );
                assert_eq!(source.fill, translated.fill);
                assert_eq!(source.stroke, translated.stroke);
                assert_eq!(source.radius, translated.radius);
            }
            (Primitive::Path(source), Primitive::Path(translated)) => {
                assert_eq!(source.elements.len(), 20);
                assert_eq!(translated.elements.len(), 20);
                assert_eq!(source.fill, translated.fill);
                assert_eq!(source.stroke, translated.stroke);
                for (source, translated) in source.elements.iter().zip(&translated.elements) {
                    assert_path_element_translation(source, translated, delta);
                }
            }
            (Primitive::Text(source), Primitive::Text(translated)) => {
                assert_point_translation(source.origin, translated.origin, delta);
                assert_eq!(source.layout, translated.layout);
                assert_eq!(source.text, translated.text);
                assert_eq!(source.family, translated.family);
                assert_eq!(source.size.to_bits(), translated.size.to_bits());
                assert_eq!(
                    source.line_height.to_bits(),
                    translated.line_height.to_bits()
                );
                assert_eq!(source.brush, translated.brush);
            }
            _ => panic!("translated focused button changed primitive topology"),
        }
    }

    assert_eq!(translated_label.layout, Some(retained_id));
    assert_eq!(
        store
            .stored_layout(retained_id)
            .expect("translated focused retained entry")
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
    assert_eq!(source_frame.semantics.nodes().len(), 1);
    assert_eq!(translated_frame.semantics.nodes().len(), 1);
    let source_semantic = &source_frame.semantics.nodes()[0];
    let translated_semantic = &translated_frame.semantics.nodes()[0];
    assert_point_translation(
        Point::new(source_semantic.bounds.x, source_semantic.bounds.y),
        Point::new(translated_semantic.bounds.x, translated_semantic.bounds.y),
        delta,
    );
    assert_eq!(
        source_semantic.bounds.width.to_bits(),
        translated_semantic.bounds.width.to_bits()
    );
    assert_eq!(
        source_semantic.bounds.height.to_bits(),
        translated_semantic.bounds.height.to_bits()
    );
    let mut normalized = translated_semantic.clone();
    normalized.bounds = source_semantic.bounds;
    assert_eq!(*source_semantic, normalized);
}

#[test]
#[allow(clippy::too_many_lines)]
fn interaction_states_preserve_label_identity_and_existing_surface_order() {
    let source = "Complete stateful button source retains one presentation identity";
    let mut store = TextLayoutStore::new();

    let mut default_memory = UiMemory::new();
    let (default_response, default_frame) = retained_button(
        &mut store,
        &mut default_memory,
        BUTTON,
        source,
        false,
        &UiInput::default(),
    );
    let expected_id = button_text(&default_frame, source)
        .layout
        .expect("default button label identity");

    let hover_input = UiInput {
        pointer: PointerInput {
            position: Some(BUTTON.center()),
            ..PointerInput::default()
        },
        ..UiInput::default()
    };
    let mut hover_memory = UiMemory::new();
    let (hover_response, hover_frame) = retained_button(
        &mut store,
        &mut hover_memory,
        BUTTON,
        source,
        false,
        &hover_input,
    );

    let pressed_input = UiInput {
        pointer: PointerInput {
            position: Some(BUTTON.center()),
            primary: PointerButtonState::new(true, true, false),
            ..PointerInput::default()
        },
        ..UiInput::default()
    };
    let mut pressed_memory = UiMemory::new();
    let (pressed_response, pressed_frame) = retained_button(
        &mut store,
        &mut pressed_memory,
        BUTTON,
        source,
        false,
        &pressed_input,
    );

    let mut focused_memory = UiMemory::new();
    focused_memory.focus(default_response.id);
    let (focused_response, focused_frame) = retained_button(
        &mut store,
        &mut focused_memory,
        BUTTON,
        source,
        false,
        &UiInput::default(),
    );

    let mut disabled_memory = UiMemory::new();
    let (disabled_response, disabled_frame) = retained_button(
        &mut store,
        &mut disabled_memory,
        BUTTON,
        source,
        true,
        &hover_input,
    );

    assert!(!default_response.state.hovered);
    assert!(hover_response.state.hovered);
    assert!(pressed_response.state.pressed);
    assert!(focused_response.state.focused);
    assert!(disabled_response.state.disabled);
    assert!(!disabled_response.state.hovered);
    assert!(!disabled_response.state.focused);

    for frame in [
        &default_frame,
        &hover_frame,
        &pressed_frame,
        &focused_frame,
        &disabled_frame,
    ] {
        let label = button_text(frame, source);
        assert_eq!(label.layout, Some(expected_id));
        assert_eq!(label.text, source);
        assert!(matches!(frame.primitives.first(), Some(Primitive::Rect(_))));
        assert!(matches!(frame.primitives.last(), Some(Primitive::Text(_))));
        assert_eq!(frame.semantics.nodes().len(), 1);
        assert_eq!(frame.semantics.nodes()[0].label.as_deref(), Some(source));
    }

    for frame in [
        &default_frame,
        &hover_frame,
        &pressed_frame,
        &disabled_frame,
    ] {
        assert_eq!(frame.primitives.len(), 2);
    }
    assert_eq!(focused_frame.primitives.len(), 4);
    assert!(matches!(focused_frame.primitives[1], Primitive::Path(_)));
    assert!(matches!(focused_frame.primitives[2], Primitive::Path(_)));
    assert!(
        hover_frame
            .platform_requests
            .contains(&PlatformRequest::SetCursor(CursorShape::PointingHand))
    );
    assert!(
        pressed_frame
            .platform_requests
            .contains(&PlatformRequest::SetCursor(CursorShape::PointingHand))
    );
    assert!(
        !disabled_frame
            .platform_requests
            .contains(&PlatformRequest::SetCursor(CursorShape::PointingHand))
    );
}

#[test]
#[allow(clippy::too_many_lines)]
fn delegated_action_button_preserves_visibility_metadata_and_exact_activation_routing() {
    let source =
        "Complete action button source remains intact while its retained presentation elides";
    let context = ActionContext::Widget(WidgetId::from_key("action-owner"));
    let action = ActionDescriptor::new("render.start", source);
    let mut store = TextLayoutStore::new();

    let mut plain_memory = UiMemory::new();
    let input = UiInput::default();
    let theme = default_dark_theme();
    let mut ui = Ui::new(&input, &mut plain_memory, &theme).with_text_layouts(&mut store);
    let plain_response = ui
        .action_button("action", BUTTON, &action, context.clone())
        .expect("visible action button");
    let plain_frame = ui.finish_output();
    let plain_label = button_text(&plain_frame, source);
    let expected_id = plain_label.layout.expect("retained action label identity");
    let stored = store
        .stored_layout(expected_id)
        .expect("resident action label entry");
    assert_eq!(stored.key.text, source);
    assert_eq!(stored.key.overflow, TextOverflow::EndEllipsis);
    assert!(stored.layout.is_elided());
    assert_eq!(marker_count(&store, plain_label), 1);
    assert!(plain_frame.actions.is_empty());

    let accounting = (
        store.len(),
        store.retained_payload_bytes(),
        store.change_cursor(),
    );
    let mut hidden = action.clone();
    hidden.state.visible = false;
    let mut hidden_memory = UiMemory::new();
    let input = pointer_transition(true);
    let mut ui = Ui::new(&input, &mut hidden_memory, &theme).with_text_layouts(&mut store);
    assert_eq!(
        ui.action_button("action", BUTTON, &hidden, context.clone()),
        None
    );
    let hidden_frame = ui.finish_output();
    assert!(hidden_frame.primitives.is_empty());
    assert!(hidden_frame.semantics.nodes().is_empty());
    assert!(hidden_frame.actions.is_empty());
    assert!(hidden_frame.platform_requests.is_empty());
    assert_eq!(hidden_frame.repaint, RepaintRequest::None);
    assert_eq!(
        (
            store.len(),
            store.retained_payload_bytes(),
            store.change_cursor()
        ),
        accounting
    );

    let mut disabled = action.clone();
    disabled.state.enabled = false;
    let mut disabled_memory = UiMemory::new();
    let mut ui = Ui::new(&input, &mut disabled_memory, &theme).with_text_layouts(&mut store);
    let disabled_response = ui
        .action_button("action", BUTTON, &disabled, context.clone())
        .expect("visible disabled action button");
    let disabled_frame = ui.finish_output();
    assert!(disabled_response.state.disabled);
    assert!(!disabled_response.clicked);
    assert!(!disabled_response.keyboard_activated);
    assert_eq!(
        button_text(&disabled_frame, source).layout,
        Some(expected_id)
    );
    assert_eq!(
        disabled_frame.semantics.nodes()[0].label.as_deref(),
        Some(source)
    );
    assert!(disabled_frame.actions.is_empty());

    let mut pressed_memory = UiMemory::new();
    let input = pointer_transition(true);
    let mut ui = Ui::new(&input, &mut pressed_memory, &theme).with_text_layouts(&mut store);
    let pressed = ui
        .action_button("action", BUTTON, &action, context.clone())
        .expect("pressed action button");
    assert!(pressed.state.pressed);
    assert!(ui.finish_output().actions.is_empty());

    let input = pointer_transition(false);
    let mut ui = Ui::new(&input, &mut pressed_memory, &theme).with_text_layouts(&mut store);
    let released = ui
        .action_button("action", BUTTON, &action, context.clone())
        .expect("released action button");
    let mut pointer_frame = ui.finish_output();
    assert!(released.clicked);
    assert!(!released.keyboard_activated);
    assert_eq!(
        button_text(&pointer_frame, source).layout,
        Some(expected_id)
    );
    let invocation = pointer_frame
        .actions
        .pop_front()
        .expect("pointer action invocation");
    assert_eq!(invocation.action_id, ActionId::new("render.start"));
    assert_eq!(invocation.source, ActionSource::Button);
    assert_eq!(invocation.context, context);
    assert!(pointer_frame.actions.is_empty());

    let followup = ActionDescriptor::new("render.followup", "Follow up");
    let keyboard = UiInput {
        keyboard: KeyboardInput {
            modifiers: Modifiers::default(),
            events: vec![KeyEvent::new(
                Key::Space,
                KeyState::Pressed,
                Modifiers::default(),
                false,
            )],
        },
        ..UiInput::default()
    };
    let mut keyboard_memory = UiMemory::new();
    keyboard_memory.focus(plain_response.id);
    let mut ui = Ui::new(&keyboard, &mut keyboard_memory, &theme).with_text_layouts(&mut store);
    let keyboard_response = ui
        .action_button("action", BUTTON, &action, context.clone())
        .expect("keyboard action button");
    assert!(ui.invoke_action_descriptor(
        &followup,
        ActionSource::Programmatic,
        ActionContext::Global,
    ));
    let mut keyboard_frame = ui.finish_output();
    assert!(keyboard_response.clicked);
    assert!(keyboard_response.keyboard_activated);
    assert_eq!(
        button_text(&keyboard_frame, source).layout,
        Some(expected_id)
    );
    let invocations = keyboard_frame.actions.drain().collect::<Vec<_>>();
    assert_eq!(invocations.len(), 2);
    assert_eq!(invocations[0].action_id, ActionId::new("render.start"));
    assert_eq!(invocations[0].source, ActionSource::Button);
    assert_eq!(invocations[0].context, context);
    assert_eq!(invocations[1].action_id, ActionId::new("render.followup"));
    assert_eq!(invocations[1].source, ActionSource::Programmatic);
    assert_eq!(invocations[1].context, ActionContext::Global);

    let mut rich = action.clone();
    rich.icon = Some(stern_icons_phosphor::regular::PLAY.into());
    rich.tooltip = Some("Longer application-owned tooltip".to_owned());
    rich.keywords = vec!["start".to_owned(), "render".to_owned()];
    rich.shortcut = Some(Shortcut::new(
        Modifiers::new(false, true, false, false),
        Key::Character("r".to_owned()),
    ));
    rich.state.checked = Some(true);
    let mut rich_memory = UiMemory::new();
    let input = UiInput::default();
    let mut ui = Ui::new(&input, &mut rich_memory, &theme).with_text_layouts(&mut store);
    let rich_response = ui
        .action_button("action", BUTTON, &rich, context)
        .expect("metadata-rich action button");
    let rich_frame = ui.finish_output();
    assert_eq!(rich_response, plain_response);
    let rich_icon = rich_frame
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            Primitive::Icon(icon) => Some(icon),
            _ => None,
        })
        .expect("metadata-rich action icon primitive");
    assert_eq!(rich_icon.icon, stern_icons_phosphor::regular::PLAY.icon());
    let rich_label = button_text(&rich_frame, source);
    let rich_layout = rich_label
        .layout
        .expect("retained rich action label identity");
    let rich_stored = store
        .stored_layout(rich_layout)
        .expect("resident rich action label entry");
    assert_eq!(rich_stored.key.text, source);
    assert_eq!(rich_stored.key.overflow, TextOverflow::EndEllipsis);
    assert!(rich_stored.layout.is_elided());
    assert!(rich_label.origin.x > plain_label.origin.x);
    assert_eq!(rich_frame.semantics, plain_frame.semantics);
    assert!(rich_frame.actions.is_empty());
}

#[test]
#[allow(clippy::too_many_lines)]
fn invalid_and_nonfinite_rects_preserve_preexisting_output_and_interaction_topology() {
    let source = "Complete invalid-geometry button source";
    let theme = default_dark_theme();
    for rect in [
        Rect::new(BUTTON.x, BUTTON.y, f32::NAN, BUTTON.height),
        Rect::new(BUTTON.x, BUTTON.y, f32::INFINITY, BUTTON.height),
        Rect::new(BUTTON.x, BUTTON.y, f32::NEG_INFINITY, BUTTON.height),
        Rect::new(f32::NAN, BUTTON.y, BUTTON.width, BUTTON.height),
        Rect::new(BUTTON.x, f32::INFINITY, BUTTON.width, BUTTON.height),
    ] {
        let input = UiInput::default();
        let mut layoutless_memory = UiMemory::new();
        let mut ui = Ui::new(&input, &mut layoutless_memory, &theme);
        let layoutless_response = ui.button("invalid", rect, source, false);
        let layoutless = ui.finish_output();

        let mut store = TextLayoutStore::new();
        let mut retained_memory = UiMemory::new();
        let mut ui = Ui::new(&input, &mut retained_memory, &theme).with_text_layouts(&mut store);
        let retained_response = ui.button("invalid", rect, source, false);
        let retained = ui.finish_output();

        assert_eq!(retained_response.id, layoutless_response.id);
        assert_rect_bits(retained_response.rect, layoutless_response.rect);
        assert_eq!(retained_response.state, layoutless_response.state);
        assert_eq!(retained_response.clicked, layoutless_response.clicked);
        assert_eq!(
            retained_response.keyboard_activated,
            layoutless_response.keyboard_activated
        );
        assert_eq!(retained.primitives.len(), layoutless.primitives.len());
        for (retained_primitive, layoutless_primitive) in
            retained.primitives.iter().zip(&layoutless.primitives)
        {
            match (retained_primitive, layoutless_primitive) {
                (Primitive::Rect(retained), Primitive::Rect(layoutless)) => {
                    assert_rect_bits(retained.rect, layoutless.rect);
                    assert_eq!(retained.fill, layoutless.fill);
                    assert_eq!(retained.stroke, layoutless.stroke);
                    assert_eq!(retained.radius, layoutless.radius);
                }
                (Primitive::Text(retained), Primitive::Text(layoutless)) => {
                    assert!(retained.layout.is_some());
                    assert_eq!(layoutless.layout, None);
                    assert_eq!(retained.origin.x.to_bits(), layoutless.origin.x.to_bits());
                    assert_eq!(retained.origin.y.to_bits(), layoutless.origin.y.to_bits());
                    assert_eq!(retained.text, layoutless.text);
                    assert_eq!(retained.family, layoutless.family);
                    assert_eq!(retained.size.to_bits(), layoutless.size.to_bits());
                    assert_eq!(
                        retained.line_height.to_bits(),
                        layoutless.line_height.to_bits()
                    );
                    assert_eq!(retained.brush, layoutless.brush);
                }
                other => panic!("button primitive topology changed: {other:?}"),
            }
        }

        assert_eq!(retained.semantics.nodes().len(), 1);
        assert_eq!(layoutless.semantics.nodes().len(), 1);
        let mut retained_semantic = retained.semantics.nodes()[0].clone();
        let mut layoutless_semantic = layoutless.semantics.nodes()[0].clone();
        assert_rect_bits(retained_semantic.bounds, layoutless_semantic.bounds);
        retained_semantic.bounds = Rect::ZERO;
        layoutless_semantic.bounds = Rect::ZERO;
        assert_eq!(retained_semantic, layoutless_semantic);
        assert_eq!(retained.repaint, layoutless.repaint);
        assert_eq!(retained.actions, layoutless.actions);
        assert_eq!(retained.platform_requests, layoutless.platform_requests);
        assert_eq!(retained.warnings, layoutless.warnings);

        let label = button_text(&retained, source);
        let stored = store
            .stored_layout(label.layout.expect("registered invalid-geometry policy"))
            .expect("resident invalid-geometry policy");
        let raw_span = rect.width - theme.controls.padding_x * 2.0_f32;
        assert_eq!(stored.key.width_bits, raw_span.max(0.0_f32).to_bits());
        assert_eq!(stored.key.overflow, TextOverflow::EndEllipsis);
        assert_eq!(stored.key.text, source);
        assert_eq!(label.text, source);
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn end_ellipsis_adoption_includes_buttons_and_canonical_chrome_toolbar_only() {
    let standard = "Complete standard button adoption source";
    let action_source = "Complete delegated action button adoption source";
    let icon_label = "Neighboring vector icon accessible label";
    let image_icon_label = "Neighboring image icon accessible label";
    let selectable_icon_label = "Neighboring selectable image icon accessible label";
    let tab = "Neighboring tab source keeps generic retained policy";
    let row = "Neighboring list row source keeps generic retained policy";
    let menu = "Neighboring menu source keeps generic retained policy";
    let toolbar = "Canonical chrome toolbar source uses explicit retained policy";
    let busy = "Neighboring busy source keeps generic retained policy";
    let busy_cancel = "Neighboring busy cancel keeps generic retained policy";
    let action = ActionDescriptor::new("adoption.action", action_source);
    let menu_bar = MenuBar::from_menus([MenuBarMenu::from_actions(
        MenuBarMenuId::from_raw(1),
        menu,
        [ActionDescriptor::new("boundary.menu", "Menu item")],
    )]);
    let toolbar_model = Toolbar::from_groups([ToolbarGroup::from_actions(
        ToolbarGroupId::from_raw(2),
        "Boundary toolbar group",
        [ActionDescriptor::new("boundary.toolbar", toolbar)],
    )]);
    let tabs = TabStrip::new();
    let status = StatusBar::new();
    let chrome_scene = ChromeScene::new(
        ChromeSceneConfig::new(
            WidgetId::from_key("boundary-chrome"),
            Rect::new(0.0, 50.0, 240.0, 28.0),
            Rect::new(0.0, 82.0, 240.0, 28.0),
            Rect::new(0.0, 114.0, 240.0, 28.0),
            Rect::new(0.0, 146.0, 240.0, 28.0),
            ActionContext::Global,
        )
        .with_widths([
            (ChromeSceneItemKey::Menu(MenuBarMenuId::from_raw(1)), 200.0),
            (
                ChromeSceneItemKey::Toolbar {
                    group: ToolbarGroupId::from_raw(2),
                    action: ActionId::new("boundary.toolbar"),
                },
                200.0,
            ),
        ]),
        &menu_bar,
        &toolbar_model,
        &tabs,
        &status,
    );
    let jobs = JobList::from_rows([JobRow::new(JobRowId::from_raw(3), busy, JobPhase::Running)
        .with_detail("")
        .with_cancel(JobCancel::new(
            ActionDescriptor::new("boundary.busy.cancel", busy_cancel),
            ActionContext::Global,
        ))]);
    let diagnostics = DiagnosticStrip::new();
    let feedback = FeedbackStack::new();
    let feedback_scene = SystemFeedbackScene::prepare(
        SystemFeedbackSceneConfig::new(
            WidgetId::from_key("boundary-busy"),
            Rect::new(0.0, 180.0, 300.0, 32.0),
            Rect::new(0.0, 216.0, 300.0, 32.0),
            Rect::new(0.0, 252.0, 300.0, 32.0),
        ),
        &jobs,
        &diagnostics,
        &feedback,
        Duration::from_secs(3),
    )
    .expect("valid busy boundary scene");
    let theme = default_dark_theme();
    let input = UiInput::default();
    let mut memory = UiMemory::new();
    let mut store = TextLayoutStore::new();
    let mut ui = Ui::new(&input, &mut memory, &theme).with_text_layouts(&mut store);

    let _ = ui.button("standard", BUTTON, standard, false);
    let _ = ui.action_button("action", BUTTON, &action, ActionContext::Global);
    let _ = ui.icon_button(
        "icon",
        BUTTON,
        stern_icons_phosphor::regular::CHECK,
        icon_label,
        false,
    );
    let _ = ui.image_icon_button(
        "image-icon",
        BUTTON,
        ImageId::from_raw(12),
        image_icon_label,
        false,
    );
    let _ = ui.image_icon_selectable_button(
        "selectable-image-icon",
        BUTTON,
        ImageId::from_raw(13),
        selectable_icon_label,
        true,
        false,
    );
    let selectable = ui.selectable("selectable", BUTTON, true, false);
    let _ = ui.tab_button("tab", BUTTON, tab, false, false);
    let _ = ui.list_row("row", BUTTON, row, false, false);
    let _ = ui.chrome_scene(&chrome_scene);
    let _ = ui.system_feedback(&feedback_scene);
    let frame = ui.finish_output();

    for source in [standard, action_source] {
        let label = button_text(&frame, source);
        let stored = store
            .stored_layout(label.layout.expect("explicit button adoption identity"))
            .expect("resident button adoption identity");
        assert_eq!(stored.key.text, source);
        assert_eq!(stored.key.overflow, TextOverflow::EndEllipsis);
        assert_eq!(
            stored.key.width_bits,
            (BUTTON.width - theme.controls.padding_x * 2.0_f32).to_bits()
        );
    }

    let toolbar_label = button_text(&frame, toolbar);
    let toolbar_layout = store
        .stored_layout(
            toolbar_label
                .layout
                .expect("explicit toolbar adoption identity"),
        )
        .expect("resident toolbar adoption identity");
    assert_eq!(toolbar_layout.key.text, toolbar);
    assert_eq!(toolbar_layout.key.overflow, TextOverflow::EndEllipsis);
    assert_eq!(
        toolbar_layout.key.width_bits,
        (200.0_f32 - theme.controls.padding_x * 2.0_f32).to_bits()
    );

    for source in [tab, row, menu, busy, busy_cancel] {
        let label = frame
            .primitives
            .iter()
            .find_map(|primitive| match primitive {
                Primitive::Text(text) if text.text == source => Some(text),
                _ => None,
            })
            .unwrap_or_else(|| panic!("missing neighboring source {source}"));
        let stored = store
            .stored_layout(label.layout.expect("generic neighboring identity"))
            .expect("resident neighboring identity");
        assert_eq!(stored.key.text, source);
        assert_eq!(stored.key.overflow, TextOverflow::Visible);
        assert_eq!(stored.key.width_bits, 0.0_f32.to_bits());
    }

    for accessible_label in [icon_label, image_icon_label, selectable_icon_label] {
        assert!(frame.primitives.iter().all(
            |primitive| !matches!(primitive, Primitive::Text(text) if text.text == accessible_label)
        ));
        assert!(
            store
                .layouts()
                .all(|entry| entry.key.text != accessible_label)
        );
        assert!(
            frame
                .semantics
                .nodes()
                .iter()
                .any(|node| node.label.as_deref() == Some(accessible_label))
        );
    }
    assert!(frame.semantics.get(selectable.id).is_none());

    let mut adopters = store
        .layouts()
        .filter(|entry| entry.key.overflow == TextOverflow::EndEllipsis)
        .map(|entry| entry.key.text.clone())
        .collect::<Vec<_>>();
    adopters.sort();
    let mut expected = vec![
        standard.to_owned(),
        action_source.to_owned(),
        toolbar.to_owned(),
    ];
    expected.sort();
    assert_eq!(adopters, expected);
}

#[test]
fn production_call_graph_bounds_button_adoption_and_absent_split_busy_consumers() {
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

    let button_widget_calls = sources
        .iter()
        .filter_map(|(path, source)| {
            let count = source
                .lines()
                .filter(|line| {
                    line.trim_start()
                        .starts_with("let mut output = button_widget(")
                })
                .count();
            (count > 0).then_some((path.as_str(), count))
        })
        .collect::<Vec<_>>();
    assert_eq!(button_widget_calls, vec![("src/ui/basic_controls.rs", 1)]);

    let retained_button_delegates = sources
        .iter()
        .filter_map(|(path, source)| {
            let count = source.matches("self.button(").count();
            (count > 0).then_some((path.as_str(), count))
        })
        .collect::<Vec<_>>();
    assert!(retained_button_delegates.is_empty());

    for (path, source) in &sources {
        let normalized = source.to_ascii_lowercase();
        assert!(
            !normalized.contains("split_button") && !normalized.contains("splitbutton"),
            "unexpected split-button rendering consumer in {path}"
        );
        assert!(
            !normalized.contains("busy_button") && !normalized.contains("busybutton"),
            "unexpected busy-button rendering consumer in {path}"
        );
    }
}
