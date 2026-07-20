//! Prepared public viewport widget contract.

use stern_core::{
    Point, PointerOrder, PointerTarget, PointerTargetPlan, Primitive, Rect, Response, ScaleFactor,
    TexturePrimitive, Vec2, WidgetId,
};

use super::{
    PanZoom, ViewportActionDescriptor, ViewportActionRequest, ViewportSurface, finite_or_zero,
};

const DEFAULT_MIN_ZOOM: f32 = 0.05;
const DEFAULT_MAX_ZOOM: f32 = 64.0;
const DEFAULT_ZOOM_STEP: f32 = 0.2;

/// Exact value-owned geometry used to present one viewport surface.
///
/// The content extent is snapped to the physical grid while finite logical
/// pan remains continuous in the origin. Paint, same-frame conversions, and
/// presented tool geometry consume this same value.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewportPresentation {
    surface: ViewportSurface,
    scale_factor: ScaleFactor,
    content_rect: Rect,
}

impl ViewportPresentation {
    /// Resolves presentation geometry from application-owned surface state.
    #[must_use]
    pub fn new(surface: ViewportSurface, scale_factor: ScaleFactor) -> Self {
        let bounds = surface.effective_bounds();
        let content_rect = surface.effective_source_size().map_or_else(
            || Rect::new(bounds.x, bounds.y, 0.0, 0.0),
            |source| {
                let content_scale = surface.content_scale_at(scale_factor);
                let width = snap_extent(source.width * content_scale, scale_factor);
                let height = snap_extent(source.height * content_scale, scale_factor);
                if width <= 0.0 || height <= 0.0 {
                    return Rect::new(bounds.x, bounds.y, 0.0, 0.0);
                }
                Rect::new(
                    bounds.x
                        + (bounds.width - width) * 0.5
                        + finite_or_zero(surface.pan_zoom.pan.x),
                    bounds.y
                        + (bounds.height - height) * 0.5
                        + finite_or_zero(surface.pan_zoom.pan.y),
                    width,
                    height,
                )
            },
        );
        Self {
            surface,
            scale_factor,
            content_rect,
        }
    }

    /// Returns the application-owned state backing this presentation.
    #[must_use]
    pub const fn surface(self) -> ViewportSurface {
        self.surface
    }

    /// Returns the scale factor used to resolve this presentation.
    #[must_use]
    pub const fn scale_factor(self) -> ScaleFactor {
        self.scale_factor
    }

    /// Returns the exact texture, conversion, and overlay rectangle.
    #[must_use]
    pub const fn content_rect(self) -> Rect {
        self.content_rect
    }

    /// Returns the resolved content-to-screen scale on each axis.
    #[must_use]
    pub fn content_scale(self) -> Vec2 {
        let Some(source) = self.surface.effective_source_size() else {
            return Vec2::ZERO;
        };
        let scale = Vec2::new(
            self.content_rect.width / source.width,
            self.content_rect.height / source.height,
        );
        if scale.x.is_finite() && scale.x > 0.0 && scale.y.is_finite() && scale.y > 0.0 {
            scale
        } else {
            Vec2::ZERO
        }
    }

    /// Converts a screen point through this exact presentation.
    #[must_use]
    pub fn screen_to_content(self, point: Point) -> Option<Point> {
        if !point.x.is_finite() || !point.y.is_finite() {
            return None;
        }
        let scale = self.content_scale();
        if scale.x <= 0.0 || scale.y <= 0.0 {
            return None;
        }
        Some(Point::new(
            (point.x - self.content_rect.x) / scale.x,
            (point.y - self.content_rect.y) / scale.y,
        ))
    }

    /// Converts a content point through this exact presentation.
    #[must_use]
    pub fn content_to_screen(self, point: Point) -> Option<Point> {
        if !point.x.is_finite() || !point.y.is_finite() {
            return None;
        }
        let scale = self.content_scale();
        if scale.x <= 0.0 || scale.y <= 0.0 {
            return None;
        }
        Some(Point::new(
            self.content_rect.x + point.x * scale.x,
            self.content_rect.y + point.y * scale.y,
        ))
    }

