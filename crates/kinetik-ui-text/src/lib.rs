//! Text layout, editing state, and engine adapters for Kinetik UI.

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use cosmic_text::{Attrs, Buffer, Family, FontSystem, Metrics, PenikoFont, Shaping, Wrap};
use kinetik_ui_core::{
    Key, KeyEvent, KeyState, Rect, Size, TextInputEvent, TextLayoutId, TextRange,
};

/// Font properties used by text measurement and layout.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TextStyle {
    /// Font family name.
    pub family: String,
    /// Font size in logical units.
    pub size_bits: u32,
    /// Line height in logical units.
    pub line_height_bits: u32,
}

impl TextStyle {
    /// Creates a text style from logical sizes.
    #[must_use]
    pub fn new(family: impl Into<String>, size: f32, line_height: f32) -> Self {
        Self {
            family: family.into(),
            size_bits: size.to_bits(),
            line_height_bits: line_height.to_bits(),
        }
    }

    /// Returns the font size.
    #[must_use]
    pub const fn size(&self) -> f32 {
        f32::from_bits(self.size_bits)
    }

    /// Returns the line height.
    #[must_use]
    pub const fn line_height(&self) -> f32 {
        f32::from_bits(self.line_height_bits)
    }
}

/// Request for measuring or laying out text.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TextLayoutKey {
    /// Text content.
    pub text: String,
    /// Style.
    pub style: TextStyle,
    /// Maximum width in logical units.
    pub width_bits: u32,
    /// Whether text may wrap.
    pub wrap: bool,
}

impl TextLayoutKey {
    /// Creates a text layout key.
    #[must_use]
    pub fn new(text: impl Into<String>, style: TextStyle, width: f32, wrap: bool) -> Self {
        Self {
            text: text.into(),
            style,
            width_bits: width.to_bits(),
            wrap,
        }
    }

    /// Returns the maximum width.
    #[must_use]
    pub const fn width(&self) -> f32 {
        f32::from_bits(self.width_bits)
    }
}

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
        if range.is_empty() {
            return Vec::new();
        }

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

        nearest
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

/// Text layout cache.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TextLayoutCache {
    layouts: HashMap<TextLayoutKey, TextLayout>,
}

impl TextLayoutCache {
    /// Creates an empty text layout cache.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns a cached layout.
    #[must_use]
    pub fn get(&self, key: &TextLayoutKey) -> Option<TextLayout> {
        self.layouts.get(key).copied()
    }

    /// Inserts a cached layout.
    pub fn insert(&mut self, key: TextLayoutKey, layout: TextLayout) {
        self.layouts.insert(key, layout);
    }

    /// Returns an existing layout or inserts a newly measured layout.
    pub fn get_or_measure(&mut self, key: TextLayoutKey) -> TextLayout {
        if let Some(layout) = self.get(&key) {
            layout
        } else {
            let layout = fallback_measure(&key);
            self.insert(key, layout);
            layout
        }
    }

    /// Clears all cached layouts.
    pub fn clear(&mut self) {
        self.layouts.clear();
    }

    /// Returns the number of cached entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.layouts.len()
    }

    /// Returns true when the cache is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.layouts.is_empty()
    }
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]
fn fallback_measure(key: &TextLayoutKey) -> TextLayout {
    let line_height = key.style.line_height();
    let char_width = key.style.size() * 0.55;
    let wrap_width = key.width().max(0.0);
    let mut line_count = 0;
    let mut measured_width = 0.0_f32;

    for line in key.text.split('\n') {
        let raw_width = line.chars().count() as f32 * char_width;
        if key.wrap && wrap_width > 0.0 && raw_width > wrap_width {
            let wrapped_lines = (raw_width / wrap_width).ceil() as usize;
            line_count += wrapped_lines;
            measured_width = measured_width.max(wrap_width);
        } else {
            line_count += 1;
            measured_width = measured_width.max(raw_width);
        }
    }

    let line_count = line_count.max(1);
    let width = if key.wrap {
        measured_width.min(wrap_width).max(0.0)
    } else {
        measured_width
    };

    TextLayout {
        size: Size::new(width, line_height * line_count as f32),
        line_count,
    }
}

