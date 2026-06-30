//! Viewport texture surfaces and editor overlay primitives.

use kinetik_ui_core::{
    Brush, ClipId, Color, CornerRadius, LinePrimitive, Point, Primitive, Rect, RectPrimitive,
    ScaleFactor, SemanticNode, SemanticRole, SemanticValue, Size, Stroke, TextPrimitive, TextureId,
    TexturePrimitive, Vec2, WidgetId,
};

/// How viewport content should fit inside its bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewportFit {
    /// Preserve aspect ratio and fit entire content.
    Fit,
    /// Preserve aspect ratio and fill the viewport bounds.
    Fill,
    /// Preserve source pixel size in logical units.
    ActualSize,
    /// Use a custom zoom factor.
    Zoom,
}

/// Pan and zoom state for viewport content.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PanZoom {
    /// Current fit mode.
    pub fit: ViewportFit,
    /// Custom zoom factor.
    pub zoom: f32,
    /// Pan offset in logical units.
    pub pan: Vec2,
}

impl Default for PanZoom {
    fn default() -> Self {
        Self {
            fit: ViewportFit::Fit,
            zoom: 1.0,
            pan: Vec2::ZERO,
        }
    }
}

impl PanZoom {
    /// Sets fit mode.
    pub fn fit(&mut self) {
        self.fit = ViewportFit::Fit;
    }

    /// Sets fill mode.
    pub fn fill(&mut self) {
        self.fit = ViewportFit::Fill;
    }

    /// Sets 100% mode.
    pub fn actual_size(&mut self) {
        self.fit = ViewportFit::ActualSize;
        self.zoom = 1.0;
    }

    /// Sets custom zoom.
    pub fn set_zoom(&mut self, zoom: f32) {
        self.fit = ViewportFit::Zoom;
        self.zoom = finite_positive(zoom).unwrap_or(1.0).max(0.01);
    }

    /// Adds a pan delta.
    pub fn pan_by(&mut self, delta: Vec2) {
        self.pan = Vec2::new(
            finite_or_zero(self.pan.x) + finite_or_zero(delta.x),
            finite_or_zero(self.pan.y) + finite_or_zero(delta.y),
        );
    }
}

/// UI-managed viewport surface backed by an application-owned texture.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewportSurface {
    /// Texture to display.
    pub texture: TextureId,
    /// Source content size.
    pub source_size: Size,
    /// Viewport bounds.
    pub bounds: Rect,
    /// Pan and zoom state.
    pub pan_zoom: PanZoom,
}

impl ViewportSurface {
    /// Returns sanitized viewport bounds.
    #[must_use]
    pub fn effective_bounds(self) -> Rect {
        Rect::new(
            finite_or_zero(self.bounds.x),
            finite_or_zero(self.bounds.y),
            finite_non_negative(self.bounds.width),
            finite_non_negative(self.bounds.height),
        )
    }

    /// Returns sanitized source size, or `None` when content cannot be displayed.
    #[must_use]
    pub fn effective_source_size(self) -> Option<Size> {
        Some(Size::new(
            finite_positive(self.source_size.width)?,
            finite_positive(self.source_size.height)?,
        ))
    }

    /// Computes the effective content-to-screen scale.
    #[must_use]
    pub fn content_scale(self) -> f32 {
        self.content_scale_at(ScaleFactor::ONE)
    }

    /// Computes the effective content-to-screen scale for a viewport scale factor.
    #[must_use]
    pub fn content_scale_at(self, scale_factor: ScaleFactor) -> f32 {
        let Some(source) = self.effective_source_size() else {
            return 0.0;
        };
        let bounds = self.effective_bounds().size();
        let native_scale = native_logical_pixel_scale(scale_factor);
        match self.pan_zoom.fit {
            ViewportFit::Fit => fit_scale(source, bounds),
            ViewportFit::Fill => fill_scale(source, bounds),
            ViewportFit::ActualSize => native_scale,
            ViewportFit::Zoom => {
                finite_positive(self.pan_zoom.zoom).unwrap_or(1.0).max(0.01) * native_scale
            }
        }
    }

    /// Computes the destination rectangle for the texture.
    #[must_use]
    pub fn content_rect(self) -> Rect {
        self.content_rect_at(ScaleFactor::ONE)
    }

    /// Computes the scale-aware destination rectangle for the texture.
    #[must_use]
    pub fn content_rect_at(self, scale_factor: ScaleFactor) -> Rect {
        let bounds = self.effective_bounds();
        let Some(source) = self.effective_source_size() else {
            return Rect::new(bounds.x, bounds.y, 0.0, 0.0);
        };
        let scale = self.content_scale_at(scale_factor);
        let width = source.width * scale;
        let height = source.height * scale;
        snap_rect_to_scale(
            Rect::new(
                bounds.x + (bounds.width - width) * 0.5 + finite_or_zero(self.pan_zoom.pan.x),
                bounds.y + (bounds.height - height) * 0.5 + finite_or_zero(self.pan_zoom.pan.y),
                width,
                height,
            ),
            scale_factor,
        )
    }

    /// Converts a UI-space point to viewport-local coordinates.
    #[must_use]
    pub fn screen_to_viewport(self, point: Point) -> Option<Point> {
        finite_point(point).map(|point| {
            let bounds = self.effective_bounds();
            Point::new(point.x - bounds.x, point.y - bounds.y)
        })
    }

    /// Converts viewport-local coordinates to UI-space.
    #[must_use]
    pub fn viewport_to_screen(self, point: Point) -> Option<Point> {
        finite_point(point).map(|point| {
            let bounds = self.effective_bounds();
            Point::new(bounds.x + point.x, bounds.y + point.y)
        })
    }

    /// Converts a UI-space point to content coordinates.
    #[must_use]
    pub fn screen_to_content(self, point: Point) -> Option<Point> {
        self.screen_to_content_at(point, ScaleFactor::ONE)
    }

    /// Converts a UI-space point to content coordinates for a viewport scale factor.
    #[must_use]
    pub fn screen_to_content_at(self, point: Point, scale_factor: ScaleFactor) -> Option<Point> {
        let point = finite_point(point)?;
        let scale = finite_positive(self.content_scale_at(scale_factor))?;
        let rect = self.content_rect_at(scale_factor);
        Some(Point::new(
            (point.x - rect.x) / scale,
            (point.y - rect.y) / scale,
        ))
    }

    /// Converts viewport-local coordinates to content coordinates.
    #[must_use]
    pub fn viewport_to_content(self, point: Point) -> Option<Point> {
        self.viewport_to_screen(point)
            .and_then(|point| self.screen_to_content(point))
    }

    /// Converts viewport-local coordinates to content coordinates for a viewport scale factor.
    #[must_use]
    pub fn viewport_to_content_at(self, point: Point, scale_factor: ScaleFactor) -> Option<Point> {
        self.viewport_to_screen(point)
            .and_then(|point| self.screen_to_content_at(point, scale_factor))
    }

    /// Converts a content-space point to UI-space.
    #[must_use]
    pub fn content_to_screen(self, point: Point) -> Option<Point> {
        self.content_to_screen_at(point, ScaleFactor::ONE)
    }

    /// Converts a content-space point to UI-space for a viewport scale factor.
    #[must_use]
    pub fn content_to_screen_at(self, point: Point, scale_factor: ScaleFactor) -> Option<Point> {
        let point = finite_point(point)?;
        let scale = finite_positive(self.content_scale_at(scale_factor))?;
        let rect = self.content_rect_at(scale_factor);
        Some(Point::new(
            rect.x + point.x * scale,
            rect.y + point.y * scale,
        ))
    }

    /// Converts a content-space rectangle to UI-space.
    #[must_use]
    pub fn content_rect_to_screen(self, rect: Rect) -> Option<Rect> {
        self.content_rect_to_screen_at(rect, ScaleFactor::ONE)
    }

    /// Converts a content-space rectangle to UI-space for a viewport scale factor.
    #[must_use]
    pub fn content_rect_to_screen_at(self, rect: Rect, scale_factor: ScaleFactor) -> Option<Rect> {
        let scale = finite_positive(self.content_scale_at(scale_factor))?;
        let origin = self.content_to_screen_at(rect.origin(), scale_factor)?;
        Some(snap_rect_to_scale(
            Rect::new(
                origin.x,
                origin.y,
                finite_non_negative(rect.width) * scale,
                finite_non_negative(rect.height) * scale,
            ),
            scale_factor,
        ))
    }

    /// Converts a UI-space rectangle to content-space.
    #[must_use]
    pub fn screen_rect_to_content(self, rect: Rect) -> Option<Rect> {
        self.screen_rect_to_content_at(rect, ScaleFactor::ONE)
    }

    /// Converts a UI-space rectangle to content-space for a viewport scale factor.
    #[must_use]
    pub fn screen_rect_to_content_at(self, rect: Rect, scale_factor: ScaleFactor) -> Option<Rect> {
        let scale = finite_positive(self.content_scale_at(scale_factor))?;
        let origin = self.screen_to_content_at(rect.origin(), scale_factor)?;
        Some(Rect::new(
            origin.x,
            origin.y,
            finite_non_negative(rect.width) / scale,
            finite_non_negative(rect.height) / scale,
        ))
    }

    /// Returns true when a UI-space point is inside the viewport bounds.
    #[must_use]
    pub fn contains_screen_point(self, point: Point) -> bool {
        finite_point(point).is_some_and(|point| self.effective_bounds().contains_point(point))
    }

    /// Returns true when a content-space point is inside the source content.
    #[must_use]
    pub fn contains_content_point(self, point: Point) -> bool {
        let Some(point) = finite_point(point) else {
            return false;
        };
        let Some(source) = self.effective_source_size() else {
            return false;
        };
        Rect::new(0.0, 0.0, source.width, source.height).contains_point(point)
    }

    /// Emits the texture primitive.
    #[must_use]
    pub fn texture_primitive(self) -> Primitive {
        self.texture_primitive_at(ScaleFactor::ONE)
    }

    /// Emits the texture primitive for a viewport scale factor.
    #[must_use]
    pub fn texture_primitive_at(self, scale_factor: ScaleFactor) -> Primitive {
        let source_size = self.effective_source_size().unwrap_or(Size::ZERO);
        Primitive::Texture(TexturePrimitive {
            texture: self.texture,
            rect: self.content_rect_at(scale_factor),
            source_size,
        })
    }

    /// Emits guide line primitives for content-space guide positions.
    #[must_use]
    pub fn content_guide_primitives(self, guides: &[Guide], color: Color) -> Vec<Primitive> {
        self.content_guide_primitives_at(guides, color, ScaleFactor::ONE)
    }

    /// Emits guide line primitives for content-space guide positions at a viewport scale factor.
    #[must_use]
    pub fn content_guide_primitives_at(
        self,
        guides: &[Guide],
        color: Color,
        scale_factor: ScaleFactor,
    ) -> Vec<Primitive> {
        let content_rect = self.content_rect_at(scale_factor);
        guides
            .iter()
            .filter_map(|guide| match *guide {
                Guide::Horizontal(y) => {
                    let from = self.content_to_screen_at(Point::new(0.0, y), scale_factor)?;
                    Some(Primitive::Line(LinePrimitive {
                        from: Point::new(content_rect.x, from.y),
                        to: Point::new(content_rect.max_x(), from.y),
                        stroke: Stroke::new(1.0, Brush::Solid(color)),
                    }))
                }
                Guide::Vertical(x) => {
                    let from = self.content_to_screen_at(Point::new(x, 0.0), scale_factor)?;
                    Some(Primitive::Line(LinePrimitive {
                        from: Point::new(from.x, content_rect.y),
                        to: Point::new(from.x, content_rect.max_y()),
                        stroke: Stroke::new(1.0, Brush::Solid(color)),
                    }))
                }
            })
            .collect()
    }

    /// Emits a content-space crosshair overlay.
    #[must_use]
    pub fn content_crosshair_primitives(self, crosshair: &Crosshair) -> Vec<Primitive> {
        self.content_crosshair_primitives_at(crosshair, ScaleFactor::ONE)
    }

