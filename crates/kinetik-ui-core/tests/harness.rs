//! Windowless core harness coverage.

use std::time::Duration;

use kinetik_ui_core::{
    ActionContext, ActionId, ActionSource, Brush, Color, CornerRadius, CursorShape, FrameWarning,
    HarnessPhase, Key, KeyState, Modifiers, MouseButton, PlatformRequest, Point, Primitive, Rect,
    RectPrimitive, RepaintRequest, ScaleFactor, ScriptedInput, SemanticNode, SemanticRole,
    SettlePendingCause, Size, TextInputEvent, TextRange, UiTestHarness, Vec2, WidgetId,
};

#[test]
fn harness_runs_two_frames_with_stable_memory() {
    let mut harness = UiTestHarness::with_viewport(Size::new(320.0, 200.0), ScaleFactor::new(2.0));
    let focused = WidgetId::from_key("field");
    let hovered = WidgetId::from_key("hovered");
    let scroll = WidgetId::from_key("scroll");

    assert_eq!(harness.viewport().logical_size, Size::new(320.0, 200.0));
    assert_eq!(harness.viewport().scale_factor, ScaleFactor::new(2.0));
    assert_eq!(harness.viewport().physical_size.width, 640);
    assert_eq!(harness.viewport().physical_size.height, 400);
    assert_eq!(harness.time().frame_index, 0);
    assert_eq!(harness.last_output(), None);
    assert_eq!(harness.last_primitives(), None);
    assert_eq!(harness.last_semantics(), None);
    assert_eq!(harness.last_actions(), None);
    assert_eq!(harness.last_platform_requests(), None);
    assert_eq!(harness.last_repaint(), None);
    assert_eq!(harness.last_warnings(), None);

    let (first_time, first_output) = harness.run_frame(|ui| {
        ui.register_id(focused);
        ui.memory_mut().focus(focused);
        ui.memory_mut().set_hovered(hovered);
        ui.memory_mut()
            .set_scroll_offset(scroll, Vec2::new(12.0, 24.0));
        ui.context().time
    });

    assert_eq!(first_time.frame_index, 0);
    assert_eq!(first_time.now, Duration::ZERO);
    assert_eq!(first_output.warnings, Vec::new());
    assert_eq!(harness.memory().focused(), Some(focused));
    assert_eq!(harness.memory().hovered(), Some(hovered));
    assert_eq!(harness.last_output(), Some(&first_output));

    harness.advance_frame(Duration::from_millis(16));
    let (second, _) = harness.run_frame(|ui| {
        ui.register_id(focused);
        (
            ui.context().time,
            ui.memory().focused(),
            ui.memory().hovered(),
            ui.memory().scroll_offset(scroll),
        )
    });

    assert_eq!(second.0.frame_index, 1);
    assert_eq!(second.0.now, Duration::from_millis(16));
    assert_eq!(second.0.delta, Duration::from_millis(16));
    assert_eq!(second.1, Some(focused));
    assert_eq!(second.2, None);
    assert_eq!(second.3, Vec2::new(12.0, 24.0));

    harness.set_viewport(Size::new(200.0, 120.0), ScaleFactor::new(1.5));
    assert_eq!(harness.viewport().logical_size, Size::new(200.0, 120.0));
    assert_eq!(harness.viewport().scale_factor, ScaleFactor::new(1.5));
    assert_eq!(harness.viewport().physical_size.width, 300);
    assert_eq!(harness.viewport().physical_size.height, 180);
}

