#[allow(clippy::wildcard_imports)]
use super::*;

/// Viewport guide orientation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ViewportGuideOrientation {
    /// Horizontal guide line.
    Horizontal,
    /// Vertical guide line.
    Vertical,
}

/// Coordinate placement for a viewport guide.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ViewportGuidePlacement {
    /// Guide position is in source content units.
    Content(f32),
    /// Guide position is already in UI logical screen space.
    Screen(f32),
}

/// Application-supplied data-only viewport guide descriptor.
#[derive(Debug, Clone, PartialEq)]
pub struct ViewportGuideDescriptor {
    /// Stable guide identity.
    pub id: ViewportGuideId,
    /// Guide orientation.
    pub orientation: ViewportGuideOrientation,
    /// Guide axis placement.
    pub placement: ViewportGuidePlacement,
    /// Explicit sorting and hit-test priority. Higher priority is visually later.
    pub priority: i32,
    /// Whether this guide can emit interaction requests.
    pub enabled: bool,
    /// Whether guide editing should be suppressed by callers.
    pub locked: bool,
    /// Optional accessible/debug label.
    pub label: Option<String>,
}

impl ViewportGuideDescriptor {
    /// Creates an enabled, unlocked viewport guide descriptor.
    #[must_use]
    pub const fn new(
        id: ViewportGuideId,
        orientation: ViewportGuideOrientation,
        placement: ViewportGuidePlacement,
    ) -> Self {
        Self {
            id,
            orientation,
            placement,
            priority: 0,
            enabled: true,
            locked: false,
            label: None,
        }
    }

    /// Sets explicit sorting priority. Higher priority is visually later.
    #[must_use]
    pub const fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    /// Marks the guide as enabled or disabled.
    #[must_use]
    pub const fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Marks the guide as locked or editable.
    #[must_use]
    pub const fn locked(mut self, locked: bool) -> Self {
        self.locked = locked;
        self
    }

    /// Adds an accessible/debug label.
    #[must_use]
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    fn screen_position(
        &self,
        surface: ViewportSurface,
        scale_factor: ScaleFactor,
    ) -> Option<(f32, Option<f32>)> {
        match self.placement {
            ViewportGuidePlacement::Content(position) => {
                finite_content_guide_position(surface, self.orientation, position)?;
                let screen = match self.orientation {
                    ViewportGuideOrientation::Horizontal => {
                        surface
                            .content_to_screen_at(Point::new(0.0, position), scale_factor)?
                            .y
                    }
                    ViewportGuideOrientation::Vertical => {
                        surface
                            .content_to_screen_at(Point::new(position, 0.0), scale_factor)?
                            .x
                    }
                };
                screen.is_finite().then_some((screen, Some(position)))
            }
            ViewportGuidePlacement::Screen(position) => {
                let position = finite_or_none(position)?;
                guide_position_inside_bounds(surface.effective_bounds(), self.orientation, position)
                    .then_some((position, None))
            }
        }
    }
}

/// Viewport guide descriptor resolved into UI logical screen space.
#[derive(Debug, Clone, PartialEq)]
pub struct ViewportResolvedGuide {
    /// Stable guide identity.
    pub id: ViewportGuideId,
    /// Guide orientation.
    pub orientation: ViewportGuideOrientation,
    /// Source placement.
    pub placement: ViewportGuidePlacement,
    /// Resolved UI logical screen-space axis position.
    pub screen_position: f32,
    /// Resolved source content axis position, when the guide is content-placed.
    pub content_position: Option<f32>,
    /// Thin semantic/hit rectangle for the guide in UI logical screen space.
    pub screen_rect: Rect,
    /// Sorting priority inherited from the source descriptor.
    pub priority: i32,
    /// Whether this guide can emit interaction requests.
    pub enabled: bool,
    /// Whether guide editing should be suppressed by callers.
    pub locked: bool,
    /// Optional accessible/debug label.
    pub label: Option<String>,
}

impl ViewportResolvedGuide {
    fn from_descriptor(
        descriptor: &ViewportGuideDescriptor,
        surface: ViewportSurface,
        scale_factor: ScaleFactor,
    ) -> Option<Self> {
        let (screen_position, content_position) =
            descriptor.screen_position(surface, scale_factor)?;
        let screen_rect = guide_screen_rect(
            surface.effective_bounds(),
            descriptor.orientation,
            screen_position,
        )?;

        Some(Self {
            id: descriptor.id,
            orientation: descriptor.orientation,
            placement: descriptor.placement,
            screen_position,
            content_position,
            screen_rect,
            priority: descriptor.priority,
            enabled: descriptor.enabled,
            locked: descriptor.locked,
            label: descriptor.label.clone(),
        })
    }

