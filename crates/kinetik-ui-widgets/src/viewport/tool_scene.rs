//! Prepared viewport transform-tool scene.

use kinetik_ui_core::{
    Modifiers, Point, PointerOrder, PointerTarget, PointerTargetPlan, Response, ScaleFactor,
    WidgetId,
};

use super::{
    ViewportSelectionOutlineDescriptor, ViewportSelectionTargetDescriptor, ViewportSurface,
    ViewportToolDescriptor, ViewportTransformDragCapture, ViewportTransformDragRequest,
    ViewportTransformHandleDescriptor, ViewportTransformHandleHit, ViewportTransformHandleId,
    ViewportTransformHandleKind, ViewportWidget, viewport_selection_outlines_at,
    viewport_transform_handles_at,
};

/// Caller-owned configuration for one viewport transform-tool scene.
#[derive(Debug, Clone, PartialEq)]
pub struct ViewportToolSceneConfig {
    /// Selection targets supplied by the application.
    pub targets: Vec<ViewportSelectionTargetDescriptor>,
    /// Optional active tool metadata used for cursor routing.
    pub active_tool: Option<ViewportToolDescriptor>,
    /// Whether all tool interaction is disabled.
    pub disabled: bool,
    /// Optional screen-space snap tolerance requested from the application.
    pub snap_tolerance: Option<f32>,
}

impl ViewportToolSceneConfig {
    /// Creates an enabled scene for the supplied application-owned targets.
    #[must_use]
    pub fn new(targets: impl IntoIterator<Item = ViewportSelectionTargetDescriptor>) -> Self {
        Self {
            targets: targets.into_iter().collect(),
            active_tool: None,
            disabled: false,
            snap_tolerance: None,
        }
    }

    /// Adds active tool metadata without embedding tool behavior.
    #[must_use]
    pub fn with_active_tool(mut self, tool: ViewportToolDescriptor) -> Self {
        self.active_tool = Some(tool);
        self
    }

    /// Sets whether tool interaction is disabled.
    #[must_use]
    pub const fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Requests application-owned snapping with a screen-space tolerance.
    #[must_use]
    pub const fn with_snap_tolerance(mut self, tolerance: f32) -> Self {
        self.snap_tolerance = Some(tolerance);
        self
    }
}

/// Immutable frame-local transform-tool scene prepared from a viewport snapshot.
#[derive(Debug, Clone, PartialEq)]
pub struct ViewportToolScene {
    viewport_id: WidgetId,
    surface: ViewportSurface,
    scale_factor: ScaleFactor,
    config: ViewportToolSceneConfig,
    outlines: Vec<ViewportSelectionOutlineDescriptor>,
    handles: Vec<ViewportTransformHandleDescriptor>,
}

impl ViewportToolScene {
    /// Resolves outlines and handles through the exact prepared viewport snapshot.
    #[must_use]
    pub fn new(viewport: &ViewportWidget, mut config: ViewportToolSceneConfig) -> Self {
        config.disabled |= viewport.config().disabled;
        config.snap_tolerance = config
            .snap_tolerance
            .filter(|tolerance| tolerance.is_finite() && *tolerance > 0.0);
        let surface = viewport.surface();
        let scale_factor = viewport.scale_factor();
        let mut outlines = viewport_selection_outlines_at(surface, &config.targets, scale_factor);
        let mut handles = viewport_transform_handles_at(surface, &config.targets, scale_factor);

        // Paint and pointer order must agree with the existing hit-test winner:
        // higher priorities win, while lower stable IDs win exact ties.
        outlines.sort_by(|left, right| {
            left.priority
                .cmp(&right.priority)
                .then_with(|| right.target.cmp(&left.target))
        });
        handles.sort_by(|left, right| {
            left.target_priority
                .cmp(&right.target_priority)
                .then_with(|| left.handle_priority.cmp(&right.handle_priority))
                .then_with(|| right.target.cmp(&left.target))
                .then_with(|| right.kind.cmp(&left.kind))
        });

        Self {
            viewport_id: viewport.widget_id(),
            surface,
            scale_factor,
            config,
            outlines,
            handles,
        }
    }

    /// Returns the scene configuration.
    #[must_use]
    pub const fn config(&self) -> &ViewportToolSceneConfig {
        &self.config
    }

    /// Returns the parent viewport widget ID.
    #[must_use]
    pub const fn viewport_id(&self) -> WidgetId {
        self.viewport_id
    }

    /// Returns the exact surface used to resolve this scene.
    #[must_use]
    pub const fn surface(&self) -> ViewportSurface {
        self.surface
    }

    /// Returns the exact scale factor used to resolve this scene.
    #[must_use]
    pub const fn scale_factor(&self) -> ScaleFactor {
        self.scale_factor
    }

    /// Returns application-owned selection targets without mutating them.
    #[must_use]
    pub fn targets(&self) -> &[ViewportSelectionTargetDescriptor] {
        &self.config.targets
    }

    /// Returns resolved selection outlines in back-to-front paint order.
    #[must_use]
    pub fn outlines(&self) -> &[ViewportSelectionOutlineDescriptor] {
        &self.outlines
    }

    /// Returns resolved transform handles in back-to-front paint order.
    #[must_use]
    pub fn handles(&self) -> &[ViewportTransformHandleDescriptor] {
        &self.handles
    }

