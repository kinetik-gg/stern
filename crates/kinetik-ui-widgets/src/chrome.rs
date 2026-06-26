//! Data-only editor chrome contracts.

use kinetik_ui_core::{ActionContext, ActionDescriptor, ActionSource, Rect, Size};

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
