//! Data-only editor chrome contracts.

use kinetik_ui_core::{
    ActionContext, ActionDescriptor, ActionIcon, ActionId, ActionInvocation, ActionQueue,
    ActionSource, Rect, Size,
};

use crate::{Menu, MenuOverlay, OverlayDismissal, OverlayId, OverlayKind, PopoverPlacement};

/// Stable identity for a top-level menu-bar heading.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct MenuBarMenuId(u64);

impl MenuBarMenuId {
    /// Creates a menu-bar menu ID from raw bits.
    #[must_use]
    pub const fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    /// Returns the raw ID bits.
    #[must_use]
    pub const fn raw(self) -> u64 {
        self.0
    }
}

/// One top-level menu-bar heading and its action-backed menu model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuBarMenu {
    /// Stable menu identity.
    pub id: MenuBarMenuId,
    /// Menu heading shown in the menu bar.
    pub title: String,
    /// Menu content shown when this heading is active.
    pub menu: Menu,
}

impl MenuBarMenu {
    /// Creates a menu-bar menu from an existing menu model.
    #[must_use]
    pub fn new(id: MenuBarMenuId, title: impl Into<String>, menu: Menu) -> Self {
        Self {
            id,
            title: title.into(),
            menu,
        }
    }

    /// Creates a menu-bar menu from action descriptors.
    #[must_use]
    pub fn from_actions(
        id: MenuBarMenuId,
        title: impl Into<String>,
        actions: impl IntoIterator<Item = ActionDescriptor>,
    ) -> Self {
        Self::new(id, title, Menu::from_actions(actions))
    }

    /// Returns true when this menu has at least one visible item.
    #[must_use]
    pub fn has_visible_items(&self) -> bool {
        !self.menu.visible_items().is_empty()
    }
}

/// Direction for keyboard-style menu-bar heading movement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuBarMove {
    /// Move to the previous visible heading.
    Previous,
    /// Move to the next visible heading.
    Next,
}

/// Overlay conversion request for the active menu-bar menu.
#[derive(Debug, Clone, PartialEq)]
pub struct MenuBarOverlayRequest {
    /// Overlay identity to use for the created menu overlay.
    pub overlay_id: OverlayId,
    /// Overlay kind to preserve for the menu-like surface.
    pub kind: OverlayKind,
    /// Anchor rectangle for placement.
    pub anchor: Rect,
    /// Requested overlay size.
    pub size: Size,
    /// Preferred placement relative to the anchor.
    pub placement: PopoverPlacement,
    /// Placement offset from the anchor.
    pub offset: f32,
    /// Whether placement should fit inside the viewport.
    pub fit_viewport: bool,
    /// Viewport bounds for fitting.
    pub viewport: Rect,
    /// Dismissal policy for the overlay.
    pub dismissal: OverlayDismissal,
    /// Action source to record for invocations emitted by the overlay.
    pub source: ActionSource,
    /// Action context captured for invocations emitted by the overlay.
    pub context: ActionContext,
}

/// Data-only menu-bar model and active-heading state.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MenuBar {
    menus: Vec<MenuBarMenu>,
    active: Option<MenuBarMenuId>,
}

impl MenuBar {
    /// Creates an empty menu bar.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a menu bar from top-level menu definitions.
    #[must_use]
    pub fn from_menus(menus: impl IntoIterator<Item = MenuBarMenu>) -> Self {
        let mut menu_bar = Self {
            menus: menus.into_iter().collect(),
            active: None,
        };
        menu_bar.reconcile_active();
        menu_bar
    }

    /// Returns top-level menus in presentation order.
    #[must_use]
    pub fn menus(&self) -> &[MenuBarMenu] {
        &self.menus
    }

    /// Replaces top-level menus and closes the active menu if its ID is no longer visible.
    pub fn replace_menus(&mut self, menus: impl IntoIterator<Item = MenuBarMenu>) {
        self.menus = menus.into_iter().collect();
        self.reconcile_active();
    }

    /// Returns the active menu ID.
    #[must_use]
    pub const fn active_id(&self) -> Option<MenuBarMenuId> {
        self.active
    }