/// Persistent shaped text layout cache.
///
/// The store owns the text engine and assigns stable layout handles from
/// layout keys. UI layers can request handles while render backends register
/// the resulting owned layouts as resources.
pub struct TextLayoutStore {
    engine: CosmicTextEngine,
    keys: HashMap<TextLayoutKey, TextLayoutId>,
    layouts: HashMap<TextLayoutId, Arc<ShapedTextLayout>>,
}

impl TextLayoutStore {
    /// Creates an empty shaped text layout store.
    #[must_use]
    pub fn new() -> Self {
        Self {
            engine: CosmicTextEngine::new(),
            keys: HashMap::new(),
            layouts: HashMap::new(),
        }
    }

    /// Returns the number of cached shaped layouts.
    #[must_use]
    pub fn len(&self) -> usize {
        self.layouts.len()
    }

    /// Returns true when no shaped layouts are cached.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.layouts.is_empty()
    }

    /// Clears all cached shaped layouts.
    pub fn clear(&mut self) {
        self.keys.clear();
        self.layouts.clear();
    }

    /// Returns the backing text engine.
    #[must_use]
    pub const fn engine(&self) -> &CosmicTextEngine {
        &self.engine
    }

    /// Returns mutable access to the backing text engine.
    pub fn engine_mut(&mut self) -> &mut CosmicTextEngine {
        &mut self.engine
    }

    /// Returns a stable layout ID for a text layout key, shaping on cache miss.
    pub fn layout_id(&mut self, key: TextLayoutKey) -> TextLayoutId {
        if let Some(id) = self.keys.get(&key) {
            return *id;
        }

        let id = text_layout_id(&key);
        let layout = self.engine.shape_text(&key);
        self.keys.insert(key, id);
        self.layouts.insert(id, Arc::new(layout));
        id
    }

    /// Returns a shaped layout by ID.
    #[must_use]
    pub fn layout(&self, id: TextLayoutId) -> Option<&ShapedTextLayout> {
        self.layouts.get(&id).map(Arc::as_ref)
    }

    /// Iterates cached shaped text layouts.
    pub fn layouts(&self) -> impl Iterator<Item = StoredTextLayout<'_>> {
        self.keys.iter().filter_map(|(key, id)| {
            self.layouts.get(id).map(|layout| StoredTextLayout {
                id: *id,
                key,
                layout: Arc::clone(layout),
            })
        })
    }
}

impl Default for TextLayoutStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Borrowed shaped text layout entry.
#[derive(Debug, Clone, PartialEq)]
pub struct StoredTextLayout<'a> {
    /// Text layout handle.
    pub id: TextLayoutId,
    /// Layout request used to shape the text.
    pub key: &'a TextLayoutKey,
    /// Shaped layout.
    pub layout: Arc<ShapedTextLayout>,
}

/// Cosmic-text backed engine handle.
pub struct CosmicTextEngine {
    font_system: FontSystem,
}

impl CosmicTextEngine {
    /// Creates a cosmic-text engine.
    #[must_use]
    pub fn new() -> Self {
        Self {
            font_system: FontSystem::new(),
        }
    }

    /// Returns access to the underlying font system for renderer adapters.
    pub fn font_system(&mut self) -> &mut FontSystem {
        &mut self.font_system
    }

