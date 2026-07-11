use super::{
    Brush, ClipId, CornerRadius, LinePrimitive, PlatformRequest, Point, Primitive, Rect,
    RectPrimitive, Response, ShapedTextLayout, Stroke, TextEditState, TextFieldRecipe,
    TextLayoutKey, TextLayoutStore, TextPrimitive, TextSelection, TextStyle, UiMemory, WidgetId,
};

pub(super) fn text_input_platform_requests(
    id: WidgetId,
    rect: Rect,
    response: &Response,
    memory: &mut UiMemory,
) -> Vec<PlatformRequest> {
    if response.state.focused && !response.state.disabled {
        let previous_owner = memory.text_input_owner();
        if previous_owner == Some(id) {
            return vec![PlatformRequest::UpdateTextInputRect { rect }];
        }
        let stopped_owner = memory.take_pending_text_input_stop();
        memory.set_text_input_owner(id);
        let mut requests = Vec::new();
        if stopped_owner.is_some_and(|owner| owner != id) {
            requests.push(PlatformRequest::StopTextInput);
        }
        requests.push(PlatformRequest::StartTextInput { rect: Some(rect) });
        requests
    } else if memory.owns_text_input(id) {
        memory.clear_text_input_owner();
        if memory.take_pending_text_input_stop().is_some() {
            vec![PlatformRequest::StopTextInput]
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    }
}

pub(super) fn display_text_with_composition(
    state: &TextEditState,
) -> (String, usize, Option<core::ops::Range<usize>>) {
    let caret = TextSelection::new(0, state.caret())
        .range_in(&state.text)
        .end;
    let mut text = state.text.clone();
    if let Some(composition) = &state.composition
        && !composition.text.is_empty()
    {
        text.insert_str(caret, &composition.text);
        let range = caret..caret + composition.text.len();
        return (text, range.end, Some(range));
    }
    (text, caret, None)
}

#[allow(clippy::cast_precision_loss)]
pub(super) fn byte_prefix_width(text: &str, byte_offset: usize, text_size: f32) -> f32 {
    let end = TextSelection::new(0, byte_offset).range_in(text).end;
    text[..end].chars().count() as f32 * text_size * 0.55
}

pub(super) fn text_field_layout<'a>(
    text_layouts: Option<&'a mut TextLayoutStore>,
    text: &str,
    rect: Rect,
    recipe: &TextFieldRecipe,
    wrap: bool,
) -> Option<&'a ShapedTextLayout> {
    let store = text_layouts?;
    let width = (rect.width - recipe.padding_x * 2.0).max(0.0);
    let id = store.layout_id(TextLayoutKey::new(
        text,
        TextStyle::new(
            recipe.font.family,
            recipe.font.size,
            recipe.font.line_height,
        ),
        width,
        wrap,
    ));
    store.layout(id)
}

pub(super) fn single_line_hit_offset(
    position: Point,
    rect: Rect,
    display_text: &str,
    recipe: &TextFieldRecipe,
    layout: Option<&ShapedTextLayout>,
) -> usize {
    let content_x = rect.x + recipe.padding_x;
    let baseline = rect.y + recipe.padding_y + recipe.font.size;
    let x = position.x - content_x;
    let y = position.y - baseline;
    layout.map_or_else(
        || fallback_x_offset(display_text, x, recipe.font.size),
        |layout| layout.hit_test_point(x, y),
    )
}

pub(super) fn multi_line_hit_offset(
    position: Point,
    rect: Rect,
    display_text: &str,
    recipe: &TextFieldRecipe,
    layout: Option<&ShapedTextLayout>,
) -> usize {
    let content_x = rect.x + recipe.padding_x;
    let baseline_origin_y = rect.y + recipe.padding_y + recipe.font.size;
    let x = position.x - content_x;
    let y = position.y - baseline_origin_y;
    layout.map_or_else(
        || {
            fallback_multiline_offset(
                display_text,
                position.y - rect.y - recipe.padding_y,
                x,
                recipe,
            )
        },
        |layout| layout.hit_test_point(x, y),
    )
}

pub(super) fn fallback_x_offset(text: &str, x: f32, text_size: f32) -> usize {
    if x <= 0.0 {
        return 0;
    }

    let char_width = (text_size * 0.55).max(1.0);
    let mut cursor_x = 0.0;
    for (index, character) in text.char_indices() {
        if x < cursor_x + char_width * 0.5 {
            return index;
        }
        cursor_x += char_width;
        if x < cursor_x + char_width * 0.5 {
            return index + character.len_utf8();
        }
    }
    text.len()
}

pub(super) fn fallback_multiline_offset(
    text: &str,
    y_from_content_top: f32,
    x_from_content_left: f32,
    recipe: &TextFieldRecipe,
) -> usize {
    let fragments = text_line_fragments(text);
    let line_height = recipe.font.line_height.max(1.0);
    let mut line_index = 0_usize;
    let mut threshold = line_height;
    while y_from_content_top >= threshold && line_index + 1 < fragments.len() {
        line_index += 1;
        threshold += line_height;
    }
    let (line_start, line) = fragments[line_index];
    line_start + fallback_x_offset(line, x_from_content_left, recipe.font.size)
}

