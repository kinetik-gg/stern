use std::time::Duration;

use super::{
    CursorShape, FrameContext, FrameOutput, FrameWarning, PlatformRequest, RepaintRequest,
    TimeInfo, Ui, ViewportInfo,
};
use crate::input::{InputStreamConflict, UiInput};
use crate::{
    ActionContext, ActionId, ActionSource, Brush, ClipId, Color, CornerRadius, InputWheelDelta,
    Key, KeyEvent, KeyState, KeyboardInput, LayerId, Modifiers, MouseButton, PhysicalSize, Point,
    PointerButtonState, PointerInput, Primitive, Rect, RectPrimitive, ScaleFactor, SemanticNode,
    SemanticRole, SemanticTreeError, Size, TextInputEvent, TextInputOwnerMode, Transform,
    UiInputEvent, UiMemory, Vec2, WidgetId,
};

fn runtime_test_context(input: UiInput) -> FrameContext {
    FrameContext::new(
        ViewportInfo::new(
            Size::new(100.0, 50.0),
            PhysicalSize::new(100, 50),
            ScaleFactor::ONE,
        ),
        input,
        TimeInfo::default(),
    )
}

fn canonical_click_input(click_count: u8) -> UiInput {
    let mut input = UiInput::default();
    input.push_event(UiInputEvent::PointerButton {
        button: MouseButton::Primary,
        down: true,
        click_count,
        position: Some(Point::new(5.0, 5.0)),
    });
    input.push_event(UiInputEvent::PointerButton {
        button: MouseButton::Primary,
        down: false,
        click_count,
        position: Some(Point::new(5.0, 5.0)),
    });
    input
}

#[test]
fn creates_viewport_info() {
    let viewport = ViewportInfo::new(
        Size::new(800.0, 600.0),
        PhysicalSize::new(1600, 1200),
        ScaleFactor::new(2.0),
    );

    assert_eq!(viewport.logical_size, Size::new(800.0, 600.0));
    assert_eq!(viewport.physical_size, PhysicalSize::new(1600, 1200));
    assert_eq!(viewport.scale_factor.value(), 2.0);
}

#[test]
fn creates_frame_context() {
    let viewport = ViewportInfo::new(
        Size::new(100.0, 50.0),
        PhysicalSize::new(100, 50),
        ScaleFactor::ONE,
    );
    let time = TimeInfo::new(Duration::from_millis(16), Duration::from_millis(16), 1);
    let context = FrameContext::new(viewport, UiInput::default(), time);

    assert_eq!(context.viewport, viewport);
    assert_eq!(context.time.frame_index, 1);
}

#[test]
fn frame_output_defaults_to_no_repaint() {
    let output = FrameOutput::new();

    assert!(output.primitives.is_empty());
    assert!(output.semantics.nodes().is_empty());
    assert_eq!(output.repaint, RepaintRequest::None);
    assert!(output.actions.is_empty());
    assert!(output.platform_requests.is_empty());
    assert!(output.warnings.is_empty());
}

#[test]
fn repaint_request_merge_keeps_most_urgent_request() {
    assert_eq!(
        RepaintRequest::After(Duration::from_secs(2))
            .merge(RepaintRequest::After(Duration::from_secs(1))),
        RepaintRequest::After(Duration::from_secs(1))
    );
    assert_eq!(
        RepaintRequest::After(Duration::from_secs(1)).merge(RepaintRequest::NextFrame),
        RepaintRequest::NextFrame
    );
    assert_eq!(
        RepaintRequest::NextFrame.merge(RepaintRequest::Continuous),
        RepaintRequest::Continuous
    );
}

#[test]
fn frame_output_accumulates_repaint_requests() {
    let mut output = FrameOutput::new();

    output.request_repaint(RepaintRequest::After(Duration::from_secs(5)));
    output.request_repaint(RepaintRequest::After(Duration::from_secs(1)));

    assert_eq!(
        output.repaint,
        RepaintRequest::After(Duration::from_secs(1))
    );
}

#[test]
fn frame_output_accumulates_actions() {
    let mut output = FrameOutput::new();

    output.invoke_action(
        ActionId::new("file.save"),
        ActionSource::Shortcut,
        ActionContext::Global,
    );

    assert_eq!(output.actions.len(), 1);
    assert_eq!(output.repaint, RepaintRequest::NextFrame);
    assert_eq!(
        output.actions.pop_front().expect("action").action_id,
        ActionId::new("file.save")
    );
}

#[test]
fn frame_output_accumulates_render_semantics_and_platform_requests() {
    let mut output = FrameOutput::new();
    let id = WidgetId::from_key("button");
    let rect = Rect::new(1.0, 2.0, 30.0, 20.0);

    output.push_primitive(Primitive::Rect(RectPrimitive {
        rect,
        fill: Some(Brush::Solid(Color::WHITE)),
        stroke: None,
        radius: CornerRadius::all(2.0),
    }));
    output.push_semantic_node(
        SemanticNode::new(id, SemanticRole::Button, rect)
            .focusable(true)
            .with_label("Run"),
    );
    output.push_platform_request(PlatformRequest::SetCursor(CursorShape::PointingHand));

    assert_eq!(output.primitives.len(), 1);
    assert_eq!(output.semantics.root(), Some(id));
    assert_eq!(output.semantics.focus_order(), vec![id]);
    assert_eq!(
        output.platform_requests,
        vec![PlatformRequest::SetCursor(CursorShape::PointingHand)]
    );
}

