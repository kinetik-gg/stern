//! Retained timeline widget conformance.
#[rustfmt::skip]
mod conformance {
use std::time::Duration;
use stern_core::{Brush, FrameContext, Modifiers, MouseButton, PhysicalSize, Point, PointerOrder, Rect, ScaleFactor, Size, TimeInfo, UiInput, UiInputEvent, UiMemory, ViewportInfo, WidgetId, default_dark_theme};
use stern_widgets::{TimelineDescriptor, TimelineDescriptorError, TimelineDescriptorState, TimelineFrame, TimelineFrameRate, TimelineHitTarget, TimelineItemDescriptor, TimelineItemId, TimelineKeyframeDescriptor, TimelineKeyframeId, TimelineLaneDescriptor, TimelineLaneId, TimelineLayout, TimelineMarkerDescriptor, TimelineMarkerId, TimelineRange, TimelineScale, TimelineSelection, TimelineSelectionOperation, TimelineSelectionTarget, TimelineTime, TimelineViewportState, TimelineWidget, TimelineWidgetConfig, TimelineWidgetIntent, TimelineWidgetOutput, TimelineZoom, Ui, timeline_item_widget_id};
const ROOT: WidgetId = WidgetId::from_raw(0x71);
const BOUNDS: Rect = Rect::new(0.0, 0.0, 320.0, 120.0);
fn descriptor() -> TimelineDescriptor {
    TimelineDescriptor::new(
        [TimelineLaneDescriptor::new(TimelineLaneId::from_raw(1), "Video"), TimelineLaneDescriptor::new(TimelineLaneId::from_raw(2), "Audio")],
        [TimelineItemDescriptor::new(TimelineItemId::from_raw(10), TimelineLaneId::from_raw(1), TimelineRange::seconds(1.0, 3.0), "Clip"), TimelineItemDescriptor::new(TimelineItemId::from_raw(11), TimelineLaneId::from_raw(2), TimelineRange::seconds(4.0, 6.0), "Voice")],
        [TimelineMarkerDescriptor::new(TimelineMarkerId::from_raw(20), TimelineTime::from_seconds(3.0), "Beat")],
        [TimelineKeyframeDescriptor::new(TimelineKeyframeId::from_raw(30), TimelineItemId::from_raw(10), TimelineTime::from_seconds(2.0))],
    )
}
fn state() -> TimelineViewportState {
    TimelineViewportState::new(TimelineScale::new(999.0, 1.0, TimelineRange::seconds(0.0, 10.0), TimelineZoom::new(24.0), 0.0))
        .with_playhead_time(TimelineTime::from_seconds(2.0))
        .with_selection(TimelineSelection::from_targets([TimelineSelectionTarget::Item(TimelineItemId::from_raw(10))]))
}
fn context(input: UiInput) -> FrameContext {
    FrameContext::new(ViewportInfo::new(Size::new(400.0, 200.0), PhysicalSize::new(400, 200), ScaleFactor::ONE), input, TimeInfo::new(Duration::ZERO, Duration::ZERO, 0))
}
fn run<'a>(config: TimelineWidgetConfig<'a>, memory: &mut UiMemory, input: UiInput) -> (TimelineWidget<'a>, TimelineWidgetOutput, stern_core::FrameOutput) {
    let theme = default_dark_theme();
    let mut ui = Ui::begin_frame(context(input), memory, &theme);
    let widget = ui.prepare_timeline_widget(config).expect("valid timeline");
    ui.resolve_pointer_targets(|plan| { widget.declare_pointer_targets(plan, PointerOrder::new(10)); }).expect("valid plan");
    let output = ui.timeline_widget(&widget);
    let frame = ui.finish_output();
    (widget, output, frame)
}
fn pointer(point: Point, down: bool, modifiers: Modifiers) -> UiInput {
    let mut input = UiInput::default();
    input.keyboard.modifiers = modifiers;
    input.push_event(UiInputEvent::PointerButton { button: MouseButton::Primary, down, click_count: 1, position: Some(point) });
    input
}
fn click(config: TimelineWidgetConfig<'_>, memory: &mut UiMemory, point: Point, modifiers: Modifiers) -> TimelineWidgetOutput {
    let _ = run(config, memory, pointer(point, true, modifiers));
    run(config, memory, pointer(point, false, modifiers)).1
}

