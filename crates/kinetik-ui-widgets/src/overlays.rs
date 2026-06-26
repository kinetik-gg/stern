//! Overlay, menu, popover, and command palette models.

use kinetik_ui_core::{
    ActionContext, ActionDescriptor, ActionId, ActionInvocation, ActionQueue, ActionSource, Point,
    Rect, SemanticAction, SemanticActionKind, SemanticNode, SemanticRole, Size, WidgetId,
};

use crate::collections::{VirtualWindowRequest, virtual_window};

/// Stable overlay identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct OverlayId(u64);

impl OverlayId {
    /// Creates an overlay ID from raw bits.
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

/// Overlay kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlayKind {
    /// Popover surface.
    Popover,
    /// Dropdown surface anchored to a control.
    Dropdown,
    /// Context menu opened from a contextual target.
    ContextMenu,
    /// Menu surface.
    Menu,
    /// Command palette surface.
    CommandPalette,
    /// Tooltip surface.
    Tooltip,
    /// Modal overlay that blocks interaction with lower layers.
    Modal,
    /// Drag preview surface.
    DragPreview,
}

/// Dismissal behavior for an overlay.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlayDismissal {
    /// Overlay remains open until explicitly closed.
    Manual,
    /// Overlay closes when the pointer activates outside its bounds.
    OutsideClick,
    /// Overlay closes when Escape is pressed.
    Escape,
    /// Overlay closes when either outside activation or Escape occurs.
    OutsideClickOrEscape,
}

impl OverlayDismissal {
    fn closes_on_outside_click(self) -> bool {
        matches!(self, Self::OutsideClick | Self::OutsideClickOrEscape)
    }

    fn closes_on_escape(self) -> bool {
        matches!(self, Self::Escape | Self::OutsideClickOrEscape)
    }
}

/// Overlay entry in top-to-bottom ordering.
#[derive(Debug, Clone, PartialEq)]
pub struct OverlayEntry {
    /// Overlay identity.
    pub id: OverlayId,
    /// Parent overlay for nested menu/popover behavior.
    pub parent: Option<OverlayId>,
    /// Overlay kind.
    pub kind: OverlayKind,
    /// Overlay bounds.
    pub rect: Rect,
    /// Whether this overlay captures interaction before lower overlays.
    pub modal: bool,
    /// Dismissal behavior.
    pub dismissal: OverlayDismissal,
}

impl OverlayEntry {
    /// Creates a manual non-modal overlay entry.
    #[must_use]
    pub const fn new(id: OverlayId, kind: OverlayKind, rect: Rect) -> Self {
        Self {
            id,
            parent: None,
            kind,
            rect,
            modal: false,
            dismissal: OverlayDismissal::Manual,
        }
    }

    /// Returns this entry with a parent overlay.
    #[must_use]
    pub const fn with_parent(mut self, parent: OverlayId) -> Self {
        self.parent = Some(parent);
        self
    }

    /// Returns this entry with modality set.
    #[must_use]
    pub const fn modal(mut self, modal: bool) -> Self {
        self.modal = modal;
        self
    }

    /// Returns this entry with dismissal behavior set.
    #[must_use]
    pub const fn dismiss_on(mut self, dismissal: OverlayDismissal) -> Self {
        self.dismissal = dismissal;
        self
    }

    fn captures_lower_layers(&self) -> bool {
        self.modal || self.kind == OverlayKind::Modal
    }

    fn receives_focus(&self) -> bool {
        self.captures_lower_layers()
            || matches!(
                self.kind,
                OverlayKind::Menu
                    | OverlayKind::Dropdown
                    | OverlayKind::ContextMenu
                    | OverlayKind::CommandPalette
            )
    }
}

/// Retained overlay stack.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct OverlayStack {
    entries: Vec<OverlayEntry>,
}

impl OverlayStack {
    /// Creates an empty overlay stack.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds or replaces an overlay at the top of the stack.
    pub fn open(&mut self, entry: OverlayEntry) {
        self.close(entry.id);
        self.entries.push(entry);
    }

    /// Opens a child overlay when its parent is still present.
    ///
    /// Returns `false` when the parent is missing and leaves the stack unchanged.
    pub fn open_child(&mut self, parent: OverlayId, entry: OverlayEntry) -> bool {
        if !self.entries.iter().any(|candidate| candidate.id == parent) {
            return false;
        }
        self.open(entry.with_parent(parent));
        true
    }

    /// Closes an overlay and any nested descendants by ID.
    pub fn close(&mut self, id: OverlayId) -> Option<OverlayEntry> {
        let closed = self.entries.iter().find(|entry| entry.id == id).cloned()?;
        let closing = self.descendant_ids(id);
        self.entries.retain(|entry| !closing.contains(&entry.id));
        Some(closed)
    }

    /// Closes and returns the top overlay.
    pub fn close_top(&mut self) -> Option<OverlayEntry> {
        let id = self.top()?.id;
        self.close(id)
    }

    /// Returns the top overlay.
    #[must_use]
    pub fn top(&self) -> Option<&OverlayEntry> {
        self.entries.last()
    }

    /// Returns overlays in bottom-to-top order.
    #[must_use]
    pub fn entries(&self) -> &[OverlayEntry] {
        &self.entries
    }

    /// Returns true when any modal overlay is open.
    #[must_use]
    pub fn has_modal(&self) -> bool {
        self.entries.iter().any(OverlayEntry::captures_lower_layers)
    }

    /// Returns the overlay that should receive focus by default.
    #[must_use]
    pub fn focus_target(&self) -> Option<OverlayId> {
        self.entries
            .iter()
            .rev()
            .find(|entry| entry.receives_focus())
            .map(|entry| entry.id)
    }

    /// Returns the topmost overlay containing a point.
    #[must_use]
    pub fn topmost_at(&self, point: Point) -> Option<&OverlayEntry> {
        self.entries
            .iter()
            .rev()
            .find(|entry| entry.rect.contains_point(point))
    }

