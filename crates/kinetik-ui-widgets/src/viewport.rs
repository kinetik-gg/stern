//! Viewport texture surfaces and editor overlay primitives.

use kinetik_ui_core::{
    Brush, ClipId, Color, LinePrimitive, Point, Primitive, Rect, ScaleFactor, Size, Stroke,
    TextPrimitive, TextureId, TexturePrimitive, Vec2,
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

#[allow(clippy::cast_possible_truncation)]
fn snap_rect_to_scale(rect: Rect, scale_factor: ScaleFactor) -> Rect {
    if !rect.x.is_finite()
        || !rect.y.is_finite()
        || !rect.width.is_finite()
        || !rect.height.is_finite()
        || !scale_factor.is_valid()
    {
        return rect;
    }
    let scale = scale_factor.value();
    let left = (f64::from(rect.min_x()) * scale).round() / scale;
    let top = (f64::from(rect.min_y()) * scale).round() / scale;
    let right = (f64::from(rect.max_x()) * scale).round() / scale;
    let bottom = (f64::from(rect.max_y()) * scale).round() / scale;
    Rect::new(
        left as f32,
        top as f32,
        (right - left) as f32,
        (bottom - top) as f32,
    )
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

    fn surface() -> ViewportSurface {
        ViewportSurface {
            texture: TextureId::from_raw(1),
            source_size: Size::new(400.0, 200.0),
            bounds: Rect::new(0.0, 0.0, 200.0, 200.0),
            pan_zoom: PanZoom::default(),
        }
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
        let rect = surface.content_rect_at(ScaleFactor::new(2.0));

        assert_approx(surface.content_scale_at(ScaleFactor::new(2.0)), 0.5);
        assert_approx(rect.width, 200.0);
        assert_approx(rect.height, 100.0);
        assert_approx(rect.x, 0.0);
        assert_approx(rect.y, 50.0);
        assert_approx(rect.width * 2.0, 400.0);
        assert_approx(rect.height * 2.0, 200.0);
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

        let overlay = surface
            .content_rect_to_screen_at(Rect::new(23.0, 17.0, 41.0, 19.0), scale_factor)
            .expect("overlay rect");

        for value in [
            overlay.x,
            overlay.y,
            overlay.width,
            overlay.height,
            overlay.max_x(),
            overlay.max_y(),
        ] {
            let physical = f64::from(value) * scale_factor.value();
            assert!(
                (physical - physical.round()).abs() <= 0.001,
                "{value} -> {physical}"
            );
        }
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
