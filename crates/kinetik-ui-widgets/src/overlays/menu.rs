use kinetik_ui_core::{
    ActionContext, ActionDescriptor, ActionId, ActionInvocation, ActionQueue, ActionSource, Rect,
    Size,
};

use super::{
    OverlayDismissal, OverlayEntry, OverlayId, OverlayKind, OverlayNavigationInput, OverlayStack,
    PopoverPlacement, PopoverRequest, TypeaheadBuffer,
    navigation::{moved_index, typeahead_index},
    placement::placed_entry,
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct MenuEntry {
    item: MenuItem,
    submenu: Option<Menu>,
}

impl MenuEntry {
    fn item_is_visible(&self) -> bool {
        match &self.item {
            MenuItem::Action(action) => action.state.visible,
            MenuItem::Label(_) | MenuItem::Separator => true,
        }
    }

    fn navigable_action(&self) -> Option<&ActionDescriptor> {
        match &self.item {
            MenuItem::Action(action) if action.can_invoke() => Some(action),
            MenuItem::Label(_) | MenuItem::Separator | MenuItem::Action(_) => None,
        }
    }
}

/// Menu model.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Menu {
    entries: Vec<MenuEntry>,
    highlighted: Option<ActionId>,
}

impl Menu {
    /// Creates an empty menu.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds an item.
    pub fn push(&mut self, item: MenuItem) {
        self.entries.push(MenuEntry {
            item,
            submenu: None,
        });
    }

    /// Adds an action-backed submenu trigger without adding a breaking `MenuItem` variant.
    pub fn push_submenu(&mut self, trigger: ActionDescriptor, submenu: Menu) {
        self.entries.push(MenuEntry {
            item: MenuItem::Action(trigger),
            submenu: Some(submenu),
        });
    }

    /// Replaces flat items and reconciles any highlighted action identity.
    pub fn replace_items(&mut self, items: impl IntoIterator<Item = MenuItem>) {
        self.entries = items
            .into_iter()
            .map(|item| MenuEntry {
                item,
                submenu: None,
            })
            .collect();
        self.reconcile_highlight();
    }

    /// Builds a menu from action descriptors.
    #[must_use]
    pub fn from_actions(actions: impl IntoIterator<Item = ActionDescriptor>) -> Self {
        Self {
            entries: actions
                .into_iter()
                .map(|action| MenuEntry {
                    item: MenuItem::Action(action),
                    submenu: None,
                })
                .collect(),
            highlighted: None,
        }
    }

    /// Returns visible menu items.
    #[must_use]
    pub fn visible_items(&self) -> Vec<&MenuItem> {
        self.visible_items_iter().collect()
    }