#[test]
fn canonical_timeline_scene_uses_one_frozen_transform_and_theme_paint() {
    let descriptor = descriptor();
    let state = state();
    let theme = default_dark_theme();
    let mut memory = UiMemory::new();
    let (widget, output, frame) = run(TimelineWidgetConfig::new(ROOT, BOUNDS, TimelineFrameRate::integer(24), &descriptor, &state).with_lane_header_width(80.0).with_ruler_height(20.0), &mut memory, UiInput::default());
    assert_eq!(widget.scale().origin_x.to_bits(), 80.0_f32.to_bits());
    assert_eq!(widget.scale().viewport_width.to_bits(), 240.0_f32.to_bits());
    assert_eq!(widget.layout().bounds, Rect::new(80.0, 20.0, 240.0, 100.0));
    let item = widget.layout().items.iter().find(|item| item.descriptor.id == TimelineItemId::from_raw(10)).expect("selected item");
    assert_eq!(item.rect.x.to_bits(), widget.scale().time_to_screen_x(item.time_range.start).to_bits());
    assert!(frame.primitives.iter().any(|primitive| matches!(primitive, stern_core::Primitive::Rect(rect) if rect.rect == item.rect && rect.fill == Some(Brush::Solid(theme.colors.selection.background)))));
    assert!(frame.primitives.iter().any(|primitive| matches!(primitive, stern_core::Primitive::Text(text) if text.text == "Video")));
    assert!(frame.semantics.get(timeline_item_widget_id(ROOT, TimelineItemId::from_raw(10))).expect("item semantics").state.selected);
    assert!(output.intent.is_none());
}

#[test]
fn pointer_activation_emits_typed_seek_and_selection_intents() {
    let descriptor = descriptor();
    let state = state();
    let config = TimelineWidgetConfig::new(ROOT, BOUNDS, TimelineFrameRate::integer(24), &descriptor, &state).with_lane_header_width(80.0).with_ruler_height(20.0);
    let mut memory = UiMemory::new();
    let selected = click(config, &mut memory, Point::new(140.0, 32.0), Modifiers::new(false, true, false, false));
    assert_eq!(selected.intent, Some(TimelineWidgetIntent::Select { target: TimelineSelectionTarget::Item(TimelineItemId::from_raw(10)), operation: TimelineSelectionOperation::Toggle }));
    let sought = click(config, &mut memory, Point::new(200.0, 10.0), Modifiers::default());
    let Some(TimelineWidgetIntent::Seek(request)) = sought.intent else { panic!("seek intent") };
    assert_eq!(request.frame, TimelineFrame::from_raw(120));
    assert_eq!(request.requested_time, TimelineTime::from_seconds(5.0));
}

#[test]
fn stable_ids_semantics_and_selection_survive_descriptor_reorder() {
    let first = descriptor();
    let mut reordered = descriptor();
    reordered.lanes.reverse();
    reordered.items.reverse();
    let state = state();
    let mut first_memory = UiMemory::new();
    let (_, _, first_frame) = run(TimelineWidgetConfig::new(ROOT, BOUNDS, TimelineFrameRate::integer(24), &first, &state), &mut first_memory, UiInput::default());
    let focused = timeline_item_widget_id(ROOT, TimelineItemId::from_raw(10));
    let mut second_memory = UiMemory::new();
    second_memory.focus(focused);
    let mut playback = state.clone();
    playback.playhead_time = Some(TimelineTime::from_seconds(7.0));
    let (_, _, second_frame) = run(TimelineWidgetConfig::new(ROOT, BOUNDS, TimelineFrameRate::integer(24), &reordered, &playback), &mut second_memory, UiInput::default());
    let mut first_ids = first_frame.semantics.traversal_order(); first_ids.sort_unstable();
    let mut second_ids = second_frame.semantics.traversal_order(); second_ids.sort_unstable();
    assert_eq!(first_ids, second_ids);
    assert!(second_frame.semantics.get(focused).expect("stable item").state.selected);
    assert!(second_memory.is_focused(focused));
    assert_eq!(playback.selection, state.selection);
}

