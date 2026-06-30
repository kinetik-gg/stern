//! Data-only editor chrome contracts.

use std::hash::{Hash, Hasher};

use kinetik_ui_core::{
    ActionContext, ActionDescriptor, ActionIcon, ActionId, ActionInvocation, ActionQueue,
    ActionSource, DiagnosticCategory, DiagnosticLocation,
    DiagnosticSeverity as CoreDiagnosticSeverity, FrameDiagnostic, Rect, Size,
};

use crate::{
    DockPathElement, DockSnapshotDiagnostic, DockSnapshotDiagnostics, DockSnapshotSplitValue,
    FrameId, FrameTab, Menu, MenuOverlay, OverlayDismissal, OverlayId, OverlayKind, PanelId,
    PanelInstanceId, PanelTypeId, PopoverPlacement, SnapshotDiagnosticSeverity,
    WorkspaceSnapshotDiagnostic, WorkspaceSnapshotDiagnostics,
};

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

/// Stable identity for a status bar item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct StatusItemId(u64);

impl StatusItemId {
    /// Creates a status item ID from raw bits.
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

/// Data category for a status bar item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StatusItemKind {
    /// General non-blocking status text.
    Message,
    /// Count of available or unavailable actions.
    ActionCount,
    /// Count of queued, active, or completed jobs.
    JobCount,
    /// Normalized progress metadata.
    Progress,
    /// Ready state.
    Ready,
    /// Pending or queued state.
    Pending,
    /// Stale or out-of-date state.
    Stale,
    /// Error state.
    Error,
}

/// Normalized progress metadata for status bar presentation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StatusProgress {
    /// Clamped progress value in the inclusive `0.0..=1.0` range.
    pub value: f32,
}

impl StatusProgress {
    /// Creates progress metadata, replacing non-finite values with `0.0` and clamping to range.
    #[must_use]
    pub fn new(value: f32) -> Self {
        let value = if value.is_finite() { value } else { 0.0 };
        Self {
            value: value.clamp(0.0, 1.0),
        }
    }

    /// Creates progress metadata from a completed/total pair.
    #[must_use]
    pub fn from_fraction(completed: f32, total: f32) -> Self {
        if !completed.is_finite() || !total.is_finite() || total <= 0.0 {
            return Self::new(0.0);
        }
        Self::new(completed / total)
    }
}

/// Data-only status bar item.
#[derive(Debug, Clone, PartialEq)]
pub struct StatusItem {
    /// Stable status item identity.
    pub id: StatusItemId,
    /// Short label for compact presentation or accessibility.
    pub label: String,
    /// Status text shown by the application.
    pub text: String,
    /// Typed status category.
    pub kind: StatusItemKind,
    /// Optional sanitized count metadata.
    pub count: Option<u32>,
    /// Optional normalized progress metadata.
    pub progress: Option<StatusProgress>,
    /// Whether this item should be presented on visible status bar surfaces.
    pub visible: bool,
}

impl StatusItem {
    /// Creates a visible status bar item.
    #[must_use]
    pub fn new(
        id: StatusItemId,
        label: impl Into<String>,
        text: impl Into<String>,
        kind: StatusItemKind,
    ) -> Self {
        Self {
            id,
            label: label.into(),
            text: text.into(),
            kind,
            count: None,
            progress: None,
            visible: true,
        }
    }

    /// Sets count metadata for action, job, or diagnostic count presentation.
    #[must_use]
    pub const fn with_count(mut self, count: u32) -> Self {
        self.count = Some(count);
        self
    }

    /// Sets normalized progress metadata.
    #[must_use]
    pub const fn with_progress(mut self, progress: StatusProgress) -> Self {
        self.progress = Some(progress);
        self
    }

    /// Sets progress metadata from a raw value.
    #[must_use]
    pub fn with_progress_value(self, value: f32) -> Self {
        self.with_progress(StatusProgress::new(value))
    }

