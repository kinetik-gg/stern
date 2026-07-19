use super::{
    DomainDragGestureAction, DomainDragGesturePhase, Key, KeyState, Point, TimelineClipMoveRequest,
    TimelineClipTrimRequest, TimelineHitTarget, TimelineItemId, TimelineLaneId, TimelineRange,
    TimelineScale, TimelineSnapMetadata, TimelineTime, TimelineTrimEdge, TimelineWidget, Ui,
    UiInputEvent, WidgetId, clamp_timeline_time, scrub_source_is_mutable,
};

/// Clip edit operation retained from pointer press through completion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimelineClipEditKind {
    /// Moves the complete clip range.
    Move,
    /// Trims one stable clip edge.
    Trim(TimelineTrimEdge),
}

/// Canonical application-owned clip mutation request.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TimelineClipEditRequest {
    /// Moves one clip without changing its duration.
    Move(TimelineClipMoveRequest),
    /// Trims one clip edge.
    Trim(TimelineClipTrimRequest),
}

impl TimelineClipEditRequest {
    /// Returns the stable clip identity.
    #[must_use]
    pub const fn target(self) -> TimelineItemId {
        match self {
            Self::Move(request) => request.target,
            Self::Trim(request) => request.target,
        }
    }

    /// Returns the committed range captured at pointer press.
    #[must_use]
    pub const fn original_range(self) -> TimelineRange {
        match self {
            Self::Move(request) => request.original_range,
            Self::Trim(request) => request.original_range,
        }
    }

    /// Returns the accepted preview or commit range.
    #[must_use]
    pub const fn accepted_range(self) -> TimelineRange {
        match self {
            Self::Move(request) => request.snapped_range,
            Self::Trim(request) => request.clamped_range,
        }
    }

    /// Returns whether pointer capture remains requested after this intent.
    #[must_use]
    pub const fn pointer_capture_requested(self) -> bool {
        match self {
            Self::Move(request) => request.pointer_capture_requested,
            Self::Trim(request) => request.pointer_capture_requested,
        }
    }

    fn with_pointer_capture_requested(mut self, value: bool) -> Self {
        match &mut self {
            Self::Move(request) => request.pointer_capture_requested = value,
            Self::Trim(request) => request.pointer_capture_requested = value,
        }
        self
    }
}

/// Ordered application-owned clip edit lifecycle intent.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TimelineClipEditIntent {
    /// Begins a preview transaction from the committed clip range.
    Begin(TimelineClipEditRequest),
    /// Updates the current accepted preview.
    Update(TimelineClipEditRequest),
    /// Ends the transaction with one canonical mutation request.
    End(TimelineClipEditRequest),
    /// Cancels the transaction and restores the committed range.
    Cancel(TimelineClipEditCancelRequest),
    /// Rejects an invalid preview or commit without accepting its range.
    Reject(TimelineClipEditRejection),
}

/// Restoration metadata emitted when a clip edit is cancelled.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimelineClipEditCancelRequest {
    /// Stable clip identity.
    pub target: TimelineItemId,
    /// Stable containing lane identity captured at pointer press.
    pub lane: TimelineLaneId,
    /// Captured edit operation.
    pub kind: TimelineClipEditKind,
    /// Committed range that the application must restore.
    pub original_range: TimelineRange,
    /// Last accepted preview range before cancellation.
    pub preview_range: TimelineRange,
    /// Whether pointer capture should remain held after cancellation.
    pub pointer_capture_requested: bool,
}

/// Lifecycle stage at which a clip edit request was rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimelineClipEditRejectionStage {
    /// The first preview request was rejected.
    Begin,
    /// A later preview request was rejected.
    Update,
    /// The final commit request was rejected.
    End,
}

/// Deterministic reason a clip edit was rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimelineClipEditRejectionReason {
    /// The requested trim crossed the opposing edge.
    InvalidRange,
    /// The requested trim would produce a clip shorter than the configured minimum.
    MinimumDuration,
}