    /// Emits a content-space crosshair overlay for a viewport scale factor.
    #[must_use]
    pub fn content_crosshair_primitives_at(
        self,
        crosshair: &Crosshair,
        scale_factor: ScaleFactor,
    ) -> Vec<Primitive> {
        if !crosshair.visible || !self.contains_content_point(crosshair.position) {
            return Vec::new();
        }
        let Some(position) = self.content_to_screen_at(crosshair.position, scale_factor) else {
            return Vec::new();
        };
        if !self.contains_screen_point(position) {
            return Vec::new();
        }
        crosshair
            .with_position(position)
            .primitives(self.effective_bounds())
    }
}

/// Stable identity for a viewport tool declared by the application.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ViewportToolId(u64);

impl ViewportToolId {
    /// Creates a viewport tool ID from raw bits.
    #[must_use]
    pub const fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    /// Returns the raw ID bits.
    #[must_use]
    pub const fn raw(self) -> u64 {
        self.0
    }
}

/// Stable identity for a viewport overlay target declared by the application.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ViewportOverlayId(u64);

impl ViewportOverlayId {
    /// Creates a viewport overlay ID from raw bits.
    #[must_use]
    pub const fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    /// Returns the raw ID bits.
    #[must_use]
    pub const fn raw(self) -> u64 {
        self.0
    }
}

/// Stable identity for a viewport guide supplied by the application.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ViewportGuideId(u64);

impl ViewportGuideId {
    /// Creates a viewport guide ID from raw bits.
    #[must_use]
    pub const fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    /// Returns the raw ID bits.
    #[must_use]
    pub const fn raw(self) -> u64 {
        self.0
    }
}

/// Stable identity for a viewport safe area supplied by the application.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ViewportSafeAreaId(u64);

impl ViewportSafeAreaId {
    /// Creates a viewport safe-area ID from raw bits.
    #[must_use]
    pub const fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    /// Returns the raw ID bits.
    #[must_use]
    pub const fn raw(self) -> u64 {
        self.0
    }
}

/// Stable identity for a viewport ruler overlay supplied by the application.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ViewportRulerId(u64);

impl ViewportRulerId {
    /// Creates a viewport ruler ID from raw bits.
    #[must_use]
    pub const fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    /// Returns the raw ID bits.
    #[must_use]
    pub const fn raw(self) -> u64 {
        self.0
    }
}

/// Backend-neutral cursor shape requested by viewport tools or overlay targets.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ViewportCursorShape {
    /// Default platform cursor.
    Default,
    /// Pointer/action cursor.
    Pointer,
    /// Crosshair cursor.
    Crosshair,
    /// Open-hand grab cursor.
    Grab,
    /// Closed-hand grabbing cursor.
    Grabbing,
    /// Text edit cursor.
    Text,
    /// Move cursor.
    Move,
    /// Horizontal resize cursor.
    ResizeHorizontal,
    /// Vertical resize cursor.
    ResizeVertical,
    /// Top-left to bottom-right diagonal resize cursor.
    ResizeTopLeftBottomRight,
    /// Top-right to bottom-left diagonal resize cursor.
    ResizeTopRightBottomLeft,
    /// Rotate cursor.
    Rotate,
    /// Application-defined cursor token interpreted outside the toolkit.
    Custom(String),
}

/// Data-only cursor request metadata.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ViewportCursorMetadata {
    /// Requested cursor shape.
    pub shape: ViewportCursorShape,
    /// Optional accessible/debug label for the cursor request.
    pub label: Option<String>,
}

impl ViewportCursorMetadata {
    /// Creates cursor metadata with no label.
    #[must_use]
    pub fn new(shape: ViewportCursorShape) -> Self {
        Self { shape, label: None }
    }

    /// Adds a cursor request label.
    #[must_use]
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }
}

/// Stable identity for a viewport selection target supplied by the application.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ViewportSelectionTargetId(u64);

impl ViewportSelectionTargetId {
    /// Creates a viewport selection target ID from raw bits.
    #[must_use]
    pub const fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    /// Returns the raw ID bits.
    #[must_use]
    pub const fn raw(self) -> u64 {
        self.0
    }
}

/// Generic 2D transform handle kind for viewport selection targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ViewportTransformHandleKind {
    /// Move the selected target.
    Move,
    /// Resize from the top-left corner.
    ResizeTopLeft,
    /// Resize from the top edge.
    ResizeTop,
    /// Resize from the top-right corner.
    ResizeTopRight,
    /// Resize from the right edge.
    ResizeRight,
    /// Resize from the bottom-right corner.
    ResizeBottomRight,
    /// Resize from the bottom edge.
    ResizeBottom,
    /// Resize from the bottom-left corner.
    ResizeBottomLeft,
    /// Resize from the left edge.
    ResizeLeft,
    /// Rotate around the target's pivot.
    Rotate,
    /// Move the target pivot or anchor.
    Pivot,
}

impl ViewportTransformHandleKind {
    /// Returns the backend-neutral cursor shape normally associated with this handle.
    #[must_use]
    pub fn cursor_shape(self) -> ViewportCursorShape {
        match self {
            Self::Move => ViewportCursorShape::Move,
            Self::ResizeTopLeft | Self::ResizeBottomRight => {
                ViewportCursorShape::ResizeTopLeftBottomRight
            }
            Self::ResizeTop | Self::ResizeBottom => ViewportCursorShape::ResizeVertical,
            Self::ResizeTopRight | Self::ResizeBottomLeft => {
                ViewportCursorShape::ResizeTopRightBottomLeft
            }
            Self::ResizeRight | Self::ResizeLeft => ViewportCursorShape::ResizeHorizontal,
            Self::Rotate => ViewportCursorShape::Rotate,
            Self::Pivot => ViewportCursorShape::Crosshair,
        }
    }

    const fn hit_priority(self) -> i32 {
        match self {
            Self::ResizeTopLeft
            | Self::ResizeTopRight
            | Self::ResizeBottomRight
            | Self::ResizeBottomLeft => 90,
            Self::Rotate => 80,
            Self::Pivot => 70,
            Self::ResizeTop | Self::ResizeRight | Self::ResizeBottom | Self::ResizeLeft => 60,
            Self::Move => 10,
        }
    }
}

/// Stable identity for a transform handle on an application-supplied selection target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ViewportTransformHandleId {
    /// Stable selection target identity.
    pub target: ViewportSelectionTargetId,
    /// Stable handle kind on the target.
    pub kind: ViewportTransformHandleKind,
}

impl ViewportTransformHandleId {
    /// Creates a stable transform handle identity.
    #[must_use]
    pub const fn new(target: ViewportSelectionTargetId, kind: ViewportTransformHandleKind) -> Self {
        Self { target, kind }
    }
}

/// Set of generic 2D transform handles exposed for a selection target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ViewportTransformHandleSet {
    bits: u16,
}

impl ViewportTransformHandleSet {
    const MOVE: u16 = 1 << 0;
    const RESIZE_EDGES: u16 = 1 << 1;
    const RESIZE_CORNERS: u16 = 1 << 2;
    const ROTATE: u16 = 1 << 3;
    const PIVOT: u16 = 1 << 4;

    /// Creates a handle set containing every generic 2D handle.
    #[must_use]
    pub const fn all_2d() -> Self {
        Self {
            bits: Self::MOVE
                | Self::RESIZE_EDGES
                | Self::RESIZE_CORNERS
                | Self::ROTATE
                | Self::PIVOT,
        }
    }

    /// Creates a handle set containing only the move handle.
    #[must_use]
    pub const fn move_only() -> Self {
        Self { bits: Self::MOVE }
    }

    /// Returns true when this set contains the requested handle kind.
    #[must_use]
    pub const fn contains(self, kind: ViewportTransformHandleKind) -> bool {
        let bit = match kind {
            ViewportTransformHandleKind::Move => Self::MOVE,
            ViewportTransformHandleKind::ResizeTop
            | ViewportTransformHandleKind::ResizeRight
            | ViewportTransformHandleKind::ResizeBottom
            | ViewportTransformHandleKind::ResizeLeft => Self::RESIZE_EDGES,
            ViewportTransformHandleKind::ResizeTopLeft
            | ViewportTransformHandleKind::ResizeTopRight
            | ViewportTransformHandleKind::ResizeBottomRight
            | ViewportTransformHandleKind::ResizeBottomLeft => Self::RESIZE_CORNERS,
            ViewportTransformHandleKind::Rotate => Self::ROTATE,
            ViewportTransformHandleKind::Pivot => Self::PIVOT,
        };
        self.bits & bit != 0
    }

    fn kinds(self) -> impl Iterator<Item = ViewportTransformHandleKind> {
        [
            ViewportTransformHandleKind::Move,
            ViewportTransformHandleKind::ResizeTopLeft,
            ViewportTransformHandleKind::ResizeTop,
            ViewportTransformHandleKind::ResizeTopRight,
            ViewportTransformHandleKind::ResizeRight,
            ViewportTransformHandleKind::ResizeBottomRight,
            ViewportTransformHandleKind::ResizeBottom,
            ViewportTransformHandleKind::ResizeBottomLeft,
            ViewportTransformHandleKind::ResizeLeft,
            ViewportTransformHandleKind::Rotate,
            ViewportTransformHandleKind::Pivot,
        ]
        .into_iter()
        .filter(move |kind| self.contains(*kind))
    }
}

impl Default for ViewportTransformHandleSet {
    fn default() -> Self {
        Self::all_2d()
    }
}

/// Compact selection and interactivity state for a viewport selection target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ViewportSelectionTargetState {
    bits: u8,
}

impl ViewportSelectionTargetState {
    const SELECTED: u8 = 1 << 0;
    const ENABLED: u8 = 1 << 1;
    const AVAILABLE: u8 = 1 << 2;
    const READ_ONLY: u8 = 1 << 3;

    /// Creates selected, enabled, available target state.
    #[must_use]
    pub const fn interactive_selected() -> Self {
        Self {
            bits: Self::SELECTED | Self::ENABLED | Self::AVAILABLE,
        }
    }

    /// Returns true when the target is part of the current selection.
    #[must_use]
    pub const fn selected(self) -> bool {
        self.bits & Self::SELECTED != 0
    }

    /// Returns true when the target can currently emit interaction requests.
    #[must_use]
    pub const fn enabled(self) -> bool {
        self.bits & Self::ENABLED != 0
    }

    /// Returns true when the target exists in the current application context.
    #[must_use]
    pub const fn available(self) -> bool {
        self.bits & Self::AVAILABLE != 0
    }

    /// Returns true when transform requests should be suppressed.
    #[must_use]
    pub const fn read_only(self) -> bool {
        self.bits & Self::READ_ONLY != 0
    }

    /// Returns state with the selected flag changed.
    #[must_use]
    pub const fn with_selected(mut self, selected: bool) -> Self {
        self.set_flag(Self::SELECTED, selected);
        self
    }

    /// Returns state with the enabled flag changed.
    #[must_use]
    pub const fn with_enabled(mut self, enabled: bool) -> Self {
        self.set_flag(Self::ENABLED, enabled);
        self
    }

    /// Returns state with the available flag changed.
    #[must_use]
    pub const fn with_available(mut self, available: bool) -> Self {
        self.set_flag(Self::AVAILABLE, available);
        self
    }

    /// Returns state with the read-only flag changed.
    #[must_use]
    pub const fn with_read_only(mut self, read_only: bool) -> Self {
        self.set_flag(Self::READ_ONLY, read_only);
        self
    }

    const fn set_flag(&mut self, flag: u8, enabled: bool) {
        if enabled {
            self.bits |= flag;
        } else {
            self.bits &= !flag;
        }
    }
}

impl Default for ViewportSelectionTargetState {
    fn default() -> Self {
        Self::interactive_selected()
    }
}