    /// Returns the overlay that captures pointer routing for a point.
    ///
    /// A point inside the topmost overlay routes to that overlay. A modal
    /// overlay captures any point that was not already claimed by a higher
    /// overlay, so lower UI cannot receive interaction through it.
    #[must_use]
    pub fn pointer_capture_target(&self, point: Point) -> Option<OverlayId> {
        self.entries.iter().rev().find_map(|entry| {
            if entry.rect.contains_point(point) || entry.captures_lower_layers() {
                Some(entry.id)
            } else {
                None
            }
        })
    }

    /// Returns overlays that should close for an outside activation point.
    #[must_use]
    pub fn outside_click_close_requests(&self, point: Point) -> Vec<OverlayId> {
        let mut requests = Vec::new();
        for entry in self.entries.iter().rev() {
            if entry.rect.contains_point(point) {
                break;
            }
            if entry.dismissal.closes_on_outside_click() {
                requests.push(entry.id);
            }
            if entry.captures_lower_layers()
                || (!entry.dismissal.closes_on_outside_click() && entry.receives_focus())
            {
                break;
            }
        }
        requests
    }

    /// Returns the top overlay that should close for Escape.
    #[must_use]
    pub fn escape_close_request(&self) -> Option<OverlayId> {
        for entry in self.entries.iter().rev() {
            if entry.dismissal.closes_on_escape() {
                return Some(entry.id);
            }
            if entry.captures_lower_layers() || entry.receives_focus() {
                return None;
            }
        }
        None
    }

    /// Returns dismissal requests for a frame's overlay input.
    #[must_use]
    pub fn dismissal_requests(
        &self,
        outside_activation: Option<Point>,
        escape_pressed: bool,
    ) -> Vec<OverlayId> {
        let mut requests = outside_activation
            .map_or_else(Vec::new, |point| self.outside_click_close_requests(point));
        if escape_pressed
            && let Some(id) = self.escape_close_request()
            && !requests.contains(&id)
        {
            requests.push(id);
        }
        requests
    }

    fn descendant_ids(&self, root: OverlayId) -> Vec<OverlayId> {
        let mut ids = vec![root];
        let mut changed = true;
        while changed {
            changed = false;
            for entry in &self.entries {
                if entry.parent.is_some_and(|parent| ids.contains(&parent))
                    && !ids.contains(&entry.id)
                {
                    ids.push(entry.id);
                    changed = true;
                }
            }
        }
        ids
    }
}

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
        self.items
            .iter()
            .filter(|item| match item {
                MenuItem::Action(action) => action.state.visible,
                MenuItem::Label(_) | MenuItem::Separator => true,
            })
            .collect()
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
        let Some(MenuItem::Action(action)) = self.visible_items().get(visible_index).copied()
        else {
            return None;
        };
        if !action.can_invoke() {
            return None;
        }
        Some(ActionInvocation::new(action.id.clone(), source, context))
    }
}

/// Stable dropdown item identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DropdownItemId(u64);

impl DropdownItemId {
    /// Creates a dropdown item ID from raw bits.
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

/// Data-only dropdown/select item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DropdownItem {
    /// Stable item identity used for selection and highlight state.
    pub id: DropdownItemId,
    /// Display label for the item.
    pub label: String,
    /// Whether the item can be highlighted and selected.
    pub enabled: bool,
}

impl DropdownItem {
    /// Creates an enabled dropdown item.
    #[must_use]
    pub fn new(id: DropdownItemId, label: impl Into<String>) -> Self {
        Self {
            id,
            label: label.into(),
            enabled: true,
        }
    }

    /// Returns this item with enabled state set.
    #[must_use]
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

/// Data-only presentation metadata for a dropdown/select trigger.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DropdownTriggerPresentation {
    /// Label to present on the closed trigger.
    pub label: String,
    /// Selected enabled item identity, if the trigger is showing a selection.
    pub selected_id: Option<DropdownItemId>,
    /// Whether the label is the placeholder fallback.
    pub placeholder: bool,
    /// Whether the trigger should be presented as disabled.
    pub disabled: bool,
    /// Whether the dropdown/select surface is currently open.
    pub open: bool,
}

impl DropdownTriggerPresentation {
    /// Returns true when the trigger is showing a selected enabled item.
    #[must_use]
    pub fn selected(&self) -> bool {
        self.selected_id.is_some() && !self.placeholder
    }

    /// Returns true when this trigger metadata permits invocation.
    #[must_use]
    pub const fn can_invoke(&self) -> bool {
        !self.disabled
    }
}

/// Keyboard-style dropdown highlight movement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropdownHighlightMove {
    /// Move to the previous enabled item, like `ArrowUp`.
    Previous,
    /// Move to the next enabled item, like `ArrowDown`.
    Next,
    /// Move to the first enabled item, like Home.
    First,
    /// Move to the last enabled item, like End.
    Last,
}

/// Data-only dropdown/select model.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DropdownModel {
    items: Vec<DropdownItem>,
    selected: Option<DropdownItemId>,
    highlighted: Option<DropdownItemId>,
}

impl DropdownModel {
    /// Creates an empty dropdown model.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a dropdown model from items.
    #[must_use]
    pub fn from_items(items: impl IntoIterator<Item = DropdownItem>) -> Self {
        let mut model = Self::new();
        model.replace_items(items);
        model
    }

    /// Returns the item list.
    #[must_use]
    pub fn items(&self) -> &[DropdownItem] {
        &self.items
    }

    /// Replaces items and keeps valid selected/highlighted IDs stable.
    pub fn replace_items(&mut self, items: impl IntoIterator<Item = DropdownItem>) {
        self.items = items.into_iter().collect();
        self.reconcile_ids();
    }

    /// Returns the selected item ID.
    #[must_use]
    pub const fn selected_id(&self) -> Option<DropdownItemId> {
        self.selected
    }

    /// Returns the selected enabled item.
    #[must_use]
    pub fn selected_item(&self) -> Option<&DropdownItem> {
        self.selected
            .and_then(|id| self.enabled_item_by_id(id).map(|(_, item)| item))
    }

    /// Resolves the trigger label from the selected enabled item or placeholder.
    #[must_use]
    pub fn trigger_label(&self, placeholder: impl AsRef<str>) -> String {
        self.selected_item().map_or_else(
            || placeholder.as_ref().to_owned(),
            |item| item.label.clone(),
        )
    }