    /// Converts a content rectangle through this exact presentation.
    #[must_use]
    pub fn content_rect_to_screen(self, rect: Rect) -> Option<Rect> {
        if !rect.x.is_finite()
            || !rect.y.is_finite()
            || !rect.width.is_finite()
            || !rect.height.is_finite()
            || rect.width < 0.0
            || rect.height < 0.0
        {
            return None;
        }
        let origin = self.content_to_screen(rect.origin())?;
        let scale = self.content_scale();
        Some(Rect::new(
            origin.x,
            origin.y,
            rect.width * scale.x,
            rect.height * scale.y,
        ))
    }

    pub(crate) fn texture_primitive(self) -> Primitive {
        Primitive::Texture(TexturePrimitive {
            texture: self.surface.texture,
            rect: self.content_rect,
            source_size: self.surface.effective_source_size().unwrap_or_default(),
        })
    }
}

fn snap_extent(value: f32, scale_factor: ScaleFactor) -> f32 {
    if !value.is_finite() || value < 0.0 || !scale_factor.is_valid() {
        return finite_or_zero(value).max(0.0);
    }
    let physical = (f64::from(value) * scale_factor.value()).round();
    let logical = physical / scale_factor.value();
    if logical.is_finite() && logical >= 0.0 && logical <= f64::from(f32::MAX) {
        #[allow(clippy::cast_possible_truncation)]
        {
            logical as f32
        }
    } else {
        0.0
    }
}

/// Caller-owned configuration for one prepared viewport widget.
#[derive(Debug, Clone, PartialEq)]
pub struct ViewportWidgetConfig {
    /// Stable widget identity.
    pub id: WidgetId,
    /// Frozen texture surface and current pan/zoom snapshot.
    pub surface: ViewportSurface,
    /// Accessible viewport label.
    pub label: String,
    /// Whether interaction is disabled.
    pub disabled: bool,
    /// Minimum custom zoom factor.
    pub min_zoom: f32,
    /// Maximum custom zoom factor.
    pub max_zoom: f32,
    /// Exponential wheel and action zoom step.
    pub zoom_step: f32,
    /// App-owned viewport actions exposed through semantics.
    pub actions: Vec<ViewportActionDescriptor>,
}

impl ViewportWidgetConfig {
    /// Creates an enabled viewport configuration with practical zoom defaults.
    #[must_use]
    pub fn new(id: WidgetId, surface: ViewportSurface) -> Self {
        Self {
            id,
            surface,
            label: "Viewport".to_owned(),
            disabled: false,
            min_zoom: DEFAULT_MIN_ZOOM,
            max_zoom: DEFAULT_MAX_ZOOM,
            zoom_step: DEFAULT_ZOOM_STEP,
            actions: Vec::new(),
        }
    }

    /// Sets the accessible label.
    #[must_use]
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    /// Sets whether interaction is disabled.
    #[must_use]
    pub const fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Sets the inclusive custom zoom range.
    #[must_use]
    pub const fn with_zoom_range(mut self, min: f32, max: f32) -> Self {
        self.min_zoom = min;
        self.max_zoom = max;
        self
    }

    /// Sets the exponential wheel and action zoom step.
    #[must_use]
    pub const fn with_zoom_step(mut self, step: f32) -> Self {
        self.zoom_step = step;
        self
    }

    /// Replaces the app-owned viewport action descriptors.
    #[must_use]
    pub fn with_actions(
        mut self,
        actions: impl IntoIterator<Item = ViewportActionDescriptor>,
    ) -> Self {
        self.actions = actions.into_iter().collect();
        self
    }
}

/// Immutable frame-local viewport widget.
#[derive(Debug, Clone, PartialEq)]
pub struct ViewportWidget {
    config: ViewportWidgetConfig,
    scale_factor: ScaleFactor,
}

impl ViewportWidget {
    /// Prepares a viewport widget and sanitizes its zoom policy.
    #[must_use]
    pub fn new(mut config: ViewportWidgetConfig, scale_factor: ScaleFactor) -> Self {
        let (min_zoom, max_zoom) = sanitize_zoom_range(config.min_zoom, config.max_zoom);
        config.min_zoom = min_zoom;
        config.max_zoom = max_zoom;
        config.zoom_step = sanitize_zoom_step(config.zoom_step);
        Self {
            config,
            scale_factor,
        }
    }

    /// Returns the prepared configuration.
    #[must_use]
    pub const fn config(&self) -> &ViewportWidgetConfig {
        &self.config
    }