/// Data-only viewport tool descriptor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewportToolDescriptor {
    /// Stable tool identity.
    pub id: ViewportToolId,
    /// Tool label.
    pub label: String,
    /// Tool cursor request metadata.
    pub cursor: Option<ViewportCursorMetadata>,
    /// Whether this tool is currently active.
    pub active: bool,
    /// Whether this tool can currently emit interaction requests.
    pub enabled: bool,
    /// Whether this tool is available in the current app context.
    pub available: bool,
}

impl ViewportToolDescriptor {
    /// Creates an available, enabled viewport tool descriptor.
    #[must_use]
    pub fn new(id: ViewportToolId, label: impl Into<String>) -> Self {
        Self {
            id,
            label: label.into(),
            cursor: None,
            active: false,
            enabled: true,
            available: true,
        }
    }

    /// Marks the tool as active.
    #[must_use]
    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    /// Marks the tool as enabled or disabled.
    #[must_use]
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Marks the tool as available or unavailable in the current app context.
    #[must_use]
    pub fn available(mut self, available: bool) -> Self {
        self.available = available;
        self
    }

    /// Adds cursor request metadata.
    #[must_use]
    pub fn with_cursor(mut self, cursor: ViewportCursorMetadata) -> Self {
        self.cursor = Some(cursor);
        self
    }

    /// Returns true when the tool may participate in interaction routing.
    #[must_use]
    pub const fn can_interact(&self) -> bool {
        self.active && self.enabled && self.available
    }

    /// Returns cursor request metadata when this active tool can interact.
    #[must_use]
    pub fn cursor_request(&self) -> Option<&ViewportCursorMetadata> {
        self.can_interact().then_some(())?;
        self.cursor.as_ref()
    }
}

/// Semantic descriptor for a viewport tool surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewportToolSurfaceDescriptor {
    /// Stable viewport widget identity.
    pub id: WidgetId,
    /// Accessible viewport label.
    pub label: String,
    /// Optional active tool metadata.
    pub active_tool: Option<ViewportToolDescriptor>,
}

impl ViewportToolSurfaceDescriptor {
    /// Creates a semantic descriptor for a viewport tool surface.
    #[must_use]
    pub fn new(id: WidgetId, label: impl Into<String>) -> Self {
        Self {
            id,
            label: label.into(),
            active_tool: None,
        }
    }

    /// Adds active tool metadata without executing tool behavior.
    #[must_use]
    pub fn with_active_tool(mut self, tool: ViewportToolDescriptor) -> Self {
        self.active_tool = Some(tool);
        self
    }

    /// Builds backend-neutral semantic metadata for the viewport surface.
    #[must_use]
    pub fn semantics(&self, surface: ViewportSurface) -> SemanticNode {
        let mut node =
            SemanticNode::new(self.id, SemanticRole::Viewport, surface.effective_bounds())
                .with_label(self.label.clone())
                .focusable(true);
        if let Some(tool) = &self.active_tool {
            node.children
                .push(viewport_tool_widget_id(self.id, tool.id));
            node.state.disabled = !tool.enabled || !tool.available;
            node.state.value = Some(SemanticValue::Text(format!(
                "Active tool {}: {}",
                tool.id.raw(),
                tool.label
            )));
        }
        node
    }

    /// Builds backend-neutral semantic metadata for the active tool, when present.
    #[must_use]
    pub fn active_tool_semantics(&self, surface: ViewportSurface) -> Option<SemanticNode> {
        let tool = self.active_tool.as_ref()?;
        Some(viewport_tool_semantics(self.id, surface, tool))
    }
}

/// Returns the stable semantic widget ID for a viewport tool.
#[must_use]
pub fn viewport_tool_widget_id(root: WidgetId, tool: ViewportToolId) -> WidgetId {
    root.child(("viewport-tool", tool.raw()))
}

/// Returns the stable semantic widget ID for a viewport overlay target.
#[must_use]
pub fn viewport_overlay_widget_id(root: WidgetId, overlay: ViewportOverlayId) -> WidgetId {
    root.child(("viewport-overlay", overlay.raw()))
}

/// Returns the stable semantic widget ID for a viewport guide.
#[must_use]
pub fn viewport_guide_widget_id(root: WidgetId, guide: ViewportGuideId) -> WidgetId {
    root.child(("viewport-guide", guide.raw()))
}

/// Returns the stable semantic widget ID for a viewport safe area.
#[must_use]
pub fn viewport_safe_area_widget_id(root: WidgetId, safe_area: ViewportSafeAreaId) -> WidgetId {
    root.child(("viewport-safe-area", safe_area.raw()))
}

/// Returns the stable semantic widget ID for a viewport ruler.
#[must_use]
pub fn viewport_ruler_widget_id(root: WidgetId, ruler: ViewportRulerId) -> WidgetId {
    root.child(("viewport-ruler", ruler.raw()))
}

/// Builds backend-neutral semantic metadata for a viewport tool.
#[must_use]
pub fn viewport_tool_semantics(
    root: WidgetId,
    surface: ViewportSurface,
    tool: &ViewportToolDescriptor,
) -> SemanticNode {
    let mut node = SemanticNode::new(
        viewport_tool_widget_id(root, tool.id),
        SemanticRole::Custom("viewport-tool".to_owned()),
        surface.effective_bounds(),
    )
    .with_label(tool.label.clone())
    .focusable(tool.enabled && tool.available);
    node.state.selected = tool.active;
    node.state.disabled = !tool.enabled || !tool.available;
    node.state.value = Some(SemanticValue::Text(format!(
        "Tool {}: {}",
        tool.id.raw(),
        tool.label
    )));
    node
}

/// Application-supplied data-only selection target descriptor.
#[derive(Debug, Clone, PartialEq)]
pub struct ViewportSelectionTargetDescriptor {
    /// Stable target identity supplied by the application.
    pub id: ViewportSelectionTargetId,
    /// Target bounds in viewport content coordinates.
    pub content_rect: Rect,
    /// Selection and interactivity state.
    pub state: ViewportSelectionTargetState,
    /// Explicit target priority. Higher priority is treated as visually topmost.
    pub priority: i32,
    /// Generic 2D transform handles exposed for this target.
    pub handles: ViewportTransformHandleSet,
    /// Logical handle size in screen-space units.
    pub handle_size: f32,
    /// Logical distance from the top edge to the rotate handle center.
    pub rotate_offset: f32,
    /// Optional accessible/debug label.
    pub label: Option<String>,
}

impl ViewportSelectionTargetDescriptor {
    /// Creates a selected, enabled, available target descriptor with all generic 2D handles.
    #[must_use]
    pub fn new(id: ViewportSelectionTargetId, content_rect: Rect) -> Self {
        Self {
            id,
            content_rect,
            state: ViewportSelectionTargetState::interactive_selected(),
            priority: 0,
            handles: ViewportTransformHandleSet::all_2d(),
            handle_size: 9.0,
            rotate_offset: 20.0,
            label: None,
        }
    }

    /// Marks the target as selected or unselected.
    #[must_use]
    pub const fn selected(mut self, selected: bool) -> Self {
        self.state = self.state.with_selected(selected);
        self
    }

    /// Marks the target as enabled or disabled.
    #[must_use]
    pub const fn enabled(mut self, enabled: bool) -> Self {
        self.state = self.state.with_enabled(enabled);
        self
    }

    /// Marks the target as available or unavailable in the current app context.
    #[must_use]
    pub const fn available(mut self, available: bool) -> Self {
        self.state = self.state.with_available(available);
        self
    }

    /// Marks the target as read-only.
    #[must_use]
    pub const fn read_only(mut self, read_only: bool) -> Self {
        self.state = self.state.with_read_only(read_only);
        self
    }

    /// Sets explicit hit-test priority. Higher priority is topmost.
    #[must_use]
    pub const fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    /// Sets the generic transform handles exposed for this target.
    #[must_use]
    pub const fn with_handles(mut self, handles: ViewportTransformHandleSet) -> Self {
        self.handles = handles;
        self
    }

    /// Sets logical transform handle size in screen-space units.
    #[must_use]
    pub const fn with_handle_size(mut self, handle_size: f32) -> Self {
        self.handle_size = handle_size;
        self
    }

    /// Sets logical rotate handle offset from the target top edge.
    #[must_use]
    pub const fn with_rotate_offset(mut self, rotate_offset: f32) -> Self {
        self.rotate_offset = rotate_offset;
        self
    }

    /// Adds an accessible/debug label.
    #[must_use]
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Returns true when the target can expose selection outline metadata.
    #[must_use]
    pub const fn can_show_selection(&self) -> bool {
        self.state.selected() && self.state.available()
    }

    /// Returns true when this target can emit transform handle requests.
    #[must_use]
    pub const fn can_request_transform(&self) -> bool {
        self.state.selected()
            && self.state.enabled()
            && self.state.available()
            && !self.state.read_only()
    }

    /// Returns the target bounds transformed into UI logical screen space.
    #[must_use]
    pub fn screen_rect(&self, surface: ViewportSurface, scale_factor: ScaleFactor) -> Option<Rect> {
        finite_positive_rect(self.content_rect)
            .and_then(|rect| surface.content_rect_to_screen_at(rect, scale_factor))
            .and_then(finite_positive_rect)
    }

    fn effective_handle_size(&self) -> f32 {
        finite_positive(self.handle_size).unwrap_or(9.0)
    }

    fn effective_rotate_offset(&self) -> f32 {
        finite_positive(self.rotate_offset).unwrap_or(20.0)
    }
}

/// Data-only selection outline descriptor resolved into screen space.
#[derive(Debug, Clone, PartialEq)]
pub struct ViewportSelectionOutlineDescriptor {
    /// Stable target identity.
    pub target: ViewportSelectionTargetId,
    /// Source target bounds in content coordinates.
    pub content_rect: Rect,
    /// Selection outline rectangle in UI logical screen space.
    pub screen_rect: Rect,
    /// Whether the target can currently emit interaction requests.
    pub enabled: bool,
    /// Whether the target exists in the current application context.
    pub available: bool,
    /// Whether transform requests are suppressed for this target.
    pub read_only: bool,
    /// Target priority used for deterministic ordering.
    pub priority: i32,
    /// Optional accessible/debug label.
    pub label: Option<String>,
}

impl ViewportSelectionOutlineDescriptor {
    fn from_target(
        target: &ViewportSelectionTargetDescriptor,
        surface: ViewportSurface,
        scale_factor: ScaleFactor,
    ) -> Option<Self> {
        target.can_show_selection().then_some(())?;
        Some(Self {
            target: target.id,
            content_rect: finite_positive_rect(target.content_rect)?,
            screen_rect: target.screen_rect(surface, scale_factor)?,
            enabled: target.state.enabled(),
            available: target.state.available(),
            read_only: target.state.read_only(),
            priority: target.priority,
            label: target.label.clone(),
        })
    }
}

/// Data-only transform handle descriptor resolved into screen space.
#[derive(Debug, Clone, PartialEq)]
pub struct ViewportTransformHandleDescriptor {
    /// Stable handle identity.
    pub id: ViewportTransformHandleId,
    /// Stable selection target identity.
    pub target: ViewportSelectionTargetId,
    /// Generic handle kind.
    pub kind: ViewportTransformHandleKind,
    /// Source target bounds in content coordinates.
    pub source_content_rect: Rect,
    /// Source target bounds in UI logical screen space.
    pub target_screen_rect: Rect,
    /// Handle hit rectangle in UI logical screen space.
    pub handle_screen_rect: Rect,
    /// Target priority used for deterministic topmost resolution.
    pub target_priority: i32,
    /// Handle-specific hit priority.
    pub handle_priority: i32,
    /// Cursor request metadata associated with this handle.
    pub cursor: ViewportCursorMetadata,
    /// Optional accessible/debug label.
    pub label: Option<String>,
}