#[test]
fn ui_system_menu_requests_require_finite_points_preserve_fifo_and_emit_no_actions() {
    let mut memory = UiMemory::new();
    let mut ui = Ui::begin_frame(runtime_test_context(UiInput::default()), &mut memory);
    let first = Point::new(12.0, 18.0);
    let second = Point::new(-4.0, 32.0);

    ui.push_platform_request(PlatformRequest::SetWindowTitle("before".to_owned()));
    assert!(ui.request_window_system_menu(first));
    assert!(!ui.request_window_system_menu(Point::new(f32::NAN, 1.0)));
    ui.push_platform_request(PlatformRequest::SetCursor(CursorShape::PointingHand));
    assert!(!ui.request_window_system_menu(Point::new(1.0, f32::INFINITY)));
    assert!(ui.request_window_system_menu(second));
    let output = ui.end_frame();

    assert_eq!(
        output.platform_requests,
        vec![
            PlatformRequest::SetWindowTitle("before".to_owned()),
            PlatformRequest::ShowWindowSystemMenu { position: first },
            PlatformRequest::SetCursor(CursorShape::PointingHand),
            PlatformRequest::ShowWindowSystemMenu { position: second },
        ]
    );
    assert!(output.actions.is_empty());
}

#[test]
fn platform_request_and_frame_debug_redact_external_payloads() {
    let clipboard = "private clipboard payload";
    let title = "private title";
    let url = "https://example.com/docs?secret=token#private-fragment";
    let requests = vec![
        PlatformRequest::CopyToClipboard(clipboard.to_owned()),
        PlatformRequest::SetWindowTitle(title.to_owned()),
        PlatformRequest::OpenUrl(url.to_owned()),
    ];
    let mut output = FrameOutput::new();
    for request in requests.clone() {
        output.push_platform_request(request);
    }

    let request_debug = format!("{requests:?}");
    let output_debug = format!("{output:?}");

    for debug in [request_debug, output_debug] {
        assert!(!debug.contains(clipboard));
        assert!(!debug.contains(title));
        assert!(!debug.contains("secret"));
        assert!(!debug.contains("private-fragment"));
        assert!(debug.contains("https"));
    }

    let malformed = format!(
        "{:?}",
        PlatformRequest::OpenUrl("private-custom-scheme:secret".to_owned())
    );
    assert!(!malformed.contains("private-custom-scheme"));
    assert!(!malformed.contains("secret"));
    assert!(malformed.contains("unsupported"));
}

#[test]
fn frame_output_exports_accessibility_snapshot_independent_from_painting() {
    let mut output = FrameOutput::new();
    let root = WidgetId::from_key("root");
    let button = WidgetId::from_key("button");
    let rect = Rect::new(1.0, 2.0, 30.0, 20.0);

    output.push_primitive(Primitive::Rect(RectPrimitive {
        rect,
        fill: Some(Brush::Solid(Color::WHITE)),
        stroke: None,
        radius: CornerRadius::all(2.0),
    }));
    output.push_semantic_node(
        SemanticNode::new(root, SemanticRole::Root, Rect::ZERO).with_children([button]),
    );
    output.push_semantic_node(
        SemanticNode::new(button, SemanticRole::Button, rect)
            .focusable(true)
            .with_label("Run"),
    );

    let snapshot = output
        .accessibility_snapshot(Some(button))
        .expect("snapshot");

    assert_eq!(output.primitives.len(), 1);
    assert_eq!(
        snapshot
            .nodes
            .iter()
            .map(|node| node.id)
            .collect::<Vec<_>>(),
        vec![root, button]
    );
    assert_eq!(snapshot.focus_order, vec![button]);
    assert_eq!(snapshot.focused, Some(button));
    assert_eq!(
        snapshot.node(button).expect("button").label.as_deref(),
        Some("Run")
    );
}

#[test]
fn ui_builder_registers_ids_and_finalizes_output() {
    let viewport = ViewportInfo::new(
        Size::new(100.0, 50.0),
        PhysicalSize::new(100, 50),
        ScaleFactor::ONE,
    );
    let context = FrameContext::new(viewport, UiInput::default(), TimeInfo::default());
    let mut memory = UiMemory::new();
    memory.focus(WidgetId::from_key("stale"));

    let mut ui = Ui::begin_frame(context, &mut memory);
    let id = ui.id("save");
    let duplicate = ui.register_id(id);
    ui.push_semantic_node(SemanticNode::new(id, SemanticRole::Button, Rect::ZERO));
    ui.invoke_action(
        ActionId::new("file.save"),
        ActionSource::Button,
        ActionContext::Global,
    );
    ui.push_platform_request(PlatformRequest::SetCursor(CursorShape::PointingHand));
    let output = ui.end_frame();

    assert_eq!(id, duplicate);
    assert_eq!(output.semantics.root(), Some(id));
    assert_eq!(output.actions.len(), 1);
    assert_eq!(output.platform_requests.len(), 1);
    assert_eq!(
        output.warnings,
        vec![FrameWarning::DuplicateWidgetId { id }]
    );
}

