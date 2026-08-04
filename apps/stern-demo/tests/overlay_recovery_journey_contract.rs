//! Public-facade contracts for the scenario-gated overlay recovery journey.

use std::collections::BTreeMap;

use stern::core::{
    ActionSource, FrameOutput, Key, KeyEvent, KeyState, KeyboardInput, Modifiers, Point,
    PointerButtonState, PointerInput, Primitive, SemanticNode, SemanticRole, UiInput, UiInputEvent,
    WidgetId,
};
use stern_demo::{DemoApp, DemoScenario, DemoWorkspace, demo_context};

#[test]
fn default_scenario_matches_pinned_base_frame_output() {
    // Structural pin (#916): the opaque frame-fingerprint hash this replaced
    // broke on every paint-recipe change (color/radius/spacing token PRs)
    // even though nothing structural moved. These assertions instead pin
    // primitive counts by type, the exact semantic-node label set, and the
    // workspace-switch row's key geometry — they only break when a control
    // is genuinely added, removed, or reordered.
    let mut maintained = DemoApp::new();
    let mut explicit = DemoApp::for_scenario(DemoScenario::Default);

    for _ in 0..2 {
        let maintained = maintained.frame(demo_context(UiInput::default()));
        let explicit = explicit.frame(demo_context(UiInput::default()));
        assert_eq!(maintained, explicit);
        assert_default_base_frame_structure(&maintained);
    }
}

/// Asserts the deterministic Default-scenario base frame's primitive-type
/// counts, exact semantic-node label multiset, and workspace-switch row
/// geometry (shared with `graph_journey_contract.rs`).
#[allow(clippy::too_many_lines)] // structural inventory reads better linear
fn assert_default_base_frame_structure(output: &FrameOutput) {
    let expected_counts: BTreeMap<&'static str, usize> = BTreeMap::from([
        ("clip_begin", 25),
        ("clip_end", 25),
        ("icon", 7),
        ("line", 1),
        ("rect", 123),
        ("text", 81),
        ("texture", 1),
        ("transform_begin", 2),
        ("transform_end", 2),
    ]);
    assert_eq!(primitive_counts(output), expected_counts);

    let expected_labels: Vec<&'static str> = vec![
        "Application menu",
        "Application status",
        "Application toolbar",
        "Applied revision 0",
        "Apply Shared State",
        "Assets",
        "Assets",
        "Assets",
        "Assets",
        "Backdrop",
        "Background jobs",
        "Bloom",
        "Character",
        "Clouds",
        "Color",
        "Credits",
        "Document tabs",
        "Edit Workspace",
        "Edit Workspace",
        "Editor dock",
        "Effects",
        "Fill color",
        "Foreground",
        "Frame tabs",
        "Frame tabs",
        "Frame tabs",
        "Frame tabs",
        "Gallery Workspace",
        "Gallery Workspace",
        "Grade",
        "Gradient editor",
        "Gradient stop 1",
        "Gradient stop 2",
        "Graph Workspace",
        "Graph Workspace",
        "Hero clip",
        "Inspector",
        "Inspector",
        "Inspector",
        "Kind",
        "Lighting",
        "Mountains",
        "Name",
        "Notifications",
        "Opacity",
        "Preview 40%",
        "Preview render",
        "Property grid",
        "Raster layer",
        "Reset Kind to default",
        "Reset Name to default",
        "Reset Opacity to default",
        "Reset Visible to default",
        "Save Color Style",
        "Select Tool",
        "Select Tool",
        "Selection",
        "Subtitle",
        "Text field",
        "Text field",
        "Timeline",
        "Timeline",
        "Timeline",
        "Timeline",
        "Title",
        "Transform Tool",
        "Transform Tool",
        "Video",
        "Viewport",
        "Viewport",
        "Viewport",
        "Viewport",
        "Visible",
        "Visible",
        "Workspace",
        "sRGB · Reverse",
    ];
    assert_eq!(
        semantic_labels(output),
        expected_labels
            .into_iter()
            .map(str::to_owned)
            .collect::<Vec<_>>()
    );

    // Key geometry: the five workspace/shared-action toolbar buttons sit in
    // one row, left to right in registry order, sharing height and baseline.
    let edit = semantic_center_rect(output, "Edit Workspace");
    let graph = semantic_center_rect(output, "Graph Workspace");
    let gallery = semantic_center_rect(output, "Gallery Workspace");
    let apply = semantic_center_rect(output, "Apply Shared State");
    let save = semantic_center_rect(output, "Save Color Style");
    for rect in [&graph, &gallery, &apply, &save] {
        assert_eq!(rect.y.to_bits(), edit.y.to_bits());
        assert_eq!(rect.height.to_bits(), edit.height.to_bits());
    }
    assert!(edit.x < graph.x);
    assert!(graph.x < gallery.x);
    assert!(gallery.x < apply.x);
    assert!(apply.x < save.x);
}

