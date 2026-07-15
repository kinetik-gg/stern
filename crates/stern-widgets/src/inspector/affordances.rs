use stern_core::{
    Brush, ComponentState, CornerRadius, CursorShape, PlatformRequest, Point, Primitive, Rect,
    Response, SemanticAction, SemanticActionKind, SemanticNode, SemanticRole, SemanticValue,
    TextPrimitive, TextRole, Theme, UiInput, UiMemory, WidgetId, focusable,
};

use crate::{
    WidgetOutput,
    components::{ButtonFocusPlacement, button_surface_primitives},
};

use super::layout::PropertyGridRowRect;
use super::row::PropertyGridRow;
use super::util::finite_non_negative;

/// Layout tuning for compact property-row affordance controls.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PropertyGridAffordanceLayout {
    /// Square control size for reset and keyframe controls.
    pub button_size: f32,
    /// Gap between controls and the value editor.
    pub gap: f32,
    /// Minimum value-editor width preserved before controls are shown.
    pub min_value_width: f32,
}

impl PropertyGridAffordanceLayout {
    /// Creates property affordance layout tuning.
    #[must_use]
    pub const fn new(button_size: f32, gap: f32) -> Self {
        Self {
            button_size,
            gap,
            min_value_width: 40.0,
        }
    }

    /// Sets the minimum value-editor width preserved before controls are shown.
    #[must_use]
    pub const fn with_min_value_width(mut self, min_value_width: f32) -> Self {
        self.min_value_width = min_value_width;
        self
    }
}

impl Default for PropertyGridAffordanceLayout {
    fn default() -> Self {
        Self::new(18.0, 4.0)
    }
}

/// Rectangles assigned to property-row value and affordance controls.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PropertyGridAffordanceRects {
    /// Value/control rectangle after reserving visible affordance controls.
    pub value_rect: Rect,
    /// Reset-to-default control rectangle, when visible.
    pub reset_rect: Option<Rect>,
    /// Keyframe toggle control rectangle, when visible.
    pub keyframe_rect: Option<Rect>,
}

/// Output from property-row affordance controls.
#[derive(Debug, Clone, PartialEq)]
pub struct PropertyGridAffordanceOutput {
    /// Widget primitives, semantics, and platform requests emitted by controls.
    pub widget: WidgetOutput,
    /// True when the user requested an app-owned reset-to-default operation.
    pub reset_requested: bool,
    /// True when the user requested an app-owned keyframe toggle operation.
    pub keyframe_toggle_requested: bool,
    /// Keyed state requested by the keyframe toggle, without storing animation data.
    pub requested_keyed: bool,
    /// Reset control response, when visible.
    pub reset_response: Option<Response>,
    /// Keyframe control response, when visible.
    pub keyframe_response: Option<Response>,
}

/// Computes compact reset/keyframe affordance rectangles for one value cell.
#[must_use]
pub fn property_grid_row_affordance_rects(
    row: &PropertyGridRow,
    value_rect: Rect,
    layout: PropertyGridAffordanceLayout,
) -> PropertyGridAffordanceRects {
    let button_size = finite_non_negative(layout.button_size)
        .min(finite_non_negative(value_rect.height))
        .min(finite_non_negative(value_rect.width));
    let gap = finite_non_negative(layout.gap).min(finite_non_negative(value_rect.width));
    let min_value_width = finite_non_negative(layout.min_value_width);
    let mut cursor = value_rect.max_x();

    let keyframe_rect = if row.state.affordances.keyframe.available
        && can_reserve_affordance(cursor, value_rect.x, button_size, gap, min_value_width)
    {
        cursor -= button_size;
        let rect = Rect::new(
            cursor,
            value_rect.y + (value_rect.height - button_size).max(0.0) * 0.5,
            button_size,
            button_size,
        );
        cursor -= gap.min((cursor - value_rect.x).max(0.0));
        Some(rect)
    } else {
        None
    };

    let reset_rect = if row.state.affordances.reset.available
        && can_reserve_affordance(cursor, value_rect.x, button_size, gap, min_value_width)
    {
        cursor -= button_size.min((cursor - value_rect.x).max(0.0));
        let width = button_size.min((value_rect.max_x() - cursor).max(0.0));
        let rect = Rect::new(
            cursor,
            value_rect.y + (value_rect.height - button_size).max(0.0) * 0.5,
            width,
            button_size,
        );
        cursor -= gap.min((cursor - value_rect.x).max(0.0));
        Some(rect)
    } else {
        None
    };

    PropertyGridAffordanceRects {
        value_rect: Rect::new(
            value_rect.x,
            value_rect.y,
            (cursor - value_rect.x).max(0.0),
            value_rect.height,
        ),
        reset_rect,
        keyframe_rect,
    }
}

fn can_reserve_affordance(
    cursor: f32,
    value_x: f32,
    button_size: f32,
    gap: f32,
    min_value_width: f32,
) -> bool {
    button_size > 0.0 && cursor - value_x >= button_size + gap + min_value_width
}

