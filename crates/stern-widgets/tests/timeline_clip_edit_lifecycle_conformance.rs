//! Retained timeline clip move/trim lifecycle conformance.
use std::time::Duration;

use stern_core::{
    FrameContext, Key, KeyEvent, KeyState, Modifiers, MouseButton, PhysicalSize, Point,
    PointerOrder, Rect, ScaleFactor, Size, TimeInfo, UiInput, UiInputEvent, UiMemory, Vec2,
    ViewportInfo, WidgetId, default_dark_theme,
};
use stern_widgets::{
    TimelineClipEditController, TimelineClipEditIntent, TimelineClipEditKind,
    TimelineClipEditRejectionReason, TimelineClipEditRejectionStage, TimelineClipEditRequest,
    TimelineDescriptor, TimelineDescriptorState, TimelineFrameRate, TimelineItemDescriptor,
    TimelineItemId, TimelineLaneDescriptor, TimelineLaneId, TimelineRange, TimelineScale,
    TimelineTime, TimelineTrimEdge, TimelineViewportState, TimelineWidgetConfig,
    TimelineWidgetOutput, TimelineZoom, Ui,
};

const ROOT: WidgetId = WidgetId::from_raw(0x832);
const ITEM: TimelineItemId = TimelineItemId::from_raw(10);
const LANE: TimelineLaneId = TimelineLaneId::from_raw(1);
const BOUNDS: Rect = Rect::new(0.0, 0.0, 320.0, 120.0);

fn descriptor(range: TimelineRange, state: TimelineDescriptorState) -> TimelineDescriptor {
    TimelineDescriptor::new(
        [TimelineLaneDescriptor::new(LANE, "Video")],
        [TimelineItemDescriptor::new(ITEM, LANE, range, "Clip").with_state(state)],
        [],
        [],
    )
}

fn state(zoom: f32) -> TimelineViewportState {
    TimelineViewportState::new(TimelineScale::new(
        999.0,
        1.0,
        TimelineRange::seconds(0.0, 10.0),
        TimelineZoom::new(zoom),
        0.0,
    ))
}

fn context(input: UiInput) -> FrameContext {
    FrameContext::new(
        ViewportInfo::new(
            Size::new(400.0, 200.0),
            PhysicalSize::new(400, 200),
            ScaleFactor::ONE,
        ),
        input,
        TimeInfo::new(Duration::ZERO, Duration::ZERO, 0),
    )
}

fn run<'a>(
    descriptor: &'a TimelineDescriptor,
    state: &'a TimelineViewportState,
    controller: &mut TimelineClipEditController,
    memory: &mut UiMemory,
    input: UiInput,
    configure: impl FnOnce(TimelineWidgetConfig<'a>) -> TimelineWidgetConfig<'a>,
) -> (Vec<TimelineClipEditIntent>, stern_core::FrameOutput) {
    let theme = default_dark_theme();
    let mut ui = Ui::begin_frame(context(input), memory, &theme);
    let config = configure(
        TimelineWidgetConfig::new(
            ROOT,
            BOUNDS,
            TimelineFrameRate::integer(24),
            descriptor,
            state,
        )
        .with_lane_header_width(80.0)
        .with_ruler_height(20.0),
    );
    let widget = ui.prepare_timeline_widget(config).expect("valid timeline");
    ui.resolve_pointer_targets(|plan| {
        widget.declare_pointer_targets(plan, PointerOrder::new(10));
    })
    .expect("valid pointer plan");
    let output = ui.timeline_widget_with_clip_edit(&widget, controller);
    let frame = ui.finish_output();
    (output.clip_edit_intents, frame)
}

fn pointer(point: Point, down: bool) -> UiInput {
    let mut input = UiInput::default();
    input.push_event(UiInputEvent::PointerButton {
        button: MouseButton::Primary,
        down,
        click_count: 1,
        position: Some(point),
    });
    input
}

fn pointer_move(point: Point, delta: Vec2) -> UiInput {
    let mut input = UiInput::default();
    input.pointer.primary.down = true;
    input.push_event(UiInputEvent::PointerMoved {
        position: point,
        delta,
    });
    input
}

fn release_without_position() -> UiInput {
    let mut input = UiInput::default();
    input.push_event(UiInputEvent::PointerButton {
        button: MouseButton::Primary,
        down: false,
        click_count: 1,
        position: None,
    });
    input
}

fn escape(point: Point) -> UiInput {
    let mut input = UiInput::default();
    input.pointer.position = Some(point);
    input.pointer.primary.down = true;
    input.push_event(UiInputEvent::Key(KeyEvent::new(
        Key::Escape,
        KeyState::Pressed,
        Modifiers::default(),
        false,
    )));
    input
}