/// Metadata for a rejected clip edit request.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimelineClipEditRejection {
    /// Stable clip identity.
    pub target: TimelineItemId,
    /// Stable containing lane identity captured at pointer press.
    pub lane: TimelineLaneId,
    /// Captured edit operation.
    pub kind: TimelineClipEditKind,
    /// Lifecycle stage that rejected the request.
    pub stage: TimelineClipEditRejectionStage,
    /// Deterministic rejection reason.
    pub reason: TimelineClipEditRejectionReason,
    /// Committed range to preserve or restore.
    pub original_range: TimelineRange,
    /// Last accepted preview range.
    pub preview_range: TimelineRange,
    /// Pointer time that produced the rejected trim.
    pub requested_time: TimelineTime,
    /// Configured minimum accepted duration.
    pub minimum_duration: TimelineTime,
    /// Whether pointer capture remains requested after the rejection.
    pub pointer_capture_requested: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct TimelineClipEditCapture {
    owner: WidgetId,
    target: TimelineItemId,
    lane: TimelineLaneId,
    kind: TimelineClipEditKind,
    scale: TimelineScale,
    press_time: TimelineTime,
    previous_time: TimelineTime,
    original_range: TimelineRange,
    preview_range: TimelineRange,
    minimum_duration: TimelineTime,
    started: bool,
}

/// Caller-owned state retained across frames of one clip move or trim gesture.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TimelineClipEditController {
    capture: Option<TimelineClipEditCapture>,
}

impl TimelineClipEditController {
    /// Returns true after the clip edit crosses the drag threshold.
    #[must_use]
    pub fn is_editing(&self) -> bool {
        self.capture.is_some_and(|capture| capture.started)
    }

    /// Returns the stable clip identity captured for the current gesture.
    #[must_use]
    pub fn target(&self) -> Option<TimelineItemId> {
        self.capture.map(|capture| capture.target)
    }

    /// Returns the stable edit kind captured for the current gesture.
    #[must_use]
    pub fn kind(&self) -> Option<TimelineClipEditKind> {
        self.capture.map(|capture| capture.kind)
    }

    /// Returns the transform frozen at pointer press.
    #[must_use]
    pub fn frozen_scale(&self) -> Option<TimelineScale> {
        self.capture.map(|capture| capture.scale)
    }

    /// Returns the last accepted preview range.
    #[must_use]
    pub fn preview_range(&self) -> Option<TimelineRange> {
        self.capture.map(|capture| capture.preview_range)
    }
}

impl Ui<'_> {
    pub(super) fn resolve_timeline_clip_edit(
        &mut self,
        widget: &TimelineWidget<'_>,
        controller: &mut TimelineClipEditController,
        actions: &[DomainDragGestureAction],
    ) -> Vec<TimelineClipEditIntent> {
        if widget.config.disabled || widget.config.read_only {
            let mut intents = Vec::new();
            if controller.capture.is_some() {
                push_clip_cancel(controller, &mut intents);
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
        if controller.capture.is_some() && !captured_item_is_mutable(widget, controller) {
            let mut intents = Vec::new();
            push_invalidated_clip_cancel(controller, &mut intents);
            self.cancel_pointer_interaction();
            return intents;
        }
        let escape_ordinal = self.input().events.iter().position(|event| {
            matches!(event, UiInputEvent::Key(event) if event.state == KeyState::Pressed && !event.repeat && matches!(event.key, Key::Escape))
        });
        let drag_crossed_threshold = self.memory().is_drag_source(widget.widget_id())
            || self.memory().released_drag_source() == Some(widget.widget_id());
        let start_move = (!controller.is_editing() && drag_crossed_threshold)
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
            apply_clip_edit_action(
                widget,
                controller,
                action,
                drag_crossed_threshold,
                start_move == Some(index),
                &mut intents,
            );
        }
        if escape_ordinal.is_some() && controller.capture.is_some() {
            push_clip_cancel(controller, &mut intents);
            self.cancel_pointer_interaction();
        }
        intents
    }
}

