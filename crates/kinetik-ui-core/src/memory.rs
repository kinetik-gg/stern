//! Retained UI memory.

use std::collections::{HashMap, HashSet};

use crate::{Vec2, WidgetId};

/// Retained interaction and widget state owned by the UI runtime.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct UiMemory {
    /// Widget hovered during the current frame.
    pub hovered: Option<WidgetId>,
    /// Widget with keyboard focus.
    pub focused: Option<WidgetId>,
    /// Widget currently active for modal-like interaction.
    pub active: Option<WidgetId>,
    /// Widget currently pressed.
    pub pressed: Option<WidgetId>,
    scroll_offsets: HashMap<WidgetId, Vec2>,
    open_popovers: HashSet<WidgetId>,
}

impl UiMemory {
    /// Creates empty UI memory.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Clears transient frame state while preserving retained widget state.
    pub fn begin_frame(&mut self) {
        self.hovered = None;
    }

    /// Clears interaction capture state at the end of an interaction.
    pub fn clear_interaction(&mut self) {
        self.active = None;
        self.pressed = None;
    }

    /// Returns the scroll offset for a widget.
    #[must_use]
    pub fn scroll_offset(&self, id: WidgetId) -> Vec2 {
        self.scroll_offsets.get(&id).copied().unwrap_or(Vec2::ZERO)
    }

    /// Sets the scroll offset for a widget.
    pub fn set_scroll_offset(&mut self, id: WidgetId, offset: Vec2) {
        self.scroll_offsets.insert(id, offset);
    }

    /// Returns true when a popover is open for a widget.
    #[must_use]
    pub fn is_popover_open(&self, id: WidgetId) -> bool {
        self.open_popovers.contains(&id)
    }

    /// Opens a popover for a widget.
    pub fn open_popover(&mut self, id: WidgetId) {
        self.open_popovers.insert(id);
    }

    /// Closes a popover for a widget.
    pub fn close_popover(&mut self, id: WidgetId) {
        self.open_popovers.remove(&id);
    }
}

#[cfg(test)]
mod tests {
    use super::UiMemory;
    use crate::{Vec2, WidgetId};

    #[test]
    fn starts_empty() {
        let memory = UiMemory::new();

        assert_eq!(memory.hovered, None);
        assert_eq!(memory.focused, None);
        assert_eq!(memory.active, None);
        assert_eq!(memory.pressed, None);
    }

    #[test]
    fn begin_frame_clears_hover_but_preserves_focus() {
        let id = WidgetId::from_key("field");
        let mut memory = UiMemory::new();
        memory.hovered = Some(id);
        memory.focused = Some(id);

        memory.begin_frame();

        assert_eq!(memory.hovered, None);
        assert_eq!(memory.focused, Some(id));
    }

    #[test]
    fn clears_interaction_state() {
        let id = WidgetId::from_key("button");
        let mut memory = UiMemory::new();
        memory.active = Some(id);
        memory.pressed = Some(id);

        memory.clear_interaction();

        assert_eq!(memory.active, None);
        assert_eq!(memory.pressed, None);
    }

    #[test]
    fn stores_scroll_offsets_by_widget_id() {
        let id = WidgetId::from_key("scroll");
        let mut memory = UiMemory::new();

        assert_eq!(memory.scroll_offset(id), Vec2::ZERO);
        memory.set_scroll_offset(id, Vec2::new(10.0, 20.0));
        assert_eq!(memory.scroll_offset(id), Vec2::new(10.0, 20.0));
    }

    #[test]
    fn tracks_open_popovers() {
        let id = WidgetId::from_key("menu");
        let mut memory = UiMemory::new();

        assert!(!memory.is_popover_open(id));
        memory.open_popover(id);
        assert!(memory.is_popover_open(id));
        memory.close_popover(id);
        assert!(!memory.is_popover_open(id));
    }
}