    /// Returns the active menu definition.
    #[must_use]
    pub fn active_menu(&self) -> Option<&MenuBarMenu> {
        let id = self.active?;
        self.visible_menu_by_id(id).map(|(_, menu)| menu)
    }

    /// Opens a visible menu by ID.
    ///
    /// Returns false for unknown IDs or menus with no visible items.
    pub fn open(&mut self, id: MenuBarMenuId) -> bool {
        if self.visible_menu_by_id(id).is_none() {
            return false;
        }
        self.active = Some(id);
        true
    }

    /// Closes the active menu and returns the previously active ID.
    pub fn close(&mut self) -> Option<MenuBarMenuId> {
        self.active.take()
    }

    /// Toggles a visible menu by ID.
    ///
    /// Returns false for unknown IDs or menus with no visible items.
    pub fn toggle(&mut self, id: MenuBarMenuId) -> bool {
        if self.active == Some(id) {
            self.active = None;
            return true;
        }
        self.open(id)
    }

    /// Moves an already-open menu bar to a visible menu on hover.
    ///
    /// Returns false when no menu is currently open or the target ID is not visible.
    pub fn hover_open(&mut self, id: MenuBarMenuId) -> bool {
        if self.active.is_none() {
            return false;
        }
        self.open(id)
    }

    /// Moves the active menu to the next visible heading and wraps deterministically.
    pub fn move_active(&mut self, movement: MenuBarMove) -> Option<MenuBarMenuId> {
        let index = match movement {
            MenuBarMove::Previous => self.previous_visible_index(),
            MenuBarMove::Next => self.next_visible_index(),
        }?;
        let id = self.menus[index].id;
        self.active = Some(id);
        Some(id)
    }

    /// Moves the active menu to the previous visible heading.
    pub fn move_previous(&mut self) -> Option<MenuBarMenuId> {
        self.move_active(MenuBarMove::Previous)
    }

    /// Moves the active menu to the next visible heading.
    pub fn move_next(&mut self) -> Option<MenuBarMenuId> {
        self.move_active(MenuBarMove::Next)
    }

    /// Converts the active menu to a menu overlay.
    #[must_use]
    pub fn active_overlay(&self, request: MenuBarOverlayRequest) -> Option<MenuOverlay> {
        let menu = self.active_menu()?;
        Some(MenuOverlay::anchored(
            request.overlay_id,
            request.kind,
            menu.menu.clone(),
            request.anchor,
            request.size,
            request.placement,
            request.offset,
            request.fit_viewport,
            request.viewport,
            request.dismissal,
            request.source,
            request.context,
        ))
    }

    fn reconcile_active(&mut self) {
        if self
            .active
            .is_some_and(|id| self.visible_menu_by_id(id).is_none())
        {
            self.active = None;
        }
    }

    fn visible_menu_by_id(&self, id: MenuBarMenuId) -> Option<(usize, &MenuBarMenu)> {
        self.menus
            .iter()
            .enumerate()
            .find(|(_, menu)| menu.id == id && menu.has_visible_items())
    }

    fn active_index(&self) -> Option<usize> {
        let active = self.active?;
        self.visible_menu_by_id(active).map(|(index, _)| index)
    }

    fn visible_indices(&self) -> impl Iterator<Item = usize> + '_ {
        self.menus
            .iter()
            .enumerate()
            .filter_map(|(index, menu)| menu.has_visible_items().then_some(index))
    }

    fn previous_visible_index(&self) -> Option<usize> {
        let visible = self.visible_indices().collect::<Vec<_>>();
        if visible.is_empty() {
            return None;
        }
        let Some(active) = self.active_index() else {
            return visible.last().copied();
        };
        visible
            .iter()
            .rfind(|index| **index < active)
            .copied()
            .or_else(|| visible.last().copied())
    }

    fn next_visible_index(&self) -> Option<usize> {
        let visible = self.visible_indices().collect::<Vec<_>>();
        if visible.is_empty() {
            return None;
        }
        let Some(active) = self.active_index() else {
            return visible.first().copied();
        };
        visible
            .iter()
            .find(|index| **index > active)
            .copied()
            .or_else(|| visible.first().copied())
    }
}

