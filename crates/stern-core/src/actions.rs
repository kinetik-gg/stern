//! Application action descriptors, shortcuts, and invocations.

use std::collections::VecDeque;
use std::fmt;
use std::hash::{Hash, Hasher};

use crate::{Key, KeyEvent, KeyState, KeyboardInput, Modifiers, PhysicalKey, WidgetId};

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
    /// Checked/toggled state when the action is checkable.
    ///
    /// `None` means the action is not checkable. `Some(false)` means it is
    /// checkable but currently off. `Some(true)` means it is checkable and on.
    pub checked: Option<bool>,
}

impl Default for ActionState {
    fn default() -> Self {
        Self {
            visible: true,
            enabled: true,
            checked: None,
        }
    }
}

impl ActionState {
    /// Returns true when this action exposes checked/toggled state.
    #[must_use]
    pub const fn is_checkable(self) -> bool {
        self.checked.is_some()
    }

    /// Returns true when this action is checkable and currently checked.
    #[must_use]
    pub const fn is_checked(self) -> bool {
        matches!(self.checked, Some(true))
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
    /// Whether menu surfaces should present this action as destructive.
    pub destructive: bool,
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
            destructive: false,
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
    /// Optional required physical key.
    ///
    /// When set, matching uses physical key identity instead of layout-resolved
    /// logical key identity.
    pub physical_key: Option<PhysicalKey>,
}

impl Shortcut {
    /// Creates a shortcut.
    #[must_use]
    pub fn new(modifiers: Modifiers, key: Key) -> Self {
        Self {
            modifiers,
            key,
            physical_key: None,
        }
    }

    /// Creates a layout-independent physical-key shortcut.
    #[must_use]
    pub const fn physical(modifiers: Modifiers, physical_key: PhysicalKey) -> Self {
        Self {
            modifiers,
            key: Key::Unidentified,
            physical_key: Some(physical_key),
        }
    }

    /// Returns this shortcut with a physical key requirement.
    #[must_use]
    pub fn with_physical_key(mut self, physical_key: PhysicalKey) -> Self {
        self.physical_key = Some(physical_key);
        self
    }

    /// Returns true when the shortcut matches a keyboard event.
    #[must_use]
    pub fn matches_event(&self, key: &Key, modifiers: Modifiers) -> bool {
        self.physical_key.is_none() && self.modifiers == modifiers && key_matches(&self.key, key)
    }

    /// Returns true when the shortcut matches a complete keyboard event.
    #[must_use]
    pub fn matches_key_event(&self, event: &KeyEvent) -> bool {
        if event.state != KeyState::Pressed || event.repeat || self.modifiers != event.modifiers {
            return false;
        }

        self.physical_key.map_or_else(
            || key_matches(&self.key, &event.key),
            |physical_key| event.physical_key == physical_key,
        )
    }

    /// Returns true when the shortcut was pressed during this frame.
    #[must_use]
    pub fn matches_keyboard(&self, input: &KeyboardInput) -> bool {
        input
            .events
            .iter()
            .any(|event| self.matches_key_event(event))
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
    /// Dock or editor-wide context above global application fallback.
    Editor,
    /// A docked frame or document-like context.
    Frame(WidgetId),
    /// A passive panel context.
    Panel(WidgetId),
    /// A specific focused widget context.
    Widget(WidgetId),
    /// Text input context, which can reserve editing shortcuts.
    TextInput(WidgetId),
    /// Active modal or modal-like interaction context.
    Modal(WidgetId),
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
    /// Dock or editor-wide action.
    Editor = 1,
    /// Docked frame or panel action.
    Container = 2,
    /// Focused widget action.
    FocusedWidget = 3,
    /// Text input action. Reserved editing shortcuts still block non-text bindings.
    TextInput = 4,
    /// Active modal or modal-like interaction action.
    Modal = 5,
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

/// Active context used when resolving shortcuts.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct ActionRoutingContext {
    /// Whether dock or editor-wide bindings are active.
    pub editor: bool,
    /// Active docked frame or document-like context.
    pub frame: Option<WidgetId>,
    /// Active passive panel context.
    pub panel: Option<WidgetId>,
    /// Focused widget context.
    pub focused_widget: Option<WidgetId>,
    /// Focused text input context.
    pub text_input: Option<WidgetId>,
    /// Active modal or modal-like interaction context.
    pub modal: Option<WidgetId>,
}

impl ActionRoutingContext {
    /// Creates a routing context with no focused container or widget.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            editor: false,
            frame: None,
            panel: None,
            focused_widget: None,
            text_input: None,
            modal: None,
        }
    }

    /// Activates dock or editor-wide bindings.
    #[must_use]
    pub const fn with_editor(mut self) -> Self {
        self.editor = true;
        self
    }