/// Data-only result of viewport transform handle hit testing.
#[derive(Debug, Clone, PartialEq)]
pub struct ViewportTransformHandleHit {
    /// Stable handle identity.
    pub handle: ViewportTransformHandleId,
    /// Stable selection target identity.
    pub target: ViewportSelectionTargetId,
    /// Generic handle kind.
    pub kind: ViewportTransformHandleKind,
    /// Source target bounds in content coordinates.
    pub source_content_rect: Rect,
    /// Source target bounds in UI logical screen space.
    pub target_screen_rect: Rect,
    /// Handle hit rectangle in UI logical screen space.
    pub handle_screen_rect: Rect,
    /// Hit point in UI logical screen space.
    pub point: Point,
    /// Hit point transformed into content coordinates when possible.
    pub content_point: Option<Point>,
    /// Target priority used for deterministic topmost resolution.
    pub target_priority: i32,
    /// Handle-specific hit priority.
    pub handle_priority: i32,
    /// Cursor request metadata associated with the hit handle.
    pub cursor: ViewportCursorMetadata,
    /// Optional accessible/debug label.
    pub label: Option<String>,
}

impl ViewportTransformHandleHit {
    fn from_descriptor(
        descriptor: &ViewportTransformHandleDescriptor,
        point: Point,
        surface: ViewportSurface,
        scale_factor: ScaleFactor,
    ) -> Self {
        Self {
            handle: descriptor.id,
            target: descriptor.target,
            kind: descriptor.kind,
            source_content_rect: descriptor.source_content_rect,
            target_screen_rect: descriptor.target_screen_rect,
            handle_screen_rect: descriptor.handle_screen_rect,
            point,
            content_point: surface.screen_to_content_at(point, scale_factor),
            target_priority: descriptor.target_priority,
            handle_priority: descriptor.handle_priority,
            cursor: descriptor.cursor.clone(),
            label: descriptor.label.clone(),
        }
    }
}

/// Pointer capture metadata for a viewport transform handle drag.
#[derive(Debug, Clone, PartialEq)]
pub struct ViewportTransformDragCapture {
    /// Stable handle identity captured at drag start.
    pub handle: ViewportTransformHandleId,
    /// Stable selection target identity captured at drag start.
    pub target: ViewportSelectionTargetId,
    /// Generic handle kind captured at drag start.
    pub kind: ViewportTransformHandleKind,
    /// Source target bounds in content coordinates at drag start.
    pub source_content_rect: Rect,
    /// Source target bounds in UI logical screen space at drag start.
    pub target_screen_rect: Rect,
    /// Handle hit rectangle in UI logical screen space at drag start.
    pub handle_screen_rect: Rect,
    /// Pointer position in UI logical screen space at drag start.
    pub pointer_origin_screen: Point,
    /// Pointer position in content coordinates at drag start, when conversion is possible.
    pub pointer_origin_content: Option<Point>,
}

impl ViewportTransformDragCapture {
    /// Creates pointer capture metadata from a handle hit.
    #[must_use]
    pub fn from_hit(hit: &ViewportTransformHandleHit) -> Self {
        Self {
            handle: hit.handle,
            target: hit.target,
            kind: hit.kind,
            source_content_rect: hit.source_content_rect,
            target_screen_rect: hit.target_screen_rect,
            handle_screen_rect: hit.handle_screen_rect,
            pointer_origin_screen: hit.point,
            pointer_origin_content: hit.content_point,
        }
    }
}

/// Status for a viewport transform drag request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ViewportTransformDragStatus {
    /// The target is present and can receive transform request metadata.
    Active,
    /// The captured target ID no longer appears in the current target descriptors.
    StaleTarget,
    /// The target is disabled, unavailable, read-only, unselected, or no longer exposes the handle.
    UnavailableTarget,
    /// The current pointer position was not finite and was replaced by the capture origin.
    InvalidPointer,
    /// The viewport could not convert screen deltas into content deltas.
    InvalidScale,
}

/// Data-only transform drag update request metadata.
#[derive(Debug, Clone, PartialEq)]
pub struct ViewportTransformDragRequest {
    /// Drag request status.
    pub status: ViewportTransformDragStatus,
    /// Stable handle identity captured at drag start.
    pub handle: ViewportTransformHandleId,
    /// Stable selection target identity captured at drag start.
    pub target: ViewportSelectionTargetId,
    /// Generic handle kind captured at drag start.
    pub kind: ViewportTransformHandleKind,
    /// Source target bounds in content coordinates at drag start.
    pub source_content_rect: Rect,
    /// Current target bounds in content coordinates, when the target is present.
    pub current_content_rect: Option<Rect>,
    /// Pointer position in UI logical screen space at drag start.
    pub pointer_origin_screen: Point,
    /// Current pointer position in UI logical screen space.
    pub pointer_current_screen: Point,
    /// Pointer position in content coordinates at drag start, when conversion is possible.
    pub pointer_origin_content: Option<Point>,
    /// Current pointer position in content coordinates, when conversion is possible.
    pub pointer_current_content: Option<Point>,
    /// Pointer delta in UI logical screen space from drag start.
    pub screen_delta: Vec2,
    /// Pointer delta in content coordinates from drag start.
    pub content_delta: Vec2,
}

impl ViewportTransformDragRequest {
    /// Creates drag update metadata from an existing pointer capture.
    #[must_use]
    pub fn update(
        surface: ViewportSurface,
        targets: &[ViewportSelectionTargetDescriptor],
        capture: &ViewportTransformDragCapture,
        pointer_current_screen: Point,
    ) -> Self {
        Self::update_at(
            surface,
            targets,
            capture,
            pointer_current_screen,
            ScaleFactor::ONE,
        )
    }

    /// Creates drag update metadata from an existing pointer capture for a viewport scale factor.
    #[must_use]
    pub fn update_at(
        surface: ViewportSurface,
        targets: &[ViewportSelectionTargetDescriptor],
        capture: &ViewportTransformDragCapture,
        pointer_current_screen: Point,
        scale_factor: ScaleFactor,
    ) -> Self {
        let pointer_is_valid = finite_point(pointer_current_screen).is_some();
        let pointer_current_screen = if pointer_is_valid {
            pointer_current_screen
        } else {
            capture.pointer_origin_screen
        };
        let screen_delta = Vec2::new(
            pointer_current_screen.x - capture.pointer_origin_screen.x,
            pointer_current_screen.y - capture.pointer_origin_screen.y,
        );
        let scale = finite_positive(surface.content_scale_at(scale_factor));
        let content_delta = scale.map_or(Vec2::ZERO, |scale| {
            Vec2::new(screen_delta.x / scale, screen_delta.y / scale)
        });
        let target = targets.iter().find(|target| target.id == capture.target);
        let current_content_rect =
            target.and_then(|target| finite_positive_rect(target.content_rect));
        let status = if !pointer_is_valid {
            ViewportTransformDragStatus::InvalidPointer
        } else if scale.is_none() {
            ViewportTransformDragStatus::InvalidScale
        } else {
            match target {
                None => ViewportTransformDragStatus::StaleTarget,
                Some(target)
                    if current_content_rect.is_some()
                        && target.can_request_transform()
                        && target.handles.contains(capture.kind) =>
                {
                    ViewportTransformDragStatus::Active
                }
                Some(_) => ViewportTransformDragStatus::UnavailableTarget,
            }
        };

        Self {
            status,
            handle: capture.handle,
            target: capture.target,
            kind: capture.kind,
            source_content_rect: capture.source_content_rect,
            current_content_rect,
            pointer_origin_screen: capture.pointer_origin_screen,
            pointer_current_screen,
            pointer_origin_content: capture.pointer_origin_content,
            pointer_current_content: surface
                .screen_to_content_at(pointer_current_screen, scale_factor),
            screen_delta,
            content_delta,
        }
    }

    /// Returns true when this request is deterministic no-op/error metadata.
    #[must_use]
    pub const fn is_noop(&self) -> bool {
        !matches!(self.status, ViewportTransformDragStatus::Active)
    }
}

/// Viewport overlay target category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ViewportOverlayKind {
    /// Application-owned texture surface under the UI.
    TextureSurface,
    /// Source content bounds transformed by the viewport pan/zoom.
    ContentBounds,
    /// Guide-like overlay region.
    Guide,
    /// Tool-owned generic overlay region.
    ToolRegion,
}

/// Coordinate space used by a viewport overlay target rectangle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ViewportOverlaySpace {
    /// Rectangle is already in UI logical screen space.
    Screen,
    /// Rectangle is local to the viewport bounds.
    Viewport,
    /// Rectangle is in content coordinates and must be transformed by the viewport surface.
    Content,
}

/// Data-only viewport overlay hit target descriptor.
#[derive(Debug, Clone, PartialEq)]
pub struct ViewportOverlayDescriptor {
    /// Stable overlay identity.
    pub id: ViewportOverlayId,
    /// Overlay target category.
    pub kind: ViewportOverlayKind,
    /// Overlay target rectangle in `space`.
    pub rect: Rect,
    /// Coordinate space used by `rect`.
    pub space: ViewportOverlaySpace,
    /// Explicit hit-test priority. Higher priority wins.
    pub priority: i32,
    /// Optional owning tool identity.
    pub tool: Option<ViewportToolId>,
    /// Optional cursor request metadata.
    pub cursor: Option<ViewportCursorMetadata>,
    /// Whether this overlay can emit interaction requests.
    pub enabled: bool,
    /// Whether this overlay is available in the current app context.
    pub available: bool,
    /// Optional accessible/debug label.
    pub label: Option<String>,
}

impl ViewportOverlayDescriptor {
    /// Creates an enabled, available overlay target descriptor.
    #[must_use]
    pub fn new(
        id: ViewportOverlayId,
        kind: ViewportOverlayKind,
        rect: Rect,
        space: ViewportOverlaySpace,
    ) -> Self {
        Self {
            id,
            kind,
            rect,
            space,
            priority: default_viewport_overlay_priority(kind),
            tool: None,
            cursor: None,
            enabled: true,
            available: true,
            label: None,
        }
    }

    /// Creates a texture-surface target from the viewport source bounds.
    #[must_use]
    pub fn texture_surface(id: ViewportOverlayId, surface: ViewportSurface) -> Self {
        Self::content_bounds_with_kind(id, surface, ViewportOverlayKind::TextureSurface)
    }

    /// Creates a content-bounds target from the viewport source bounds.
    #[must_use]
    pub fn content_bounds(id: ViewportOverlayId, surface: ViewportSurface) -> Self {
        Self::content_bounds_with_kind(id, surface, ViewportOverlayKind::ContentBounds)
    }

    fn content_bounds_with_kind(
        id: ViewportOverlayId,
        surface: ViewportSurface,
        kind: ViewportOverlayKind,
    ) -> Self {
        let rect = surface.effective_source_size().map_or(Rect::ZERO, |size| {
            Rect::new(0.0, 0.0, size.width, size.height)
        });
        Self::new(id, kind, rect, ViewportOverlaySpace::Content)
    }

    /// Sets explicit hit-test priority. Higher priority wins.
    #[must_use]
    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    /// Attaches an owning tool identity.
    #[must_use]
    pub fn with_tool(mut self, tool: ViewportToolId) -> Self {
        self.tool = Some(tool);
        self
    }

    /// Adds cursor request metadata.
    #[must_use]
    pub fn with_cursor(mut self, cursor: ViewportCursorMetadata) -> Self {
        self.cursor = Some(cursor);
        self
    }

    /// Adds an accessible/debug label.
    #[must_use]
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Marks the overlay target as enabled or disabled.
    #[must_use]
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Marks the overlay target as available or unavailable in the current app context.
    #[must_use]
    pub fn available(mut self, available: bool) -> Self {
        self.available = available;
        self
    }

    /// Returns true when this overlay can emit interaction requests.
    #[must_use]
    pub const fn can_interact(&self) -> bool {
        self.enabled && self.available
    }

