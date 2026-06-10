//! Application action descriptors, shortcuts, and invocations.

use std::collections::VecDeque;
use std::fmt;
use std::hash::{Hash, Hasher};

use crate::{Key, KeyState, KeyboardInput, Modifiers, WidgetId};

/// Stable identity for an application-provided action.
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ActionId(String);

impl ActionId {
    /// Creates an action ID.
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Returns the action ID string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for ActionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("ActionId").field(&self.0).finish()
    }
}

/// Optional symbolic icon name attached to an action.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ActionIcon(String);

impl ActionIcon {
    /// Creates an action icon name.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    /// Returns the symbolic icon name.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Runtime state used by UI surfaces before presenting an action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActionState {
    /// Whether the action should be visible in UI surfaces.
    pub visible: bool,
    /// Whether the action can currently be invoked.
    pub enabled: bool,
    /// Whether the action is currently checked/toggled.
    pub checked: bool,
}

impl Default for ActionState {
    fn default() -> Self {
        Self {
            visible: true,
            enabled: true,
            checked: false,
        }
    }
}

/// Application-provided action metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionDescriptor {
    /// Stable action identity.
    pub id: ActionId,
    /// Human-readable label for menus, command palettes, and buttons.
    pub label: String,
    /// Optional symbolic icon.
    pub icon: Option<ActionIcon>,
    /// Optional tooltip or longer description.
    pub tooltip: Option<String>,
    /// Search keywords for command palette use.
    pub keywords: Vec<String>,
    /// Optional shortcut shown by UI surfaces and matched by the action router.
    pub shortcut: Option<Shortcut>,
    /// Presentation and availability state.
    pub state: ActionState,
}

impl ActionDescriptor {
    /// Creates an enabled, visible action descriptor.
    #[must_use]
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: ActionId::new(id),
            label: label.into(),
            icon: None,
            tooltip: None,
            keywords: Vec::new(),
            shortcut: None,
            state: ActionState::default(),
        }
    }

    /// Returns true when a UI surface should be allowed to invoke the action.
    #[must_use]
    pub const fn can_invoke(&self) -> bool {
        self.state.visible && self.state.enabled
    }
}

/// Platform-independent keyboard shortcut.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Shortcut {
    /// Required modifiers.
    pub modifiers: Modifiers,
    /// Required key.
    pub key: Key,
}

impl Shortcut {
    /// Creates a shortcut.
    #[must_use]
    pub fn new(modifiers: Modifiers, key: Key) -> Self {
        Self { modifiers, key }
    }

    /// Returns true when the shortcut matches a keyboard event.
    #[must_use]
    pub fn matches_event(&self, key: &Key, modifiers: Modifiers) -> bool {
        self.modifiers == modifiers && key_matches(&self.key, key)
    }

    /// Returns true when the shortcut was pressed during this frame.
    #[must_use]
    pub fn matches_keyboard(&self, input: &KeyboardInput) -> bool {
        input.events.iter().any(|event| {
            event.state == KeyState::Pressed && self.matches_event(&event.key, event.modifiers)
        })
    }
}

fn key_matches(expected: &Key, actual: &Key) -> bool {
    match (expected, actual) {
        (Key::Character(a), Key::Character(b)) => a.eq_ignore_ascii_case(b),
        _ => expected == actual,
    }
}

/// Broad UI context that emitted or resolved an action.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ActionContext {
    /// Global application context.
    Global,
    /// A docked frame or document-like context.
    Frame(WidgetId),
    /// A passive panel context.
    Panel(WidgetId),
    /// A specific focused widget context.
    Widget(WidgetId),
    /// Text input context, which can reserve editing shortcuts.
    TextInput(WidgetId),
}

/// Source surface that emitted an action invocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ActionSource {
    /// Menu bar or menu item.
    Menu,
    /// Keyboard shortcut.
    Shortcut,
    /// Command palette.
    CommandPalette,
    /// Button or icon button.
    Button,
    /// Programmatic UI path.
    Programmatic,
}

/// Action emitted by UI surfaces for the application to execute.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionInvocation {
    /// Invoked action identity.
    pub action_id: ActionId,
    /// Source surface.
    pub source: ActionSource,
    /// Context snapshot at the time of invocation.
    pub context: ActionContext,
}

impl ActionInvocation {
    /// Creates an action invocation.
    #[must_use]
    pub fn new(action_id: ActionId, source: ActionSource, context: ActionContext) -> Self {
        Self {
            action_id,
            source,
            context,
        }
    }
}