fn capture_lost(point: Point) -> UiInput {
    let mut input = UiInput::default();
    input.pointer.position = Some(point);
    input.pointer.primary.down = true;
    input.push_event(UiInputEvent::WindowFocusChanged(false));
    input
}

fn assert_seconds(actual: TimelineTime, expected: f64) {
    assert!((actual.seconds() - expected).abs() <= 1.0e-9);
}

#[test]
fn legacy_timeline_output_remains_exactly_constructible_and_destructurable() {
    let descriptor = descriptor(
        TimelineRange::seconds(1.0, 3.0),
        TimelineDescriptorState::default(),
    );
    let state = state(24.0);
    let theme = default_dark_theme();
    let mut memory = UiMemory::new();
    let mut ui = Ui::begin_frame(context(UiInput::default()), &mut memory, &theme);
    let widget = ui
        .prepare_timeline_widget(
            TimelineWidgetConfig::new(
                ROOT,
                BOUNDS,
                TimelineFrameRate::integer(24),
                &descriptor,
                &state,
            )
            .with_lane_header_width(80.0)
            .with_ruler_height(20.0),
        )
        .expect("valid timeline");
    let output: TimelineWidgetOutput = ui.timeline_widget(&widget);
    let TimelineWidgetOutput {
        response,
        hit,
        intent,
        scrub_intents,
    } = output;
    let rebuilt = TimelineWidgetOutput {
        response,
        hit,
        intent,
        scrub_intents,
    };

    assert!(rebuilt.intent.is_none());
}

#[test]
fn clip_move_uses_frozen_press_scale_and_commits_once_with_stable_identity() {
    let committed = descriptor(
        TimelineRange::seconds(1.0, 3.0),
        TimelineDescriptorState::default(),
    );
    let initial_state = state(24.0);
    let mut controller = TimelineClipEditController::default();
    let mut memory = UiMemory::new();

    let (pressed, _) = run(
        &committed,
        &initial_state,
        &mut controller,
        &mut memory,
        pointer(Point::new(128.0, 32.0), true),
        |config| config,
    );
    let frozen = controller.frozen_scale().expect("press-time scale");
    assert!(pressed.is_empty());
    assert_eq!(controller.target(), Some(ITEM));
    assert_eq!(controller.kind(), Some(TimelineClipEditKind::Move));

    let zoomed_state = state(48.0);
    let (begin_intents, _) = run(
        &committed,
        &zoomed_state,
        &mut controller,
        &mut memory,
        pointer_move(Point::new(176.0, 32.0), Vec2::new(48.0, 0.0)),
        |config| config,
    );
    let [TimelineClipEditIntent::Begin(TimelineClipEditRequest::Move(begin_request))] =
        begin_intents.as_slice()
    else {
        panic!("one move begin intent")
    };
    assert_eq!(controller.frozen_scale(), Some(frozen));
    assert_eq!(begin_request.target, ITEM);
    assert_eq!(begin_request.lane, LANE);
    assert_seconds(begin_request.snapped_range.start, 3.0);
    assert_seconds(begin_request.snapped_range.end, 5.0);

    let preview = descriptor(
        begin_request.snapped_range,
        TimelineDescriptorState::default(),
    );
    let (updated, _) = run(
        &preview,
        &zoomed_state,
        &mut controller,
        &mut memory,
        pointer_move(Point::new(200.0, 32.0), Vec2::new(24.0, 0.0)),
        |config| config,
    );
    let [TimelineClipEditIntent::Update(TimelineClipEditRequest::Move(update))] =
        updated.as_slice()
    else {
        panic!("one move update intent")
    };
    assert_eq!(update.original_range, TimelineRange::seconds(1.0, 3.0));
    assert_seconds(update.snapped_range.start, 4.0);
    assert_seconds(update.snapped_range.end, 6.0);

    let (ended, frame) = run(
        &preview,
        &zoomed_state,
        &mut controller,
        &mut memory,
        pointer(Point::new(224.0, 32.0), false),
        |config| config,
    );
    let [TimelineClipEditIntent::End(TimelineClipEditRequest::Move(commit))] = ended.as_slice()
    else {
        panic!("one canonical move commit")
    };
    assert_seconds(commit.snapped_range.start, 5.0);
    assert_seconds(commit.snapped_range.end, 7.0);
    assert!(!commit.pointer_capture_requested);
    assert!(!controller.is_editing());
    assert!(
        frame
            .semantics
            .get(stern_widgets::timeline_item_widget_id(ROOT, ITEM))
            .is_some()
    );
}