#[test]
fn ui_builder_registers_scopes_for_duplicate_detection() {
    let viewport = ViewportInfo::new(
        Size::new(100.0, 50.0),
        PhysicalSize::new(100, 50),
        ScaleFactor::ONE,
    );
    let context = FrameContext::new(viewport, UiInput::default(), TimeInfo::default());
    let mut memory = UiMemory::new();
    let mut ui = Ui::begin_frame(context, &mut memory);

    let id = ui.push_id_scope("panel");
    ui.pop_id_scope();
    ui.push_id_scope("panel");
    let output = ui.end_frame();

    assert_eq!(
        output.warnings,
        vec![FrameWarning::DuplicateWidgetId { id }]
    );
}

#[test]
fn ui_builder_clears_transient_memory_at_frame_start() {
    let viewport = ViewportInfo::new(
        Size::new(100.0, 50.0),
        PhysicalSize::new(100, 50),
        ScaleFactor::ONE,
    );
    let context = FrameContext::new(viewport, UiInput::default(), TimeInfo::default());
    let mut memory = UiMemory::new();
    let hovered = WidgetId::from_key("hovered");
    let focused = WidgetId::from_key("focused");
    memory.set_hovered(hovered);
    memory.focus(focused);

    let ui = Ui::begin_frame(context, &mut memory);

    assert_eq!(ui.memory().hovered(), None);
    assert_eq!(ui.memory().focused(), Some(focused));
}

#[test]
fn ui_builder_cancels_pointer_interaction_on_focus_loss_at_frame_start() {
    let viewport = ViewportInfo::new(
        Size::new(100.0, 50.0),
        PhysicalSize::new(100, 50),
        ScaleFactor::ONE,
    );
    let context = FrameContext::new(
        viewport,
        UiInput {
            window_focused: false,
            ..UiInput::default()
        },
        TimeInfo::default(),
    );
    let mut memory = UiMemory::new();
    let focused = WidgetId::from_key("focused");
    let owner = WidgetId::from_key("owner");
    memory.focus(focused);
    memory.set_text_input_owner(focused);
    memory.activate(owner);
    memory.press(owner);
    memory.capture_pointer(owner);
    memory.start_drag(owner);

    let ui = Ui::begin_frame(context, &mut memory);

    assert_eq!(ui.memory().active(), None);
    assert_eq!(ui.memory().pressed(), None);
    assert_eq!(ui.memory().pointer_capture(), None);
    assert_eq!(ui.memory().drag_source(), None);
    assert!(ui.memory().pointer_interaction_cancelled());
    assert_eq!(ui.memory().focused(), Some(focused));
    assert_eq!(ui.memory().text_input_owner(), Some(focused));
}

#[test]
fn ui_builder_defers_ordered_release_all_cleanup_until_frame_end() {
    let viewport = ViewportInfo::new(
        Size::new(100.0, 50.0),
        PhysicalSize::new(100, 50),
        ScaleFactor::ONE,
    );
    let mut input = UiInput {
        window_focused: true,
        ..UiInput::default()
    };
    input.release_pointer_buttons();
    let context = FrameContext::new(viewport, input, TimeInfo::default());
    let mut memory = UiMemory::new();
    let focused = WidgetId::from_key("focused");
    let owner = WidgetId::from_key("owner");
    memory.focus(focused);
    memory.set_text_input_owner(focused);
    memory.activate(owner);
    memory.press(owner);
    memory.capture_pointer(owner);
    memory.start_drag(owner);

    let mut ui = Ui::begin_frame(context, &mut memory);

    assert_eq!(ui.memory().active(), Some(owner));
    assert_eq!(ui.memory().pointer_capture(), Some(owner));
    assert!(!ui.memory().pointer_interaction_cancelled());
    ui.register_id(owner);
    ui.register_id(focused);
    let output = ui.end_frame();

    assert_eq!(memory.active(), None);
    assert_eq!(memory.pressed(), None);
    assert_eq!(memory.pointer_capture(), None);
    assert_eq!(memory.drag_source(), None);
    assert!(memory.pointer_interaction_cancelled());
    assert_eq!(memory.focused(), Some(focused));
    assert_eq!(memory.text_input_owner(), Some(focused));
    assert_eq!(output.repaint, RepaintRequest::NextFrame);
}

