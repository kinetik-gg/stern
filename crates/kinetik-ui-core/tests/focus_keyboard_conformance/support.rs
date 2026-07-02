use kinetik_ui_core::{
    ActionBinding, ActionContext, ActionDescriptor, ActionPriority, ActionRouter, Key, KeyEvent,
    KeyState, KeyboardInput, Modifiers, MouseButton, PhysicalKey, Point, Rect, SemanticNode,
    SemanticRole, SemanticTree, Shortcut, Ui, UiTestHarness, WidgetId, focusable,
};

pub(crate) fn ctrl() -> Modifiers {
    Modifiers::new(false, true, false, false)
}

pub(crate) fn super_key() -> Modifiers {
    Modifiers::new(false, false, false, true)
}

pub(crate) fn key_input(key: Key, modifiers: Modifiers) -> KeyboardInput {
    KeyboardInput {
        modifiers,
        events: vec![KeyEvent::new(key, KeyState::Pressed, modifiers, false)],
    }
}

pub(crate) fn physical_input(
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

pub(crate) fn shortcut_action(id: &str, shortcut: Shortcut) -> ActionDescriptor {
    let mut descriptor = ActionDescriptor::new(id, id);
    descriptor.shortcut = Some(shortcut);
    descriptor
}

pub(crate) fn ctrl_shortcut(character: &str) -> Shortcut {
    Shortcut::new(ctrl(), Key::Character(character.to_owned()))
}

pub(crate) fn bind_global(router: &mut ActionRouter, id: &str, shortcut: Shortcut) {
    router.bind(ActionBinding::new(
        shortcut_action(id, shortcut),
        ActionContext::Global,
        ActionPriority::Global,
    ));
}

pub(crate) fn ids() -> (WidgetId, WidgetId, WidgetId, WidgetId) {
    (
        WidgetId::from_key("root"),
        WidgetId::from_key("first"),
        WidgetId::from_key("second"),
        WidgetId::from_key("third"),
    )
}

pub(crate) fn focus_tree() -> SemanticTree {
    let (root, first, second, third) = ids();
    let disabled = WidgetId::from_key("disabled");
    let mut disabled_node =
        SemanticNode::new(disabled, SemanticRole::Button, Rect::ZERO).focusable(true);
    disabled_node.state.disabled = true;

    let mut tree = SemanticTree::new();
    tree.push(
        SemanticNode::new(root, SemanticRole::Root, Rect::ZERO)
            .with_children([second, disabled, first, third]),
    );
    tree.push(SemanticNode::new(first, SemanticRole::Button, Rect::ZERO).focusable(true));
    tree.push(SemanticNode::new(second, SemanticRole::Button, Rect::ZERO).focusable(true));
    tree.push(disabled_node);
    tree.push(SemanticNode::new(third, SemanticRole::Button, Rect::ZERO).focusable(true));
    tree
}

pub(crate) fn focus_tree_with_non_focusable() -> SemanticTree {
    let (root, first, second, third) = ids();
    let disabled = WidgetId::from_key("disabled");
    let label = WidgetId::from_key("label");
    let mut disabled_node =
        SemanticNode::new(disabled, SemanticRole::Button, Rect::ZERO).focusable(true);
    disabled_node.state.disabled = true;

    let mut tree = SemanticTree::new();
    tree.push(
        SemanticNode::new(root, SemanticRole::Root, Rect::ZERO)
            .with_children([second, disabled, label, first, third]),
    );
    tree.push(SemanticNode::new(first, SemanticRole::Button, Rect::ZERO).focusable(true));
    tree.push(SemanticNode::new(second, SemanticRole::Button, Rect::ZERO).focusable(true));
    tree.push(disabled_node);
    tree.push(SemanticNode::new(label, SemanticRole::Label, Rect::ZERO));
    tree.push(SemanticNode::new(third, SemanticRole::Button, Rect::ZERO).focusable(true));
    tree
}

pub(crate) fn focus_tree_with_disabled_parent_subtree() -> SemanticTree {
    let (root, first, second, third) = ids();
    let disabled_parent = WidgetId::from_key("disabled-parent");
    let disabled_child = WidgetId::from_key("disabled-child");
    let mut disabled_parent_node =
        SemanticNode::new(disabled_parent, SemanticRole::Panel, Rect::ZERO)
            .with_children([disabled_child, first]);
    disabled_parent_node.state.disabled = true;

    let mut tree = SemanticTree::new();
    tree.push(
        SemanticNode::new(root, SemanticRole::Root, Rect::ZERO).with_children([
            second,
            disabled_parent,
            third,
        ]),
    );
    tree.push(SemanticNode::new(first, SemanticRole::Button, Rect::ZERO).focusable(true));
    tree.push(SemanticNode::new(second, SemanticRole::Button, Rect::ZERO).focusable(true));
    tree.push(disabled_parent_node);
    tree.push(SemanticNode::new(disabled_child, SemanticRole::Button, Rect::ZERO).focusable(true));
    tree.push(SemanticNode::new(third, SemanticRole::Button, Rect::ZERO).focusable(true));
    tree
}

pub(crate) fn text_owner_tree(owner: WidgetId, rect: Rect) -> SemanticTree {
    let root = WidgetId::from_key("root");
    let mut tree = SemanticTree::new();
    tree.push(SemanticNode::new(root, SemanticRole::Root, Rect::ZERO).with_children([owner]));
    tree.push(SemanticNode::new(owner, SemanticRole::TextField, rect).focusable(true));
    tree
}

pub(crate) fn text_owner_tree_with_disabled_text_field(
    owner: WidgetId,
    owner_rect: Rect,
    disabled: WidgetId,
    disabled_rect: Rect,
) -> SemanticTree {
    let root = WidgetId::from_key("root");
    let mut disabled_node =
        SemanticNode::new(disabled, SemanticRole::TextField, disabled_rect).focusable(true);
    disabled_node.state.disabled = true;

    let mut tree = SemanticTree::new();
    tree.push(
        SemanticNode::new(root, SemanticRole::Root, Rect::ZERO).with_children([owner, disabled]),
    );
    tree.push(SemanticNode::new(owner, SemanticRole::TextField, owner_rect).focusable(true));
    tree.push(disabled_node);
    tree
}

pub(crate) fn emit_tree(ui: &mut Ui<'_>, tree: &SemanticTree) {
    if let Some(root) = tree.root() {
        ui.set_semantic_root(root);
    }
    for node in tree.nodes().iter().cloned() {
        ui.push_semantic_node(node);
    }
}

pub(crate) fn click_focusable(
    harness: &mut UiTestHarness,
    id: WidgetId,
    rect: Rect,
    disabled: bool,
) -> kinetik_ui_core::Response {
    harness.set_pointer_position(Point::new(
        rect.min_x() + rect.width * 0.5,
        rect.min_y() + rect.height * 0.5,
    ));
    harness.pointer_press(MouseButton::Primary);
    let _ = harness.run_frame(|ui| {
        let (input, memory) = ui.input_and_memory_mut();
        focusable(id, rect, input, memory, disabled)
    });
    harness.pointer_release(MouseButton::Primary);
    harness
        .run_frame(|ui| {
            let (input, memory) = ui.input_and_memory_mut();
            focusable(id, rect, input, memory, disabled)
        })
        .0
}
