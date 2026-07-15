use super::{
    CornerRadius, CursorShape, PlatformRequest, Point, Primitive, Rect, Response, SemanticNode,
    TextRole, Theme,
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
