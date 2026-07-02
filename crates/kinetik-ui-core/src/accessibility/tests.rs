use std::convert::Infallible;

use super::{
    AccessibilityAdapter, AccessibilitySnapshot, FocusTraversal, SemanticAction,
    SemanticActionKind, SemanticNode, SemanticRole, SemanticState, SemanticTree, SemanticTreeError,
    SemanticValue,
};
use crate::{ActionDescriptor, Rect, WidgetId};

#[derive(Debug, Default)]
struct RecordingAdapter {
    synchronized: Vec<AccessibilitySnapshot>,
    focused: Vec<WidgetId>,
    actions: Vec<(WidgetId, SemanticActionKind)>,
}

impl AccessibilityAdapter for RecordingAdapter {
    type Error = Infallible;

    fn synchronize(&mut self, snapshot: &AccessibilitySnapshot) -> Result<(), Self::Error> {
        self.synchronized.push(snapshot.clone());
        Ok(())
    }

    fn focus(&mut self, node: WidgetId) -> Result<(), Self::Error> {
        self.focused.push(node);
        Ok(())
    }

    fn perform_action(
        &mut self,
        node: WidgetId,
        action: &SemanticActionKind,
    ) -> Result<(), Self::Error> {
        self.actions.push((node, action.clone()));
        Ok(())
    }
}

#[test]
fn semantic_tree_preserves_nodes_and_focus_order() {
    let root = WidgetId::from_key("root");
    let disabled = WidgetId::from_key("disabled");
    let button = WidgetId::from_key("button");
    let mut tree = SemanticTree::new();
    tree.push(SemanticNode::new(root, SemanticRole::Root, Rect::ZERO));
    tree.push(
        SemanticNode::new(disabled, SemanticRole::Button, Rect::ZERO)
            .focusable(true)
            .with_label("Disabled"),
    );
    tree.nodes[1].state.disabled = true;
    tree.push(SemanticNode::new(button, SemanticRole::Button, Rect::ZERO).focusable(true));

    assert_eq!(tree.root(), Some(root));
    assert_eq!(
        tree.get(button).map(|node| &node.role),
        Some(&SemanticRole::Button)
    );
    assert_eq!(tree.focus_order(), vec![button]);
}

#[test]
fn focus_traversal_wraps_in_both_directions() {
    let first = WidgetId::from_key("first");
    let second = WidgetId::from_key("second");
    let traversal = FocusTraversal {
        order: vec![first, second],
        focused: Some(second),
    };

    assert_eq!(traversal.next(), Some(first));
    assert_eq!(traversal.previous(), Some(first));
}

#[test]
fn action_descriptor_maps_to_semantic_invoke_action() {
    let descriptor = ActionDescriptor::new("file.save", "Save");
    let action = SemanticAction::from_action_descriptor(&descriptor);

    assert_eq!(action.kind, SemanticActionKind::Invoke);
    assert_eq!(action.label, "Save");
    assert_eq!(action.action_id, Some(descriptor.id));
}

#[test]
fn semantic_state_tracks_roles_values_and_actions() {
    let id = WidgetId::from_key("slider");
    let node = SemanticNode::new(id, SemanticRole::Slider, Rect::new(0.0, 0.0, 100.0, 16.0))
        .focusable(true)
        .with_action(SemanticAction::new(
            SemanticActionKind::Increment,
            "Increase",
        ));

    let mut state = SemanticState {
        value: Some(SemanticValue::Number {
            current: 0.5,
            min: 0.0,
            max: 1.0,
        }),
        ..SemanticState::default()
    };
    state.checked = Some(false);

    assert!(node.focusable);
    assert!(
        node.actions
            .iter()
            .any(|action| action.kind == SemanticActionKind::Focus)
    );
    assert_eq!(state.checked, Some(false));
}

