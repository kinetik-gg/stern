//! CPU preview rasterizer for showcase primitives.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]

use std::fs::File;
use std::io::{self, Write};
use std::path::Path;

use ab_glyph::{Font, FontArc, Glyph, PxScale, ScaleFont, point};
use kinetik_ui::core::{
    Brush, Color, LinePrimitive, LinearGradient, PathElement, PathPrimitive, Point, Primitive,
    Rect, RectPrimitive, ShadowPrimitive, Stroke, TextPrimitive, TexturePrimitive,
};

/// RGBA-like packed pixel in `0x00RRGGBB` form for minifb.
pub type Pixel = u32;

/// Rasterized preview frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RasterFrame {
    /// Frame width in pixels.
    pub width: usize,
    /// Frame height in pixels.
    pub height: usize,
    /// Packed pixels.
    pub pixels: Vec<Pixel>,
}

impl RasterFrame {
    /// Creates a blank frame.
    #[must_use]
    pub fn new(width: usize, height: usize, color: Pixel) -> Self {
        Self {
            width,
            height,
            pixels: vec![color; width.saturating_mul(height)],
        }
    }

    /// Returns true when the frame contains more than one unique pixel value.
    #[must_use]
    pub fn has_visible_variation(&self) -> bool {
        self.pixels
            .first()
            .is_some_and(|first| self.pixels.iter().any(|pixel| pixel != first))
    }

    /// Counts pixels that match the provided color.
    #[must_use]
    pub fn count_color(&self, color: Pixel) -> usize {
        self.pixels.iter().filter(|pixel| **pixel == color).count()
    }

    /// Returns true when the frame has at least the requested number of unique colors.
    #[must_use]
    pub fn unique_color_count_at_least(&self, expected: usize) -> bool {
        let mut colors = Vec::new();
        for pixel in &self.pixels {
            if !colors.contains(pixel) {
                colors.push(*pixel);
                if colors.len() >= expected {
                    return true;
                }
            }
        }
        false
    }
}

/// Rasterizes primitives into a preview frame.
#[must_use]
pub fn rasterize(primitives: &[Primitive], width: usize, height: usize) -> RasterFrame {
    let mut target = RasterTarget::new(width, height, rgb(14, 14, 14));
    target.draw(primitives);
    target.frame
}

/// Writes a raster frame as a 24-bit BMP image.
///
/// # Errors
///
/// Returns an I/O error when the destination cannot be written.
pub fn write_bmp(frame: &RasterFrame, path: impl AsRef<Path>) -> io::Result<()> {
    let row_stride = (frame.width * 3).div_ceil(4) * 4;
    let pixel_data_size = row_stride * frame.height;
    let file_size = 14 + 40 + pixel_data_size;
    let mut file = File::create(path)?;

    file.write_all(b"BM")?;
    write_u32(&mut file, usize_to_u32(file_size))?;
    write_u16(&mut file, 0)?;
    write_u16(&mut file, 0)?;
    write_u32(&mut file, 54)?;
    write_u32(&mut file, 40)?;
    write_i32(&mut file, usize_to_i32(frame.width))?;
    write_i32(&mut file, usize_to_i32(frame.height))?;
    write_u16(&mut file, 1)?;
    write_u16(&mut file, 24)?;
    write_u32(&mut file, 0)?;
    write_u32(&mut file, usize_to_u32(pixel_data_size))?;
    write_i32(&mut file, 2835)?;
    write_i32(&mut file, 2835)?;
    write_u32(&mut file, 0)?;
    write_u32(&mut file, 0)?;

    let padding = vec![0; row_stride - frame.width * 3];
    for y in (0..frame.height).rev() {
        for x in 0..frame.width {
            let pixel = frame.pixels[y * frame.width + x];
            file.write_all(&[
                (pixel & 0xff) as u8,
                ((pixel >> 8) & 0xff) as u8,
                ((pixel >> 16) & 0xff) as u8,
            ])?;
        }
        file.write_all(&padding)?;
    }

    Ok(())
}

fn write_u16(writer: &mut impl Write, value: u16) -> io::Result<()> {
    writer.write_all(&value.to_le_bytes())
}

fn write_u32(writer: &mut impl Write, value: u32) -> io::Result<()> {
    writer.write_all(&value.to_le_bytes())
}

fn write_i32(writer: &mut impl Write, value: i32) -> io::Result<()> {
    writer.write_all(&value.to_le_bytes())
}