/// FIFO queue of action invocations emitted during a frame.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ActionQueue {
    invocations: VecDeque<ActionInvocation>,
}

impl ActionQueue {
    /// Creates an empty action queue.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds an invocation to the back of the queue.
    pub fn push(&mut self, invocation: ActionInvocation) {
        self.invocations.push_back(invocation);
    }

    /// Adds an invocation from simple parts.
    pub fn invoke(&mut self, action_id: ActionId, source: ActionSource, context: ActionContext) {
        self.push(ActionInvocation::new(action_id, source, context));
    }

    /// Returns the number of queued invocations.
    #[must_use]
    pub fn len(&self) -> usize {
        self.invocations.len()
    }

    /// Returns true when no invocations are queued.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.invocations.is_empty()
    }

    /// Removes and returns the next queued invocation.
    pub fn pop_front(&mut self) -> Option<ActionInvocation> {
        self.invocations.pop_front()
    }

    /// Drains all queued invocations in FIFO order.
    pub fn drain(&mut self) -> impl Iterator<Item = ActionInvocation> + '_ {
        self.invocations.drain(..)
    }
}

/// Context priority used when multiple action bindings could match.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ActionPriority {
    /// Global fallback action.
    Global = 0,
    /// Docked frame or panel action.
    Container = 1,
    /// Focused widget action.
    FocusedWidget = 2,
    /// Text input action. This has highest priority because editing shortcuts should win.
    TextInput = 3,
}

/// A shortcut binding to an action in a specific context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionBinding {
    /// Action descriptor.
    pub descriptor: ActionDescriptor,
    /// Context to report if this binding matches.
    pub context: ActionContext,
    /// Routing priority.
    pub priority: ActionPriority,
}

impl ActionBinding {
    /// Creates an action binding.
    #[must_use]
    pub fn new(
        descriptor: ActionDescriptor,
        context: ActionContext,
        priority: ActionPriority,
    ) -> Self {
        Self {
            descriptor,
            context,
            priority,
        }
    }
}

/// Resolves shortcuts into action invocations.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ActionRouter {
    bindings: Vec<ActionBinding>,
}

impl ActionRouter {
    /// Creates an empty action router.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a binding and preserves insertion order for equal-priority matches.
    pub fn bind(&mut self, binding: ActionBinding) {
        self.bindings.push(binding);
    }

    /// Resolves the highest-priority enabled visible action for the keyboard input.
    #[must_use]
    pub fn resolve_shortcut(&self, input: &KeyboardInput) -> Option<ActionInvocation> {
        self.bindings
            .iter()
            .enumerate()
            .filter(|(_, binding)| {
                binding.descriptor.can_invoke()
                    && binding
                        .descriptor
                        .shortcut
                        .as_ref()
                        .is_some_and(|shortcut| shortcut.matches_keyboard(input))
            })
            .max_by(|(left_index, left), (right_index, right)| {
                left.priority
                    .cmp(&right.priority)
                    .then_with(|| right_index.cmp(left_index))
            })
            .map(|(_, binding)| {
                ActionInvocation::new(
                    binding.descriptor.id.clone(),
                    ActionSource::Shortcut,
                    binding.context.clone(),
                )
            })
    }
}