    /// Sets the active frame context.
    #[must_use]
    pub const fn with_frame(mut self, frame: WidgetId) -> Self {
        self.frame = Some(frame);
        self
    }

    /// Sets the active panel context.
    #[must_use]
    pub const fn with_panel(mut self, panel: WidgetId) -> Self {
        self.panel = Some(panel);
        self
    }

    /// Sets the focused widget context.
    #[must_use]
    pub const fn with_focused_widget(mut self, widget: WidgetId) -> Self {
        self.focused_widget = Some(widget);
        self
    }

    /// Sets the focused text-input context.
    #[must_use]
    pub const fn with_text_input(mut self, widget: WidgetId) -> Self {
        self.text_input = Some(widget);
        self.focused_widget = Some(widget);
        self
    }

    /// Sets the active modal or modal-like interaction context.
    #[must_use]
    pub const fn with_modal(mut self, modal: WidgetId) -> Self {
        self.modal = Some(modal);
        self
    }
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

    /// Resolves the highest-priority enabled visible global action for the keyboard input.
    ///
    /// Contextual bindings require [`Self::resolve_shortcut_in_context`] so
    /// frame, panel, widget, and text-input shortcuts cannot fire without their
    /// active owner.
    #[must_use]
    pub fn resolve_shortcut(&self, input: &KeyboardInput) -> Option<ActionInvocation> {
        self.resolve_shortcuts(input).pop_front()
    }

    /// Resolves all enabled visible global actions for pressed shortcut events.
    ///
    /// Invocations are returned in keyboard event order. When multiple bindings
    /// match the same event, the highest-priority active binding wins, with
    /// first registration used as the deterministic equal-priority tie breaker.
    #[must_use]
    pub fn resolve_shortcuts(&self, input: &KeyboardInput) -> ActionQueue {
        self.resolve_shortcuts_in_context(input, ActionRoutingContext::new())
    }

    /// Resolves the highest-priority enabled visible action active in a routing context.
    ///
    /// Text inputs reserve editing shortcuts. Global/container/widget bindings do
    /// not receive those shortcuts while a text input is focused unless the
    /// matching binding is explicitly scoped to that text input.
    #[must_use]
    pub fn resolve_shortcut_in_context(
        &self,
        input: &KeyboardInput,
        routing: ActionRoutingContext,
    ) -> Option<ActionInvocation> {
        self.resolve_shortcuts_in_context(input, routing)
            .pop_front()
    }

    /// Resolves all enabled visible actions active in a routing context.
    ///
    /// Invocations are returned in keyboard event order. Each event resolves to
    /// at most one action using modal, text input, focused widget, container,
    /// editor, then global priority.
    #[must_use]
    pub fn resolve_shortcuts_in_context(
        &self,
        input: &KeyboardInput,
        routing: ActionRoutingContext,
    ) -> ActionQueue {
        self.resolve_shortcuts_matching(input, |binding, event| {
            let text_input_reserved =
                routing.text_input.is_some() && text_input_reserves_event(event);
            binding_context_is_active(&binding.context, routing)
                && (!text_input_reserved
                    || binding_context_is_text_input(&binding.context, routing))
        })
    }

    fn resolve_shortcuts_matching(
        &self,
        input: &KeyboardInput,
        filter: impl Fn(&ActionBinding, &KeyEvent) -> bool,
    ) -> ActionQueue {
        let mut queue = ActionQueue::new();
        for event in &input.events {
            if let Some(invocation) = self.resolve_event_matching(event, &filter) {
                queue.push(invocation);
            }
        }
        queue
    }

