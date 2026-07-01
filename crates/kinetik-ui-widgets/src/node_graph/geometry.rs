#[allow(clippy::wildcard_imports)]
use super::*;

/// A point in node graph content space.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct GraphPoint {
    /// Horizontal graph coordinate.
    pub x: f32,
    /// Vertical graph coordinate.
    pub y: f32,
}

impl GraphPoint {
    /// The graph origin.
    pub const ZERO: Self = Self::new(0.0, 0.0);

    /// Creates a graph point.
    #[must_use]
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    /// Returns this point translated by a graph vector.
    #[must_use]
    pub const fn translate(self, offset: GraphVector) -> Self {
        Self::new(self.x + offset.x, self.y + offset.y)
    }

    /// Returns a copy with non-finite coordinates replaced by zero.
    #[must_use]
    pub fn sanitized(self) -> Self {
        Self::new(finite_or_zero(self.x), finite_or_zero(self.y))
    }
}

/// A vector in node graph coordinate calculations.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct GraphVector {
    /// Horizontal component.
    pub x: f32,
    /// Vertical component.
    pub y: f32,
}

impl GraphVector {
    /// The zero vector.
    pub const ZERO: Self = Self::new(0.0, 0.0);

    /// Creates a graph vector.
    #[must_use]
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    /// Returns a copy with non-finite components replaced by zero.
    #[must_use]
    pub fn sanitized(self) -> Self {
        Self::new(finite_or_zero(self.x), finite_or_zero(self.y))
    }
}

/// An axis-aligned rectangle in node graph content space.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct GraphRect {
    /// Minimum x coordinate.
    pub x: f32,
    /// Minimum y coordinate.
    pub y: f32,
    /// Rectangle width in graph units.
    pub width: f32,
    /// Rectangle height in graph units.
    pub height: f32,
}

impl GraphRect {
    /// An empty graph-space rectangle at the origin.
    pub const ZERO: Self = Self::new(0.0, 0.0, 0.0, 0.0);

    /// Creates a graph-space rectangle.
    #[must_use]
    pub const fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Creates a graph-space rectangle from an origin and size vector.
    #[must_use]
    pub const fn from_origin_size(origin: GraphPoint, size: GraphVector) -> Self {
        Self::new(origin.x, origin.y, size.x, size.y)
    }

    /// Returns the rectangle origin.
    #[must_use]
    pub const fn origin(self) -> GraphPoint {
        GraphPoint::new(self.x, self.y)
    }

    /// Returns the rectangle size as a graph vector.
    #[must_use]
    pub const fn size(self) -> GraphVector {
        GraphVector::new(self.width, self.height)
    }

    /// Returns the maximum x coordinate.
    #[must_use]
    pub const fn max_x(self) -> f32 {
        self.x + self.width
    }

    /// Returns the maximum y coordinate.
    #[must_use]
    pub const fn max_y(self) -> f32 {
        self.y + self.height
    }

    /// Returns true when either dimension is zero or negative.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.width <= 0.0 || self.height <= 0.0
    }

    /// Returns a copy with non-finite coordinates replaced by zero and invalid
    /// dimensions clamped to zero.
    #[must_use]
    pub fn sanitized(self) -> Self {
        Self::new(
            finite_or_zero(self.x),
            finite_or_zero(self.y),
            finite_non_negative(self.width),
            finite_non_negative(self.height),
        )
    }

    /// Creates a graph-space rectangle from two corners.
    #[must_use]
    pub fn from_min_max(min: GraphPoint, max: GraphPoint) -> Self {
        let min = min.sanitized();
        let max = max.sanitized();
        let x = min.x.min(max.x);
        let y = min.y.min(max.y);
        Self::new(x, y, (max.x - min.x).abs(), (max.y - min.y).abs())
    }

    /// Returns true when this rectangle fully contains another rectangle.
    #[must_use]
    pub fn contains_rect(self, other: GraphRect) -> bool {
        let rect = self.sanitized();
        let other = other.sanitized();
        !rect.is_empty()
            && !other.is_empty()
            && other.x >= rect.x
            && other.y >= rect.y
            && other.max_x() <= rect.max_x()
            && other.max_y() <= rect.max_y()
    }

    /// Returns true when this rectangle overlaps another rectangle.
    #[must_use]
    pub fn intersects_rect(self, other: GraphRect) -> bool {
        let rect = self.sanitized();
        let other = other.sanitized();
        !rect.is_empty()
            && !other.is_empty()
            && rect.x < other.max_x()
            && rect.max_x() > other.x
            && rect.y < other.max_y()
            && rect.max_y() > other.y
    }
}

/// Pan and zoom state for a node graph viewport.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NodeGraphPanZoom {
    /// Screen-space pan offset in viewport-local logical units.
    pub pan: GraphVector,
    /// Screen units per graph unit.
    pub zoom: f32,
}

