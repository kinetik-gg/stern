use kinetik_ui_core::{
    ActionContext, ActionDescriptor, ActionIcon, ActionId, ActionInvocation, ActionQueue,
    ActionSource,
};

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
        self.visible_items_iter().collect()
    }

    /// Returns visible toolbar items as a borrowed iterator.
    pub fn visible_items_iter(&self) -> impl Iterator<Item = &ToolbarItem> + '_ {
        self.items.iter().filter(|item| item.visible())
    }

    /// Returns true when this group has at least one visible item.
    #[must_use]
    pub fn has_visible_items(&self) -> bool {
        self.visible_items_iter().next().is_some()
    }

    /// Creates an invocation for an enabled visible toolbar item by visible index.
    #[must_use]
    pub fn invocation_for_visible(
        &self,
        visible_index: usize,
        context: ActionContext,
    ) -> Option<ActionInvocation> {
        self.visible_items_iter()
            .nth(visible_index)
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
        self.visible_groups_iter().collect()
    }

    /// Returns visible groups as a borrowed iterator.
    pub fn visible_groups_iter(&self) -> impl Iterator<Item = &ToolbarGroup> + '_ {
        self.groups.iter().filter(|group| group.has_visible_items())
    }

    /// Creates an invocation by visible group index and visible item index.
    #[must_use]
    pub fn invocation_for_visible(
        &self,
        visible_group_index: usize,
        visible_item_index: usize,
        context: ActionContext,
    ) -> Option<ActionInvocation> {
        self.visible_groups_iter()
            .nth(visible_group_index)
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