#[test]
fn ui_builder_starts_text_input_for_focused_widget() {
    let viewport = ViewportInfo::new(
        Size::new(100.0, 50.0),
        PhysicalSize::new(100, 50),
        ScaleFactor::ONE,
    );
    let context = FrameContext::new(viewport, UiInput::default(), TimeInfo::default());
    let field = WidgetId::from_key("field");
    let rect = Rect::new(4.0, 8.0, 60.0, 18.0);
    let mut memory = UiMemory::new();
    memory.focus(field);

    let mut ui = Ui::begin_frame(context, &mut memory);
    assert!(ui.start_text_input(field, Some(rect)));
    let output = ui.end_frame();

    assert_eq!(memory.text_input_owner(), Some(field));
    assert_eq!(
        output.platform_requests,
        vec![PlatformRequest::StartTextInput { rect: Some(rect) }]
    );
}

#[test]
fn ui_builder_updates_text_input_rect_without_restarting_same_owner() {
    let viewport = ViewportInfo::new(
        Size::new(100.0, 50.0),
        PhysicalSize::new(100, 50),
        ScaleFactor::ONE,
    );
    let context = FrameContext::new(viewport, UiInput::default(), TimeInfo::default());
    let field = WidgetId::from_key("field");
    let rect = Rect::new(6.0, 9.0, 1.0, 18.0);
    let mut memory = UiMemory::new();
    memory.focus(field);
    memory.set_text_input_owner(field);

    let mut ui = Ui::begin_frame(context, &mut memory);
    assert!(ui.start_text_input(field, Some(rect)));
    let output = ui.end_frame();

    assert_eq!(memory.text_input_owner(), Some(field));
    assert_eq!(
        output.platform_requests,
        vec![PlatformRequest::UpdateTextInputRect { rect }]
    );
}

#[test]
fn ui_builder_same_text_input_owner_without_rect_emits_no_geometry_work() {
    let viewport = ViewportInfo::new(
        Size::new(100.0, 50.0),
        PhysicalSize::new(100, 50),
        ScaleFactor::ONE,
    );
    let context = FrameContext::new(viewport, UiInput::default(), TimeInfo::default());
    let field = WidgetId::from_key("field");
    let mut memory = UiMemory::new();
    memory.focus(field);
    memory.set_text_input_owner(field);

    let mut ui = Ui::begin_frame(context, &mut memory);
    assert!(ui.start_text_input(field, None));
    let output = ui.end_frame();

    assert!(output.platform_requests.is_empty());
}

#[test]
fn ui_builder_does_not_start_text_input_for_unfocused_widget() {
    let viewport = ViewportInfo::new(
        Size::new(100.0, 50.0),
        PhysicalSize::new(100, 50),
        ScaleFactor::ONE,
    );
    let context = FrameContext::new(viewport, UiInput::default(), TimeInfo::default());
    let focused = WidgetId::from_key("focused");
    let field = WidgetId::from_key("field");
    let mut memory = UiMemory::new();
    memory.focus(focused);

    let mut ui = Ui::begin_frame(context, &mut memory);
    assert!(!ui.start_text_input(field, None));
    let output = ui.end_frame();

    assert_eq!(memory.text_input_owner(), None);
    assert!(output.platform_requests.is_empty());
}

#[test]
fn end_frame_warns_about_unbalanced_primitive_stacks() {
    let viewport = ViewportInfo::new(
        Size::new(100.0, 50.0),
        PhysicalSize::new(100, 50),
        ScaleFactor::ONE,
    );
    let context = FrameContext::new(viewport, UiInput::default(), TimeInfo::default());
    let mut memory = UiMemory::new();
    let mut ui = Ui::begin_frame(context, &mut memory);
    let open_clip = ClipId::from_raw(1);
    let wrong_clip = ClipId::from_raw(2);
    let open_layer = LayerId::from_raw(3);

    ui.extend_primitives([
        Primitive::ClipBegin {
            id: open_clip,
            rect: Rect::ZERO,
        },
        Primitive::ClipEnd { id: wrong_clip },
        Primitive::LayerBegin { id: open_layer },
        Primitive::TransformEnd,
        Primitive::TransformBegin(Transform::IDENTITY),
    ]);
    let output = ui.end_frame();

    assert_eq!(
        output.warnings,
        vec![
            FrameWarning::UnmatchedClipEnd { id: wrong_clip },
            FrameWarning::UnmatchedTransformEnd,
            FrameWarning::UnclosedClip { id: open_clip },
            FrameWarning::UnclosedLayer { id: open_layer },
            FrameWarning::UnclosedTransforms { count: 1 },
        ]
    );
}