pub(super) fn text_line_fragments(text: &str) -> Vec<(usize, &str)> {
    if text.is_empty() {
        return vec![(0, "")];
    }

    let mut fragments = Vec::new();
    let mut start = 0;
    for segment in text.split_inclusive('\n') {
        let line = segment.strip_suffix('\n').unwrap_or(segment);
        fragments.push((start, line));
        start += segment.len();
    }
    if text.ends_with('\n') {
        fragments.push((start, ""));
    }
    fragments
}

pub(super) fn single_line_text_primitives(
    id: WidgetId,
    rect: Rect,
    state: &TextEditState,
    focused: bool,
    caret_visible: bool,
    recipe: &TextFieldRecipe,
    layout: Option<&ShapedTextLayout>,
) -> Vec<Primitive> {
    let (display_text, display_caret, composition_range) = display_text_with_composition(state);
    let clip = ClipId::from_raw(id.raw());
    let content_x = rect.x + recipe.padding_x;
    let content_y = rect.y + recipe.padding_y;
    let baseline = content_y + recipe.font.size;
    let mut primitives = vec![Primitive::ClipBegin {
        id: clip,
        rect: rect.inset(2.0).max_zero(),
    }];

    let selection = state.selection.range_in(&state.text);
    if !selection.is_empty() {
        if let Some(layout) = layout {
            for selection_rect in layout.selection_rects(selection) {
                primitives.push(Primitive::Rect(RectPrimitive {
                    rect: selection_rect.translate(kinetik_ui_core::Vec2::new(content_x, baseline)),
                    fill: Some(recipe.selection),
                    stroke: None,
                    radius: CornerRadius::all(0.0),
                }));
            }
        } else {
            let start_x =
                content_x + byte_prefix_width(&display_text, selection.start, recipe.font.size);
            let end_x =
                content_x + byte_prefix_width(&display_text, selection.end, recipe.font.size);
            primitives.push(Primitive::Rect(RectPrimitive {
                rect: Rect::new(
                    start_x,
                    content_y,
                    (end_x - start_x).max(1.0),
                    (rect.height - recipe.padding_y * 2.0).max(1.0),
                ),
                fill: Some(recipe.selection),
                stroke: None,
                radius: CornerRadius::all(0.0),
            }));
        }
    }

    primitives.push(Primitive::Text(TextPrimitive {
        layout: None,
        origin: Point::new(content_x, baseline),
        text: display_text.clone(),
        family: recipe.font.family.to_owned(),
        size: recipe.font.size,
        line_height: recipe.font.line_height,
        brush: Brush::Solid(recipe.foreground),
    }));

    if let Some(range) = composition_range {
        if let Some(layout) = layout {
            for rect in layout.selection_rects(range) {
                let y = baseline + rect.y + rect.height + 1.0;
                primitives.push(Primitive::Line(LinePrimitive {
                    from: Point::new(content_x + rect.x, y),
                    to: Point::new(content_x + rect.x + rect.width.max(1.0), y),
                    stroke: Stroke::new(1.0, recipe.selection),
                }));
            }
        } else {
            let start_x =
                content_x + byte_prefix_width(&display_text, range.start, recipe.font.size);
            let end_x = content_x + byte_prefix_width(&display_text, range.end, recipe.font.size);
            primitives.push(Primitive::Line(LinePrimitive {
                from: Point::new(start_x, baseline + 2.0),
                to: Point::new(end_x.max(start_x + 1.0), baseline + 2.0),
                stroke: Stroke::new(1.0, recipe.selection),
            }));
        }
    }

    if focused && caret_visible {
        let caret_rect = layout.map_or_else(
            || {
                Rect::new(
                    content_x + byte_prefix_width(&display_text, display_caret, recipe.font.size),
                    content_y,
                    1.0,
                    (rect.height - recipe.padding_y * 2.0).max(1.0),
                )
            },
            |layout| {
                layout
                    .caret_rect(display_caret)
                    .translate(kinetik_ui_core::Vec2::new(content_x, baseline))
            },
        );
        primitives.push(Primitive::Rect(RectPrimitive {
            rect: caret_rect,
            fill: Some(Brush::Solid(recipe.caret)),
            stroke: None,
            radius: CornerRadius::all(0.0),
        }));
    }

    primitives.push(Primitive::ClipEnd { id: clip });
    primitives
}