    /// Returns the stable viewport widget identity.
    #[must_use]
    pub const fn widget_id(&self) -> WidgetId {
        self.config.id
    }

    /// Returns the frozen texture and pan/zoom snapshot.
    #[must_use]
    pub const fn surface(&self) -> ViewportSurface {
        self.config.surface
    }

    /// Resolves the widget's frozen presentation geometry.
    #[must_use]
    pub fn presentation(&self) -> ViewportPresentation {
        ViewportPresentation::new(self.config.surface, self.scale_factor)
    }

    /// Returns the frame scale used by paint and coordinate conversion.
    #[must_use]
    pub const fn scale_factor(&self) -> ScaleFactor {
        self.scale_factor
    }

    /// Converts a screen point through the frozen painted snapshot.
    #[must_use]
    pub fn screen_to_content(&self, point: Point) -> Option<Point> {
        self.presentation().screen_to_content(point)
    }

    /// Converts a content point through the frozen painted snapshot.
    #[must_use]
    pub fn content_to_screen(&self, point: Point) -> Option<Point> {
        self.presentation().content_to_screen(point)
    }

    /// Adds the viewport blocker and routed interaction target to a pointer plan.
    pub fn declare_pointer_targets(
        &self,
        plan: &mut PointerTargetPlan,
        first_order: PointerOrder,
    ) -> PointerOrder {
        let bounds = self.config.surface.effective_bounds();
        if bounds.width <= 0.0 || bounds.height <= 0.0 {
            return first_order;
        }

        plan.blocker(bounds, first_order);
        let target_order = PointerOrder::new(first_order.raw().saturating_add(1));
        plan.target(
            PointerTarget::new(self.config.id, bounds, target_order)
                .wheel_owner(self.config.id)
                .domain_drag_source()
                .enabled(!self.config.disabled),
        );
        PointerOrder::new(target_order.raw().saturating_add(1))
    }
}

/// Output from one public viewport widget evaluation.
#[derive(Debug, Clone, PartialEq)]
pub struct ViewportWidgetOutput {
    /// Common interaction response for the viewport surface.
    pub response: Response,
    /// Accepted effective surface used for current-frame presentation.
    pub surface: ViewportSurface,
    /// Pan/zoom state staged for the caller's next prepared frame.
    pub next_pan_zoom: PanZoom,
    /// Pointer position converted through the effective presentation, when inside it.
    pub content_pointer: Option<Point>,
    /// Whether pan changed this frame.
    pub pan_changed: bool,
    /// Whether custom zoom changed this frame.
    pub zoom_changed: bool,
    /// Whether fit/display mode changed this frame.
    pub fit_changed: bool,
    /// Targeted action requests not consumed by generic viewport navigation.
    pub action_requests: Vec<ViewportActionRequest>,
}

impl ViewportWidgetOutput {
    /// Returns true when the caller should prepare a new pan/zoom snapshot.
    #[must_use]
    pub const fn changed(&self) -> bool {
        self.pan_changed || self.zoom_changed || self.fit_changed
    }

    /// Resolves this frame's effective presentation at the supplied scale.
    #[must_use]
    pub fn presentation_at(&self, scale_factor: ScaleFactor) -> ViewportPresentation {
        ViewportPresentation::new(self.surface, scale_factor)
    }

    /// Converts a screen point through this frame's effective presentation.
    #[must_use]
    pub fn screen_to_content_at(&self, point: Point, scale_factor: ScaleFactor) -> Option<Point> {
        self.presentation_at(scale_factor).screen_to_content(point)
    }

    /// Converts a content point through this frame's effective presentation.
    #[must_use]
    pub fn content_to_screen_at(&self, point: Point, scale_factor: ScaleFactor) -> Option<Point> {
        self.presentation_at(scale_factor).content_to_screen(point)
    }
}

fn sanitize_zoom_range(min: f32, max: f32) -> (f32, f32) {
    if min.is_finite() && max.is_finite() && min > 0.0 && max >= min {
        (min, max)
    } else {
        (DEFAULT_MIN_ZOOM, DEFAULT_MAX_ZOOM)
    }
}

fn sanitize_zoom_step(step: f32) -> f32 {
    if step.is_finite() && step > 0.0 {
        step.min(4.0)
    } else {
        DEFAULT_ZOOM_STEP
    }
}
