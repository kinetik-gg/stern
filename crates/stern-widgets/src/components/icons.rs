use super::{
    BTreeMap, Brush, ButtonFocusPlacement, ComponentState, CursorShape, IconId, ImageId,
    ImagePrimitive, LinePrimitive, PathElement, PathPrimitive, Point, Primitive, Rect, Stroke,
    Theme, UiInput, UiMemory, WidgetId, WidgetOutput, button_surface_primitives,
    clicked_select_state, fit_box, focusable, icon_button_semantics, response_reported_focus,
    response_reported_pressed, suppress_disabled_interaction_reporting, with_hover_cursor,
    with_response_state,
};

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

/// Emits an icon button backed by a bitmap image resource.
#[allow(clippy::too_many_arguments)]
pub fn image_icon_button(
    id: WidgetId,
    rect: Rect,
    image: ImageId,
    label: impl Into<String>,
    input: &UiInput,
    memory: &mut UiMemory,
    theme: &Theme,
    disabled: bool,
) -> WidgetOutput {
    image_icon_button_sized(
        id,
        rect,
        image,
        label,
        theme.controls.icon_size,
        input,
        memory,
        theme,
        disabled,
    )
}

/// Emits a bitmap icon button with an explicit icon side length.
#[allow(clippy::too_many_arguments)]
pub fn image_icon_button_sized(
    id: WidgetId,
    rect: Rect,
    image: ImageId,
    label: impl Into<String>,
    icon_size: f32,
    input: &UiInput,
    memory: &mut UiMemory,
    theme: &Theme,
    disabled: bool,
) -> WidgetOutput {
    image_icon_selectable_button_sized(
        id, rect, image, label, false, icon_size, input, memory, theme, disabled,
    )
}

/// Emits a selectable icon button backed by a bitmap image resource.
#[allow(clippy::too_many_arguments)]
pub fn image_icon_selectable_button(
    id: WidgetId,
    rect: Rect,
    image: ImageId,
    label: impl Into<String>,
    selected: bool,
    input: &UiInput,
    memory: &mut UiMemory,
    theme: &Theme,
    disabled: bool,
) -> WidgetOutput {
    image_icon_selectable_button_sized(
        id,
        rect,
        image,
        label,
        selected,
        theme.controls.icon_size,
        input,
        memory,
        theme,
        disabled,
    )
}

/// Emits a selectable bitmap icon button with an explicit icon side length.
#[allow(clippy::too_many_arguments)]
pub fn image_icon_selectable_button_sized(
    id: WidgetId,
    rect: Rect,
    image: ImageId,
    label: impl Into<String>,
    selected: bool,
    icon_size: f32,
    input: &UiInput,
    memory: &mut UiMemory,
    theme: &Theme,
    disabled: bool,
) -> WidgetOutput {
    let mut response = focusable(id, rect, input, memory, disabled);
    let selected = clicked_select_state(selected, response.clicked);
    response.state.selected = selected;
    let state = ComponentState {
        hovered: response.state.hovered,
        pressed: response.state.pressed,
        focused: response.state.focused,
        disabled,
        selected,
    };
    let recipe = theme.button(state);
    let icon_size = sanitized_icon_size(icon_size, theme.controls.icon_size);
    let icon_rect = fit_box(
        rect,
        stern_core::Size::new(icon_size, icon_size),
        stern_core::Alignment::Center,
        stern_core::Alignment::Center,
    );
    let mut semantics = icon_button_semantics(id, rect, label, disabled);
    semantics.state.selected = selected;

    let mut primitives = button_surface_primitives(
        theme,
        &recipe,
        state,
        rect,
        recipe.radius,
        ButtonFocusPlacement::Inward,
    );
    primitives.push(Primitive::Image(ImagePrimitive {
        image,
        rect: icon_rect,
        tint: None,
    }));
    with_hover_cursor(
        WidgetOutput::new(Some(response), primitives)
            .with_semantic(with_response_state(semantics, &response)),
        &response,
        CursorShape::PointingHand,
    )
}

fn sanitized_icon_size(size: f32, fallback: f32) -> f32 {
    if size.is_finite() && size > 0.0 {
        size
    } else {
        fallback
    }
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
    let mut response = focusable(id, rect, input, memory, disabled);
    suppress_disabled_interaction_reporting(&mut response);
    let state = ComponentState {
        hovered: response.state.hovered,
        pressed: response_reported_pressed(&response),
        focused: response_reported_focus(&response),
        disabled,
        selected: false,
    };
    let recipe = theme.button(state);
    let icon_rect = fit_box(
        rect,
        stern_core::Size::new(theme.controls.icon_size, theme.controls.icon_size),
        stern_core::Alignment::Center,
        stern_core::Alignment::Center,
    );
    let mut primitives = button_surface_primitives(
        theme,
        &recipe,
        state,
        rect,
        recipe.radius,
        ButtonFocusPlacement::Inward,
    );
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
    color: stern_core::Color,
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
        stern_core::Size::new(
            graphic.view_box.width * scale,
            graphic.view_box.height * scale,
        ),
        stern_core::Alignment::Center,
        stern_core::Alignment::Center,
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

fn missing_icon_primitives(rect: Rect, color: stern_core::Color) -> Vec<Primitive> {
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