    /// Returns the transformed UI-space rectangle for hit testing.
    #[must_use]
    pub fn screen_rect(&self, surface: ViewportSurface, scale_factor: ScaleFactor) -> Option<Rect> {
        let rect = finite_positive_rect(self.rect)?;
        match self.space {
            ViewportOverlaySpace::Screen => Some(rect),
            ViewportOverlaySpace::Viewport => {
                let bounds = surface.effective_bounds();
                finite_positive_rect(Rect::new(
                    bounds.x + rect.x,
                    bounds.y + rect.y,
                    rect.width,
                    rect.height,
                ))
            }
            ViewportOverlaySpace::Content => surface.content_rect_to_screen_at(rect, scale_factor),
        }
        .and_then(finite_positive_rect)
    }
}

/// Data-only result of viewport overlay hit testing.
#[derive(Debug, Clone, PartialEq)]
pub struct ViewportOverlayHit {
    /// Stable overlay identity.
    pub overlay: ViewportOverlayId,
    /// Overlay target category.
    pub kind: ViewportOverlayKind,
    /// Hit rectangle in UI logical screen space.
    pub rect: Rect,
    /// Hit point in UI logical screen space.
    pub point: Point,
    /// Hit point transformed into content coordinates when possible.
    pub content_point: Option<Point>,
    /// Winning explicit hit priority.
    pub priority: i32,
    /// Optional owning tool identity.
    pub tool: Option<ViewportToolId>,
    /// Cursor request metadata associated with the hit target.
    pub cursor: Option<ViewportCursorMetadata>,
    /// Optional accessible/debug label.
    pub label: Option<String>,
}

impl ViewportOverlayHit {
    fn from_descriptor(
        descriptor: &ViewportOverlayDescriptor,
        rect: Rect,
        point: Point,
        surface: ViewportSurface,
        scale_factor: ScaleFactor,
    ) -> Self {
        Self {
            overlay: descriptor.id,
            kind: descriptor.kind,
            rect,
            point,
            content_point: surface.screen_to_content_at(point, scale_factor),
            priority: descriptor.priority,
            tool: descriptor.tool,
            cursor: descriptor.cursor.clone(),
            label: descriptor.label.clone(),
        }
    }
}

/// Resolves a UI-space point to the highest-priority viewport overlay target.
///
/// Disabled or unavailable overlays are skipped. Higher `priority` wins. When
/// priorities tie, the lower stable `ViewportOverlayId` wins so the result does
/// not depend on primitive emission order or descriptor ordering.
#[must_use]
pub fn hit_test_viewport_overlays(
    surface: ViewportSurface,
    overlays: &[ViewportOverlayDescriptor],
    point: Point,
) -> Option<ViewportOverlayHit> {
    hit_test_viewport_overlays_at(surface, overlays, point, ScaleFactor::ONE)
}

/// Resolves a UI-space point to the highest-priority viewport overlay target
/// for a viewport scale factor.
#[must_use]
pub fn hit_test_viewport_overlays_at(
    surface: ViewportSurface,
    overlays: &[ViewportOverlayDescriptor],
    point: Point,
    scale_factor: ScaleFactor,
) -> Option<ViewportOverlayHit> {
    let point = finite_point(point)?;
    overlays
        .iter()
        .filter(|overlay| overlay.can_interact())
        .filter_map(|overlay| {
            let rect = overlay.screen_rect(surface, scale_factor)?;
            rect.contains_point(point).then(|| {
                ViewportOverlayHit::from_descriptor(overlay, rect, point, surface, scale_factor)
            })
        })
        .max_by(|left, right| {
            left.priority
                .cmp(&right.priority)
                .then_with(|| right.overlay.cmp(&left.overlay))
        })
}

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

/// Data-only pan/zoom HUD descriptor supplied by the application.
#[derive(Debug, Clone, PartialEq)]
pub struct ViewportPanZoomHudDescriptor {
    /// Stable HUD semantic identity.
    pub id: WidgetId,
    /// HUD accessible/debug label.
    pub label: String,
    /// Optional focused selection target.
    pub focused_target: Option<ViewportSelectionTargetId>,
    /// Current selected target IDs.
    pub selected_targets: Vec<ViewportSelectionTargetId>,
}

impl ViewportPanZoomHudDescriptor {
    /// Creates a viewport pan/zoom HUD descriptor.
    #[must_use]
    pub fn new(id: WidgetId, label: impl Into<String>) -> Self {
        Self {
            id,
            label: label.into(),
            focused_target: None,
            selected_targets: Vec::new(),
        }
    }

    /// Adds focused target metadata.
    #[must_use]
    pub const fn with_focused_target(mut self, target: ViewportSelectionTargetId) -> Self {
        self.focused_target = Some(target);
        self
    }

    /// Adds selected target metadata.
    #[must_use]
    pub fn with_selected_targets(mut self, targets: &[ViewportSelectionTargetId]) -> Self {
        self.selected_targets.extend_from_slice(targets);
        self
    }

    /// Resolves the HUD descriptor against a viewport surface.
    #[must_use]
    pub fn resolve(&self, surface: ViewportSurface) -> ViewportPanZoomHud {
        let mut selected_targets = self.selected_targets.clone();
        selected_targets.sort();
        selected_targets.dedup();
        ViewportPanZoomHud {
            id: self.id,
            label: self.label.clone(),
            fit: surface.pan_zoom.fit,
            zoom: finite_positive(surface.pan_zoom.zoom).unwrap_or(1.0),
            effective_content_scale: finite_or_zero(surface.content_scale()),
            pan: Vec2::new(
                finite_or_zero(surface.pan_zoom.pan.x),
                finite_or_zero(surface.pan_zoom.pan.y),
            ),
            content_size: surface.effective_source_size().unwrap_or(Size::ZERO),
            focused_target: self.focused_target,
            selected_targets,
        }
    }
}

/// Pan/zoom HUD metadata resolved from a viewport surface.
#[derive(Debug, Clone, PartialEq)]
pub struct ViewportPanZoomHud {
    /// Stable HUD semantic identity.
    pub id: WidgetId,
    /// HUD accessible/debug label.
    pub label: String,
    /// Current fit mode.
    pub fit: ViewportFit,
    /// Current custom zoom value.
    pub zoom: f32,
    /// Effective content-to-screen scale after fit, zoom, and DPI policy.
    pub effective_content_scale: f32,
    /// Sanitized pan offset in logical screen units.
    pub pan: Vec2,
    /// Sanitized source content size.
    pub content_size: Size,
    /// Optional focused selection target.
    pub focused_target: Option<ViewportSelectionTargetId>,
    /// Sorted, deduplicated selected target IDs.
    pub selected_targets: Vec<ViewportSelectionTargetId>,
}

impl ViewportPanZoomHud {
    /// Builds a stable, generic text value for HUD semantics or debug surfaces.
    #[must_use]
    pub fn value_text(&self) -> String {
        format!(
            "{:?} zoom {:.3}, scale {:.3}, pan {:.3},{:.3}, content {:.3}x{:.3}, selected {}",
            self.fit,
            self.zoom,
            self.effective_content_scale,
            self.pan.x,
            self.pan.y,
            self.content_size.width,
            self.content_size.height,
            self.selected_targets.len()
        )
    }

    /// Builds backend-neutral semantic metadata for this HUD.
    #[must_use]
    pub fn semantics(&self, bounds: Rect) -> SemanticNode {
        let mut node = SemanticNode::new(
            self.id,
            SemanticRole::Custom("viewport-pan-zoom-hud".to_owned()),
            sanitize_rect(bounds),
        )
        .with_label(self.label.clone());
        node.state.value = Some(SemanticValue::Text(self.value_text()));
        node
    }
}

/// Resolves selected target outlines into UI logical screen space.
#[must_use]
pub fn viewport_selection_outlines(
    surface: ViewportSurface,
    targets: &[ViewportSelectionTargetDescriptor],
) -> Vec<ViewportSelectionOutlineDescriptor> {
    viewport_selection_outlines_at(surface, targets, ScaleFactor::ONE)
}

/// Resolves selected target outlines into UI logical screen space for a viewport scale factor.
#[must_use]
pub fn viewport_selection_outlines_at(
    surface: ViewportSurface,
    targets: &[ViewportSelectionTargetDescriptor],
    scale_factor: ScaleFactor,
) -> Vec<ViewportSelectionOutlineDescriptor> {
    let mut outlines = targets
        .iter()
        .filter_map(|target| {
            ViewportSelectionOutlineDescriptor::from_target(target, surface, scale_factor)
        })
        .collect::<Vec<_>>();
    outlines.sort_by(|left, right| {
        left.priority
            .cmp(&right.priority)
            .then_with(|| left.target.cmp(&right.target))
    });
    outlines
}

/// Resolves selected target transform handles into UI logical screen space.
#[must_use]
pub fn viewport_transform_handles(
    surface: ViewportSurface,
    targets: &[ViewportSelectionTargetDescriptor],
) -> Vec<ViewportTransformHandleDescriptor> {
    viewport_transform_handles_at(surface, targets, ScaleFactor::ONE)
}

/// Resolves selected target transform handles into UI logical screen space for a scale factor.
#[must_use]
pub fn viewport_transform_handles_at(
    surface: ViewportSurface,
    targets: &[ViewportSelectionTargetDescriptor],
    scale_factor: ScaleFactor,
) -> Vec<ViewportTransformHandleDescriptor> {
    let mut handles = targets
        .iter()
        .filter(|target| target.can_request_transform())
        .filter_map(|target| {
            let source_content_rect = finite_positive_rect(target.content_rect)?;
            let target_screen_rect = target.screen_rect(surface, scale_factor)?;
            Some((target, source_content_rect, target_screen_rect))
        })
        .flat_map(|(target, source_content_rect, target_screen_rect)| {
            target.handles.kinds().filter_map(move |kind| {
                let handle_screen_rect = transform_handle_rect(target, target_screen_rect, kind)?;
                Some(ViewportTransformHandleDescriptor {
                    id: ViewportTransformHandleId::new(target.id, kind),
                    target: target.id,
                    kind,
                    source_content_rect,
                    target_screen_rect,
                    handle_screen_rect,
                    target_priority: target.priority,
                    handle_priority: kind.hit_priority(),
                    cursor: ViewportCursorMetadata::new(kind.cursor_shape()),
                    label: target.label.clone(),
                })
            })
        })
        .collect::<Vec<_>>();
    handles.sort_by(|left, right| {
        left.target_priority
            .cmp(&right.target_priority)
            .then_with(|| left.handle_priority.cmp(&right.handle_priority))
            .then_with(|| left.target.cmp(&right.target))
            .then_with(|| left.kind.cmp(&right.kind))
    });
    handles
}

/// Resolves a UI-space point to the highest-priority viewport transform handle.
///
/// Disabled, unavailable, read-only, and unselected targets are skipped.
/// Higher target priority is treated as topmost. Within one target, more
/// specific handles win over broad move regions. Stable target and handle IDs
/// break remaining ties so descriptor order does not affect the result.
#[must_use]
pub fn hit_test_viewport_transform_handles(
    surface: ViewportSurface,
    targets: &[ViewportSelectionTargetDescriptor],
    point: Point,
) -> Option<ViewportTransformHandleHit> {
    hit_test_viewport_transform_handles_at(surface, targets, point, ScaleFactor::ONE)
}

/// Resolves a UI-space point to the highest-priority viewport transform handle
/// for a viewport scale factor.
#[must_use]
pub fn hit_test_viewport_transform_handles_at(
    surface: ViewportSurface,
    targets: &[ViewportSelectionTargetDescriptor],
    point: Point,
    scale_factor: ScaleFactor,
) -> Option<ViewportTransformHandleHit> {
    let point = finite_point(point)?;
    viewport_transform_handles_at(surface, targets, scale_factor)
        .into_iter()
        .filter(|handle| handle.handle_screen_rect.contains_point(point))
        .map(|handle| {
            ViewportTransformHandleHit::from_descriptor(&handle, point, surface, scale_factor)
        })
        .max_by(|left, right| {
            left.target_priority
                .cmp(&right.target_priority)
                .then_with(|| left.handle_priority.cmp(&right.handle_priority))
                .then_with(|| right.target.cmp(&left.target))
                .then_with(|| right.kind.cmp(&left.kind))
        })
}

