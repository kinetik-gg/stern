use kinetik_ui_core::{
    ActionContext, ActionDescriptor, ActionInvocation, ActionQueue, ActionSource, Rect, Size,
};

use super::{
    OverlayDismissal, OverlayEntry, OverlayId, OverlayKind, OverlayStack, PopoverPlacement,
    PopoverRequest, placement::placed_entry,
};
/// Menu item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuItem {
    /// Non-interactive label.
    Label(String),
    /// Visual separator.
    Separator,
    /// Action-backed item.
    Action(ActionDescriptor),
}

/// Menu model.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Menu {
    items: Vec<MenuItem>,
}

impl Menu {
    /// Creates an empty menu.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds an item.
    pub fn push(&mut self, item: MenuItem) {
        self.items.push(item);
    }

    /// Builds a menu from action descriptors.
    #[must_use]
    pub fn from_actions(actions: impl IntoIterator<Item = ActionDescriptor>) -> Self {
        Self {
            items: actions.into_iter().map(MenuItem::Action).collect(),
        }
    }

    /// Returns visible menu items.
    #[must_use]
    pub fn visible_items(&self) -> Vec<&MenuItem> {
        self.visible_items_iter().collect()
    }

    /// Returns visible menu items as a borrowed iterator.
    pub fn visible_items_iter(&self) -> impl Iterator<Item = &MenuItem> + '_ {
        self.items.iter().filter(|item| match item {
            MenuItem::Action(action) => action.state.visible,
            MenuItem::Label(_) | MenuItem::Separator => true,
        })
    }

    /// Invokes an enabled visible action item by visible index.
    pub fn invoke_visible(
        &self,
        visible_index: usize,
        queue: &mut ActionQueue,
        context: ActionContext,
    ) -> bool {
        self.invoke_visible_from(visible_index, queue, ActionSource::Menu, context)
    }

    /// Invokes an enabled visible action item by visible index from a menu-like source.
    pub fn invoke_visible_from(
        &self,
        visible_index: usize,
        queue: &mut ActionQueue,
        source: ActionSource,
        context: ActionContext,
    ) -> bool {
        let Some(invocation) = self.invocation_for_visible(visible_index, source, context) else {
            return false;
        };
        queue.push(invocation);
        true
    }

    /// Creates an invocation for an enabled visible action item by visible index.
    #[must_use]
    pub fn invocation_for_visible(
        &self,
        visible_index: usize,
        source: ActionSource,
        context: ActionContext,
    ) -> Option<ActionInvocation> {
        let Some(MenuItem::Action(action)) = self.visible_items_iter().nth(visible_index) else {
            return None;
        };
        if !action.can_invoke() {
            return None;
        }
        Some(ActionInvocation::new(action.id.clone(), source, context))
    }
}

/// Action-backed menu-like overlay model.
#[derive(Debug, Clone, PartialEq)]
pub struct MenuOverlay {
    /// Overlay stack entry for placement, z-order, focus, and dismissal.
    pub entry: OverlayEntry,
    /// Menu items displayed by the overlay.
    pub menu: Menu,
    /// Source used for action invocations emitted by this surface.
    pub source: ActionSource,
    /// Context captured for action invocations emitted by this surface.
    pub context: ActionContext,
}

impl MenuOverlay {
    /// Creates a menu overlay from an existing stack entry and menu model.
    #[must_use]
    pub const fn new(
        entry: OverlayEntry,
        menu: Menu,
        source: ActionSource,
        context: ActionContext,
    ) -> Self {
        Self {
            entry,
            menu,
            source,
            context,
        }
    }

    /// Creates a placed menu-like overlay from an anchor rectangle.
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub fn anchored(
        id: OverlayId,
        kind: OverlayKind,
        menu: Menu,
        anchor: Rect,
        size: Size,
        placement: PopoverPlacement,
        offset: f32,
        fit_viewport: bool,
        viewport: Rect,
        dismissal: OverlayDismissal,
        source: ActionSource,
        context: ActionContext,
    ) -> Self {
        Self::new(
            placed_entry(
                id,
                kind,
                PopoverRequest {
                    anchor,
                    size,
                    placement,
                    offset,
                    fit_viewport,
                },
                viewport,
            )
            .dismiss_on(dismissal),
            menu,
            source,
            context,
        )
    }

    /// Opens this overlay at the top of an overlay stack.
    pub fn open_in(&self, stack: &mut OverlayStack) {
        stack.open(self.entry.clone());
    }

    /// Opens this overlay as a child of an existing overlay.
    pub fn open_child_in(&self, stack: &mut OverlayStack, parent: OverlayId) -> bool {
        stack.open_child(parent, self.entry.clone())
    }

    /// Returns visible menu items.
    #[must_use]
    pub fn visible_items(&self) -> Vec<&MenuItem> {
        self.menu.visible_items()
    }

    /// Returns visible menu items as a borrowed iterator.
    pub fn visible_items_iter(&self) -> impl Iterator<Item = &MenuItem> + '_ {
        self.menu.visible_items_iter()
    }

    /// Creates an invocation for an enabled visible action item.
    #[must_use]
    pub fn invocation_for_visible(&self, visible_index: usize) -> Option<ActionInvocation> {
        self.menu
            .invocation_for_visible(visible_index, self.source, self.context.clone())
    }

    /// Invokes an enabled visible action item into an action queue.
    pub fn invoke_visible(&self, visible_index: usize, queue: &mut ActionQueue) -> bool {
        let Some(invocation) = self.invocation_for_visible(visible_index) else {
            return false;
        };
        queue.push(invocation);
        true
    }
}