    /// Returns visible menu items as a borrowed iterator.
    pub fn visible_items_iter(&self) -> impl Iterator<Item = &MenuItem> + '_ {
        self.visible_entries_iter().map(|entry| &entry.item)
    }

    /// Returns the stable action identity of the highlighted item.
    #[must_use]
    pub fn highlighted_action_id(&self) -> Option<&ActionId> {
        self.highlighted.as_ref()
    }

    /// Returns the visible index of the highlighted item.
    #[must_use]
    pub fn highlighted_visible_index(&self) -> Option<usize> {
        let highlighted = self.highlighted.as_ref()?;
        self.visible_entries_iter()
            .enumerate()
            .find_map(|(index, entry)| {
                entry
                    .navigable_action()
                    .is_some_and(|action| &action.id == highlighted)
                    .then_some(index)
            })
    }

    /// Clears the highlighted item.
    pub fn clear_highlight(&mut self) {
        self.highlighted = None;
    }

    /// Moves the highlight through enabled action and submenu entries with wrapping.
    pub fn move_highlight(&mut self, input: OverlayNavigationInput) -> Option<ActionId> {
        let candidates = self.navigable_visible_indices();
        let index = moved_index(&candidates, self.highlighted_visible_index(), input)?;
        let action_id = self.visible_entry_at(index)?.navigable_action()?.id.clone();
        self.highlighted = Some(action_id.clone());
        Some(action_id)
    }

    /// Highlights a matching enabled item using bounded, timeout-aware typeahead state.
    pub fn typeahead(
        &mut self,
        state: &mut TypeaheadBuffer,
        text: &str,
        now_millis: u64,
    ) -> Option<ActionId> {
        let query = state.update(text, now_millis)?;
        let candidates = self
            .visible_entries_iter()
            .enumerate()
            .filter_map(|(index, entry)| {
                entry
                    .navigable_action()
                    .map(|action| (index, action.label.as_str()))
            })
            .collect::<Vec<_>>();
        let index = typeahead_index(&candidates, self.highlighted_visible_index(), query)?;
        let action_id = self.visible_entry_at(index)?.navigable_action()?.id.clone();
        self.highlighted = Some(action_id.clone());
        Some(action_id)
    }

    /// Returns nested menu data for a visible submenu trigger.
    #[must_use]
    pub fn submenu_for_visible(&self, visible_index: usize) -> Option<&Menu> {
        self.visible_entry_at(visible_index)?.submenu.as_ref()
    }

    /// Returns nested menu data for a stable submenu-trigger action identity.
    #[must_use]
    pub fn submenu_for_action(&self, action_id: &ActionId) -> Option<&Menu> {
        self.entries.iter().find_map(|entry| {
            entry
                .navigable_action()
                .is_some_and(|action| &action.id == action_id)
                .then_some(entry.submenu.as_ref())
                .flatten()
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
        let entry = self.visible_entry_at(visible_index)?;
        if entry.submenu.is_some() {
            return None;
        }
        let action = entry.navigable_action()?;
        Some(ActionInvocation::new(action.id.clone(), source, context))
    }

    fn activation_intent(
        &self,
        parent_overlay: OverlayId,
        source: ActionSource,
        context: ActionContext,
    ) -> Option<MenuNavigationIntent> {
        let visible_index = self.highlighted_visible_index()?;
        let entry = self.visible_entry_at(visible_index)?;
        let action = entry.navigable_action()?;
        if entry.submenu.is_some() {
            return Some(MenuNavigationIntent::OpenSubmenu(MenuSubmenuOpenIntent {
                parent_overlay,
                trigger_action: action.id.clone(),
                visible_index,
                source,
                context,
            }));
        }
        Some(MenuNavigationIntent::Invoke(ActionInvocation::new(
            action.id.clone(),
            source,
            context,
        )))
    }

    fn visible_entries_iter(&self) -> impl Iterator<Item = &MenuEntry> + '_ {
        self.entries.iter().filter(|entry| entry.item_is_visible())
    }

    fn visible_entry_at(&self, visible_index: usize) -> Option<&MenuEntry> {
        self.visible_entries_iter().nth(visible_index)
    }

    fn navigable_visible_indices(&self) -> Vec<usize> {
        self.visible_entries_iter()
            .enumerate()
            .filter_map(|(index, entry)| entry.navigable_action().map(|_| index))
            .collect()
    }

    fn reconcile_highlight(&mut self) {
        let valid = self.highlighted.as_ref().is_some_and(|highlighted| {
            self.entries.iter().any(|entry| {
                entry
                    .navigable_action()
                    .is_some_and(|action| &action.id == highlighted)
            })
        });
        if !valid {
            self.highlighted = None;
        }
    }
}

/// Pure result of menu keyboard navigation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuNavigationIntent {
    /// Highlight changed to an enabled action or submenu trigger.
    Highlighted {
        /// Stable action identity.
        action_id: ActionId,
        /// Index in the visible menu item sequence.
        visible_index: usize,
    },
    /// Application-owned action invocation requested by Enter.
    Invoke(ActionInvocation),
    /// Nested submenu opening requested by Enter.
    OpenSubmenu(MenuSubmenuOpenIntent),
    /// Overlay close requested by Escape.
    Close {
        /// Overlay that should close.
        overlay_id: OverlayId,
    },
}

/// Pure request to open nested menu data for a highlighted submenu trigger.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuSubmenuOpenIntent {
    /// Parent overlay that owns the submenu trigger.
    pub parent_overlay: OverlayId,
    /// Stable trigger action identity used to resolve nested menu data.
    pub trigger_action: ActionId,
    /// Trigger index in the visible menu item sequence.
    pub visible_index: usize,
    /// Source preserved for any nested action invocation.
    pub source: ActionSource,
    /// Context preserved for any nested action invocation.
    pub context: ActionContext,
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

    /// Applies a keyboard input and returns a pure highlight, action, submenu, or close intent.
    pub fn navigate(&mut self, input: OverlayNavigationInput) -> Option<MenuNavigationIntent> {
        match input {
            OverlayNavigationInput::Previous
            | OverlayNavigationInput::Next
            | OverlayNavigationInput::First
            | OverlayNavigationInput::Last => {
                let action_id = self.menu.move_highlight(input)?;
                let visible_index = self.menu.highlighted_visible_index()?;
                Some(MenuNavigationIntent::Highlighted {
                    action_id,
                    visible_index,
                })
            }
            OverlayNavigationInput::Activate => {
                self.menu
                    .activation_intent(self.entry.id, self.source, self.context.clone())
            }
            OverlayNavigationInput::Escape => Some(MenuNavigationIntent::Close {
                overlay_id: self.entry.id,
            }),
        }
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
