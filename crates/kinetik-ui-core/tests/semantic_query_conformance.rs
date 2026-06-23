//! Semantic accessibility query conformance coverage.

#![allow(clippy::float_cmp)]

use kinetik_ui_core::{
    Brush, Color, CornerRadius, FrameWarning, Primitive, Rect, RectPrimitive, SemanticAction,
    SemanticActionKind, SemanticNode, SemanticRole, SemanticState, SemanticTree, SemanticTreeError,
    SemanticValue, UiTestHarness, WidgetId,
};

#[test]
fn semantic_query_finds_button_by_role_and_label() {
    let root = WidgetId::from_key("root");
    let run = WidgetId::from_key("run");
    let cancel = WidgetId::from_key("cancel");
    let mut tree = SemanticTree::new();
    tree.push(SemanticNode::new(root, SemanticRole::Root, Rect::ZERO).with_children([run, cancel]));
    tree.push(
        SemanticNode::new(run, SemanticRole::Button, Rect::new(4.0, 8.0, 80.0, 24.0))
            .focusable(true)
            .with_label("Run"),
    );
    tree.push(
        SemanticNode::new(
            cancel,
            SemanticRole::Button,
            Rect::new(88.0, 8.0, 80.0, 24.0),
        )
        .focusable(true)
        .with_label("Cancel"),
    );

    let snapshot = tree.accessibility_snapshot(None).expect("valid snapshot");

    assert_eq!(
        snapshot
            .nodes_by_role(&SemanticRole::Button)
            .map(|node| node.id)
            .collect::<Vec<_>>(),
        vec![run, cancel]
    );
    assert_eq!(snapshot.find_by_label("Run").map(|node| node.id), Some(run));
    assert_eq!(
        snapshot
            .find_by_role_and_label(&SemanticRole::Button, "Run")
            .map(|node| node.id),
        Some(run)
    );
    assert_eq!(snapshot.find_by_label("Missing"), None);
    assert_eq!(
        snapshot
            .find_by_role_and_label(&SemanticRole::Slider, "Run")
            .map(|node| node.id),
        None
    );
    assert!(snapshot.nodes_by_label("Missing").next().is_none());
    assert!(
        snapshot
            .nodes_by_role(&SemanticRole::Slider)
            .next()
            .is_none()
    );
}

#[test]
fn semantic_query_finds_slider_by_numeric_value_and_exposes_node_fields() {
    let root = WidgetId::from_key("root");
    let slider = WidgetId::from_key("opacity");
    let value = SemanticValue::Number {
        current: 0.5,
        min: 0.0,
        max: 1.0,
    };
    let mut slider_node = SemanticNode::new(
        slider,
        SemanticRole::Slider,
        Rect::new(12.0, 40.0, 180.0, 18.0),
    )
    .focusable(true)
    .with_label("Opacity")
    .with_action(SemanticAction::new(
        SemanticActionKind::Increment,
        "Increase",
    ))
    .with_action(SemanticAction::new(
        SemanticActionKind::Decrement,
        "Decrease",
    ));
    slider_node.state = SemanticState {
        focused: true,
        selected: true,
        value: Some(value.clone()),
        ..SemanticState::default()
    };

    let mut tree = SemanticTree::new();
    tree.push(SemanticNode::new(root, SemanticRole::Root, Rect::ZERO).with_children([slider]));
    tree.push(slider_node);

    let snapshot = tree
        .accessibility_snapshot(Some(slider))
        .expect("valid snapshot");
    let matched = snapshot.find_by_value(&value).expect("slider value");

    assert_eq!(matched.id, slider);
    assert_eq!(matched.parent, Some(root));
    assert_eq!(matched.children, Vec::<WidgetId>::new());
    assert_eq!(matched.bounds, Rect::new(12.0, 40.0, 180.0, 18.0));
    assert_eq!(matched.label.as_deref(), Some("Opacity"));
    assert_eq!(matched.role, SemanticRole::Slider);
    assert_eq!(matched.state.value, Some(value.clone()));
    assert!(matched.focusable);
    assert!(!matched.state.disabled);
    assert!(matched.state.focused);
    assert!(matched.state.selected);
    assert!(
        matched
            .actions
            .iter()
            .any(|action| action.kind == SemanticActionKind::Focus)
    );
    assert!(
        matched
            .actions
            .iter()
            .any(|action| action.kind == SemanticActionKind::Increment)
    );
    assert_eq!(
        snapshot
            .nodes_by_value(&value)
            .map(|node| node.id)
            .collect::<Vec<_>>(),
        vec![slider]
    );
    assert_eq!(
        snapshot.find_by_value(&SemanticValue::Text("0.5".to_owned())),
        None
    );
}