#[test]
fn input_events_are_visible_only_in_the_intended_frame() {
    let mut harness = UiTestHarness::new();
    let field = WidgetId::from_key("field");

    harness.set_pointer_position(Point::new(10.0, 20.0));
    harness.pointer_press(MouseButton::Primary);
    harness.set_click_count(2);
    harness.wheel(Vec2::new(1.0, -2.0));
    harness.set_modifiers(Modifiers::new(false, true, false, false));
    harness.key_press(Key::Character("s".to_owned()));
    harness.text_composition_start();
    harness.text_composition("preedit", Some(TextRange::new(0, 3)));
    harness.text_commit("saved");
    harness.text_composition_end();
    harness.clipboard_text(field, "paste");
    harness.set_window_focused(false);

    let (pressed_frame, _) = harness.run_frame(|ui| ui.input().clone());

    assert_eq!(pressed_frame.pointer.position, None);
    assert_eq!(pressed_frame.pointer.click_count, 2);
    assert!(!pressed_frame.pointer.primary.down);
    assert!(pressed_frame.pointer.primary.pressed);
    assert!(pressed_frame.pointer.primary.released);
    assert_eq!(pressed_frame.pointer.wheel_delta, Vec2::new(1.0, -2.0));
    assert_eq!(pressed_frame.keyboard.events.len(), 1);
    assert_eq!(pressed_frame.keyboard.events[0].state, KeyState::Pressed);
    assert_eq!(pressed_frame.text_events.len(), 4);
    assert!(matches!(
        &pressed_frame.text_events[1],
        TextInputEvent::Composition {
            selection: Some(TextRange { start: 0, end: 3 }),
            ..
        }
    ));
    assert_eq!(pressed_frame.clipboard_text.len(), 1);
    assert!(!pressed_frame.window_focused);

    let (held_frame, _) = harness.run_frame(|ui| ui.input().clone());

    assert!(!held_frame.pointer.primary.down);
    assert_eq!(held_frame.pointer.click_count, 0);
    assert!(!held_frame.pointer.primary.pressed);
    assert!(!held_frame.pointer.primary.released);
    assert_eq!(held_frame.pointer.wheel_delta, Vec2::ZERO);
    assert!(held_frame.keyboard.events.is_empty());
    assert!(held_frame.text_events.is_empty());
    assert!(held_frame.clipboard_text.is_empty());
    assert_eq!(
        held_frame.keyboard.modifiers,
        pressed_frame.keyboard.modifiers
    );
    assert!(!held_frame.window_focused);

    harness.pointer_release(MouseButton::Primary);
    harness.key_release(Key::Character("s".to_owned()));
    let (released_frame, _) = harness.run_frame(|ui| ui.input().clone());

    assert!(!released_frame.pointer.primary.down);
    assert!(!released_frame.pointer.primary.pressed);
    assert!(released_frame.pointer.primary.released);
    assert_eq!(released_frame.keyboard.events.len(), 1);
    assert_eq!(released_frame.keyboard.events[0].state, KeyState::Released);

    let (idle_frame, _) = harness.run_frame(|ui| ui.input().clone());

    assert!(!idle_frame.pointer.primary.down);
    assert!(!idle_frame.pointer.primary.pressed);
    assert!(!idle_frame.pointer.primary.released);
    assert!(idle_frame.keyboard.events.is_empty());
}

#[test]
fn frame_output_channels_are_inspectable_and_deterministic() {
    let mut harness = UiTestHarness::new();

    let (registered_id, output) = harness.run_frame(|ui| {
        let id = ui.id("run");
        ui.register_id(id);
        ui.push_primitive(Primitive::Rect(RectPrimitive {
            rect: Rect::new(1.0, 2.0, 30.0, 20.0),
            fill: Some(Brush::Solid(Color::WHITE)),
            stroke: None,
            radius: CornerRadius::all(2.0),
        }));
        ui.push_semantic_node(
            SemanticNode::new(id, SemanticRole::Button, Rect::new(1.0, 2.0, 30.0, 20.0))
                .focusable(true)
                .with_label("Run"),
        );
        ui.invoke_action(
            ActionId::new("project.run"),
            ActionSource::Button,
            ActionContext::Widget(id),
        );
        ui.push_platform_request(PlatformRequest::SetCursor(CursorShape::PointingHand));
        ui.request_repaint(RepaintRequest::After(Duration::from_millis(50)));
        id
    });

    assert_eq!(output.primitives.len(), 1);
    assert_eq!(output.semantics.root(), Some(registered_id));
    assert_eq!(output.semantics.focus_order(), vec![registered_id]);
    assert_eq!(
        output
            .semantics
            .get(registered_id)
            .and_then(|node| node.label.as_deref()),
        Some("Run")
    );
    assert_eq!(output.actions.len(), 1);
    assert_eq!(output.repaint, RepaintRequest::NextFrame);
    assert_eq!(
        output.platform_requests,
        vec![PlatformRequest::SetCursor(CursorShape::PointingHand)]
    );
    assert_eq!(
        output.warnings,
        vec![FrameWarning::DuplicateWidgetId { id: registered_id }]
    );
    assert_eq!(harness.last_output(), Some(&output));
    assert_eq!(
        harness.last_primitives(),
        Some(output.primitives.as_slice())
    );
    assert_eq!(harness.last_semantics(), Some(&output.semantics));
    assert_eq!(harness.last_actions(), Some(&output.actions));
    assert_eq!(
        harness.last_platform_requests(),
        Some(output.platform_requests.as_slice())
    );
    assert_eq!(harness.last_repaint(), Some(output.repaint));
    assert_eq!(harness.last_warnings(), Some(output.warnings.as_slice()));
}

