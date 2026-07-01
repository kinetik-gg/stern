use cosmic_text::PenikoFont;
use kinetik_ui_core::{Rect, Size};

/// A measured text run.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextLayout {
    /// Logical size of the laid out text.
    pub size: Size,
    /// Number of visible lines.
    pub line_count: usize,
}

/// Fully shaped, owned text layout ready for renderer resource registration.
#[derive(Debug, Clone, PartialEq)]
pub struct ShapedTextLayout {
    /// Logical size of the laid out text.
    pub size: Size,
    /// Number of visible lines.
    pub line_count: usize,
    /// Visual lines produced by shaping and wrapping.
    pub lines: Vec<ShapedTextLine>,
    /// Glyph runs grouped by font and font size.
    pub runs: Vec<ShapedGlyphRun>,
}

impl ShapedTextLayout {
    /// Returns the total number of glyphs in the layout.
    #[must_use]
    pub fn glyph_count(&self) -> usize {
        self.runs.iter().map(|run| run.glyphs.len()).sum()
    }

    /// Returns true when the layout has no drawable glyphs.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.glyph_count() == 0
    }

    /// Returns the caret rectangle for a UTF-8 byte offset.
    #[must_use]
    pub fn caret_rect(&self, byte_offset: usize) -> Rect {
        let byte_offset = self.clamp_to_layout_boundary(byte_offset);
        let Some(line) = self.line_for_offset(byte_offset) else {
            return Rect::new(
                0.0,
                -self.size.height.max(1.0),
                1.0,
                self.size.height.max(1.0),
            );
        };
        let x = self.x_for_offset_in_line(line.visual_index, byte_offset);
        Rect::new(x, line.top_y, 1.0, line.height.max(1.0))
    }

    /// Returns selection rectangles for a UTF-8 byte range.
    #[must_use]
    pub fn selection_rects(&self, range: core::ops::Range<usize>) -> Vec<Rect> {
        let start = self.clamp_to_layout_boundary(range.start);
        let end = self.clamp_to_layout_boundary(range.end);
        if start >= end {
            return Vec::new();
        }
        let range = start..end;

        let mut rects = Vec::new();
        for line in &self.lines {
            let start = range.start.max(line.text_start);
            let end = range.end.min(line.text_end);
            if start >= end {
                continue;
            }

            let mut spans = Vec::<(f32, f32)>::new();
            for glyph in self.glyphs_for_visual_line(line.visual_index) {
                let glyph_start = start.max(glyph.start);
                let glyph_end = end.min(glyph.end);
                if glyph_start >= glyph_end {
                    continue;
                }
                let left = glyph.x_for_offset(glyph_start);
                let right = glyph.x_for_offset(glyph_end);
                spans.push((left.min(right), left.max(right)));
            }

            if spans.is_empty() {
                let left = self.x_for_offset_in_line(line.visual_index, start);
                let right = self.x_for_offset_in_line(line.visual_index, end);
                spans.push((left.min(right), left.max(right)));
            }

            spans.sort_by(|a, b| a.0.total_cmp(&b.0));
            let mut merged = Vec::<(f32, f32)>::new();
            for (left, right) in spans {
                if let Some((_, existing_right)) = merged.last_mut()
                    && left <= *existing_right + f32::EPSILON
                {
                    *existing_right = (*existing_right).max(right);
                    continue;
                }
                merged.push((left, right));
            }

            rects.extend(merged.into_iter().filter_map(|(left, right)| {
                let width = right - left;
                (width > 0.0).then_some(Rect::new(left, line.top_y, width, line.height.max(1.0)))
            }));
        }
        rects
    }

    /// Returns the nearest UTF-8 byte offset for a point in layout coordinates.
    ///
    /// Coordinates are relative to the same origin used by [`Self::caret_rect`]:
    /// x starts at the text origin and y is relative to the first line baseline.
    #[must_use]
    pub fn hit_test_point(&self, x: f32, y: f32) -> usize {
        let Some(line) = self.nearest_line_for_y(y) else {
            return 0;
        };

        if x <= 0.0 {
            return line.text_start;
        }
        if x >= line.width {
            return line.text_end;
        }

        let mut nearest = line.text_start;
        let mut nearest_distance = x.abs();
        for glyph in self.glyphs_for_visual_line(line.visual_index) {
            let start_x = glyph.x_for_offset(glyph.start);
            let end_x = glyph.x_for_offset(glyph.end);
            let start_distance = (x - start_x).abs();
            let end_distance = (x - end_x).abs();
            if start_distance < nearest_distance {
                nearest = glyph.start;
                nearest_distance = start_distance;
            }
            if end_distance < nearest_distance {
                nearest = glyph.end;
                nearest_distance = end_distance;
            }

            let left = start_x.min(end_x);
            let right = start_x.max(end_x);
            if x >= left && x <= right {
                return if (x - left) <= (right - x) {
                    if start_x <= end_x {
                        glyph.start
                    } else {
                        glyph.end
                    }
                } else if start_x <= end_x {
                    glyph.end
                } else {
                    glyph.start
                };
            }
        }

        self.clamp_to_layout_boundary(nearest)
    }

    fn clamp_to_layout_boundary(&self, byte_offset: usize) -> usize {
        self.layout_boundaries()
            .filter(|boundary| *boundary <= byte_offset)
            .max()
            .unwrap_or(0)
    }

    fn layout_boundaries(&self) -> impl Iterator<Item = usize> + '_ {
        self.lines
            .iter()
            .flat_map(|line| [line.text_start, line.text_end])
            .chain(
                self.runs
                    .iter()
                    .flat_map(|run| run.glyphs.iter().flat_map(|glyph| [glyph.start, glyph.end])),
            )
    }

    fn line_for_offset(&self, byte_offset: usize) -> Option<&ShapedTextLine> {
        self.lines
            .iter()
            .find(|line| byte_offset >= line.text_start && byte_offset < line.text_end)
            .or_else(|| self.lines.iter().find(|line| byte_offset == line.text_end))
            .or_else(|| self.lines.last())
    }

    fn nearest_line_for_y(&self, y: f32) -> Option<&ShapedTextLine> {
        if let Some(line) = self
            .lines
            .iter()
            .find(|line| y >= line.top_y && y < line.top_y + line.height)
        {
            return Some(line);
        }

        self.lines
            .iter()
            .min_by(|a, b| distance_to_line(a, y).total_cmp(&distance_to_line(b, y)))
    }

    fn x_for_offset_in_line(&self, visual_line: usize, byte_offset: usize) -> f32 {
        let Some(line) = self
            .lines
            .iter()
            .find(|line| line.visual_index == visual_line)
        else {
            return 0.0;
        };
        if byte_offset <= line.text_start {
            return 0.0;
        }
        if byte_offset >= line.text_end {
            return line.width;
        }

        for glyph in self.glyphs_for_visual_line(visual_line) {
            if byte_offset >= glyph.start && byte_offset <= glyph.end {
                return glyph.x_for_offset(byte_offset);
            }
        }

        line.width
    }

    fn glyphs_for_visual_line(
        &self,
        visual_line: usize,
    ) -> impl Iterator<Item = &ShapedGlyph> + '_ {
        self.runs
            .iter()
            .filter(move |run| run.visual_line == visual_line)
            .flat_map(|run| run.glyphs.iter())
    }
}