#[test]
fn lane_headers_and_content_share_virtualized_vertical_window() {
    let lanes = (0..6).map(|id| TimelineLaneDescriptor::new(TimelineLaneId::from_raw(id), format!("Lane {id}"))).collect::<Vec<_>>();
    let descriptor = TimelineDescriptor::new(lanes, [], [], []);
    let mut state = state();
    state.lane_scroll_offset = 24.0;
    let bounds = Rect::new(0.0, 0.0, 260.0, 68.0);
    let mut memory = UiMemory::new();
    let (widget, _, frame) = run(TimelineWidgetConfig::new(ROOT, bounds, TimelineFrameRate::integer(24), &descriptor, &state).with_layout(TimelineLayout::new(24.0)).with_lane_header_width(80.0).with_ruler_height(20.0), &mut memory, UiInput::default());
    assert_eq!(widget.layout().visible_lane_range, 1..3);
    assert_eq!(widget.layout().materialized_lane_ids(), vec![TimelineLaneId::from_raw(1), TimelineLaneId::from_raw(2), TimelineLaneId::from_raw(3)]);
    let labels = frame.primitives.iter().filter_map(|primitive| match primitive { stern_core::Primitive::Text(text) if text.text.starts_with("Lane ") => Some(text.text.as_str()), _ => None }).collect::<Vec<_>>();
    assert_eq!(labels, vec!["Lane 1", "Lane 2", "Lane 3"]);
    assert!(widget.layout().lanes.iter().all(|lane| lane.rect.x.to_bits() == 80.0_f32.to_bits()));
}

#[test]
fn disabled_invalid_and_overlapping_targets_are_inert_and_deterministic() {
    let lane = TimelineLaneDescriptor::new(TimelineLaneId::from_raw(1), "Lane");
    let inert = TimelineDescriptorState::default().disabled(true).read_only(true);
    let item = |id, state| TimelineItemDescriptor::new(TimelineItemId::from_raw(id), TimelineLaneId::from_raw(1), TimelineRange::seconds(1.0, 3.0), "Overlap").with_state(state);
    let descriptor = TimelineDescriptor::new([lane.clone()], [item(11, inert), item(10, TimelineDescriptorState::default())], [], []);
    let state = state();
    let config = TimelineWidgetConfig::new(ROOT, BOUNDS, TimelineFrameRate::integer(24), &descriptor, &state).with_lane_header_width(80.0).with_ruler_height(20.0);
    let mut memory = UiMemory::new();
    let output = click(config, &mut memory, Point::new(140.0, 32.0), Modifiers::default());
    assert!(matches!(output.hit, Some(hit) if hit.target == TimelineHitTarget::Item(TimelineItemId::from_raw(11))));
    assert!(output.intent.is_none());
    let disabled = click(config.disabled(true), &mut UiMemory::new(), Point::new(140.0, 32.0), Modifiers::default());
    assert!(disabled.response.state.disabled && disabled.intent.is_none());
    let invalid = TimelineDescriptor::new([lane.clone(), lane], [], [], []);
    assert!(matches!(TimelineWidget::new(TimelineWidgetConfig::new(ROOT, BOUNDS, TimelineFrameRate::integer(24), &invalid, &state)), Err(TimelineDescriptorError::DuplicateLaneId { .. })));
}
}