    /// Sets progress metadata from completed/total values.
    #[must_use]
    pub fn with_progress_fraction(self, completed: f32, total: f32) -> Self {
        self.with_progress(StatusProgress::from_fraction(completed, total))
    }

    /// Sets visibility metadata.
    #[must_use]
    pub const fn with_visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }
}

/// Data-only status bar model made of ordered items.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct StatusBar {
    items: Vec<StatusItem>,
}

impl StatusBar {
    /// Creates an empty status bar.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a status bar from ordered item definitions.
    #[must_use]
    pub fn from_items(items: impl IntoIterator<Item = StatusItem>) -> Self {
        Self {
            items: items.into_iter().collect(),
        }
    }

    /// Returns status items in presentation order.
    #[must_use]
    pub fn items(&self) -> &[StatusItem] {
        &self.items
    }

    /// Replaces status items.
    pub fn replace_items(&mut self, items: impl IntoIterator<Item = StatusItem>) {
        self.items = items.into_iter().collect();
    }

    /// Returns a status item by stable identity.
    #[must_use]
    pub fn item(&self, id: StatusItemId) -> Option<&StatusItem> {
        self.items.iter().find(|item| item.id == id)
    }

    /// Returns visible status items in presentation order.
    #[must_use]
    pub fn visible_items(&self) -> Vec<&StatusItem> {
        self.items.iter().filter(|item| item.visible).collect()
    }
}

/// Stable application-owned identity for a job row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct JobRowId(u64);

impl JobRowId {
    /// Creates a job row ID from raw bits.
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

/// Application-supplied presentation phase for a job row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum JobPhase {
    /// Work is waiting to start.
    Queued,
    /// Work is currently running.
    Running,
    /// Cancellation has been requested and is being acknowledged by the application.
    Cancelling,
    /// Work finished successfully.
    Succeeded,
    /// Work finished with an application-owned failure state.
    Failed,
}

impl JobPhase {
    /// Returns true when the phase still represents active or pending work.
    #[must_use]
    pub const fn is_active(self) -> bool {
        matches!(self, Self::Queued | Self::Running | Self::Cancelling)
    }
}

/// Application-supplied progress metadata for a job row.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum JobProgress {
    /// Work has no deterministic fraction yet.
    Indeterminate,
    /// Work has a sanitized deterministic fraction.
    Determinate(StatusProgress),
}

impl JobProgress {
    /// Creates determinate progress metadata, replacing non-finite values with `0.0` and clamping.
    #[must_use]
    pub fn determinate(value: f32) -> Self {
        Self::Determinate(StatusProgress::new(value))
    }

    /// Creates determinate progress metadata from a completed/total pair.
    #[must_use]
    pub fn from_fraction(completed: f32, total: f32) -> Self {
        Self::Determinate(StatusProgress::from_fraction(completed, total))
    }

    /// Returns the determinate status progress value, when available.
    #[must_use]
    pub const fn status_progress(self) -> Option<StatusProgress> {
        match self {
            Self::Indeterminate => None,
            Self::Determinate(progress) => Some(progress),
        }
    }
}

/// Application-owned cancel action metadata for a job row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobCancel {
    /// Action metadata shared with menus, command palettes, shortcuts, and buttons.
    pub action: ActionDescriptor,
    /// Context emitted with the cancel action invocation.
    pub context: ActionContext,
}

impl JobCancel {
    /// Creates cancel metadata from an application-owned action descriptor.
    #[must_use]
    pub const fn new(action: ActionDescriptor, context: ActionContext) -> Self {
        Self { action, context }
    }

    /// Returns true when the cancel affordance should be presented.
    #[must_use]
    pub const fn visible(&self) -> bool {
        self.action.state.visible
    }

    /// Returns true when the cancel affordance can currently be invoked.
    #[must_use]
    pub const fn enabled(&self) -> bool {
        self.action.state.enabled
    }

    /// Returns true when this cancel action is both visible and enabled.
    #[must_use]
    pub const fn can_request(&self) -> bool {
        self.action.can_invoke()
    }