#[test]
fn semantic_query_preserves_traversal_and_filters_focus_order() {
    let root = WidgetId::from_key("root");
    let first = WidgetId::from_key("first");
    let second = WidgetId::from_key("second");
    let disabled = WidgetId::from_key("disabled");
    let mut disabled_node = SemanticNode::new(
        disabled,
        SemanticRole::Button,
        Rect::new(0.0, 48.0, 80.0, 24.0),
    )
    .focusable(true)
    .with_label("Disabled");
    disabled_node.state.disabled = true;

    let mut tree = SemanticTree::new();
    tree.push(
        SemanticNode::new(root, SemanticRole::Panel, Rect::new(0.0, 0.0, 200.0, 100.0))
            .with_children([second, first, disabled]),
    );
    tree.push(
        SemanticNode::new(first, SemanticRole::Button, Rect::new(0.0, 8.0, 80.0, 24.0))
            .focusable(true)
            .with_label("First"),
    );
    tree.push(
        SemanticNode::new(
            second,
            SemanticRole::Button,
            Rect::new(0.0, 32.0, 80.0, 24.0),
        )
        .focusable(true)
        .with_label("Second"),
    );
    tree.push(disabled_node);

    let snapshot = tree
        .accessibility_snapshot(Some(disabled))
        .expect("valid snapshot");

    assert_eq!(
        snapshot
            .nodes
            .iter()
            .map(|node| node.id)
            .collect::<Vec<_>>(),
        vec![root, second, first, disabled]
    );
    assert_eq!(snapshot.focus_order, vec![second, first]);
    assert_eq!(snapshot.focused, None);
    assert_eq!(snapshot.node(second).expect("second").parent, Some(root));
    assert_eq!(
        snapshot.node(root).expect("root").children,
        vec![second, first, disabled]
    );
    assert!(snapshot.node(disabled).expect("disabled").state.disabled);
}

#[test]
fn semantic_query_invalid_semantic_tree_warns_at_frame_end() {
    let mut harness = UiTestHarness::new();
    let root = WidgetId::from_key("root");
    let missing = WidgetId::from_key("missing");

    let ((), output) = harness.run_frame(|ui| {
        ui.push_semantic_node(
            SemanticNode::new(root, SemanticRole::Root, Rect::ZERO).with_children([missing]),
        );
    });

    assert_eq!(
        output.warnings,
        vec![FrameWarning::InvalidSemanticTree {
            error: SemanticTreeError::UnknownChild {
                parent: root,
                child: missing,
            }
        }]
    );
    assert_eq!(
        output.accessibility_snapshot(None).expect_err("invalid"),
        SemanticTreeError::UnknownChild {
            parent: root,
            child: missing,
        }
    );
}

