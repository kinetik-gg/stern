//! CPU preview rasterizer for showcase primitives.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]

use std::fs::File;
use std::io::{self, Write};
use std::path::Path;

use kinetik_ui_core::{
    Brush, Color, LinePrimitive, Point, Primitive, Rect, RectPrimitive, TextPrimitive,
    TexturePrimitive,
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
}

impl RasterTarget {
    fn new(width: usize, height: usize, color: Pixel) -> Self {
        Self {
            frame: RasterFrame::new(width, height, color),
            clip: None,
        }
    }

    fn draw(&mut self, primitives: &[Primitive]) {
        for primitive in primitives {
            match primitive {
                Primitive::Rect(rect) => self.rect(rect),
                Primitive::Line(line) => self.line(line),
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
            self.fill_rect(primitive.rect, pixel_from_brush(fill));
        }
        if let Some(stroke) = primitive.stroke {
            let color = pixel_from_brush(stroke.brush);
            let width = stroke.width.max(1.0);
            self.fill_rect(
                Rect::new(
                    primitive.rect.x,
                    primitive.rect.y,
                    primitive.rect.width,
                    width,
                ),
                color,
            );
            self.fill_rect(
                Rect::new(
                    primitive.rect.x,
                    primitive.rect.max_y() - width,
                    primitive.rect.width,
                    width,
                ),
                color,
            );
            self.fill_rect(
                Rect::new(
                    primitive.rect.x,
                    primitive.rect.y,
                    width,
                    primitive.rect.height,
                ),
                color,
            );
            self.fill_rect(
                Rect::new(
                    primitive.rect.max_x() - width,
                    primitive.rect.y,
                    width,
                    primitive.rect.height,
                ),
                color,
            );
        }
    }

    fn line(&mut self, primitive: &LinePrimitive) {
        let color = pixel_from_brush(primitive.stroke.brush);
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
            self.put(x0, y0, color);
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

    fn text(&mut self, primitive: &TextPrimitive) {
        let color = pixel_from_brush(primitive.brush);
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

    fn texture(&mut self, primitive: &TexturePrimitive) {
        self.fill_rect(primitive.rect, rgb(166, 174, 141));
        let face = Rect::new(
            primitive.rect.x + primitive.rect.width * 0.34,
            primitive.rect.y + primitive.rect.height * 0.12,
            primitive.rect.width * 0.28,
            primitive.rect.height * 0.68,
        );
        self.fill_rect(face, rgb(207, 142, 98));
        self.fill_rect(
            Rect::new(face.x, face.y, face.width, face.height * 0.28),
            rgb(68, 38, 24),
        );
        self.fill_rect(
            Rect::new(
                face.x + face.width * 0.22,
                face.y + face.height * 0.42,
                10.0,
                5.0,
            ),
            rgb(24, 30, 36),
        );
        self.fill_rect(
            Rect::new(
                face.x + face.width * 0.62,
                face.y + face.height * 0.42,
                10.0,
                5.0,
            ),
            rgb(24, 30, 36),
        );
        self.fill_rect(
            Rect::new(
                face.x + face.width * 0.34,
                face.y + face.height * 0.72,
                face.width * 0.32,
                4.0,
            ),
            rgb(128, 18, 36),
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
}

/// Creates a packed RGB pixel.
#[must_use]
pub const fn rgb(red: u8, green: u8, blue: u8) -> Pixel {
    ((red as u32) << 16) | ((green as u32) << 8) | (blue as u32)
}

fn pixel_from_brush(brush: Brush) -> Pixel {
    match brush {
        Brush::Solid(color) => pixel_from_color(color),
    }
}

fn pixel_from_color(color: Color) -> Pixel {
    let red = (color.r.clamp(0.0, 1.0) * 255.0).round() as u8;
    let green = (color.g.clamp(0.0, 1.0) * 255.0).round() as u8;
    let blue = (color.b.clamp(0.0, 1.0) * 255.0).round() as u8;
    rgb(red, green, blue)
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
        ':' => [0, 0b01100, 0b01100, 0, 0b01100, 0b01100, 0],
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
    use crate::editor_shell;

    #[test]
    fn rasterized_editor_shell_is_not_blank() {
        let scenario = editor_shell();
        let frame = rasterize(&scenario.primitives, 1440, 900);

        assert!(frame.has_visible_variation());
        assert!(frame.unique_color_count_at_least(8));
        assert!(frame.count_color(rgb(41, 97, 255)) > 1_000);
    }

    #[test]
    fn raster_frame_writes_bmp() {
        let scenario = editor_shell();
        let frame = rasterize(&scenario.primitives, 320, 200);
        let path = std::env::temp_dir().join("kinetik-ui-showcase-smoke.bmp");

        super::write_bmp(&frame, &path).expect("write bmp");

        assert!(std::fs::metadata(path).expect("metadata").len() > 54);
    }
}
