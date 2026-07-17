use cosmic_text::{
    Attrs, BidiParagraphs, Buffer, Ellipsize, EllipsizeHeightLimit, Family, FeatureTag,
    FontFeatures, FontSystem, Metrics, Shaping, Wrap, fontdb,
    skrifa::{FontRef, MetadataProvider, Tag},
};
use stern_core::Size;

use crate::fonts::INTER_FONTDB_FAMILY;
use crate::{
    DEFAULT_FONT_FAMILY, DEFAULT_MONOSPACE_FONT_FAMILY, ShapedGlyph, ShapedGlyphRun,
    ShapedTextLayout, ShapedTextLine, TextLayoutKey, TextOverflow, TextStyle, fonts,
};

/// Cosmic-text backed engine handle.
pub struct CosmicTextEngine {
    pub(crate) font_system: FontSystem,
}

impl CosmicTextEngine {
    /// Creates a cosmic-text engine.
    #[must_use]
    pub fn new() -> Self {
        Self {
            font_system: bundled_font_system(),
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
        let requested_width = key.width();
        let width = requested_width.max(0.0);
        let end_ellipsis = key.overflow == TextOverflow::EndEllipsis
            && requested_width.is_finite()
            && requested_width > 0.0
            && !key.wrap
            && is_single_bidi_paragraph(&key.text);
        buffer.set_size(
            (end_ellipsis || key.wrap && width > 0.0).then_some(width),
            None,
        );
        buffer.set_wrap(if key.wrap {
            Wrap::WordOrGlyph
        } else {
            Wrap::None
        });
        if end_ellipsis {
            buffer.set_ellipsize(Ellipsize::End(EllipsizeHeightLimit::Lines(1)));
        }
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
                let normalized_coords = normalized_coords(&font, glyph.font_weight);
                let font = font.as_peniko();
                let needs_new_run = current.as_ref().is_none_or(|glyph_run| {
                    glyph_run.font != font
                        || glyph_run.normalized_coords != normalized_coords
                        || glyph_run.font_size.to_bits() != glyph.font_size.to_bits()
                        || glyph_run.line_index != run.line_i
                });

                if needs_new_run {
                    if let Some(run) = current.take() {
                        runs.push(run);
                    }
                    current = Some(ShapedGlyphRun {
                        font: font.clone(),
                        normalized_coords,
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
                        elided: end_ellipsis && glyph.start == glyph.end,
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

fn is_single_bidi_paragraph(text: &str) -> bool {
    if text
        .chars()
        .any(|character| matches!(character, '\r' | '\n'))
    {
        return false;
    }

    let mut paragraphs = BidiParagraphs::new(text);
    match paragraphs.next() {
        Some(paragraph) => paragraph.len() == text.len() && paragraphs.next().is_none(),
        None => text.is_empty(),
    }
}

fn attrs_for_style(style: &TextStyle) -> Attrs<'_> {
    let family = match style.family.as_str() {
        "" | "sans" | "sans-serif" | "system" => Family::SansSerif,
        "serif" => Family::Serif,
        "monospace" | "mono" => Family::Monospace,
        "cursive" => Family::Cursive,
        "fantasy" => Family::Fantasy,
        DEFAULT_FONT_FAMILY => Family::Name(INTER_FONTDB_FAMILY),
        family => Family::Name(family),
    };
    let mut features = FontFeatures::new();
    if style.features.has_tabular_numbers() {
        features.set(FeatureTag::new(b"tnum"), 1);
    }
    Attrs::new()
        .family(family)
        .weight(fontdb::Weight(style.weight))
        .font_features(features)
}

fn normalized_coords(font: &cosmic_text::Font, weight: fontdb::Weight) -> Vec<i16> {
    let peniko = font.as_peniko();
    let Ok(font_ref) = FontRef::from_index(font.data(), peniko.index) else {
        return Vec::new();
    };
    font_ref
        .axes()
        .location([(Tag::new(b"wght"), f32::from(weight.0))])
        .coords()
        .iter()
        .map(|coord| coord.to_bits())
        .collect()
}

fn bundled_font_system() -> FontSystem {
    let mut db = fontdb::Database::new();
    db.load_font_data(fonts::INTER_VARIABLE.to_vec());
    db.load_font_data(fonts::SPACE_GROTESK_VARIABLE.to_vec());
    db.load_font_data(fonts::SPACE_MONO_REGULAR.to_vec());
    db.set_sans_serif_family(INTER_FONTDB_FAMILY);
    db.set_monospace_family(DEFAULT_MONOSPACE_FONT_FAMILY);
    FontSystem::new_with_locale_and_db("en-US".to_owned(), db)
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
