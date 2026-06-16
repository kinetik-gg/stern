//! Reusable widgets built from Kinetik UI core primitives.

pub mod collections;
pub mod dock;
pub mod inspector;
pub mod overlays;
pub mod ui;
pub mod viewport;

pub use collections::*;
pub use dock::*;
pub use inspector::*;
pub use overlays::*;
pub use ui::*;
pub use viewport::*;

pub use kinetik_ui_core::IconId;

use std::collections::BTreeMap;

use kinetik_ui_core::{
    Brush, ClipId, ComponentState, CornerRadius, CursorShape, ImageId, ImagePrimitive, Insets, Key,
    KeyState, LinePrimitive, PathElement, PathPrimitive, PlatformRequest, Point, Primitive, Rect,
    RectPrimitive, Response, SemanticAction, SemanticActionKind, SemanticNode, SemanticRole,
    SemanticState, SemanticValue, Stroke, TextFieldRecipe, TextInputEvent, TextPrimitive, TextRole,
    Theme, UiInput, UiMemory, WidgetId, draggable, fit_box, focusable, pad_rect, selectable,
};
use kinetik_ui_text::{
    ShapedTextLayout, TextEditState, TextLayoutKey, TextLayoutStore, TextSelection, TextStyle,
};

/// Output emitted by a widget.
#[derive(Debug, Clone, PartialEq)]
pub struct WidgetOutput {
    /// Interaction response, when the widget is interactive.
    pub response: Option<Response>,
    /// Render primitives emitted by the widget.
    pub primitives: Vec<Primitive>,
    /// Semantic nodes emitted by the widget.
    pub semantics: Vec<SemanticNode>,
    /// Platform requests emitted by the widget.
    pub platform_requests: Vec<PlatformRequest>,
}

impl WidgetOutput {
    /// Creates widget output.
    #[must_use]
    pub const fn new(response: Option<Response>, primitives: Vec<Primitive>) -> Self {
        Self {
            response,
            primitives,
            semantics: Vec::new(),
            platform_requests: Vec::new(),
        }
    }

    /// Adds a semantic node to the widget output.
    #[must_use]
    pub fn with_semantic(mut self, node: SemanticNode) -> Self {
        self.semantics.push(node);
        self
    }

    /// Adds semantic nodes to the widget output.
    #[must_use]
    pub fn with_semantics(mut self, nodes: impl IntoIterator<Item = SemanticNode>) -> Self {
        self.semantics.extend(nodes);
        self
    }

    /// Adds a platform request to the widget output.
    #[must_use]
    pub fn with_platform_request(mut self, request: PlatformRequest) -> Self {
        self.platform_requests.push(request);
        self
    }

    /// Adds platform requests to the widget output.
    #[must_use]
    pub fn with_platform_requests(
        mut self,
        requests: impl IntoIterator<Item = PlatformRequest>,
    ) -> Self {
        self.platform_requests.extend(requests);
        self
    }
}

/// One path inside a theme-colored vector icon.
#[derive(Debug, Clone, PartialEq)]
pub struct IconPath {
    /// Path elements in icon view-box coordinates.
    pub elements: Vec<PathElement>,
    /// Whether the path is filled with the current icon color.
    pub fill: bool,
    /// Optional stroke width in icon view-box units.
    pub stroke_width: Option<f32>,
}

impl IconPath {
    /// Creates a filled icon path.
    #[must_use]
    pub fn filled(elements: impl Into<Vec<PathElement>>) -> Self {
        Self {
            elements: elements.into(),
            fill: true,
            stroke_width: None,
        }
    }

    /// Creates a stroked icon path.
    #[must_use]
    pub fn stroked(elements: impl Into<Vec<PathElement>>, stroke_width: f32) -> Self {
        Self {
            elements: elements.into(),
            fill: false,
            stroke_width: Some(stroke_width),
        }
    }

    /// Creates a filled and stroked icon path.
    #[must_use]
    pub fn filled_and_stroked(elements: impl Into<Vec<PathElement>>, stroke_width: f32) -> Self {
        Self {
            elements: elements.into(),
            fill: true,
            stroke_width: Some(stroke_width),
        }
    }
}

/// Theme-colored vector icon definition.
#[derive(Debug, Clone, PartialEq)]
pub struct IconGraphic {
    /// Coordinate space used by the icon paths.
    pub view_box: Rect,
    /// Icon paths in draw order.
    pub paths: Vec<IconPath>,
}

impl IconGraphic {
    /// Creates a vector icon graphic.
    #[must_use]
    pub fn new(view_box: Rect, paths: impl Into<Vec<IconPath>>) -> Self {
        Self {
            view_box,
            paths: paths.into(),
        }
    }
}

/// Registry of theme-colored vector icons.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct IconLibrary {
    icons: BTreeMap<IconId, IconGraphic>,
}

impl IconLibrary {
    /// Creates an empty icon library.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers an icon graphic.
    pub fn register(&mut self, id: IconId, graphic: IconGraphic) {
        self.icons.insert(id, graphic);
    }

    /// Returns true when an icon is registered.
    #[must_use]
    pub fn has_icon(&self, id: IconId) -> bool {
        self.icons.contains_key(&id)
    }

    /// Returns a registered icon graphic.
    #[must_use]
    pub fn icon(&self, id: IconId) -> Option<&IconGraphic> {
        self.icons.get(&id)
    }
}

fn with_response_state(mut node: SemanticNode, response: &Response) -> SemanticNode {
    node.state.disabled = response.state.disabled;
    node.state.focused = response.state.focused;
    node.state.pressed = response.state.pressed;
    node.state.selected = response.state.selected;
    node
}

fn with_hover_cursor(
    mut output: WidgetOutput,
    response: &Response,
    cursor: CursorShape,
) -> WidgetOutput {
    if response.state.hovered && !response.state.disabled {
        output
            .platform_requests
            .push(PlatformRequest::SetCursor(cursor));
    }
    output
}

fn text_input_platform_requests(
    id: WidgetId,
    rect: Rect,
    response: &Response,
    memory: &mut UiMemory,
) -> Vec<PlatformRequest> {
    if response.state.focused && !response.state.disabled {
        memory.set_text_input_owner(id);
        vec![PlatformRequest::StartTextInput { rect: Some(rect) }]
    } else if memory.owns_text_input(id) {
        memory.clear_text_input_owner();
        vec![PlatformRequest::StopTextInput]
    } else {
        Vec::new()
    }
}

fn single_line_text_events(events: &[TextInputEvent]) -> Vec<TextInputEvent> {
    events
        .iter()
        .filter_map(|event| match event {
            TextInputEvent::Commit(text) => {
                let text = text.replace(['\r', '\n'], "");
                (!text.is_empty()).then_some(TextInputEvent::Commit(text))
            }
            TextInputEvent::Composition { text, selection } => Some(TextInputEvent::Composition {
                text: text.replace(['\r', '\n'], " "),
                selection: *selection,
            }),
            TextInputEvent::CompositionStart => Some(TextInputEvent::CompositionStart),
            TextInputEvent::CompositionEnd => Some(TextInputEvent::CompositionEnd),
        })
        .collect()
}

fn text_events_for_text_field(
    id: WidgetId,
    input: &UiInput,
    multiline: bool,
) -> Vec<TextInputEvent> {
    let mut events = if multiline {
        input.text_events.clone()
    } else {
        single_line_text_events(&input.text_events)
    };
    events.extend(
        input
            .clipboard_text
            .iter()
            .filter(|clipboard| clipboard.target == id)
            .filter_map(|clipboard| clipboard_text_event(&clipboard.text, multiline)),
    );
    events
}