impl Default for NodeGraphPanZoom {
    fn default() -> Self {
        Self {
            pan: GraphVector::ZERO,
            zoom: DEFAULT_ZOOM,
        }
    }
}

impl NodeGraphPanZoom {
    /// Creates pan/zoom state.
    #[must_use]
    pub const fn new(pan: GraphVector, zoom: f32) -> Self {
        Self { pan, zoom }
    }

    /// Returns a copy with deterministic, valid numeric values.
    #[must_use]
    pub fn sanitized(self) -> Self {
        Self {
            pan: self.pan.sanitized(),
            zoom: sanitize_zoom(self.zoom),
        }
    }

    /// Sets custom zoom, falling back to the default for invalid values.
    pub fn set_zoom(&mut self, zoom: f32) {
        self.zoom = sanitize_zoom(zoom);
    }

    /// Adds a screen-space pan delta.
    pub fn pan_by(&mut self, delta: GraphVector) {
        let pan = self.pan.sanitized();
        let delta = delta.sanitized();
        self.pan = GraphVector::new(finite_sum(pan.x, delta.x), finite_sum(pan.y, delta.y));
    }
}

/// Node graph viewport bounds plus pan/zoom conversion state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NodeGraphViewport {
    /// Viewport bounds in UI logical screen coordinates.
    pub bounds: Rect,
    /// Pan and zoom state.
    pub pan_zoom: NodeGraphPanZoom,
}

impl NodeGraphViewport {
    /// Creates a node graph viewport.
    #[must_use]
    pub const fn new(bounds: Rect, pan_zoom: NodeGraphPanZoom) -> Self {
        Self { bounds, pan_zoom }
    }

    /// Returns sanitized viewport bounds.
    #[must_use]
    pub fn effective_bounds(self) -> Rect {
        sanitize_rect(self.bounds)
    }

    /// Returns sanitized pan/zoom state.
    #[must_use]
    pub fn effective_pan_zoom(self) -> NodeGraphPanZoom {
        self.pan_zoom.sanitized()
    }

    /// Converts a graph-space point to UI logical screen coordinates.
    #[must_use]
    pub fn graph_to_screen(self, point: GraphPoint) -> Point {
        let point = point.sanitized();
        let bounds = self.effective_bounds();
        let pan_zoom = self.effective_pan_zoom();
        Point::new(
            finite_sum(
                finite_sum(bounds.x, pan_zoom.pan.x),
                finite_product(point.x, pan_zoom.zoom),
            ),
            finite_sum(
                finite_sum(bounds.y, pan_zoom.pan.y),
                finite_product(point.y, pan_zoom.zoom),
            ),
        )
    }

    /// Converts a UI logical screen point to graph-space coordinates.
    #[must_use]
    pub fn screen_to_graph(self, point: Point) -> GraphPoint {
        let point = sanitize_point(point);
        let bounds = self.effective_bounds();
        let pan_zoom = self.effective_pan_zoom();
        GraphPoint::new(
            finite_div(
                finite_sum(finite_sum(point.x, -bounds.x), -pan_zoom.pan.x),
                pan_zoom.zoom,
            ),
            finite_div(
                finite_sum(finite_sum(point.y, -bounds.y), -pan_zoom.pan.y),
                pan_zoom.zoom,
            ),
        )
    }

    /// Converts a UI logical screen-space delta to graph-space units.
    #[must_use]
    pub fn screen_delta_to_graph(self, delta: GraphVector) -> GraphVector {
        let delta = delta.sanitized();
        let zoom = self.effective_pan_zoom().zoom;
        GraphVector::new(finite_div(delta.x, zoom), finite_div(delta.y, zoom))
    }

    /// Converts a graph-space rectangle to UI logical screen coordinates.
    #[must_use]
    pub fn graph_rect_to_screen(self, rect: GraphRect) -> Rect {
        let rect = rect.sanitized();
        let origin = self.graph_to_screen(rect.origin());
        let zoom = self.effective_pan_zoom().zoom;
        Rect::new(
            origin.x,
            origin.y,
            finite_product(rect.width, zoom).max(0.0),
            finite_product(rect.height, zoom).max(0.0),
        )
    }

    /// Converts a UI logical screen rectangle to graph-space coordinates.
    #[must_use]
    pub fn screen_rect_to_graph(self, rect: Rect) -> GraphRect {
        let rect = sanitize_rect(rect);
        let origin = self.screen_to_graph(rect.origin());
        let zoom = self.effective_pan_zoom().zoom;
        GraphRect::new(
            origin.x,
            origin.y,
            finite_div(rect.width, zoom).max(0.0),
            finite_div(rect.height, zoom).max(0.0),
        )
    }
}