    /// Emits a backend-neutral guide line primitive.
    #[must_use]
    pub fn primitive(&self, color: Color) -> Primitive {
        match self.orientation {
            ViewportGuideOrientation::Horizontal => Primitive::Line(LinePrimitive {
                from: Point::new(self.screen_rect.x, self.screen_position),
                to: Point::new(self.screen_rect.max_x(), self.screen_position),
                stroke: Stroke::new(1.0, Brush::Solid(color)),
            }),
            ViewportGuideOrientation::Vertical => Primitive::Line(LinePrimitive {
                from: Point::new(self.screen_position, self.screen_rect.y),
                to: Point::new(self.screen_position, self.screen_rect.max_y()),
                stroke: Stroke::new(1.0, Brush::Solid(color)),
            }),
        }
    }

    /// Builds backend-neutral semantic metadata for this guide.
    #[must_use]
    pub fn semantics(&self, root: WidgetId) -> SemanticNode {
        let mut node = SemanticNode::new(
            viewport_guide_widget_id(root, self.id),
            SemanticRole::Custom("viewport-guide".to_owned()),
            self.screen_rect,
        )
        .with_label(
            self.label
                .clone()
                .unwrap_or_else(|| format!("Viewport guide {}", self.id.raw())),
        );
        node.state.disabled = !self.enabled;
        node.state.value = Some(SemanticValue::Text(format!(
            "{:?} guide at {:.3}{}",
            self.orientation,
            self.screen_position,
            if self.locked { " locked" } else { "" }
        )));
        node
    }
}

/// Resolves viewport guide descriptors into finite UI logical screen-space metadata.
#[must_use]
pub fn viewport_guides(
    surface: ViewportSurface,
    guides: &[ViewportGuideDescriptor],
) -> Vec<ViewportResolvedGuide> {
    viewport_guides_at(surface, guides, ScaleFactor::ONE)
}

/// Resolves viewport guide descriptors into finite UI logical screen-space metadata
/// for a viewport scale factor.
#[must_use]
pub fn viewport_guides_at(
    surface: ViewportSurface,
    guides: &[ViewportGuideDescriptor],
    scale_factor: ScaleFactor,
) -> Vec<ViewportResolvedGuide> {
    let mut guides = guides
        .iter()
        .filter_map(|guide| ViewportResolvedGuide::from_descriptor(guide, surface, scale_factor))
        .collect::<Vec<_>>();
    guides.sort_by(|left, right| {
        left.priority
            .cmp(&right.priority)
            .then_with(|| left.orientation.cmp(&right.orientation))
            .then_with(|| guide_sort_key(left).total_cmp(&guide_sort_key(right)))
            .then_with(|| left.id.cmp(&right.id))
    });
    guides
}

/// Coordinate space used by a viewport safe-area rectangle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ViewportSafeAreaSpace {
    /// Rectangle is in source content coordinates.
    Content,
    /// Rectangle is local to the viewport bounds.
    Viewport,
}

/// Application-supplied data-only viewport safe-area descriptor.
#[derive(Debug, Clone, PartialEq)]
pub struct ViewportSafeAreaDescriptor {
    /// Stable safe-area identity.
    pub id: ViewportSafeAreaId,
    /// Safe-area rectangle in `space`.
    pub rect: Rect,
    /// Coordinate space used by `rect`.
    pub space: ViewportSafeAreaSpace,
    /// Explicit sorting priority. Higher priority is visually later.
    pub priority: i32,
    /// Whether this safe-area metadata is enabled.
    pub enabled: bool,
    /// Optional accessible/debug label.
    pub label: Option<String>,
}

impl ViewportSafeAreaDescriptor {
    /// Creates an enabled viewport safe-area descriptor.
    #[must_use]
    pub const fn new(id: ViewportSafeAreaId, rect: Rect, space: ViewportSafeAreaSpace) -> Self {
        Self {
            id,
            rect,
            space,
            priority: 0,
            enabled: true,
            label: None,
        }
    }

    /// Sets explicit sorting priority. Higher priority is visually later.
    #[must_use]
    pub const fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    /// Marks the safe area as enabled or disabled.
    #[must_use]
    pub const fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Adds an accessible/debug label.
    #[must_use]
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }
}

