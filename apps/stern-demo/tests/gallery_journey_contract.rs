//! Public-facade journey contract for the out-of-the-box component Gallery
//! workspace (issue #916): navigates to the workspace through a real event
//! path and exercises one real control per component family, asserting
//! observable state through the same public `FrameOutput` surface every
//! other journey contract test uses.

use stern::core::{
    FrameOutput, Key, KeyEvent, KeyState, KeyboardInput, Modifiers, Point, PointerButtonState,
    PointerInput, SemanticNode, SemanticRole, UiInput, UiInputEvent,
};
use stern_demo::{DemoApp, DemoWorkspace, demo_context};

#[test]
fn gallery_journey_navigates_and_exercises_one_control_per_family() {
    let mut app = DemoApp::new();
    let initial = app.frame(demo_context(UiInput::default()));
    assert_eq!(app.workspace(), DemoWorkspace::Edit);

    // Buttons family: navigate through the real, action-backed workspace
    // toolbar button (the same control Edit and Graph use to switch here).
    let nav_point = center(&initial, &SemanticRole::IconButton, "Gallery Workspace");
    let gallery = click_and_settle(&mut app, nav_point);
    assert_eq!(app.workspace(), DemoWorkspace::Gallery);
    assert!(has_role(&gallery, &SemanticRole::CheckBox));
    assert!(has_role(&gallery, &SemanticRole::RadioButton));
    assert!(has_role(&gallery, &SemanticRole::Toggle));
    assert!(has_role(&gallery, &SemanticRole::Tab));
    assert!(has_role(&gallery, &SemanticRole::Slider));
    assert!(has_role(&gallery, &SemanticRole::TextField));
    assert!(has_role(&gallery, &SemanticRole::List));

    // Choice family: click the real "Unchecked" checkbox and observe the
    // real toolkit checkbox recipe check it on the settled next frame.
    let checkbox_before = node(&gallery, &SemanticRole::CheckBox, "Unchecked");
    assert_eq!(checkbox_before.state.checked, Some(false));
    let checkbox_point = checkbox_before.bounds.center();
    let after_checkbox = click_and_settle(&mut app, checkbox_point);
    assert_eq!(
        node(&after_checkbox, &SemanticRole::CheckBox, "Unchecked")
            .state
            .checked,
        Some(true)
    );

    // Collections family: click a real live list-specimen row and observe
    // the virtual list's own retained selection state.
    let list_before = node(&after_checkbox, &SemanticRole::ListItem, "Beta");
    assert!(!list_before.state.selected);
    let list_point = list_before.bounds.center();
    let after_list = click_and_settle(&mut app, list_point);
    assert!(
        node(&after_list, &SemanticRole::ListItem, "Beta")
            .state
            .selected
    );

    // Overlays family: click the real Menu trigger and observe the real
    // menu overlay open with its labeled surface.
    let menu_point = center(&after_list, &SemanticRole::Button, "Menu");
    let with_menu = click_and_settle(&mut app, menu_point);
    assert!(has_label(&with_menu, "Gallery menu specimen"));
    let _ = app.frame(demo_context(key(Key::Escape)));
    let settled = app.frame(demo_context(UiInput::default()));
    assert!(!has_label(&settled, "Gallery menu specimen"));

    // Chrome specimens family: activate "Tab B" in the embedded specimen
    // tab strip and observe the specimen status bar reflect it.
    let tab_point = center(&settled, &SemanticRole::Tab, "Tab B");
    let with_tab = click_and_settle(&mut app, tab_point);
    assert!(has_label(&with_tab, "Active Tab B"));

    // Layout-engine seam (RFC 0001 L1): the "Layout engine" strip is
    // composed through `ui.layout` with content-sized builders, and its
    // pointer targets are declared from solved geometry. Clicking the real
    // toggle at its solved rect flips real state; clicking the real button
    // increments a real counter shown through a real label.
    let snap_before = node(&with_tab, &SemanticRole::Toggle, "Snap to grid");
    assert_eq!(snap_before.state.checked, Some(false));
    let after_snap = click_and_settle(&mut app, snap_before.bounds.center());
    assert_eq!(
        node(&after_snap, &SemanticRole::Toggle, "Snap to grid")
            .state
            .checked,
        Some(true)
    );
    assert!(has_label(&after_snap, "Created 0"));
    let new_point = center(&after_snap, &SemanticRole::Button, "New");
    let after_new = click_and_settle(&mut app, new_point);
    assert!(has_label(&after_new, "Created 1"));
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
        .unwrap_or_else(|| panic!("semantic {role:?}: {label}"))
}

fn has_role(output: &FrameOutput, role: &SemanticRole) -> bool {
    output
        .semantics
        .nodes()
        .iter()
        .any(|node| &node.role == role)
}

fn has_label(output: &FrameOutput, label: &str) -> bool {
    output
        .semantics
        .nodes()
        .iter()
        .any(|node| node.label.as_deref() == Some(label))
}

fn click_and_settle(app: &mut DemoApp, point: Point) -> FrameOutput {
    let _ = app.frame(demo_context(pointer(point, true, true, false)));
    let _ = app.frame(demo_context(pointer(point, false, false, true)));
    app.frame(demo_context(UiInput::default()))
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
    let modifiers = Modifiers::default();
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