#[test]
fn left_trim_rejects_minimum_duration_and_invalid_terminal_ranges() {
    let descriptor = descriptor(
        TimelineRange::seconds(1.0, 3.0),
        TimelineDescriptorState::default(),
    );
    let state = state(24.0);
    let mut controller = TimelineClipEditController::default();
    let mut memory = UiMemory::new();
    let minimum = TimelineTime::from_seconds(0.5);

    let _ = run(
        &descriptor,
        &state,
        &mut controller,
        &mut memory,
        pointer(Point::new(105.0, 32.0), true),
        |config| config.with_minimum_clip_duration(minimum),
    );
    let (begin_intents, _) = run(
        &descriptor,
        &state,
        &mut controller,
        &mut memory,
        pointer_move(Point::new(128.0, 32.0), Vec2::new(23.0, 0.0)),
        |config| config.with_minimum_clip_duration(minimum),
    );
    let [TimelineClipEditIntent::Begin(TimelineClipEditRequest::Trim(begin_request))] =
        begin_intents.as_slice()
    else {
        panic!("one left-trim begin")
    };
    assert_eq!(begin_request.edge, TimelineTrimEdge::Start);
    assert_eq!(
        begin_request.clamped_range,
        TimelineRange::seconds(2.0, 3.0)
    );

    let (too_short, _) = run(
        &descriptor,
        &state,
        &mut controller,
        &mut memory,
        pointer_move(Point::new(146.0, 32.0), Vec2::new(18.0, 0.0)),
        |config| config.with_minimum_clip_duration(minimum),
    );
    let [TimelineClipEditIntent::Reject(rejection)] = too_short.as_slice() else {
        panic!("one minimum-duration rejection")
    };
    assert_eq!(rejection.stage, TimelineClipEditRejectionStage::Update);
    assert_eq!(
        rejection.reason,
        TimelineClipEditRejectionReason::MinimumDuration
    );
    assert_eq!(rejection.preview_range, TimelineRange::seconds(2.0, 3.0));
    assert!(rejection.pointer_capture_requested);

    let (invalid_end, _) = run(
        &descriptor,
        &state,
        &mut controller,
        &mut memory,
        pointer(Point::new(158.0, 32.0), false),
        |config| config.with_minimum_clip_duration(minimum),
    );
    let [TimelineClipEditIntent::Reject(rejection)] = invalid_end.as_slice() else {
        panic!("one invalid terminal rejection")
    };
    assert_eq!(rejection.stage, TimelineClipEditRejectionStage::End);
    assert_eq!(
        rejection.reason,
        TimelineClipEditRejectionReason::InvalidRange
    );
    assert_eq!(rejection.original_range, TimelineRange::seconds(1.0, 3.0));
    assert!(!rejection.pointer_capture_requested);
    assert!(!controller.is_editing());
    assert!(!memory.has_pointer_capture(ROOT));
}

#[test]
fn right_trim_previews_and_commits_the_same_canonical_range() {
    let descriptor = descriptor(
        TimelineRange::seconds(1.0, 3.0),
        TimelineDescriptorState::default(),
    );
    let state = state(24.0);
    let mut controller = TimelineClipEditController::default();
    let mut memory = UiMemory::new();

    let _ = run(
        &descriptor,
        &state,
        &mut controller,
        &mut memory,
        pointer(Point::new(151.0, 32.0), true),
        |config| config,
    );
    let (begin_intents, _) = run(
        &descriptor,
        &state,
        &mut controller,
        &mut memory,
        pointer_move(Point::new(176.0, 32.0), Vec2::new(25.0, 0.0)),
        |config| config,
    );
    let [TimelineClipEditIntent::Begin(TimelineClipEditRequest::Trim(begin_request))] =
        begin_intents.as_slice()
    else {
        panic!("one right-trim begin")
    };
    assert_eq!(begin_request.edge, TimelineTrimEdge::End);
    assert_eq!(
        begin_request.clamped_range,
        TimelineRange::seconds(1.0, 4.0)
    );

    let (ended, _) = run(
        &descriptor,
        &state,
        &mut controller,
        &mut memory,
        release_without_position(),
        |config| config,
    );
    let [TimelineClipEditIntent::End(TimelineClipEditRequest::Trim(commit))] = ended.as_slice()
    else {
        panic!("one right-trim commit")
    };
    assert_eq!(commit.clamped_range, begin_request.clamped_range);
    assert!(!commit.pointer_capture_requested);
}

