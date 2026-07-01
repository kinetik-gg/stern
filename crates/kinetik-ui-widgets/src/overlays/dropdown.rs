use kinetik_ui_core::{Point, Rect, Size, WidgetId};

use crate::collections::{VirtualWindowRequest, virtual_window};

use super::{
    OverlayDismissal, OverlayEntry, OverlayId, OverlayKind, OverlayStack, PopoverPlacement,
    PopoverRequest, placement::placed_entry,
};
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