    /// Resolves closed-trigger presentation metadata for select-like dropdowns.
    #[must_use]
    pub fn trigger_presentation(
        &self,
        placeholder: impl Into<String>,
        disabled: bool,
        open: bool,
    ) -> DropdownTriggerPresentation {
        let placeholder = placeholder.into();
        let selected = self.selected_item();
        DropdownTriggerPresentation {
            label: selected.map_or(placeholder, |item| item.label.clone()),
            selected_id: selected.map(|item| item.id),
            placeholder: selected.is_none(),
            disabled,
            open,
        }
    }

    /// Selects an enabled item by stable ID.
    pub fn set_selected_id(&mut self, id: DropdownItemId) -> bool {
        if self.enabled_item_by_id(id).is_none() {
            return false;
        }
        self.selected = Some(id);
        true
    }

    /// Clears the selected item.
    pub fn clear_selection(&mut self) {
        self.selected = None;
    }

    /// Returns the highlighted item ID.
    #[must_use]
    pub const fn highlighted_id(&self) -> Option<DropdownItemId> {
        self.highlighted
    }

    /// Returns the highlighted enabled item.
    #[must_use]
    pub fn highlighted_item(&self) -> Option<&DropdownItem> {
        self.highlighted
            .and_then(|id| self.enabled_item_by_id(id).map(|(_, item)| item))
    }

    /// Highlights an enabled item by stable ID.
    pub fn set_highlighted_id(&mut self, id: DropdownItemId) -> bool {
        if self.enabled_item_by_id(id).is_none() {
            return false;
        }
        self.highlighted = Some(id);
        true
    }

    /// Clears the highlighted item.
    pub fn clear_highlight(&mut self) {
        self.highlighted = None;
    }

    /// Moves the highlight to the previous enabled item.
    pub fn highlight_previous(&mut self) -> Option<DropdownItemId> {
        self.move_highlight(DropdownHighlightMove::Previous)
    }

    /// Moves the highlight to the next enabled item.
    pub fn highlight_next(&mut self) -> Option<DropdownItemId> {
        self.move_highlight(DropdownHighlightMove::Next)
    }

    /// Moves the highlight to the first enabled item.
    pub fn highlight_first(&mut self) -> Option<DropdownItemId> {
        self.move_highlight(DropdownHighlightMove::First)
    }

    /// Moves the highlight to the last enabled item.
    pub fn highlight_last(&mut self) -> Option<DropdownItemId> {
        self.move_highlight(DropdownHighlightMove::Last)
    }

    /// Moves the highlight using keyboard-style movement.
    pub fn move_highlight(&mut self, movement: DropdownHighlightMove) -> Option<DropdownItemId> {
        let index = match movement {
            DropdownHighlightMove::Previous => self.previous_highlight_index(),
            DropdownHighlightMove::Next => self.next_highlight_index(),
            DropdownHighlightMove::First => self.first_enabled_index(),
            DropdownHighlightMove::Last => self.last_enabled_index(),
        }?;
        let id = self.items[index].id;
        self.highlighted = Some(id);
        Some(id)
    }

    /// Selects the highlighted enabled item.
    pub fn select_highlighted(&mut self) -> Option<DropdownItemId> {
        let id = self.highlighted?;
        self.enabled_item_by_id(id)?;
        self.selected = Some(id);
        Some(id)
    }

    fn reconcile_ids(&mut self) {
        if self
            .selected
            .is_some_and(|id| self.enabled_item_by_id(id).is_none())
        {
            self.selected = None;
        }
        if self
            .highlighted
            .is_some_and(|id| self.enabled_item_by_id(id).is_none())
        {
            self.highlighted = None;
        }
    }

    fn enabled_item_by_id(&self, id: DropdownItemId) -> Option<(usize, &DropdownItem)> {
        self.items
            .iter()
            .enumerate()
            .find(|(_, item)| item.id == id && item.enabled)
    }

    fn highlighted_index(&self) -> Option<usize> {
        self.highlighted
            .and_then(|id| self.enabled_item_by_id(id).map(|(index, _)| index))
    }

    fn first_enabled_index(&self) -> Option<usize> {
        self.items.iter().position(|item| item.enabled)
    }

    fn last_enabled_index(&self) -> Option<usize> {
        self.items.iter().rposition(|item| item.enabled)
    }

    fn previous_highlight_index(&self) -> Option<usize> {
        let Some(current) = self.highlighted_index() else {
            return self.last_enabled_index();
        };
        self.items[..current]
            .iter()
            .rposition(|item| item.enabled)
            .or(Some(current))
    }

    fn next_highlight_index(&self) -> Option<usize> {
        let Some(current) = self.highlighted_index() else {
            return self.first_enabled_index();
        };
        self.items[current.saturating_add(1)..]
            .iter()
            .position(|item| item.enabled)
            .map(|offset| current + 1 + offset)
            .or(Some(current))
    }
}

/// Reason a dropdown overlay closed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropdownCloseReason {
    /// The dropdown was closed by pointer activation outside the overlay stack.
    OutsideClick,
    /// The dropdown was closed by Escape.
    Escape,
    /// The dropdown was closed because an enabled item was selected.
    Selection(DropdownItemId),
    /// The dropdown was closed directly by application-owned state.
    Programmatic,
}

/// Result of closing a dropdown overlay.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DropdownCloseResult {
    /// Closed overlay identity.
    pub overlay_id: OverlayId,
    /// Reason the dropdown closed.
    pub reason: DropdownCloseReason,
    /// Widget that should receive focus after the dropdown closes.
    pub focus_return: WidgetId,
    /// Selected item when the close was caused by item selection.
    pub selected_id: Option<DropdownItemId>,
}

impl DropdownCloseResult {
    /// Creates a dropdown close result.
    #[must_use]
    pub const fn new(
        overlay_id: OverlayId,
        reason: DropdownCloseReason,
        focus_return: WidgetId,
    ) -> Self {
        let selected_id = match reason {
            DropdownCloseReason::Selection(id) => Some(id),
            DropdownCloseReason::OutsideClick
            | DropdownCloseReason::Escape
            | DropdownCloseReason::Programmatic => None,
        };
        Self {
            overlay_id,
            reason,
            focus_return,
            selected_id,
        }
    }
}