#[test]
fn end_frame_warns_about_crossed_primitive_scopes() {
    let viewport = ViewportInfo::new(
        Size::new(100.0, 50.0),
        PhysicalSize::new(100, 50),
        ScaleFactor::ONE,
    );
    let context = FrameContext::new(viewport, UiInput::default(), TimeInfo::default());
    let mut memory = UiMemory::new();
    let mut ui = Ui::begin_frame(context, &mut memory);
    let layer = LayerId::from_raw(1);
    let clip = ClipId::from_raw(2);

    ui.extend_primitives([
        Primitive::LayerBegin { id: layer },
        Primitive::ClipBegin {
            id: clip,
            rect: Rect::ZERO,
        },
        Primitive::LayerEnd { id: layer },
        Primitive::ClipEnd { id: clip },
    ]);
    let output = ui.end_frame();

    assert_eq!(
        output.warnings,
        vec![
            FrameWarning::UnmatchedLayerEnd { id: layer },
            FrameWarning::UnclosedLayer { id: layer },
        ]
    );
}

#[test]
fn end_frame_warns_about_invalid_semantic_tree() {
    let viewport = ViewportInfo::new(
        Size::new(100.0, 50.0),
        PhysicalSize::new(100, 50),
        ScaleFactor::ONE,
    );
    let context = FrameContext::new(viewport, UiInput::default(), TimeInfo::default());
    let mut memory = UiMemory::new();
    let mut ui = Ui::begin_frame(context, &mut memory);
    let root = WidgetId::from_key("root");
    let missing = WidgetId::from_key("missing");

    ui.push_semantic_node(
        SemanticNode::new(root, SemanticRole::Root, Rect::ZERO).with_children([missing]),
    );
    let output = ui.end_frame();

    assert_eq!(
        output.warnings,
        vec![FrameWarning::InvalidSemanticTree {
            error: SemanticTreeError::UnknownChild {
                parent: root,
                child: missing,
            }
        }]
    );
}

#[test]
fn ordered_text_preview_freezes_localized_ordinals_and_reuses_one_materialization() {
    let owner = WidgetId::from_key("owner");
    let mut input = UiInput::default();
    input.push_event(UiInputEvent::Wheel {
        delta: InputWheelDelta::Pixels(Vec2::new(0.0, 4.0)),
        position: Some(Point::new(90.0, 40.0)),
    });
    input.push_event(UiInputEvent::Text(TextInputEvent::Commit("A".to_owned())));
    input.push_event(UiInputEvent::PointerMoved {
        position: Point::new(90.0, 40.0),
        delta: Vec2::new(1.0, 0.0),
    });
    input.push_event(UiInputEvent::Key(KeyEvent::new(
        Key::ArrowLeft,
        KeyState::Pressed,
        Modifiers::default(),
        false,
    )));
    let mut memory = UiMemory::new();
    memory.focus(owner);
    memory.set_text_input_owner_mode(owner, TextInputOwnerMode::Editable);
    let mut ui = Ui::begin_frame(runtime_test_context(input), &mut memory);
    let clip = ClipId::from_raw(90);
    ui.push_primitive(Primitive::ClipBegin {
        id: clip,
        rect: Rect::new(0.0, 0.0, 20.0, 20.0),
    });

    assert!(ui.memory().can_claim_text_input_events(owner));
    let first = ui
        .preview_ordered_text_input_events(owner)
        .expect("valid preview")
        .expect("owner preview")
        .to_vec();
    let repeated = ui
        .preview_ordered_text_input_events(owner)
        .expect("valid repeated preview")
        .expect("owner repeated preview")
        .to_vec();
    assert_eq!(first, repeated);
    assert_eq!(
        first.iter().map(|event| event.ordinal).collect::<Vec<_>>(),
        [Some(1), Some(3)]
    );
    assert_eq!(ui.ordered_text_input_materialization_count(), 1);
    assert!(ui.memory().can_claim_text_input_events(owner));

    ui.push_primitive(Primitive::ClipEnd { id: clip });
    let claimed = ui
        .claim_ordered_text_input_events(owner)
        .expect("valid claim")
        .expect("owner claim");
    assert_eq!(claimed, first);
    assert_eq!(ui.ordered_text_input_materialization_count(), 1);
    assert!(
        ui.preview_ordered_text_input_events(owner)
            .expect("valid post-claim preview")
            .is_none()
    );
}

#[test]
fn ordered_text_preview_preserves_legacy_none_ordinals_and_owner_arbitration() {
    let owner = WidgetId::from_key("legacy-owner");
    let other = WidgetId::from_key("legacy-other");
    let input = UiInput {
        keyboard: KeyboardInput {
            modifiers: Modifiers::default(),
            events: vec![KeyEvent::new(
                Key::ArrowRight,
                KeyState::Pressed,
                Modifiers::default(),
                false,
            )],
        },
        text_events: vec![TextInputEvent::Commit("legacy".to_owned())],
        ..UiInput::default()
    };
    let mut memory = UiMemory::new();
    memory.set_text_input_owner_mode(owner, TextInputOwnerMode::ReadOnly);
    let mut ui = Ui::begin_frame(runtime_test_context(input), &mut memory);

    assert!(
        ui.preview_ordered_text_input_events(other)
            .expect("valid wrong-owner preview")
            .is_none()
    );
    assert_eq!(ui.ordered_text_input_materialization_count(), 0);
    assert!(ui.memory().can_claim_text_input_events(owner));
    let preview = ui
        .preview_ordered_text_input_events(owner)
        .expect("valid legacy preview")
        .expect("legacy owner preview")
        .to_vec();
    assert_eq!(preview.len(), 2);
    assert!(preview.iter().all(|event| event.ordinal.is_none()));
    assert!(ui.memory().can_claim_text_input_events(owner));
    let claimed = ui
        .claim_ordered_text_input_events(owner)
        .expect("valid legacy claim")
        .expect("legacy owner claim");
    assert_eq!(claimed, preview);
}