fn transform_handle_rect(
    target: &ViewportSelectionTargetDescriptor,
    target_screen_rect: Rect,
    kind: ViewportTransformHandleKind,
) -> Option<Rect> {
    if kind == ViewportTransformHandleKind::Move {
        return finite_positive_rect(target_screen_rect);
    }

    let size = target.effective_handle_size();
    let half = size * 0.5;
    let center =
        transform_handle_center(target_screen_rect, target.effective_rotate_offset(), kind);
    finite_positive_rect(Rect::new(center.x - half, center.y - half, size, size))
}

fn transform_handle_center(
    rect: Rect,
    rotate_offset: f32,
    kind: ViewportTransformHandleKind,
) -> Point {
    let center = rect.center();
    match kind {
        ViewportTransformHandleKind::Move | ViewportTransformHandleKind::Pivot => center,
        ViewportTransformHandleKind::ResizeTopLeft => Point::new(rect.x, rect.y),
        ViewportTransformHandleKind::ResizeTop => Point::new(center.x, rect.y),
        ViewportTransformHandleKind::ResizeTopRight => Point::new(rect.max_x(), rect.y),
        ViewportTransformHandleKind::ResizeRight => Point::new(rect.max_x(), center.y),
        ViewportTransformHandleKind::ResizeBottomRight => Point::new(rect.max_x(), rect.max_y()),
        ViewportTransformHandleKind::ResizeBottom => Point::new(center.x, rect.max_y()),
        ViewportTransformHandleKind::ResizeBottomLeft => Point::new(rect.x, rect.max_y()),
        ViewportTransformHandleKind::ResizeLeft => Point::new(rect.x, center.y),
        ViewportTransformHandleKind::Rotate => Point::new(center.x, rect.y - rotate_offset),
    }
}

fn fit_scale(source: Size, bounds: Size) -> f32 {
    let Some(source_width) = finite_positive(source.width) else {
        return 0.0;
    };
    let Some(source_height) = finite_positive(source.height) else {
        return 0.0;
    };
    let Some(bounds_width) = finite_positive(bounds.width) else {
        return 0.0;
    };
    let Some(bounds_height) = finite_positive(bounds.height) else {
        return 0.0;
    };
    (bounds_width / source_width).min(bounds_height / source_height)
}

fn fill_scale(source: Size, bounds: Size) -> f32 {
    let Some(source_width) = finite_positive(source.width) else {
        return 0.0;
    };
    let Some(source_height) = finite_positive(source.height) else {
        return 0.0;
    };
    let Some(bounds_width) = finite_positive(bounds.width) else {
        return 0.0;
    };
    let Some(bounds_height) = finite_positive(bounds.height) else {
        return 0.0;
    };
    (bounds_width / source_width).max(bounds_height / source_height)
}

fn finite_or_zero(value: f32) -> f32 {
    if value.is_finite() { value } else { 0.0 }
}

fn finite_or_none(value: f32) -> Option<f32> {
    value.is_finite().then_some(value)
}

fn finite_non_negative(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}

fn finite_positive(value: f32) -> Option<f32> {
    value
        .is_finite()
        .then_some(value)
        .filter(|value| *value > 0.0)
}

fn finite_point(point: Point) -> Option<Point> {
    (point.x.is_finite() && point.y.is_finite()).then_some(point)
}

fn finite_positive_rect(rect: Rect) -> Option<Rect> {
    (rect.x.is_finite()
        && rect.y.is_finite()
        && rect.width.is_finite()
        && rect.height.is_finite()
        && rect.width > 0.0
        && rect.height > 0.0)
        .then_some(rect)
}

fn sanitize_rect(rect: Rect) -> Rect {
    Rect::new(
        finite_or_zero(rect.x),
        finite_or_zero(rect.y),
        finite_non_negative(rect.width),
        finite_non_negative(rect.height),
    )
}

fn finite_content_guide_position(
    surface: ViewportSurface,
    orientation: ViewportGuideOrientation,
    position: f32,
) -> Option<f32> {
    let position = finite_or_none(position)?;
    let source = surface.effective_source_size()?;
    let max = match orientation {
        ViewportGuideOrientation::Horizontal => source.height,
        ViewportGuideOrientation::Vertical => source.width,
    };
    (position >= 0.0 && position <= max).then_some(position)
}

fn guide_position_inside_bounds(
    bounds: Rect,
    orientation: ViewportGuideOrientation,
    position: f32,
) -> bool {
    match orientation {
        ViewportGuideOrientation::Horizontal => position >= bounds.y && position <= bounds.max_y(),
        ViewportGuideOrientation::Vertical => position >= bounds.x && position <= bounds.max_x(),
    }
}

fn guide_screen_rect(
    bounds: Rect,
    orientation: ViewportGuideOrientation,
    position: f32,
) -> Option<Rect> {
    if !position.is_finite() || !guide_position_inside_bounds(bounds, orientation, position) {
        return None;
    }
    let rect = match orientation {
        ViewportGuideOrientation::Horizontal => {
            Rect::new(bounds.x, position - 0.5, bounds.width, 1.0)
        }
        ViewportGuideOrientation::Vertical => {
            Rect::new(position - 0.5, bounds.y, 1.0, bounds.height)
        }
    };
    finite_positive_rect(rect)
}

fn guide_sort_key(guide: &ViewportResolvedGuide) -> f32 {
    match guide.placement {
        ViewportGuidePlacement::Content(position) | ViewportGuidePlacement::Screen(position) => {
            position
        }
    }
}

fn visible_ruler_content_range(
    surface: ViewportSurface,
    edge: ViewportRulerEdge,
    scale_factor: ScaleFactor,
) -> Option<(f32, f32)> {
    let bounds = surface.effective_bounds();
    let source = surface.effective_source_size()?;
    let (screen_min, screen_max, content_max) = match edge {
        ViewportRulerEdge::Top => (
            Point::new(bounds.x, bounds.y),
            Point::new(bounds.max_x(), bounds.y),
            source.width,
        ),
        ViewportRulerEdge::Left => (
            Point::new(bounds.x, bounds.y),
            Point::new(bounds.x, bounds.max_y()),
            source.height,
        ),
    };
    let content_min = surface.screen_to_content_at(screen_min, scale_factor)?;
    let content_max_point = surface.screen_to_content_at(screen_max, scale_factor)?;
    let (start, end) = match edge {
        ViewportRulerEdge::Top => (content_min.x, content_max_point.x),
        ViewportRulerEdge::Left => (content_min.y, content_max_point.y),
    };
    let min = start.min(end).max(0.0);
    let max = start.max(end).min(content_max);
    (min.is_finite() && max.is_finite() && max > min).then_some((min, max))
}

fn ruler_axis_screen_position(
    surface: ViewportSurface,
    edge: ViewportRulerEdge,
    value: f32,
    scale_factor: ScaleFactor,
) -> Option<f32> {
    let value = finite_or_none(value)?;
    let point = match edge {
        ViewportRulerEdge::Top => {
            surface.content_to_screen_at(Point::new(value, 0.0), scale_factor)?
        }
        ViewportRulerEdge::Left => {
            surface.content_to_screen_at(Point::new(0.0, value), scale_factor)?
        }
    };
    let position = match edge {
        ViewportRulerEdge::Top => point.x,
        ViewportRulerEdge::Left => point.y,
    };
    finite_or_none(position)
}

fn viewport_ruler_ticks(
    surface: ViewportSurface,
    edge: ViewportRulerEdge,
    visible_content_range: (f32, f32),
    origin_content: f32,
    max_ticks: usize,
    scale_factor: ScaleFactor,
) -> Vec<ViewportRulerTick> {
    let scale = finite_positive(surface.content_scale_at(scale_factor)).unwrap_or(1.0);
    let mut ticks = ruler_ticks(visible_content_range.0, visible_content_range.1, scale)
        .into_iter()
        .filter(|value| {
            value.is_finite()
                && *value >= visible_content_range.0
                && *value <= visible_content_range.1
        })
        .take(max_ticks)
        .filter_map(|value| {
            let screen_position = ruler_axis_screen_position(surface, edge, value, scale_factor)?;
            let major = is_major_ruler_tick(value, origin_content);
            Some(ViewportRulerTick {
                value,
                screen_position,
                major,
                label: major.then(|| ruler_tick_label(value - origin_content)),
            })
        })
        .collect::<Vec<_>>();
    ticks.sort_by(|left, right| {
        left.value
            .total_cmp(&right.value)
            .then_with(|| left.screen_position.total_cmp(&right.screen_position))
    });
    ticks
}

fn is_major_ruler_tick(value: f32, origin_content: f32) -> bool {
    let relative = value - origin_content;
    if !relative.is_finite() {
        return false;
    }
    let rounded = (relative / 50.0).round();
    (relative / 50.0 - rounded).abs() <= 0.001
}

fn ruler_tick_label(value: f32) -> String {
    if (value - value.round()).abs() <= 0.001 {
        format!("{value:.0}")
    } else {
        format!("{value:.2}")
    }
}

const fn default_viewport_overlay_priority(kind: ViewportOverlayKind) -> i32 {
    match kind {
        ViewportOverlayKind::TextureSurface => 0,
        ViewportOverlayKind::ContentBounds => 10,
        ViewportOverlayKind::Guide => 20,
        ViewportOverlayKind::ToolRegion => 30,
    }
}

/// Viewport guide line.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Guide {
    /// Horizontal guide at y.
    Horizontal(f32),
    /// Vertical guide at x.
    Vertical(f32),
}

/// Computes ruler tick positions.
#[must_use]
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]
pub fn ruler_ticks(start: f32, end: f32, zoom: f32) -> Vec<f32> {
    let Some(zoom) = finite_positive(zoom) else {
        return Vec::new();
    };
    if !start.is_finite() || !end.is_finite() {
        return Vec::new();
    }
    let min = start.min(end);
    let max = start.max(end);
    let span = max - min;
    if span <= 0.0 {
        return Vec::new();
    }
    let mut step = if zoom >= 2.0 {
        10.0
    } else if zoom >= 1.0 {
        25.0
    } else {
        50.0
    };

    let mut first = (min / step).floor() as i32;
    let mut last = (max / step).ceil() as i32;
    while last.saturating_sub(first) > 4096 {
        step *= 2.0;
        first = (min / step).floor() as i32;
        last = (max / step).ceil() as i32;
    }
    (first..=last).map(|index| index as f32 * step).collect()
}

/// Emits guide line primitives.
#[must_use]
pub fn guide_primitives(bounds: Rect, guides: &[Guide], color: Color) -> Vec<Primitive> {
    guides
        .iter()
        .map(|guide| match *guide {
            Guide::Horizontal(y) => Primitive::Line(LinePrimitive {
                from: Point::new(bounds.x, y),
                to: Point::new(bounds.max_x(), y),
                stroke: Stroke::new(1.0, Brush::Solid(color)),
            }),
            Guide::Vertical(x) => Primitive::Line(LinePrimitive {
                from: Point::new(x, bounds.y),
                to: Point::new(x, bounds.max_y()),
                stroke: Stroke::new(1.0, Brush::Solid(color)),
            }),
        })
        .collect()
}

/// Crosshair overlay state.
#[derive(Debug, Clone, PartialEq)]
pub struct Crosshair {
    /// Whether the crosshair is visible.
    pub visible: bool,
    /// Cursor position.
    pub position: Point,
    /// Optional label.
    pub label: Option<String>,
    /// Crosshair color.
    pub color: Color,
}

impl Crosshair {
    fn with_position(&self, position: Point) -> Self {
        Self {
            visible: self.visible,
            position,
            label: self.label.clone(),
            color: self.color,
        }
    }

