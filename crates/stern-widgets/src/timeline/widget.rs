//! Retained timeline composition over the public timeline contracts.
use super::{
    TimelineDescriptor, TimelineDescriptorError, TimelineDescriptorState, TimelineFrameRate,
    TimelineHitMetadata, TimelineHitTarget, TimelineHitTestConfig, TimelineId, TimelineLayout,
    TimelineLayoutResult, TimelinePlayheadSeekRequest, TimelineRulerId, TimelineRulerTickKind,
    TimelineRulerTickRequest, TimelineScale, TimelineScrubBeginRequest, TimelineScrubEndRequest,
    TimelineScrubUpdateRequest, TimelineSelectionOperation, TimelineSelectionTarget,
    TimelineSnapMetadata, TimelineTime, TimelineViewportState, clamp_timeline_time,
    timeline_item_widget_id, timeline_keyframe_widget_id, timeline_lane_widget_id,
    timeline_marker_widget_id, timeline_semantics,
};
use crate::{Ui, label, panel, separator};
use stern_core::{
    Brush, Color, CornerRadius, DomainDragGestureAction, DomainDragGesturePhase, Key, KeyState,
    Modifiers, Point, PointerOrder, PointerTarget, PointerTargetPlan, Primitive, Rect,
    RectPrimitive, Response, UiInputEvent, WidgetId,
};
/// Caller-owned inputs for one immutable timeline frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimelineWidgetConfig<'a> {
    id: WidgetId,
    bounds: Rect,
    frame_rate: TimelineFrameRate,
    descriptor: &'a TimelineDescriptor,
    state: &'a TimelineViewportState,
    label: &'a str,
    layout: TimelineLayout,
    ruler_height: f32,
    lane_header_width: f32,
    disabled: bool,
    read_only: bool,
}
impl<'a> TimelineWidgetConfig<'a> {
    /// Creates an enabled timeline with compact default geometry.
    #[must_use]
    pub fn new(
        id: WidgetId,
        bounds: Rect,
        frame_rate: TimelineFrameRate,
        descriptor: &'a TimelineDescriptor,
        state: &'a TimelineViewportState,
    ) -> Self {
        Self {
            id,
            bounds,
            frame_rate,
            descriptor,
            state,
            label: "Timeline",
            layout: TimelineLayout::new(24.0).with_overscan(1),
            ruler_height: 24.0,
            lane_header_width: 120.0,
            disabled: false,
            read_only: false,
        }
    }
    /// Sets the accessible label.
    #[must_use]
    pub const fn with_label(mut self, value: &'a str) -> Self {
        self.label = value;
        self
    }
    /// Replaces lane virtualization and hit geometry.
    #[must_use]
    pub const fn with_layout(mut self, value: TimelineLayout) -> Self {
        self.layout = value;
        self
    }
    /// Sets the top ruler height.
    #[must_use]
    pub const fn with_ruler_height(mut self, value: f32) -> Self {
        self.ruler_height = value;
        self
    }
    /// Sets the left lane-header width.
    #[must_use]
    pub const fn with_lane_header_width(mut self, value: f32) -> Self {
        self.lane_header_width = value;
        self
    }
    /// Sets whether all interaction is disabled.
    #[must_use]
    pub const fn disabled(mut self, value: bool) -> Self {
        self.disabled = value;
        self
    }
    /// Sets whether mutation intents are suppressed while preserving presentation.
    #[must_use]
    pub const fn read_only(mut self, value: bool) -> Self {
        self.read_only = value;
        self
    }
}
/// Immutable frame-local timeline composition.
#[derive(Debug, Clone, PartialEq)]
pub struct TimelineWidget<'a> {
    config: TimelineWidgetConfig<'a>,
    bounds: Rect,
    ruler: Rect,
    header: Rect,
    scale: TimelineScale,
    layout: TimelineLayoutResult<'a>,
}
impl<'a> TimelineWidget<'a> {
    /// Freezes one validated layout, transform, and ruler snapshot.
    ///
    /// # Errors
    /// Returns descriptor validation errors from the canonical layout resolver.
    pub fn new(config: TimelineWidgetConfig<'a>) -> Result<Self, TimelineDescriptorError> {
        let bounds = finite_rect(config.bounds);
        let ruler_height = finite(config.ruler_height).min(bounds.height);
        let header_width = finite(config.lane_header_width).min(bounds.width);
        let track = Rect::new(
            bounds.x + header_width,
            bounds.y + ruler_height,
            bounds.width - header_width,
            bounds.height - ruler_height,
        );
        let ruler = Rect::new(track.x, bounds.y, track.width, ruler_height);
        let header = Rect::new(bounds.x, track.y, header_width, track.height);
        let mut scale = config.state.scale.sanitized();
        scale.origin_x = track.x;
        scale.viewport_width = track.width;
        scale = scale.sanitized();
        let layout = config.layout.resolve(
            track,
            scale,
            config.descriptor,
            config.state.lane_scroll_offset,
        )?;
        Ok(Self {
            config,
            bounds,
            ruler,
            header,
            scale,
            layout,
        })
    }
    /// Returns the frozen configuration.
    #[must_use]
    pub const fn config(&self) -> &TimelineWidgetConfig<'a> {
        &self.config
    }
    /// Returns the stable widget identity.
    #[must_use]
    pub const fn widget_id(&self) -> WidgetId {
        self.config.id
    }
    /// Returns the shared frozen transform.
    #[must_use]
    pub const fn scale(&self) -> TimelineScale {
        self.scale
    }
    /// Returns the shared virtualized layout.
    #[must_use]
    pub const fn layout(&self) -> &TimelineLayoutResult<'a> {
        &self.layout
    }
    /// Declares one blocker and routed activation surface.
    pub fn declare_pointer_targets(
        &self,
        plan: &mut PointerTargetPlan,
        order: PointerOrder,
    ) -> PointerOrder {
        if !self.valid() {
            return order;
        }
        plan.blocker(self.bounds, order);
        let target = PointerOrder::new(order.raw().saturating_add(1));
        plan.target(
            PointerTarget::new(self.widget_id(), self.bounds, target)
                .enabled(!self.config.disabled),
        );
        PointerOrder::new(target.raw().saturating_add(1))
    }
    fn valid(&self) -> bool {
        self.bounds.width > 0.0
            && self.bounds.height > 0.0
            && self.layout.bounds.width > 0.0
            && self.layout.bounds.height > 0.0
    }
    fn ruler_id(&self) -> TimelineRulerId {
        TimelineRulerId::from_raw(self.widget_id().child("ruler").raw())
    }
    fn hit(&self, point: Point) -> Option<TimelineHitMetadata> {
        if !self.bounds.contains_point(point) {
            return None;
        }
        let time = self.scale.screen_x_to_time(point.x);
        if self.header.contains_point(point) {
            let lane = self
                .layout
                .lanes
                .iter()
                .find(|lane| point.y >= lane.rect.y && point.y < lane.rect.max_y())?;
            return Some(TimelineHitMetadata {
                target: TimelineHitTarget::LaneHeader(lane.descriptor.id),
                rect: Rect::new(
                    self.header.x,
                    lane.rect.y,
                    self.header.width,
                    lane.rect.height,
                ),
                time,
                state: lane.descriptor.state,
            });
        } else if self.ruler.contains_point(point) {
            return Some(TimelineHitMetadata {
                target: TimelineHitTarget::Ruler(self.ruler_id()),
                rect: self.ruler,
                time,
                state: TimelineDescriptorState::default(),
            });
        }
        let mut config = TimelineHitTestConfig::new(
            TimelineId::from_raw(self.widget_id().raw()),
            self.ruler_id(),
            self.scale,
        );
        if let Some(time) = self.config.state.playhead_time {
            config = config.with_playhead_time(time);
        }
        let selection_range = self.config.state.selection_range;
        if let Some(range) = selection_range {
            config = config.with_selection_range(range);
        }
        self.layout.hit_test(point, config)
    }
}
/// Application-owned intent emitted by timeline activation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TimelineWidgetIntent {
    /// Selects one stable target.
    Select {
        /// Stable selection target.
        target: TimelineSelectionTarget,
        /// Modifier-derived operation.
        operation: TimelineSelectionOperation,
    },
    /// Seeks the application-owned playhead.
    Seek(TimelinePlayheadSeekRequest),
}
/// Ordered application-owned timeline scrub lifecycle intent.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TimelineScrubIntent {
    /// Begins a preview transaction from the committed playhead time.
    Begin(TimelineScrubBeginRequest),
    /// Updates the current preview transaction.
    Update(TimelineScrubUpdateRequest),
    /// Commits the current preview transaction.
    End(TimelineScrubEndRequest),
    /// Cancels the transaction and requests restoration of its starting time.
    Cancel(TimelineScrubEndRequest),
}
#[derive(Debug, Clone, Copy, PartialEq)]
struct TimelineScrubCapture {
    owner: WidgetId,
    source: TimelineHitTarget,
    scale: TimelineScale,
    start_time: TimelineTime,
    previous_time: TimelineTime,
    started: bool,
}
/// Caller-owned state retained across frames of one timeline scrub gesture.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TimelineScrubController {
    capture: Option<TimelineScrubCapture>,
}
impl TimelineScrubController {
    /// Returns true after a scrub has crossed the drag threshold.
    #[must_use]
    pub fn is_scrubbing(&self) -> bool {
        self.capture.is_some_and(|capture| capture.started)
    }
    /// Returns the stable source captured for the current gesture.
    #[must_use]
    pub fn source(&self) -> Option<TimelineHitTarget> {
        self.capture.map(|capture| capture.source)
    }
    /// Returns the transform frozen at pointer press.
    #[must_use]
    pub fn frozen_scale(&self) -> Option<TimelineScale> {
        self.capture.map(|capture| capture.scale)
    }
}
/// Output from one timeline evaluation.
#[derive(Debug, Clone, PartialEq)]
pub struct TimelineWidgetOutput {
    /// Common surface response.
    pub response: Response,
    /// Deterministic target below the pointer.
    pub hit: Option<TimelineHitMetadata>,
    /// Typed app-owned intent emitted by activation.
    pub intent: Option<TimelineWidgetIntent>,
    /// Ordered scrub lifecycle intents emitted from canonical input events.
    pub scrub_intents: Vec<TimelineScrubIntent>,
}
impl Ui<'_> {
    /// Prepares one immutable timeline frame.
    ///
    /// # Errors
    /// Returns descriptor validation errors from the canonical layout resolver.
    pub fn prepare_timeline_widget<'a>(
        &self,
        config: TimelineWidgetConfig<'a>,
    ) -> Result<TimelineWidget<'a>, TimelineDescriptorError> {
        TimelineWidget::new(config)
    }
    /// Evaluates, paints, and exposes semantics for one prepared timeline.
    pub fn timeline_widget(&mut self, widget: &TimelineWidget<'_>) -> TimelineWidgetOutput {
        self.timeline_widget_with_scrub(widget, &mut TimelineScrubController::default())
    }
    /// Evaluates a prepared timeline with caller-owned scrub lifecycle state.
    pub fn timeline_widget_with_scrub(
        &mut self,
        widget: &TimelineWidget<'_>,
        controller: &mut TimelineScrubController,
    ) -> TimelineWidgetOutput {
        let disabled = widget.config.disabled || !widget.valid();
        self.register_id(widget.widget_id());
        let gesture =
            self.captured_domain_drag_gesture_with_id(widget.widget_id(), widget.bounds, disabled);
        let response = gesture.response;
        let hit = self
            .input()
            .pointer
            .position
            .and_then(|point| widget.hit(point));
        let intent = response
            .clicked
            .then_some(hit)
            .flatten()
            .filter(|hit| !widget.config.read_only && !hit.disabled() && !hit.read_only())
            .and_then(|hit| intent(hit, widget, self.input().keyboard.modifiers));
        let scrub_intents = self.resolve_timeline_scrub(widget, controller, &gesture.actions);
        paint(self, widget);
        let selected = widget.config.state.selection.targets();
        for mut node in timeline_semantics(
            widget.widget_id(),
            widget.bounds,
            &widget.layout,
            widget.config.label,
        ) {
            node.state.selected = selected
                .iter()
                .any(|target| semantic_id(widget.widget_id(), *target) == node.id);
            if disabled {
                node.state.disabled = true;
                node.focusable = false;
                node.actions.clear();
            } else if widget.config.read_only {
                node.description = Some("Read-only".to_owned());
            }
            self.push_semantic_node(node);
        }
        TimelineWidgetOutput {
            response,
            hit,
            intent,
            scrub_intents,
        }
    }

    fn resolve_timeline_scrub(
        &mut self,
        widget: &TimelineWidget<'_>,
        controller: &mut TimelineScrubController,
        actions: &[DomainDragGestureAction],
    ) -> Vec<TimelineScrubIntent> {
        if widget.config.disabled || widget.config.read_only {
            let mut intents = Vec::new();
            if controller.capture.is_some() {
                push_scrub_cancel(controller, &mut intents);
                self.cancel_pointer_interaction();
            }
            return intents;
        }
        if controller
            .capture
            .is_some_and(|capture| capture.owner != widget.widget_id())
        {
            controller.capture = None;
        }
        let escape_ordinal = self.input().events.iter().position(|event| {
            matches!(event, UiInputEvent::Key(event) if event.state == KeyState::Pressed && !event.repeat && matches!(event.key, Key::Escape))
        });
        let drag_crossed_threshold = self.memory().is_drag_source(widget.widget_id())
            || self.memory().released_drag_source() == Some(widget.widget_id());
        let start_move = (!controller.is_scrubbing() && drag_crossed_threshold)
            .then(|| {
                actions
                    .iter()
                    .rposition(|action| matches!(action.phase, DomainDragGesturePhase::Move))
            })
            .flatten();
        let mut intents = Vec::new();
        for (index, action) in actions.iter().enumerate() {
            if escape_ordinal
                .is_some_and(|escape| action.ordinal.is_some_and(|ordinal| ordinal >= escape))
            {
                break;
            }
            Self::apply_timeline_scrub_action(
                widget,
                controller,
                action,
                drag_crossed_threshold,
                start_move == Some(index),
                &mut intents,
            );
        }
        if escape_ordinal.is_some() && controller.capture.is_some() {
            push_scrub_cancel(controller, &mut intents);
            self.cancel_pointer_interaction();
        }
        intents
    }

    fn apply_timeline_scrub_action(
        widget: &TimelineWidget<'_>,
        controller: &mut TimelineScrubController,
        action: &DomainDragGestureAction,
        drag_crossed_threshold: bool,
        starts_scrub: bool,
        intents: &mut Vec<TimelineScrubIntent>,
    ) {
        match action.phase {
            DomainDragGesturePhase::Press => {
                let Some(point) = action.position else {
                    return;
                };
                let Some(hit) = widget.hit(point).filter(scrub_source_is_mutable) else {
                    return;
                };
                if widget.config.disabled || widget.config.read_only || !scrub_source(hit.target) {
                    return;
                }
                let start_time = widget.config.state.playhead_time.unwrap_or(hit.time);
                controller.capture = Some(TimelineScrubCapture {
                    owner: widget.widget_id(),
                    source: hit.target,
                    scale: widget.scale,
                    start_time,
                    previous_time: start_time,
                    started: false,
                });
            }
            DomainDragGesturePhase::Move => {
                if !controller.is_scrubbing() && !starts_scrub {
                    return;
                }
                let Some(capture) = controller.capture.as_mut() else {
                    return;
                };
                let current_time =
                    scrub_time(capture.scale, action.position, capture.previous_time);
                let snap = TimelineSnapMetadata::unsnapped(current_time);
                if capture.started {
                    intents.push(TimelineScrubIntent::Update(
                        TimelineScrubUpdateRequest::new(
                            capture.source,
                            capture.previous_time,
                            current_time,
                            snap,
                        ),
                    ));
                } else {
                    capture.started = true;
                    intents.push(TimelineScrubIntent::Begin(TimelineScrubBeginRequest::new(
                        capture.source,
                        capture.start_time,
                        current_time,
                        snap,
                    )));
                }
                capture.previous_time = current_time;
            }
            DomainDragGesturePhase::Release => {
                let Some(mut capture) = controller.capture.take() else {
                    return;
                };
                let current_time =
                    scrub_time(capture.scale, action.position, capture.previous_time);
                if !capture.started && drag_crossed_threshold {
                    capture.started = true;
                    intents.push(TimelineScrubIntent::Begin(TimelineScrubBeginRequest::new(
                        capture.source,
                        capture.start_time,
                        current_time,
                        TimelineSnapMetadata::unsnapped(current_time),
                    )));
                }
                if capture.started {
                    intents.push(TimelineScrubIntent::End(TimelineScrubEndRequest::new(
                        capture.source,
                        capture.start_time,
                        capture.previous_time,
                        current_time,
                        TimelineSnapMetadata::unsnapped(current_time),
                    )));
                }
            }
            DomainDragGesturePhase::Cancel => push_scrub_cancel(controller, intents),
        }
    }
}
fn scrub_source(target: TimelineHitTarget) -> bool {
    matches!(
        target,
        TimelineHitTarget::Background(_)
            | TimelineHitTarget::Ruler(_)
            | TimelineHitTarget::Playhead(_)
    )
}
fn scrub_source_is_mutable(hit: &TimelineHitMetadata) -> bool {
    !hit.disabled() && !hit.read_only()
}
fn scrub_time(
    scale: TimelineScale,
    position: Option<Point>,
    fallback: TimelineTime,
) -> TimelineTime {
    position.map_or(fallback, |point| {
        clamp_timeline_time(scale.screen_x_to_time(point.x), scale.content_range)
    })
}
fn push_scrub_cancel(
    controller: &mut TimelineScrubController,
    intents: &mut Vec<TimelineScrubIntent>,
) {
    let Some(capture) = controller.capture.take() else {
        return;
    };
    if capture.started {
        intents.push(TimelineScrubIntent::Cancel(TimelineScrubEndRequest::new(
            capture.source,
            capture.start_time,
            capture.previous_time,
            capture.start_time,
            TimelineSnapMetadata::unsnapped(capture.start_time),
        )));
    }
}
fn intent(
    hit: TimelineHitMetadata,
    widget: &TimelineWidget<'_>,
    modifiers: Modifiers,
) -> Option<TimelineWidgetIntent> {
    let operation = if modifiers.shift {
        TimelineSelectionOperation::Extend
    } else if modifiers.ctrl || modifiers.super_key {
        TimelineSelectionOperation::Toggle
    } else {
        TimelineSelectionOperation::Replace
    };
    let target = match hit.target {
        TimelineHitTarget::LaneHeader(id) => TimelineSelectionTarget::Lane(id),
        TimelineHitTarget::Item(id) => TimelineSelectionTarget::Item(id),
        TimelineHitTarget::Marker(id) => TimelineSelectionTarget::Marker(id),
        TimelineHitTarget::Keyframe(id) => TimelineSelectionTarget::Keyframe(id),
        TimelineHitTarget::Background(_)
        | TimelineHitTarget::Ruler(_)
        | TimelineHitTarget::Playhead(_) => {
            return Some(TimelineWidgetIntent::Seek(
                TimelinePlayheadSeekRequest::new(
                    hit.time,
                    widget.config.frame_rate,
                    TimelineSnapMetadata::unsnapped(hit.time),
                ),
            ));
        }
        _ => return None,
    };
    Some(TimelineWidgetIntent::Select { target, operation })
}
fn paint(ui: &mut Ui<'_>, widget: &TimelineWidget<'_>) {
    let theme = *ui.theme();
    let selection = &widget.config.state.selection;
    for rect in [widget.bounds, widget.ruler, widget.header] {
        ui.extend(panel(rect, &theme).primitives);
    }
    for lane in &widget.layout.lanes {
        let selected = selection.contains(TimelineSelectionTarget::Lane(lane.descriptor.id));
        let header = Rect::new(
            widget.header.x,
            lane.rect.y,
            widget.header.width,
            lane.rect.height,
        );
        if selected {
            ui.primitive(fill(header, theme.colors.selection.background));
        }
        ui.primitive(separator(
            Rect::new(
                widget.bounds.x,
                lane.rect.max_y(),
                widget.bounds.width,
                theme.strokes.hairline,
            ),
            &theme,
        ));
        ui.extend(
            label(
                Rect::new(header.x + 6.0, header.y, header.width - 6.0, header.height),
                &lane.descriptor.label,
                &theme,
            )
            .primitives,
        );
    }
    for item in &widget.layout.items {
        let color = if item.descriptor.state.disabled {
            theme.colors.surface.control_disabled
        } else if selection.contains(TimelineSelectionTarget::Item(item.descriptor.id)) {
            theme.colors.selection.background
        } else {
            theme.colors.surface.control
        };
        ui.primitive(fill(item.rect, color));
    }
    for marker in &widget.layout.markers {
        let width = if selection.contains(TimelineSelectionTarget::Marker(marker.descriptor.id)) {
            theme.strokes.emphasis
        } else {
            theme.strokes.hairline
        };
        ui.primitive(fill(
            Rect::new(
                marker.x,
                widget.layout.bounds.y,
                width,
                widget.layout.bounds.height,
            ),
            theme.colors.accent.default,
        ));
    }
    for keyframe in &widget.layout.keyframes {
        let color = if selection.contains(TimelineSelectionTarget::Keyframe(keyframe.descriptor.id))
        {
            theme.colors.selection.background
        } else {
            theme.colors.accent.default
        };
        ui.primitive(fill(keyframe.hit_rect, color));
    }
    if let Some(time) = widget.config.state.playhead_time {
        ui.primitive(fill(
            Rect::new(
                widget.scale.time_to_screen_x(time),
                widget.ruler.y,
                theme.strokes.emphasis,
                widget.bounds.height,
            ),
            theme.colors.accent.pressed,
        ));
    }
    paint_ruler(ui, widget);
}
fn paint_ruler(ui: &mut Ui<'_>, widget: &TimelineWidget<'_>) {
    let theme = *ui.theme();
    for tick in TimelineRulerTickRequest::new(
        widget.scale.visible_range(),
        widget.config.frame_rate,
        widget.scale.zoom,
    )
    .ticks()
    {
        let x = widget
            .scale
            .time_to_screen_x(tick.time(widget.config.frame_rate));
        if x < widget.ruler.x || x >= widget.ruler.max_x() {
            continue;
        }
        ui.primitive(fill(
            Rect::new(x, widget.ruler.max_y() - 6.0, 1.0, 6.0),
            theme.colors.content.muted,
        ));
        if tick.kind == TimelineRulerTickKind::Major {
            ui.extend(
                label(
                    Rect::new(x + 3.0, widget.ruler.y, 78.0, widget.ruler.height),
                    tick.label,
                    &theme,
                )
                .primitives,
            );
        }
    }
}
fn semantic_id(root: WidgetId, target: TimelineSelectionTarget) -> WidgetId {
    match target {
        TimelineSelectionTarget::Lane(id) => timeline_lane_widget_id(root, id),
        TimelineSelectionTarget::Item(id) => timeline_item_widget_id(root, id),
        TimelineSelectionTarget::Marker(id) => timeline_marker_widget_id(root, id),
        TimelineSelectionTarget::Keyframe(id) => timeline_keyframe_widget_id(root, id),
    }
}
fn fill(rect: Rect, color: Color) -> Primitive {
    Primitive::Rect(RectPrimitive {
        rect,
        fill: Some(Brush::Solid(color)),
        stroke: None,
        radius: CornerRadius::default(),
    })
}
fn finite_rect(rect: Rect) -> Rect {
    Rect::new(
        if rect.x.is_finite() { rect.x } else { 0.0 },
        if rect.y.is_finite() { rect.y } else { 0.0 },
        finite(rect.width),
        finite(rect.height),
    )
}
fn finite(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}