#[test]
fn traced_frame_records_deterministic_phase_order_without_mutating_output() {
    let mut harness = UiTestHarness::new();

    let (_, output, trace) = harness.run_frame_with_trace(|ui| {
        let id = ui.id("run");
        ui.invoke_action(
            ActionId::new("project.run"),
            ActionSource::Button,
            ActionContext::Widget(id),
        );
        id
    });

    assert_eq!(
        trace.phases(),
        &[
            HarnessPhase::FrameBegin,
            HarnessPhase::Build,
            HarnessPhase::FrameFinalization,
            HarnessPhase::InspectSemantics,
            HarnessPhase::InspectActions,
            HarnessPhase::InspectPlatformRequests,
            HarnessPhase::InspectRepaint,
            HarnessPhase::InspectWarnings,
        ]
    );
    assert_eq!(output.actions.len(), 1);
    assert_eq!(harness.last_output(), Some(&output));
    assert_eq!(harness.last_actions(), Some(&output.actions));
}

#[test]
fn traced_scripted_frame_records_input_preparation_when_applicable() {
    let mut harness = UiTestHarness::new();

    let (pointer, _, trace) = harness
        .run_scripted_frame_with_trace([ScriptedInput::PointerMove(Point::new(4.0, 8.0))], |ui| {
            ui.input().pointer.position
        });

    assert_eq!(pointer, Some(Point::new(4.0, 8.0)));
    assert_eq!(
        trace.phases(),
        &[
            HarnessPhase::ScriptedInputPrep,
            HarnessPhase::FrameBegin,
            HarnessPhase::Build,
            HarnessPhase::FrameFinalization,
            HarnessPhase::InspectSemantics,
            HarnessPhase::InspectActions,
            HarnessPhase::InspectPlatformRequests,
            HarnessPhase::InspectRepaint,
            HarnessPhase::InspectWarnings,
        ]
    );
}

#[test]
fn idle_frame_settles_immediately_within_budget() {
    let mut harness = UiTestHarness::new();
    let mut builds = 0;

    let result = harness.settle_frames(4, |_| {
        builds += 1;
    });

    assert!(result.is_idle());
    assert_eq!(result.frames_run(), 1);
    assert_eq!(builds, 1);
    assert_eq!(result.pending_cause(), None);
    assert_eq!(
        result.trace().phases(),
        &[
            HarnessPhase::FrameBegin,
            HarnessPhase::Build,
            HarnessPhase::FrameFinalization,
            HarnessPhase::InspectSemantics,
            HarnessPhase::InspectActions,
            HarnessPhase::InspectPlatformRequests,
            HarnessPhase::InspectRepaint,
            HarnessPhase::InspectWarnings,
        ]
    );
}

