use kinetik_ui_core::{
    ActionContext, ActionDescriptor, ActionId, ActionInvocation, ActionQueue, ActionSource, Rect,
    Size,
};

use super::{
    OverlayDismissal, OverlayEntry, OverlayId, OverlayKind, OverlayStack, PopoverPlacement,
    PopoverRequest, placement::placed_entry,
};
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
        self.matches_iter().collect()
    }

    /// Returns entries matching the current query as a borrowed iterator.
    pub fn matches_iter(&self) -> impl Iterator<Item = &CommandPaletteEntry> + '_ {
        let query = self.query.trim().to_lowercase();
        self.entries.iter().filter(move |entry| {
            query.is_empty()
                || entry.label.to_lowercase().contains(&query)
                || entry
                    .keywords
                    .iter()
                    .any(|keyword| keyword.to_lowercase().contains(&query))
        })
    }

    /// Returns the number of entries matching the current query.
    #[must_use]
    pub fn match_count(&self) -> usize {
        self.matches_iter().count()
    }

    /// Returns a matching entry by visible match index.
    #[must_use]
    pub fn match_at(&self, visible_index: usize) -> Option<&CommandPaletteEntry> {
        self.matches_iter().nth(visible_index)
    }

    fn selected_match(&self) -> Option<&CommandPaletteEntry> {
        let mut last_match = None;
        for (index, entry) in self.matches_iter().enumerate() {
            if index == self.selected {
                return Some(entry);
            }
            last_match = Some(entry);
        }
        last_match
    }

    /// Clamps the selected index to the current match set.
    ///
    /// Empty result sets deterministically reset selection to zero.
    pub fn clamp_selection(&mut self) {
        self.selected = clamped_selection(self.selected, self.match_count());
    }

    /// Moves selection by a signed amount.
    pub fn move_selection(&mut self, delta: isize) {
        let count = self.match_count();
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
        let entry = self.selected_match()?;
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

    /// Returns entries matching the current query as a borrowed iterator.
    pub fn matches_iter(&self) -> impl Iterator<Item = &CommandPaletteEntry> + '_ {
        self.palette.matches_iter()
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