/// Viewport safe-area descriptor resolved into UI logical screen space.
#[derive(Debug, Clone, PartialEq)]
pub struct ViewportResolvedSafeArea {
    /// Stable safe-area identity.
    pub id: ViewportSafeAreaId,
    /// Source coordinate space.
    pub space: ViewportSafeAreaSpace,
    /// Sanitized source rectangle in the descriptor coordinate space.
    pub rect: Rect,
    /// Resolved UI logical screen-space rectangle.
    pub screen_rect: Rect,
    /// Resolved source content rectangle, when conversion is possible.
    pub content_rect: Option<Rect>,
    /// Sorting priority inherited from the source descriptor.
    pub priority: i32,
    /// Whether this safe-area metadata is enabled.
    pub enabled: bool,
    /// Optional accessible/debug label.
    pub label: Option<String>,
}

impl ViewportResolvedSafeArea {
    fn from_descriptor(
        descriptor: &ViewportSafeAreaDescriptor,
        surface: ViewportSurface,
        scale_factor: ScaleFactor,
    ) -> Option<Self> {
        let viewport_bounds = surface.effective_bounds();
        let (rect, screen_rect, content_rect) = match descriptor.space {
            ViewportSafeAreaSpace::Content => {
                let source = surface.effective_source_size()?;
                let content_bounds = Rect::new(0.0, 0.0, source.width, source.height);
                let rect = sanitize_rect(descriptor.rect).intersection(content_bounds)?;
                let screen_rect = surface.content_rect_to_screen_at(rect, scale_factor)?;
                (rect, screen_rect, Some(rect))
            }
            ViewportSafeAreaSpace::Viewport => {
                let local_bounds =
                    Rect::new(0.0, 0.0, viewport_bounds.width, viewport_bounds.height);
                let rect = sanitize_rect(descriptor.rect).intersection(local_bounds)?;
                let screen_rect = Rect::new(
                    viewport_bounds.x + rect.x,
                    viewport_bounds.y + rect.y,
                    rect.width,
                    rect.height,
                );
                let content_rect = surface.screen_rect_to_content_at(screen_rect, scale_factor);
                (rect, screen_rect, content_rect)
            }
        };

        Some(Self {
            id: descriptor.id,
            space: descriptor.space,
            rect,
            screen_rect: finite_positive_rect(screen_rect)?,
            content_rect: content_rect.and_then(finite_positive_rect),
            priority: descriptor.priority,
            enabled: descriptor.enabled,
            label: descriptor.label.clone(),
        })
    }

    /// Emits a backend-neutral safe-area rectangle primitive.
    #[must_use]
    pub fn primitive(&self, fill: Color, stroke: Color) -> Primitive {
        Primitive::Rect(RectPrimitive {
            rect: self.screen_rect,
            fill: Some(Brush::Solid(fill)),
            stroke: Some(Stroke::new(1.0, Brush::Solid(stroke))),
            radius: CornerRadius::all(0.0),
        })
    }

    /// Builds backend-neutral semantic metadata for this safe area.
    #[must_use]
    pub fn semantics(&self, root: WidgetId) -> SemanticNode {
        let mut node = SemanticNode::new(
            viewport_safe_area_widget_id(root, self.id),
            SemanticRole::Custom("viewport-safe-area".to_owned()),
            self.screen_rect,
        )
        .with_label(
            self.label
                .clone()
                .unwrap_or_else(|| format!("Viewport safe area {}", self.id.raw())),
        );
        node.state.disabled = !self.enabled;
        node.state.value = Some(SemanticValue::Text(format!(
            "{:?} safe area {:.3}x{:.3}",
            self.space, self.screen_rect.width, self.screen_rect.height
        )));
        node
    }
}

/// Resolves viewport safe-area descriptors into finite UI logical screen-space metadata.
#[must_use]
pub fn viewport_safe_areas(
    surface: ViewportSurface,
    safe_areas: &[ViewportSafeAreaDescriptor],
) -> Vec<ViewportResolvedSafeArea> {
    viewport_safe_areas_at(surface, safe_areas, ScaleFactor::ONE)
}