fn primitive_kind(primitive: &Primitive) -> &'static str {
    match primitive {
        Primitive::Rect(_) => "rect",
        Primitive::Line(_) => "line",
        Primitive::Shadow(_) => "shadow",
        Primitive::Path(_) => "path",
        Primitive::Icon(_) => "icon",
        Primitive::Text(_) => "text",
        Primitive::Image(_) => "image",
        Primitive::Texture(_) => "texture",
        Primitive::ClipBegin { .. } => "clip_begin",
        Primitive::ClipEnd { .. } => "clip_end",
        Primitive::LayerBegin { .. } => "layer_begin",
        Primitive::LayerEnd { .. } => "layer_end",
        Primitive::TransformBegin(_) => "transform_begin",
        Primitive::TransformEnd => "transform_end",
    }
}

fn primitive_counts(output: &FrameOutput) -> BTreeMap<&'static str, usize> {
    let mut counts = BTreeMap::new();
    for primitive in &output.primitives {
        *counts.entry(primitive_kind(primitive)).or_insert(0) += 1;
    }
    counts
}

fn semantic_labels(output: &FrameOutput) -> Vec<String> {
    let mut labels = output
        .semantics
        .nodes()
        .iter()
        .filter_map(|node| node.label.clone())
        .collect::<Vec<_>>();
    labels.sort();
    labels
}

fn semantic_center_rect(output: &FrameOutput, label: &str) -> stern::core::Rect {
    output
        .semantics
        .nodes()
        .iter()
        .find(|node| node.label.as_deref() == Some(label))
        .unwrap_or_else(|| panic!("semantic control: {label}"))
        .bounds
}

#[test]
fn public_tooltip_uses_the_exclusive_shared_overlay_route() {
    let mut app = DemoApp::for_scenario(DemoScenario::OverlayRecoveryJourney);
    let initial = app.frame(demo_context(UiInput::default()));
    let help = center(&initial, &SemanticRole::Button, "Overlay help");

    let tooltip = app.frame(demo_context(hover(help)));
    assert_only_overlay(&tooltip, OverlayExpectation::Tooltip);

    let clear = app.frame(demo_context(hover(Point::new(8.0, 440.0))));
    assert_only_overlay(&clear, OverlayExpectation::None);

    let menu = open_workspace_menu(&mut app, &clear);
    assert_only_overlay(&menu, OverlayExpectation::Menu);
    let _ = app.frame(demo_context(key(Key::Escape)));
    let clear = app.frame(demo_context(UiInput::default()));
    assert_only_overlay(&clear, OverlayExpectation::None);

    let palette = app.frame(demo_context(key_with_modifiers(
        Key::Character("p".to_owned()),
        Modifiers::new(true, true, false, false),
    )));
    assert_only_overlay(&palette, OverlayExpectation::Palette);
    let _ = app.frame(demo_context(key(Key::Escape)));
    let clear = app.frame(demo_context(UiInput::default()));

    let failed_action = invoke_workspace_action_from(&mut app, &clear, "Save Color Style");
    assert_eq!(action_count(&failed_action, "color-style.save"), 1);
    let popover = app.frame(demo_context(UiInput::default()));
    assert_only_overlay(&popover, OverlayExpectation::Popover);
    dismiss_outside(&mut app, Point::new(8.0, 440.0));
    let clear = app.frame(demo_context(UiInput::default()));

    let recovered_action = invoke_workspace_action_from(&mut app, &clear, "Save Color Style");
    assert_eq!(action_count(&recovered_action, "color-style.save"), 1);
    let modal = app.frame(demo_context(UiInput::default()));
    assert_only_overlay(&modal, OverlayExpectation::Modal);
    let _ = app.frame(demo_context(key(Key::Escape)));
    let clear = app.frame(demo_context(UiInput::default()));
    assert_only_overlay(&clear, OverlayExpectation::None);

    let tooltip = app.frame(demo_context(hover(help)));
    assert_only_overlay(&tooltip, OverlayExpectation::Tooltip);
}