/// Stable identity for a toolbar group.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ToolbarGroupId(u64);

impl ToolbarGroupId {
    /// Creates a toolbar group ID from raw bits.
    #[must_use]
    pub const fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    /// Returns the raw ID bits.
    #[must_use]
    pub const fn raw(self) -> u64 {
        self.0
    }
}

/// Preferred presentation style for an action-backed toolbar item.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum ToolbarItemPresentation {
    /// Show only the symbolic icon when available.
    #[default]
    IconOnly,
    /// Show only the action label.
    TextOnly,
    /// Show both the symbolic icon and the action label.
    IconAndText,
}

/// Data-only toolbar item backed by an application-owned action descriptor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolbarItem {
    /// Action metadata shared with menus, command palettes, shortcuts, and other surfaces.
    pub action: ActionDescriptor,
    /// Preferred item presentation.
    pub presentation: ToolbarItemPresentation,
}

impl ToolbarItem {
    /// Creates a toolbar item with default icon-only presentation metadata.
    #[must_use]
    pub fn new(action: ActionDescriptor) -> Self {
        Self {
            action,
            presentation: ToolbarItemPresentation::IconOnly,
        }
    }

    /// Sets presentation metadata for this toolbar item.
    #[must_use]
    pub fn with_presentation(mut self, presentation: ToolbarItemPresentation) -> Self {
        self.presentation = presentation;
        self
    }

    /// Returns the backing action ID.
    #[must_use]
    pub const fn action_id(&self) -> &ActionId {
        &self.action.id
    }

    /// Returns the toolbar display label from the backing action.
    #[must_use]
    pub fn label(&self) -> &str {
        &self.action.label
    }

    /// Returns the optional symbolic icon from the backing action.
    #[must_use]
    pub const fn icon(&self) -> Option<&ActionIcon> {
        self.action.icon.as_ref()
    }

    /// Returns true when the item should be presented on visible toolbar surfaces.
    #[must_use]
    pub const fn visible(&self) -> bool {
        self.action.state.visible
    }

    /// Returns true when the item can currently be invoked.
    #[must_use]
    pub const fn enabled(&self) -> bool {
        self.action.state.enabled
    }

    /// Returns the checked/toggled action state when the item is checkable.
    #[must_use]
    pub const fn checked(&self) -> Option<bool> {
        self.action.state.checked
    }

    /// Returns true when this item represents a selected/toggled-on tool.
    #[must_use]
    pub const fn selected(&self) -> bool {
        self.action.state.is_checked()
    }

    /// Returns true when this item is both visible and enabled.
    #[must_use]
    pub const fn can_invoke(&self) -> bool {
        self.action.can_invoke()
    }

    /// Creates an invocation for this toolbar item when it is visible and enabled.
    #[must_use]
    pub fn invocation(&self, context: ActionContext) -> Option<ActionInvocation> {
        self.can_invoke()
            .then(|| ActionInvocation::new(self.action.id.clone(), ActionSource::Button, context))
    }
}

impl From<ActionDescriptor> for ToolbarItem {
    fn from(action: ActionDescriptor) -> Self {
        Self::new(action)
    }
}

/// Ordered group of action-backed toolbar items.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolbarGroup {
    /// Stable group identity.
    pub id: ToolbarGroupId,
    /// Group title or accessibility label.
    pub title: String,
    items: Vec<ToolbarItem>,
}

impl ToolbarGroup {
    /// Creates a toolbar group from item models.
    #[must_use]
    pub fn new(
        id: ToolbarGroupId,
        title: impl Into<String>,
        items: impl IntoIterator<Item = ToolbarItem>,
    ) -> Self {
        Self {
            id,
            title: title.into(),
            items: items.into_iter().collect(),
        }
    }

    /// Creates a toolbar group from action descriptors.
    #[must_use]
    pub fn from_actions(
        id: ToolbarGroupId,
        title: impl Into<String>,
        actions: impl IntoIterator<Item = ActionDescriptor>,
    ) -> Self {
        Self::new(id, title, actions.into_iter().map(ToolbarItem::from))
    }