/// Resolves viewport safe-area descriptors into finite UI logical screen-space metadata
/// for a viewport scale factor.
#[must_use]
pub fn viewport_safe_areas_at(
    surface: ViewportSurface,
    safe_areas: &[ViewportSafeAreaDescriptor],
    scale_factor: ScaleFactor,
) -> Vec<ViewportResolvedSafeArea> {
    let mut safe_areas = safe_areas
        .iter()
        .filter_map(|safe_area| {
            ViewportResolvedSafeArea::from_descriptor(safe_area, surface, scale_factor)
        })
        .collect::<Vec<_>>();
    safe_areas.sort_by(|left, right| {
        left.priority
            .cmp(&right.priority)
            .then_with(|| left.id.cmp(&right.id))
    });
    safe_areas
}

/// Viewport ruler overlay edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ViewportRulerEdge {
    /// Top horizontal ruler measuring content x units.
    Top,
    /// Left vertical ruler measuring content y units.
    Left,
}

/// Application-supplied data-only viewport ruler overlay descriptor.
#[derive(Debug, Clone, PartialEq)]
pub struct ViewportRulerDescriptor {
    /// Stable ruler identity.
    pub id: ViewportRulerId,
    /// Ruler edge.
    pub edge: ViewportRulerEdge,
    /// Ruler thickness in UI logical screen units.
    pub thickness: f32,
    /// Content-space origin value used for labels and origin metadata.
    pub origin_content: f32,
    /// Maximum number of ticks emitted by this ruler.
    pub max_ticks: usize,
    /// Optional accessible/debug label.
    pub label: Option<String>,
}

impl ViewportRulerDescriptor {
    /// Creates a viewport ruler descriptor.
    #[must_use]
    pub const fn new(id: ViewportRulerId, edge: ViewportRulerEdge) -> Self {
        Self {
            id,
            edge,
            thickness: 18.0,
            origin_content: 0.0,
            max_ticks: 128,
            label: None,
        }
    }

    /// Sets ruler thickness in UI logical screen units.
    #[must_use]
    pub const fn with_thickness(mut self, thickness: f32) -> Self {
        self.thickness = thickness;
        self
    }

    /// Sets the content-space origin value.
    #[must_use]
    pub const fn with_origin_content(mut self, origin_content: f32) -> Self {
        self.origin_content = origin_content;
        self
    }

    /// Sets the maximum number of emitted ticks.
    #[must_use]
    pub const fn with_max_ticks(mut self, max_ticks: usize) -> Self {
        self.max_ticks = max_ticks;
        self
    }

    /// Adds an accessible/debug label.
    #[must_use]
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }
}

/// Stable ruler tick metadata.
#[derive(Debug, Clone, PartialEq)]
pub struct ViewportRulerTick {
    /// Tick value in generic source content units.
    pub value: f32,
    /// Tick axis position in UI logical screen space.
    pub screen_position: f32,
    /// Whether this is a major tick with a visible label.
    pub major: bool,
    /// Optional generic content-unit label.
    pub label: Option<String>,
}

/// Viewport ruler overlay resolved into UI logical screen space.
#[derive(Debug, Clone, PartialEq)]
pub struct ViewportResolvedRuler {
    /// Stable ruler identity.
    pub id: ViewportRulerId,
    /// Ruler edge.
    pub edge: ViewportRulerEdge,
    /// Ruler rectangle in UI logical screen space.
    pub rect: Rect,
    /// Content-space visible range represented by this ruler.
    pub visible_content_range: (f32, f32),
    /// Content-space origin value.
    pub origin_content: f32,
    /// Origin axis position in UI logical screen space.
    pub origin_screen_position: f32,
    /// Deterministic finite ruler ticks.
    pub ticks: Vec<ViewportRulerTick>,
    /// Optional accessible/debug label.
    pub label: Option<String>,
}

impl ViewportResolvedRuler {
    fn from_descriptor(
        descriptor: &ViewportRulerDescriptor,
        surface: ViewportSurface,
        scale_factor: ScaleFactor,
    ) -> Option<Self> {
        let thickness = finite_positive(descriptor.thickness).unwrap_or(18.0);
        let bounds = surface.effective_bounds();
        let rect = match descriptor.edge {
            ViewportRulerEdge::Top => Rect::new(bounds.x, bounds.y, bounds.width, thickness),
            ViewportRulerEdge::Left => Rect::new(bounds.x, bounds.y, thickness, bounds.height),
        };
        let visible_content_range =
            visible_ruler_content_range(surface, descriptor.edge, scale_factor)?;
        let origin_content = finite_or_zero(descriptor.origin_content);
        let origin_screen_position =
            ruler_axis_screen_position(surface, descriptor.edge, origin_content, scale_factor)?;
        let max_ticks = descriptor.max_ticks.min(4096);
        let ticks = viewport_ruler_ticks(
            surface,
            descriptor.edge,
            visible_content_range,
            origin_content,
            max_ticks,
            scale_factor,
        );

        Some(Self {
            id: descriptor.id,
            edge: descriptor.edge,
            rect: finite_positive_rect(rect)?,
            visible_content_range,
            origin_content,
            origin_screen_position,
            ticks,
            label: descriptor.label.clone(),
        })
    }