    /// Emits crosshair primitives.
    #[must_use]
    pub fn primitives(&self, bounds: Rect) -> Vec<Primitive> {
        let bounds = Rect::new(
            finite_or_zero(bounds.x),
            finite_or_zero(bounds.y),
            finite_non_negative(bounds.width),
            finite_non_negative(bounds.height),
        );
        if !self.visible
            || finite_point(self.position).is_none()
            || !bounds.contains_point(self.position)
        {
            return Vec::new();
        }
        let mut primitives = vec![
            Primitive::Line(LinePrimitive {
                from: Point::new(bounds.x, self.position.y),
                to: Point::new(bounds.max_x(), self.position.y),
                stroke: Stroke::new(1.0, Brush::Solid(self.color)),
            }),
            Primitive::Line(LinePrimitive {
                from: Point::new(self.position.x, bounds.y),
                to: Point::new(self.position.x, bounds.max_y()),
                stroke: Stroke::new(1.0, Brush::Solid(self.color)),
            }),
        ];
        if let Some(label) = &self.label {
            primitives.push(Primitive::Text(TextPrimitive {
                layout: None,
                origin: Point::new(self.position.x + 6.0, self.position.y - 6.0),
                text: label.clone(),
                family: "sans-serif".to_owned(),
                size: 11.0,
                line_height: 15.0,
                brush: Brush::Solid(self.color),
            }));
        }
        primitives
    }
}

/// Viewport overlay composition request.
#[derive(Debug, Clone, PartialEq)]
pub struct ViewportComposition {
    /// Surface.
    pub surface: ViewportSurface,
    /// Guides.
    pub guides: Vec<Guide>,
    /// Crosshair.
    pub crosshair: Option<Crosshair>,
    /// Clip identity.
    pub clip: ClipId,
}

impl ViewportComposition {
    /// Emits primitives in deterministic viewport order.
    #[must_use]
    pub fn primitives(&self) -> Vec<Primitive> {
        self.primitives_at(ScaleFactor::ONE)
    }

    /// Emits primitives in deterministic viewport order for a viewport scale factor.
    #[must_use]
    pub fn primitives_at(&self, scale_factor: ScaleFactor) -> Vec<Primitive> {
        let mut primitives = vec![
            Primitive::ClipBegin {
                id: self.clip,
                rect: self.surface.bounds,
            },
            self.surface.texture_primitive_at(scale_factor),
        ];
        primitives.extend(self.surface.content_guide_primitives_at(
            &self.guides,
            Color::rgba(1.0, 1.0, 1.0, 0.35),
            scale_factor,
        ));
        if let Some(crosshair) = &self.crosshair {
            primitives.extend(
                self.surface
                    .content_crosshair_primitives_at(crosshair, scale_factor),
            );
        }
        primitives.push(Primitive::ClipEnd { id: self.clip });
        primitives
    }
}

#[allow(clippy::cast_possible_truncation)]
fn native_logical_pixel_scale(scale_factor: ScaleFactor) -> f32 {
    if scale_factor.is_valid() {
        (1.0 / scale_factor.value()) as f32
    } else {
        1.0
    }
}

fn snap_rect_to_scale(rect: Rect, scale_factor: ScaleFactor) -> Rect {
    if !rect.x.is_finite()
        || !rect.y.is_finite()
        || !rect.width.is_finite()
        || !rect.height.is_finite()
        || !scale_factor.is_valid()
        || rect.width < 0.0
        || rect.height < 0.0
    {
        return rect;
    }

    scale_factor.snap_rect_to_physical_grid(rect)
}

#[cfg(test)]
mod tests {
    use super::{
        Crosshair, Guide, PanZoom, ViewportComposition, ViewportFit, ViewportSurface,
        guide_primitives, ruler_ticks,
    };
    use kinetik_ui_core::{
        ClipId, Color, Point, Primitive, Rect, ScaleFactor, Size, TextureId, Vec2,
    };