/// Data-driven dropdown overlay lifecycle descriptor.
#[derive(Debug, Clone, PartialEq)]
pub struct DropdownOverlay {
    /// Overlay stack entry for placement, z-order, focus, and dismissal.
    pub entry: OverlayEntry,
    /// Widget that opened the dropdown and should regain focus when it closes.
    pub trigger_id: WidgetId,
    /// Data-only item model displayed by the dropdown.
    pub model: DropdownModel,
}

impl DropdownOverlay {
    /// Creates a dropdown overlay from an existing stack entry and item model.
    #[must_use]
    pub const fn new(entry: OverlayEntry, trigger_id: WidgetId, model: DropdownModel) -> Self {
        Self {
            entry,
            trigger_id,
            model,
        }
    }

    /// Creates a placed dropdown overlay from an anchor rectangle.
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub fn anchored(
        id: OverlayId,
        trigger_id: WidgetId,
        model: DropdownModel,
        anchor: Rect,
        size: Size,
        placement: PopoverPlacement,
        offset: f32,
        fit_viewport: bool,
        viewport: Rect,
        dismissal: OverlayDismissal,
    ) -> Self {
        Self::new(
            placed_entry(
                id,
                OverlayKind::Dropdown,
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
            trigger_id,
            model,
        )
    }

    /// Opens this dropdown at the top of an overlay stack.
    pub fn open_in(&self, stack: &mut OverlayStack) {
        stack.open(self.entry.clone());
    }

    /// Opens this dropdown as a child of an existing overlay.
    pub fn open_child_in(&self, stack: &mut OverlayStack, parent: OverlayId) -> bool {
        stack.open_child(parent, self.entry.clone())
    }

    /// Closes this dropdown if it is open and returns its focus target.
    pub fn close_in(
        &self,
        stack: &mut OverlayStack,
        reason: DropdownCloseReason,
    ) -> Option<DropdownCloseResult> {
        stack
            .close(self.entry.id)
            .map(|_| DropdownCloseResult::new(self.entry.id, reason, self.trigger_id))
    }

    /// Applies existing overlay-stack dismissal rules and closes this dropdown if requested.
    pub fn dismiss_in(
        &self,
        stack: &mut OverlayStack,
        outside_activation: Option<Point>,
        escape_pressed: bool,
    ) -> Option<DropdownCloseResult> {
        if outside_activation.is_some_and(|point| {
            stack
                .outside_click_close_requests(point)
                .contains(&self.entry.id)
        }) {
            return self.close_in(stack, DropdownCloseReason::OutsideClick);
        }

        if escape_pressed && stack.escape_close_request() == Some(self.entry.id) {
            return self.close_in(stack, DropdownCloseReason::Escape);
        }

        None
    }

    /// Selects an enabled item, closes the dropdown, and returns the selection close result.
    pub fn select_and_close(
        &mut self,
        item_id: DropdownItemId,
        stack: &mut OverlayStack,
    ) -> Option<DropdownCloseResult> {
        self.model.enabled_item_by_id(item_id)?;
        let result = self.close_in(stack, DropdownCloseReason::Selection(item_id))?;
        let selected = self.model.set_selected_id(item_id);
        debug_assert!(selected);
        Some(result)
    }

    /// Selects the highlighted item, closes the dropdown, and returns the selection close result.
    pub fn select_highlighted_and_close(
        &mut self,
        stack: &mut OverlayStack,
    ) -> Option<DropdownCloseResult> {
        let selected = self.model.highlighted_id()?;
        self.model.enabled_item_by_id(selected)?;
        let result = self.close_in(stack, DropdownCloseReason::Selection(selected))?;
        let selected = self.model.set_selected_id(selected);
        debug_assert!(selected);
        Some(result)
    }
}

/// Deterministic visible row range for a dropdown's scrollable menu body.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DropdownVisibleRange {
    /// First visible item index.
    pub start: usize,
    /// Exclusive visible item index.
    pub end: usize,
    /// Finite scroll offset clamped to the content bounds.
    pub scroll_offset: f32,
}

impl DropdownVisibleRange {
    /// Creates an empty visible range.
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            start: 0,
            end: 0,
            scroll_offset: 0.0,
        }
    }

    /// Returns the visible range as a standard exclusive range.
    #[must_use]
    pub fn range(self) -> std::ops::Range<usize> {
        self.start..self.end
    }
}

/// Calculates the visible dropdown item range for a scrollable menu body.
#[must_use]
pub fn dropdown_visible_range(
    item_count: usize,
    row_height: f32,
    viewport_height: f32,
    scroll_offset: f32,
) -> DropdownVisibleRange {
    let window = virtual_window(VirtualWindowRequest {
        item_count,
        scroll_offset,
        viewport_extent: viewport_height,
        item_extent: row_height,
        overscan: 0,
    });

    DropdownVisibleRange {
        start: window.visible_range.start,
        end: window.visible_range.end,
        scroll_offset: window.clamped_scroll_offset,
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

/// Preferred popover placement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PopoverPlacement {
    /// Place below the anchor.
    Below,
    /// Place above the anchor.
    Above,
    /// Place to the right of the anchor.
    Right,
    /// Place to the left of the anchor.
    Left,
}

/// Popover positioning request.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PopoverRequest {
    /// Anchor rectangle.
    pub anchor: Rect,
    /// Overlay size.
    pub size: Size,
    /// Preferred placement.
    pub placement: PopoverPlacement,
    /// Gap between anchor and overlay.
    pub offset: f32,
    /// Whether the output should be clamped inside viewport bounds.
    pub fit_viewport: bool,
}