#[test]
fn ordered_text_preview_conflicts_dominate_without_consuming_claim() {
    let owner = WidgetId::from_key("conflicted-owner");
    let other = WidgetId::from_key("conflicted-other");
    let mut input = UiInput::default();
    input.push_event(UiInputEvent::Text(TextInputEvent::Commit(
        "canonical".to_owned(),
    )));
    input.text_events.clear();
    let mut memory = UiMemory::new();
    memory.set_text_input_owner_mode(owner, TextInputOwnerMode::Editable);
    let mut ui = Ui::begin_frame(runtime_test_context(input), &mut memory);

    assert_eq!(
        ui.preview_ordered_text_input_events(other),
        Err(InputStreamConflict::TextEvents)
    );
    assert_eq!(
        ui.preview_ordered_text_input_events(owner),
        Err(InputStreamConflict::TextEvents)
    );
    assert!(ui.memory().can_claim_text_input_events(owner));
    assert_eq!(ui.ordered_text_input_materialization_count(), 0);
    assert_eq!(
        ui.claim_ordered_text_input_events(owner),
        Err(InputStreamConflict::TextEvents)
    );
    assert!(!ui.memory().can_claim_text_input_events(owner));
    assert_eq!(
        ui.preview_ordered_text_input_events(owner),
        Err(InputStreamConflict::TextEvents)
    );
}

#[test]
fn ordered_text_preview_invalidates_across_aba_owner_handoff() {
    let owner = WidgetId::from_key("owner-a");
    let other = WidgetId::from_key("owner-b");
    let mut input = UiInput::default();
    input.push_event(UiInputEvent::Text(TextInputEvent::Commit("A".to_owned())));
    let mut memory = UiMemory::new();
    memory.set_text_input_owner_mode(owner, TextInputOwnerMode::Editable);
    let mut ui = Ui::begin_frame(runtime_test_context(input), &mut memory);

    let preview = ui
        .preview_ordered_text_input_events(owner)
        .expect("valid preview")
        .expect("owner preview")
        .to_vec();
    assert_eq!(ui.ordered_text_input_materialization_count(), 1);
    ui.memory_mut()
        .set_text_input_owner_mode(other, TextInputOwnerMode::Editable);
    ui.memory_mut()
        .set_text_input_owner_mode(owner, TextInputOwnerMode::Editable);
    let claimed = ui
        .claim_ordered_text_input_events(owner)
        .expect("valid claim after handoff")
        .expect("owner reclaims after handoff");

    assert_eq!(claimed, preview);
    assert_eq!(ui.ordered_text_input_materialization_count(), 2);
}

#[test]
fn selection_clicked_release_provenance_is_exact_and_ordered() {
    let id = WidgetId::from_key("selection");
    let rect = Rect::new(0.0, 0.0, 20.0, 20.0);
    let mut input = canonical_click_input(1);
    for event in canonical_click_input(2).events {
        input.push_event(event);
    }
    input.push_event(UiInputEvent::PointerButton {
        button: MouseButton::Primary,
        down: true,
        click_count: 2,
        position: Some(Point::new(5.0, 5.0)),
    });
    input.push_event(UiInputEvent::PointerMoved {
        position: Point::new(30.0, 5.0),
        delta: Vec2::new(25.0, 0.0),
    });
    input.push_event(UiInputEvent::PointerButton {
        button: MouseButton::Primary,
        down: false,
        click_count: 2,
        position: Some(Point::new(30.0, 5.0)),
    });
    let mut memory = UiMemory::new();
    let mut ui = Ui::begin_frame(runtime_test_context(input), &mut memory);
    let (gesture, clicked_releases) =
        ui.captured_selection_gesture_with_clicked_releases(id, rect, false);

    assert!(gesture.response.clicked);
    assert!(gesture.response.double_clicked);
    assert_eq!(clicked_releases, [Some(1), Some(3)]);
    assert_eq!(gesture.actions.len(), 7);
}