fn clipboard_text_event(text: &str, multiline: bool) -> Option<TextInputEvent> {
    let text = if multiline {
        text.replace("\r\n", "\n").replace('\r', "\n")
    } else {
        text.replace(['\r', '\n'], "")
    };
    (!text.is_empty()).then_some(TextInputEvent::Commit(text))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ClipboardShortcut {
    Copy,
    Cut,
    Paste,
}

fn apply_clipboard_shortcuts(
    id: WidgetId,
    state: &mut TextEditState,
    input: &UiInput,
    platform_requests: &mut Vec<PlatformRequest>,
) {
    for event in &input.keyboard.events {
        match clipboard_shortcut(event) {
            Some(ClipboardShortcut::Copy) => {
                if let Some(selected) = state.selected_text() {
                    platform_requests.push(PlatformRequest::CopyToClipboard(selected.to_owned()));
                }
            }
            Some(ClipboardShortcut::Cut) => {
                if let Some(selected) = state.cut_selection() {
                    platform_requests.push(PlatformRequest::CopyToClipboard(selected));
                }
            }
            Some(ClipboardShortcut::Paste) => {
                platform_requests.push(PlatformRequest::RequestClipboardText { target: id });
            }
            None => {}
        }
    }
}

fn clipboard_shortcut(event: &kinetik_ui_core::KeyEvent) -> Option<ClipboardShortcut> {
    if event.state != KeyState::Pressed
        || event.repeat
        || event.modifiers.alt
        || !(event.modifiers.ctrl || event.modifiers.super_key)
    {
        return None;
    }
    let Key::Character(character) = &event.key else {
        return None;
    };
    match character.to_ascii_lowercase().as_str() {
        "c" => Some(ClipboardShortcut::Copy),
        "x" => Some(ClipboardShortcut::Cut),
        "v" => Some(ClipboardShortcut::Paste),
        _ => None,
    }
}

fn display_text_with_composition(
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
fn byte_prefix_width(text: &str, byte_offset: usize, text_size: f32) -> f32 {
    let end = TextSelection::new(0, byte_offset).range_in(text).end;
    text[..end].chars().count() as f32 * text_size * 0.55
}

fn text_field_layout<'a>(
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

fn single_line_hit_offset(
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

fn multi_line_hit_offset(
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

fn fallback_x_offset(text: &str, x: f32, text_size: f32) -> usize {
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

fn fallback_multiline_offset(
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

fn text_line_fragments(text: &str) -> Vec<(usize, &str)> {
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

fn label_baseline(rect: Rect, theme: &Theme, role: TextRole) -> f32 {
    rect.y + theme.font(role).size
}

fn control_text_origin(rect: Rect, theme: &Theme) -> Point {
    let font = theme.font(TextRole::Label);
    let extra = (rect.height - font.line_height).max(0.0) * 0.5;
    Point::new(
        rect.x + theme.controls.padding_x,
        rect.y + extra + font.size,
    )
}

fn single_line_text_primitives(
    id: WidgetId,
    rect: Rect,
    state: &TextEditState,
    focused: bool,
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
        size: recipe.font.size,
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

    if focused {
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
fn multi_line_text_primitives(
    id: WidgetId,
    rect: Rect,
    state: &TextEditState,
    focused: bool,
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
                size: recipe.font.size,
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

        if focused {
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
            size: recipe.font.size,
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

        if focused && (*line_start..=line_end).contains(&display_caret) {
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

/// Emits a text label.
#[must_use]
pub fn label(rect: Rect, text: impl Into<String>, theme: &Theme) -> WidgetOutput {
    let recipe = theme.label(TextRole::Body, false);
    WidgetOutput::new(
        None,
        vec![Primitive::Text(TextPrimitive {
            layout: None,
            origin: Point::new(rect.x, label_baseline(rect, theme, TextRole::Body)),
            text: text.into(),
            size: recipe.font.size,
            brush: Brush::Solid(recipe.foreground),
        })],
    )
}

/// Emits a push button.
pub fn button(
    id: WidgetId,
    rect: Rect,
    text: impl Into<String>,
    input: &UiInput,
    memory: &mut UiMemory,
    theme: &Theme,
    disabled: bool,
) -> WidgetOutput {
    let response = focusable(id, rect, input, memory, disabled);
    let recipe = theme.button(ComponentState {
        hovered: response.state.hovered,
        pressed: response.state.pressed,
        focused: response.state.focused,
        disabled,
        selected: false,
    });
    let text = text.into();

    with_hover_cursor(
        WidgetOutput::new(
            Some(response),
            vec![
                Primitive::Rect(RectPrimitive {
                    rect,
                    fill: Some(recipe.background),
                    stroke: Some(recipe.border),
                    radius: recipe.radius,
                }),
                Primitive::Text(TextPrimitive {
                    layout: None,
                    origin: control_text_origin(rect, theme),
                    text: text.clone(),
                    size: theme.font(TextRole::Label).size,
                    brush: Brush::Solid(recipe.foreground),
                }),
            ],
        )
        .with_semantic(with_response_state(
            button_semantics(id, rect, text, disabled),
            &response,
        )),
        &response,
        CursorShape::PointingHand,
    )
}

/// Emits a tab header.
#[allow(clippy::too_many_arguments)]
pub fn tab_button(
    id: WidgetId,
    rect: Rect,
    text: impl Into<String>,
    selected: bool,
    input: &UiInput,
    memory: &mut UiMemory,
    theme: &Theme,
    disabled: bool,
) -> WidgetOutput {
    let response = selectable(id, rect, input, memory, selected, disabled);
    let recipe = theme.tab(ComponentState {
        hovered: response.state.hovered,
        pressed: response.state.pressed,
        focused: response.state.focused,
        disabled,
        selected,
    });
    let text = text.into();

    let mut semantics = SemanticNode::new(id, SemanticRole::Tab, rect)
        .with_label(text.clone())
        .focusable(!disabled)
        .with_action(SemanticAction::new(SemanticActionKind::Invoke, "Select"));
    semantics.state.disabled = disabled;
    semantics.state.selected = selected;

    with_hover_cursor(
        WidgetOutput::new(
            Some(response),
            vec![
                Primitive::Rect(RectPrimitive {
                    rect,
                    fill: Some(recipe.background),
                    stroke: Some(recipe.border),
                    radius: recipe.radius,
                }),
                Primitive::Text(TextPrimitive {
                    layout: None,
                    origin: control_text_origin(rect, theme),
                    text,
                    size: theme.font(TextRole::Label).size,
                    brush: Brush::Solid(recipe.foreground),
                }),
                Primitive::Rect(RectPrimitive {
                    rect: Rect::new(
                        rect.x,
                        rect.max_y() - recipe.indicator_thickness,
                        rect.width,
                        recipe.indicator_thickness,
                    ),
                    fill: recipe.indicator,
                    stroke: None,
                    radius: CornerRadius::all(0.0),
                }),
            ],
        )
        .with_semantic(with_response_state(semantics, &response)),
        &response,
        CursorShape::PointingHand,
    )
}

/// Emits a selectable list or table row surface.
#[allow(clippy::too_many_arguments)]
pub fn list_row(
    id: WidgetId,
    rect: Rect,
    text: impl Into<String>,
    selected: bool,
    input: &UiInput,
    memory: &mut UiMemory,
    theme: &Theme,
    disabled: bool,
) -> WidgetOutput {
    let response = selectable(id, rect, input, memory, selected, disabled);
    let recipe = theme.row(ComponentState {
        hovered: response.state.hovered,
        pressed: response.state.pressed,
        focused: response.state.focused,
        disabled,
        selected,
    });
    let text = text.into();

    let mut semantics = SemanticNode::new(id, SemanticRole::ListItem, rect)
        .with_label(text.clone())
        .focusable(!disabled)
        .with_action(SemanticAction::new(SemanticActionKind::Invoke, "Select"));
    semantics.state.disabled = disabled;
    semantics.state.selected = selected;

    with_hover_cursor(
        WidgetOutput::new(
            Some(response),
            vec![
                Primitive::Rect(RectPrimitive {
                    rect,
                    fill: Some(recipe.background),
                    stroke: Some(recipe.border),
                    radius: recipe.radius,
                }),
                Primitive::Text(TextPrimitive {
                    layout: None,
                    origin: control_text_origin(rect, theme),
                    text,
                    size: theme.font(TextRole::Label).size,
                    brush: Brush::Solid(recipe.foreground),
                }),
            ],
        )
        .with_semantic(with_response_state(semantics, &response)),
        &response,
        CursorShape::PointingHand,
    )
}

/// Emits an icon button.
pub fn icon_button(
    id: WidgetId,
    rect: Rect,
    icon: IconId,
    input: &UiInput,
    memory: &mut UiMemory,
    theme: &Theme,
    disabled: bool,
) -> WidgetOutput {
    icon_button_with_label(
        id,
        rect,
        icon,
        format!("Icon {}", icon.raw()),
        input,
        memory,
        theme,
        disabled,
    )
}

/// Emits an icon button with an accessible label.
#[allow(clippy::too_many_arguments)]
pub fn icon_button_with_label(
    id: WidgetId,
    rect: Rect,
    icon: IconId,
    label: impl Into<String>,
    input: &UiInput,
    memory: &mut UiMemory,
    theme: &Theme,
    disabled: bool,
) -> WidgetOutput {
    icon_button_with_optional_library(id, rect, icon, label, None, input, memory, theme, disabled)
}

/// Emits an icon button with icons resolved from a vector icon library.
#[allow(clippy::too_many_arguments)]
pub fn icon_button_with_library(
    id: WidgetId,
    rect: Rect,
    icon: IconId,
    label: impl Into<String>,
    icons: &IconLibrary,
    input: &UiInput,
    memory: &mut UiMemory,
    theme: &Theme,
    disabled: bool,
) -> WidgetOutput {
    icon_button_with_optional_library(
        id,
        rect,
        icon,
        label,
        Some(icons),
        input,
        memory,
        theme,
        disabled,
    )
}

#[allow(clippy::too_many_arguments)]
fn icon_button_with_optional_library(
    id: WidgetId,
    rect: Rect,
    icon: IconId,
    label: impl Into<String>,
    icons: Option<&IconLibrary>,
    input: &UiInput,
    memory: &mut UiMemory,
    theme: &Theme,
    disabled: bool,
) -> WidgetOutput {
    let response = focusable(id, rect, input, memory, disabled);
    let recipe = theme.button(ComponentState {
        hovered: response.state.hovered,
        pressed: response.state.pressed,
        focused: response.state.focused,
        disabled,
        selected: false,
    });
    let icon_rect = fit_box(
        rect,
        kinetik_ui_core::Size::new(theme.controls.icon_size, theme.controls.icon_size),
        kinetik_ui_core::Alignment::Center,
        kinetik_ui_core::Alignment::Center,
    );
    let mut primitives = vec![Primitive::Rect(RectPrimitive {
        rect,
        fill: Some(recipe.background),
        stroke: Some(recipe.border),
        radius: recipe.radius,
    })];
    if let Some(graphic) = icons.and_then(|icons| icons.icon(icon)) {
        primitives.extend(icon_graphic_primitives(
            graphic,
            icon_rect,
            recipe.foreground,
        ));
    } else {
        primitives.extend(missing_icon_primitives(icon_rect, recipe.foreground));
    }

    with_hover_cursor(
        WidgetOutput::new(Some(response), primitives).with_semantic(with_response_state(
            icon_button_semantics(id, rect, label, disabled),
            &response,
        )),
        &response,
        CursorShape::PointingHand,
    )
}

fn icon_graphic_primitives(
    graphic: &IconGraphic,
    rect: Rect,
    color: kinetik_ui_core::Color,
) -> Vec<Primitive> {
    if graphic.view_box.width <= 0.0 || graphic.view_box.height <= 0.0 || graphic.paths.is_empty() {
        return missing_icon_primitives(rect, color);
    }

    let scale = (rect.width / graphic.view_box.width)
        .min(rect.height / graphic.view_box.height)
        .max(0.0);
    if !scale.is_finite() || scale <= 0.0 {
        return missing_icon_primitives(rect, color);
    }

    let target = fit_box(
        rect,
        kinetik_ui_core::Size::new(
            graphic.view_box.width * scale,
            graphic.view_box.height * scale,
        ),
        kinetik_ui_core::Alignment::Center,
        kinetik_ui_core::Alignment::Center,
    );
    let primitives = graphic
        .paths
        .iter()
        .filter(|path| !path.elements.is_empty() && (path.fill || path.stroke_width.is_some()))
        .map(|path| {
            Primitive::Path(PathPrimitive::new(
                path.elements
                    .iter()
                    .copied()
                    .map(|element| {
                        transform_icon_path_element(element, graphic.view_box, target, scale)
                    })
                    .collect::<Vec<_>>(),
                path.fill.then_some(Brush::Solid(color)),
                path.stroke_width
                    .map(|width| Stroke::new((width * scale).max(1.0), Brush::Solid(color))),
            ))
        })
        .collect::<Vec<_>>();

    if primitives.is_empty() {
        missing_icon_primitives(rect, color)
    } else {
        primitives
    }
}

fn transform_icon_path_element(
    element: PathElement,
    view_box: Rect,
    target: Rect,
    scale: f32,
) -> PathElement {
    match element {
        PathElement::MoveTo(point) => {
            PathElement::MoveTo(transform_icon_point(point, view_box, target, scale))
        }
        PathElement::LineTo(point) => {
            PathElement::LineTo(transform_icon_point(point, view_box, target, scale))
        }
        PathElement::QuadTo { ctrl, to } => PathElement::QuadTo {
            ctrl: transform_icon_point(ctrl, view_box, target, scale),
            to: transform_icon_point(to, view_box, target, scale),
        },
        PathElement::CubicTo { ctrl1, ctrl2, to } => PathElement::CubicTo {
            ctrl1: transform_icon_point(ctrl1, view_box, target, scale),
            ctrl2: transform_icon_point(ctrl2, view_box, target, scale),
            to: transform_icon_point(to, view_box, target, scale),
        },
        PathElement::Close => PathElement::Close,
    }
}

fn transform_icon_point(point: Point, view_box: Rect, target: Rect, scale: f32) -> Point {
    Point::new(
        target.x + (point.x - view_box.x) * scale,
        target.y + (point.y - view_box.y) * scale,
    )
}

fn missing_icon_primitives(rect: Rect, color: kinetik_ui_core::Color) -> Vec<Primitive> {
    let size = rect.width.min(rect.height);
    if size <= 0.0 {
        return Vec::new();
    }
    let inset = (size * 0.18).max(1.0);
    let stroke_width = (size * 0.10).clamp(1.0, 2.0);
    let left = rect.x + inset;
    let right = rect.max_x() - inset;
    let top = rect.y + inset;
    let bottom = rect.max_y() - inset;
    let center = rect.center();
    let stroke = Stroke::new(stroke_width, Brush::Solid(color));

    vec![
        Primitive::Path(PathPrimitive::new(
            vec![
                PathElement::MoveTo(Point::new(center.x, top)),
                PathElement::LineTo(Point::new(right, center.y)),
                PathElement::LineTo(Point::new(center.x, bottom)),
                PathElement::LineTo(Point::new(left, center.y)),
                PathElement::Close,
            ],
            None,
            Some(stroke),
        )),
        Primitive::Line(LinePrimitive {
            from: Point::new(left, top),
            to: Point::new(right, bottom),
            stroke,
        }),
    ]
}

/// Emits a checkbox.
pub fn checkbox(
    id: WidgetId,
    rect: Rect,
    checked: bool,
    input: &UiInput,
    memory: &mut UiMemory,
    theme: &Theme,
    disabled: bool,
) -> WidgetOutput {
    checkbox_with_label(
        id, rect, "Checkbox", checked, input, memory, theme, disabled,
    )
}

/// Emits a checkbox with an accessible label.
#[allow(clippy::too_many_arguments)]
pub fn checkbox_with_label(
    id: WidgetId,
    rect: Rect,
    label: impl Into<String>,
    checked: bool,
    input: &UiInput,
    memory: &mut UiMemory,
    theme: &Theme,
    disabled: bool,
) -> WidgetOutput {
    let response = selectable(id, rect, input, memory, checked, disabled);
    let recipe = theme.checkbox(ComponentState {
        hovered: response.state.hovered,
        pressed: response.state.pressed,
        focused: response.state.focused,
        disabled,
        selected: checked,
    });
    let box_rect = Rect::new(rect.x, rect.y, recipe.size, recipe.size);

    with_hover_cursor(
        WidgetOutput::new(
            Some(response),
            vec![Primitive::Rect(RectPrimitive {
                rect: box_rect,
                fill: Some(recipe.fill),
                stroke: Some(recipe.border),
                radius: recipe.radius,
            })],
        )
        .with_semantic(with_response_state(
            checkbox_semantics(id, rect, label, checked, disabled),
            &response,
        )),
        &response,
        CursorShape::PointingHand,
    )
}

/// Emits a radio button.
pub fn radio_button(
    id: WidgetId,
    rect: Rect,
    selected: bool,
    input: &UiInput,
    memory: &mut UiMemory,
    theme: &Theme,
    disabled: bool,
) -> WidgetOutput {
    radio_button_with_label(
        id,
        rect,
        "Radio button",
        selected,
        input,
        memory,
        theme,
        disabled,
    )
}

/// Emits a radio button with an accessible label.
#[allow(clippy::too_many_arguments)]
pub fn radio_button_with_label(
    id: WidgetId,
    rect: Rect,
    label: impl Into<String>,
    selected: bool,
    input: &UiInput,
    memory: &mut UiMemory,
    theme: &Theme,
    disabled: bool,
) -> WidgetOutput {
    let mut output = checkbox_with_label(id, rect, label, selected, input, memory, theme, disabled);
    let recipe = theme.radio_button(ComponentState {
        hovered: output
            .response
            .as_ref()
            .is_some_and(|response| response.state.hovered),
        pressed: output
            .response
            .as_ref()
            .is_some_and(|response| response.state.pressed),
        focused: output
            .response
            .as_ref()
            .is_some_and(|response| response.state.focused),
        disabled,
        selected,
    });
    if let Some(Primitive::Rect(primitive)) = output.primitives.first_mut() {
        primitive.radius = recipe.radius;
    }
    for node in &mut output.semantics {
        node.role = SemanticRole::RadioButton;
        node.state.selected = selected;
    }
    output
}

/// Emits a toggle control.
pub fn toggle(
    id: WidgetId,
    rect: Rect,
    on: bool,
    input: &UiInput,
    memory: &mut UiMemory,
    theme: &Theme,
    disabled: bool,
) -> WidgetOutput {
    toggle_with_label(id, rect, "Toggle", on, input, memory, theme, disabled)
}

/// Emits a toggle control with an accessible label.
#[allow(clippy::too_many_arguments)]
pub fn toggle_with_label(
    id: WidgetId,
    rect: Rect,
    label: impl Into<String>,
    on: bool,
    input: &UiInput,
    memory: &mut UiMemory,
    theme: &Theme,
    disabled: bool,
) -> WidgetOutput {
    let response = selectable(id, rect, input, memory, on, disabled);
    let recipe = theme.toggle(ComponentState {
        hovered: response.state.hovered,
        pressed: response.state.pressed,
        focused: response.state.focused,
        disabled,
        selected: on,
    });
    let knob_x = if on {
        rect.max_x() - rect.height
    } else {
        rect.x
    };

    with_hover_cursor(
        WidgetOutput::new(
            Some(response),
            vec![
                Primitive::Rect(RectPrimitive {
                    rect,
                    fill: Some(recipe.track),
                    stroke: Some(recipe.border),
                    radius: CornerRadius::all(rect.height * 0.5),
                }),
                Primitive::Rect(RectPrimitive {
                    rect: Rect::new(
                        knob_x + recipe.padding,
                        rect.y + recipe.padding,
                        rect.height - recipe.padding * 2.0,
                        rect.height - recipe.padding * 2.0,
                    ),
                    fill: Some(recipe.thumb),
                    stroke: None,
                    radius: CornerRadius::all((rect.height - recipe.padding * 2.0) * 0.5),
                }),
            ],
        )
        .with_semantic(with_response_state(
            toggle_semantics(id, rect, label, on, disabled),
            &response,
        )),
        &response,
        CursorShape::PointingHand,
    )
}

/// Emits a slider and updates its value while active.
#[allow(clippy::too_many_arguments)]
pub fn slider(
    id: WidgetId,
    rect: Rect,
    value: &mut f32,
    range: core::ops::RangeInclusive<f32>,
    input: &UiInput,
    memory: &mut UiMemory,
    theme: &Theme,
    disabled: bool,
) -> WidgetOutput {
    slider_with_label(
        id, rect, "Slider", value, range, input, memory, theme, disabled,
    )
}

/// Emits a slider with an accessible label and updates its value while active.
#[allow(clippy::too_many_arguments)]
pub fn slider_with_label(
    id: WidgetId,
    rect: Rect,
    label: impl Into<String>,
    value: &mut f32,
    range: core::ops::RangeInclusive<f32>,
    input: &UiInput,
    memory: &mut UiMemory,
    theme: &Theme,
    disabled: bool,
) -> WidgetOutput {
    let response = draggable(id, rect, input, memory, disabled);
    if !disabled
        && (response.state.active || response.clicked)
        && let Some(position) = input.pointer.position
    {
        let t = ((position.x - rect.x) / rect.width).clamp(0.0, 1.0);
        let start = *range.start();
        let end = *range.end();
        *value = start + (end - start) * t;
    }
    let start = *range.start();
    let end = *range.end();
    let t = ((*value - start) / (end - start)).clamp(0.0, 1.0);
    let fill_rect = Rect::new(rect.x, rect.y, rect.width * t, rect.height);
    let recipe = theme.slider(ComponentState {
        hovered: response.state.hovered,
        pressed: response.state.pressed,
        focused: response.state.focused,
        disabled,
        selected: false,
    });

    with_hover_cursor(
        WidgetOutput::new(
            Some(response),
            vec![
                Primitive::Rect(RectPrimitive {
                    rect,
                    fill: Some(recipe.track),
                    stroke: Some(recipe.border),
                    radius: recipe.radius,
                }),
                Primitive::Rect(RectPrimitive {
                    rect: fill_rect,
                    fill: Some(recipe.fill),
                    stroke: None,
                    radius: recipe.radius,
                }),
            ],
        )
        .with_semantic(with_response_state(
            slider_semantics(id, rect, label, *value, range, disabled),
            &response,
        )),
        &response,
        CursorShape::ResizeHorizontal,
    )
}

/// Emits a passive panel surface.
#[must_use]
pub fn panel(rect: Rect, theme: &Theme) -> WidgetOutput {
    let recipe = theme.panel();
    let mut primitives = Vec::new();
    if let Some(shadow) = recipe.shadow {
        primitives.push(Primitive::Shadow(shadow.primitive(rect)));
    }
    primitives.push(Primitive::Rect(RectPrimitive {
        rect,
        fill: Some(recipe.background),
        stroke: Some(recipe.border),
        radius: recipe.radius,
    }));
    WidgetOutput::new(None, primitives)
}

/// Resolved panel surface and content body rectangles.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PanelFrame {
    /// Full panel surface rectangle.
    pub outer: Rect,
    /// Inner content body after insets.
    pub body: Rect,
}

impl PanelFrame {
    /// Resolves a panel body from an outer rectangle and content insets.
    #[must_use]
    pub fn new(outer: Rect, body_insets: Insets) -> Self {
        Self {
            outer,
            body: pad_rect(outer, body_insets),
        }
    }
}

/// Emits a simple horizontal separator line.
#[must_use]
pub fn separator(rect: Rect, theme: &Theme) -> Primitive {
    let recipe = theme.separator();
    Primitive::Line(LinePrimitive {
        from: Point::new(rect.x, rect.center().y),
        to: Point::new(rect.max_x(), rect.center().y),
        stroke: recipe.stroke,
    })
}

/// Emits an image primitive for a static icon-like resource.
#[must_use]
pub fn image(rect: Rect, image: ImageId) -> WidgetOutput {
    WidgetOutput::new(None, vec![Primitive::Image(ImagePrimitive { image, rect })])
}

/// Returns semantics for a static label.
#[must_use]
pub fn label_semantics(id: WidgetId, rect: Rect, text: impl Into<String>) -> SemanticNode {
    SemanticNode::new(id, SemanticRole::Label, rect).with_label(text)
}

/// Returns semantics for a push button.
#[must_use]
pub fn button_semantics(
    id: WidgetId,
    rect: Rect,
    text: impl Into<String>,
    disabled: bool,
) -> SemanticNode {
    let mut node = SemanticNode::new(id, SemanticRole::Button, rect)
        .with_label(text)
        .focusable(!disabled)
        .with_action(SemanticAction::new(SemanticActionKind::Invoke, "Invoke"));
    node.state.disabled = disabled;
    node
}

/// Returns semantics for an icon button.
#[must_use]
pub fn icon_button_semantics(
    id: WidgetId,
    rect: Rect,
    label: impl Into<String>,
    disabled: bool,
) -> SemanticNode {
    let mut node = button_semantics(id, rect, label, disabled);
    node.role = SemanticRole::IconButton;
    node
}

/// Returns semantics for a checkbox.
#[must_use]
pub fn checkbox_semantics(
    id: WidgetId,
    rect: Rect,
    label: impl Into<String>,
    checked: bool,
    disabled: bool,
) -> SemanticNode {
    let mut node = SemanticNode::new(id, SemanticRole::CheckBox, rect)
        .with_label(label)
        .focusable(!disabled)
        .with_action(SemanticAction::new(SemanticActionKind::Invoke, "Toggle"));
    node.state = SemanticState {
        disabled,
        checked: Some(checked),
        ..SemanticState::default()
    };
    node
}

/// Returns semantics for a radio button.
#[must_use]
pub fn radio_button_semantics(
    id: WidgetId,
    rect: Rect,
    label: impl Into<String>,
    selected: bool,
    disabled: bool,
) -> SemanticNode {
    let mut node = checkbox_semantics(id, rect, label, selected, disabled);
    node.role = SemanticRole::RadioButton;
    node.state.selected = selected;
    node
}

/// Returns semantics for a toggle control.
#[must_use]
pub fn toggle_semantics(
    id: WidgetId,
    rect: Rect,
    label: impl Into<String>,
    on: bool,
    disabled: bool,
) -> SemanticNode {
    let mut node = checkbox_semantics(id, rect, label, on, disabled);
    node.role = SemanticRole::Toggle;
    node
}

/// Returns semantics for a slider.
#[must_use]
pub fn slider_semantics(
    id: WidgetId,
    rect: Rect,
    label: impl Into<String>,
    value: f32,
    range: core::ops::RangeInclusive<f32>,
    disabled: bool,
) -> SemanticNode {
    let mut node = SemanticNode::new(id, SemanticRole::Slider, rect)
        .with_label(label)
        .focusable(!disabled)
        .with_action(SemanticAction::new(
            SemanticActionKind::Increment,
            "Increase",
        ))
        .with_action(SemanticAction::new(
            SemanticActionKind::Decrement,
            "Decrease",
        ))
        .with_action(SemanticAction::new(
            SemanticActionKind::SetValue,
            "Set value",
        ));
    node.state = SemanticState {
        disabled,
        value: Some(SemanticValue::Number {
            current: value,
            min: *range.start(),
            max: *range.end(),
        }),
        ..SemanticState::default()
    };
    node
}

/// Returns semantics for a text field.
#[must_use]
pub fn text_field_semantics(
    id: WidgetId,
    rect: Rect,
    label: impl Into<String>,
    text: impl Into<String>,
    disabled: bool,
) -> SemanticNode {
    let mut node = SemanticNode::new(id, SemanticRole::TextField, rect)
        .with_label(label)
        .focusable(!disabled)
        .with_action(SemanticAction::new(SemanticActionKind::SetText, "Set text"));
    node.state = SemanticState {
        disabled,
        value: Some(SemanticValue::Text(text.into())),
        ..SemanticState::default()
    };
    node
}

/// Returns semantics for a search field.
#[must_use]
pub fn search_field_semantics(
    id: WidgetId,
    rect: Rect,
    label: impl Into<String>,
    query: impl Into<String>,
    disabled: bool,
) -> SemanticNode {
    let mut node = text_field_semantics(id, rect, label, query, disabled);
    node.role = SemanticRole::SearchField;
    node
}

/// Returns semantics for a passive panel.
#[must_use]
pub fn panel_semantics(id: WidgetId, rect: Rect, label: impl Into<String>) -> SemanticNode {
    SemanticNode::new(id, SemanticRole::Panel, rect).with_label(label)
}

/// Returns semantics for a static image.
#[must_use]
pub fn image_semantics(id: WidgetId, rect: Rect, label: impl Into<String>) -> SemanticNode {
    SemanticNode::new(id, SemanticRole::Image, rect).with_label(label)
}

/// Output emitted by editable text widgets.
#[derive(Debug, Clone, PartialEq)]
pub struct TextFieldOutput {
    /// Base widget output.
    pub widget: WidgetOutput,
    /// Whether the text changed this frame.
    pub changed: bool,
}

/// Emits a single-line text field and applies text input while focused.
pub fn text_field(
    id: WidgetId,
    rect: Rect,
    state: &mut TextEditState,
    input: &UiInput,
    memory: &mut UiMemory,
    theme: &Theme,
    disabled: bool,
) -> TextFieldOutput {
    text_field_with_text_layouts(id, rect, state, input, memory, theme, disabled, None)
}

/// Emits a single-line text field using shaped text layout cache when available.
#[allow(clippy::too_many_arguments)]
pub fn text_field_with_text_layouts(
    id: WidgetId,
    rect: Rect,
    state: &mut TextEditState,
    input: &UiInput,
    memory: &mut UiMemory,
    theme: &Theme,
    disabled: bool,
    mut text_layouts: Option<&mut TextLayoutStore>,
) -> TextFieldOutput {
    let before = state.text.clone();
    let mut response = focusable(id, rect, input, memory, disabled);
    let hit_recipe = theme.text_field(ComponentState {
        hovered: response.state.hovered,
        pressed: response.state.pressed,
        focused: response.state.focused,
        disabled,
        selected: false,
    });
    let (hit_text, _, _) = display_text_with_composition(state);
    if !disabled
        && response.state.hovered
        && input.pointer.primary.pressed
        && let Some(position) = input.pointer.position
    {
        let hit_layout = text_field_layout(
            text_layouts.as_deref_mut(),
            &hit_text,
            rect,
            &hit_recipe,
            false,
        );
        state.set_caret(single_line_hit_offset(
            position,
            rect,
            &hit_text,
            &hit_recipe,
            hit_layout,
        ));
        memory.focus(id);
        response.state.focused = true;
    }
    let mut platform_requests = text_input_platform_requests(id, rect, &response, memory);
    if response.state.focused && !disabled {
        apply_clipboard_shortcuts(id, state, input, &mut platform_requests);
        let text_events = text_events_for_text_field(id, input, false);
        state.apply_input(&text_events, &input.keyboard.events);
    }
    let recipe = theme.text_field(ComponentState {
        hovered: response.state.hovered,
        pressed: response.state.pressed,
        focused: response.state.focused,
        disabled,
        selected: false,
    });
    let (display_text, _, _) = display_text_with_composition(state);
    let layout = text_field_layout(text_layouts, &display_text, rect, &recipe, false);
    let mut primitives = vec![Primitive::Rect(RectPrimitive {
        rect,
        fill: Some(recipe.background),
        stroke: Some(recipe.border),
        radius: recipe.radius,
    })];
    primitives.extend(single_line_text_primitives(
        id,
        rect,
        state,
        response.state.focused && !disabled,
        &recipe,
        layout,
    ));

    TextFieldOutput {
        widget: with_hover_cursor(
            WidgetOutput::new(Some(response), primitives)
                .with_semantic(with_response_state(
                    text_field_semantics(id, rect, "Text field", state.text.clone(), disabled),
                    &response,
                ))
                .with_platform_requests(platform_requests),
            &response,
            CursorShape::Text,
        ),
        changed: before != state.text,
    }
}

/// Output emitted by multi-line text fields.
#[derive(Debug, Clone, PartialEq)]
pub struct MultiLineTextFieldOutput {
    /// Base widget output.
    pub widget: WidgetOutput,
    /// Whether the text changed this frame.
    pub changed: bool,
    /// Visible line count emitted by the widget.
    pub visible_lines: usize,
}

/// Emits a multi-line text field and applies text input while focused.
pub fn multi_line_text_field(
    id: WidgetId,
    rect: Rect,
    state: &mut TextEditState,
    input: &UiInput,
    memory: &mut UiMemory,
    theme: &Theme,
    disabled: bool,
) -> MultiLineTextFieldOutput {
    multi_line_text_field_with_text_layouts(id, rect, state, input, memory, theme, disabled, None)
}

/// Emits a multi-line text field using shaped text layout cache when available.
#[allow(clippy::too_many_arguments)]
pub fn multi_line_text_field_with_text_layouts(
    id: WidgetId,
    rect: Rect,
    state: &mut TextEditState,
    input: &UiInput,
    memory: &mut UiMemory,
    theme: &Theme,
    disabled: bool,
    mut text_layouts: Option<&mut TextLayoutStore>,
) -> MultiLineTextFieldOutput {
    let before = state.text.clone();
    let mut response = focusable(id, rect, input, memory, disabled);
    let hit_recipe = theme.text_field(ComponentState {
        hovered: response.state.hovered,
        pressed: response.state.pressed,
        focused: response.state.focused,
        disabled,
        selected: false,
    });
    let (hit_text, _, _) = display_text_with_composition(state);
    if !disabled
        && response.state.hovered
        && input.pointer.primary.pressed
        && let Some(position) = input.pointer.position
    {
        let hit_layout = text_field_layout(
            text_layouts.as_deref_mut(),
            &hit_text,
            rect,
            &hit_recipe,
            true,
        );
        state.set_caret(multi_line_hit_offset(
            position,
            rect,
            &hit_text,
            &hit_recipe,
            hit_layout,
        ));
        memory.focus(id);
        response.state.focused = true;
    }
    let mut platform_requests = text_input_platform_requests(id, rect, &response, memory);
    if response.state.focused && !disabled {
        apply_clipboard_shortcuts(id, state, input, &mut platform_requests);
        let mut text_events = text_events_for_text_field(id, input, true);
        if input.keyboard.events.iter().any(|event| {
            event.state == KeyState::Pressed
                && event.key == Key::Enter
                && event.modifiers.is_empty()
        }) {
            text_events.push(TextInputEvent::Commit("\n".to_owned()));
        }
        state.apply_input(&text_events, &input.keyboard.events);
    }
    let recipe = theme.text_field(ComponentState {
        hovered: response.state.hovered,
        pressed: response.state.pressed,
        focused: response.state.focused,
        disabled,
        selected: false,
    });
    let (display_text, _, _) = display_text_with_composition(state);
    let layout = text_field_layout(text_layouts, &display_text, rect, &recipe, true);
    let mut primitives = vec![Primitive::Rect(RectPrimitive {
        rect,
        fill: Some(recipe.background),
        stroke: Some(recipe.border),
        radius: recipe.radius,
    })];
    primitives.extend(multi_line_text_primitives(
        id,
        rect,
        state,
        response.state.focused && !disabled,
        &recipe,
        layout,
    ));

    MultiLineTextFieldOutput {
        widget: with_hover_cursor(
            WidgetOutput::new(Some(response), primitives)
                .with_semantic(with_response_state(
                    text_field_semantics(id, rect, "Text field", state.text.clone(), disabled),
                    &response,
                ))
                .with_platform_requests(platform_requests),
            &response,
            CursorShape::Text,
        ),
        changed: before != state.text,
        visible_lines: text_line_fragments(&state.text).len(),
    }
}

/// Output emitted by numeric input.
#[derive(Debug, Clone, PartialEq)]
pub struct NumericInputOutput {
    /// Text field output.
    pub field: TextFieldOutput,
    /// Parsed numeric value, if valid.
    pub value: Option<f32>,
    /// Whether the current text parses as a number.
    pub valid: bool,
}

/// Emits a numeric input field.
pub fn numeric_input(
    id: WidgetId,
    rect: Rect,
    state: &mut TextEditState,
    input: &UiInput,
    memory: &mut UiMemory,
    theme: &Theme,
    disabled: bool,
) -> NumericInputOutput {
    numeric_input_with_text_layouts(id, rect, state, input, memory, theme, disabled, None)
}

/// Emits a numeric input field using shaped text layout cache when available.
#[allow(clippy::too_many_arguments)]
pub fn numeric_input_with_text_layouts(
    id: WidgetId,
    rect: Rect,
    state: &mut TextEditState,
    input: &UiInput,
    memory: &mut UiMemory,
    theme: &Theme,
    disabled: bool,
    text_layouts: Option<&mut TextLayoutStore>,
) -> NumericInputOutput {
    let field = text_field_with_text_layouts(
        id,
        rect,
        state,
        input,
        memory,
        theme,
        disabled,
        text_layouts,
    );
    let value = state.text.trim().parse::<f32>().ok();

    NumericInputOutput {
        field,
        value,
        valid: value.is_some() || state.text.trim().is_empty(),
    }
}

/// Output emitted by search fields.
#[derive(Debug, Clone, PartialEq)]
pub struct SearchFieldOutput {
    /// Text field output.
    pub field: TextFieldOutput,
    /// Current query.
    pub query: String,
    /// Whether the query is empty.
    pub empty: bool,
}

/// Emits a search-oriented text field.
pub fn search_field(
    id: WidgetId,
    rect: Rect,
    state: &mut TextEditState,
    input: &UiInput,
    memory: &mut UiMemory,
    theme: &Theme,
    disabled: bool,
) -> SearchFieldOutput {
    search_field_with_text_layouts(id, rect, state, input, memory, theme, disabled, None)
}

/// Emits a search-oriented text field using shaped text layout cache when available.
#[allow(clippy::too_many_arguments)]
pub fn search_field_with_text_layouts(
    id: WidgetId,
    rect: Rect,
    state: &mut TextEditState,
    input: &UiInput,
    memory: &mut UiMemory,
    theme: &Theme,
    disabled: bool,
    text_layouts: Option<&mut TextLayoutStore>,
) -> SearchFieldOutput {
    let mut field = text_field_with_text_layouts(
        id,
        rect,
        state,
        input,
        memory,
        theme,
        disabled,
        text_layouts,
    );
    let query = state.text.clone();
    for node in &mut field.widget.semantics {
        if node.id == id {
            node.role = SemanticRole::SearchField;
            node.label = Some("Search".to_owned());
        }
    }

    SearchFieldOutput {
        field,
        empty: query.is_empty(),
        query,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        IconId, PanelFrame, button, button_semantics, checkbox, checkbox_semantics,
        checkbox_with_label, icon_button, icon_button_with_label, icon_button_with_library, image,
        label, list_row, multi_line_text_field, multi_line_text_field_with_text_layouts,
        numeric_input, panel, panel_semantics, radio_button_with_label, search_field,
        search_field_semantics, slider, slider_semantics, slider_with_label, tab_button,
        text_field, text_field_semantics, text_field_with_text_layouts, toggle, toggle_with_label,
    };
    use crate::{IconGraphic, IconLibrary, IconPath};
    use kinetik_ui_core::{
        ClipboardText, ImageId, Insets, Key, KeyEvent, KeyState, KeyboardInput, Modifiers,
        PathElement, PlatformRequest, Point, PointerButtonState, PointerInput, Primitive, Rect,
        SemanticActionKind, SemanticRole, SemanticValue, UiInput, UiMemory, WidgetId,
        default_dark_theme,
    };
    use kinetik_ui_text::{TextEditState, TextLayoutStore, TextSelection};

    fn input_at(x: f32, y: f32) -> UiInput {
        UiInput {
            pointer: PointerInput {
                position: Some(Point::new(x, y)),
                ..PointerInput::default()
            },
            ..UiInput::default()
        }
    }

    fn shortcut_input(character: &str) -> UiInput {
        let modifiers = Modifiers::new(false, true, false, false);
        UiInput {
            keyboard: KeyboardInput {
                modifiers,
                events: vec![KeyEvent::new(
                    Key::Character(character.to_owned()),
                    KeyState::Pressed,
                    modifiers,
                    false,
                )],
            },
            ..UiInput::default()
        }
    }

    fn check_icon() -> IconGraphic {
        IconGraphic::new(
            Rect::new(0.0, 0.0, 24.0, 24.0),
            [IconPath::stroked(
                vec![
                    PathElement::MoveTo(Point::new(5.0, 12.0)),
                    PathElement::LineTo(Point::new(10.0, 17.0)),
                    PathElement::LineTo(Point::new(19.0, 7.0)),
                ],
                2.0,
            )],
        )
    }

    #[test]
    fn label_emits_text() {
        let output = label(
            Rect::new(0.0, 0.0, 100.0, 20.0),
            "Name",
            &default_dark_theme(),
        );

        assert!(matches!(output.primitives[0], Primitive::Text(_)));
        assert!(output.response.is_none());
    }

    #[test]
    fn panel_frame_resolves_clamped_body_rect() {
        let frame = PanelFrame::new(Rect::new(10.0, 20.0, 100.0, 50.0), Insets::all(12.0));

        assert_eq!(frame.outer, Rect::new(10.0, 20.0, 100.0, 50.0));
        assert_eq!(frame.body, Rect::new(22.0, 32.0, 76.0, 26.0));

        let clamped = PanelFrame::new(Rect::new(0.0, 0.0, 10.0, 8.0), Insets::all(20.0));
        assert_eq!(clamped.body, Rect::new(20.0, 20.0, 0.0, 0.0));
    }

    #[test]
    fn button_emits_surface_and_text_and_clicks() {
        let theme = default_dark_theme();
        let mut memory = UiMemory::new();
        let id = WidgetId::from_key("button");
        let rect = Rect::new(0.0, 0.0, 80.0, 28.0);
        let mut input = input_at(4.0, 4.0);

        input.pointer.primary = PointerButtonState::new(true, true, false);
        button(id, rect, "Run", &input, &mut memory, &theme, false);
        input.pointer.primary = PointerButtonState::new(false, false, true);
        let output = button(id, rect, "Run", &input, &mut memory, &theme, false);

        assert_eq!(output.primitives.len(), 2);
        assert!(output.response.expect("button response").clicked);
    }

    #[test]
    fn icon_button_emits_vector_fallback_symbol() {
        let output = icon_button(
            WidgetId::from_key("icon"),
            Rect::new(0.0, 0.0, 24.0, 24.0),
            IconId::from_raw(1),
            &UiInput::default(),
            &mut UiMemory::new(),
            &default_dark_theme(),
            false,
        );

        assert_eq!(output.primitives.len(), 3);
        assert!(matches!(output.primitives[0], Primitive::Rect(_)));
        assert!(matches!(output.primitives[1], Primitive::Path(_)));
        assert!(matches!(output.primitives[2], Primitive::Line(_)));
    }

    #[test]
    fn icon_button_with_label_preserves_accessible_name() {
        let output = icon_button_with_label(
            WidgetId::from_key("icon"),
            Rect::new(0.0, 0.0, 24.0, 24.0),
            IconId::from_raw(1),
            "Save project",
            &UiInput::default(),
            &mut UiMemory::new(),
            &default_dark_theme(),
            false,
        );

        assert_eq!(output.semantics[0].role, SemanticRole::IconButton);
        assert_eq!(output.semantics[0].label.as_deref(), Some("Save project"));
    }

    #[test]
    fn icon_button_uses_registered_vector_icon() {
        let mut icons = IconLibrary::new();
        let icon = IconId::from_raw(7);
        icons.register(icon, check_icon());

        let output = icon_button_with_library(
            WidgetId::from_key("icon"),
            Rect::new(0.0, 0.0, 24.0, 24.0),
            icon,
            "Check",
            &icons,
            &UiInput::default(),
            &mut UiMemory::new(),
            &default_dark_theme(),
            false,
        );

        assert!(icons.has_icon(icon));
        assert_eq!(output.primitives.len(), 2);
        assert!(matches!(output.primitives[0], Primitive::Rect(_)));
        assert!(matches!(output.primitives[1], Primitive::Path(_)));
    }

    #[test]
    fn tab_and_row_surfaces_are_not_button_clones() {
        let theme = default_dark_theme();
        let mut memory = UiMemory::new();
        let input = input_at(4.0, 4.0);
        let tab = tab_button(
            WidgetId::from_key("tab"),
            Rect::new(0.0, 0.0, 90.0, 28.0),
            "Tab",
            true,
            &input,
            &mut memory,
            &theme,
            false,
        );
        let row = list_row(
            WidgetId::from_key("row"),
            Rect::new(0.0, 32.0, 140.0, 26.0),
            "Row",
            true,
            &input,
            &mut memory,
            &theme,
            false,
        );

        assert!(tab.response.expect("tab response").state.selected);
        assert!(row.response.expect("row response").state.selected);
        assert_ne!(tab.primitives.len(), row.primitives.len());
    }

    #[test]
    fn checkbox_and_toggle_reflect_selection() {
        let theme = default_dark_theme();
        let mut memory = UiMemory::new();
        let checkbox = checkbox(
            WidgetId::from_key("check"),
            Rect::new(0.0, 0.0, 20.0, 20.0),
            true,
            &input_at(1.0, 1.0),
            &mut memory,
            &theme,
            false,
        );
        let toggle = toggle(
            WidgetId::from_key("toggle"),
            Rect::new(0.0, 0.0, 36.0, 18.0),
            true,
            &UiInput::default(),
            &mut memory,
            &theme,
            false,
        );

        assert!(checkbox.response.expect("checkbox response").state.selected);
        assert_eq!(toggle.primitives.len(), 2);
    }

    #[test]
    fn labeled_controls_preserve_accessible_names() {
        let theme = default_dark_theme();
        let mut memory = UiMemory::new();
        let input = UiInput::default();
        let mut slider_value = 0.5;

        let checkbox = checkbox_with_label(
            WidgetId::from_key("check"),
            Rect::new(0.0, 0.0, 20.0, 20.0),
            "Enable snapping",
            true,
            &input,
            &mut memory,
            &theme,
            false,
        );
        let radio = radio_button_with_label(
            WidgetId::from_key("radio"),
            Rect::new(0.0, 24.0, 20.0, 20.0),
            "Blend mode",
            true,
            &input,
            &mut memory,
            &theme,
            false,
        );
        let toggle = toggle_with_label(
            WidgetId::from_key("toggle"),
            Rect::new(0.0, 48.0, 36.0, 18.0),
            "Loop playback",
            true,
            &input,
            &mut memory,
            &theme,
            false,
        );
        let slider = slider_with_label(
            WidgetId::from_key("slider"),
            Rect::new(0.0, 72.0, 100.0, 12.0),
            "Brush opacity",
            &mut slider_value,
            0.0..=1.0,
            &input,
            &mut memory,
            &theme,
            false,
        );

        assert_eq!(
            checkbox.semantics[0].label.as_deref(),
            Some("Enable snapping")
        );
        assert_eq!(radio.semantics[0].role, SemanticRole::RadioButton);
        assert_eq!(radio.semantics[0].label.as_deref(), Some("Blend mode"));
        assert_eq!(toggle.semantics[0].label.as_deref(), Some("Loop playback"));
        assert_eq!(slider.semantics[0].label.as_deref(), Some("Brush opacity"));
    }

    #[test]
    fn slider_updates_value_from_pointer_position() {
        let theme = default_dark_theme();
        let id = WidgetId::from_key("slider");
        let rect = Rect::new(0.0, 0.0, 100.0, 12.0);
        let mut memory = UiMemory::new();
        let mut value = 0.0;
        let mut input = input_at(50.0, 6.0);

        input.pointer.primary = PointerButtonState::new(true, true, false);
        slider(
            id,
            rect,
            &mut value,
            0.0..=1.0,
            &input,
            &mut memory,
            &theme,
            false,
        );

        assert!((value - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn panel_emits_shadow_and_surface_before_image_primitives_stay_single() {
        let panel = panel(Rect::new(0.0, 0.0, 10.0, 10.0), &default_dark_theme());

        assert!(matches!(panel.primitives[0], Primitive::Shadow(_)));
        assert!(matches!(panel.primitives[1], Primitive::Rect(_)));
        assert!(matches!(
            image(Rect::new(0.0, 0.0, 10.0, 10.0), ImageId::from_raw(1)).primitives[0],
            Primitive::Image(_)
        ));
    }

    #[test]
    fn text_field_applies_input_while_focused() {
        let theme = default_dark_theme();
        let id = WidgetId::from_key("text");
        let mut memory = UiMemory::new();
        memory.focus(id);
        let mut state = TextEditState::new("");
        let input = UiInput {
            text_events: vec![kinetik_ui_core::TextInputEvent::Commit("a".to_owned())],
            ..UiInput::default()
        };

        let output = text_field(
            id,
            Rect::new(0.0, 0.0, 80.0, 24.0),
            &mut state,
            &input,
            &mut memory,
            &theme,
            false,
        );

        assert!(output.changed);
        assert_eq!(state.text, "a");
    }

    #[test]
    fn text_field_copies_selected_text_through_platform_request() {
        let theme = default_dark_theme();
        let id = WidgetId::from_key("text");
        let mut memory = UiMemory::new();
        memory.focus(id);
        let mut state = TextEditState::new("abcd");
        state.set_selection(TextSelection::new(1, 3));
        let input = shortcut_input("c");

        let output = text_field(
            id,
            Rect::new(0.0, 0.0, 80.0, 24.0),
            &mut state,
            &input,
            &mut memory,
            &theme,
            false,
        );

        assert!(!output.changed);
        assert_eq!(state.text, "abcd");
        assert!(output.widget.platform_requests.iter().any(|request| {
            matches!(request, PlatformRequest::CopyToClipboard(text) if text == "bc")
        }));
    }

    #[test]
    fn text_field_cuts_selected_text_through_platform_request_and_undo() {
        let theme = default_dark_theme();
        let id = WidgetId::from_key("text");
        let mut memory = UiMemory::new();
        memory.focus(id);
        let mut state = TextEditState::new("abcd");
        state.set_selection(TextSelection::new(1, 3));
        let input = shortcut_input("x");

        let output = text_field(
            id,
            Rect::new(0.0, 0.0, 80.0, 24.0),
            &mut state,
            &input,
            &mut memory,
            &theme,
            false,
        );

        assert!(output.changed);
        assert_eq!(state.text, "ad");
        assert!(output.widget.platform_requests.iter().any(|request| {
            matches!(request, PlatformRequest::CopyToClipboard(text) if text == "bc")
        }));
        assert!(state.undo());
        assert_eq!(state.text, "abcd");
    }

    #[test]
    fn text_field_requests_targeted_clipboard_text_on_paste() {
        let theme = default_dark_theme();
        let id = WidgetId::from_key("text");
        let mut memory = UiMemory::new();
        memory.focus(id);
        let mut state = TextEditState::new("abcd");
        state.set_caret(2);
        let input = shortcut_input("v");

        let output = text_field(
            id,
            Rect::new(0.0, 0.0, 80.0, 24.0),
            &mut state,
            &input,
            &mut memory,
            &theme,
            false,
        );

        assert!(!output.changed);
        assert_eq!(state.text, "abcd");
        assert!(output.widget.platform_requests.iter().any(|request| {
            matches!(request, PlatformRequest::RequestClipboardText { target } if *target == id)
        }));
    }

    #[test]
    fn text_field_applies_only_targeted_clipboard_text() {
        let theme = default_dark_theme();
        let id = WidgetId::from_key("text");
        let other = WidgetId::from_key("other");
        let mut memory = UiMemory::new();
        memory.focus(id);
        let mut state = TextEditState::new("a");
        state.set_caret(1);
        let input = UiInput {
            clipboard_text: vec![
                ClipboardText::new(other, "wrong"),
                ClipboardText::new(id, "b\nc"),
            ],
            ..UiInput::default()
        };

        let output = text_field(
            id,
            Rect::new(0.0, 0.0, 80.0, 24.0),
            &mut state,
            &input,
            &mut memory,
            &theme,
            false,
        );

        assert!(output.changed);
        assert_eq!(state.text, "abc");
        assert!(state.undo());
        assert_eq!(state.text, "a");
    }

    #[test]
    fn text_field_places_caret_from_pointer_press_with_shaped_layout() {
        let theme = default_dark_theme();
        let id = WidgetId::from_key("text");
        let rect = Rect::new(0.0, 0.0, 180.0, 28.0);
        let mut memory = UiMemory::new();
        let mut state = TextEditState::new("abcdef");
        let mut text_layouts = TextLayoutStore::new();
        let mut input = input_at(rect.max_x() - 4.0, 12.0);
        input.pointer.primary = PointerButtonState::new(true, true, false);

        let output = text_field_with_text_layouts(
            id,
            rect,
            &mut state,
            &input,
            &mut memory,
            &theme,
            false,
            Some(&mut text_layouts),
        );

        assert_eq!(state.caret(), state.text.len());
        assert!(
            output
                .widget
                .response
                .as_ref()
                .expect("text field response")
                .state
                .focused
        );
        assert!(!text_layouts.is_empty());
    }

    #[test]
    fn multi_line_text_field_accepts_enter_while_focused() {
        let theme = default_dark_theme();
        let id = WidgetId::from_key("multiline");
        let mut memory = UiMemory::new();
        memory.focus(id);
        let mut state = TextEditState::new("first");
        let input = UiInput {
            keyboard: kinetik_ui_core::KeyboardInput {
                modifiers: Modifiers::default(),
                events: vec![KeyEvent::new(
                    Key::Enter,
                    KeyState::Pressed,
                    Modifiers::default(),
                    false,
                )],
            },
            ..UiInput::default()
        };

        let output = multi_line_text_field(
            id,
            Rect::new(0.0, 0.0, 180.0, 80.0),
            &mut state,
            &input,
            &mut memory,
            &theme,
            false,
        );

        assert!(output.changed);
        assert!(state.text.ends_with('\n'));
        assert!(
            output
                .widget
                .primitives
                .iter()
                .any(|primitive| matches!(primitive, Primitive::ClipBegin { .. }))
        );
    }

    #[test]
    fn multi_line_text_field_places_caret_on_clicked_line() {
        let theme = default_dark_theme();
        let id = WidgetId::from_key("multiline");
        let rect = Rect::new(0.0, 0.0, 180.0, 80.0);
        let mut memory = UiMemory::new();
        let mut state = TextEditState::new("one\ntwo");
        let mut text_layouts = TextLayoutStore::new();
        let mut input = input_at(rect.max_x() - 4.0, 42.0);
        input.pointer.primary = PointerButtonState::new(true, true, false);

        multi_line_text_field_with_text_layouts(
            id,
            rect,
            &mut state,
            &input,
            &mut memory,
            &theme,
            false,
            Some(&mut text_layouts),
        );

        assert_eq!(state.caret(), state.text.len());
    }

    #[test]
    fn numeric_input_reports_parse_state() {
        let theme = default_dark_theme();
        let mut state = TextEditState::new("42");
        let output = numeric_input(
            WidgetId::from_key("number"),
            Rect::new(0.0, 0.0, 80.0, 24.0),
            &mut state,
            &UiInput::default(),
            &mut UiMemory::new(),
            &theme,
            false,
        );

        assert!(output.valid);
        assert_eq!(output.value, Some(42.0));
    }

    #[test]
    fn search_field_reports_query() {
        let theme = default_dark_theme();
        let mut state = TextEditState::new("media");
        let output = search_field(
            WidgetId::from_key("search"),
            Rect::new(0.0, 0.0, 80.0, 24.0),
            &mut state,
            &UiInput::default(),
            &mut UiMemory::new(),
            &theme,
            false,
        );

        assert_eq!(output.query, "media");
        assert!(!output.empty);
    }

    #[test]
    fn widget_semantics_map_roles_states_values_and_actions() {
        let button = button_semantics(
            WidgetId::from_key("button"),
            Rect::new(0.0, 0.0, 80.0, 24.0),
            "Analyze",
            false,
        );
        let checkbox = checkbox_semantics(
            WidgetId::from_key("checkbox"),
            Rect::new(0.0, 28.0, 20.0, 20.0),
            "Enabled",
            true,
            false,
        );
        let slider = slider_semantics(
            WidgetId::from_key("slider"),
            Rect::new(0.0, 56.0, 100.0, 12.0),
            "Strength",
            0.62,
            0.0..=1.0,
            false,
        );
        let field = text_field_semantics(
            WidgetId::from_key("field"),
            Rect::new(0.0, 72.0, 120.0, 24.0),
            "Name",
            "Project",
            false,
        );
        let search = search_field_semantics(
            WidgetId::from_key("search"),
            Rect::new(0.0, 100.0, 120.0, 24.0),
            "Search",
            "media",
            false,
        );
        let panel = panel_semantics(
            WidgetId::from_key("panel"),
            Rect::new(0.0, 0.0, 200.0, 200.0),
            "Inspector",
        );

        assert_eq!(button.role, SemanticRole::Button);
        assert!(button.focusable);
        assert!(
            button
                .actions
                .iter()
                .any(|action| action.kind == SemanticActionKind::Invoke)
        );
        assert_eq!(checkbox.state.checked, Some(true));
        assert!(matches!(
            slider.state.value,
            Some(SemanticValue::Number { current, .. }) if (current - 0.62).abs() < f32::EPSILON
        ));
        assert!(
            matches!(field.state.value, Some(SemanticValue::Text(ref text)) if text == "Project")
        );
        assert_eq!(search.role, SemanticRole::SearchField);
        assert_eq!(panel.role, SemanticRole::Panel);
    }
}