#[test]
fn semantic_query_validation_rejects_structural_failures() {
    let root = WidgetId::from_key("root");
    let other = WidgetId::from_key("other");
    let child = WidgetId::from_key("child");
    let missing = WidgetId::from_key("missing");

    let mut duplicate = SemanticTree::new();
    duplicate.push(SemanticNode::new(root, SemanticRole::Root, Rect::ZERO));
    duplicate.push(SemanticNode::new(root, SemanticRole::Button, Rect::ZERO));
    assert_eq!(
        duplicate.validate().expect_err("duplicate"),
        SemanticTreeError::DuplicateNodeId { id: root }
    );

    let mut unknown_root = SemanticTree::new();
    unknown_root.push(SemanticNode::new(root, SemanticRole::Root, Rect::ZERO));
    unknown_root.set_root(missing);
    assert_eq!(
        unknown_root.validate().expect_err("unknown root"),
        SemanticTreeError::UnknownRoot { id: missing }
    );

    let mut unknown_child = SemanticTree::new();
    unknown_child
        .push(SemanticNode::new(root, SemanticRole::Root, Rect::ZERO).with_children([missing]));
    assert_eq!(
        unknown_child.validate().expect_err("unknown child"),
        SemanticTreeError::UnknownChild {
            parent: root,
            child: missing,
        }
    );

    let mut multiple_parents = SemanticTree::new();
    multiple_parents
        .push(SemanticNode::new(root, SemanticRole::Root, Rect::ZERO).with_children([child]));
    multiple_parents
        .push(SemanticNode::new(other, SemanticRole::Panel, Rect::ZERO).with_children([child]));
    multiple_parents.push(SemanticNode::new(child, SemanticRole::Button, Rect::ZERO));
    assert_eq!(
        multiple_parents.validate().expect_err("multiple parents"),
        SemanticTreeError::MultipleParents {
            child,
            first_parent: root,
            second_parent: other,
        }
    );

    let mut cycle = SemanticTree::new();
    cycle.push(SemanticNode::new(root, SemanticRole::Root, Rect::ZERO).with_children([child]));
    cycle.push(SemanticNode::new(child, SemanticRole::Panel, Rect::ZERO).with_children([root]));
    assert_eq!(
        cycle.validate().expect_err("cycle"),
        SemanticTreeError::Cycle { id: root }
    );
}

#[test]
fn semantic_query_is_independent_from_render_primitives_in_harness_output() {
    let mut harness = UiTestHarness::new();
    let root = WidgetId::from_key("root");
    let run = WidgetId::from_key("run");

    let ((), output) = harness.run_frame(|ui| {
        ui.push_primitive(Primitive::Rect(RectPrimitive {
            rect: Rect::new(4.0, 4.0, 12.0, 12.0),
            fill: Some(Brush::Solid(Color::WHITE)),
            stroke: None,
            radius: CornerRadius::all(1.0),
        }));
        ui.push_semantic_node(
            SemanticNode::new(root, SemanticRole::Root, Rect::ZERO).with_children([run]),
        );
        ui.push_primitive(Primitive::Rect(RectPrimitive {
            rect: Rect::new(24.0, 4.0, 12.0, 12.0),
            fill: Some(Brush::Solid(Color::BLACK)),
            stroke: None,
            radius: CornerRadius::all(1.0),
        }));
        ui.push_semantic_node(
            SemanticNode::new(run, SemanticRole::Button, Rect::new(40.0, 8.0, 80.0, 24.0))
                .focusable(true)
                .with_label("Run"),
        );
    });

    assert_eq!(output.primitives.len(), 2);
    let snapshot = output
        .accessibility_snapshot(Some(run))
        .expect("snapshot from harness output");
    let button = snapshot
        .find_by_role_and_label(&SemanticRole::Button, "Run")
        .expect("button");

    assert_eq!(button.id, run);
    assert_eq!(button.bounds, Rect::new(40.0, 8.0, 80.0, 24.0));
    assert_eq!(snapshot.focus_order, vec![run]);
    assert_eq!(snapshot.focused, Some(run));
}

#[test]
fn semantic_query_rejects_invalid_snapshot_source_before_queries() {
    let root = WidgetId::from_key("root");
    let button = WidgetId::from_key("button");
    let mut tree = SemanticTree::new();
    tree.push(SemanticNode::new(root, SemanticRole::Root, Rect::ZERO).with_children([button]));
    tree.push(
        SemanticNode::new(button, SemanticRole::Button, Rect::ZERO)
            .focusable(true)
            .with_label("Run"),
    );
    tree.push(SemanticNode::new(button, SemanticRole::Button, Rect::ZERO).with_label("Duplicate"));

    assert_eq!(
        tree.accessibility_snapshot(Some(button))
            .expect_err("snapshot must fail before queries"),
        SemanticTreeError::DuplicateNodeId { id: button }
    );
}
