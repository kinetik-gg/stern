use crate::{Rect, Vec2, WidgetId};

/// Interaction state flags for a widget.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)]
pub struct InteractionState {
    /// Pointer is over the widget.
    pub hovered: bool,
    /// Widget has keyboard focus.
    pub focused: bool,
    /// Widget is active for pointer/modal interaction.
    pub active: bool,
    /// Widget is currently pressed.
    pub pressed: bool,
    /// Widget is disabled.
    pub disabled: bool,
    /// Widget is selected.
    pub selected: bool,
}

/// Common response returned by interaction primitives and components.
#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(clippy::struct_excessive_bools)]
pub struct Response {
    /// Widget identity.
    pub id: WidgetId,
    /// Widget bounds.
    pub rect: Rect,
    /// Interaction state.
    pub state: InteractionState,
    /// Whether the widget was clicked this frame.
    pub clicked: bool,
    /// Whether the widget was double-clicked this frame.
    pub double_clicked: bool,
    /// Whether the secondary pointer button clicked this frame.
    pub secondary_clicked: bool,
    /// Whether the widget was dragged this frame.
    pub dragged: bool,
    /// Whether the widget was activated by keyboard this frame.
    pub keyboard_activated: bool,
    /// Whether the widget requested a context menu this frame.
    pub context_requested: bool,
    /// Whether the widget requested a tooltip this frame.
    pub tooltip_requested: bool,
    /// Drag delta in logical units.
    pub drag_delta: Vec2,
}

impl Response {
    /// Creates a response with default event flags.
    #[must_use]
    pub const fn new(id: WidgetId, rect: Rect, state: InteractionState) -> Self {
        Self {
            id,
            rect,
            state,
            clicked: false,
            double_clicked: false,
            secondary_clicked: false,
            dragged: false,
            keyboard_activated: false,
            context_requested: false,
            tooltip_requested: false,
            drag_delta: Vec2::ZERO,
        }
    }
}

/// Scrollable behavior output.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScrollResponse {
    /// Common interaction response for the scrollable region.
    pub response: Response,
    /// Clamped retained scroll offset after applying input.
    pub offset: Vec2,
    /// Offset delta applied during this frame.
    pub delta: Vec2,
    /// Maximum legal scroll offset.
    pub max_offset: Vec2,
}

/// Drop target behavior output.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DropTargetResponse {
    /// Common interaction response for the target region.
    pub response: Response,
    /// Eligible active or released drag source over this target.
    pub source: Option<WidgetId>,
    /// Whether an eligible source was released over this target this frame.
    pub dropped: bool,
}
