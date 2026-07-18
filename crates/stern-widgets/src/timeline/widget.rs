//! Retained timeline composition over the public timeline contracts.
use super::{
    TimelineDescriptor, TimelineDescriptorError, TimelineDescriptorState, TimelineFrameRate,
    TimelineHitMetadata, TimelineHitTarget, TimelineHitTestConfig, TimelineId, TimelineLayout,
    TimelineLayoutResult, TimelinePlayheadSeekRequest, TimelineRulerId, TimelineRulerTickKind,
    TimelineRulerTickRequest, TimelineScale, TimelineSelectionOperation, TimelineSelectionTarget,
    TimelineSnapMetadata, TimelineViewportState, timeline_item_widget_id,
    timeline_keyframe_widget_id, timeline_lane_widget_id, timeline_marker_widget_id,
    timeline_semantics,
};
use crate::{Ui, label, panel, separator};
use stern_core::{
    Brush, Color, CornerRadius, Modifiers, Point, PointerOrder, PointerTarget, PointerTargetPlan,
    Primitive, Rect, RectPrimitive, Response, WidgetId,
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
/// Output from one timeline evaluation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimelineWidgetOutput {
    /// Common surface response.
    pub response: Response,
    /// Deterministic target below the pointer.
    pub hit: Option<TimelineHitMetadata>,
    /// Typed app-owned intent emitted by activation.
    pub intent: Option<TimelineWidgetIntent>,
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
        let disabled = widget.config.disabled || !widget.valid();
        self.register_id(widget.widget_id());
        let response = self.pressable_with_id(widget.widget_id(), widget.bounds, disabled);
        let hit = self
            .input()
            .pointer
            .position
            .and_then(|point| widget.hit(point));
        let intent = response
            .clicked
            .then_some(hit)
            .flatten()
            .filter(|hit| !hit.disabled() && !hit.read_only())
            .and_then(|hit| intent(hit, widget, self.input().keyboard.modifiers));
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
            }
            self.push_semantic_node(node);
        }
        TimelineWidgetOutput {
            response,
            hit,
            intent,
        }
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