    fn resolve_event_matching(
        &self,
        event: &KeyEvent,
        filter: &impl Fn(&ActionBinding, &KeyEvent) -> bool,
    ) -> Option<ActionInvocation> {
        self.bindings
            .iter()
            .enumerate()
            .filter_map(|(index, binding)| {
                let shortcut = binding.descriptor.shortcut.as_ref()?;
                if !shortcut.matches_key_event(event) {
                    return None;
                }
                (filter(binding, event) && binding.descriptor.can_invoke())
                    .then_some((index, binding))
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

fn binding_context_is_active(context: &ActionContext, routing: ActionRoutingContext) -> bool {
    match context {
        ActionContext::Global => true,
        ActionContext::Editor => routing.editor,
        ActionContext::Frame(id) => routing.frame == Some(*id),
        ActionContext::Panel(id) => routing.panel == Some(*id),
        ActionContext::Widget(id) => routing.focused_widget == Some(*id),
        ActionContext::TextInput(id) => routing.text_input == Some(*id),
        ActionContext::Modal(id) => routing.modal == Some(*id),
    }
}

fn binding_context_is_text_input(context: &ActionContext, routing: ActionRoutingContext) -> bool {
    matches!(context, ActionContext::TextInput(id) if routing.text_input == Some(*id))
}

fn text_input_reserves_event(event: &KeyEvent) -> bool {
    let standard_editing_modifier = event.modifiers.ctrl || event.modifiers.super_key;
    if standard_editing_modifier && physical_key_is_standard_text_editing(event.physical_key) {
        return true;
    }

    match &event.key {
        Key::Character(character) => {
            if standard_editing_modifier {
                matches!(
                    character.to_ascii_lowercase().as_str(),
                    "a" | "c" | "v" | "x" | "y" | "z"
                )
            } else {
                event.modifiers.is_empty()
            }
        }
        Key::Enter
        | Key::Tab
        | Key::Backspace
        | Key::Delete
        | Key::Insert
        | Key::Home
        | Key::End
        | Key::PageUp
        | Key::PageDown
        | Key::ArrowLeft
        | Key::ArrowRight
        | Key::ArrowUp
        | Key::ArrowDown => true,
        Key::Space => event.modifiers.is_empty(),
        Key::Escape | Key::ContextMenu | Key::Function(_) | Key::Unidentified => false,
    }
}

fn physical_key_is_standard_text_editing(physical_key: PhysicalKey) -> bool {
    matches!(
        physical_key,
        PhysicalKey::KeyA
            | PhysicalKey::KeyC
            | PhysicalKey::KeyV
            | PhysicalKey::KeyX
            | PhysicalKey::KeyY
            | PhysicalKey::KeyZ
    )
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
        ActionPriority, ActionQueue, ActionRouter, ActionRoutingContext, ActionSource, ActionState,
        Shortcut,
    };
    use crate::{Key, KeyEvent, KeyState, KeyboardInput, Modifiers, PhysicalKey, WidgetId};

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

    fn key_input(key: Key, modifiers: Modifiers) -> KeyboardInput {
        KeyboardInput {
            modifiers,
            events: vec![KeyEvent::new(key, KeyState::Pressed, modifiers, false)],
        }
    }

    fn mixed_keyboard(events: Vec<(Key, Modifiers)>) -> KeyboardInput {
        KeyboardInput {
            modifiers: Modifiers::default(),
            events: events
                .into_iter()
                .map(|(key, modifiers)| KeyEvent::new(key, KeyState::Pressed, modifiers, false))
                .collect(),
        }
    }

    fn physical_keyboard(
        character: &str,
        physical_key: PhysicalKey,
        modifiers: Modifiers,
    ) -> KeyboardInput {
        KeyboardInput {
            modifiers,
            events: vec![KeyEvent::with_physical_key(
                Key::Character(character.to_owned()),
                physical_key,
                KeyState::Pressed,
                modifiers,
                false,
            )],
        }
    }

    #[test]
    fn descriptor_defaults_to_visible_enabled_not_checkable() {
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
    fn destructive_metadata_is_opt_in_without_changing_identity_or_invocability() {
        use std::{collections::hash_map::DefaultHasher, hash::Hasher};

        let neutral = ActionDescriptor::new("file.delete", "Delete");
        let mut destructive = neutral.clone();
        destructive.destructive = true;
        let mut neutral_hash = DefaultHasher::new();
        let mut destructive_hash = DefaultHasher::new();
        std::hash::Hash::hash(&neutral, &mut neutral_hash);
        std::hash::Hash::hash(&destructive, &mut destructive_hash);

        assert!(!neutral.destructive);
        assert!(destructive.destructive);
        assert_eq!(neutral.id, destructive.id);
        assert_eq!(neutral_hash.finish(), destructive_hash.finish());
        assert!(neutral.can_invoke() && destructive.can_invoke());
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
    fn physical_shortcut_matches_layout_independent_key() {
        let modifiers = Modifiers::new(false, true, false, false);
        let shortcut = Shortcut::physical(modifiers, PhysicalKey::KeyY);

        assert!(shortcut.matches_keyboard(&physical_keyboard("z", PhysicalKey::KeyY, modifiers,)));
        assert!(!shortcut.matches_keyboard(&keyboard("y")));
    }

    #[test]
    fn contextual_router_uses_highest_priority_active_context_for_same_shortcut() {
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
            .resolve_shortcut_in_context(
                &keyboard("a"),
                ActionRoutingContext::new().with_text_input(widget),
            )
            .expect("shortcut invocation");

        assert_eq!(invocation.action_id, ActionId::new("select.all.text"));
        assert_eq!(invocation.context, ActionContext::TextInput(widget));
    }

    #[test]
    fn contextless_router_ignores_inactive_contextual_bindings() {
        let widget = WidgetId::from_key("field");
        let mut text = ActionDescriptor::new("select.all.text", "Select All In Field");
        text.shortcut = Some(ctrl_key("a"));
        let mut router = ActionRouter::new();
        router.bind(ActionBinding::new(
            text,
            ActionContext::TextInput(widget),
            ActionPriority::TextInput,
        ));

        assert_eq!(router.resolve_shortcut(&keyboard("a")), None);
    }

    #[test]
    fn contextual_router_blocks_global_text_editing_shortcuts() {
        let widget = WidgetId::from_key("field");
        let mut global = ActionDescriptor::new("select.all.global", "Select All");
        global.shortcut = Some(ctrl_key("a"));
        let mut router = ActionRouter::new();
        router.bind(ActionBinding::new(
            global,
            ActionContext::Global,
            ActionPriority::Global,
        ));

        let routing = ActionRoutingContext::new().with_text_input(widget);

        assert_eq!(
            router.resolve_shortcut_in_context(&keyboard("a"), routing),
            None
        );
    }

    #[test]
    fn contextual_router_blocks_global_space_while_text_input_is_focused() {
        let widget = WidgetId::from_key("field");
        let mut global = ActionDescriptor::new("play.pause", "Play/Pause");
        global.shortcut = Some(Shortcut::new(Modifiers::default(), Key::Space));
        let mut router = ActionRouter::new();
        router.bind(ActionBinding::new(
            global,
            ActionContext::Global,
            ActionPriority::Global,
        ));

        assert_eq!(
            router.resolve_shortcut_in_context(
                &key_input(Key::Space, Modifiers::default()),
                ActionRoutingContext::new().with_text_input(widget),
            ),
            None
        );
    }

    #[test]
    fn contextual_router_allows_unrelated_global_shortcut_in_mixed_text_frame() {
        let widget = WidgetId::from_key("field");
        let ctrl = Modifiers::new(false, true, false, false);
        let mut save = ActionDescriptor::new("file.save", "Save");
        save.shortcut = Some(ctrl_key("s"));
        let mut router = ActionRouter::new();
        router.bind(ActionBinding::new(
            save,
            ActionContext::Global,
            ActionPriority::Global,
        ));
        let input = mixed_keyboard(vec![
            (Key::Character("x".to_owned()), Modifiers::default()),
            (Key::Character("s".to_owned()), ctrl),
        ]);

        let invocation = router
            .resolve_shortcut_in_context(
                &input,
                ActionRoutingContext::new().with_text_input(widget),
            )
            .expect("global shortcut");

        assert_eq!(invocation.action_id, ActionId::new("file.save"));
        assert_eq!(invocation.context, ActionContext::Global);
    }

    #[test]
    fn contextual_router_still_blocks_global_binding_for_reserved_text_event() {
        let widget = WidgetId::from_key("field");
        let mut type_x = ActionDescriptor::new("global.x", "Global X");
        type_x.shortcut = Some(Shortcut::new(
            Modifiers::default(),
            Key::Character("x".to_owned()),
        ));
        let mut router = ActionRouter::new();
        router.bind(ActionBinding::new(
            type_x,
            ActionContext::Global,
            ActionPriority::Global,
        ));
        let input = mixed_keyboard(vec![(Key::Character("x".to_owned()), Modifiers::default())]);

        assert_eq!(
            router.resolve_shortcut_in_context(
                &input,
                ActionRoutingContext::new().with_text_input(widget),
            ),
            None
        );
    }

    #[test]
    fn contextual_router_allows_text_input_scoped_editing_action() {
        let widget = WidgetId::from_key("field");
        let mut global = ActionDescriptor::new("select.all.global", "Select All");
        global.shortcut = Some(ctrl_key("a"));
        let mut text = ActionDescriptor::new("select.all.text", "Select All In Field");
        text.shortcut = Some(ctrl_key("a"));
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
            .resolve_shortcut_in_context(
                &keyboard("a"),
                ActionRoutingContext::new().with_text_input(widget),
            )
            .expect("text action");

        assert_eq!(invocation.action_id, ActionId::new("select.all.text"));
        assert_eq!(invocation.context, ActionContext::TextInput(widget));
    }

    #[test]
    fn contextual_router_ignores_inactive_widget_binding() {
        let focused = WidgetId::from_key("focused");
        let other = WidgetId::from_key("other");
        let mut widget_action = ActionDescriptor::new("widget.run", "Run");
        widget_action.shortcut = Some(ctrl_key("r"));
        let mut router = ActionRouter::new();
        router.bind(ActionBinding::new(
            widget_action,
            ActionContext::Widget(other),
            ActionPriority::FocusedWidget,
        ));

        assert_eq!(
            router.resolve_shortcut_in_context(
                &keyboard("r"),
                ActionRoutingContext::new().with_focused_widget(focused),
            ),
            None
        );
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
        descriptor.state.checked = Some(true);

        assert!(descriptor.state.is_checkable());
        assert!(descriptor.state.is_checked());
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