/// Emits compact property-row reset and keyframe controls.
#[must_use]
pub fn property_grid_row_affordance_controls(
    id: WidgetId,
    row: &PropertyGridRow,
    rects: PropertyGridAffordanceRects,
    input: &UiInput,
    memory: &mut UiMemory,
    theme: &Theme,
) -> PropertyGridAffordanceOutput {
    let mut widget = WidgetOutput::new(None, Vec::new());
    let mut reset_response = None;
    let mut keyframe_response = None;
    let mut reset_requested = false;
    let mut keyframe_toggle_requested = false;
    let mut requested_keyed = row.state.affordances.keyframe.keyed;

    if let Some(rect) = rects.reset_rect {
        let disabled = !row.can_request_reset();
        let response = affordance_button(
            &mut widget,
            id.child("reset"),
            rect,
            format!("Reset {} to default", row.label),
            "Reset",
            "R",
            false,
            disabled,
            input,
            memory,
            theme,
        );
        reset_requested = !disabled && response.clicked;
        reset_response = Some(response);
    }

    if let Some(rect) = rects.keyframe_rect {
        let disabled = !row.can_request_keyframe_toggle();
        let response = affordance_button(
            &mut widget,
            id.child("keyframe"),
            rect,
            format!("Toggle keyframe for {}", row.label),
            "Toggle keyframe",
            "K",
            row.state.affordances.keyframe.keyed,
            disabled,
            input,
            memory,
            theme,
        );
        keyframe_toggle_requested = !disabled && response.clicked;
        if keyframe_toggle_requested {
            requested_keyed = !row.state.affordances.keyframe.keyed;
        }
        keyframe_response = Some(response);
    }

    PropertyGridAffordanceOutput {
        widget,
        reset_requested,
        keyframe_toggle_requested,
        requested_keyed,
        reset_response,
        keyframe_response,
    }
}

/// Builds deterministic semantic metadata for a property-grid row status.
#[must_use]
pub fn property_grid_row_status_semantics(
    id: WidgetId,
    row: &PropertyGridRow,
    row_rect: PropertyGridRowRect,
) -> Option<SemanticNode> {
    let status_text = row.state.status.semantic_text()?;
    let mut node = SemanticNode::new(id.child("status"), SemanticRole::Label, row_rect.rect)
        .with_label(format!("{} status", row.label));
    node.description = Some(status_text.clone());
    node.state.value = Some(SemanticValue::Text(status_text));
    node.state.disabled = row.state.disabled;
    Some(node)
}

#[allow(clippy::too_many_arguments)]
fn affordance_button(
    widget: &mut WidgetOutput,
    id: WidgetId,
    rect: Rect,
    label: String,
    action_label: &'static str,
    glyph: &'static str,
    selected: bool,
    disabled: bool,
    input: &UiInput,
    memory: &mut UiMemory,
    theme: &Theme,
) -> Response {
    let mut response = focusable(id, rect, input, memory, disabled);
    suppress_disabled_interaction_reporting(&mut response);
    response.state.selected = selected;
    let state = ComponentState {
        hovered: response.state.hovered,
        pressed: response.state.pressed && !response.state.disabled,
        focused: response.state.focused && !response.state.disabled,
        disabled,
        selected,
    };
    let recipe = theme.button(state);

    widget.primitives.extend(button_surface_primitives(
        theme,
        &recipe,
        state,
        rect,
        CornerRadius::all(3.0),
        ButtonFocusPlacement::Inward,
    ));
    widget.primitives.push(Primitive::Text(TextPrimitive {
        layout: None,
        origin: Point::new(
            rect.x + (rect.width * 0.5 - theme.font(TextRole::Label).size * 0.28).max(1.0),
            rect.y
                + (rect.height - theme.font(TextRole::Label).line_height).max(0.0) * 0.5
                + theme.font(TextRole::Label).size,
        ),
        text: glyph.to_owned(),
        family: theme.font(TextRole::Label).family.to_owned(),
        size: theme.font(TextRole::Label).size,
        line_height: theme.font(TextRole::Label).line_height,
        brush: Brush::Solid(recipe.foreground),
    }));

    let mut node = SemanticNode::new(id, SemanticRole::IconButton, rect)
        .with_label(label)
        .focusable(!disabled);
    node.state.disabled = disabled;
    node.state.focused = response.state.focused && !response.state.disabled;
    node.state.pressed = response.state.pressed && !response.state.disabled;
    node.state.selected = selected;
    if !disabled {
        node.actions.push(SemanticAction::new(
            SemanticActionKind::Invoke,
            action_label,
        ));
    }
    widget.semantics.push(node);

    if response.state.hovered && !response.state.disabled {
        widget
            .platform_requests
            .push(PlatformRequest::SetCursor(CursorShape::PointingHand));
    }

    response
}

fn suppress_disabled_interaction_reporting(response: &mut Response) {
    if response.state.disabled {
        response.state.focused = false;
        response.state.active = false;
        response.state.pressed = false;
    }
}