    fn assert_approx(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() < f32::EPSILON,
            "expected {actual} to equal {expected}"
        );
    }

    fn assert_close(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() <= 0.001,
            "expected {actual} to be close to {expected}"
        );
    }

    fn assert_rect_close(actual: Rect, expected: Rect) {
        assert_close(actual.x, expected.x);
        assert_close(actual.y, expected.y);
        assert_close(actual.width, expected.width);
        assert_close(actual.height, expected.height);
    }

    fn assert_point_close(actual: Point, expected: Point) {
        assert_close(actual.x, expected.x);
        assert_close(actual.y, expected.y);
    }

    fn assert_edge_aligned(value: f32, scale_factor: ScaleFactor) {
        let physical = f64::from(value) * scale_factor.value();
        assert!(
            (physical - physical.round()).abs() <= 0.001,
            "{value} -> {physical}"
        );
    }

    fn assert_rect_edges_aligned(rect: Rect, scale_factor: ScaleFactor) {
        for edge in [rect.x, rect.y, rect.max_x(), rect.max_y()] {
            assert_edge_aligned(edge, scale_factor);
        }
    }

    fn surface() -> ViewportSurface {
        ViewportSurface {
            texture: TextureId::from_raw(1),
            source_size: Size::new(400.0, 200.0),
            bounds: Rect::new(0.0, 0.0, 200.0, 200.0),
            pan_zoom: PanZoom::default(),
        }
    }

    fn unsnapped_content_rect(surface: ViewportSurface, scale_factor: ScaleFactor) -> Rect {
        let bounds = surface.effective_bounds();
        let source = surface.effective_source_size().expect("source");
        let scale = surface.content_scale_at(scale_factor);
        let width = source.width * scale;
        let height = source.height * scale;

        Rect::new(
            bounds.x + (bounds.width - width) * 0.5 + surface.pan_zoom.pan.x,
            bounds.y + (bounds.height - height) * 0.5 + surface.pan_zoom.pan.y,
            width,
            height,
        )
    }

    fn expected_content_scale_at(surface: ViewportSurface, native_scale: f32) -> f32 {
        match surface.pan_zoom.fit {
            ViewportFit::Fit => {
                let width_scale = surface.bounds.width / surface.source_size.width;
                let height_scale = surface.bounds.height / surface.source_size.height;
                width_scale.min(height_scale)
            }
            ViewportFit::Fill => {
                let width_scale = surface.bounds.width / surface.source_size.width;
                let height_scale = surface.bounds.height / surface.source_size.height;
                width_scale.max(height_scale)
            }
            ViewportFit::ActualSize => native_scale,
            ViewportFit::Zoom => surface.pan_zoom.zoom * native_scale,
        }
    }

    fn expected_unsnapped_content_rect(surface: ViewportSurface, content_scale: f32) -> Rect {
        let width = surface.source_size.width * content_scale;
        let height = surface.source_size.height * content_scale;

        Rect::new(
            surface.bounds.x + (surface.bounds.width - width) * 0.5 + surface.pan_zoom.pan.x,
            surface.bounds.y + (surface.bounds.height - height) * 0.5 + surface.pan_zoom.pan.y,
            width,
            height,
        )
    }

    fn expected_snapped_content_rect(
        surface: ViewportSurface,
        scale_factor: ScaleFactor,
        content_scale: f32,
    ) -> Rect {
        scale_factor
            .snap_rect_to_physical_grid(expected_unsnapped_content_rect(surface, content_scale))
    }

    fn expected_screen_point(content_rect: Rect, content_scale: f32, point: Point) -> Point {
        Point::new(
            content_rect.x + point.x * content_scale,
            content_rect.y + point.y * content_scale,
        )
    }

    #[test]
    fn fit_mode_preserves_aspect_ratio() {
        let rect = surface().content_rect();

        assert_approx(rect.width, 200.0);
        assert_approx(rect.height, 100.0);
        assert_approx(rect.y, 50.0);
    }

    #[test]
    fn fill_mode_preserves_aspect_ratio_and_covers_bounds() {
        let mut surface = surface();
        surface.pan_zoom.fill();
        let rect = surface.content_rect();

        assert_approx(rect.width, 400.0);
        assert_approx(rect.height, 200.0);
        assert_approx(rect.x, -100.0);
        assert_approx(rect.y, 0.0);
    }

    #[test]
    fn pan_zoom_supports_actual_size_custom_zoom_and_pan() {
        let mut surface = surface();
        surface.pan_zoom.actual_size();
        assert_approx(surface.content_rect().width, 400.0);

        surface.pan_zoom.set_zoom(0.5);
        surface.pan_zoom.pan_by(Vec2::new(10.0, 5.0));
        let rect = surface.content_rect();

        assert_eq!(surface.pan_zoom.fit, ViewportFit::Zoom);
        assert_approx(rect.x, 10.0);
        assert_approx(rect.y, 55.0);
    }

    #[test]
    fn actual_size_maps_source_pixels_to_physical_pixels() {
        let mut surface = surface();
        surface.pan_zoom.actual_size();

        for scale_value in [1.0_f32, 1.25, 1.5, 2.0] {
            let scale_factor = ScaleFactor::new(f64::from(scale_value));
            let rect = surface.content_rect_at(scale_factor);
            let expected_scale = 1.0 / scale_value;

            assert_close(surface.content_scale_at(scale_factor), expected_scale);
            assert_close(rect.width * scale_value, surface.source_size.width);
            assert_close(rect.height * scale_value, surface.source_size.height);
            assert_rect_edges_aligned(rect, scale_factor);
        }
    }

    #[test]
    fn content_rect_at_delegates_valid_snapping_to_core_policy() {
        let mut surface = surface();
        surface.bounds = Rect::new(0.35, 0.65, 205.0, 153.0);
        surface.pan_zoom.actual_size();
        surface.pan_zoom.pan_by(Vec2::new(0.4, -0.2));

        for scale_value in [1.25, 1.5, 2.0] {
            let scale_factor = ScaleFactor::new(scale_value);
            let expected = scale_factor
                .snap_rect_to_physical_grid(unsnapped_content_rect(surface, scale_factor));

            assert_rect_close(surface.content_rect_at(scale_factor), expected);
        }
    }

    #[test]
    fn zoom_mode_maps_zoom_to_physical_scale() {
        let mut surface = surface();
        surface.pan_zoom.set_zoom(1.0);

        assert_approx(surface.content_scale_at(ScaleFactor::new(2.0)), 0.5);
        assert_approx(surface.content_rect_at(ScaleFactor::new(2.0)).width, 200.0);
    }

    #[test]
    fn pan_zoom_sanitizes_invalid_zoom_and_pan() {
        let mut surface = surface();
        surface.pan_zoom.set_zoom(f32::NAN);
        surface.pan_zoom.pan_by(Vec2::new(f32::INFINITY, 4.0));
        let rect = surface.content_rect();

        assert_eq!(surface.pan_zoom.fit, ViewportFit::Zoom);
        assert_approx(surface.content_scale(), 1.0);
        assert_approx(rect.x, -100.0);
        assert_approx(rect.y, 4.0);
    }

    #[test]
    fn invalid_surface_sizes_emit_zero_sized_texture_rect() {
        let surface = ViewportSurface {
            texture: TextureId::from_raw(1),
            source_size: Size::new(f32::NAN, 200.0),
            bounds: Rect::new(10.0, 20.0, f32::INFINITY, 200.0),
            pan_zoom: PanZoom::default(),
        };
        let rect = surface.content_rect();

        assert_approx(rect.x, 10.0);
        assert_approx(rect.y, 20.0);
        assert_approx(rect.width, 0.0);
        assert_approx(rect.height, 0.0);
        assert!(surface.screen_to_content(Point::new(10.0, 20.0)).is_none());
    }

    #[test]
    fn viewport_coordinate_conversions_round_trip() {
        let surface = surface();
        let screen = surface
            .content_to_screen(Point::new(100.0, 50.0))
            .expect("screen");
        let content = surface.screen_to_content(screen).expect("content");
        let local = surface
            .screen_to_viewport(screen)
            .and_then(|point| surface.viewport_to_content(point))
            .expect("local content");
        let rect = surface
            .content_rect_to_screen(Rect::new(100.0, 50.0, 20.0, 10.0))
            .expect("rect");

        assert_approx(screen.x, 50.0);
        assert_approx(screen.y, 75.0);
        assert_approx(content.x, 100.0);
        assert_approx(content.y, 50.0);
        assert_approx(local.x, 100.0);
        assert_approx(local.y, 50.0);
        assert_approx(rect.x, 50.0);
        assert_approx(rect.y, 75.0);
        assert_approx(rect.width, 10.0);
        assert_approx(rect.height, 5.0);
    }

    #[test]
    fn fractional_scale_coordinate_conversions_round_trip() {
        let mut surface = surface();
        surface.bounds = Rect::new(0.25, 0.5, 203.0, 177.0);
        surface.pan_zoom.set_zoom(1.35);
        surface.pan_zoom.pan_by(Vec2::new(7.25, -3.5));

        for (scale_factor, scale_value) in [
            (ScaleFactor::new(1.25), 1.25_f32),
            (ScaleFactor::new(1.5), 1.5_f32),
        ] {
            let content_scale = expected_content_scale_at(surface, 1.0 / scale_value);
            let content_rect = expected_snapped_content_rect(surface, scale_factor, content_scale);

            for point in [
                Point::new(0.0, 0.0),
                Point::new(123.25, 77.5),
                Point::new(399.0, 199.0),
            ] {
                let expected_screen = expected_screen_point(content_rect, content_scale, point);
                let expected_viewport = Point::new(
                    expected_screen.x - surface.bounds.x,
                    expected_screen.y - surface.bounds.y,
                );
                let screen = surface
                    .content_to_screen_at(point, scale_factor)
                    .expect("screen");
                let content = surface
                    .screen_to_content_at(expected_screen, scale_factor)
                    .expect("content");
                let local = surface
                    .viewport_to_content_at(expected_viewport, scale_factor)
                    .expect("local content");

                assert_point_close(screen, expected_screen);
                assert_point_close(content, point);
                assert_point_close(local, point);
            }
        }
    }

    #[test]
    fn texture_surface_emits_texture_primitive() {
        assert!(matches!(
            surface().texture_primitive(),
            Primitive::Texture(_)
        ));
    }

    #[test]
    fn texture_surface_emits_scale_aware_native_rect() {
        let mut surface = surface();
        surface.pan_zoom.actual_size();

        let Primitive::Texture(texture) = surface.texture_primitive_at(ScaleFactor::new(2.0))
        else {
            panic!("expected texture primitive");
        };

        assert_approx(texture.rect.width, 200.0);
        assert_approx(texture.rect.height, 100.0);
    }

    #[test]
    fn ruler_ticks_change_with_zoom() {
        assert!(ruler_ticks(0.0, 100.0, 2.0).len() > ruler_ticks(0.0, 100.0, 0.5).len());
    }

    #[test]
    fn ruler_ticks_handle_reversed_and_invalid_ranges() {
        assert_eq!(ruler_ticks(100.0, 0.0, 1.0), ruler_ticks(0.0, 100.0, 1.0));
        assert!(ruler_ticks(0.0, f32::NAN, 1.0).is_empty());
        assert!(ruler_ticks(0.0, 100.0, f32::NAN).is_empty());
        assert!(ruler_ticks(0.0, 1_000_000.0, 2.0).len() <= 4097);
    }

    #[test]
    fn guide_primitives_emit_lines() {
        let primitives = guide_primitives(
            Rect::new(0.0, 0.0, 100.0, 100.0),
            &[Guide::Horizontal(50.0), Guide::Vertical(25.0)],
            Color::WHITE,
        );

        assert_eq!(primitives.len(), 2);
        assert!(matches!(primitives[0], Primitive::Line(_)));
    }

    #[test]
    fn crosshair_emits_lines_and_label_inside_bounds() {
        let crosshair = Crosshair {
            visible: true,
            position: Point::new(50.0, 50.0),
            label: Some("50,50".to_owned()),
            color: Color::WHITE,
        };

        let primitives = crosshair.primitives(Rect::new(0.0, 0.0, 100.0, 100.0));

        assert_eq!(primitives.len(), 3);
    }

    #[test]
    fn surface_content_overlays_transform_to_screen_space() {
        let surface = surface();
        let guide = surface.content_guide_primitives(&[Guide::Vertical(200.0)], Color::WHITE);
        let crosshair = Crosshair {
            visible: true,
            position: Point::new(200.0, 100.0),
            label: None,
            color: Color::WHITE,
        };
        let crosshair_primitives = surface.content_crosshair_primitives(&crosshair);

        let Primitive::Line(line) = &guide[0] else {
            panic!("expected guide line");
        };
        assert_approx(line.from.x, 100.0);
        assert_approx(line.from.y, 50.0);
        assert_approx(line.to.y, 150.0);

        let Primitive::Line(horizontal) = &crosshair_primitives[0] else {
            panic!("expected crosshair horizontal line");
        };
        assert_approx(horizontal.from.y, 100.0);
        assert_approx(horizontal.to.y, 100.0);
    }

    #[test]
    fn scale_aware_content_overlays_share_texture_rect() {
        let mut surface = surface();
        surface.bounds = Rect::new(0.25, 0.25, 201.0, 201.0);
        surface.pan_zoom.actual_size();

        let Primitive::Texture(texture) = surface.texture_primitive_at(ScaleFactor::new(1.5))
        else {
            panic!("expected texture primitive");
        };
        let guide = surface.content_guide_primitives_at(
            &[Guide::Vertical(200.0)],
            Color::WHITE,
            ScaleFactor::new(1.5),
        );
        let Primitive::Line(line) = &guide[0] else {
            panic!("expected guide line");
        };

        assert_approx(line.from.y, texture.rect.y);
        assert_approx(line.to.y, texture.rect.max_y());
        assert!(line.from.x >= texture.rect.x);
        assert!(line.from.x <= texture.rect.max_x());
    }

    #[test]
    fn scale_aware_content_rect_overlays_snap_to_physical_pixels() {
        let mut surface = surface();
        surface.bounds = Rect::new(0.25, 0.25, 201.0, 201.0);
        surface.pan_zoom.actual_size();
        let scale_factor = ScaleFactor::new(1.25);
        let content_rect = surface.content_rect_at(scale_factor);
        let content_scale = surface.content_scale_at(scale_factor);
        let content_overlay = Rect::new(23.0, 17.0, 41.0, 19.0);
        let expected = scale_factor.snap_rect_to_physical_grid(Rect::new(
            content_rect.x + content_overlay.x * content_scale,
            content_rect.y + content_overlay.y * content_scale,
            content_overlay.width * content_scale,
            content_overlay.height * content_scale,
        ));

        let overlay = surface
            .content_rect_to_screen_at(content_overlay, scale_factor)
            .expect("overlay rect");

        assert_rect_close(overlay, expected);
        assert_rect_edges_aligned(overlay, scale_factor);
    }

    #[test]
    fn scale_aware_guides_and_crosshair_align_with_snapped_texture_rect() {
        let mut surface = surface();
        surface.bounds = Rect::new(0.25, 0.25, 201.0, 201.0);
        surface.pan_zoom.actual_size();
        let scale_factor = ScaleFactor::new(1.5);
        let content_scale = expected_content_scale_at(surface, 1.0 / 1.5);
        let content_rect = expected_snapped_content_rect(surface, scale_factor, content_scale);
        let expected_horizontal_y = content_rect.y + 100.0 * content_scale;
        let expected_vertical_x = content_rect.x + 200.0 * content_scale;
        let viewport_bounds = surface.bounds;

        let guides = surface.content_guide_primitives_at(
            &[Guide::Horizontal(100.0), Guide::Vertical(200.0)],
            Color::WHITE,
            scale_factor,
        );
        assert_eq!(guides.len(), 2);

        let Primitive::Line(horizontal_guide) = &guides[0] else {
            panic!("expected horizontal guide");
        };
        assert_close(horizontal_guide.from.x, content_rect.x);
        assert_close(horizontal_guide.to.x, content_rect.max_x());
        assert_close(horizontal_guide.from.y, expected_horizontal_y);
        assert_close(horizontal_guide.to.y, expected_horizontal_y);
        assert!(horizontal_guide.from.y >= content_rect.y);
        assert!(horizontal_guide.from.y <= content_rect.max_y());
        assert_edge_aligned(horizontal_guide.from.y, scale_factor);

        let Primitive::Line(vertical_guide) = &guides[1] else {
            panic!("expected vertical guide");
        };
        assert_close(vertical_guide.from.y, content_rect.y);
        assert_close(vertical_guide.to.y, content_rect.max_y());
        assert_close(vertical_guide.from.x, expected_vertical_x);
        assert_close(vertical_guide.to.x, expected_vertical_x);
        assert!(vertical_guide.from.x >= content_rect.x);
        assert!(vertical_guide.from.x <= content_rect.max_x());
        assert_edge_aligned(vertical_guide.from.x, scale_factor);

        let crosshair = Crosshair {
            visible: true,
            position: Point::new(200.0, 100.0),
            label: None,
            color: Color::WHITE,
        };
        let crosshair_primitives =
            surface.content_crosshair_primitives_at(&crosshair, scale_factor);
        assert_eq!(crosshair_primitives.len(), 2);
        let Primitive::Line(horizontal_crosshair) = &crosshair_primitives[0] else {
            panic!("expected crosshair horizontal line");
        };
        let Primitive::Line(vertical_crosshair) = &crosshair_primitives[1] else {
            panic!("expected crosshair vertical line");
        };
        let expected_crosshair_screen =
            expected_screen_point(content_rect, content_scale, crosshair.position);

        assert_close(horizontal_crosshair.from.x, viewport_bounds.x);
        assert_close(horizontal_crosshair.to.x, viewport_bounds.max_x());
        assert_close(horizontal_crosshair.from.y, expected_crosshair_screen.y);
        assert_close(horizontal_crosshair.to.y, expected_crosshair_screen.y);
        assert_close(vertical_crosshair.from.x, expected_crosshair_screen.x);
        assert_close(vertical_crosshair.to.x, expected_crosshair_screen.x);
        assert_close(vertical_crosshair.from.y, viewport_bounds.y);
        assert_close(vertical_crosshair.to.y, viewport_bounds.max_y());
        assert!(horizontal_crosshair.from.y >= content_rect.y);
        assert!(horizontal_crosshair.from.y <= content_rect.max_y());
        assert!(vertical_crosshair.from.x >= content_rect.x);
        assert!(vertical_crosshair.from.x <= content_rect.max_x());
        assert_edge_aligned(horizontal_crosshair.from.y, scale_factor);
        assert_edge_aligned(vertical_crosshair.from.x, scale_factor);
    }

    #[test]
    fn invalid_scale_factor_preserves_viewport_rect_behavior() {
        let mut surface = surface();
        surface.bounds = Rect::new(0.25, 0.25, 201.0, 201.0);
        surface.pan_zoom.actual_size();
        let invalid_scale = ScaleFactor::new(0.0);

        let rect = surface.content_rect_at(invalid_scale);
        let overlay = surface
            .content_rect_to_screen_at(Rect::new(20.0, 10.0, 40.0, 20.0), invalid_scale)
            .expect("overlay rect");

        assert_rect_close(rect, unsnapped_content_rect(surface, invalid_scale));
        assert!(rect.width > 0.0);
        assert!(rect.height > 0.0);
        assert!(overlay.width > 0.0);
        assert!(overlay.height > 0.0);
    }

    #[test]
    fn composition_orders_clip_texture_guides_crosshair() {
        let composition = ViewportComposition {
            surface: surface(),
            guides: vec![Guide::Horizontal(50.0)],
            crosshair: Some(Crosshair {
                visible: true,
                position: Point::new(50.0, 50.0),
                label: None,
                color: Color::WHITE,
            }),
            clip: ClipId::from_raw(1),
        };
        let primitives = composition.primitives();

        assert!(matches!(primitives[0], Primitive::ClipBegin { .. }));
        assert!(matches!(primitives[1], Primitive::Texture(_)));
        assert!(matches!(primitives.last(), Some(Primitive::ClipEnd { .. })));
    }
}