fn apply_clip_edit_action(
    widget: &TimelineWidget<'_>,
    controller: &mut TimelineClipEditController,
    action: &DomainDragGestureAction,
    drag_crossed_threshold: bool,
    starts_edit: bool,
    intents: &mut Vec<TimelineClipEditIntent>,
) {
    match action.phase {
        DomainDragGesturePhase::Press => begin_capture(widget, controller, action.position),
        DomainDragGesturePhase::Move => {
            if !controller.is_editing() && !starts_edit {
                return;
            }
            let Some(capture) = controller.capture.as_mut() else {
                return;
            };
            let stage = if capture.started {
                TimelineClipEditRejectionStage::Update
            } else {
                TimelineClipEditRejectionStage::Begin
            };
            match clip_request(*capture, action.position) {
                Ok((request, pointer_time)) => {
                    capture.previous_time = pointer_time;
                    capture.preview_range = request.accepted_range();
                    if capture.started {
                        intents.push(TimelineClipEditIntent::Update(request));
                    } else {
                        capture.started = true;
                        intents.push(TimelineClipEditIntent::Begin(request));
                    }
                }
                Err((reason, requested_time)) => intents.push(TimelineClipEditIntent::Reject(
                    clip_rejection(*capture, stage, reason, requested_time, true),
                )),
            }
        }
        DomainDragGesturePhase::Release => {
            let Some(mut capture) = controller.capture.take() else {
                return;
            };
            if !capture.started && !drag_crossed_threshold {
                return;
            }
            match clip_request(capture, action.position) {
                Ok((request, pointer_time)) => {
                    capture.previous_time = pointer_time;
                    capture.preview_range = request.accepted_range();
                    if !capture.started {
                        intents.push(TimelineClipEditIntent::Begin(request));
                    }
                    intents.push(TimelineClipEditIntent::End(
                        request.with_pointer_capture_requested(false),
                    ));
                }
                Err((reason, requested_time)) => {
                    intents.push(TimelineClipEditIntent::Reject(clip_rejection(
                        capture,
                        TimelineClipEditRejectionStage::End,
                        reason,
                        requested_time,
                        false,
                    )));
                }
            }
        }
        DomainDragGesturePhase::Cancel => push_clip_cancel(controller, intents),
    }
}

fn begin_capture(
    widget: &TimelineWidget<'_>,
    controller: &mut TimelineClipEditController,
    position: Option<Point>,
) {
    let Some(point) = position else {
        return;
    };
    let Some(hit) = widget.hit(point).filter(scrub_source_is_mutable) else {
        return;
    };
    let (target, kind) = match hit.target {
        TimelineHitTarget::Item(target) => (target, TimelineClipEditKind::Move),
        TimelineHitTarget::ItemTrimStartHandle(target) => {
            (target, TimelineClipEditKind::Trim(TimelineTrimEdge::Start))
        }
        TimelineHitTarget::ItemTrimEndHandle(target) => {
            (target, TimelineClipEditKind::Trim(TimelineTrimEdge::End))
        }
        _ => return,
    };
    let Some(item) = widget
        .layout
        .items
        .iter()
        .find(|item| item.descriptor.id == target)
    else {
        return;
    };
    if item.descriptor.state.disabled || item.descriptor.state.read_only {
        return;
    }
    let original_range = item.time_range.sanitized();
    let minimum_duration = TimelineTime::from_seconds(
        widget
            .config
            .minimum_clip_duration
            .sanitized()
            .seconds()
            .max(0.0),
    );
    controller.capture = Some(TimelineClipEditCapture {
        owner: widget.widget_id(),
        target,
        lane: item.descriptor.lane,
        kind,
        scale: widget.scale,
        press_time: clip_time(widget.scale, Some(point), hit.time),
        previous_time: clip_time(widget.scale, Some(point), hit.time),
        original_range,
        preview_range: original_range,
        minimum_duration,
        started: false,
    });
}

fn clip_request(
    capture: TimelineClipEditCapture,
    position: Option<Point>,
) -> Result<(TimelineClipEditRequest, TimelineTime), (TimelineClipEditRejectionReason, TimelineTime)>
{
    let pointer_time = clip_time(capture.scale, position, capture.previous_time);
    match capture.kind {
        TimelineClipEditKind::Move => {
            let requested_delta =
                TimelineTime::from_seconds(pointer_time.seconds() - capture.press_time.seconds())
                    .sanitized();
            let requested_range = offset_range(capture.original_range, requested_delta);
            let snap = TimelineSnapMetadata::unsnapped(requested_range.start);
            Ok((
                TimelineClipEditRequest::Move(TimelineClipMoveRequest {
                    target: capture.target,
                    lane: capture.lane,
                    original_range: capture.original_range,
                    requested_delta,
                    snapped_delta: requested_delta,
                    requested_range,
                    snapped_range: requested_range,
                    snap,
                    pointer_capture_requested: true,
                }),
                pointer_time,
            ))
        }
        TimelineClipEditKind::Trim(edge) => {
            validate_trim(capture, edge, pointer_time)?;
            let clamped_range = match edge {
                TimelineTrimEdge::Start => {
                    TimelineRange::new(pointer_time, capture.original_range.end)
                }
                TimelineTrimEdge::End => {
                    TimelineRange::new(capture.original_range.start, pointer_time)
                }
            };
            Ok((
                TimelineClipEditRequest::Trim(TimelineClipTrimRequest {
                    target: capture.target,
                    edge,
                    original_range: capture.original_range,
                    requested_time: pointer_time,
                    clamped_time: pointer_time,
                    clamped_range,
                    snap: TimelineSnapMetadata::unsnapped(pointer_time),
                    pointer_capture_requested: true,
                }),
                pointer_time,
            ))
        }
    }
}

