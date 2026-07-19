//! Combined timeline scrub and clip-edit controller intent isolation.

use std::time::Duration;

use stern_core::{
    FrameContext, Key, KeyEvent, KeyState, Modifiers, MouseButton, PhysicalSize, Point,
    PointerOrder, Rect, ScaleFactor, Size, TimeInfo, UiInput, UiInputEvent, UiMemory, Vec2,
    ViewportInfo, WidgetId, default_dark_theme,
};
use stern_widgets::{
    TimelineClipEditController, TimelineClipEditIntent, TimelineDescriptor, TimelineFrameRate,
    TimelineHitTarget, TimelineItemDescriptor, TimelineItemId, TimelineLaneDescriptor,
    TimelineLaneId, TimelineRange, TimelineScale, TimelineScrubController, TimelineScrubIntent,
    TimelineTime, TimelineViewportState, TimelineWidgetConfig, TimelineZoom, Ui,
};

const ROOT: WidgetId = WidgetId::from_raw(0x835);
const ITEM: TimelineItemId = TimelineItemId::from_raw(10);
const LANE: TimelineLaneId = TimelineLaneId::from_raw(1);
const BOUNDS: Rect = Rect::new(0.0, 0.0, 320.0, 120.0);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IntentFamily {
    Scrub,
    Clip,
}

#[derive(Debug, Clone, Copy)]
struct GestureCase {
    name: &'static str,
    press: Point,
    moved: Point,
    family: IntentFamily,
}

const CASES: [GestureCase; 6] = [
    GestureCase {
        name: "background",
        press: Point::new(200.0, 80.0),
        moved: Point::new(224.0, 80.0),
        family: IntentFamily::Scrub,
    },
    GestureCase {
        name: "ruler",
        press: Point::new(200.0, 10.0),
        moved: Point::new(224.0, 10.0),
        family: IntentFamily::Scrub,
    },
    GestureCase {
        name: "playhead",
        press: Point::new(128.0, 80.0),
        moved: Point::new(176.0, 80.0),
        family: IntentFamily::Scrub,
    },
    GestureCase {
        name: "item body",
        press: Point::new(128.0, 32.0),
        moved: Point::new(176.0, 32.0),
        family: IntentFamily::Clip,
    },
    GestureCase {
        name: "item start trim",
        press: Point::new(105.0, 32.0),
        moved: Point::new(128.0, 32.0),
        family: IntentFamily::Clip,
    },
    GestureCase {
        name: "item end trim",
        press: Point::new(151.0, 32.0),
        moved: Point::new(176.0, 32.0),
        family: IntentFamily::Clip,
    },
];

fn descriptor() -> TimelineDescriptor {
    TimelineDescriptor::new(
        [TimelineLaneDescriptor::new(LANE, "Video")],
        [TimelineItemDescriptor::new(
            ITEM,
            LANE,
            TimelineRange::seconds(1.0, 3.0),
            "Clip",
        )],
        [],
        [],
    )
}