fn usize_to_u32(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

fn usize_to_i32(value: usize) -> i32 {
    i32::try_from(value).unwrap_or(i32::MAX)
}

struct RasterTarget {
    frame: RasterFrame,
    clip: Option<Rect>,
    font: Option<FontArc>,
}

impl RasterTarget {
    fn new(width: usize, height: usize, color: Pixel) -> Self {
        Self {
            frame: RasterFrame::new(width, height, color),
            clip: None,
            font: load_system_font(),
        }
    }

    fn draw(&mut self, primitives: &[Primitive]) {
        for primitive in primitives {
            match primitive {
                Primitive::Rect(rect) => self.rect(rect),
                Primitive::Line(line) => self.line(line),
                Primitive::Shadow(shadow) => self.shadow(shadow),
                Primitive::Path(path) => self.path(path),
                Primitive::Text(text) => self.text(text),
                Primitive::Image(image) => self.placeholder(image.rect, rgb(80, 80, 84)),
                Primitive::Texture(texture) => self.texture(texture),
                Primitive::ClipBegin { rect, .. } => self.clip = Some(*rect),
                Primitive::ClipEnd { .. } => self.clip = None,
                Primitive::LayerBegin { .. }
                | Primitive::LayerEnd { .. }
                | Primitive::TransformBegin(_)
                | Primitive::TransformEnd => {}
            }
        }
    }

    fn rect(&mut self, primitive: &RectPrimitive) {
        if let Some(fill) = primitive.fill {
            self.fill_rect_brush(primitive.rect, fill);
        }
        if let Some(stroke) = primitive.stroke {
            let width = stroke.width.max(1.0);
            self.fill_rect_brush(
                Rect::new(
                    primitive.rect.x,
                    primitive.rect.y,
                    primitive.rect.width,
                    width,
                ),
                stroke.brush,
            );
            self.fill_rect_brush(
                Rect::new(
                    primitive.rect.x,
                    primitive.rect.max_y() - width,
                    primitive.rect.width,
                    width,
                ),
                stroke.brush,
            );
            self.fill_rect_brush(
                Rect::new(
                    primitive.rect.x,
                    primitive.rect.y,
                    width,
                    primitive.rect.height,
                ),
                stroke.brush,
            );
            self.fill_rect_brush(
                Rect::new(
                    primitive.rect.max_x() - width,
                    primitive.rect.y,
                    width,
                    primitive.rect.height,
                ),
                stroke.brush,
            );
        }
    }

    fn line(&mut self, primitive: &LinePrimitive) {
        let brush = primitive.stroke.brush;
        let mut x0 = primitive.from.x.round() as i32;
        let mut y0 = primitive.from.y.round() as i32;
        let x1 = primitive.to.x.round() as i32;
        let y1 = primitive.to.y.round() as i32;
        let dx = (x1 - x0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let dy = -(y1 - y0).abs();
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;

        loop {
            self.put(
                x0,
                y0,
                pixel_from_brush_at(brush, Point::new(x0 as f32, y0 as f32)),
            );
            if x0 == x1 && y0 == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x0 += sx;
            }
            if e2 <= dx {
                err += dx;
                y0 += sy;
            }
        }
    }

    fn shadow(&mut self, primitive: &ShadowPrimitive) {
        let blur_extent = primitive.blur_radius.max(0.0) * 2.5;
        let rect = primitive
            .rect
            .translate(primitive.offset)
            .outset(primitive.spread + blur_extent)
            .max_zero();
        self.fill_rect(rect, pixel_from_color(primitive.color));
    }

    fn path(&mut self, primitive: &PathPrimitive) {
        let stroke = primitive
            .stroke
            .or_else(|| primitive.fill.map(|fill| Stroke::new(1.0, fill)));
        let Some(stroke) = stroke else {
            return;
        };

        let mut start = None;
        let mut current = None;
        for element in &primitive.elements {
            match *element {
                PathElement::MoveTo(point) => {
                    start = Some(point);
                    current = Some(point);
                }
                PathElement::LineTo(point) => {
                    if let Some(from) = current {
                        self.line(&LinePrimitive {
                            from,
                            to: point,
                            stroke,
                        });
                    }
                    current = Some(point);
                }
                PathElement::QuadTo { ctrl, to } => {
                    if let Some(from) = current {
                        self.sample_quadratic(from, ctrl, to, stroke);
                    }
                    current = Some(to);
                }
                PathElement::CubicTo { ctrl1, ctrl2, to } => {
                    if let Some(from) = current {
                        self.sample_cubic(from, ctrl1, ctrl2, to, stroke);
                    }
                    current = Some(to);
                }
                PathElement::Close => {
                    if let (Some(from), Some(to)) = (current, start) {
                        self.line(&LinePrimitive { from, to, stroke });
                    }
                    current = start;
                }
            }
        }
    }

    fn sample_quadratic(&mut self, from: Point, ctrl: Point, to: Point, stroke: Stroke) {
        let mut previous = from;
        for step in 1..=16 {
            let t = step as f32 / 16.0;
            let inv = 1.0 - t;
            let point = Point::new(
                inv * inv * from.x + 2.0 * inv * t * ctrl.x + t * t * to.x,
                inv * inv * from.y + 2.0 * inv * t * ctrl.y + t * t * to.y,
            );
            self.line(&LinePrimitive {
                from: previous,
                to: point,
                stroke,
            });
            previous = point;
        }
    }

    fn sample_cubic(&mut self, from: Point, ctrl1: Point, ctrl2: Point, to: Point, stroke: Stroke) {
        let mut previous = from;
        for step in 1..=24 {
            let t = step as f32 / 24.0;
            let inv = 1.0 - t;
            let point = Point::new(
                inv * inv * inv * from.x
                    + 3.0 * inv * inv * t * ctrl1.x
                    + 3.0 * inv * t * t * ctrl2.x
                    + t * t * t * to.x,
                inv * inv * inv * from.y
                    + 3.0 * inv * inv * t * ctrl1.y
                    + 3.0 * inv * t * t * ctrl2.y
                    + t * t * t * to.y,
            );
            self.line(&LinePrimitive {
                from: previous,
                to: point,
                stroke,
            });
            previous = point;
        }
    }

    fn text(&mut self, primitive: &TextPrimitive) {
        let color = pixel_from_brush(primitive.brush);
        if let Some(font) = self.font.clone() {
            self.font_text(&font, primitive, color);
            return;
        }

        let scale = (primitive.size / 9.0).round().clamp(1.0, 3.0) as i32;
        let mut x = primitive.origin.x.round() as i32;
        let y = (primitive.origin.y - primitive.size).round() as i32;
        for character in primitive.text.chars() {
            if character == ' ' {
                x += 4 * scale;
                continue;
            }
            self.glyph(x, y, scale, character, color);
            x += 6 * scale;
        }
    }

    fn font_text(&mut self, font: &FontArc, primitive: &TextPrimitive, color: Pixel) {
        let scale = PxScale::from(primitive.size.max(1.0));
        let scaled = font.as_scaled(scale);
        let mut caret = primitive.origin.x;
        let baseline = primitive.origin.y;
        let mut previous = None;

        for character in primitive.text.chars() {
            if character == '\n' {
                caret = primitive.origin.x;
                previous = None;
                continue;
            }

            let glyph_id = scaled.glyph_id(character);
            if let Some(previous_id) = previous {
                caret += scaled.kern(previous_id, glyph_id);
            }

            let glyph = Glyph {
                id: glyph_id,
                scale,
                position: point(caret, baseline),
            };
            if let Some(outlined) = font.outline_glyph(glyph) {
                let bounds = outlined.px_bounds();
                outlined.draw(|x, y, coverage| {
                    self.blend(
                        bounds.min.x.floor() as i32 + i32::try_from(x).unwrap_or(0),
                        bounds.min.y.floor() as i32 + i32::try_from(y).unwrap_or(0),
                        color,
                        coverage,
                    );
                });
            }

            caret += scaled.h_advance(glyph_id);
            previous = Some(glyph_id);
        }
    }

    fn texture(&mut self, primitive: &TexturePrimitive) {
        self.fill_rect(primitive.rect, rgb(18, 22, 28));
        let cols = 12;
        let rows = 8;
        let cell_width = primitive.rect.width / cols as f32;
        let cell_height = primitive.rect.height / rows as f32;

        for row in 0..rows {
            for col in 0..cols {
                let shade = if (row + col) % 2 == 0 {
                    rgb(28, 34, 42)
                } else {
                    rgb(22, 27, 34)
                };
                self.fill_rect(
                    Rect::new(
                        primitive.rect.x + col as f32 * cell_width,
                        primitive.rect.y + row as f32 * cell_height,
                        cell_width.ceil(),
                        cell_height.ceil(),
                    ),
                    shade,
                );
            }
        }

        self.fill_rect(
            Rect::new(
                primitive.rect.x,
                primitive.rect.center().y,
                primitive.rect.width,
                1.0,
            ),
            rgb(96, 120, 150),
        );
        self.fill_rect(
            Rect::new(
                primitive.rect.center().x,
                primitive.rect.y,
                1.0,
                primitive.rect.height,
            ),
            rgb(96, 120, 150),
        );
    }

    fn placeholder(&mut self, rect: Rect, color: Pixel) {
        self.fill_rect(rect, color);
        self.fill_rect(
            Rect::new(rect.x + 4.0, rect.y + 4.0, rect.width - 8.0, 1.0),
            rgb(120, 120, 124),
        );
    }

    fn glyph(&mut self, x: i32, y: i32, scale: i32, character: char, color: Pixel) {
        let pattern = glyph_pattern(character);
        for (row, bits) in pattern.iter().enumerate() {
            for col in 0..5 {
                if bits & (1 << (4 - col)) != 0 {
                    self.fill_rect(
                        Rect::new(
                            (x + col * scale) as f32,
                            (y + i32::try_from(row).unwrap_or(0) * scale) as f32,
                            scale as f32,
                            scale as f32,
                        ),
                        color,
                    );
                }
            }
        }
    }

    fn fill_rect(&mut self, rect: Rect, color: Pixel) {
        let rect = self.clipped(rect);
        if rect.is_empty() {
            return;
        }
        let x0 = rect.x.floor().max(0.0) as i32;
        let y0 = rect.y.floor().max(0.0) as i32;
        let x1 = rect.max_x().ceil().min(self.frame.width as f32) as i32;
        let y1 = rect.max_y().ceil().min(self.frame.height as f32) as i32;
        for y in y0..y1 {
            for x in x0..x1 {
                self.put(x, y, color);
            }
        }
    }

    fn fill_rect_brush(&mut self, rect: Rect, brush: Brush) {
        match brush {
            Brush::Solid(color) => self.fill_rect(rect, pixel_from_color(color)),
            Brush::LinearGradient(_) => {
                let rect = self.clipped(rect);
                if rect.is_empty() {
                    return;
                }
                let x0 = rect.x.floor().max(0.0) as i32;
                let y0 = rect.y.floor().max(0.0) as i32;
                let x1 = rect.max_x().ceil().min(self.frame.width as f32) as i32;
                let y1 = rect.max_y().ceil().min(self.frame.height as f32) as i32;
                for y in y0..y1 {
                    for x in x0..x1 {
                        self.put(
                            x,
                            y,
                            pixel_from_brush_at(brush, Point::new(x as f32, y as f32)),
                        );
                    }
                }
            }
        }
    }

    fn clipped(&self, rect: Rect) -> Rect {
        let Some(clip) = self.clip else {
            return rect;
        };
        let x0 = rect.x.max(clip.x);
        let y0 = rect.y.max(clip.y);
        let x1 = rect.max_x().min(clip.max_x());
        let y1 = rect.max_y().min(clip.max_y());
        Rect::new(x0, y0, (x1 - x0).max(0.0), (y1 - y0).max(0.0))
    }

    fn put(&mut self, x: i32, y: i32, color: Pixel) {
        if x < 0 || y < 0 {
            return;
        }
        let x = usize::try_from(x).unwrap_or(usize::MAX);
        let y = usize::try_from(y).unwrap_or(usize::MAX);
        if x >= self.frame.width || y >= self.frame.height {
            return;
        }
        let point = Point::new(x as f32, y as f32);
        if self.clip.is_some_and(|clip| !clip.contains_point(point)) {
            return;
        }
        self.frame.pixels[y * self.frame.width + x] = color;
    }

    fn blend(&mut self, x: i32, y: i32, color: Pixel, alpha: f32) {
        if x < 0 || y < 0 || alpha <= 0.0 {
            return;
        }
        if alpha >= 1.0 {
            self.put(x, y, color);
            return;
        }
        let x = usize::try_from(x).unwrap_or(usize::MAX);
        let y = usize::try_from(y).unwrap_or(usize::MAX);
        if x >= self.frame.width || y >= self.frame.height {
            return;
        }
        let point = Point::new(x as f32, y as f32);
        if self.clip.is_some_and(|clip| !clip.contains_point(point)) {
            return;
        }

        let index = y * self.frame.width + x;
        self.frame.pixels[index] = blend_pixel(self.frame.pixels[index], color, alpha);
    }
}

/// Creates a packed RGB pixel.
#[must_use]
pub const fn rgb(red: u8, green: u8, blue: u8) -> Pixel {
    ((red as u32) << 16) | ((green as u32) << 8) | (blue as u32)
}

fn pixel_from_brush(brush: Brush) -> Pixel {
    pixel_from_brush_at(brush, Point::new(0.0, 0.0))
}

fn pixel_from_brush_at(brush: Brush, point: Point) -> Pixel {
    match brush {
        Brush::Solid(color) => pixel_from_color(color),
        Brush::LinearGradient(gradient) => {
            pixel_from_color(sample_linear_gradient(gradient, point))
        }
    }
}

fn sample_linear_gradient(gradient: LinearGradient, point: Point) -> Color {
    let start = gradient.start();
    let end = gradient.end();
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let len_sq = dx.mul_add(dx, dy * dy);
    if !len_sq.is_finite() || len_sq <= f32::EPSILON {
        return gradient
            .stops()
            .first()
            .map_or(Color::TRANSPARENT, |stop| stop.color);
    }

    let raw_t = ((point.x - start.x) * dx + (point.y - start.y) * dy) / len_sq;
    let t = raw_t.clamp(0.0, 1.0);
    let stops = gradient.stops();
    for pair in stops.windows(2) {
        let [from, to] = pair else {
            continue;
        };
        if t <= to.offset {
            let span = (to.offset - from.offset).max(f32::EPSILON);
            let local_t = ((t - from.offset) / span).clamp(0.0, 1.0);
            return lerp_color(from.color, to.color, local_t);
        }
    }
    stops.last().map_or(Color::TRANSPARENT, |stop| stop.color)
}

fn lerp_color(from: Color, to: Color, t: f32) -> Color {
    Color::rgba(
        from.r + (to.r - from.r) * t,
        from.g + (to.g - from.g) * t,
        from.b + (to.b - from.b) * t,
        from.a + (to.a - from.a) * t,
    )
}

fn pixel_from_color(color: Color) -> Pixel {
    let red = (color.r.clamp(0.0, 1.0) * 255.0).round() as u8;
    let green = (color.g.clamp(0.0, 1.0) * 255.0).round() as u8;
    let blue = (color.b.clamp(0.0, 1.0) * 255.0).round() as u8;
    rgb(red, green, blue)
}

fn blend_pixel(destination: Pixel, source: Pixel, alpha: f32) -> Pixel {
    let alpha = alpha.clamp(0.0, 1.0);
    let inv_alpha = 1.0 - alpha;
    let red = channel(destination, 16) * inv_alpha + channel(source, 16) * alpha;
    let green = channel(destination, 8) * inv_alpha + channel(source, 8) * alpha;
    let blue = channel(destination, 0) * inv_alpha + channel(source, 0) * alpha;

    rgb(red.round() as u8, green.round() as u8, blue.round() as u8)
}

fn channel(pixel: Pixel, shift: u8) -> f32 {
    ((pixel >> shift) & 0xff) as f32
}

fn load_system_font() -> Option<FontArc> {
    for path in system_font_candidates() {
        let Ok(bytes) = std::fs::read(path) else {
            continue;
        };
        if let Ok(font) = FontArc::try_from_vec(bytes) {
            return Some(font);
        }
    }

    None
}

fn system_font_candidates() -> &'static [&'static str] {
    &[
        "C:\\Windows\\Fonts\\segoeui.ttf",
        "C:\\Windows\\Fonts\\arial.ttf",
        "/System/Library/Fonts/SFNS.ttf",
        "/System/Library/Fonts/Supplemental/Arial.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        "/usr/share/fonts/truetype/liberation2/LiberationSans-Regular.ttf",
    ]
}

