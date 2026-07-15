use super::{
    ComponentState, CornerRadius, CursorShape, PlatformRequest, Point, Primitive, Rect,
    RectPrimitive, Response, SemanticNode, TextRole, Theme,
};
use stern_core::{ButtonRecipe, RowRecipe, TabRecipe};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ButtonFocusPlacement {
    Inward,
}

pub(crate) fn button_surface_primitives(
    theme: &Theme,
    recipe: &ButtonRecipe,
    state: ComponentState,
    rect: Rect,
    radius: CornerRadius,
    placement: ButtonFocusPlacement,
) -> Vec<Primitive> {
    let mut primitives = vec![Primitive::Rect(RectPrimitive {
        rect,
        fill: Some(recipe.background),
        stroke: Some(recipe.border),
        radius,
    })];
    if state.focused
        && !state.disabled
        && let Some(focus) = theme.focus_ring(true)
    {
        let annuli = match placement {
            ButtonFocusPlacement::Inward => {
                focus.inward_annulus_primitives(rect, radius, recipe.border.width)
            }
        };
        primitives.extend(annuli);
    }
    primitives
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TabFocusPlacement {
    Inward,
}

pub(crate) fn tab_surface_primitives(
    theme: &Theme,
    recipe: &TabRecipe,
    state: ComponentState,
    rect: Rect,
    radius: CornerRadius,
    placement: TabFocusPlacement,
) -> Vec<Primitive> {
    let mut primitives = vec![Primitive::Rect(RectPrimitive {
        rect,
        fill: Some(recipe.background),
        stroke: Some(recipe.border),
        radius,
    })];
    if state.focused
        && !state.disabled
        && let Some(focus) = theme.focus_ring(true)
    {
        let annuli = match placement {
            TabFocusPlacement::Inward => {
                focus.inward_annulus_primitives(rect, radius, recipe.border.width)
            }
        };
        primitives.extend(annuli);
    }
    primitives
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RowFocusPlacement {
    Inward,
}

pub(crate) fn row_surface_primitives(
    theme: &Theme,
    recipe: &RowRecipe,
    state: ComponentState,
    rect: Rect,
    radius: CornerRadius,
    placement: RowFocusPlacement,
) -> Vec<Primitive> {
    let mut primitives = vec![Primitive::Rect(RectPrimitive {
        rect,
        fill: Some(recipe.background),
        stroke: Some(recipe.border),
        radius,
    })];
    if state.focused
        && !state.disabled
        && let Some(focus) = theme.focus_ring(true)
    {
        let annuli = match placement {
            RowFocusPlacement::Inward => {
                focus.inward_annulus_primitives(rect, radius, recipe.border.width)
            }
        };
        primitives.extend(annuli);
    }
    primitives
}

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

pub(super) fn response_reported_focus(response: &Response) -> bool {
    response.state.focused && !response.state.disabled
}

pub(super) fn response_reported_pressed(response: &Response) -> bool {
    response.state.pressed && !response.state.disabled
}

pub(super) fn suppress_disabled_interaction_reporting(response: &mut Response) {
    if response.state.disabled {
        response.state.focused = false;
        response.state.active = false;
        response.state.pressed = false;
    }
}

pub(super) fn push_focus_ring(
    primitives: &mut Vec<Primitive>,
    theme: &Theme,
    visible: bool,
    rect: Rect,
    radius: CornerRadius,
) {
    if let Some(recipe) = theme.focus_ring(visible) {
        primitives.extend(recipe.primitives(rect, radius));
    }
}

pub(super) fn with_response_state(mut node: SemanticNode, response: &Response) -> SemanticNode {
    node.state.disabled = response.state.disabled;
    node.state.focused = response_reported_focus(response);
    node.state.pressed = response_reported_pressed(response);
    node.state.selected = response.state.selected;
    node
}

pub(super) fn with_hover_cursor(
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

pub(super) fn label_baseline(rect: Rect, theme: &Theme, role: TextRole) -> f32 {
    rect.y + theme.font(role).size
}

pub(super) fn control_text_origin(rect: Rect, theme: &Theme) -> Point {
    let font = theme.font(TextRole::Label);
    let extra = (rect.height - font.line_height).max(0.0) * 0.5;
    Point::new(
        rect.x + theme.controls.padding_x,
        rect.y + extra + font.size,
    )
}

pub(super) fn clicked_toggle_state(selected: bool, clicked: bool) -> bool {
    if clicked { !selected } else { selected }
}

pub(super) fn clicked_select_state(selected: bool, clicked: bool) -> bool {
    selected || clicked
}

#[cfg(test)]
mod button_focus_tests {
    use super::{ButtonFocusPlacement, button_surface_primitives};
    use stern_core::{
        Brush, ButtonVariant, Color, ComponentState, CornerRadius, PathElement, Point, Primitive,
        Rect, default_dark_theme,
    };

    fn endpoint(element: &PathElement) -> Option<Point> {
        match *element {
            PathElement::MoveTo(point)
            | PathElement::LineTo(point)
            | PathElement::QuadTo { to: point, .. }
            | PathElement::CubicTo { to: point, .. } => Some(point),
            PathElement::Close => None,
        }
    }

    fn winding_at(elements: &[PathElement], point: Point) -> i32 {
        let mut winding = 0;
        let mut current = Point::ZERO;
        let mut start = Point::ZERO;
        for element in elements {
            if let PathElement::MoveTo(to) = *element {
                current = to;
                start = to;
                continue;
            }
            let to = if matches!(element, PathElement::Close) {
                start
            } else {
                endpoint(element).expect("drawable path endpoint")
            };
            let cross = (to.x - current.x) * (point.y - current.y)
                - (point.x - current.x) * (to.y - current.y);
            if current.y <= point.y && to.y > point.y && cross > 0.0 {
                winding += 1;
            } else if current.y > point.y && to.y <= point.y && cross < 0.0 {
                winding -= 1;
            }
            current = to;
        }
        winding
    }

    #[test]
    fn button_surface_uses_exact_inward_order_and_suppresses_disabled_focus() {
        let theme = default_dark_theme();
        let rect = Rect::new(4.25, 6.75, 18.0, 18.0);
        let radius = CornerRadius::all(3.0);
        let focused = ComponentState {
            focused: true,
            ..ComponentState::default()
        };
        let recipe = theme.button_variant(ButtonVariant::Standard, focused);
        let primitives = button_surface_primitives(
            &theme,
            &recipe,
            focused,
            rect,
            radius,
            ButtonFocusPlacement::Inward,
        );
        let expected = theme
            .focus_ring(true)
            .expect("focus recipe")
            .inward_annulus_primitives(rect, radius, recipe.border.width);

        assert_eq!(primitives.len(), 3);
        let Primitive::Rect(base) = &primitives[0] else {
            panic!("neutral base first");
        };
        assert_eq!(base.rect, rect);
        assert_eq!(base.fill, Some(recipe.background));
        assert_eq!(base.stroke, Some(recipe.border));
        assert_eq!(base.radius, radius);
        assert_eq!(primitives[1], expected[0]);
        assert_eq!(primitives[2], expected[1]);

        let unfocused_state = ComponentState::default();
        let unfocused_recipe = theme.button_variant(ButtonVariant::Standard, unfocused_state);
        let unfocused = button_surface_primitives(
            &theme,
            &unfocused_recipe,
            unfocused_state,
            rect,
            radius,
            ButtonFocusPlacement::Inward,
        );
        assert_eq!(unfocused.len(), 1);
        let disabled = ComponentState {
            focused: true,
            disabled: true,
            ..ComponentState::default()
        };
        let disabled_recipe = theme.button_variant(ButtonVariant::Standard, disabled);
        assert_eq!(
            button_surface_primitives(
                &theme,
                &disabled_recipe,
                disabled,
                rect,
                radius,
                ButtonFocusPlacement::Inward,
            )
            .len(),
            1
        );
    }

    #[test]
    fn ghost_surface_keeps_a_transparent_hole_and_focus_is_state_independent() {
        let theme = default_dark_theme();
        let rect = Rect::new(10.25, 20.5, 36.0, 22.0);
        let radius = theme.radii.sm;
        let baseline = ComponentState {
            focused: true,
            ..ComponentState::default()
        };
        let recipe = theme.button_variant(ButtonVariant::Ghost, baseline);
        assert_eq!(recipe.background, Brush::Solid(Color::TRANSPARENT));
        let output = button_surface_primitives(
            &theme,
            &recipe,
            baseline,
            rect,
            radius,
            ButtonFocusPlacement::Inward,
        );
        for primitive in &output[1..] {
            let Primitive::Path(path) = primitive else {
                panic!("focus paint must be a hollow compound path");
            };
            assert_eq!(winding_at(&path.elements, rect.center()), 0);
            assert_eq!(path.stroke, None);
        }

        for state in [
            ComponentState {
                focused: true,
                hovered: true,
                ..ComponentState::default()
            },
            ComponentState {
                focused: true,
                pressed: true,
                ..ComponentState::default()
            },
            ComponentState {
                focused: true,
                selected: true,
                ..ComponentState::default()
            },
        ] {
            let state_recipe = theme.button_variant(ButtonVariant::Ghost, state);
            let state_output = button_surface_primitives(
                &theme,
                &state_recipe,
                state,
                rect,
                radius,
                ButtonFocusPlacement::Inward,
            );
            assert_eq!(state_output[1..], output[1..]);
        }
    }
}

#[cfg(test)]
mod tab_focus_tests {
    use super::{TabFocusPlacement, tab_surface_primitives};
    use stern_core::{
        Brush, ComponentState, CornerRadius, PathElement, Primitive, Rect, default_dark_theme,
    };

    fn path_bounds(elements: &[PathElement]) -> Rect {
        let mut min_x = f32::INFINITY;
        let mut min_y = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut max_y = f32::NEG_INFINITY;
        for point in elements.iter().flat_map(|element| match *element {
            PathElement::MoveTo(point) | PathElement::LineTo(point) => vec![point],
            PathElement::QuadTo { ctrl, to } => vec![ctrl, to],
            PathElement::CubicTo { ctrl1, ctrl2, to } => vec![ctrl1, ctrl2, to],
            PathElement::Close => Vec::new(),
        }) {
            min_x = min_x.min(point.x);
            min_y = min_y.min(point.y);
            max_x = max_x.max(point.x);
            max_y = max_y.max(point.y);
        }
        Rect::new(min_x, min_y, max_x - min_x, max_y - min_y)
    }

    #[test]
    fn tab_surface_uses_exact_inward_order_and_suppresses_disabled_focus() {
        let theme = default_dark_theme();
        let rect = Rect::new(4.25, 6.75, 42.5, 22.25);
        let radius = CornerRadius::all(0.0);
        let focused = ComponentState {
            focused: true,
            selected: true,
            ..ComponentState::default()
        };
        let recipe = stern_core::Theme::tab(&theme, focused);
        let primitives = tab_surface_primitives(
            &theme,
            &recipe,
            focused,
            rect,
            radius,
            TabFocusPlacement::Inward,
        );
        let expected = theme
            .focus_ring(true)
            .expect("focus recipe")
            .inward_annulus_primitives(rect, radius, recipe.border.width);

        assert_eq!(primitives.len(), 3);
        let Primitive::Rect(base) = &primitives[0] else {
            panic!("neutral tab base first");
        };
        assert_eq!(base.rect, rect);
        assert_eq!(base.fill, Some(recipe.background));
        assert_eq!(base.stroke, Some(recipe.border));
        assert_eq!(base.radius, radius);
        assert_eq!(
            base.stroke.unwrap().brush,
            Brush::Solid(theme.colors.border.default)
        );
        assert_eq!(primitives[1], expected[0]);
        assert_eq!(primitives[2], expected[1]);

        for state in [
            ComponentState::default(),
            ComponentState {
                focused: true,
                disabled: true,
                ..ComponentState::default()
            },
        ] {
            let recipe = stern_core::Theme::tab(&theme, state);
            assert_eq!(
                tab_surface_primitives(
                    &theme,
                    &recipe,
                    state,
                    rect,
                    radius,
                    TabFocusPlacement::Inward,
                )
                .len(),
                1
            );
        }
    }

    #[test]
    fn narrow_and_degenerate_tab_annuli_remain_finite_contained_compound_paths() {
        let theme = default_dark_theme();
        let state = ComponentState {
            focused: true,
            ..ComponentState::default()
        };
        let recipe = stern_core::Theme::tab(&theme, state);
        for rect in [
            Rect::new(0.25, 0.75, 3.5, 2.0),
            Rect::new(4.25, 8.75, 0.0, 0.0),
        ] {
            let output = tab_surface_primitives(
                &theme,
                &recipe,
                state,
                rect,
                recipe.radius,
                TabFocusPlacement::Inward,
            );
            assert_eq!(output.len(), 3);
            for primitive in &output[1..] {
                let Primitive::Path(path) = primitive else {
                    panic!("tab focus must remain a hollow compound path");
                };
                assert_eq!(path.elements.len(), 20);
                assert_eq!(path.stroke, None);
                assert!(path.elements.iter().all(|element| match *element {
                    PathElement::MoveTo(point) | PathElement::LineTo(point) =>
                        point.x.is_finite() && point.y.is_finite(),
                    PathElement::QuadTo { ctrl, to } =>
                        ctrl.x.is_finite()
                            && ctrl.y.is_finite()
                            && to.x.is_finite()
                            && to.y.is_finite(),
                    PathElement::CubicTo { ctrl1, ctrl2, to } =>
                        ctrl1.x.is_finite()
                            && ctrl1.y.is_finite()
                            && ctrl2.x.is_finite()
                            && ctrl2.y.is_finite()
                            && to.x.is_finite()
                            && to.y.is_finite(),
                    PathElement::Close => true,
                }));
                assert!(rect.contains_rect(path_bounds(&path.elements)));
            }
        }
    }
}

#[cfg(test)]
mod row_focus_tests {
    use super::{RowFocusPlacement, row_surface_primitives};
    use stern_core::{ComponentState, PathElement, Primitive, Rect, default_dark_theme};

    fn path_bounds(elements: &[PathElement]) -> Rect {
        let mut min_x = f32::INFINITY;
        let mut min_y = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut max_y = f32::NEG_INFINITY;
        for point in elements.iter().flat_map(|element| match *element {
            PathElement::MoveTo(point) | PathElement::LineTo(point) => vec![point],
            PathElement::QuadTo { ctrl, to } => vec![ctrl, to],
            PathElement::CubicTo { ctrl1, ctrl2, to } => vec![ctrl1, ctrl2, to],
            PathElement::Close => Vec::new(),
        }) {
            min_x = min_x.min(point.x);
            min_y = min_y.min(point.y);
            max_x = max_x.max(point.x);
            max_y = max_y.max(point.y);
        }
        Rect::new(min_x, min_y, max_x - min_x, max_y - min_y)
    }

    #[test]
    fn row_surface_uses_exact_inward_order_and_suppresses_disabled_focus() {
        let theme = default_dark_theme();
        let rect = Rect::new(4.25, 6.75, 92.5, 22.25);
        let focused = ComponentState {
            focused: true,
            selected: true,
            ..ComponentState::default()
        };
        let recipe = theme.row(focused);
        let primitives = row_surface_primitives(
            &theme,
            &recipe,
            focused,
            rect,
            recipe.radius,
            RowFocusPlacement::Inward,
        );
        let expected = theme
            .focus_ring(true)
            .expect("focus recipe")
            .inward_annulus_primitives(rect, recipe.radius, recipe.border.width);

        assert_eq!(primitives.len(), 3);
        let Primitive::Rect(base) = &primitives[0] else {
            panic!("neutral row base first");
        };
        assert_eq!(base.rect, rect);
        assert_eq!(base.fill, Some(recipe.background));
        assert_eq!(base.stroke, Some(recipe.border));
        assert_eq!(base.radius, recipe.radius);
        assert_eq!(primitives[1], expected[0]);
        assert_eq!(primitives[2], expected[1]);

        for state in [
            ComponentState::default(),
            ComponentState {
                focused: true,
                disabled: true,
                ..ComponentState::default()
            },
        ] {
            let recipe = theme.row(state);
            assert_eq!(
                row_surface_primitives(
                    &theme,
                    &recipe,
                    state,
                    rect,
                    recipe.radius,
                    RowFocusPlacement::Inward,
                )
                .len(),
                1
            );
        }
    }

    #[test]
    fn narrow_and_degenerate_row_annuli_remain_finite_contained_compound_paths() {
        let theme = default_dark_theme();
        let state = ComponentState {
            focused: true,
            ..ComponentState::default()
        };
        let recipe = theme.row(state);
        for rect in [
            Rect::new(0.25, 0.75, 3.5, 2.0),
            Rect::new(4.25, 8.75, 0.0, 0.0),
        ] {
            let output = row_surface_primitives(
                &theme,
                &recipe,
                state,
                rect,
                recipe.radius,
                RowFocusPlacement::Inward,
            );
            assert_eq!(output.len(), 3);
            for primitive in &output[1..] {
                let Primitive::Path(path) = primitive else {
                    panic!("row focus must remain a hollow compound path");
                };
                assert_eq!(path.elements.len(), 20);
                assert_eq!(path.stroke, None);
                assert!(path.elements.iter().all(|element| match *element {
                    PathElement::MoveTo(point) | PathElement::LineTo(point) =>
                        point.x.is_finite() && point.y.is_finite(),
                    PathElement::QuadTo { ctrl, to } =>
                        ctrl.x.is_finite()
                            && ctrl.y.is_finite()
                            && to.x.is_finite()
                            && to.y.is_finite(),
                    PathElement::CubicTo { ctrl1, ctrl2, to } =>
                        ctrl1.x.is_finite()
                            && ctrl1.y.is_finite()
                            && ctrl2.x.is_finite()
                            && ctrl2.y.is_finite()
                            && to.x.is_finite()
                            && to.y.is_finite(),
                    PathElement::Close => true,
                }));
                assert!(rect.contains_rect(path_bounds(&path.elements)));
            }
        }
    }
}