fn state() -> TimelineViewportState {
    TimelineViewportState::new(TimelineScale::new(
        999.0,
        1.0,
        TimelineRange::seconds(0.0, 10.0),
        TimelineZoom::new(24.0),
        0.0,
    ))
    .with_playhead_time(TimelineTime::from_seconds(2.0))
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

fn run(
    descriptor: &TimelineDescriptor,
    state: &TimelineViewportState,
    scrub: &mut TimelineScrubController,
    clip: &mut TimelineClipEditController,
    memory: &mut UiMemory,
    input: UiInput,
) -> stern_widgets::TimelineClipEditWidgetOutput {
    let theme = default_dark_theme();
    let mut ui = Ui::begin_frame(context(input), memory, &theme);
    let widget = ui
        .prepare_timeline_widget(
            TimelineWidgetConfig::new(
                ROOT,
                BOUNDS,
                TimelineFrameRate::integer(24),
                descriptor,
                state,
            )
            .with_lane_header_width(80.0)
            .with_ruler_height(20.0),
        )
        .expect("valid timeline");
    ui.resolve_pointer_targets(|plan| {
        widget.declare_pointer_targets(plan, PointerOrder::new(10));
    })
    .expect("valid pointer plan");
    ui.timeline_widget_with_controllers(&widget, scrub, clip)
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

fn pointer_move(from: Point, to: Point) -> UiInput {
    let mut input = UiInput::default();
    input.pointer.primary.down = true;
    input.push_event(UiInputEvent::PointerMoved {
        position: to,
        delta: Vec2::new(to.x - from.x, to.y - from.y),
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

fn assert_hit_family(case: GestureCase, target: TimelineHitTarget) {
    let matches = match case.name {
        "background" => matches!(target, TimelineHitTarget::Background(_)),
        "ruler" => matches!(target, TimelineHitTarget::Ruler(_)),
        "playhead" => matches!(target, TimelineHitTarget::Playhead(_)),
        "item body" => target == TimelineHitTarget::Item(ITEM),
        "item start trim" => target == TimelineHitTarget::ItemTrimStartHandle(ITEM),
        "item end trim" => target == TimelineHitTarget::ItemTrimEndHandle(ITEM),
        _ => false,
    };
    assert!(matches, "{} resolved to {target:?}", case.name);
}

fn assert_no_cross_family_intents(output: &stern_widgets::TimelineClipEditWidgetOutput) {
    assert!(
        output.timeline.scrub_intents.is_empty() || output.clip_edit_intents.is_empty(),
        "one gesture emitted both scrub and clip-edit intents"
    );
}

fn assert_begin(case: GestureCase, output: &stern_widgets::TimelineClipEditWidgetOutput) {
    assert_no_cross_family_intents(output);
    match case.family {
        IntentFamily::Scrub => {
            assert!(matches!(
                output.timeline.scrub_intents.as_slice(),
                [TimelineScrubIntent::Begin(_)]
            ));
            assert!(output.clip_edit_intents.is_empty());
        }
        IntentFamily::Clip => {
            assert!(output.timeline.scrub_intents.is_empty());
            assert!(matches!(
                output.clip_edit_intents.as_slice(),
                [TimelineClipEditIntent::Begin(_)]
            ));
        }
    }
}

fn assert_end(case: GestureCase, output: &stern_widgets::TimelineClipEditWidgetOutput) {
    assert_no_cross_family_intents(output);
    match case.family {
        IntentFamily::Scrub => {
            assert!(matches!(
                output.timeline.scrub_intents.as_slice(),
                [TimelineScrubIntent::End(_)]
            ));
            assert!(output.clip_edit_intents.is_empty());
        }
        IntentFamily::Clip => {
            assert!(output.timeline.scrub_intents.is_empty());
            assert!(matches!(
                output.clip_edit_intents.as_slice(),
                [TimelineClipEditIntent::End(_)]
            ));
        }
    }
}

fn assert_cancel(case: GestureCase, output: &stern_widgets::TimelineClipEditWidgetOutput) {
    assert_no_cross_family_intents(output);
    match case.family {
        IntentFamily::Scrub => {
            assert!(matches!(
                output.timeline.scrub_intents.as_slice(),
                [TimelineScrubIntent::Cancel(_)]
            ));
            assert!(output.clip_edit_intents.is_empty());
        }
        IntentFamily::Clip => {
            assert!(output.timeline.scrub_intents.is_empty());
            assert!(matches!(
                output.clip_edit_intents.as_slice(),
                [TimelineClipEditIntent::Cancel(_)]
            ));
        }
    }
}

fn assert_capture_owner(
    case: GestureCase,
    scrub: &TimelineScrubController,
    clip: &TimelineClipEditController,
) {
    match case.family {
        IntentFamily::Scrub => {
            assert!(
                scrub.source().is_some(),
                "{} missed scrub capture",
                case.name
            );
            assert_eq!(clip.target(), None, "{} captured clip edit", case.name);
        }
        IntentFamily::Clip => {
            assert_eq!(scrub.source(), None, "{} captured scrub", case.name);
            assert_eq!(
                clip.target(),
                Some(ITEM),
                "{} missed clip capture",
                case.name
            );
        }
    }
}

fn assert_capture_cleared(
    scrub: &TimelineScrubController,
    clip: &TimelineClipEditController,
    memory: &UiMemory,
) {
    assert_eq!(scrub.source(), None);
    assert!(!scrub.is_scrubbing());
    assert_eq!(clip.target(), None);
    assert!(!clip.is_editing());
    assert!(!memory.has_pointer_capture(ROOT));
}

#[test]
fn gestures_commit_exactly_one_intent_family_and_clear_retained_capture() {
    let descriptor = descriptor();
    let state = state();

    for case in CASES {
        let mut scrub = TimelineScrubController::default();
        let mut clip = TimelineClipEditController::default();
        let mut memory = UiMemory::new();

        let pressed = run(
            &descriptor,
            &state,
            &mut scrub,
            &mut clip,
            &mut memory,
            pointer(case.press, true),
        );
        assert_hit_family(case, pressed.timeline.hit.expect("press hit").target);
        assert!(pressed.timeline.scrub_intents.is_empty());
        assert!(pressed.clip_edit_intents.is_empty());
        assert_capture_owner(case, &scrub, &clip);

        let began = run(
            &descriptor,
            &state,
            &mut scrub,
            &mut clip,
            &mut memory,
            pointer_move(case.press, case.moved),
        );
        assert_begin(case, &began);
        assert_capture_owner(case, &scrub, &clip);

        let ended = run(
            &descriptor,
            &state,
            &mut scrub,
            &mut clip,
            &mut memory,
            pointer(case.moved, false),
        );
        assert_end(case, &ended);
        assert_capture_cleared(&scrub, &clip, &memory);
    }
}

#[test]
fn gestures_cancel_exactly_one_intent_family_and_clear_retained_capture() {
    let descriptor = descriptor();
    let state = state();

    for case in CASES {
        let mut scrub = TimelineScrubController::default();
        let mut clip = TimelineClipEditController::default();
        let mut memory = UiMemory::new();

        let _ = run(
            &descriptor,
            &state,
            &mut scrub,
            &mut clip,
            &mut memory,
            pointer(case.press, true),
        );
        let began = run(
            &descriptor,
            &state,
            &mut scrub,
            &mut clip,
            &mut memory,
            pointer_move(case.press, case.moved),
        );
        assert_begin(case, &began);

        let cancelled = run(
            &descriptor,
            &state,
            &mut scrub,
            &mut clip,
            &mut memory,
            escape(case.moved),
        );
        assert_cancel(case, &cancelled);
        assert_capture_cleared(&scrub, &clip, &memory);
    }
}
