//! Explicit application-owned timeline journey through the public Stern facade.

use stern::core::{
    ActionContext, ActionDescriptor, ActionInvocation, ActionSource, FrameOutput, Key, KeyEvent,
    KeyState, KeyboardInput, Modifiers, Point, PointerButtonState, PointerInput, SemanticNode,
    SemanticRole, SemanticValue, UiInput, UiInputEvent, Vec2,
};
use stern_demo::{
    DemoActionRegistry, DemoApp, DemoApplicationModel, DemoJobPhase, DemoScenario,
    DemoTransportState, demo_context,
};

#[test]
fn default_constructor_preserves_the_explicit_default_scenario_frame() {
    let mut default_app = DemoApp::new();
    let mut explicit_default = DemoApp::for_scenario(DemoScenario::Default);
    assert_eq!(
        default_app.frame(demo_context(UiInput::default())),
        explicit_default.frame(demo_context(UiInput::default()))
    );
}

#[test]
fn timeline_scenario_owns_stable_time_clip_keyframes_and_transport() {
    let default_model = DemoApplicationModel::new();
    let default_actions = DemoActionRegistry::new();
    assert_eq!(default_model.scenario(), DemoScenario::Default);
    assert!(default_model.timeline().keyframes().is_empty());
    assert!(!default_actions.transport_play_pause().state.visible);
    assert!(!default_actions.transport_stop().state.visible);

    let actions = DemoActionRegistry::for_scenario(DemoScenario::TimelineJourney);
    let mut model = DemoApplicationModel::for_scenario(DemoScenario::TimelineJourney);
    let timeline = model.timeline();
    assert_eq!(timeline.frame_rate().numerator, 30);
    assert_eq!(timeline.frame_rate().denominator, 1);
    assert_eq!(timeline.frame_range(), (0, 240));
    assert_eq!(timeline.clip_id(), 1);
    assert_eq!(timeline.clip_label(), "Hero clip");
    assert_eq!(timeline.clip_frames(), (30, 90));
    assert_eq!(
        timeline
            .keyframes()
            .iter()
            .map(|keyframe| (keyframe.id(), keyframe.frame(), keyframe.label()))
            .collect::<Vec<_>>(),
        [
            (101, 36, "Position A"),
            (102, 60, "Position B"),
            (103, 84, "Position C"),
        ]
    );
    assert_eq!(model.timeline().position().frame(), 24);
    assert_eq!(
        model.timeline().position().time().seconds().to_bits(),
        0.8_f64.to_bits()
    );
    assert_eq!(model.transport_state(), DemoTransportState::Stopped);
    assert!(actions.transport_play_pause().state.visible);
    assert!(actions.transport_stop().state.visible);

    assert!(model.execute(&invocation(actions.transport_play_pause())));
    assert_eq!(model.transport_state(), DemoTransportState::Playing);
    assert!(model.execute(&invocation(actions.transport_play_pause())));
    assert_eq!(model.transport_state(), DemoTransportState::Paused);
    assert!(model.execute(&invocation(actions.transport_play_pause())));
    assert_eq!(model.transport_state(), DemoTransportState::Playing);
    assert!(model.execute(&invocation(actions.transport_stop())));
    assert_eq!(model.transport_state(), DemoTransportState::Stopped);
}

#[test]
fn timeline_viewport_status_and_transport_project_one_application_position() {
    let mut app = DemoApp::for_scenario(DemoScenario::TimelineJourney);
    let initial = app.frame(demo_context(UiInput::default()));
    let position = app.timeline_position().label();

    assert_eq!(
        custom_node(&initial, "timeline", "Timeline").state.value,
        Some(SemanticValue::Text(
            "1 lanes, 1 items, 0 markers, 3 keyframes".to_owned()
        ))
    );
    for label in [
        "Position A · frame 36",
        "Position B · frame 60",
        "Position C · frame 84",
    ] {
        custom_node(&initial, "timeline-keyframe", label);
    }
    node(
        &initial,
        &SemanticRole::Viewport,
        &format!("Viewport · {position}"),
    );
    let status_label = format!("{position} · Stopped");
    assert!(
        has_label(&initial, &status_label),
        "labels: {:?}",
        initial
            .semantics
            .nodes()
            .iter()
            .filter_map(|node| node.label.as_deref())
            .collect::<Vec<_>>()
    );
    assert!(label_node(&initial, "Stop").state.disabled);

    let played = click_label(&mut app, &initial, "Play");
    assert_eq!(action_count(&played, "transport.play-pause"), 1);
    assert_eq!(app.transport_state(), DemoTransportState::Playing);
    let playing = app.frame(demo_context(UiInput::default()));
    assert!(has_label(&playing, &format!("{position} · Playing")));

    let paused = click_label(&mut app, &playing, "Pause");
    assert_eq!(action_count(&paused, "transport.play-pause"), 1);
    assert_eq!(app.transport_state(), DemoTransportState::Paused);
    let paused = app.frame(demo_context(UiInput::default()));
    assert!(has_label(&paused, &format!("{position} · Paused")));

    let _ = click_label(&mut app, &paused, "Play");
    assert_eq!(app.transport_state(), DemoTransportState::Playing);
    let playing = app.frame(demo_context(UiInput::default()));
    let stopped = click_label(&mut app, &playing, "Stop");
    assert_eq!(action_count(&stopped, "transport.stop"), 1);
    assert_eq!(app.transport_state(), DemoTransportState::Stopped);
}

