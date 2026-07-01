use crate::{FrameTab, PanelId};

/// Direction for keyboard-style tab strip movement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabStripMove {
    /// Move to the previous tab, wrapping at the beginning.
    Previous,
    /// Move to the next tab, wrapping at the end.
    Next,
}

/// Stable target for tab strip focus, activation, close, or drag affordances.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TabStripTarget {
    /// Panel identity owned by the tab.
    pub panel: PanelId,
    /// Presentation index in the ordered tab strip.
    pub index: usize,
}

impl TabStripTarget {
    /// Creates a tab strip target.
    #[must_use]
    pub const fn new(panel: PanelId, index: usize) -> Self {
        Self { panel, index }
    }
}

/// Data-only tab strip model over ordered frame tab presentation records.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TabStrip {
    tabs: Vec<FrameTab>,
}

impl TabStrip {
    /// Creates an empty tab strip.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a tab strip from ordered frame tab presentation records.
    #[must_use]
    pub fn from_tabs(tabs: impl IntoIterator<Item = FrameTab>) -> Self {
        Self {
            tabs: tabs.into_iter().collect(),
        }
    }

    /// Creates a tab strip from `frame_tabs` output.
    #[must_use]
    pub fn from_frame_tabs(tabs: impl IntoIterator<Item = FrameTab>) -> Self {
        Self::from_tabs(tabs)
    }

    /// Returns ordered tab presentation records.
    #[must_use]
    pub fn tabs(&self) -> &[FrameTab] {
        &self.tabs
    }

    /// Replaces ordered tab presentation records.
    pub fn replace_tabs(&mut self, tabs: impl IntoIterator<Item = FrameTab>) {
        self.tabs = tabs.into_iter().collect();
    }

    /// Returns the number of tabs.
    #[must_use]
    pub fn len(&self) -> usize {
        self.tabs.len()
    }

    /// Returns true when there are no tabs.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.tabs.is_empty()
    }

    /// Returns a tab by presentation index.
    #[must_use]
    pub fn tab(&self, index: usize) -> Option<&FrameTab> {
        self.tabs.get(index)
    }

    /// Returns a tab by stable panel identity.
    #[must_use]
    pub fn tab_by_panel(&self, panel: PanelId) -> Option<&FrameTab> {
        self.index_by_panel(panel)
            .and_then(|index| self.tabs.get(index))
    }

    /// Returns the first active tab index, if any.
    #[must_use]
    pub fn active_index(&self) -> Option<usize> {
        self.tabs.iter().position(|tab| tab.active)
    }

    /// Returns the first active tab, if any.
    #[must_use]
    pub fn active_tab(&self) -> Option<&FrameTab> {
        self.active_index().and_then(|index| self.tabs.get(index))
    }

    /// Returns the active panel identity, if any.
    #[must_use]
    pub fn active_panel(&self) -> Option<PanelId> {
        self.active_tab().map(|tab| tab.panel)
    }

    /// Returns an activation target by presentation index.
    #[must_use]
    pub fn activation_target_by_index(&self, index: usize) -> Option<TabStripTarget> {
        self.tabs
            .get(index)
            .map(|tab| TabStripTarget::new(tab.panel, index))
    }

    /// Returns an activation target by stable panel identity.
    #[must_use]
    pub fn activation_target_by_panel(&self, panel: PanelId) -> Option<TabStripTarget> {
        self.index_by_panel(panel)
            .map(|index| TabStripTarget::new(panel, index))
    }

    /// Returns a focus target by presentation index.
    #[must_use]
    pub fn focus_target_by_index(&self, index: usize) -> Option<TabStripTarget> {
        self.activation_target_by_index(index)
    }

    /// Returns a focus target by stable panel identity.
    #[must_use]
    pub fn focus_target_by_panel(&self, panel: PanelId) -> Option<TabStripTarget> {
        self.activation_target_by_panel(panel)
    }

    /// Returns the previous tab activation target with deterministic wrapping.
    #[must_use]
    pub fn previous_target(&self) -> Option<TabStripTarget> {
        self.movement_target(TabStripMove::Previous)
    }

    /// Returns the next tab activation target with deterministic wrapping.
    #[must_use]
    pub fn next_target(&self) -> Option<TabStripTarget> {
        self.movement_target(TabStripMove::Next)
    }

    /// Returns a tab activation target for keyboard-style movement.
    #[must_use]
    pub fn movement_target(&self, movement: TabStripMove) -> Option<TabStripTarget> {
        let len = self.tabs.len();
        if len == 0 {
            return None;
        }

        let index = match (self.active_index(), movement) {
            (Some(0) | None, TabStripMove::Previous) => len - 1,
            (Some(index), TabStripMove::Previous) => index - 1,
            (Some(index), TabStripMove::Next) => (index + 1) % len,
            (None, TabStripMove::Next) => 0,
        };
        self.activation_target_by_index(index)
    }

    /// Returns a close target by presentation index when the close affordance is visible.
    #[must_use]
    pub fn close_target_by_index(&self, index: usize) -> Option<TabStripTarget> {
        self.tabs
            .get(index)
            .filter(|tab| tab.close_visible)
            .map(|tab| TabStripTarget::new(tab.panel, index))
    }

    /// Returns a close target by stable panel identity when the close affordance is visible.
    #[must_use]
    pub fn close_target_by_panel(&self, panel: PanelId) -> Option<TabStripTarget> {
        let index = self.index_by_panel(panel)?;
        self.close_target_by_index(index)
    }

    /// Returns a drag target by presentation index when the drag affordance is enabled.
    #[must_use]
    pub fn drag_target_by_index(&self, index: usize) -> Option<TabStripTarget> {
        self.tabs
            .get(index)
            .filter(|tab| tab.draggable)
            .map(|tab| TabStripTarget::new(tab.panel, index))
    }

    /// Returns a drag target by stable panel identity when the drag affordance is enabled.
    #[must_use]
    pub fn drag_target_by_panel(&self, panel: PanelId) -> Option<TabStripTarget> {
        let index = self.index_by_panel(panel)?;
        self.drag_target_by_index(index)
    }

    fn index_by_panel(&self, panel: PanelId) -> Option<usize> {
        self.tabs.iter().position(|tab| tab.panel == panel)
    }
}