    /// Shapes text into owned glyph runs using cosmic-text.
    #[allow(clippy::too_many_lines)]
    pub fn shape_text(&mut self, key: &TextLayoutKey) -> ShapedTextLayout {
        let style = &key.style;
        let metrics = Metrics::new(
            style.size().max(1.0),
            style.line_height().max(style.size().max(1.0)),
        );
        let mut buffer = Buffer::new(&mut self.font_system, metrics);
        let width = key.width().max(0.0);
        buffer.set_size((key.wrap && width > 0.0).then_some(width), None);
        buffer.set_wrap(if key.wrap {
            Wrap::WordOrGlyph
        } else {
            Wrap::None
        });
        let attrs = attrs_for_style(style);
        buffer.set_text(&key.text, &attrs, Shaping::Advanced, None);
        buffer.shape_until_scroll(&mut self.font_system, false);

        let mut runs = Vec::new();
        let mut lines = Vec::new();
        let mut measured_width = 0.0_f32;
        let mut measured_height = 0.0_f32;
        let mut line_count = 0_usize;
        let mut first_baseline = None::<f32>;
        let line_starts = line_start_offsets(&key.text);

        for run in buffer.layout_runs() {
            line_count += 1;
            let baseline = *first_baseline.get_or_insert(run.line_y);
            let line_y = run.line_y - baseline;
            let line_top = run.line_top - baseline;
            let visual_line = lines.len();
            let source_line_start = line_starts.get(run.line_i).copied().unwrap_or(0);
            let glyph_start = run
                .glyphs
                .iter()
                .map(|glyph| source_line_start + glyph.start)
                .min();
            let glyph_end = run
                .glyphs
                .iter()
                .map(|glyph| source_line_start + glyph.end)
                .max();
            let text_start = glyph_start.unwrap_or(source_line_start);
            let text_end = glyph_end.unwrap_or(source_line_start + run.text.len());
            lines.push(ShapedTextLine {
                visual_index: visual_line,
                source_line_index: run.line_i,
                text_start,
                text_end,
                top_y: line_top,
                baseline_y: line_y,
                height: run.line_height,
                width: run.line_w,
                rtl: run.rtl,
            });
            measured_width = measured_width.max(run.line_w);
            measured_height = measured_height.max(run.line_top + run.line_height);
            let mut current: Option<ShapedGlyphRun> = None;

            for glyph in run.glyphs {
                let Some(font) = self.font_system.get_font(glyph.font_id, glyph.font_weight) else {
                    continue;
                };
                let font = font.as_peniko();
                let needs_new_run = current.as_ref().is_none_or(|glyph_run| {
                    glyph_run.font != font
                        || glyph_run.font_size.to_bits() != glyph.font_size.to_bits()
                        || glyph_run.line_index != run.line_i
                });

                if needs_new_run {
                    if let Some(run) = current.take() {
                        runs.push(run);
                    }
                    current = Some(ShapedGlyphRun {
                        font: font.clone(),
                        font_size: glyph.font_size,
                        line_index: run.line_i,
                        visual_line,
                        line_y,
                        glyphs: Vec::new(),
                    });
                }

                if let Some(active_run) = current.as_mut() {
                    active_run.glyphs.push(ShapedGlyph {
                        id: u32::from(glyph.glyph_id),
                        x: glyph.x + glyph.font_size * glyph.x_offset,
                        y: line_y + glyph.y - glyph.font_size * glyph.y_offset,
                        start: source_line_start + glyph.start,
                        end: source_line_start + glyph.end,
                        width: glyph.w,
                        rtl: glyph.level.is_rtl(),
                    });
                }
            }

            if let Some(run) = current
                && !run.glyphs.is_empty()
            {
                runs.push(run);
            }
        }

        if line_count == 0 {
            line_count = 1;
            measured_height = metrics.line_height;
            lines.push(ShapedTextLine {
                visual_index: 0,
                source_line_index: 0,
                text_start: 0,
                text_end: 0,
                top_y: -metrics.font_size,
                baseline_y: 0.0,
                height: metrics.line_height,
                width: 0.0,
                rtl: false,
            });
        }

        ShapedTextLayout {
            size: Size::new(measured_width, measured_height.max(metrics.line_height)),
            line_count,
            lines,
            runs,
        }
    }
}

impl Default for CosmicTextEngine {
    fn default() -> Self {
        Self::new()
    }
}

fn attrs_for_style(style: &TextStyle) -> Attrs<'_> {
    let family = match style.family.as_str() {
        "" | "sans" | "sans-serif" | "system" => Family::SansSerif,
        "serif" => Family::Serif,
        "monospace" | "mono" => Family::Monospace,
        "cursive" => Family::Cursive,
        "fantasy" => Family::Fantasy,
        family => Family::Name(family),
    };
    Attrs::new().family(family)
}

fn text_layout_id(key: &TextLayoutKey) -> TextLayoutId {
    let mut hasher = StableHasher::new();
    key.hash(&mut hasher);
    TextLayoutId::from_raw(hasher.finish().max(1))
}

fn line_start_offsets(text: &str) -> Vec<usize> {
    let mut starts = vec![0];
    for (index, character) in text.char_indices() {
        if character == '\n' {
            starts.push(index + character.len_utf8());
        }
    }
    starts
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

struct StableHasher(u64);

impl StableHasher {
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;

    const fn new() -> Self {
        Self(Self::OFFSET)
    }
}

impl Hasher for StableHasher {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.0 ^= u64::from(*byte);
            self.0 = self.0.wrapping_mul(Self::PRIME);
        }
    }

    fn write_u8(&mut self, i: u8) {
        self.write(&[i]);
    }

    fn write_u16(&mut self, i: u16) {
        self.write(&i.to_le_bytes());
    }

    fn write_u32(&mut self, i: u32) {
        self.write(&i.to_le_bytes());
    }

    fn write_u64(&mut self, i: u64) {
        self.write(&i.to_le_bytes());
    }

    fn write_usize(&mut self, i: usize) {
        self.write_u64(u64::try_from(i).unwrap_or(u64::MAX));
    }
}