#[test]
fn repaint_request_exhausts_settle_budget_without_sleeping() {
    let mut harness = UiTestHarness::new();
    let delay = Duration::from_millis(50);

    let result = harness.settle_frames(2, |ui| {
        ui.request_repaint(RepaintRequest::After(delay));
    });

    assert!(!result.is_idle());
    assert_eq!(result.frames_run(), 2);
    assert_eq!(
        result.pending_cause(),
        Some(SettlePendingCause::Repaint(RepaintRequest::After(delay)))
    );
    assert_eq!(harness.last_repaint(), Some(RepaintRequest::After(delay)));
}

#[test]
fn continuous_repaint_reports_repaint_pending_cause() {
    let mut harness = UiTestHarness::new();

    let result = harness.settle_frames(1, |ui| {
        ui.request_repaint(RepaintRequest::Continuous);
    });

    assert_eq!(
        result.pending_cause(),
        Some(SettlePendingCause::Repaint(RepaintRequest::Continuous))
    );
}

#[test]
fn next_frame_repaint_reports_repaint_pending_cause() {
    let mut harness = UiTestHarness::new();

    let result = harness.settle_frames(2, |ui| {
        ui.request_repaint(RepaintRequest::NextFrame);
    });

    assert!(!result.is_idle());
    assert_eq!(result.frames_run(), 2);
    assert_eq!(
        result.pending_cause(),
        Some(SettlePendingCause::Repaint(RepaintRequest::NextFrame))
    );
    assert_eq!(harness.last_repaint(), Some(RepaintRequest::NextFrame));
}

#[test]
fn action_output_reports_actions_pending_cause_and_remains_inspectable() {
    let mut harness = UiTestHarness::new();

    let result = harness.settle_frames(1, |ui| {
        ui.invoke_action(
            ActionId::new("project.run"),
            ActionSource::Button,
            ActionContext::Global,
        );
    });

    assert_eq!(result.pending_cause(), Some(SettlePendingCause::Actions));
    assert_eq!(harness.last_actions().expect("actions").len(), 1);
    assert_eq!(harness.last_repaint(), Some(RepaintRequest::NextFrame));
}

#[test]
fn platform_request_output_reports_platform_pending_cause() {
    let mut harness = UiTestHarness::new();

    let result = harness.settle_frames(1, |ui| {
        ui.push_platform_request(PlatformRequest::SetCursor(CursorShape::PointingHand));
    });

    assert_eq!(
        result.pending_cause(),
        Some(SettlePendingCause::PlatformRequests)
    );
    assert_eq!(
        harness.last_platform_requests(),
        Some([PlatformRequest::SetCursor(CursorShape::PointingHand)].as_slice())
    );
}

#[test]
fn warning_output_reports_warnings_pending_cause() {
    let mut harness = UiTestHarness::new();
    let mut duplicate = None;

    let result = harness.settle_frames(1, |ui| {
        let id = ui.id("duplicate");
        ui.register_id(id);
        duplicate = Some(id);
    });

    assert_eq!(result.pending_cause(), Some(SettlePendingCause::Warnings));
    assert_eq!(
        harness.last_warnings(),
        Some(
            [FrameWarning::DuplicateWidgetId {
                id: duplicate.expect("duplicate id"),
            }]
            .as_slice()
        )
    );
}

#[test]
fn scripted_settle_applies_script_once_then_runs_until_idle() {
    let mut harness = UiTestHarness::new();
    let mut builds = 0;
    harness.set_pointer_position(Point::new(8.0, 16.0));

    let result = harness.settle_scripted_frames(
        [ScriptedInput::PointerMove(Point::new(12.0, 24.0))],
        3,
        |ui| {
            builds += 1;
            if ui.input().pointer.delta != Vec2::ZERO {
                ui.request_repaint(RepaintRequest::NextFrame);
            }
        },
    );

    assert!(result.is_idle());
    assert_eq!(result.frames_run(), 2);
    assert_eq!(builds, 2);
    assert_eq!(
        harness.input().pointer.position,
        Some(Point::new(12.0, 24.0))
    );
    assert_eq!(harness.input().pointer.delta, Vec2::ZERO);
    assert_eq!(result.trace().phases()[0], HarnessPhase::ScriptedInputPrep);
}