    /// Builds backend-neutral primitive metadata for the ruler and its ticks.
    #[must_use]
    pub fn primitives(&self, background: Color, tick: Color, label: Color) -> Vec<Primitive> {
        let mut primitives = vec![Primitive::Rect(RectPrimitive {
            rect: self.rect,
            fill: Some(Brush::Solid(background)),
            stroke: Some(Stroke::new(1.0, Brush::Solid(tick))),
            radius: CornerRadius::all(0.0),
        })];
        for ruler_tick in &self.ticks {
            primitives.push(match self.edge {
                ViewportRulerEdge::Top => Primitive::Line(LinePrimitive {
                    from: Point::new(ruler_tick.screen_position, self.rect.max_y()),
                    to: Point::new(
                        ruler_tick.screen_position,
                        self.rect.max_y() - if ruler_tick.major { 8.0 } else { 4.0 },
                    ),
                    stroke: Stroke::new(1.0, Brush::Solid(tick)),
                }),
                ViewportRulerEdge::Left => Primitive::Line(LinePrimitive {
                    from: Point::new(self.rect.max_x(), ruler_tick.screen_position),
                    to: Point::new(
                        self.rect.max_x() - if ruler_tick.major { 8.0 } else { 4.0 },
                        ruler_tick.screen_position,
                    ),
                    stroke: Stroke::new(1.0, Brush::Solid(tick)),
                }),
            });
            if let Some(text) = &ruler_tick.label {
                primitives.push(Primitive::Text(TextPrimitive {
                    layout: None,
                    origin: match self.edge {
                        ViewportRulerEdge::Top => {
                            Point::new(ruler_tick.screen_position + 2.0, self.rect.y + 11.0)
                        }
                        ViewportRulerEdge::Left => {
                            Point::new(self.rect.x + 2.0, ruler_tick.screen_position - 2.0)
                        }
                    },
                    text: text.clone(),
                    family: "sans-serif".to_owned(),
                    size: 10.0,
                    line_height: 12.0,
                    brush: Brush::Solid(label),
                }));
            }
        }
        primitives
    }

    /// Builds backend-neutral semantic metadata for this ruler.
    #[must_use]
    pub fn semantics(&self, root: WidgetId) -> SemanticNode {
        let mut node = SemanticNode::new(
            viewport_ruler_widget_id(root, self.id),
            SemanticRole::Custom("viewport-ruler".to_owned()),
            self.rect,
        )
        .with_label(
            self.label
                .clone()
                .unwrap_or_else(|| format!("Viewport {:?} ruler", self.edge)),
        );
        node.state.value = Some(SemanticValue::Text(format!(
            "{:.3} to {:.3}, {} ticks",
            self.visible_content_range.0,
            self.visible_content_range.1,
            self.ticks.len()
        )));
        node
    }
}

/// Resolves viewport ruler descriptors into finite UI logical screen-space metadata.
#[must_use]
pub fn viewport_rulers(
    surface: ViewportSurface,
    rulers: &[ViewportRulerDescriptor],
) -> Vec<ViewportResolvedRuler> {
    viewport_rulers_at(surface, rulers, ScaleFactor::ONE)
}

/// Resolves viewport ruler descriptors into finite UI logical screen-space metadata
/// for a viewport scale factor.
#[must_use]
pub fn viewport_rulers_at(
    surface: ViewportSurface,
    rulers: &[ViewportRulerDescriptor],
    scale_factor: ScaleFactor,
) -> Vec<ViewportResolvedRuler> {
    let mut rulers = rulers
        .iter()
        .filter_map(|ruler| ViewportResolvedRuler::from_descriptor(ruler, surface, scale_factor))
        .collect::<Vec<_>>();
    rulers.sort_by(|left, right| {
        left.edge
            .cmp(&right.edge)
            .then_with(|| left.id.cmp(&right.id))
    });
    rulers
}
