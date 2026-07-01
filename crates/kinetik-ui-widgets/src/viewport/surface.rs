#[allow(clippy::wildcard_imports)]
use super::*;

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