#[test]
fn escape_and_capture_loss_cancel_to_the_committed_clip_range() {
    let descriptor = descriptor(
        TimelineRange::seconds(1.0, 3.0),
        TimelineDescriptorState::default(),
    );
    let state = state(24.0);

    for cancellation in [
        escape(Point::new(176.0, 32.0)),
        capture_lost(Point::new(176.0, 32.0)),
    ] {
        let mut controller = TimelineClipEditController::default();
        let mut memory = UiMemory::new();
        let _ = run(
            &descriptor,
            &state,
            &mut controller,
            &mut memory,
            pointer(Point::new(128.0, 32.0), true),
            |config| config,
        );
        let _ = run(
            &descriptor,
            &state,
            &mut controller,
            &mut memory,
            pointer_move(Point::new(176.0, 32.0), Vec2::new(48.0, 0.0)),
            |config| config,
        );
        let (cancelled, _) = run(
            &descriptor,
            &state,
            &mut controller,
            &mut memory,
            cancellation,
            |config| config,
        );
        let [TimelineClipEditIntent::Cancel(cancel)] = cancelled.as_slice() else {
            panic!("one clip cancellation")
        };
        assert_eq!(cancel.target, ITEM);
        assert_eq!(cancel.kind, TimelineClipEditKind::Move);
        assert_eq!(cancel.original_range, TimelineRange::seconds(1.0, 3.0));
        assert_eq!(cancel.preview_range, TimelineRange::seconds(3.0, 5.0));
        assert!(!cancel.pointer_capture_requested);
        assert!(!controller.is_editing());
        assert!(!memory.has_pointer_capture(ROOT));
    }
}

#[test]
fn disabled_read_only_and_locked_clips_suppress_edit_lifecycle() {
    let state = state(24.0);
    for (descriptor_state, configure) in [
        (TimelineDescriptorState::default(), Some(true)),
        (TimelineDescriptorState::default(), Some(false)),
        (TimelineDescriptorState::default().read_only(true), None),
        (TimelineDescriptorState::default().disabled(true), None),
    ] {
        let descriptor = descriptor(TimelineRange::seconds(1.0, 3.0), descriptor_state);
        let mut controller = TimelineClipEditController::default();
        let mut memory = UiMemory::new();
        let (pressed, _) = run(
            &descriptor,
            &state,
            &mut controller,
            &mut memory,
            pointer(Point::new(128.0, 32.0), true),
            |config| match configure {
                Some(true) => config.disabled(true),
                Some(false) => config.read_only(true),
                None => config,
            },
        );
        let (moved, _) = run(
            &descriptor,
            &state,
            &mut controller,
            &mut memory,
            pointer_move(Point::new(176.0, 32.0), Vec2::new(48.0, 0.0)),
            |config| match configure {
                Some(true) => config.disabled(true),
                Some(false) => config.read_only(true),
                None => config,
            },
        );
        assert!(pressed.is_empty() && moved.is_empty());
        assert_eq!(controller.target(), None);
    }
}

#[test]
fn captured_clip_becoming_unavailable_cancels_move_and_release() {
    let initial = descriptor(
        TimelineRange::seconds(1.0, 3.0),
        TimelineDescriptorState::default(),
    );
    let state = state(24.0);
    for (unavailable_state, transition_input) in [
        (
            TimelineDescriptorState::default().disabled(true),
            pointer_move(Point::new(200.0, 32.0), Vec2::new(24.0, 0.0)),
        ),
        (
            TimelineDescriptorState::default().read_only(true),
            pointer(Point::new(200.0, 32.0), false),
        ),
    ] {
        let mut controller = TimelineClipEditController::default();
        let mut memory = UiMemory::new();
        let _ = run(
            &initial,
            &state,
            &mut controller,
            &mut memory,
            pointer(Point::new(128.0, 32.0), true),
            |config| config,
        );
        let (began, _) = run(
            &initial,
            &state,
            &mut controller,
            &mut memory,
            pointer_move(Point::new(176.0, 32.0), Vec2::new(48.0, 0.0)),
            |config| config,
        );
        assert!(matches!(
            began.as_slice(),
            [TimelineClipEditIntent::Begin(_)]
        ));

        let unavailable = descriptor(TimelineRange::seconds(1.0, 3.0), unavailable_state);
        let (cancelled, _) = run(
            &unavailable,
            &state,
            &mut controller,
            &mut memory,
            transition_input,
            |config| config,
        );
        let [TimelineClipEditIntent::Cancel(cancel)] = cancelled.as_slice() else {
            panic!("unavailable captured clip must cancel")
        };
        assert_eq!(cancel.target, ITEM);
        assert_eq!(cancel.original_range, TimelineRange::seconds(1.0, 3.0));
        assert_eq!(cancel.preview_range, TimelineRange::seconds(3.0, 5.0));
        assert!(!cancel.pointer_capture_requested);
        assert!(!controller.is_editing());
        assert!(!memory.has_pointer_capture(ROOT));
    }
}
