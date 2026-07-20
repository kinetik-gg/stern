//! Public-facade contracts for the scenario-gated overlay recovery journey.

use stern::core::{
    ActionSource, FrameOutput, Key, KeyEvent, KeyState, KeyboardInput, Modifiers, Point,
    PointerButtonState, PointerInput, SemanticNode, SemanticRole, UiInput, UiInputEvent, WidgetId,
};
use stern_demo::{DemoApp, DemoScenario, DemoWorkspace, demo_context};

#[test]
fn default_scenario_matches_pinned_base_frame_output() {
    const BASE_FRAME_FINGERPRINT: u64 = 0x366e_65a5_f223_5ad0;
    let mut maintained = DemoApp::new();
    let mut explicit = DemoApp::for_scenario(DemoScenario::Default);

    for _ in 0..2 {
        let maintained = maintained.frame(demo_context(UiInput::default()));
        let explicit = explicit.frame(demo_context(UiInput::default()));
        assert_eq!(maintained, explicit);
        assert_eq!(frame_fingerprint(&maintained), BASE_FRAME_FINGERPRINT);
    }
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
    let settled = app.frame(demo_context(UiInput::default()));
    assert_only_overlay(&settled, OverlayExpectation::None);
    assert!(settled.semantics.get(old_owner).is_none());
    let restored = settled
        .semantics
        .get(graph_action)
        .expect("stable Graph workspace action remains live");
    assert!(restored.focusable && restored.state.focused);
    assert_eq!(app.focused(), Some(graph_action));
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

fn frame_fingerprint(output: &FrameOutput) -> u64 {
    let fields = format!(
        "{:?}",
        (
            &output.primitives,
            &output.semantics,
            &output.repaint,
            &output.actions,
            &output.platform_requests,
            &output.warnings,
        )
    );
    fields.bytes().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3)
    })
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