#[test]
fn playhead_and_clip_edits_distinguish_commit_escape_and_capture_loss() {
    let mut app = DemoApp::for_scenario(DemoScenario::TimelineJourney);
    let initial = app.frame(demo_context(UiInput::default()));
    let timeline = custom_node(&initial, "timeline", "Timeline");
    let ruler = Point::new(timeline.bounds.x + 110.0, timeline.bounds.y + 10.0);

    let committed_playhead = app.committed_playhead_frame();
    let preview = Point::new(ruler.x + 36.0, ruler.y);
    begin_drag(&mut app, ruler, preview, 36.0);
    assert_ne!(app.playhead_frame(), committed_playhead);
    let _ = app.frame(demo_context(pointer(preview, false, false, true)));
    assert_eq!(app.playhead_frame(), app.committed_playhead_frame());
    assert_ne!(app.committed_playhead_frame(), committed_playhead);

    let committed_playhead = app.committed_playhead_frame();
    let frame = app.frame(demo_context(UiInput::default()));
    let timeline = custom_node(&frame, "timeline", "Timeline");
    let ruler = Point::new(timeline.bounds.x + 120.0, timeline.bounds.y + 10.0);
    let preview = Point::new(ruler.x + 30.0, ruler.y);
    begin_drag(&mut app, ruler, preview, 30.0);
    assert_ne!(app.playhead_frame(), committed_playhead);
    let _ = app.frame(demo_context(escape_while_dragging(preview)));
    assert_eq!(app.playhead_frame(), committed_playhead);
    assert_eq!(app.committed_playhead_frame(), committed_playhead);

    let frame = app.frame(demo_context(UiInput::default()));
    let timeline = custom_node(&frame, "timeline", "Timeline");
    let ruler = Point::new(timeline.bounds.x + 130.0, timeline.bounds.y + 10.0);
    let preview = Point::new(ruler.x + 24.0, ruler.y);
    begin_drag(&mut app, ruler, preview, 24.0);
    assert_ne!(app.playhead_frame(), committed_playhead);
    let _ = app.frame(demo_context(capture_lost(preview)));
    assert_eq!(app.playhead_frame(), committed_playhead);
    assert_eq!(app.committed_playhead_frame(), committed_playhead);

    let committed_clip = app.committed_clip_frames();
    let frame = app.frame(demo_context(UiInput::default()));
    let clip = custom_node(&frame, "timeline-item", "Hero clip");
    let start = Point::new(
        clip.bounds.x + clip.bounds.width * 0.35,
        clip.bounds.center().y,
    );
    let preview = Point::new(start.x + 24.0, start.y);
    begin_drag(&mut app, start, preview, 24.0);
    assert_ne!(app.clip_frames(), committed_clip);
    let _ = app.frame(demo_context(pointer(preview, false, false, true)));
    assert_eq!(app.clip_frames(), app.committed_clip_frames());
    assert_ne!(app.committed_clip_frames(), committed_clip);

    let committed_clip = app.committed_clip_frames();
    let frame = app.frame(demo_context(UiInput::default()));
    let clip = custom_node(&frame, "timeline-item", "Hero clip");
    let trim = Point::new(clip.bounds.x + 1.0, clip.bounds.center().y);
    let preview = Point::new(trim.x + 12.0, trim.y);
    begin_drag(&mut app, trim, preview, 12.0);
    assert_ne!(app.clip_frames(), committed_clip);
    let _ = app.frame(demo_context(escape_while_dragging(preview)));
    assert_eq!(app.clip_frames(), committed_clip);
    assert_eq!(app.committed_clip_frames(), committed_clip);

    let frame = app.frame(demo_context(UiInput::default()));
    let clip = custom_node(&frame, "timeline-item", "Hero clip");
    let start = Point::new(
        clip.bounds.x + clip.bounds.width * 0.45,
        clip.bounds.center().y,
    );
    let preview = Point::new(start.x + 18.0, start.y);
    begin_drag(&mut app, start, preview, 18.0);
    assert_ne!(app.clip_frames(), committed_clip);
    let _ = app.frame(demo_context(capture_lost(preview)));
    assert_eq!(app.clip_frames(), committed_clip);
    assert_eq!(app.committed_clip_frames(), committed_clip);
}