/// Places a popover in logical units.
#[must_use]
pub fn place_popover(request: PopoverRequest, viewport: Rect) -> Rect {
    let viewport = sanitize_rect(viewport).max_zero();
    let size = Size::new(
        sanitize_extent(request.size.width),
        sanitize_extent(request.size.height),
    );
    let anchor = sanitize_rect(request.anchor);
    let offset = sanitize_extent(request.offset);
    let preferred = popover_rect(anchor, size, request.placement, offset);
    if !request.fit_viewport {
        return preferred;
    }

    let mut rect = preferred;
    for placement in placement_candidates(request.placement) {
        let candidate = popover_rect(anchor, size, placement, offset);
        let adjusted = clamp_popover_cross_axis(candidate, placement, viewport);
        if placement_axis_fits(candidate, placement, viewport) && viewport.contains_rect(adjusted) {
            rect = adjusted;
            break;
        }
    }

    clamp_rect_to_viewport(rect, viewport)
}

fn placed_entry(
    id: OverlayId,
    kind: OverlayKind,
    request: PopoverRequest,
    viewport: Rect,
) -> OverlayEntry {
    OverlayEntry::new(id, kind, place_popover(request, viewport))
}

fn popover_rect(anchor: Rect, size: Size, placement: PopoverPlacement, offset: f32) -> Rect {
    match placement {
        PopoverPlacement::Below => {
            Rect::new(anchor.x, anchor.max_y() + offset, size.width, size.height)
        }
        PopoverPlacement::Above => Rect::new(
            anchor.x,
            anchor.y - offset - size.height,
            size.width,
            size.height,
        ),
        PopoverPlacement::Right => {
            Rect::new(anchor.max_x() + offset, anchor.y, size.width, size.height)
        }
        PopoverPlacement::Left => Rect::new(
            anchor.x - offset - size.width,
            anchor.y,
            size.width,
            size.height,
        ),
    }
}

fn placement_candidates(preferred: PopoverPlacement) -> [PopoverPlacement; 4] {
    match preferred {
        PopoverPlacement::Below => [
            PopoverPlacement::Below,
            PopoverPlacement::Above,
            PopoverPlacement::Right,
            PopoverPlacement::Left,
        ],
        PopoverPlacement::Above => [
            PopoverPlacement::Above,
            PopoverPlacement::Below,
            PopoverPlacement::Right,
            PopoverPlacement::Left,
        ],
        PopoverPlacement::Right => [
            PopoverPlacement::Right,
            PopoverPlacement::Left,
            PopoverPlacement::Below,
            PopoverPlacement::Above,
        ],
        PopoverPlacement::Left => [
            PopoverPlacement::Left,
            PopoverPlacement::Right,
            PopoverPlacement::Below,
            PopoverPlacement::Above,
        ],
    }
}

fn placement_axis_fits(rect: Rect, placement: PopoverPlacement, viewport: Rect) -> bool {
    match placement {
        PopoverPlacement::Below | PopoverPlacement::Above => {
            rect.y >= viewport.y && rect.max_y() <= viewport.max_y()
        }
        PopoverPlacement::Right | PopoverPlacement::Left => {
            rect.x >= viewport.x && rect.max_x() <= viewport.max_x()
        }
    }
}

fn clamp_popover_cross_axis(rect: Rect, placement: PopoverPlacement, viewport: Rect) -> Rect {
    match placement {
        PopoverPlacement::Below | PopoverPlacement::Above => Rect::new(
            clamp_origin(rect.x, rect.width, viewport.x, viewport.max_x()),
            rect.y,
            rect.width,
            rect.height,
        ),
        PopoverPlacement::Right | PopoverPlacement::Left => Rect::new(
            rect.x,
            clamp_origin(rect.y, rect.height, viewport.y, viewport.max_y()),
            rect.width,
            rect.height,
        ),
    }
}

fn clamp_rect_to_viewport(rect: Rect, viewport: Rect) -> Rect {
    Rect::new(
        clamp_origin(rect.x, rect.width, viewport.x, viewport.max_x()),
        clamp_origin(rect.y, rect.height, viewport.y, viewport.max_y()),
        rect.width,
        rect.height,
    )
}

fn clamp_origin(origin: f32, extent: f32, min: f32, max: f32) -> f32 {
    let max_origin = max - extent;
    if max_origin < min {
        min
    } else {
        origin.clamp(min, max_origin)
    }
}

fn sanitize_rect(rect: Rect) -> Rect {
    Rect::new(
        sanitize_coordinate(rect.x),
        sanitize_coordinate(rect.y),
        sanitize_extent(rect.width),
        sanitize_extent(rect.height),
    )
}

fn sanitize_coordinate(value: f32) -> f32 {
    if value.is_finite() { value } else { 0.0 }
}

fn sanitize_extent(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}

/// Builds a semantic node for an overlay surface.
#[must_use]
pub fn overlay_semantics(entry: &OverlayEntry, label: impl Into<String>) -> SemanticNode {
    let role = match entry.kind {
        OverlayKind::Menu | OverlayKind::ContextMenu | OverlayKind::Dropdown => SemanticRole::Menu,
        OverlayKind::CommandPalette => SemanticRole::CommandPalette,
        OverlayKind::Popover => SemanticRole::Custom("popover".to_owned()),
        OverlayKind::Tooltip => SemanticRole::Custom("tooltip".to_owned()),
        OverlayKind::Modal => SemanticRole::Custom("modal".to_owned()),
        OverlayKind::DragPreview => SemanticRole::Custom("drag-preview".to_owned()),
    };
    let mut node =
        SemanticNode::new(WidgetId::from_raw(entry.id.raw()), role, entry.rect).with_label(label);
    if entry.receives_focus() {
        node = node.focusable(true);
    }
    if entry.dismissal != OverlayDismissal::Manual {
        node = node.with_action(SemanticAction::new(SemanticActionKind::Dismiss, "Dismiss"));
    }
    node
}

/// Command palette entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandPaletteEntry {
    /// Action identity.
    pub action_id: ActionId,
    /// Display label.
    pub label: String,
    /// Search keywords.
    pub keywords: Vec<String>,
    /// Whether the action can currently be invoked.
    pub enabled: bool,
    /// Checked/toggled state when the action is checkable.
    pub checked: Option<bool>,
}

impl From<&ActionDescriptor> for CommandPaletteEntry {
    fn from(action: &ActionDescriptor) -> Self {
        Self {
            action_id: action.id.clone(),
            label: action.label.clone(),
            keywords: action.keywords.clone(),
            enabled: action.can_invoke(),
            checked: action.state.checked,
        }
    }
}