/// Selection range in byte offsets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextSelection {
    /// Anchor byte offset.
    pub anchor: usize,
    /// Active byte offset.
    pub active: usize,
}

impl TextSelection {
    /// Creates a selection.
    #[must_use]
    pub const fn new(anchor: usize, active: usize) -> Self {
        Self { anchor, active }
    }

    /// Returns the sorted selection range.
    #[must_use]
    pub fn range(self) -> core::ops::Range<usize> {
        self.anchor.min(self.active)..self.anchor.max(self.active)
    }

    /// Returns this selection clamped to UTF-8 boundaries in text.
    #[must_use]
    pub fn clamp_to_text(self, text: &str) -> Self {
        Self {
            anchor: clamp_boundary(text, self.anchor),
            active: clamp_boundary(text, self.active),
        }
    }

    /// Returns the sorted selection range clamped to UTF-8 boundaries in text.
    #[must_use]
    pub fn range_in(self, text: &str) -> core::ops::Range<usize> {
        self.clamp_to_text(text).range()
    }

    /// Returns true when the selection is collapsed.
    #[must_use]
    pub const fn is_caret(self) -> bool {
        self.anchor == self.active
    }
}

/// Active IME/preedit composition state for a text field.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TextComposition {
    /// Current preedit text.
    pub text: String,
    /// Optional selected byte range inside the preedit text.
    pub selection: Option<TextRange>,
}

impl TextComposition {
    /// Creates a composition snapshot.
    #[must_use]
    pub fn new(text: impl Into<String>, selection: Option<TextRange>) -> Self {
        Self {
            text: text.into(),
            selection,
        }
    }
}

/// Editable single-line text state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextEditState {
    /// Text buffer.
    pub text: String,
    /// Current selection.
    pub selection: TextSelection,
    /// Active text composition, if any.
    pub composition: Option<TextComposition>,
    undo: TextUndoStack,
}

impl TextEditState {
    /// Creates text editing state.
    #[must_use]
    pub fn new(text: impl Into<String>) -> Self {
        let text = text.into();
        let caret = text.len();
        Self {
            text,
            selection: TextSelection::new(caret, caret),
            composition: None,
            undo: TextUndoStack::new(),
        }
    }

    /// Returns the caret byte offset.
    #[must_use]
    pub const fn caret(&self) -> usize {
        self.selection.active
    }

    /// Sets a collapsed caret.
    pub fn set_caret(&mut self, caret: usize) {
        let caret = clamp_boundary(&self.text, caret);
        self.selection = TextSelection::new(caret, caret);
    }

    /// Sets a selection after clamping both endpoints to UTF-8 boundaries.
    pub fn set_selection(&mut self, selection: TextSelection) {
        self.selection = selection.clamp_to_text(&self.text);
    }

    /// Selects the full text buffer.
    pub fn select_all(&mut self) {
        self.selection = TextSelection::new(0, self.text.len());
    }

    /// Returns the selected text, if the current selection is non-empty.
    #[must_use]
    pub fn selected_text(&self) -> Option<&str> {
        let range = self.selection.range_in(&self.text);
        (!range.is_empty()).then(|| &self.text[range])
    }

    /// Applies committed text input.
    pub fn insert_text(&mut self, text: &str) {
        self.record_undo();
        self.composition = None;
        self.replace_selection(text);
    }

    /// Inserts pasted text and records it in the local undo stack.
    pub fn paste_text(&mut self, text: &str) {
        self.insert_text(text);
    }

    /// Removes and returns the current selected text.
    pub fn cut_selection(&mut self) -> Option<String> {
        let selected = self.selected_text()?.to_owned();
        self.insert_text("");
        Some(selected)
    }

    /// Deletes backward from the current selection or caret.
    pub fn backspace(&mut self) {
        if !self.selection.is_caret() {
            self.record_undo();
            self.replace_selection("");
        } else if let Some(previous) = previous_boundary(&self.text, self.caret()) {
            self.record_undo();
            self.text.replace_range(previous..self.caret(), "");
            self.set_caret(previous);
        }
    }