#[allow(clippy::too_many_lines)]
fn glyph_pattern(character: char) -> [u8; 7] {
    match character.to_ascii_uppercase() {
        'A' => [
            0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
        ],
        'B' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110,
        ],
        'C' => [
            0b01111, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b01111,
        ],
        'D' => [
            0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110,
        ],
        'E' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111,
        ],
        'F' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000,
        ],
        'G' => [
            0b01111, 0b10000, 0b10000, 0b10111, 0b10001, 0b10001, 0b01110,
        ],
        'H' => [
            0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
        ],
        'I' => [
            0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b11111,
        ],
        'J' => [
            0b00111, 0b00010, 0b00010, 0b00010, 0b10010, 0b10010, 0b01100,
        ],
        'K' => [
            0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001,
        ],
        'L' => [
            0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111,
        ],
        'M' => [
            0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001,
        ],
        'N' => [
            0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001, 0b10001,
        ],
        'O' => [
            0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ],
        'P' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000,
        ],
        'Q' => [
            0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10010, 0b01101,
        ],
        'R' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001,
        ],
        'S' => [
            0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110,
        ],
        'T' => [
            0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100,
        ],
        'U' => [
            0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ],
        'V' => [
            0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100,
        ],
        'W' => [
            0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b10101, 0b01010,
        ],
        'X' => [
            0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001,
        ],
        'Y' => [
            0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100,
        ],
        'Z' => [
            0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111,
        ],
        '0' => [
            0b01110, 0b10011, 0b10101, 0b10101, 0b11001, 0b10001, 0b01110,
        ],
        '1' => [
            0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
        ],
        '2' => [
            0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111,
        ],
        '3' => [
            0b11110, 0b00001, 0b00001, 0b01110, 0b00001, 0b00001, 0b11110,
        ],
        '4' => [
            0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010,
        ],
        '5' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b00001, 0b00001, 0b11110,
        ],
        '6' => [
            0b01110, 0b10000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110,
        ],
        '7' => [
            0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000,
        ],
        '8' => [
            0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110,
        ],
        '9' => [
            0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00001, 0b01110,
        ],
        '-' => [0, 0, 0, 0b11111, 0, 0, 0],
        '_' => [0, 0, 0, 0, 0, 0, 0b11111],
        '.' => [0, 0, 0, 0, 0, 0b01100, 0b01100],
        ',' => [0, 0, 0, 0, 0, 0b01100, 0b01000],
        ':' => [0, 0b01100, 0b01100, 0, 0b01100, 0b01100, 0],
        '/' => [
            0b00001, 0b00010, 0b00010, 0b00100, 0b01000, 0b01000, 0b10000,
        ],
        '%' => [0b11001, 0b11010, 0b00100, 0b01000, 0b10110, 0b00110, 0],
        '>' => [
            0b10000, 0b01000, 0b00100, 0b00010, 0b00100, 0b01000, 0b10000,
        ],
        '[' => [
            0b01110, 0b01000, 0b01000, 0b01000, 0b01000, 0b01000, 0b01110,
        ],
        ']' => [
            0b01110, 0b00010, 0b00010, 0b00010, 0b00010, 0b00010, 0b01110,
        ],
        _ => [0b11111, 0b10001, 0b00010, 0b00100, 0b00100, 0, 0b00100],
    }
}