#[allow(clippy::cast_precision_loss)]
#[allow(clippy::too_many_lines)]
pub(super) fn multi_line_text_primitives(
    id: WidgetId,
    rect: Rect,
    state: &TextEditState,
    focused: bool,
    caret_visible: bool,
    recipe: &TextFieldRecipe,
    layout: Option<&ShapedTextLayout>,
) -> Vec<Primitive> {
    let (display_text, display_caret, composition_range) = display_text_with_composition(state);
    let clip = ClipId::from_raw(id.raw());
    let content_x = rect.x + recipe.padding_x;
    let content_y = rect.y + recipe.padding_y;
    let line_height = recipe.font.line_height;
    let selection = state.selection.range_in(&state.text);
    let mut primitives = vec![Primitive::ClipBegin {
        id: clip,
        rect: rect.inset(2.0).max_zero(),
    }];

    if let Some(layout) = layout {
        for selection_rect in layout.selection_rects(selection.clone()) {
            primitives.push(Primitive::Rect(RectPrimitive {
                rect: selection_rect.translate(kinetik_ui_core::Vec2::new(
                    content_x,
                    content_y + recipe.font.size,
                )),
                fill: Some(recipe.selection),
                stroke: None,
                radius: CornerRadius::all(0.0),
            }));
        }

        for (line_index, (_, line)) in text_line_fragments(&display_text).iter().enumerate() {
            let baseline = content_y + recipe.font.size + line_index as f32 * line_height;
            if baseline > rect.max_y() {
                break;
            }
            primitives.push(Primitive::Text(TextPrimitive {
                layout: None,
                origin: Point::new(content_x, baseline),
                text: (*line).to_owned(),
                family: recipe.font.family.to_owned(),
                size: recipe.font.size,
                line_height: recipe.font.line_height,
                brush: Brush::Solid(recipe.foreground),
            }));
        }

        if let Some(range) = composition_range {
            for rect in layout.selection_rects(range) {
                let y = content_y + recipe.font.size + rect.y + rect.height + 1.0;
                primitives.push(Primitive::Line(LinePrimitive {
                    from: Point::new(content_x + rect.x, y),
                    to: Point::new(content_x + rect.x + rect.width.max(1.0), y),
                    stroke: Stroke::new(1.0, recipe.selection),
                }));
            }
        }

        if focused && caret_visible {
            primitives.push(Primitive::Rect(RectPrimitive {
                rect: layout
                    .caret_rect(display_caret)
                    .translate(kinetik_ui_core::Vec2::new(
                        content_x,
                        content_y + recipe.font.size,
                    )),
                fill: Some(Brush::Solid(recipe.caret)),
                stroke: None,
                radius: CornerRadius::all(0.0),
            }));
        }

        primitives.push(Primitive::ClipEnd { id: clip });
        return primitives;
    }

    let fragments = text_line_fragments(&display_text);
    for (line_index, (line_start, line)) in fragments.iter().enumerate() {
        let baseline = content_y + recipe.font.size + line_index as f32 * line_height;
        if baseline > rect.max_y() {
            break;
        }
        let line_end = line_start + line.len();

        if !selection.is_empty() {
            let start = selection.start.max(*line_start).min(line_end);
            let end = selection.end.max(*line_start).min(line_end);
            if start < end {
                let start_x =
                    content_x + byte_prefix_width(line, start - *line_start, recipe.font.size);
                let end_x =
                    content_x + byte_prefix_width(line, end - *line_start, recipe.font.size);
                primitives.push(Primitive::Rect(RectPrimitive {
                    rect: Rect::new(
                        start_x,
                        baseline - recipe.font.size,
                        (end_x - start_x).max(1.0),
                        line_height,
                    ),
                    fill: Some(recipe.selection),
                    stroke: None,
                    radius: CornerRadius::all(0.0),
                }));
            }
        }

        primitives.push(Primitive::Text(TextPrimitive {
            layout: None,
            origin: Point::new(content_x, baseline),
            text: (*line).to_owned(),
            family: recipe.font.family.to_owned(),
            size: recipe.font.size,
            line_height: recipe.font.line_height,
            brush: Brush::Solid(recipe.foreground),
        }));

        if let Some(range) = &composition_range {
            let start = range.start.max(*line_start).min(line_end);
            let end = range.end.max(*line_start).min(line_end);
            if start < end {
                let start_x =
                    content_x + byte_prefix_width(line, start - *line_start, recipe.font.size);
                let end_x =
                    content_x + byte_prefix_width(line, end - *line_start, recipe.font.size);
                primitives.push(Primitive::Line(LinePrimitive {
                    from: Point::new(start_x, baseline + 2.0),
                    to: Point::new(end_x.max(start_x + 1.0), baseline + 2.0),
                    stroke: Stroke::new(1.0, recipe.selection),
                }));
            }
        }

        if focused && caret_visible && (*line_start..=line_end).contains(&display_caret) {
            let caret_x =
                content_x + byte_prefix_width(line, display_caret - *line_start, recipe.font.size);
            primitives.push(Primitive::Rect(RectPrimitive {
                rect: Rect::new(caret_x, baseline - recipe.font.size, 1.0, line_height),
                fill: Some(Brush::Solid(recipe.caret)),
                stroke: None,
                radius: CornerRadius::all(0.0),
            }));
        }
    }

    primitives.push(Primitive::ClipEnd { id: clip });
    primitives
}