    /// Deletes forward from the current selection or caret.
    pub fn delete_forward(&mut self) {
        if !self.selection.is_caret() {
            self.record_undo();
            self.replace_selection("");
        } else if let Some(next) = next_boundary(&self.text, self.caret()) {
            self.record_undo();
            let caret = self.caret();
            self.text.replace_range(caret..next, "");
            self.set_caret(caret);
        }
    }

    /// Moves the caret left.
    pub fn move_left(&mut self) {
        if !self.selection.is_caret() {
            let start = self.selection.range_in(&self.text).start;
            self.set_caret(start);
            return;
        }
        if let Some(previous) = previous_boundary(&self.text, self.caret()) {
            self.set_caret(previous);
        }
    }

    /// Moves the caret right.
    pub fn move_right(&mut self) {
        if !self.selection.is_caret() {
            let end = self.selection.range_in(&self.text).end;
            self.set_caret(end);
            return;
        }
        if let Some(next) = next_boundary(&self.text, self.caret()) {
            self.set_caret(next);
        }
    }

    /// Moves the caret to the start of the buffer.
    pub fn move_home(&mut self) {
        self.set_caret(0);
    }

    /// Moves the caret to the end of the buffer.
    pub fn move_end(&mut self) {
        self.set_caret(self.text.len());
    }

    /// Applies text and key events from a frame.
    pub fn apply_input(&mut self, text_events: &[TextInputEvent], key_events: &[KeyEvent]) {
        for event in text_events {
            match event {
                TextInputEvent::CompositionStart => {
                    self.composition = Some(TextComposition::default());
                }
                TextInputEvent::Composition { text, selection } => {
                    self.composition = Some(TextComposition::new(text.clone(), *selection));
                }
                TextInputEvent::Commit(text) => {
                    self.insert_text(text);
                }
                TextInputEvent::CompositionEnd => {
                    self.composition = None;
                }
            }
        }
        for event in key_events {
            if event.state != KeyState::Pressed {
                continue;
            }
            if self.apply_shortcut_event(event) {
                continue;
            }
            match event.key {
                Key::Backspace => self.backspace(),
                Key::Delete => self.delete_forward(),
                Key::ArrowLeft => self.move_left(),
                Key::ArrowRight => self.move_right(),
                Key::Home => self.move_home(),
                Key::End => self.move_end(),
                _ => {}
            }
        }
    }

    /// Performs local undo.
    pub fn undo(&mut self) -> bool {
        if let Some(previous) = self.undo.undo(EditSnapshot::from_state(self)) {
            self.restore(previous);
            true
        } else {
            false
        }
    }

    /// Performs local redo.
    pub fn redo(&mut self) -> bool {
        if let Some(next) = self.undo.redo(EditSnapshot::from_state(self)) {
            self.restore(next);
            true
        } else {
            false
        }
    }

    fn replace_selection(&mut self, replacement: &str) {
        let range = self.selection.range_in(&self.text);
        self.text.replace_range(range.clone(), replacement);
        self.set_caret(range.start + replacement.len());
    }

    fn apply_shortcut_event(&mut self, event: &KeyEvent) -> bool {
        if !(event.modifiers.ctrl || event.modifiers.super_key) {
            return false;
        }
        let Key::Character(character) = &event.key else {
            return false;
        };
        match character.to_ascii_lowercase().as_str() {
            "a" => {
                self.select_all();
                true
            }
            "z" => {
                self.undo();
                true
            }
            "y" => {
                self.redo();
                true
            }
            _ => false,
        }
    }

    fn record_undo(&mut self) {
        self.undo.push(EditSnapshot::from_state(self));
    }

    fn restore(&mut self, snapshot: EditSnapshot) {
        self.text = snapshot.text;
        self.selection = snapshot.selection;
        self.composition = None;
    }
}

/// Text-field-local undo/redo history.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TextUndoStack {
    undo: Vec<EditSnapshot>,
    redo: Vec<EditSnapshot>,
}