#[test]
fn selection_clicked_release_provenance_rejects_inexact_completions() {
    let id = WidgetId::from_key("selection");
    let rect = Rect::new(0.0, 0.0, 20.0, 20.0);
    let mut missing_position = UiInput::default();
    missing_position.push_event(UiInputEvent::PointerButton {
        button: MouseButton::Primary,
        down: true,
        click_count: 2,
        position: Some(Point::new(5.0, 5.0)),
    });
    missing_position.push_event(UiInputEvent::PointerButton {
        button: MouseButton::Primary,
        down: false,
        click_count: 2,
        position: None,
    });

    let mut cancelled = UiInput::default();
    cancelled.push_event(UiInputEvent::PointerButton {
        button: MouseButton::Primary,
        down: true,
        click_count: 2,
        position: Some(Point::new(5.0, 5.0)),
    });
    cancelled.push_event(UiInputEvent::PointerReleaseAll {
        position: Some(Point::new(5.0, 5.0)),
    });

    let mut crossed = UiInput::default();
    crossed.push_event(UiInputEvent::PointerButton {
        button: MouseButton::Primary,
        down: true,
        click_count: 2,
        position: Some(Point::new(5.0, 5.0)),
    });
    crossed.push_event(UiInputEvent::PointerMoved {
        position: Point::new(10.0, 5.0),
        delta: Vec2::new(5.0, 0.0),
    });
    crossed.push_event(UiInputEvent::PointerButton {
        button: MouseButton::Primary,
        down: false,
        click_count: 2,
        position: Some(Point::new(10.0, 5.0)),
    });

    let mut outside_below_threshold = UiInput::default();
    outside_below_threshold.push_event(UiInputEvent::PointerButton {
        button: MouseButton::Primary,
        down: true,
        click_count: 2,
        position: Some(Point::new(19.0, 5.0)),
    });
    outside_below_threshold.push_event(UiInputEvent::PointerButton {
        button: MouseButton::Primary,
        down: false,
        click_count: 2,
        position: Some(Point::new(20.5, 5.0)),
    });

    for input in [
        missing_position,
        cancelled,
        crossed,
        outside_below_threshold,
    ] {
        let mut memory = UiMemory::new();
        let mut ui = Ui::begin_frame(runtime_test_context(input), &mut memory);
        let (gesture, clicked_releases) =
            ui.captured_selection_gesture_with_clicked_releases(id, rect, false);
        assert!(!gesture.response.clicked);
        assert!(clicked_releases.is_empty());
    }
}

#[test]
fn conflicted_retained_selection_release_has_no_clicked_provenance() {
    let id = WidgetId::from_key("conflicted-retained-selection");
    let rect = Rect::new(0.0, 0.0, 20.0, 20.0);
    let mut memory = UiMemory::new();
    let pressed = UiInput {
        pointer: PointerInput {
            position: Some(Point::new(5.0, 5.0)),
            primary: PointerButtonState::new(true, true, false),
            ..PointerInput::default()
        },
        ..UiInput::default()
    };
    let mut ui = Ui::begin_frame(runtime_test_context(pressed), &mut memory);
    ui.register_id(id);
    let _ = ui.captured_selection_gesture(id, rect, false);
    let _ = ui.end_frame();

    let mut conflicted_release = UiInput::default();
    conflicted_release.push_event(UiInputEvent::PointerButton {
        button: MouseButton::Primary,
        down: false,
        click_count: 2,
        position: Some(Point::new(5.0, 5.0)),
    });
    conflicted_release.pointer.position = Some(Point::new(6.0, 5.0));
    let mut ui = Ui::begin_frame(runtime_test_context(conflicted_release), &mut memory);
    let (gesture, provenance) =
        ui.captured_selection_gesture_with_clicked_releases(id, rect, false);

    assert!(!gesture.response.clicked);
    assert!(provenance.is_empty());
    assert!(
        gesture
            .actions
            .iter()
            .all(|action| action.phase == crate::SelectionGesturePhase::Cancel)
    );
}

#[test]
fn clipped_cleanup_selection_release_has_no_clicked_provenance() {
    let id = WidgetId::from_key("clipped-cleanup-selection");
    let rect = Rect::new(0.0, 0.0, 20.0, 20.0);
    let clip = ClipId::from_raw(91);
    let clip_rect = Rect::new(0.0, 0.0, 10.0, 20.0);
    let mut pressed = UiInput::default();
    pressed.push_event(UiInputEvent::PointerButton {
        button: MouseButton::Primary,
        down: true,
        click_count: 2,
        position: Some(Point::new(9.0, 5.0)),
    });
    let mut memory = UiMemory::new();
    let mut ui = Ui::begin_frame(runtime_test_context(pressed), &mut memory);
    ui.push_primitive(Primitive::ClipBegin {
        id: clip,
        rect: clip_rect,
    });
    ui.register_id(id);
    let _ = ui.captured_selection_gesture(id, rect, false);
    ui.push_primitive(Primitive::ClipEnd { id: clip });
    let _ = ui.end_frame();

    let mut released = UiInput::default();
    released.push_event(UiInputEvent::PointerButton {
        button: MouseButton::Primary,
        down: false,
        click_count: 2,
        position: Some(Point::new(10.5, 5.0)),
    });
    let mut ui = Ui::begin_frame(runtime_test_context(released), &mut memory);
    ui.push_primitive(Primitive::ClipBegin {
        id: clip,
        rect: clip_rect,
    });
    let (gesture, provenance) =
        ui.captured_selection_gesture_with_clicked_releases(id, rect, false);

    assert!(!gesture.response.clicked);
    assert!(provenance.is_empty());
    assert!(
        gesture
            .actions
            .iter()
            .all(|action| action.phase == crate::SelectionGesturePhase::Cancel)
    );
}