impl Hash for ActionDescriptor {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ActionBinding, ActionContext, ActionDescriptor, ActionIcon, ActionId, ActionInvocation,
        ActionPriority, ActionQueue, ActionRouter, ActionSource, ActionState, Shortcut,
    };
    use crate::{Key, KeyEvent, KeyState, KeyboardInput, Modifiers, WidgetId};

    fn ctrl_key(character: &str) -> Shortcut {
        Shortcut::new(
            Modifiers::new(false, true, false, false),
            Key::Character(character.to_owned()),
        )
    }

    fn keyboard(character: &str) -> KeyboardInput {
        KeyboardInput {
            modifiers: Modifiers::new(false, true, false, false),
            events: vec![KeyEvent::new(
                Key::Character(character.to_owned()),
                KeyState::Pressed,
                Modifiers::new(false, true, false, false),
                false,
            )],
        }
    }

    #[test]
    fn descriptor_defaults_to_visible_enabled_unchecked() {
        let mut descriptor = ActionDescriptor::new("file.save", "Save");
        descriptor.icon = Some(ActionIcon::new("save"));
        descriptor.tooltip = Some("Save current project".to_owned());
        descriptor.keywords = vec!["write".to_owned(), "persist".to_owned()];

        assert_eq!(descriptor.id.as_str(), "file.save");
        assert_eq!(descriptor.icon.as_ref().expect("icon").as_str(), "save");
        assert!(descriptor.can_invoke());
        assert_eq!(descriptor.state, ActionState::default());
    }

    #[test]
    fn disabled_or_hidden_descriptors_cannot_invoke() {
        let mut descriptor = ActionDescriptor::new("export", "Export");

        descriptor.state.enabled = false;
        assert!(!descriptor.can_invoke());

        descriptor.state.enabled = true;
        descriptor.state.visible = false;
        assert!(!descriptor.can_invoke());
    }

    #[test]
    fn action_queue_preserves_fifo_order_and_drains() {
        let mut queue = ActionQueue::new();
        queue.invoke(
            ActionId::new("one"),
            ActionSource::Button,
            ActionContext::Global,
        );
        queue.invoke(
            ActionId::new("two"),
            ActionSource::Menu,
            ActionContext::Global,
        );

        assert_eq!(queue.len(), 2);
        assert_eq!(queue.pop_front().expect("first").action_id.as_str(), "one");
        assert_eq!(
            queue
                .drain()
                .map(|invocation| invocation.action_id)
                .collect::<Vec<_>>(),
            vec![ActionId::new("two")]
        );
        assert!(queue.is_empty());
    }

    #[test]
    fn shortcut_matches_keyboard_press_case_insensitively() {
        let shortcut = ctrl_key("s");

        assert!(shortcut.matches_keyboard(&keyboard("S")));
    }

    #[test]
    fn router_uses_highest_priority_context_for_same_shortcut() {
        let shortcut = ctrl_key("a");
        let widget = WidgetId::from_key("field");
        let mut global = ActionDescriptor::new("select.all.global", "Select All");
        global.shortcut = Some(shortcut.clone());
        let mut text = ActionDescriptor::new("select.all.text", "Select All In Field");
        text.shortcut = Some(shortcut);

        let mut router = ActionRouter::new();
        router.bind(ActionBinding::new(
            global,
            ActionContext::Global,
            ActionPriority::Global,
        ));
        router.bind(ActionBinding::new(
            text,
            ActionContext::TextInput(widget),
            ActionPriority::TextInput,
        ));

        let invocation = router
            .resolve_shortcut(&keyboard("a"))
            .expect("shortcut invocation");

        assert_eq!(invocation.action_id, ActionId::new("select.all.text"));
        assert_eq!(invocation.context, ActionContext::TextInput(widget));
    }

    #[test]
    fn router_ignores_disabled_actions() {
        let mut disabled = ActionDescriptor::new("save", "Save");
        disabled.shortcut = Some(ctrl_key("s"));
        disabled.state.enabled = false;

        let mut router = ActionRouter::new();
        router.bind(ActionBinding::new(
            disabled,
            ActionContext::Global,
            ActionPriority::Global,
        ));

        assert_eq!(router.resolve_shortcut(&keyboard("s")), None);
    }

    #[test]
    fn checked_action_state_is_available_to_surfaces() {
        let mut descriptor = ActionDescriptor::new("view.grid", "Grid");
        descriptor.state.checked = true;

        assert!(descriptor.state.checked);
        assert!(descriptor.can_invoke());
    }

    #[test]
    fn equal_priority_uses_first_registered_binding() {
        let mut first = ActionDescriptor::new("first", "First");
        first.shortcut = Some(ctrl_key("k"));
        let mut second = ActionDescriptor::new("second", "Second");
        second.shortcut = Some(ctrl_key("k"));

        let mut router = ActionRouter::new();
        router.bind(ActionBinding::new(
            first,
            ActionContext::Global,
            ActionPriority::Global,
        ));
        router.bind(ActionBinding::new(
            second,
            ActionContext::Global,
            ActionPriority::Global,
        ));

        let invocation = router.resolve_shortcut(&keyboard("k")).expect("invocation");

        assert_eq!(invocation.action_id, ActionId::new("first"));
    }

    #[test]
    fn invocation_records_source_context_and_action() {
        let id = ActionId::new("run");
        let invocation = ActionInvocation::new(
            id.clone(),
            ActionSource::CommandPalette,
            ActionContext::Global,
        );

        assert_eq!(invocation.action_id, id);
        assert_eq!(invocation.source, ActionSource::CommandPalette);
        assert_eq!(invocation.context, ActionContext::Global);
    }
}