    /// Creates a cancel request for an enabled visible cancel action.
    #[must_use]
    pub fn request(&self, job_id: JobRowId) -> Option<JobCancelRequest> {
        self.can_request().then(|| {
            JobCancelRequest::new(
                job_id,
                ActionInvocation::new(
                    self.action.id.clone(),
                    ActionSource::Button,
                    self.context.clone(),
                ),
            )
        })
    }
}

/// Cancel request metadata emitted by a job presentation affordance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobCancelRequest {
    /// Stable job row identity supplied by the application.
    pub job_id: JobRowId,
    /// Action invocation for the application to execute.
    pub invocation: ActionInvocation,
}

impl JobCancelRequest {
    /// Creates cancel request metadata.
    #[must_use]
    pub const fn new(job_id: JobRowId, invocation: ActionInvocation) -> Self {
        Self { job_id, invocation }
    }
}

/// Data-only application-supplied job row snapshot.
#[derive(Debug, Clone, PartialEq)]
pub struct JobRow {
    /// Stable job row identity.
    pub id: JobRowId,
    /// Short label for compact presentation or accessibility.
    pub label: String,
    /// Current phase supplied by the application.
    pub phase: JobPhase,
    /// Progress metadata supplied by the application.
    pub progress: JobProgress,
    /// Optional secondary detail text supplied by the application.
    pub detail: Option<String>,
    /// Optional cancel action metadata supplied by the application.
    pub cancel: Option<JobCancel>,
}

impl JobRow {
    /// Creates a job row with indeterminate progress and no cancel metadata.
    #[must_use]
    pub fn new(id: JobRowId, label: impl Into<String>, phase: JobPhase) -> Self {
        Self {
            id,
            label: label.into(),
            phase,
            progress: JobProgress::Indeterminate,
            detail: None,
            cancel: None,
        }
    }

    /// Sets progress metadata.
    #[must_use]
    pub const fn with_progress(mut self, progress: JobProgress) -> Self {
        self.progress = progress;
        self
    }

    /// Sets secondary detail text.
    #[must_use]
    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    /// Sets cancel metadata.
    #[must_use]
    pub fn with_cancel(mut self, cancel: JobCancel) -> Self {
        self.cancel = Some(cancel);
        self
    }

    /// Returns true when this row can emit a cancel request.
    #[must_use]
    pub fn can_cancel(&self) -> bool {
        self.cancel.as_ref().is_some_and(JobCancel::can_request)
    }

    /// Creates cancel request metadata for this row when cancellation is available and enabled.
    #[must_use]
    pub fn cancel_request(&self) -> Option<JobCancelRequest> {
        self.cancel.as_ref()?.request(self.id)
    }
}

/// Deterministic summary counts for a job list snapshot.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct JobSummaryCounts {
    /// Number of queued jobs.
    pub queued: u32,
    /// Number of running jobs.
    pub running: u32,
    /// Number of cancelling jobs.
    pub cancelling: u32,
    /// Number of succeeded jobs.
    pub succeeded: u32,
    /// Number of failed jobs.
    pub failed: u32,
}

impl JobSummaryCounts {
    /// Returns active or pending jobs.
    #[must_use]
    pub const fn active(self) -> u32 {
        self.queued + self.running + self.cancelling
    }

    /// Returns the total number of jobs in the summary.
    #[must_use]
    pub const fn total(self) -> u32 {
        self.active() + self.succeeded + self.failed
    }
}

/// Active job progress metadata suitable for status bar presentation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ActiveJobProgress {
    /// Number of active or pending jobs.
    pub active: u32,
    /// Number of active or pending jobs with determinate progress.
    pub determinate: u32,
    /// Number of active or pending jobs with indeterminate progress.
    pub indeterminate: u32,
    /// Aggregate active progress without inventing percentages for indeterminate work.
    pub progress: JobProgress,
}