    /// Returns the stable widget ID for one transform handle.
    #[must_use]
    pub fn handle_widget_id(&self, handle: ViewportTransformHandleId) -> WidgetId {
        viewport_transform_handle_widget_id(self.viewport_id, handle)
    }

    /// Adds clipped handle targets above the already-declared viewport target.
    pub fn declare_pointer_targets(
        &self,
        plan: &mut PointerTargetPlan,
        first_order: PointerOrder,
    ) -> PointerOrder {
        let bounds = self.surface.effective_bounds();
        let mut order = first_order;
        plan.with_clip(bounds, |plan| {
            for handle in &self.handles {
                let id = self.handle_widget_id(handle.id);
                plan.target(
                    PointerTarget::new(id, handle.handle_screen_rect, order)
                        .domain_drag_source()
                        .enabled(!self.config.disabled),
                );
                order = PointerOrder::new(order.raw().saturating_add(1));
            }
        });
        order
    }

    pub(crate) fn handle(
        &self,
        id: ViewportTransformHandleId,
    ) -> Option<&ViewportTransformHandleDescriptor> {
        self.handles.iter().find(|handle| handle.id == id)
    }

    pub(crate) fn capture_from_handle(
        &self,
        handle: &ViewportTransformHandleDescriptor,
        point: Point,
    ) -> ViewportTransformDragCapture {
        let hit = ViewportTransformHandleHit::from_descriptor(
            handle,
            point,
            self.surface,
            self.scale_factor,
        );
        ViewportTransformDragCapture::from_hit(&hit)
    }
}

/// Caller-owned state retained across frames of a viewport handle gesture.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ViewportToolController {
    pub(crate) capture: Option<ViewportTransformDragCapture>,
    pub(crate) started: bool,
}

impl ViewportToolController {
    /// Returns the currently captured handle, if any.
    #[must_use]
    pub fn captured_handle(&self) -> Option<ViewportTransformHandleId> {
        self.capture.as_ref().map(|capture| capture.handle)
    }

    /// Returns true after a captured gesture crosses the drag threshold.
    #[must_use]
    pub const fn transform_started(&self) -> bool {
        self.started
    }
}

/// Application-owned viewport transform interaction phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ViewportTransformInteractionPhase {
    /// First accepted drag update after pointer capture.
    Started,
    /// Later captured drag update.
    Updated,
    /// Captured drag released normally.
    Finished,
    /// Capture was cancelled or became invalid.
    Cancelled,
}

/// Ordered transform request emitted for application execution.
#[derive(Debug, Clone, PartialEq)]
pub struct ViewportTransformInteractionRequest {
    /// Interaction lifecycle phase.
    pub phase: ViewportTransformInteractionPhase,
    /// Original canonical event ordinal, when available.
    pub event_ordinal: Option<usize>,
    /// Modifier state effective at the causal pointer event.
    pub modifiers: Modifiers,
    /// Optional screen-space tolerance requesting domain-owned snapping.
    pub snap_tolerance: Option<f32>,
    /// Raw, unsnapped transform metadata.
    pub drag: ViewportTransformDragRequest,
}

/// Common response paired with stable viewport handle identity.
#[derive(Debug, Clone, PartialEq)]
pub struct ViewportTransformHandleResponse {
    /// Stable handle identity.
    pub handle: ViewportTransformHandleId,
    /// Stable routed widget identity.
    pub widget_id: WidgetId,
    /// Common interaction response.
    pub response: Response,
}

/// Output from one painted viewport transform-tool scene.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ViewportToolSceneOutput {
    /// Responses for resolved handles in paint order.
    pub handle_responses: Vec<ViewportTransformHandleResponse>,
    /// Ordered application-owned transform requests.
    pub interactions: Vec<ViewportTransformInteractionRequest>,
    /// Highest routed handle under the pointer, if any.
    pub hovered_handle: Option<ViewportTransformHandleId>,
}

/// Returns the stable widget ID used to route one viewport transform handle.
#[must_use]
pub fn viewport_transform_handle_widget_id(
    root: WidgetId,
    handle: ViewportTransformHandleId,
) -> WidgetId {
    root.child((
        "viewport-transform-handle",
        handle.target.raw(),
        transform_handle_kind_key(handle.kind),
    ))
}

fn transform_handle_kind_key(kind: ViewportTransformHandleKind) -> &'static str {
    match kind {
        ViewportTransformHandleKind::Move => "move",
        ViewportTransformHandleKind::ResizeTopLeft => "resize-top-left",
        ViewportTransformHandleKind::ResizeTop => "resize-top",
        ViewportTransformHandleKind::ResizeTopRight => "resize-top-right",
        ViewportTransformHandleKind::ResizeRight => "resize-right",
        ViewportTransformHandleKind::ResizeBottomRight => "resize-bottom-right",
        ViewportTransformHandleKind::ResizeBottom => "resize-bottom",
        ViewportTransformHandleKind::ResizeBottomLeft => "resize-bottom-left",
        ViewportTransformHandleKind::ResizeLeft => "resize-left",
        ViewportTransformHandleKind::Rotate => "rotate",
        ViewportTransformHandleKind::Pivot => "pivot",
    }
}