fn validate_trim(
    capture: TimelineClipEditCapture,
    edge: TimelineTrimEdge,
    requested_time: TimelineTime,
) -> Result<(), (TimelineClipEditRejectionReason, TimelineTime)> {
    let requested = requested_time.seconds();
    let start = capture.original_range.start.seconds();
    let end = capture.original_range.end.seconds();
    let duration = match edge {
        TimelineTrimEdge::Start if requested > end => {
            return Err((
                TimelineClipEditRejectionReason::InvalidRange,
                requested_time,
            ));
        }
        TimelineTrimEdge::End if requested < start => {
            return Err((
                TimelineClipEditRejectionReason::InvalidRange,
                requested_time,
            ));
        }
        TimelineTrimEdge::Start => end - requested,
        TimelineTrimEdge::End => requested - start,
    };
    if duration < capture.minimum_duration.seconds() {
        return Err((
            TimelineClipEditRejectionReason::MinimumDuration,
            requested_time,
        ));
    }
    Ok(())
}

fn clip_rejection(
    capture: TimelineClipEditCapture,
    stage: TimelineClipEditRejectionStage,
    reason: TimelineClipEditRejectionReason,
    requested_time: TimelineTime,
    pointer_capture_requested: bool,
) -> TimelineClipEditRejection {
    TimelineClipEditRejection {
        target: capture.target,
        lane: capture.lane,
        kind: capture.kind,
        stage,
        reason,
        original_range: capture.original_range,
        preview_range: capture.preview_range,
        requested_time,
        minimum_duration: capture.minimum_duration,
        pointer_capture_requested,
    }
}

fn push_clip_cancel(
    controller: &mut TimelineClipEditController,
    intents: &mut Vec<TimelineClipEditIntent>,
) {
    let Some(capture) = controller.capture.take() else {
        return;
    };
    if capture.started {
        intents.push(TimelineClipEditIntent::Cancel(clip_cancel_request(capture)));
    }
}

fn push_invalidated_clip_cancel(
    controller: &mut TimelineClipEditController,
    intents: &mut Vec<TimelineClipEditIntent>,
) {
    if let Some(capture) = controller.capture.take() {
        intents.push(TimelineClipEditIntent::Cancel(clip_cancel_request(capture)));
    }
}

fn clip_cancel_request(capture: TimelineClipEditCapture) -> TimelineClipEditCancelRequest {
    TimelineClipEditCancelRequest {
        target: capture.target,
        lane: capture.lane,
        kind: capture.kind,
        original_range: capture.original_range,
        preview_range: capture.preview_range,
        pointer_capture_requested: false,
    }
}

fn captured_item_is_mutable(
    widget: &TimelineWidget<'_>,
    controller: &TimelineClipEditController,
) -> bool {
    let Some(capture) = controller.capture else {
        return true;
    };
    widget
        .config
        .descriptor
        .items
        .iter()
        .find(|item| item.id == capture.target)
        .is_some_and(|item| !item.state.disabled && !item.state.read_only)
}

fn clip_time(
    scale: TimelineScale,
    position: Option<Point>,
    fallback: TimelineTime,
) -> TimelineTime {
    position.map_or(fallback, |point| {
        clamp_timeline_time(scale.screen_x_to_time(point.x), scale.content_range)
    })
}

fn offset_range(range: TimelineRange, delta: TimelineTime) -> TimelineRange {
    TimelineRange::new(
        TimelineTime::from_seconds(range.start.seconds() + delta.seconds()).sanitized(),
        TimelineTime::from_seconds(range.end.seconds() + delta.seconds()).sanitized(),
    )
}
