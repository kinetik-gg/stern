use core::ops::Range;

use stern_core::{
    Brush, ClipId, CornerRadius, LinePrimitive, Point, Primitive, Rect, RectPrimitive, Size,
    Stroke, TextFieldRecipe, TextLayoutId, TextPrimitive, Transform, Vec2, WidgetId,
};
use stern_text::{
    ShapedTextLayout, ShapedTextNavigation, TextAffinity, TextCaret, TextEditState, TextFeatureSet,
    TextLayoutKey, TextLayoutStore, TextSelection, TextStyle, TextViewport, TextViewportMode,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TextFieldKind {
    SingleLine,
    WrappedMultiLine,
}

impl TextFieldKind {
    pub(crate) const fn viewport_mode(self) -> TextViewportMode {
        match self {
            Self::SingleLine => TextViewportMode::SingleLine,
            Self::WrappedMultiLine => TextViewportMode::WrappedMultiLine,
        }
    }

    pub(crate) const fn wraps(self) -> bool {
        matches!(self, Self::WrappedMultiLine)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DisplayTextMap {
    text: String,
    model_len: usize,
    insertion: usize,
    preedit_range: Option<Range<usize>>,
    display_caret: TextCaret,
}

impl DisplayTextMap {
    fn from_state(state: &TextEditState) -> Self {
        let model_caret = state.caret_position();
        let insertion = model_caret.offset;
        let mut text = state.text.clone();
        let mut preedit_range = None;
        let mut display_caret = model_caret;

        if let Some(composition) = &state.composition
            && !composition.text.is_empty()
        {
            text.insert_str(insertion, &composition.text);
            let end = insertion + composition.text.len();
            preedit_range = Some(insertion..end);
            let selection = clamped_composition_selection(composition);
            let selection_end =
                selection.map_or(composition.text.len(), |selection| selection.active);
            let display_offset = insertion + selection_end;
            display_caret = TextCaret::new(
                display_offset,
                default_caret_affinity(&text, display_offset),
            );
        }

        Self {
            text,
            model_len: state.text.len(),
            insertion,
            preedit_range,
            display_caret,
        }
    }

    fn display_to_model_caret(&self, display_caret: TextCaret) -> TextCaret {
        let display_offset = clamp_grapheme_boundary(&self.text, display_caret.offset);
        let Some(range) = &self.preedit_range else {
            return TextCaret::new(display_offset.min(self.model_len), display_caret.affinity);
        };
        if display_offset < range.start {
            TextCaret::new(display_offset, display_caret.affinity)
        } else if display_offset <= range.end {
            TextCaret::new(self.insertion, display_caret.affinity)
        } else {
            TextCaret::new(
                display_offset
                    .saturating_sub(range.len())
                    .min(self.model_len),
                display_caret.affinity,
            )
        }
    }

    fn model_to_display(&self, model_offset: usize) -> usize {
        let model_offset = model_offset.min(self.model_len);
        self.preedit_range.as_ref().map_or(model_offset, |range| {
            if model_offset > self.insertion {
                model_offset + range.len()
            } else {
                model_offset
            }
        })
    }

    fn model_range_to_display(&self, selection: TextSelection) -> Range<usize> {
        let range = selection.range();
        self.model_to_display(range.start)..self.model_to_display(range.end)
    }
}

#[derive(Debug, Clone, PartialEq)]
struct VisualRow {
    start: usize,
    end: usize,
    top: f32,
    width: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TextFieldGeometry {
    display: DisplayTextMap,
    kind: TextFieldKind,
    recipe: TextFieldRecipe,
    field_rect: Rect,
    content_rect: Rect,
    viewport: TextViewport,
    layout_id: Option<TextLayoutId>,
    navigation: Option<ShapedTextNavigation>,
    rows: Vec<VisualRow>,
    selection_rects: Vec<Rect>,
    composition_rects: Vec<Rect>,
    caret_content_rect: Rect,
}

impl TextFieldGeometry {
    pub(crate) fn build(
        rect: Rect,
        state: &TextEditState,
        recipe: &TextFieldRecipe,
        kind: TextFieldKind,
        retained_offset: Vec2,
        text_layouts: Option<&mut TextLayoutStore>,
    ) -> Self {
        Self::build_with_features(
            rect,
            state,
            recipe,
            kind,
            retained_offset,
            TextFeatureSet::NONE,
            text_layouts,
        )
    }

    pub(crate) fn build_with_features(
        rect: Rect,
        state: &TextEditState,
        recipe: &TextFieldRecipe,
        kind: TextFieldKind,
        retained_offset: Vec2,
        features: TextFeatureSet,
        text_layouts: Option<&mut TextLayoutStore>,
    ) -> Self {
        Self::build_with_retention(
            rect,
            state,
            recipe,
            kind,
            retained_offset,
            features,
            text_layouts,
            TextGeometryRetention::Retained,
        )
    }

    pub(crate) fn build_transient(
        rect: Rect,
        state: &TextEditState,
        recipe: &TextFieldRecipe,
        kind: TextFieldKind,
        retained_offset: Vec2,
        text_layouts: Option<&mut TextLayoutStore>,
    ) -> Self {
        Self::build_transient_with_features(
            rect,
            state,
            recipe,
            kind,
            retained_offset,
            TextFeatureSet::NONE,
            text_layouts,
        )
    }

    pub(crate) fn build_transient_with_features(
        rect: Rect,
        state: &TextEditState,
        recipe: &TextFieldRecipe,
        kind: TextFieldKind,
        retained_offset: Vec2,
        features: TextFeatureSet,
        text_layouts: Option<&mut TextLayoutStore>,
    ) -> Self {
        Self::build_with_retention(
            rect,
            state,
            recipe,
            kind,
            retained_offset,
            features,
            text_layouts,
            TextGeometryRetention::Transient,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn build_with_retention(
        rect: Rect,
        state: &TextEditState,
        recipe: &TextFieldRecipe,
        kind: TextFieldKind,
        retained_offset: Vec2,
        features: TextFeatureSet,
        text_layouts: Option<&mut TextLayoutStore>,
        retention: TextGeometryRetention,
    ) -> Self {
        let display = DisplayTextMap::from_state(state);
        let model_selection = state.selection.clamp_to_text(&state.text);
        let content_rect = Rect::new(
            rect.x + recipe.padding_x,
            rect.y + recipe.padding_y,
            (rect.width - recipe.padding_x * 2.0).max(0.0),
            (rect.height - recipe.padding_y * 2.0).max(0.0),
        );
        let (
            layout_id,
            navigation,
            rows,
            selection_rects,
            composition_rects,
            caret_content_rect,
            size,
        ) = text_layouts.map_or_else(
            || fallback_geometry(&display, model_selection, content_rect, recipe, kind),
            |store| {
                shaped_geometry(
                    &display,
                    model_selection,
                    content_rect,
                    recipe,
                    kind,
                    features,
                    store,
                    retention,
                )
            },
        );
        let content_size = Size::new(
            size.width.max(caret_content_rect.max_x()),
            size.height.max(caret_content_rect.max_y()),
        );
        let viewport = TextViewport::new(
            kind.viewport_mode(),
            content_rect.size(),
            content_size,
            retained_offset,
        );

        Self {
            display,
            kind,
            recipe: *recipe,
            field_rect: rect,
            content_rect,
            viewport,
            layout_id,
            navigation,
            rows,
            selection_rects,
            composition_rects,
            caret_content_rect,
        }
    }

    pub(crate) fn model_caret_at(&self, position: Point) -> TextCaret {
        let offset = self.viewport.offset();
        let x = position.x - self.content_rect.x + offset.x;
        let y = position.y - self.content_rect.y + offset.y;
        let display_caret = self.navigation.as_ref().map_or_else(
            || {
                let offset =
                    fallback_hit_offset(&self.display.text, &self.rows, x, y, &self.recipe);
                let offset = clamp_grapheme_boundary(&self.display.text, offset);
                TextCaret::at(offset)
            },
            |navigation| navigation.hit_test_caret(x, y - self.recipe.font.size),
        );
        self.display.display_to_model_caret(display_caret)
    }

    pub(crate) const fn viewport(&self) -> TextViewport {
        self.viewport
    }

    pub(crate) const fn caret_content_rect(&self) -> Rect {
        self.caret_content_rect
    }

    pub(crate) fn visible_caret_rect(&self) -> Option<Rect> {
        let offset = self.viewport.offset();
        let caret = self.caret_content_rect.translate(Vec2::new(
            self.content_rect.x - offset.x,
            self.content_rect.y - offset.y,
        ));
        self.content_rect.intersection(caret)
    }

    pub(crate) fn primitives(
        &self,
        id: WidgetId,
        focused: bool,
        interactive: bool,
        caret_visible: bool,
    ) -> Vec<Primitive> {
        let mut primitives = vec![Primitive::Rect(RectPrimitive {
            rect: self.field_rect,
            fill: Some(self.recipe.background),
            stroke: Some(self.recipe.border),
            radius: self.recipe.radius,
        })];
        let clip = ClipId::from_raw(id.raw());
        primitives.push(Primitive::ClipBegin {
            id: clip,
            rect: self.content_rect,
        });
        let offset = self.viewport.offset();
        primitives.push(Primitive::TransformBegin(Transform::translation(
            Vec2::new(-offset.x, -offset.y),
        )));

        if focused && interactive {
            primitives.extend(self.selection_rects.iter().map(|rect| {
                Primitive::Rect(RectPrimitive {
                    rect: rect.translate(Vec2::new(self.content_rect.x, self.content_rect.y)),
                    fill: Some(self.recipe.selection),
                    stroke: None,
                    radius: CornerRadius::all(0.0),
                })
            }));
        }

        if let Some(layout) = self.layout_id {
            primitives.push(Primitive::Text(TextPrimitive {
                layout: Some(layout),
                origin: Point::new(
                    self.content_rect.x,
                    self.content_rect.y + self.recipe.font.size,
                ),
                text: self.display.text.clone(),
                family: self.recipe.font.family.to_owned(),
                size: self.recipe.font.size,
                line_height: self.recipe.font.line_height,
                brush: Brush::Solid(self.recipe.foreground),
            }));
        } else {
            for row in &self.rows {
                primitives.push(Primitive::Text(TextPrimitive {
                    layout: None,
                    origin: Point::new(
                        self.content_rect.x,
                        self.content_rect.y + row.top + self.recipe.font.size,
                    ),
                    text: self.display.text[row.start..row.end].to_owned(),
                    family: self.recipe.font.family.to_owned(),
                    size: self.recipe.font.size,
                    line_height: self.recipe.font.line_height,
                    brush: Brush::Solid(self.recipe.foreground),
                }));
            }
        }

        if focused && interactive {
            for rect in &self.composition_rects {
                let rect = rect.translate(Vec2::new(self.content_rect.x, self.content_rect.y));
                let y = rect.max_y() + 1.0;
                primitives.push(Primitive::Line(LinePrimitive {
                    from: Point::new(rect.x, y),
                    to: Point::new(rect.max_x().max(rect.x + 1.0), y),
                    stroke: Stroke::new(1.0, self.recipe.selection),
                }));
            }
        }

        if focused && interactive && caret_visible {
            primitives.push(Primitive::Rect(RectPrimitive {
                rect: self
                    .caret_content_rect
                    .translate(Vec2::new(self.content_rect.x, self.content_rect.y)),
                fill: Some(Brush::Solid(self.recipe.caret)),
                stroke: None,
                radius: CornerRadius::all(0.0),
            }));
        }

        primitives.push(Primitive::TransformEnd);
        primitives.push(Primitive::ClipEnd { id: clip });
        primitives
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TextGeometryRetention {
    Transient,
    Retained,
}

type GeometryParts = (
    Option<TextLayoutId>,
    Option<ShapedTextNavigation>,
    Vec<VisualRow>,
    Vec<Rect>,
    Vec<Rect>,
    Rect,
    Size,
);

#[allow(clippy::too_many_arguments)]
fn shaped_geometry(
    display: &DisplayTextMap,
    selection: TextSelection,
    content_rect: Rect,
    recipe: &TextFieldRecipe,
    kind: TextFieldKind,
    features: TextFeatureSet,
    store: &mut TextLayoutStore,
    retention: TextGeometryRetention,
) -> GeometryParts {
    let key = TextLayoutKey::new(
        display.text.clone(),
        TextStyle::new(
            recipe.font.family,
            recipe.font.size,
            recipe.font.line_height,
        )
        .with_features(features),
        content_rect.width,
        kind.wraps(),
    );
    match retention {
        TextGeometryRetention::Transient => {
            let layout = store.shape_transient(&key);
            geometry_from_layout_or_fallback(
                display,
                selection,
                content_rect,
                recipe,
                kind,
                None,
                &layout,
            )
        }
        TextGeometryRetention::Retained => {
            let Some(id) = store.try_layout_id(key) else {
                return fallback_geometry(display, selection, content_rect, recipe, kind);
            };
            let Some(layout) = store.layout(id) else {
                return fallback_geometry(display, selection, content_rect, recipe, kind);
            };
            geometry_from_layout_or_fallback(
                display,
                selection,
                content_rect,
                recipe,
                kind,
                Some(id),
                layout,
            )
        }
    }
}

fn geometry_from_layout_or_fallback(
    display: &DisplayTextMap,
    selection: TextSelection,
    content_rect: Rect,
    recipe: &TextFieldRecipe,
    kind: TextFieldKind,
    id: Option<TextLayoutId>,
    layout: &ShapedTextLayout,
) -> GeometryParts {
    authoritative_geometry(display, selection, recipe, id, layout)
        .unwrap_or_else(|| fallback_geometry(display, selection, content_rect, recipe, kind))
}

fn authoritative_geometry(
    display: &DisplayTextMap,
    selection: TextSelection,
    recipe: &TextFieldRecipe,
    id: Option<TextLayoutId>,
    layout: &ShapedTextLayout,
) -> Option<GeometryParts> {
    let navigation = layout.navigation(&display.text).ok()?;
    let baseline = Vec2::new(0.0, recipe.font.size);
    let selection_rects = navigation
        .selection_rects(display.model_range_to_display(selection))
        .into_iter()
        .map(|rect| rect.translate(baseline))
        .collect();
    let composition_rects = display
        .preedit_range
        .as_ref()
        .map_or_else(Vec::new, |range| {
            navigation
                .selection_rects(range.clone())
                .into_iter()
                .map(|rect| rect.translate(baseline))
                .collect()
        });
    let caret = navigation
        .caret_rect(display.display_caret)
        .translate(baseline);
    Some((
        id,
        Some(navigation),
        Vec::new(),
        selection_rects,
        composition_rects,
        caret,
        layout.size,
    ))
}

fn fallback_geometry(
    display: &DisplayTextMap,
    selection: TextSelection,
    content_rect: Rect,
    recipe: &TextFieldRecipe,
    kind: TextFieldKind,
) -> GeometryParts {
    let rows = fallback_rows(&display.text, content_rect.width, recipe, kind);
    let selection_rects = fallback_range_rects(
        &display.text,
        &rows,
        display.model_range_to_display(selection),
        recipe,
    );
    let composition_rects = display
        .preedit_range
        .as_ref()
        .map_or_else(Vec::new, |range| {
            fallback_range_rects(&display.text, &rows, range.clone(), recipe)
        });
    let caret = fallback_caret_rect(&display.text, &rows, display.display_caret, recipe);
    let width = rows.iter().map(|row| row.width).fold(0.0_f32, f32::max);
    let height = rows.last().map_or(recipe.font.line_height.max(1.0), |row| {
        row.top + recipe.font.line_height.max(1.0)
    });
    (
        None,
        None,
        rows,
        selection_rects,
        composition_rects,
        caret,
        Size::new(width, height),
    )
}

fn fallback_rows(
    text: &str,
    width: f32,
    recipe: &TextFieldRecipe,
    kind: TextFieldKind,
) -> Vec<VisualRow> {
    let char_width = fallback_char_width(recipe);
    let line_height = recipe.font.line_height.max(1.0);
    let mut rows = Vec::new();
    let mut start = 0;
    let mut row_width = 0.0;

    for (index, character) in text.char_indices() {
        if character == '\n' && kind.wraps() {
            push_row(&mut rows, start, index, row_width, line_height);
            start = index + character.len_utf8();
            row_width = 0.0;
            continue;
        }
        if kind.wraps() && row_width > 0.0 && row_width + char_width > width.max(0.0) {
            push_row(&mut rows, start, index, row_width, line_height);
            start = index;
            row_width = 0.0;
        }
        row_width += char_width;
    }
    push_row(&mut rows, start, text.len(), row_width, line_height);
    rows
}

#[allow(clippy::cast_precision_loss)]
fn push_row(rows: &mut Vec<VisualRow>, start: usize, end: usize, width: f32, line_height: f32) {
    rows.push(VisualRow {
        start,
        end,
        top: rows.len() as f32 * line_height,
        width,
    });
}

fn fallback_range_rects(
    text: &str,
    rows: &[VisualRow],
    range: Range<usize>,
    recipe: &TextFieldRecipe,
) -> Vec<Rect> {
    if range.start >= range.end {
        return Vec::new();
    }
    rows.iter()
        .filter_map(|row| {
            let start = range.start.max(row.start).min(row.end);
            let end = range.end.max(row.start).min(row.end);
            if start >= end {
                return None;
            }
            let left = fallback_prefix_width(&text[row.start..row.end], start - row.start, recipe);
            let right = fallback_prefix_width(&text[row.start..row.end], end - row.start, recipe);
            Some(Rect::new(
                left,
                row.top,
                (right - left).max(1.0),
                recipe.font.line_height.max(1.0),
            ))
        })
        .collect()
}

fn fallback_caret_rect(
    text: &str,
    rows: &[VisualRow],
    caret: TextCaret,
    recipe: &TextFieldRecipe,
) -> Rect {
    let offset = clamp_grapheme_boundary(text, caret.offset);
    let index = fallback_row_index(rows, offset);
    let row = &rows[index];
    let x = fallback_prefix_width(&text[row.start..row.end], offset - row.start, recipe);
    Rect::new(x, row.top, 1.0, recipe.font.line_height.max(1.0))
}

fn fallback_hit_offset(
    text: &str,
    rows: &[VisualRow],
    x: f32,
    y: f32,
    recipe: &TextFieldRecipe,
) -> usize {
    let line_height = recipe.font.line_height.max(1.0);
    let mut row_index = 0;
    let mut threshold = line_height;
    while y >= threshold && row_index + 1 < rows.len() {
        row_index += 1;
        threshold += line_height;
    }
    let row = &rows[row_index];
    row.start + fallback_x_offset(&text[row.start..row.end], x, recipe)
}

fn fallback_row_index(rows: &[VisualRow], offset: usize) -> usize {
    for (index, row) in rows.iter().enumerate() {
        let next = rows.get(index + 1);
        if offset < row.end || (offset == row.end && next.is_none_or(|next| next.start != row.end))
        {
            return index;
        }
    }
    rows.len().saturating_sub(1)
}

fn fallback_x_offset(text: &str, x: f32, recipe: &TextFieldRecipe) -> usize {
    if x <= 0.0 {
        return 0;
    }
    let char_width = fallback_char_width(recipe);
    let mut cursor = 0.0;
    for (index, character) in text.char_indices() {
        if x < cursor + char_width * 0.5 {
            return index;
        }
        cursor += char_width;
        if x < cursor + char_width * 0.5 {
            return index + character.len_utf8();
        }
    }
    text.len()
}

#[allow(clippy::cast_precision_loss)]
fn fallback_prefix_width(text: &str, offset: usize, recipe: &TextFieldRecipe) -> f32 {
    let offset = clamp_boundary(text, offset);
    text[..offset].chars().count() as f32 * fallback_char_width(recipe)
}

fn fallback_char_width(recipe: &TextFieldRecipe) -> f32 {
    (recipe.font.size * 0.55).max(1.0)
}

fn clamp_boundary(text: &str, offset: usize) -> usize {
    let mut offset = offset.min(text.len());
    while !text.is_char_boundary(offset) {
        offset -= 1;
    }
    offset
}

fn clamp_grapheme_boundary(text: &str, offset: usize) -> usize {
    TextSelection::new(offset, offset)
        .clamp_to_text(text)
        .active
}

fn clamped_composition_selection(
    composition: &stern_text::TextComposition,
) -> Option<TextSelection> {
    composition.selection.map(|selection| {
        TextSelection::new(selection.start, selection.end).clamp_to_text(&composition.text)
    })
}

const fn default_caret_affinity(text: &str, offset: usize) -> TextAffinity {
    if offset == 0 {
        TextAffinity::After
    } else if offset >= text.len() {
        TextAffinity::Before
    } else {
        TextAffinity::After
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use stern_core::{ComponentState, TextRange, default_dark_theme};
    use stern_text::{ShapedTextLine, TextComposition};

    #[test]
    fn malformed_shaped_navigation_discards_the_whole_snapshot() {
        let theme = default_dark_theme();
        let recipe = theme.text_field(ComponentState {
            hovered: false,
            pressed: false,
            focused: true,
            disabled: false,
            selected: false,
        });
        let state = TextEditState::new("a");
        let display = DisplayTextMap::from_state(&state);
        let content_rect = Rect::new(0.0, 0.0, 120.0, 24.0);
        let malformed = ShapedTextLayout {
            size: Size::new(8.0, 24.0),
            line_count: 1,
            lines: vec![ShapedTextLine {
                visual_index: 0,
                source_line_index: 0,
                text_start: 0,
                text_end: 1,
                top_y: -18.0,
                baseline_y: 0.0,
                height: 24.0,
                width: 8.0,
                rtl: false,
            }],
            runs: Vec::new(),
        };

        let parts = geometry_from_layout_or_fallback(
            &display,
            state.selection,
            content_rect,
            &recipe,
            TextFieldKind::SingleLine,
            Some(TextLayoutId::from_raw(77)),
            &malformed,
        );
        assert!(parts.0.is_none(), "invalid layout ID must not be painted");
        assert!(parts.1.is_none(), "invalid navigation must not escape");
        assert!(!parts.2.is_empty(), "fallback rows must own geometry");
    }

    #[test]
    fn public_preedit_selection_clamps_both_grapheme_endpoints() {
        let composition = TextComposition {
            text: "e\u{301}o\u{301}".to_owned(),
            selection: Some(TextRange::new(2, 5)),
        };
        assert_eq!(
            clamped_composition_selection(&composition),
            Some(TextSelection::new(0, 3))
        );
    }

    #[test]
    fn retained_preedit_uses_one_navigation_for_caret_underline_and_hits() {
        let theme = default_dark_theme();
        let recipe = theme.text_field(ComponentState {
            hovered: false,
            pressed: false,
            focused: true,
            disabled: false,
            selected: false,
        });
        let rect = Rect::new(10.0, 8.0, 260.0, 32.0);
        let mut state = TextEditState::new("ab");
        state.set_caret_position(TextCaret::new(1, TextAffinity::After));
        state.composition = Some(TextComposition::new("e\u{301}o\u{301}", None));
        let mut store = TextLayoutStore::new();
        let geometry = TextFieldGeometry::build(
            rect,
            &state,
            &recipe,
            TextFieldKind::SingleLine,
            Vec2::ZERO,
            Some(&mut store),
        );

        let navigation = geometry
            .navigation
            .as_ref()
            .expect("valid retained preedit navigation");
        let preedit = geometry
            .display
            .preedit_range
            .clone()
            .expect("non-empty preedit range");
        assert_eq!(preedit, 1..7);
        assert_eq!(
            geometry.display.display_caret,
            TextCaret::new(preedit.end, TextAffinity::After),
            "an absent platform selection places the display caret at preedit end"
        );

        let baseline = Vec2::new(0.0, recipe.font.size);
        assert_eq!(
            geometry.caret_content_rect,
            navigation
                .caret_rect(geometry.display.display_caret)
                .translate(baseline)
        );
        let expected_underline_rects = navigation
            .selection_rects(preedit.clone())
            .into_iter()
            .map(|rect| rect.translate(baseline))
            .collect::<Vec<_>>();
        assert_eq!(geometry.composition_rects, expected_underline_rects);

        let painted_underlines = geometry
            .primitives(WidgetId::from_key("field"), true, true, true)
            .into_iter()
            .filter_map(|primitive| match primitive {
                Primitive::Line(line) => Some((line.from, line.to)),
                _ => None,
            })
            .collect::<Vec<_>>();
        let expected_underlines = expected_underline_rects
            .iter()
            .map(|rect| {
                let rect =
                    rect.translate(Vec2::new(geometry.content_rect.x, geometry.content_rect.y));
                let y = rect.max_y() + 1.0;
                (
                    Point::new(rect.x, y),
                    Point::new(rect.max_x().max(rect.x + 1.0), y),
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(painted_underlines, expected_underlines);

        for display_offset in [4, preedit.end] {
            let witness = navigation
                .caret_stops()
                .iter()
                .filter(|stop| stop.caret.offset == display_offset)
                .find_map(|stop| {
                    let rect = navigation.caret_rect(stop.caret);
                    let y = rect.y + rect.height * 0.5;
                    [0.0, -0.25, 0.25].into_iter().find_map(|delta| {
                        let x = rect.x + delta;
                        let hit = navigation.hit_test_caret(x, y);
                        (hit.offset == display_offset).then_some((x, y, hit))
                    })
                })
                .unwrap_or_else(|| panic!("shaped hit witness at display offset {display_offset}"));
            let model = geometry.model_caret_at(Point::new(
                geometry.content_rect.x + witness.0,
                geometry.content_rect.y + recipe.font.size + witness.1,
            ));
            assert_eq!(
                model,
                TextCaret::new(1, witness.2.affinity),
                "preedit hits collapse to the insertion and preserve shaped affinity"
            );
        }
    }
}