    /// Returns toolbar items in presentation order.
    #[must_use]
    pub fn items(&self) -> &[ToolbarItem] {
        &self.items
    }

    /// Returns visible toolbar items in presentation order.
    #[must_use]
    pub fn visible_items(&self) -> Vec<&ToolbarItem> {
        self.items.iter().filter(|item| item.visible()).collect()
    }

    /// Returns true when this group has at least one visible item.
    #[must_use]
    pub fn has_visible_items(&self) -> bool {
        !self.visible_items().is_empty()
    }

    /// Creates an invocation for an enabled visible toolbar item by visible index.
    #[must_use]
    pub fn invocation_for_visible(
        &self,
        visible_index: usize,
        context: ActionContext,
    ) -> Option<ActionInvocation> {
        self.visible_items()
            .get(visible_index)
            .and_then(|item| item.invocation(context))
    }

    /// Invokes an enabled visible toolbar item by visible index.
    pub fn invoke_visible(
        &self,
        visible_index: usize,
        queue: &mut ActionQueue,
        context: ActionContext,
    ) -> bool {
        let Some(invocation) = self.invocation_for_visible(visible_index, context) else {
            return false;
        };
        queue.push(invocation);
        true
    }
}

/// Data-only toolbar model made of ordered tool groups.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Toolbar {
    groups: Vec<ToolbarGroup>,
}

impl Toolbar {
    /// Creates an empty toolbar.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a toolbar from ordered group definitions.
    #[must_use]
    pub fn from_groups(groups: impl IntoIterator<Item = ToolbarGroup>) -> Self {
        Self {
            groups: groups.into_iter().collect(),
        }
    }

    /// Returns toolbar groups in presentation order.
    #[must_use]
    pub fn groups(&self) -> &[ToolbarGroup] {
        &self.groups
    }

    /// Replaces toolbar groups.
    pub fn replace_groups(&mut self, groups: impl IntoIterator<Item = ToolbarGroup>) {
        self.groups = groups.into_iter().collect();
    }

    /// Returns a group by stable identity.
    #[must_use]
    pub fn group(&self, id: ToolbarGroupId) -> Option<&ToolbarGroup> {
        self.groups.iter().find(|group| group.id == id)
    }

    /// Returns visible groups, skipping groups with no visible items.
    #[must_use]
    pub fn visible_groups(&self) -> Vec<&ToolbarGroup> {
        self.groups
            .iter()
            .filter(|group| group.has_visible_items())
            .collect()
    }

    /// Creates an invocation by visible group index and visible item index.
    #[must_use]
    pub fn invocation_for_visible(
        &self,
        visible_group_index: usize,
        visible_item_index: usize,
        context: ActionContext,
    ) -> Option<ActionInvocation> {
        self.visible_groups()
            .get(visible_group_index)
            .and_then(|group| group.invocation_for_visible(visible_item_index, context))
    }

    /// Invokes an enabled visible item by visible group index and visible item index.
    pub fn invoke_visible(
        &self,
        visible_group_index: usize,
        visible_item_index: usize,
        queue: &mut ActionQueue,
        context: ActionContext,
    ) -> bool {
        let Some(invocation) =
            self.invocation_for_visible(visible_group_index, visible_item_index, context)
        else {
            return false;
        };
        queue.push(invocation);
        true
    }

    /// Creates an invocation by stable group identity and visible item index.
    #[must_use]
    pub fn invocation_for_group_visible(
        &self,
        group_id: ToolbarGroupId,
        visible_item_index: usize,
        context: ActionContext,
    ) -> Option<ActionInvocation> {
        self.group(group_id)
            .and_then(|group| group.invocation_for_visible(visible_item_index, context))
    }

    /// Invokes an enabled visible item by stable group identity and visible item index.
    pub fn invoke_group_visible(
        &self,
        group_id: ToolbarGroupId,
        visible_item_index: usize,
        queue: &mut ActionQueue,
        context: ActionContext,
    ) -> bool {
        let Some(invocation) =
            self.invocation_for_group_visible(group_id, visible_item_index, context)
        else {
            return false;
        };
        queue.push(invocation);
        true
    }
}