impl ActiveJobProgress {
    /// Returns determinate status progress only when all active work is determinate.
    #[must_use]
    pub const fn status_progress(self) -> Option<StatusProgress> {
        match self.progress {
            JobProgress::Determinate(progress) if self.indeterminate == 0 => Some(progress),
            JobProgress::Determinate(_) | JobProgress::Indeterminate => None,
        }
    }
}

/// Data-only job list model made of ordered application-owned rows.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct JobList {
    rows: Vec<JobRow>,
}

impl JobList {
    /// Creates an empty job list.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a job list from ordered row snapshots.
    #[must_use]
    pub fn from_rows(rows: impl IntoIterator<Item = JobRow>) -> Self {
        Self {
            rows: rows.into_iter().collect(),
        }
    }

    /// Returns job rows in application-supplied presentation order.
    #[must_use]
    pub fn rows(&self) -> &[JobRow] {
        &self.rows
    }

    /// Replaces job rows while preserving application-supplied order.
    pub fn replace_rows(&mut self, rows: impl IntoIterator<Item = JobRow>) {
        self.rows = rows.into_iter().collect();
    }

    /// Returns a job row by stable identity.
    #[must_use]
    pub fn row(&self, id: JobRowId) -> Option<&JobRow> {
        self.rows.iter().find(|row| row.id == id)
    }

    /// Returns summary counts by phase.
    #[must_use]
    pub fn summary(&self) -> JobSummaryCounts {
        let mut summary = JobSummaryCounts::default();
        for row in &self.rows {
            match row.phase {
                JobPhase::Queued => summary.queued += 1,
                JobPhase::Running => summary.running += 1,
                JobPhase::Cancelling => summary.cancelling += 1,
                JobPhase::Succeeded => summary.succeeded += 1,
                JobPhase::Failed => summary.failed += 1,
            }
        }
        summary
    }

    /// Returns active or pending job count suitable for `StatusItemKind::JobCount`.
    #[must_use]
    pub fn active_count(&self) -> u32 {
        self.summary().active()
    }

    /// Returns aggregate progress metadata for active or pending jobs.
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn active_progress(&self) -> Option<ActiveJobProgress> {
        let mut active = 0_u32;
        let mut determinate = 0_u32;
        let mut indeterminate = 0_u32;
        let mut progress_sum = 0.0_f32;

        for row in &self.rows {
            if !row.phase.is_active() {
                continue;
            }
            active += 1;
            match row.progress {
                JobProgress::Indeterminate => indeterminate += 1,
                JobProgress::Determinate(progress) => {
                    determinate += 1;
                    progress_sum += progress.value;
                }
            }
        }

        if active == 0 {
            return None;
        }

        let progress = if indeterminate == 0 && determinate > 0 {
            JobProgress::determinate(progress_sum / determinate as f32)
        } else {
            JobProgress::Indeterminate
        };

        Some(ActiveJobProgress {
            active,
            determinate,
            indeterminate,
            progress,
        })
    }

    /// Returns determinate status progress only when all active work has determinate progress.
    #[must_use]
    pub fn active_status_progress(&self) -> Option<StatusProgress> {
        self.active_progress()?.status_progress()
    }

    /// Creates cancel request metadata for a row by stable job identity.
    #[must_use]
    pub fn cancel_request(&self, id: JobRowId) -> Option<JobCancelRequest> {
        self.row(id)?.cancel_request()
    }
}

/// Stable identity for a diagnostics strip item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DiagnosticStripItemId(u64);

impl DiagnosticStripItemId {
    /// Creates a diagnostics strip item ID from raw bits.
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

/// Severity ordering for diagnostics strip presentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DiagnosticStripSeverity {
    /// Error diagnostics should be presented first.
    Error,
    /// Warning diagnostics follow errors.
    Warning,
    /// Informational diagnostics follow warnings.
    Info,
}

impl DiagnosticStripSeverity {
    const fn sort_rank(self) -> u8 {
        match self {
            Self::Error => 0,
            Self::Warning => 1,
            Self::Info => 2,
        }
    }
}