#[test]
fn selection_capture_old_and_new_methods_share_one_claim() {
    let id = WidgetId::from_key("selection");
    let rect = Rect::new(0.0, 0.0, 20.0, 20.0);

    let mut memory = UiMemory::new();
    let mut ui = Ui::begin_frame(runtime_test_context(canonical_click_input(1)), &mut memory);
    let old_first = ui.captured_selection_gesture(id, rect, false);
    let (new_second, provenance) =
        ui.captured_selection_gesture_with_clicked_releases(id, rect, false);
    assert!(!old_first.actions.is_empty());
    assert!(new_second.actions.is_empty());
    assert!(provenance.is_empty());

    let mut memory = UiMemory::new();
    let mut ui = Ui::begin_frame(runtime_test_context(canonical_click_input(1)), &mut memory);
    let (new_first, provenance) =
        ui.captured_selection_gesture_with_clicked_releases(id, rect, false);
    let old_second = ui.captured_selection_gesture(id, rect, false);
    assert!(!new_first.actions.is_empty());
    assert_eq!(provenance, [Some(1)]);
    assert!(old_second.actions.is_empty());
    assert_eq!(new_first.response, old_first.response);
    assert_eq!(new_first.actions, old_first.actions);

    let mut memory = UiMemory::new();
    let mut ui = Ui::begin_frame(runtime_test_context(canonical_click_input(1)), &mut memory);
    let (first, first_provenance) =
        ui.captured_selection_gesture_with_clicked_releases(id, rect, false);
    let (second, second_provenance) =
        ui.captured_selection_gesture_with_clicked_releases(id, rect, false);
    assert!(!first.actions.is_empty());
    assert_eq!(first_provenance, [Some(1)]);
    assert!(second.actions.is_empty());
    assert!(second_provenance.is_empty());
}

#[test]
fn selection_claim_blocks_recovered_cancel_after_mode_mismatch() {
    let id = WidgetId::from_key("selection-mismatch");
    let rect = Rect::new(0.0, 0.0, 20.0, 20.0);
    let pressed = UiInput {
        pointer: PointerInput {
            position: Some(Point::new(5.0, 5.0)),
            primary: PointerButtonState::new(true, true, false),
            ..PointerInput::default()
        },
        ..UiInput::default()
    };

    let mut memory = UiMemory::new();
    let mut ui = Ui::begin_frame(runtime_test_context(pressed.clone()), &mut memory);
    let (first, provenance) = ui.captured_selection_gesture_with_clicked_releases(id, rect, false);
    assert!(!first.actions.is_empty());
    assert!(provenance.is_empty());
    let _ = ui.captured_domain_drag_gesture(id, rect, false);
    let repeated = ui.captured_selection_gesture(id, rect, false);
    assert!(repeated.actions.is_empty());

    let mut memory = UiMemory::new();
    let mut ui = Ui::begin_frame(runtime_test_context(pressed), &mut memory);
    let first = ui.captured_selection_gesture(id, rect, false);
    assert!(!first.actions.is_empty());
    let _ = ui.captured_domain_drag_gesture(id, rect, false);
    let (repeated, provenance) =
        ui.captured_selection_gesture_with_clicked_releases(id, rect, false);
    assert!(repeated.actions.is_empty());
    assert!(provenance.is_empty());
}

#[test]
fn legacy_retained_selection_release_reports_none_provenance() {
    let id = WidgetId::from_key("legacy-selection");
    let rect = Rect::new(0.0, 0.0, 20.0, 20.0);
    let mut memory = UiMemory::new();
    let pressed = UiInput {
        pointer: PointerInput {
            position: Some(Point::new(5.0, 5.0)),
            primary: PointerButtonState::new(true, true, false),
            click_count: 2,
            ..PointerInput::default()
        },
        ..UiInput::default()
    };
    let mut ui = Ui::begin_frame(runtime_test_context(pressed), &mut memory);
    ui.register_id(id);
    let (_, provenance) = ui.captured_selection_gesture_with_clicked_releases(id, rect, false);
    assert!(provenance.is_empty());
    let _ = ui.end_frame();

    let released = UiInput {
        pointer: PointerInput {
            position: Some(Point::new(5.0, 5.0)),
            primary: PointerButtonState::new(false, false, true),
            click_count: 2,
            ..PointerInput::default()
        },
        ..UiInput::default()
    };
    let mut ui = Ui::begin_frame(runtime_test_context(released), &mut memory);
    ui.register_id(id);
    let (gesture, provenance) =
        ui.captured_selection_gesture_with_clicked_releases(id, rect, false);
    assert!(gesture.response.double_clicked);
    assert_eq!(provenance, [None]);
}