#[test]
fn semantic_tree_traversal_uses_declared_child_order() {
    let root = WidgetId::from_key("root");
    let first = WidgetId::from_key("first");
    let second = WidgetId::from_key("second");
    let mut tree = SemanticTree::new();
    tree.push(
        SemanticNode::new(root, SemanticRole::Root, Rect::ZERO).with_children([second, first]),
    );
    tree.push(SemanticNode::new(first, SemanticRole::Button, Rect::ZERO).focusable(true));
    tree.push(SemanticNode::new(second, SemanticRole::Button, Rect::ZERO).focusable(true));

    assert_eq!(tree.traversal_order(), vec![root, second, first]);
    assert_eq!(tree.focus_order(), vec![second, first]);
    assert_eq!(tree.parent_of(first), Some(root));
    assert!(tree.validate().is_ok());
}

#[test]
fn accessibility_snapshot_exports_validated_semantics_in_traversal_order() {
    let root = WidgetId::from_key("root");
    let button = WidgetId::from_key("button");
    let slider = WidgetId::from_key("slider");
    let unparented = WidgetId::from_key("unparented");
    let mut tree = SemanticTree::new();
    tree.push(
        SemanticNode::new(root, SemanticRole::Root, Rect::ZERO).with_children([slider, button]),
    );
    tree.push(
        SemanticNode::new(
            button,
            SemanticRole::Button,
            Rect::new(0.0, 0.0, 80.0, 28.0),
        )
        .focusable(true)
        .with_label("Run")
        .with_action(SemanticAction::new(SemanticActionKind::Invoke, "Run")),
    );
    let mut slider_node = SemanticNode::new(
        slider,
        SemanticRole::Slider,
        Rect::new(0.0, 32.0, 120.0, 18.0),
    )
    .focusable(true)
    .with_label("Opacity")
    .with_action(SemanticAction::new(
        SemanticActionKind::Increment,
        "Increase",
    ));
    slider_node.state.value = Some(SemanticValue::Number {
        current: 0.5,
        min: 0.0,
        max: 1.0,
    });
    tree.push(slider_node);
    tree.push(SemanticNode::new(
        unparented,
        SemanticRole::Label,
        Rect::new(0.0, 56.0, 100.0, 18.0),
    ));

    let snapshot = tree.accessibility_snapshot(Some(button)).expect("snapshot");

    assert_eq!(snapshot.root, Some(root));
    assert_eq!(
        snapshot
            .nodes
            .iter()
            .map(|node| node.id)
            .collect::<Vec<_>>(),
        vec![root, slider, button, unparented]
    );
    assert_eq!(snapshot.focus_order, vec![slider, button]);
    assert_eq!(snapshot.focused, Some(button));
    assert_eq!(snapshot.node(slider).expect("slider").parent, Some(root));
    assert_eq!(
        snapshot.node(button).expect("button").label.as_deref(),
        Some("Run")
    );
    assert_eq!(
        snapshot.node(slider).expect("slider").state.value,
        Some(SemanticValue::Number {
            current: 0.5,
            min: 0.0,
            max: 1.0,
        })
    );
    assert!(
        snapshot
            .node(slider)
            .expect("slider")
            .actions
            .iter()
            .any(|action| action.kind == SemanticActionKind::Increment)
    );
}

#[test]
fn accessibility_snapshot_rejects_invalid_semantic_trees() {
    let root = WidgetId::from_key("root");
    let missing = WidgetId::from_key("missing");
    let mut tree = SemanticTree::new();
    tree.push(SemanticNode::new(root, SemanticRole::Root, Rect::ZERO).with_children([missing]));

    assert_eq!(
        tree.accessibility_snapshot(None).expect_err("error"),
        SemanticTreeError::UnknownChild {
            parent: root,
            child: missing,
        }
    );
}

#[test]
fn accessibility_adapter_synchronizes_snapshots_and_actions() {
    let root = WidgetId::from_key("root");
    let button = WidgetId::from_key("button");
    let mut tree = SemanticTree::new();
    tree.push(SemanticNode::new(root, SemanticRole::Root, Rect::ZERO).with_children([button]));
    tree.push(SemanticNode::new(button, SemanticRole::Button, Rect::ZERO).focusable(true));
    let snapshot = tree.accessibility_snapshot(Some(button)).expect("snapshot");
    let mut adapter = RecordingAdapter::default();

    adapter.synchronize(&snapshot).expect("sync");
    adapter.focus(button).expect("focus");
    adapter
        .perform_action(button, &SemanticActionKind::Invoke)
        .expect("action");

    assert_eq!(adapter.synchronized, vec![snapshot]);
    assert_eq!(adapter.focused, vec![button]);
    assert_eq!(adapter.actions, vec![(button, SemanticActionKind::Invoke)]);
}