impl From<CoreDiagnosticSeverity> for DiagnosticStripSeverity {
    fn from(severity: CoreDiagnosticSeverity) -> Self {
        match severity {
            CoreDiagnosticSeverity::Warning => Self::Warning,
        }
    }
}

impl From<SnapshotDiagnosticSeverity> for DiagnosticStripSeverity {
    fn from(severity: SnapshotDiagnosticSeverity) -> Self {
        match severity {
            SnapshotDiagnosticSeverity::Error => Self::Error,
            SnapshotDiagnosticSeverity::Warning => Self::Warning,
        }
    }
}

/// Structured diagnostic source suitable for later debug presentation.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DiagnosticSource {
    /// Core runtime diagnostics.
    Core,
    /// Dock snapshot or dock workspace diagnostics.
    Dock,
    /// Workspace snapshot shell diagnostics.
    Workspace,
    /// Renderer diagnostics.
    Renderer,
    /// Platform adapter diagnostics.
    Platform,
    /// Application-owned diagnostics.
    Application,
    /// Named external or future diagnostic source.
    Other(String),
}

/// Typed diagnostic context value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiagnosticFieldValue {
    /// Application, renderer, platform, or other free-form text.
    Text(String),
    /// Stable unsigned index or count.
    Usize(usize),
    /// Core runtime diagnostic category.
    CoreDiagnosticCategory(DiagnosticCategory),
    /// Core runtime diagnostic location.
    CoreDiagnosticLocation(DiagnosticLocation),
    /// Dock tree path elements.
    DockPath(Vec<DockPathElement>),
    /// Split value identified by dock snapshot validation.
    DockSplitValue(DockSnapshotSplitValue),
    /// Stable frame identity.
    FrameId(FrameId),
    /// Stable panel identity.
    PanelId(PanelId),
    /// Stable panel instance identity.
    PanelInstanceId(PanelInstanceId),
    /// Stable panel type identity.
    PanelTypeId(PanelTypeId),
}

impl Hash for DiagnosticFieldValue {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Self::Text(value) => {
                0_u8.hash(state);
                value.hash(state);
            }
            Self::Usize(value) => {
                1_u8.hash(state);
                value.hash(state);
            }
            Self::CoreDiagnosticCategory(category) => {
                2_u8.hash(state);
                category.hash(state);
            }
            Self::CoreDiagnosticLocation(location) => {
                3_u8.hash(state);
                location.hash(state);
            }
            Self::DockPath(path) => {
                4_u8.hash(state);
                path.hash(state);
            }
            Self::DockSplitValue(value) => {
                5_u8.hash(state);
                match value {
                    DockSnapshotSplitValue::Ratio => 0_u8.hash(state),
                    DockSnapshotSplitValue::MinFirst => 1_u8.hash(state),
                    DockSnapshotSplitValue::MinSecond => 2_u8.hash(state),
                }
            }
            Self::FrameId(frame) => {
                6_u8.hash(state);
                frame.hash(state);
            }
            Self::PanelId(panel) => {
                7_u8.hash(state);
                panel.hash(state);
            }
            Self::PanelInstanceId(panel_instance) => {
                8_u8.hash(state);
                panel_instance.hash(state);
            }
            Self::PanelTypeId(panel_type) => {
                9_u8.hash(state);
                panel_type.hash(state);
            }
        }
    }
}

impl DiagnosticFieldValue {
    /// Returns a presentation string without requiring downstream tools to parse it.
    #[must_use]
    pub fn display_value(&self) -> String {
        match self {
            Self::Text(value) => value.clone(),
            Self::Usize(value) => value.to_string(),
            Self::CoreDiagnosticCategory(category) => format!("{category:?}"),
            Self::CoreDiagnosticLocation(location) => format!("{location:?}"),
            Self::DockPath(path) => format!("{path:?}"),
            Self::DockSplitValue(value) => format!("{value:?}"),
            Self::FrameId(frame) => frame.raw().to_string(),
            Self::PanelId(panel) => panel.raw().to_string(),
            Self::PanelInstanceId(panel_instance) => panel_instance.raw().to_string(),
            Self::PanelTypeId(panel_type) => panel_type.raw().to_string(),
        }
    }
}

