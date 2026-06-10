//! Overlay, menu, popover, and command palette models.

use kinetik_ui_core::{
    ActionContext, ActionDescriptor, ActionId, ActionQueue, ActionSource, Point, Rect, Size,
};

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
    /// Menu surface.
    Menu,
    /// Command palette surface.
    CommandPalette,
    /// Tooltip surface.
    Tooltip,
}

/// Dismissal behavior for an overlay.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlayDismissal {
    /// Overlay remains open until explicitly closed.
    Manual,
    /// Overlay closes when the pointer activates outside its bounds.
    OutsideClick,
}

/// Overlay entry in top-to-bottom ordering.
#[derive(Debug, Clone, PartialEq)]
pub struct OverlayEntry {
    /// Overlay identity.
    pub id: OverlayId,
    /// Overlay kind.
    pub kind: OverlayKind,
    /// Overlay bounds.
    pub rect: Rect,
    /// Whether this overlay captures interaction before lower overlays.
    pub modal: bool,
    /// Dismissal behavior.
    pub dismissal: OverlayDismissal,
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

    /// Closes an overlay by ID.
    pub fn close(&mut self, id: OverlayId) -> Option<OverlayEntry> {
        let index = self.entries.iter().position(|entry| entry.id == id)?;
        Some(self.entries.remove(index))
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
        self.entries.iter().any(|entry| entry.modal)
    }

    /// Returns overlays that should close for an outside activation point.
    #[must_use]
    pub fn outside_click_close_requests(&self, point: Point) -> Vec<OverlayId> {
        self.entries
            .iter()
            .rev()
            .take_while(|entry| !entry.rect.contains_point(point))
            .filter(|entry| entry.dismissal == OverlayDismissal::OutsideClick)
            .map(|entry| entry.id)
            .collect()
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
        let Some(MenuItem::Action(action)) = self.visible_items().get(visible_index).copied()
        else {
            return false;
        };
        if !action.can_invoke() {
            return false;
        }
        queue.invoke(action.id.clone(), ActionSource::Menu, context);
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
    let mut rect = match request.placement {
        PopoverPlacement::Below => Rect::new(
            request.anchor.x,
            request.anchor.max_y() + request.offset,
            request.size.width,
            request.size.height,
        ),
        PopoverPlacement::Above => Rect::new(
            request.anchor.x,
            request.anchor.y - request.offset - request.size.height,
            request.size.width,
            request.size.height,
        ),
        PopoverPlacement::Right => Rect::new(
            request.anchor.max_x() + request.offset,
            request.anchor.y,
            request.size.width,
            request.size.height,
        ),
        PopoverPlacement::Left => Rect::new(
            request.anchor.x - request.offset - request.size.width,
            request.anchor.y,
            request.size.width,
            request.size.height,
        ),
    };

    if request.fit_viewport {
        rect.x = rect.x.clamp(viewport.x, viewport.max_x() - rect.width);
        rect.y = rect.y.clamp(viewport.y, viewport.max_y() - rect.height);
    }
    rect
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
}

impl From<&ActionDescriptor> for CommandPaletteEntry {
    fn from(action: &ActionDescriptor) -> Self {
        Self {
            action_id: action.id.clone(),
            label: action.label.clone(),
            keywords: action.keywords.clone(),
            enabled: action.can_invoke(),
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
        let matches = self.matches();
        let Some(entry) = matches.get(self.selected) else {
            return false;
        };
        if !entry.enabled {
            return false;
        }
        queue.invoke(
            entry.action_id.clone(),
            ActionSource::CommandPalette,
            context,
        );
        true
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CommandPalette, Menu, OverlayDismissal, OverlayEntry, OverlayId, OverlayKind, OverlayStack,
        PopoverPlacement, PopoverRequest, place_popover,
    };
    use kinetik_ui_core::{
        ActionContext, ActionDescriptor, ActionId, ActionQueue, Point, Rect, Size,
    };

    fn action(id: &str, label: &str) -> ActionDescriptor {
        ActionDescriptor::new(id, label)
    }

    #[test]
    fn overlay_stack_preserves_order_and_replaces_ids() {
        let mut stack = OverlayStack::new();
        let first = OverlayEntry {
            id: OverlayId::from_raw(1),
            kind: OverlayKind::Menu,
            rect: Rect::new(0.0, 0.0, 10.0, 10.0),
            modal: false,
            dismissal: OverlayDismissal::OutsideClick,
        };
        let replacement = OverlayEntry {
            rect: Rect::new(1.0, 1.0, 10.0, 10.0),
            ..first.clone()
        };

        stack.open(first);
        stack.open(OverlayEntry {
            id: OverlayId::from_raw(2),
            kind: OverlayKind::CommandPalette,
            rect: Rect::new(0.0, 0.0, 20.0, 20.0),
            modal: true,
            dismissal: OverlayDismissal::Manual,
        });
        stack.open(replacement);

        assert_eq!(stack.entries().len(), 2);
        assert_eq!(stack.top().expect("top").id, OverlayId::from_raw(1));
        assert!(stack.has_modal());
    }

    #[test]
    fn outside_click_requests_dismissible_overlays() {
        let mut stack = OverlayStack::new();
        stack.open(OverlayEntry {
            id: OverlayId::from_raw(1),
            kind: OverlayKind::Popover,
            rect: Rect::new(0.0, 0.0, 10.0, 10.0),
            modal: false,
            dismissal: OverlayDismissal::OutsideClick,
        });

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
        assert!((rect.y - 60.0).abs() < f32::EPSILON);
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