#[test]
fn job_progress_success_and_failure_preserve_the_committed_timeline() {
    let mut app = DemoApp::for_scenario(DemoScenario::TimelineJourney);
    let running = app.frame(demo_context(UiInput::default()));
    assert!(matches!(
        &custom_node(&running, "job", "Preview render").state.value,
        Some(SemanticValue::Number { current, .. }) if current.to_bits() == 0.4_f32.to_bits()
    ));
    let committed_playhead = app.committed_playhead_frame();
    let committed_clip = app.committed_clip_frames();

    app.set_job(DemoJobPhase::Succeeded, 100);
    let succeeded = app.frame(demo_context(UiInput::default()));
    custom_node(&succeeded, "notification", "Preview complete");
    assert_eq!(app.committed_playhead_frame(), committed_playhead);
    assert_eq!(app.committed_clip_frames(), committed_clip);

    app.set_job(DemoJobPhase::Failed, 65);
    let failed = app.frame(demo_context(UiInput::default()));
    custom_node(&failed, "notification", "Preview failed");
    assert_eq!(app.committed_playhead_frame(), committed_playhead);
    assert_eq!(app.committed_clip_frames(), committed_clip);
}

fn invocation(action: &ActionDescriptor) -> ActionInvocation {
    ActionInvocation::new(
        action.id.clone(),
        ActionSource::Button,
        ActionContext::Editor,
    )
}

fn node<'a>(output: &'a FrameOutput, role: &SemanticRole, label: &str) -> &'a SemanticNode {
    output
        .semantics
        .nodes()
        .iter()
        .find(|node| &node.role == role && node.label.as_deref() == Some(label))
        .unwrap_or_else(|| panic!("semantic node {role:?} {label}"))
}

fn custom_node<'a>(output: &'a FrameOutput, role: &str, label: &str) -> &'a SemanticNode {
    node(output, &SemanticRole::Custom(role.to_owned()), label)
}

fn label_node<'a>(output: &'a FrameOutput, label: &str) -> &'a SemanticNode {
    output
        .semantics
        .nodes()
        .iter()
        .find(|node| node.label.as_deref() == Some(label))
        .unwrap_or_else(|| panic!("semantic node {label}"))
}

fn has_label(output: &FrameOutput, label: &str) -> bool {
    output
        .semantics
        .nodes()
        .iter()
        .any(|node| node.label.as_deref() == Some(label))
}

fn action_count(output: &FrameOutput, id: &str) -> usize {
    let mut actions = output.actions.clone();
    actions
        .drain()
        .filter(|action| action.action_id.as_str() == id)
        .count()
}

fn click_label(app: &mut DemoApp, output: &FrameOutput, label: &str) -> FrameOutput {
    let point = label_node(output, label).bounds.center();
    let _ = app.frame(demo_context(pointer(point, true, true, false)));
    app.frame(demo_context(pointer(point, false, false, true)))
}

fn begin_drag(app: &mut DemoApp, start: Point, preview: Point, delta_x: f32) {
    let _ = app.frame(demo_context(pointer(start, true, true, false)));
    let _ = app.frame(demo_context(drag(preview, delta_x)));
}

fn pointer(point: Point, down: bool, pressed: bool, released: bool) -> UiInput {
    UiInput {
        pointer: PointerInput {
            position: Some(point),
            primary: PointerButtonState::new(down, pressed, released),
            ..PointerInput::default()
        },
        ..UiInput::default()
    }
}

fn drag(point: Point, delta_x: f32) -> UiInput {
    UiInput {
        pointer: PointerInput {
            position: Some(point),
            delta: Vec2::new(delta_x, 0.0),
            primary: PointerButtonState::new(true, false, false),
            ..PointerInput::default()
        },
        ..UiInput::default()
    }
}

fn escape_while_dragging(point: Point) -> UiInput {
    let mut input = key(Key::Escape);
    input.pointer.position = Some(point);
    input.pointer.primary = PointerButtonState::new(true, false, false);
    input.events.push(UiInputEvent::Key(KeyEvent::new(
        Key::Escape,
        KeyState::Pressed,
        Modifiers::default(),
        false,
    )));
    input
}

fn capture_lost(point: Point) -> UiInput {
    let mut input = UiInput {
        pointer: PointerInput {
            position: Some(point),
            primary: PointerButtonState::new(true, false, false),
            ..PointerInput::default()
        },
        ..UiInput::default()
    };
    input.push_event(UiInputEvent::WindowFocusChanged(false));
    input
}

fn key(key: Key) -> UiInput {
    let event = KeyEvent::new(key, KeyState::Pressed, Modifiers::default(), false);
    UiInput {
        keyboard: KeyboardInput {
            modifiers: Modifiers::default(),
            events: vec![event],
        },
        ..UiInput::default()
    }
}