impl From<&str> for DiagnosticFieldValue {
    fn from(value: &str) -> Self {
        Self::Text(value.to_owned())
    }
}

impl From<String> for DiagnosticFieldValue {
    fn from(value: String) -> Self {
        Self::Text(value)
    }
}

impl From<usize> for DiagnosticFieldValue {
    fn from(value: usize) -> Self {
        Self::Usize(value)
    }
}

/// Typed diagnostic context field.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DiagnosticField {
    /// Stable field name.
    pub name: String,
    /// Typed field value for downstream tools and presentation.
    pub value: DiagnosticFieldValue,
}

impl DiagnosticField {
    /// Creates a diagnostic context field.
    #[must_use]
    pub fn new(name: impl Into<String>, value: impl Into<DiagnosticFieldValue>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
        }
    }

    /// Returns a presentation string for this field value.
    #[must_use]
    pub fn display_value(&self) -> String {
        self.value.display_value()
    }
}

/// Data-only diagnostics strip item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticStripItem {
    /// Stable diagnostic identity.
    pub id: DiagnosticStripItemId,
    /// Diagnostic severity.
    pub severity: DiagnosticStripSeverity,
    /// Stable diagnostic code.
    pub code: String,
    /// Short diagnostic message or label.
    pub message: String,
    /// Optional typed source metadata.
    pub source: Option<DiagnosticSource>,
    /// Optional typed context fields.
    pub fields: Vec<DiagnosticField>,
}

impl DiagnosticStripItem {
    /// Creates a diagnostics strip item.
    #[must_use]
    pub fn new(
        id: DiagnosticStripItemId,
        severity: DiagnosticStripSeverity,
        code: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            id,
            severity,
            code: code.into(),
            message: message.into(),
            source: None,
            fields: Vec::new(),
        }
    }

    /// Creates a diagnostics strip item from a core frame diagnostic.
    #[must_use]
    pub fn from_frame_diagnostic(id: DiagnosticStripItemId, diagnostic: FrameDiagnostic) -> Self {
        Self::from_frame_diagnostic_ref(id, &diagnostic)
    }

    /// Creates a diagnostics strip item from a borrowed core frame diagnostic.
    #[must_use]
    pub fn from_frame_diagnostic_ref(
        id: DiagnosticStripItemId,
        diagnostic: &FrameDiagnostic,
    ) -> Self {
        Self::new(
            id,
            diagnostic.severity.into(),
            diagnostic.code,
            diagnostic.code,
        )
        .with_source(DiagnosticSource::Core)
        .with_field(
            "category",
            DiagnosticFieldValue::CoreDiagnosticCategory(diagnostic.category),
        )
        .with_field(
            "location",
            DiagnosticFieldValue::CoreDiagnosticLocation(diagnostic.location),
        )
    }

    /// Creates a diagnostics strip item from a dock snapshot diagnostic.
    #[must_use]
    pub fn from_dock_snapshot_diagnostic(
        id: DiagnosticStripItemId,
        diagnostic: &DockSnapshotDiagnostic,
    ) -> Self {
        let mut item = Self::new(
            id,
            diagnostic.severity.into(),
            diagnostic.stable_code(),
            diagnostic.stable_code(),
        )
        .with_source(DiagnosticSource::Dock)
        .with_field(
            "path",
            DiagnosticFieldValue::DockPath(diagnostic.path.elements().to_vec()),
        );

        if let Some(frame) = diagnostic.frame {
            item = item.with_field("frame", DiagnosticFieldValue::FrameId(frame));
        }
        if let Some(panel) = diagnostic.panel {
            item = item.with_field("panel", DiagnosticFieldValue::PanelId(panel));
        }
        if let Some(active_index) = diagnostic.active_index {
            item = item.with_field("active_index", active_index);
        }
        if let Some(panel_count) = diagnostic.panel_count {
            item = item.with_field("panel_count", panel_count);
        }
        if let Some(split_value) = diagnostic.split_value {
            item = item.with_field(
                "split_value",
                DiagnosticFieldValue::DockSplitValue(split_value),
            );
        }

        item
    }

    /// Creates a diagnostics strip item from a workspace snapshot diagnostic.
    #[must_use]
    pub fn from_workspace_snapshot_diagnostic(
        id: DiagnosticStripItemId,
        diagnostic: &WorkspaceSnapshotDiagnostic,
    ) -> Self {
        let mut item = Self::new(
            id,
            diagnostic.severity.into(),
            diagnostic.stable_code(),
            diagnostic.stable_code(),
        )
        .with_source(DiagnosticSource::Workspace);

        if let Some(panel_instance) = diagnostic.panel_instance {
            item = item.with_field(
                "panel_instance",
                DiagnosticFieldValue::PanelInstanceId(panel_instance),
            );
        }
        if let Some(panel_type) = diagnostic.panel_type {
            item = item.with_field("panel_type", DiagnosticFieldValue::PanelTypeId(panel_type));
        }
        if let Some(frame) = diagnostic.frame {
            item = item.with_field("frame", DiagnosticFieldValue::FrameId(frame));
        }
        if let Some(panel) = diagnostic.panel {
            item = item.with_field("panel", DiagnosticFieldValue::PanelId(panel));
        }
        if let Some(dock_title) = &diagnostic.dock_title {
            item = item.with_field("dock_title", dock_title.as_str());
        }
        if let Some(instance_title) = &diagnostic.instance_title {
            item = item.with_field("instance_title", instance_title.as_str());
        }

        item
    }

    /// Sets source metadata.
    #[must_use]
    pub fn with_source(mut self, source: DiagnosticSource) -> Self {
        self.source = Some(source);
        self
    }

    /// Appends a typed context field.
    #[must_use]
    pub fn with_field(
        mut self,
        name: impl Into<String>,
        value: impl Into<DiagnosticFieldValue>,
    ) -> Self {
        self.fields.push(DiagnosticField::new(name, value));
        self
    }

    /// Appends typed context fields.
    #[must_use]
    pub fn with_fields(mut self, fields: impl IntoIterator<Item = DiagnosticField>) -> Self {
        self.fields.extend(fields);
        self
    }
}

