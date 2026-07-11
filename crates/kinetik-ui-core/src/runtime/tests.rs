use std::time::Duration;

use super::{
    CursorShape, FrameContext, FrameOutput, FrameWarning, PlatformRequest, RepaintRequest,
    TimeInfo, Ui, ViewportInfo,
};
use crate::input::UiInput;
use crate::{
    ActionContext, ActionId, ActionSource, Brush, ClipId, Color, CornerRadius, LayerId,
    PhysicalSize, Primitive, Rect, RectPrimitive, ScaleFactor, SemanticNode, SemanticRole,
    SemanticTreeError, Size, Transform, UiMemory, WidgetId,
};

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
fn ui_builder_cancels_pointer_interaction_on_release_all_at_frame_start() {
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