/// Command palette model.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CommandPalette {
    /// Search query.
    pub query: String,
    /// Selected visible index.
    pub selected: usize,
    entries: Vec<CommandPaletteEntry>,
}

impl CommandPalette {
    /// Creates a palette from action descriptors.
    #[must_use]
    pub fn from_actions(actions: &[ActionDescriptor]) -> Self {
        Self {
            query: String::new(),
            selected: 0,
            entries: actions
                .iter()
                .filter(|action| action.state.visible)
                .map(CommandPaletteEntry::from)
                .collect(),
        }
    }

    /// Returns entries matching the current query.
    #[must_use]
    pub fn matches(&self) -> Vec<&CommandPaletteEntry> {
        let query = self.query.trim().to_lowercase();
        if query.is_empty() {
            return self.entries.iter().collect();
        }
        self.entries
            .iter()
            .filter(|entry| {
                entry.label.to_lowercase().contains(&query)
                    || entry
                        .keywords
                        .iter()
                        .any(|keyword| keyword.to_lowercase().contains(&query))
            })
            .collect()
    }

    /// Clamps the selected index to the current match set.
    ///
    /// Empty result sets deterministically reset selection to zero.
    pub fn clamp_selection(&mut self) {
        self.selected = clamped_selection(self.selected, self.matches().len());
    }

    /// Moves selection by a signed amount.
    pub fn move_selection(&mut self, delta: isize) {
        let count = self.matches().len();
        if count == 0 {
            self.selected = 0;
            return;
        }
        self.selected = self
            .selected
            .saturating_add_signed(delta)
            .min(count.saturating_sub(1));
    }

    /// Invokes the selected command palette entry.
    pub fn invoke_selected(&self, queue: &mut ActionQueue, context: ActionContext) -> bool {
        let Some(invocation) = self.invocation_for_selected(context) else {
            return false;
        };
        queue.push(invocation);
        true
    }

    /// Creates an invocation for the selected command palette entry.
    #[must_use]
    pub fn invocation_for_selected(&self, context: ActionContext) -> Option<ActionInvocation> {
        let matches = self.matches();
        let entry = matches.get(clamped_selection(self.selected, matches.len()))?;
        if !entry.enabled {
            return None;
        }
        Some(ActionInvocation::new(
            entry.action_id.clone(),
            ActionSource::CommandPalette,
            context,
        ))
    }
}

/// Command palette overlay model.
#[derive(Debug, Clone, PartialEq)]
pub struct CommandPaletteOverlay {
    /// Overlay stack entry for placement, z-order, focus, and dismissal.
    pub entry: OverlayEntry,
    /// Filterable command palette model.
    pub palette: CommandPalette,
    /// Context captured for action invocations emitted by this surface.
    pub context: ActionContext,
}

impl CommandPaletteOverlay {
    /// Creates a command palette overlay from an existing stack entry and palette model.
    #[must_use]
    pub const fn new(entry: OverlayEntry, palette: CommandPalette, context: ActionContext) -> Self {
        Self {
            entry,
            palette,
            context,
        }
    }

    /// Creates a command palette overlay from action descriptors.
    #[must_use]
    pub fn from_actions(
        entry: OverlayEntry,
        actions: &[ActionDescriptor],
        context: ActionContext,
    ) -> Self {
        Self::new(entry, CommandPalette::from_actions(actions), context)
    }

    /// Creates a placed command palette overlay from an anchor rectangle.
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub fn anchored_from_actions(
        id: OverlayId,
        actions: &[ActionDescriptor],
        anchor: Rect,
        size: Size,
        placement: PopoverPlacement,
        offset: f32,
        fit_viewport: bool,
        viewport: Rect,
        dismissal: OverlayDismissal,
        context: ActionContext,
    ) -> Self {
        Self::from_actions(
            placed_entry(
                id,
                OverlayKind::CommandPalette,
                PopoverRequest {
                    anchor,
                    size,
                    placement,
                    offset,
                    fit_viewport,
                },
                viewport,
            )
            .modal(true)
            .dismiss_on(dismissal),
            actions,
            context,
        )
    }

    /// Opens this overlay at the top of an overlay stack.
    pub fn open_in(&self, stack: &mut OverlayStack) {
        stack.open(self.entry.clone());
    }

    /// Returns entries matching the current query.
    #[must_use]
    pub fn matches(&self) -> Vec<&CommandPaletteEntry> {
        self.palette.matches()
    }

    /// Creates an invocation for the selected command palette entry.
    #[must_use]
    pub fn invocation_for_selected(&self) -> Option<ActionInvocation> {
        self.palette.invocation_for_selected(self.context.clone())
    }

    /// Invokes the selected command palette entry into an action queue.
    pub fn invoke_selected(&self, queue: &mut ActionQueue) -> bool {
        let Some(invocation) = self.invocation_for_selected() else {
            return false;
        };
        queue.push(invocation);
        true
    }
}