#[test]
fn semantic_tree_validation_accepts_empty_trees() {
    let tree = SemanticTree::new();

    assert!(tree.is_empty());
    assert_eq!(tree.len(), 0);
    assert!(tree.validate().is_ok());
}

#[test]
fn semantic_tree_validation_rejects_bad_roots_and_duplicate_nodes() {
    let root = WidgetId::from_key("root");
    let missing = WidgetId::from_key("missing");
    let mut unknown_root = SemanticTree::new();
    unknown_root.push(SemanticNode::new(root, SemanticRole::Root, Rect::ZERO));
    unknown_root.set_root(missing);

    assert_eq!(
        unknown_root.validate().expect_err("error"),
        SemanticTreeError::UnknownRoot { id: missing }
    );

    let mut missing_root = SemanticTree {
        nodes: vec![SemanticNode::new(root, SemanticRole::Root, Rect::ZERO)],
        root: None,
    };
    assert_eq!(
        missing_root.validate().expect_err("error"),
        SemanticTreeError::MissingRoot
    );

    missing_root.root = Some(root);
    missing_root
        .nodes
        .push(SemanticNode::new(root, SemanticRole::Panel, Rect::ZERO));
    assert_eq!(
        missing_root.validate().expect_err("error"),
        SemanticTreeError::DuplicateNodeId { id: root }
    );
}

#[test]
fn semantic_tree_validation_rejects_bad_child_edges() {
    let root = WidgetId::from_key("root");
    let child = WidgetId::from_key("child");
    let missing = WidgetId::from_key("missing");

    let mut unknown_child = SemanticTree::new();
    unknown_child
        .push(SemanticNode::new(root, SemanticRole::Root, Rect::ZERO).with_children([missing]));
    assert_eq!(
        unknown_child.validate().expect_err("error"),
        SemanticTreeError::UnknownChild {
            parent: root,
            child: missing
        }
    );

    let mut duplicate_child = SemanticTree::new();
    duplicate_child.push(
        SemanticNode::new(root, SemanticRole::Root, Rect::ZERO).with_children([child, child]),
    );
    duplicate_child.push(SemanticNode::new(child, SemanticRole::Button, Rect::ZERO));
    assert_eq!(
        duplicate_child.validate().expect_err("error"),
        SemanticTreeError::DuplicateChild {
            parent: root,
            child
        }
    );

    let mut self_child = SemanticTree::new();
    self_child.push(SemanticNode::new(root, SemanticRole::Root, Rect::ZERO).with_children([root]));
    assert_eq!(
        self_child.validate().expect_err("error"),
        SemanticTreeError::SelfChild { id: root }
    );
}

#[test]
fn semantic_tree_validation_rejects_ambiguous_parentage_and_cycles() {
    let root = WidgetId::from_key("root");
    let other = WidgetId::from_key("other");
    let child = WidgetId::from_key("child");

    let mut multiple_parents = SemanticTree::new();
    multiple_parents
        .push(SemanticNode::new(root, SemanticRole::Root, Rect::ZERO).with_children([child]));
    multiple_parents
        .push(SemanticNode::new(other, SemanticRole::Panel, Rect::ZERO).with_children([child]));
    multiple_parents.push(SemanticNode::new(child, SemanticRole::Button, Rect::ZERO));
    assert_eq!(
        multiple_parents.validate().expect_err("error"),
        SemanticTreeError::MultipleParents {
            child,
            first_parent: root,
            second_parent: other
        }
    );

    let mut cycle = SemanticTree::new();
    cycle.push(SemanticNode::new(root, SemanticRole::Root, Rect::ZERO).with_children([child]));
    cycle.push(SemanticNode::new(child, SemanticRole::Panel, Rect::ZERO).with_children([root]));
    assert_eq!(
        cycle.validate().expect_err("error"),
        SemanticTreeError::Cycle { id: root }
    );
}