/// Visual line metadata for a shaped text layout.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShapedTextLine {
    /// Visual line index in layout order.
    pub visual_index: usize,
    /// Source text line index before wrapping.
    pub source_line_index: usize,
    /// Start byte offset in the full source text.
    pub text_start: usize,
    /// End byte offset in the full source text.
    pub text_end: usize,
    /// Top y position relative to the first baseline origin.
    pub top_y: f32,
    /// Baseline y position relative to the first baseline origin.
    pub baseline_y: f32,
    /// Visual line height.
    pub height: f32,
    /// Visual line width.
    pub width: f32,
    /// Whether the paragraph direction is right-to-left.
    pub rtl: bool,
}

/// A sequence of shaped glyphs sharing one font and font size.
#[derive(Debug, Clone, PartialEq)]
pub struct ShapedGlyphRun {
    /// Font data used by the run.
    pub font: PenikoFont,
    /// Font size in logical units.
    pub font_size: f32,
    /// Source text line index.
    pub line_index: usize,
    /// Visual line index in layout order.
    pub visual_line: usize,
    /// Baseline y position for the source line.
    pub line_y: f32,
    /// Shaped glyphs in visual order.
    pub glyphs: Vec<ShapedGlyph>,
}

/// Positioned shaped glyph.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShapedGlyph {
    /// Glyph identifier in the run font.
    pub id: u32,
    /// Glyph x offset relative to the text origin.
    pub x: f32,
    /// Glyph y offset relative to the text origin.
    pub y: f32,
    /// Source byte range start within the full text.
    pub start: usize,
    /// Source byte range end within the full text.
    pub end: usize,
    /// Advance/hitbox width in logical units.
    pub width: f32,
    /// Whether this glyph cluster is right-to-left.
    pub rtl: bool,
}

impl ShapedGlyph {
    fn x_for_offset(&self, byte_offset: usize) -> f32 {
        if self.end <= self.start {
            return self.x;
        }
        let numerator = u16::try_from(byte_offset.saturating_sub(self.start)).unwrap_or(u16::MAX);
        let denominator = u16::try_from(self.end - self.start)
            .unwrap_or(u16::MAX)
            .max(1);
        let t = (f32::from(numerator) / f32::from(denominator)).clamp(0.0, 1.0);
        if self.rtl {
            self.x + self.width * (1.0 - t)
        } else {
            self.x + self.width * t
        }
    }
}

fn distance_to_line(line: &ShapedTextLine, y: f32) -> f32 {
    if y < line.top_y {
        line.top_y - y
    } else if y > line.top_y + line.height {
        y - (line.top_y + line.height)
    } else {
        0.0
    }
}