fn clamped_selection(selected: usize, count: usize) -> usize {
    if count == 0 {
        0
    } else {
        selected.min(count - 1)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CommandPalette, Menu, OverlayDismissal, OverlayEntry, OverlayId, OverlayKind, OverlayStack,
        PopoverPlacement, PopoverRequest, overlay_semantics, place_popover,
    };
    use kinetik_ui_core::{
        ActionContext, ActionDescriptor, ActionId, ActionQueue, Point, Rect, SemanticActionKind,
        SemanticRole, Size,
    };

    fn action(id: &str, label: &str) -> ActionDescriptor {
        ActionDescriptor::new(id, label)
    }

    #[test]
    fn overlay_stack_preserves_order_and_replaces_ids() {
        let mut stack = OverlayStack::new();
        let first = OverlayEntry::new(
            OverlayId::from_raw(1),
            OverlayKind::Menu,
            Rect::new(0.0, 0.0, 10.0, 10.0),
        )
        .dismiss_on(OverlayDismissal::OutsideClick);
        let replacement = OverlayEntry {
            rect: Rect::new(1.0, 1.0, 10.0, 10.0),
            ..first.clone()
        };

        stack.open(first);
        stack.open(
            OverlayEntry::new(
                OverlayId::from_raw(2),
                OverlayKind::CommandPalette,
                Rect::new(0.0, 0.0, 20.0, 20.0),
            )
            .modal(true),
        );
        stack.open(replacement);

        assert_eq!(stack.entries().len(), 2);
        assert_eq!(stack.top().expect("top").id, OverlayId::from_raw(1));
        assert!(stack.has_modal());
    }

    #[test]
    fn outside_click_requests_dismissible_overlays() {
        let mut stack = OverlayStack::new();
        stack.open(
            OverlayEntry::new(
                OverlayId::from_raw(1),
                OverlayKind::Popover,
                Rect::new(0.0, 0.0, 10.0, 10.0),
            )
            .dismiss_on(OverlayDismissal::OutsideClick),
        );

        assert_eq!(
            stack.outside_click_close_requests(Point::new(20.0, 20.0)),
            vec![OverlayId::from_raw(1)]
        );
    }

    #[test]
    fn menu_invokes_enabled_visible_actions() {
        let mut disabled = action("disabled", "Disabled");
        disabled.state.enabled = false;
        let menu = Menu::from_actions([action("open", "Open"), disabled]);
        let mut queue = ActionQueue::new();

        assert!(menu.invoke_visible(0, &mut queue, ActionContext::Global));
        assert!(!menu.invoke_visible(1, &mut queue, ActionContext::Global));

        assert_eq!(
            queue.pop_front().expect("invocation").action_id,
            ActionId::new("open")
        );
    }

    #[test]
    fn hidden_menu_actions_are_filtered() {
        let mut hidden = action("hidden", "Hidden");
        hidden.state.visible = false;
        let menu = Menu::from_actions([action("shown", "Shown"), hidden]);

        assert_eq!(menu.visible_items().len(), 1);
    }

    #[test]
    fn popover_can_be_clamped_inside_viewport() {
        let rect = place_popover(
            PopoverRequest {
                anchor: Rect::new(90.0, 90.0, 10.0, 10.0),
                size: Size::new(40.0, 40.0),
                placement: PopoverPlacement::Below,
                offset: 4.0,
                fit_viewport: true,
            },
            Rect::new(0.0, 0.0, 100.0, 100.0),
        );

        assert!((rect.x - 60.0).abs() < f32::EPSILON);
        assert!((rect.y - 46.0).abs() < f32::EPSILON);
    }

    #[test]
    fn popover_clamp_handles_overlay_larger_than_viewport() {
        let rect = place_popover(
            PopoverRequest {
                anchor: Rect::new(80.0, 80.0, 10.0, 10.0),
                size: Size::new(180.0, 160.0),
                placement: PopoverPlacement::Below,
                offset: 4.0,
                fit_viewport: true,
            },
            Rect::new(0.0, 0.0, 100.0, 100.0),
        );

        assert!((rect.x - 0.0).abs() < f32::EPSILON);
        assert!((rect.y - 0.0).abs() < f32::EPSILON);
        assert_eq!(rect.size(), Size::new(180.0, 160.0));
    }

    #[test]
    fn nested_close_removes_descendants() {
        let mut stack = OverlayStack::new();
        let parent = OverlayId::from_raw(1);
        let child = OverlayId::from_raw(2);
        let grandchild = OverlayId::from_raw(3);

        stack.open(OverlayEntry::new(
            parent,
            OverlayKind::Menu,
            Rect::new(0.0, 0.0, 20.0, 20.0),
        ));
        assert!(stack.open_child(
            parent,
            OverlayEntry::new(
                child,
                OverlayKind::Popover,
                Rect::new(20.0, 0.0, 20.0, 20.0)
            )
        ));
        assert!(stack.open_child(
            child,
            OverlayEntry::new(
                grandchild,
                OverlayKind::ContextMenu,
                Rect::new(40.0, 0.0, 20.0, 20.0),
            )
        ));

        assert_eq!(stack.entries().len(), 3);
        assert_eq!(stack.close(parent).map(|entry| entry.id), Some(parent));
        assert!(stack.entries().is_empty());
    }

    #[test]
    fn overlay_routing_prefers_topmost_hit_and_modal_capture() {
        let mut stack = OverlayStack::new();
        stack.open(OverlayEntry::new(
            OverlayId::from_raw(1),
            OverlayKind::Popover,
            Rect::new(0.0, 0.0, 100.0, 100.0),
        ));
        stack.open(
            OverlayEntry::new(
                OverlayId::from_raw(2),
                OverlayKind::Modal,
                Rect::new(10.0, 10.0, 20.0, 20.0),
            )
            .modal(true),
        );

        assert_eq!(
            stack.pointer_capture_target(Point::new(15.0, 15.0)),
            Some(OverlayId::from_raw(2))
        );
        assert_eq!(
            stack.pointer_capture_target(Point::new(90.0, 90.0)),
            Some(OverlayId::from_raw(2))
        );
        assert_eq!(
            stack.pointer_capture_target(Point::new(150.0, 150.0)),
            Some(OverlayId::from_raw(2))
        );
        assert_eq!(stack.focus_target(), Some(OverlayId::from_raw(2)));
    }

    #[test]
    fn modal_capture_does_not_block_higher_overlay_hits() {
        let mut stack = OverlayStack::new();
        stack.open(OverlayEntry::new(
            OverlayId::from_raw(1),
            OverlayKind::Popover,
            Rect::new(0.0, 0.0, 100.0, 100.0),
        ));
        stack.open(
            OverlayEntry::new(
                OverlayId::from_raw(2),
                OverlayKind::Modal,
                Rect::new(10.0, 10.0, 20.0, 20.0),
            )
            .dismiss_on(OverlayDismissal::OutsideClickOrEscape),
        );
        stack.open(OverlayEntry::new(
            OverlayId::from_raw(3),
            OverlayKind::Tooltip,
            Rect::new(80.0, 80.0, 10.0, 10.0),
        ));

        assert_eq!(
            stack.pointer_capture_target(Point::new(85.0, 85.0)),
            Some(OverlayId::from_raw(3))
        );
        assert_eq!(
            stack.pointer_capture_target(Point::new(50.0, 50.0)),
            Some(OverlayId::from_raw(2))
        );
        assert_eq!(
            stack.outside_click_close_requests(Point::new(50.0, 50.0)),
            vec![OverlayId::from_raw(2)]
        );
        assert_eq!(
            stack.dismissal_requests(None, true),
            vec![OverlayId::from_raw(2)]
        );
    }

    #[test]
    fn dismissal_requests_cover_escape_and_outside_click() {
        let mut stack = OverlayStack::new();
        stack.open(
            OverlayEntry::new(
                OverlayId::from_raw(1),
                OverlayKind::Menu,
                Rect::new(0.0, 0.0, 50.0, 50.0),
            )
            .dismiss_on(OverlayDismissal::OutsideClick),
        );
        stack.open(
            OverlayEntry::new(
                OverlayId::from_raw(2),
                OverlayKind::CommandPalette,
                Rect::new(10.0, 10.0, 50.0, 50.0),
            )
            .dismiss_on(OverlayDismissal::OutsideClickOrEscape),
        );

        assert_eq!(
            stack.dismissal_requests(Some(Point::new(80.0, 80.0)), false),
            vec![OverlayId::from_raw(2), OverlayId::from_raw(1)]
        );
        assert_eq!(
            stack.dismissal_requests(None, true),
            vec![OverlayId::from_raw(2)]
        );
    }

    #[test]
    fn focus_target_skips_non_focusable_overlay_surfaces() {
        let mut stack = OverlayStack::new();

        assert_eq!(stack.focus_target(), None);

        stack.open(OverlayEntry::new(
            OverlayId::from_raw(1),
            OverlayKind::Tooltip,
            Rect::new(0.0, 0.0, 20.0, 20.0),
        ));
        assert_eq!(stack.focus_target(), None);

        stack.open(OverlayEntry::new(
            OverlayId::from_raw(2),
            OverlayKind::Menu,
            Rect::new(0.0, 0.0, 60.0, 60.0),
        ));
        stack.open(OverlayEntry::new(
            OverlayId::from_raw(3),
            OverlayKind::DragPreview,
            Rect::new(10.0, 10.0, 20.0, 20.0),
        ));
        assert_eq!(stack.focus_target(), Some(OverlayId::from_raw(2)));

        stack.open(
            OverlayEntry::new(
                OverlayId::from_raw(4),
                OverlayKind::Popover,
                Rect::new(20.0, 20.0, 20.0, 20.0),
            )
            .modal(true),
        );
        assert_eq!(stack.focus_target(), Some(OverlayId::from_raw(4)));
    }

    #[test]
    fn popover_clamp_handles_non_origin_viewport_edges() {
        let rect = place_popover(
            PopoverRequest {
                anchor: Rect::new(180.0, 95.0, 10.0, 10.0),
                size: Size::new(50.0, 30.0),
                placement: PopoverPlacement::Right,
                offset: 4.0,
                fit_viewport: true,
            },
            Rect::new(100.0, 50.0, 100.0, 80.0),
        );

        assert_eq!(rect, Rect::new(126.0, 95.0, 50.0, 30.0));
    }

    #[test]
    fn overlay_semantics_describe_surface_and_dismissal() {
        let entry = OverlayEntry::new(
            OverlayId::from_raw(7),
            OverlayKind::CommandPalette,
            Rect::new(0.0, 0.0, 100.0, 50.0),
        )
        .dismiss_on(OverlayDismissal::Escape);

        let node = overlay_semantics(&entry, "Commands");

        assert_eq!(node.role, SemanticRole::CommandPalette);
        assert_eq!(node.label.as_deref(), Some("Commands"));
        assert!(node.focusable);
        assert!(
            node.actions
                .iter()
                .any(|action| action.kind == SemanticActionKind::Dismiss)
        );
    }

    #[test]
    fn overlay_semantics_expose_focusability_for_modal_and_non_focusable_surfaces() {
        let modal_popover = OverlayEntry::new(
            OverlayId::from_raw(8),
            OverlayKind::Popover,
            Rect::new(0.0, 0.0, 100.0, 50.0),
        )
        .modal(true)
        .dismiss_on(OverlayDismissal::OutsideClick);
        let tooltip = OverlayEntry::new(
            OverlayId::from_raw(9),
            OverlayKind::Tooltip,
            Rect::new(0.0, 0.0, 40.0, 20.0),
        );

        let modal_node = overlay_semantics(&modal_popover, "Inspector");
        let tooltip_node = overlay_semantics(&tooltip, "Tip");

        assert_eq!(modal_node.role, SemanticRole::Custom("popover".to_owned()));
        assert_eq!(modal_node.label.as_deref(), Some("Inspector"));
        assert!(modal_node.focusable);
        assert!(
            modal_node
                .actions
                .iter()
                .any(|action| action.kind == SemanticActionKind::Dismiss)
        );
        assert_eq!(
            tooltip_node.role,
            SemanticRole::Custom("tooltip".to_owned())
        );
        assert_eq!(tooltip_node.label.as_deref(), Some("Tip"));
        assert!(!tooltip_node.focusable);
        assert!(tooltip_node.actions.is_empty());
    }

    #[test]
    fn command_palette_filters_by_label_and_keyword() {
        let mut save = action("save", "Save Project");
        save.keywords = vec!["write".to_owned()];
        let mut palette = CommandPalette::from_actions(&[save, action("export", "Export")]);

        palette.query = "wri".to_owned();

        assert_eq!(palette.matches()[0].action_id, ActionId::new("save"));
    }

    #[test]
    fn command_palette_entries_preserve_checked_action_state() {
        let mut grid = action("view.grid", "Grid");
        grid.state.checked = Some(true);

        let palette = CommandPalette::from_actions(&[grid]);

        assert_eq!(palette.matches()[0].checked, Some(true));
    }

    #[test]
    fn command_palette_moves_selection_and_invokes() {
        let mut palette =
            CommandPalette::from_actions(&[action("first", "First"), action("second", "Second")]);
        let mut queue = ActionQueue::new();

        palette.move_selection(1);
        assert!(palette.invoke_selected(&mut queue, ActionContext::Global));

        assert_eq!(
            queue.pop_front().expect("invocation").action_id,
            ActionId::new("second")
        );
    }
}