/// Summary counts for diagnostics strip presentation.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct DiagnosticStripSummary {
    /// Number of error diagnostics.
    pub errors: u32,
    /// Number of warning diagnostics.
    pub warnings: u32,
    /// Number of informational diagnostics.
    pub info: u32,
}

impl DiagnosticStripSummary {
    /// Returns the total number of diagnostics.
    #[must_use]
    pub const fn total(self) -> u32 {
        self.errors + self.warnings + self.info
    }
}

/// Data-only diagnostics strip model.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DiagnosticStrip {
    items: Vec<DiagnosticStripItem>,
}

impl DiagnosticStrip {
    /// Creates an empty diagnostics strip.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a diagnostics strip from item definitions.
    #[must_use]
    pub fn from_items(items: impl IntoIterator<Item = DiagnosticStripItem>) -> Self {
        Self {
            items: items.into_iter().collect(),
        }
    }

    /// Returns diagnostics in insertion order.
    #[must_use]
    pub fn items(&self) -> &[DiagnosticStripItem] {
        &self.items
    }

    /// Replaces diagnostics.
    pub fn replace_items(&mut self, items: impl IntoIterator<Item = DiagnosticStripItem>) {
        self.items = items.into_iter().collect();
    }

    /// Appends one diagnostic in aggregation order.
    pub fn push_item(&mut self, item: DiagnosticStripItem) {
        self.items.push(item);
    }

    /// Appends diagnostics in aggregation order.
    pub fn extend_items(&mut self, items: impl IntoIterator<Item = DiagnosticStripItem>) {
        self.items.extend(items);
    }