impl TextUndoStack {
    /// Creates an empty undo stack.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            undo: Vec::new(),
            redo: Vec::new(),
        }
    }

    /// Pushes a new undo snapshot and clears redo history.
    fn push(&mut self, snapshot: EditSnapshot) {
        if self.undo.last() != Some(&snapshot) {
            self.undo.push(snapshot);
            self.redo.clear();
        }
    }

    /// Returns the previous snapshot and stores the current snapshot for redo.
    fn undo(&mut self, current: EditSnapshot) -> Option<EditSnapshot> {
        let previous = self.undo.pop()?;
        self.redo.push(current);
        Some(previous)
    }

    /// Returns the redo snapshot and stores the current snapshot for undo.
    fn redo(&mut self, current: EditSnapshot) -> Option<EditSnapshot> {
        let next = self.redo.pop()?;
        self.undo.push(current);
        Some(next)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct EditSnapshot {
    text: String,
    selection: TextSelection,
}

impl EditSnapshot {
    fn from_state(state: &TextEditState) -> Self {
        Self {
            text: state.text.clone(),
            selection: state.selection,
        }
    }
}

fn clamp_boundary(text: &str, offset: usize) -> usize {
    let offset = offset.min(text.len());
    if text.is_char_boundary(offset) {
        offset
    } else {
        text.char_indices()
            .map(|(index, _)| index)
            .take_while(|index| *index < offset)
            .last()
            .unwrap_or(0)
    }
}

fn previous_boundary(text: &str, offset: usize) -> Option<usize> {
    text.char_indices()
        .map(|(index, _)| index)
        .take_while(|index| *index < offset)
        .last()
}

fn next_boundary(text: &str, offset: usize) -> Option<usize> {
    text.char_indices()
        .map(|(index, _)| index)
        .find(|index| *index > offset)
        .or_else(|| (offset < text.len()).then_some(text.len()))
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::{
        CosmicTextEngine, ShapedTextLayout, TextComposition, TextEditState, TextLayoutCache,
        TextLayoutKey, TextLayoutStore, TextSelection, TextStyle,
    };
    use kinetik_ui_core::{Key, KeyEvent, KeyState, Modifiers, TextInputEvent, TextRange};

    #[test]
    fn creates_cosmic_text_engine() {
        let mut engine = CosmicTextEngine::new();

        let _ = engine.font_system();
    }

    #[test]
    fn cosmic_text_engine_shapes_owned_glyph_runs() {
        let mut engine = CosmicTextEngine::new();
        let key = TextLayoutKey::new(
            "Hello",
            TextStyle::new("sans-serif", 16.0, 22.0),
            200.0,
            false,
        );

        let layout = engine.shape_text(&key);

        assert_eq!(layout.line_count, 1);
        assert!(!layout.is_empty());
        assert!(layout.size.width > 0.0);
        assert!(layout.size.height >= 22.0);
        assert!(layout.runs.iter().all(|run| !run.font.data.is_empty()));
    }

    #[test]
    fn shaped_text_layout_counts_explicit_lines() {
        let mut engine = CosmicTextEngine::new();
        let key = TextLayoutKey::new(
            "one\ntwo",
            TextStyle::new("sans-serif", 14.0, 20.0),
            200.0,
            true,
        );

        let layout = engine.shape_text(&key);

        assert_eq!(layout.line_count, 2);
        assert_eq!(layout.lines.len(), 2);
        assert_eq!(layout.lines[0].text_start, 0);
        assert_eq!(layout.lines[0].text_end, 3);
        assert_eq!(layout.lines[1].text_start, 4);
        assert_eq!(layout.lines[1].text_end, 7);
        assert_eq!(
            layout.glyph_count(),
            layout.runs.iter().map(|run| run.glyphs.len()).sum()
        );
    }

    #[test]
    fn shaped_text_layout_returns_caret_rects_for_byte_offsets() {
        let mut engine = CosmicTextEngine::new();
        let key = TextLayoutKey::new(
            "one\ntwo",
            TextStyle::new("sans-serif", 14.0, 20.0),
            200.0,
            false,
        );
        let layout = engine.shape_text(&key);

        let start = layout.caret_rect(0);
        let after_first = layout.caret_rect(3);
        let second_line = layout.caret_rect(4);

        assert!(after_first.x > start.x);
        assert!(second_line.y > start.y);
        assert_eq!(second_line.x, 0.0);
        assert!(second_line.height >= 20.0);
    }

    #[test]
    fn shaped_text_layout_returns_selection_rects_from_glyph_positions() {
        let mut engine = CosmicTextEngine::new();
        let key = TextLayoutKey::new(
            "one\ntwo",
            TextStyle::new("sans-serif", 14.0, 20.0),
            200.0,
            false,
        );
        let layout = engine.shape_text(&key);

        let rects = layout.selection_rects(1..6);

        assert_eq!(rects.len(), 2);
        assert!(rects[0].width > 0.0);
        assert!(rects[1].width > 0.0);
        assert!(rects[1].y > rects[0].y);
    }

    #[test]
    fn shaped_text_layout_hit_tests_points_to_byte_offsets() {
        let mut engine = CosmicTextEngine::new();
        let key = TextLayoutKey::new(
            "one\ntwo",
            TextStyle::new("sans-serif", 14.0, 20.0),
            200.0,
            false,
        );
        let layout = engine.shape_text(&key);
        let first_end = layout.caret_rect(3);
        let second_line = layout.caret_rect(4);

        assert_eq!(layout.hit_test_point(-10.0, 0.0), 0);
        assert_eq!(layout.hit_test_point(first_end.x + 40.0, 0.0), 3);
        assert_eq!(layout.hit_test_point(0.0, second_line.y), 4);
        assert_eq!(layout.hit_test_point(first_end.x + 40.0, second_line.y), 7);
    }

    #[test]
    fn shaped_text_layout_reports_empty_layout() {
        let layout = ShapedTextLayout {
            size: kinetik_ui_core::Size::new(0.0, 20.0),
            line_count: 1,
            lines: Vec::new(),
            runs: Vec::new(),
        };

        assert!(layout.is_empty());
        assert_eq!(layout.glyph_count(), 0);
    }

    #[test]
    fn text_layout_store_assigns_stable_cached_ids() {
        let mut store = TextLayoutStore::new();
        let key = TextLayoutKey::new(
            "Label",
            TextStyle::new("sans-serif", 12.0, 16.0),
            100.0,
            false,
        );

        let first = store.layout_id(key.clone());
        let second = store.layout_id(key);

        assert_eq!(first, second);
        assert_eq!(store.len(), 1);
        assert!(!store.layout(first).expect("layout is cached").is_empty());
    }

    #[test]
    fn text_layout_store_exports_cached_layout_entries() {
        let mut store = TextLayoutStore::new();
        let key = TextLayoutKey::new(
            "Label",
            TextStyle::new("sans-serif", 12.0, 16.0),
            100.0,
            false,
        );
        let id = store.layout_id(key.clone());

        let entries = store.layouts().collect::<Vec<_>>();

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].id, id);
        assert_eq!(entries[0].key, &key);
        assert_eq!(
            entries[0].layout.glyph_count(),
            store.layout(id).unwrap().glyph_count()
        );
        assert!(std::sync::Arc::ptr_eq(
            &entries[0].layout,
            store.layouts.get(&id).expect("cached layout")
        ));
    }

    #[test]
    fn cache_returns_hits_and_can_invalidate() {
        let style = TextStyle::new("Inter", 12.0, 16.0);
        let key = TextLayoutKey::new("hello", style, 100.0, false);
        let mut cache = TextLayoutCache::new();

        let first = cache.get_or_measure(key.clone());
        let second = cache.get_or_measure(key);

        assert_eq!(cache.len(), 1);
        assert_eq!(first, second);
        cache.clear();
        assert!(cache.is_empty());
    }

    #[test]
    fn wrapped_measurement_increases_line_count() {
        let style = TextStyle::new("Inter", 10.0, 14.0);
        let key = TextLayoutKey::new("long text string", style, 10.0, true);
        let mut cache = TextLayoutCache::new();

        let layout = cache.get_or_measure(key);

        assert!(layout.line_count > 1);
    }

    #[test]
    fn measurement_counts_explicit_lines() {
        let style = TextStyle::new("Inter", 10.0, 14.0);
        let key = TextLayoutKey::new("one\ntwo\nthree", style, 200.0, true);
        let mut cache = TextLayoutCache::new();

        let layout = cache.get_or_measure(key);

        assert_eq!(layout.line_count, 3);
    }

    #[test]
    fn inserts_text_at_caret() {
        let mut state = TextEditState::new("ab");
        state.set_caret(1);

        state.insert_text("X");

        assert_eq!(state.text, "aXb");
        assert_eq!(state.caret(), 2);
    }

    #[test]
    fn replaces_selection() {
        let mut state = TextEditState::new("abcd");
        state.selection = TextSelection::new(1, 3);

        state.insert_text("X");

        assert_eq!(state.text, "aXd");
        assert_eq!(state.caret(), 2);
    }

    #[test]
    fn selected_text_and_cut_use_current_selection() {
        let mut state = TextEditState::new("abcd");
        state.set_selection(TextSelection::new(1, 3));

        assert_eq!(state.selected_text(), Some("bc"));
        assert_eq!(state.cut_selection(), Some("bc".to_owned()));

        assert_eq!(state.text, "ad");
        assert_eq!(state.caret(), 1);
        assert!(state.undo());
        assert_eq!(state.text, "abcd");
    }

    #[test]
    fn paste_text_records_local_undo() {
        let mut state = TextEditState::new("ad");
        state.set_caret(1);

        state.paste_text("bc");

        assert_eq!(state.text, "abcd");
        assert!(state.undo());
        assert_eq!(state.text, "ad");
    }

    #[test]
    fn clamps_public_selection_before_replacing_text() {
        let mut state = TextEditState::new("éa");
        state.selection = TextSelection::new(1, 99);

        state.insert_text("X");

        assert_eq!(state.text, "X");
        assert_eq!(state.caret(), 1);
    }

    #[test]
    fn applies_text_and_key_events() {
        let mut state = TextEditState::new("");

        state.apply_input(&[TextInputEvent::Commit("a".to_owned())], &[]);
        state.apply_input(
            &[],
            &[KeyEvent::new(
                Key::Backspace,
                KeyState::Pressed,
                Modifiers::default(),
                false,
            )],
        );

        assert_eq!(state.text, "");
    }

    #[test]
    fn moves_caret_by_character_boundaries() {
        let mut state = TextEditState::new("aé");

        state.move_left();
        assert_eq!(state.caret(), 1);
        state.move_right();
        assert_eq!(state.caret(), 3);
    }

    #[test]
    fn movement_collapses_selection_and_supports_home_end() {
        let mut state = TextEditState::new("abcd");
        state.set_selection(TextSelection::new(1, 3));

        state.move_left();
        assert_eq!(state.caret(), 1);

        state.set_selection(TextSelection::new(1, 3));
        state.move_right();
        assert_eq!(state.caret(), 3);

        state.move_home();
        assert_eq!(state.caret(), 0);
        state.move_end();
        assert_eq!(state.caret(), 4);
    }

    #[test]
    fn tracks_composition_lifecycle_without_committing_preedit() {
        let mut state = TextEditState::new("");

        state.apply_input(
            &[
                TextInputEvent::CompositionStart,
                TextInputEvent::Composition {
                    text: "pre".to_owned(),
                    selection: Some(TextRange::new(1, 2)),
                },
            ],
            &[],
        );

        assert_eq!(
            state.composition,
            Some(TextComposition::new("pre", Some(TextRange::new(1, 2))))
        );
        assert_eq!(state.text, "");

        state.apply_input(&[TextInputEvent::Commit("done".to_owned())], &[]);
        assert_eq!(state.text, "done");
        assert_eq!(state.composition, None);
    }

    #[test]
    fn keyboard_shortcuts_select_all_undo_and_redo() {
        let modifiers = Modifiers::new(false, true, false, false);
        let mut state = TextEditState::new("abc");

        state.apply_input(
            &[],
            &[KeyEvent::new(
                Key::Character("a".to_owned()),
                KeyState::Pressed,
                modifiers,
                false,
            )],
        );
        assert_eq!(state.selection, TextSelection::new(0, 3));

        state.apply_input(&[TextInputEvent::Commit("X".to_owned())], &[]);
        assert_eq!(state.text, "X");
        state.apply_input(
            &[],
            &[KeyEvent::new(
                Key::Character("z".to_owned()),
                KeyState::Pressed,
                modifiers,
                false,
            )],
        );
        assert_eq!(state.text, "abc");
        state.apply_input(
            &[],
            &[KeyEvent::new(
                Key::Character("y".to_owned()),
                KeyState::Pressed,
                modifiers,
                false,
            )],
        );
        assert_eq!(state.text, "X");
    }

    #[test]
    fn undo_and_redo_are_local_to_text_state() {
        let mut state = TextEditState::new("");

        state.insert_text("a");
        state.insert_text("b");
        assert_eq!(state.text, "ab");

        assert!(state.undo());
        assert_eq!(state.text, "a");
        assert!(state.redo());
        assert_eq!(state.text, "ab");
    }
}