#[test]
fn edit_owner_removal_closes_menu_and_restores_live_graph_focus() {
    let mut app = DemoApp::for_scenario(DemoScenario::OverlayRecoveryJourney);
    let initial = app.frame(demo_context(UiInput::default()));
    let focused = click(&mut app, &initial, &SemanticRole::ListItem, "Backdrop");
    let old_owner = app.focused().expect("Edit asset owns focus");
    assert!(focused.semantics.get(old_owner).is_some());

    let menu = open_workspace_menu(&mut app, &focused);
    assert_only_overlay(&menu, OverlayExpectation::Menu);
    let transition = click(&mut app, &menu, &SemanticRole::MenuItem, "Graph Workspace");
    assert_eq!(action_count(&transition, "workspace.graph"), 1);
    assert!(transition.actions.clone().drain().any(|invocation| {
        invocation.action_id.as_str() == "workspace.graph"
            && invocation.source == ActionSource::Menu
    }));
    assert_eq!(app.workspace(), DemoWorkspace::Graph);

    let graph_action = WidgetId::from_key("root").child("workspace.graph");
    assert_eq!(app.focused(), Some(graph_action));
    // The activation focus target is synthetic (set directly by `DemoApp::dispatch`)
    // and is not re-declared by any real Graph-workspace widget on a settled frame
    // with no input, so it is naturally cleared rather than retargeting a stale
    // identity. The behavior this test guards is that the old Edit-owned focus
    // target is dropped, not resurrected, once its dock content unmounts.
    let settled = app.frame(demo_context(UiInput::default()));
    assert_only_overlay(&settled, OverlayExpectation::None);
    assert!(settled.semantics.get(old_owner).is_none());
    assert!(settled.semantics.get(graph_action).is_none());
    assert_eq!(app.focused(), None);
}

#[derive(Clone, Copy)]
enum OverlayExpectation {
    None,
    Tooltip,
    Menu,
    Palette,
    Popover,
    Modal,
}

fn assert_only_overlay(output: &FrameOutput, expected: OverlayExpectation) {
    let observed = [
        has_label(output, "Overlay help tooltip"),
        has_label(output, "Workspace commands"),
        has_role(output, &SemanticRole::SearchField),
        has_label(output, "Color recovery hint"),
        has_label(output, "Color style recovered"),
    ];
    let expected = match expected {
        OverlayExpectation::None => [false, false, false, false, false],
        OverlayExpectation::Tooltip => [true, false, false, false, false],
        OverlayExpectation::Menu => [false, true, false, false, false],
        OverlayExpectation::Palette => [false, false, true, false, false],
        OverlayExpectation::Popover => [false, false, false, true, false],
        OverlayExpectation::Modal => [false, false, false, false, true],
    };
    assert_eq!(observed, expected);
}

fn open_workspace_menu(app: &mut DemoApp, current: &FrameOutput) -> FrameOutput {
    let _ = click(app, current, &SemanticRole::MenuItem, "Workspace");
    app.frame(demo_context(UiInput::default()))
}

fn invoke_workspace_action_from(
    app: &mut DemoApp,
    current: &FrameOutput,
    label: &str,
) -> FrameOutput {
    let menu = open_workspace_menu(app, current);
    click(app, &menu, &SemanticRole::MenuItem, label)
}

fn dismiss_outside(app: &mut DemoApp, point: Point) {
    let _ = app.frame(demo_context(pointer(point, true, true, false)));
    let _ = app.frame(demo_context(pointer(point, false, false, true)));
}

fn center(output: &FrameOutput, role: &SemanticRole, label: &str) -> Point {
    node(output, role, label).bounds.center()
}

fn node<'a>(output: &'a FrameOutput, role: &SemanticRole, label: &str) -> &'a SemanticNode {
    output
        .semantics
        .nodes()
        .iter()
        .find(|node| &node.role == role && node.label.as_deref() == Some(label))
        .expect("semantic node")
}

fn has_label(output: &FrameOutput, label: &str) -> bool {
    output
        .semantics
        .nodes()
        .iter()
        .any(|node| node.label.as_deref() == Some(label))
}

fn has_role(output: &FrameOutput, role: &SemanticRole) -> bool {
    output
        .semantics
        .nodes()
        .iter()
        .any(|node| &node.role == role)
}

fn action_count(output: &FrameOutput, id: &str) -> usize {
    output
        .actions
        .clone()
        .drain()
        .filter(|invocation| invocation.action_id.as_str() == id)
        .count()
}

fn click(app: &mut DemoApp, output: &FrameOutput, role: &SemanticRole, label: &str) -> FrameOutput {
    let point = center(output, role, label);
    let _ = app.frame(demo_context(pointer(point, true, true, false)));
    app.frame(demo_context(pointer(point, false, false, true)))
}

fn hover(point: Point) -> UiInput {
    UiInput {
        pointer: PointerInput {
            position: Some(point),
            ..PointerInput::default()
        },
        ..UiInput::default()
    }
}

fn pointer(point: Point, down: bool, pressed: bool, released: bool) -> UiInput {
    UiInput {
        pointer: PointerInput {
            position: Some(point),
            primary: PointerButtonState::new(down, pressed, released),
            ..PointerInput::default()
        },
        ..UiInput::default()
    }
}

fn key(key: Key) -> UiInput {
    key_with_modifiers(key, Modifiers::default())
}

fn key_with_modifiers(key: Key, modifiers: Modifiers) -> UiInput {
    let event = KeyEvent::new(key, KeyState::Pressed, modifiers, false);
    UiInput {
        keyboard: KeyboardInput {
            modifiers,
            events: vec![event.clone()],
        },
        events: vec![UiInputEvent::Key(event)],
        ..UiInput::default()
    }
}