    /// Appends core frame diagnostics using deterministic IDs from `first_id`.
    pub fn extend_frame_diagnostics(
        &mut self,
        first_id: DiagnosticStripItemId,
        diagnostics: impl IntoIterator<Item = FrameDiagnostic>,
    ) {
        self.items.extend(
            diagnostics
                .into_iter()
                .enumerate()
                .map(|(index, diagnostic)| {
                    DiagnosticStripItem::from_frame_diagnostic(
                        offset_diagnostic_id(first_id, index),
                        diagnostic,
                    )
                }),
        );
    }

    /// Appends borrowed core frame diagnostics using deterministic IDs from `first_id`.
    pub fn extend_frame_diagnostics_ref<'a>(
        &mut self,
        first_id: DiagnosticStripItemId,
        diagnostics: impl IntoIterator<Item = &'a FrameDiagnostic>,
    ) {
        self.items.extend(
            diagnostics
                .into_iter()
                .enumerate()
                .map(|(index, diagnostic)| {
                    DiagnosticStripItem::from_frame_diagnostic_ref(
                        offset_diagnostic_id(first_id, index),
                        diagnostic,
                    )
                }),
        );
    }

    /// Appends dock snapshot diagnostics in their deterministic validation order.
    pub fn extend_dock_snapshot_diagnostics(
        &mut self,
        first_id: DiagnosticStripItemId,
        diagnostics: &DockSnapshotDiagnostics,
    ) {
        self.items.extend(
            diagnostics
                .diagnostics
                .iter()
                .enumerate()
                .map(|(index, diagnostic)| {
                    DiagnosticStripItem::from_dock_snapshot_diagnostic(
                        offset_diagnostic_id(first_id, index),
                        diagnostic,
                    )
                }),
        );
    }

    /// Appends workspace diagnostics as dock diagnostics followed by workspace-shell diagnostics.
    pub fn extend_workspace_snapshot_diagnostics(
        &mut self,
        first_id: DiagnosticStripItemId,
        diagnostics: &WorkspaceSnapshotDiagnostics,
    ) {
        self.extend_dock_snapshot_diagnostics(first_id, &diagnostics.dock);
        let workspace_first_id = offset_diagnostic_id(first_id, diagnostics.dock.diagnostics.len());
        self.items.extend(
            diagnostics
                .workspace
                .iter()
                .enumerate()
                .map(|(index, diagnostic)| {
                    DiagnosticStripItem::from_workspace_snapshot_diagnostic(
                        offset_diagnostic_id(workspace_first_id, index),
                        diagnostic,
                    )
                }),
        );
    }

    /// Returns a diagnostic by stable identity.
    #[must_use]
    pub fn item(&self, id: DiagnosticStripItemId) -> Option<&DiagnosticStripItem> {
        self.items.iter().find(|item| item.id == id)
    }

    /// Returns diagnostics ordered by severity while preserving insertion order within severity.
    #[must_use]
    pub fn ordered_items(&self) -> Vec<&DiagnosticStripItem> {
        let mut ordered = self.items.iter().enumerate().collect::<Vec<_>>();
        ordered.sort_by_key(|(index, item)| (item.severity.sort_rank(), *index));
        ordered.into_iter().map(|(_, item)| item).collect()
    }

    /// Returns deterministic severity counts.
    #[must_use]
    pub fn summary(&self) -> DiagnosticStripSummary {
        let mut summary = DiagnosticStripSummary::default();
        for item in &self.items {
            match item.severity {
                DiagnosticStripSeverity::Error => summary.errors += 1,
                DiagnosticStripSeverity::Warning => summary.warnings += 1,
                DiagnosticStripSeverity::Info => summary.info += 1,
            }
        }
        summary
    }
}

fn offset_diagnostic_id(id: DiagnosticStripItemId, offset: usize) -> DiagnosticStripItemId {
    let offset = u64::try_from(offset).unwrap_or(u64::MAX);
    DiagnosticStripItemId::from_raw(id.raw().saturating_add(offset))
}