#[cfg(test)]
mod tests {
    use super::{rasterize, rgb};
    use crate::app::{ShowcaseApp, ShowcasePage};

    #[test]
    fn rasterized_showcase_is_not_blank() {
        let app = ShowcaseApp::new();
        let frame = rasterize(&app.primitives(), 1440, 900);

        assert!(frame.has_visible_variation());
        assert!(frame.unique_color_count_at_least(8));
        assert!(
            frame
                .pixels
                .iter()
                .filter(|pixel| **pixel != rgb(12, 12, 13))
                .count()
                > 50_000
        );
    }

    #[test]
    fn raster_frame_writes_bmp() {
        let app = ShowcaseApp::new();
        let frame = rasterize(&app.primitives(), 320, 200);
        let path = std::env::temp_dir().join("kinetik-ui-showcase-smoke.bmp");

        super::write_bmp(&frame, &path).expect("write bmp");

        assert!(std::fs::metadata(path).expect("metadata").len() > 54);
    }

    #[test]
    fn every_showcase_page_rasterizes_with_visible_content() {
        let mut app = ShowcaseApp::new();

        for page in [
            ShowcasePage::Components,
            ShowcasePage::Layout,
            ShowcasePage::Viewport,
            ShowcasePage::Systems,
        ] {
            app.set_page(page);
            let frame = rasterize(&app.primitives(), 1440, 900);

            assert!(frame.has_visible_variation(), "{page:?}");
            assert!(frame.unique_color_count_at_least(8), "{page:?}");
        }
    }
}
