//! Deterministic conformance for noninteractive menu rows.

use std::{cell::Cell, time::Duration};

use stern_core::{
    ActionContext, ActionDescriptor, ActionId, ActionInvocation, ActionSource, FrameContext, Key,
    KeyEvent, KeyState, KeyboardInput, Modifiers, PhysicalSize, Point, PointerButtonState,
    PointerInput, PointerOrder, PointerRoute, Primitive, Rect, SemanticActionKind, SemanticRole,
    Shortcut, ShortcutLabelLocalizer, ShortcutLabelToken, ShortcutPlatform, Size, TimeInfo,
    UiInput, UiMemory, ViewportInfo, WidgetId, default_dark_theme,
};
use stern_widgets::overlays::{OverlayNavigationInput, TypeaheadBuffer};
use stern_widgets::{
    Menu, MenuItem, MenuOverlay, OverlayEntry, OverlayId, OverlayKind, OverlayScene,
    OverlaySceneIntent, OverlaySceneOutput, OverlaySceneSurface, Ui,
};

const OVERLAY_ID: OverlayId = OverlayId::from_raw(73);
const SURFACE_RECT: Rect = Rect::new(20.0, 20.0, 320.0, 260.0);

fn action(id: &str, label: &str) -> ActionDescriptor {
    ActionDescriptor::new(id, label)
}

fn mixed_menu() -> Menu {
    let mut menu = Menu::new();
    menu.push(MenuItem::Label("Heading Match".to_owned()));
    menu.push(MenuItem::Action(action("file.open", "Open")));
    menu.push(MenuItem::Separator);
    let mut disabled = action("file.disabled", "Disabled Match");
    disabled.state.enabled = false;
    menu.push(MenuItem::Action(disabled));
    menu.push(MenuItem::Action(action("file.save", "Save")));
    menu.push(MenuItem::Label("Section Match".to_owned()));
    menu.push(MenuItem::Separator);
    let mut hidden = action("file.hidden", "Hidden Match");
    hidden.state.visible = false;
    menu.push(MenuItem::Action(hidden));
    menu.push(MenuItem::Action(action("file.share", "Share")));
    menu
}

fn menu_scene(menu: Menu) -> OverlayScene {
    let mut scene = OverlayScene::new();
    scene.push(OverlaySceneSurface::menu(
        "File commands",
        MenuOverlay::new(
            OverlayEntry::new(OVERLAY_ID, OverlayKind::Menu, SURFACE_RECT),
            menu,
            ActionSource::Menu,
            ActionContext::Frame(WidgetId::from_key("document:passive-order")),
        ),
    ));
    scene
}

fn context(input: UiInput) -> FrameContext {
    FrameContext::new(
        ViewportInfo::new(
            Size::new(640.0, 480.0),
            PhysicalSize::new(640, 480),
            stern_core::ScaleFactor::ONE,
        ),
        input,
        TimeInfo::new(Duration::from_millis(500), Duration::from_millis(16), 1),
    )
}

fn pointer_input(position: Point, pressed: bool) -> UiInput {
    UiInput {
        pointer: PointerInput {
            position: Some(position),
            primary: if pressed {
                PointerButtonState::new(true, true, false)
            } else {
                PointerButtonState::new(false, false, true)
            },
            ..PointerInput::default()
        },
        ..UiInput::default()
    }
}

fn key_sequence(keys: &[Key]) -> UiInput {
    UiInput {
        keyboard: KeyboardInput {
            events: keys
                .iter()
                .cloned()
                .map(|key| KeyEvent::new(key, KeyState::Pressed, Modifiers::default(), false))
                .collect(),
            ..KeyboardInput::default()
        },
        ..UiInput::default()
    }
}

fn rid(action_id: &str) -> WidgetId {
    WidgetId::from_raw(OVERLAY_ID.raw())
        .child("overlay-scene")
        .child(("overlay-action", action_id))
}

struct RecordingLocalizer(Cell<usize>);

impl ShortcutLabelLocalizer for RecordingLocalizer {
    fn token_label(
        &self,
        _platform: ShortcutPlatform,
        _token: ShortcutLabelToken<'_>,
    ) -> Option<String> {
        self.0.set(self.0.get() + 1);
        Some("localized".to_owned())
    }
    #[allow(clippy::unnecessary_literal_bound)]
    fn separator(&self, _platform: ShortcutPlatform) -> &str {
        self.0.set(self.0.get() + 1);
        "+"
    }
}

fn run(
    scene: &mut OverlayScene,
    memory: &mut UiMemory,
    input: UiInput,
    presentation: Option<(ShortcutPlatform, &dyn ShortcutLabelLocalizer)>,
) -> (PointerRoute, OverlaySceneOutput, stern_core::FrameOutput) {
    let theme = default_dark_theme();
    let mut ui = Ui::begin_frame(context(input), memory, &theme);
    ui.resolve_pointer_targets(|plan| {
        scene.declare_pointer_targets(plan, PointerOrder::new(100));
    })
    .expect("valid pointer plan");
    let route = ui.memory().pointer_route();
    let output = match presentation {
        Some((platform, localizer)) => {
            ui.overlay_scene_with_menu_presentation(scene, platform, localizer)
        }
        None => ui.overlay_scene(scene),
    };
    (route, output, ui.finish_output())
}

fn passives(frame: &stern_core::FrameOutput) -> Vec<(WidgetId, SemanticRole, Rect)> {
    let surface = frame
        .semantics
        .get(WidgetId::from_raw(OVERLAY_ID.raw()))
        .expect("menu surface");
    surface
        .children
        .iter()
        .filter_map(|id| {
            let node = frame.semantics.get(*id).expect("surface child");
            (node.role == SemanticRole::Label
                || node.role == SemanticRole::Custom("separator".to_owned()))
            .then(|| (node.id, node.role.clone(), node.bounds))
        })
        .collect()
}

fn assert_passive(
    route: PointerRoute,
    output: &OverlaySceneOutput,
    frame: &stern_core::FrameOutput,
) {
    let passive = passives(frame);
    assert!(!passive.is_empty());
    for (id, _, _) in &passive {
        let node = frame.semantics.get(*id).expect("passive semantics");
        assert!(!node.focusable && node.actions.is_empty());
        assert!(!node.state.focused && !node.state.pressed);
        assert!(output.responses.iter().all(|response| response.id != *id));
        assert_ne!(route, PointerRoute::Target(*id));
    }
    assert!(output.intents.is_empty() && frame.actions.is_empty());
}

#[test]
fn passive_rows_are_excluded_from_navigation_and_typeahead_order() {
    let mut menu = mixed_menu();
    for (input, expected, visible_index) in [
        (OverlayNavigationInput::Next, "file.open", 1),
        (OverlayNavigationInput::Next, "file.save", 4),
        (OverlayNavigationInput::Next, "file.share", 7),
        (OverlayNavigationInput::Next, "file.open", 1),
        (OverlayNavigationInput::Previous, "file.share", 7),
        (OverlayNavigationInput::First, "file.open", 1),
        (OverlayNavigationInput::Last, "file.share", 7),
    ] {
        assert_eq!(menu.move_highlight(input), Some(ActionId::new(expected)));
        assert_eq!(menu.highlighted_visible_index(), Some(visible_index));
    }
    for prefix in ["heading", "section", "separator", "disabled", "hidden"] {
        menu.clear_highlight();
        let mut typeahead = TypeaheadBuffer::default();
        assert_eq!(menu.typeahead(&mut typeahead, prefix, 0), None);
        assert_eq!(menu.highlighted_action_id(), None);
    }
    let mut typeahead = TypeaheadBuffer::default();
    assert_eq!(
        menu.typeahead(&mut typeahead, "s", 0),
        Some(ActionId::new("file.save"))
    );
    assert_eq!(menu.highlighted_visible_index(), Some(4));
    assert_eq!(
        menu.typeahead(&mut typeahead, "s", 100),
        Some(ActionId::new("file.share"))
    );
    assert_eq!(menu.highlighted_visible_index(), Some(7));
}

#[test]
fn passive_rows_cannot_activate_or_enqueue_actions() {
    let mut scene = menu_scene(mixed_menu());
    let mut memory = UiMemory::new();
    let (_, _, initial) = run(&mut scene, &mut memory, UiInput::default(), None);
    let passive_rows = passives(&initial);
    assert_eq!(passive_rows.len(), 4);
    let focused = rid("file.save");
    memory.focus(focused);
    for (passive_id, _, bounds) in passive_rows {
        for pressed in [true, false] {
            let (_, output, frame) = run(
                &mut scene,
                &mut memory,
                pointer_input(bounds.center(), pressed),
                None,
            );
            assert_eq!(memory.focused(), Some(focused));
            assert!(output.intents.is_empty());
            assert!(frame.actions.is_empty());
            assert!(output.responses.iter().all(|response| {
                !response.state.hovered
                    && !response.state.pressed
                    && !response.clicked
                    && response.id != passive_id
            }));
        }
    }
    let (_, output, mut frame) = run(
        &mut scene,
        &mut memory,
        key_sequence(&[Key::End, Key::Enter]),
        None,
    );
    let expected = ActionInvocation::new(
        ActionId::new("file.share"),
        ActionSource::Menu,
        ActionContext::Frame(WidgetId::from_key("document:passive-order")),
    );
    assert_eq!(
        output.intents,
        vec![OverlaySceneIntent::Action(expected.clone())]
    );
    assert_eq!(frame.actions.drain().collect::<Vec<_>>(), vec![expected]);
}

#[test]
fn passive_rows_have_no_pointer_targets_responses_or_semantic_actions() {
    let mut scene = menu_scene(mixed_menu());
    let (_, output, frame) = run(&mut scene, &mut UiMemory::new(), UiInput::default(), None);
    assert_passive(PointerRoute::Blocked, &output, &frame);
    let surface = frame
        .semantics
        .get(WidgetId::from_raw(73))
        .expect("surface");
    assert_eq!(surface.children.len(), 8);
    let read = surface
        .children
        .iter()
        .map(|id| {
            let node = frame.semantics.get(*id).expect("child");
            format!("{:?}:{}", node.role, node.label.as_deref().unwrap_or(""))
        })
        .collect::<Vec<_>>();
    assert_eq!(
        read,
        [
            "Label:Heading Match",
            "MenuItem:Open",
            "Custom(\"separator\"):Separator",
            "MenuItem:Disabled Match",
            "MenuItem:Save",
            "Label:Section Match",
            "Custom(\"separator\"):Separator",
            "MenuItem:Share"
        ]
    );
    let enabled = [rid("file.open"), rid("file.save"), rid("file.share")];
    for child in &surface.children {
        let node = frame.semantics.get(*child).expect("child semantics");
        assert_eq!(node.focusable, enabled.contains(child));
        let operates = node.actions.iter().any(|action| {
            matches!(
                action.kind,
                SemanticActionKind::Invoke | SemanticActionKind::Open
            )
        });
        assert_eq!(operates, enabled.contains(child));
        let (route, probe, _) = run(
            &mut scene,
            &mut UiMemory::new(),
            pointer_input(node.bounds.center(), true),
            None,
        );
        if enabled.contains(child) {
            assert_eq!(route, PointerRoute::Target(*child));
            assert!(probe.responses.iter().any(|response| response.id == *child));
        } else {
            assert_eq!(route, PointerRoute::Blocked);
            assert!(probe.responses.iter().all(|response| response.id != *child));
        }
    }
    assert_eq!(
        frame
            .semantics
            .focus_order()
            .into_iter()
            .filter(|id| surface.children.contains(id))
            .collect::<Vec<_>>(),
        enabled
    );
}

#[test]
fn legacy_and_presented_paths_preserve_passive_scene_isolation() {
    let shortcut = Shortcut::new(
        Modifiers::new(true, false, false, false),
        Key::Character("r".to_owned()),
    );
    let mut submenu = action("file.recent", "Recent");
    submenu.shortcut = Some(shortcut);
    let mut menu = Menu::new();
    menu.push(MenuItem::Label("Projects".to_owned()));
    menu.push_submenu(submenu, Menu::from_actions([action("file.child", "Child")]));
    menu.push(MenuItem::Separator);
    menu.push(MenuItem::Action(action("file.quit", "Quit")));
    let mut legacy_scene = menu_scene(menu.clone());
    let mut presented_scene = menu_scene(menu);
    let mut legacy_memory = UiMemory::new();
    let mut presented_memory = UiMemory::new();
    let localizer = RecordingLocalizer(Cell::new(0));
    let (legacy_route, legacy_output, legacy_frame) = run(
        &mut legacy_scene,
        &mut legacy_memory,
        UiInput::default(),
        None,
    );
    let (presented_route, presented_output, presented_frame) = run(
        &mut presented_scene,
        &mut presented_memory,
        UiInput::default(),
        Some((ShortcutPlatform::Windows, &localizer)),
    );
    assert_eq!(legacy_route, presented_route);
    assert_eq!(legacy_output, presented_output);
    assert_eq!(legacy_frame.semantics, presented_frame.semantics);
    assert_eq!(legacy_memory.focused(), presented_memory.focused());
    assert_eq!(passives(&legacy_frame), passives(&presented_frame));
    assert_passive(legacy_route, &legacy_output, &legacy_frame);
    assert_passive(presented_route, &presented_output, &presented_frame);
    assert!(localizer.0.get() > 0);
    assert!(presented_frame.primitives.iter().any(
        |primitive| matches!(primitive, Primitive::Text(text) if text.text.contains("localized"))
    ));
    assert!(
        presented_frame
            .primitives
            .iter()
            .any(|primitive| matches!(primitive, Primitive::Text(text) if text.text == "›"))
    );
    for (_, _, bounds) in passives(&legacy_frame) {
        let input = pointer_input(bounds.center(), true);
        let (legacy_route, legacy_output, legacy_frame) =
            run(&mut legacy_scene, &mut legacy_memory, input.clone(), None);
        let (presented_route, presented_output, presented_frame) = run(
            &mut presented_scene,
            &mut presented_memory,
            input,
            Some((ShortcutPlatform::Windows, &localizer)),
        );
        assert_eq!(legacy_route, PointerRoute::Blocked);
        assert_eq!(legacy_route, presented_route);
        assert_eq!(legacy_output, presented_output);
        assert_eq!(legacy_frame.semantics, presented_frame.semantics);
        assert_eq!(legacy_memory.focused(), presented_memory.focused());
        assert!(legacy_frame.actions.is_empty() && presented_frame.actions.is_empty());
    }
}

#[test]
fn repeated_evaluation_and_visibility_changes_preserve_passive_identity_and_order() {
    let items = |hidden: bool| {
        let mut earlier = action("file.earlier", "Earlier");
        earlier.state.visible = !hidden;
        [
            MenuItem::Label("Before".to_owned()),
            MenuItem::Action(earlier),
            MenuItem::Separator,
            MenuItem::Label("After".to_owned()),
            MenuItem::Action(action("file.final", "Final")),
        ]
    };
    let mut scene = menu_scene(Menu::from_actions([]));
    let OverlaySceneSurface::Menu { overlay, .. } = &mut scene.surfaces_mut()[0] else {
        panic!("menu surface");
    };
    overlay.menu.replace_items(items(false));
    let final_id = rid("file.final");
    let mut memory = UiMemory::new();
    memory.focus(final_id);
    let (route_a, output_a, frame_a) = run(&mut scene, &mut memory, UiInput::default(), None);
    let (route_b, output_b, frame_b) = run(&mut scene, &mut memory, UiInput::default(), None);
    assert_eq!(frame_a.semantics, frame_b.semantics);
    assert_eq!(output_a, output_b);
    assert_eq!(passives(&frame_a), passives(&frame_b));
    let old_center = frame_b
        .semantics
        .get(final_id)
        .expect("final action")
        .bounds
        .center();
    let (target_route, target_output, target_frame) = run(
        &mut scene,
        &mut UiMemory::new(),
        pointer_input(old_center, true),
        None,
    );
    assert_eq!(target_route, PointerRoute::Target(final_id));
    assert!(
        target_output
            .responses
            .iter()
            .any(|response| response.id == final_id)
    );
    assert_passive(target_route, &target_output, &target_frame);
    let OverlaySceneSurface::Menu { overlay, .. } = &mut scene.surfaces_mut()[0] else {
        panic!("menu surface");
    };
    overlay.menu.replace_items(items(true));
    let (route_c, output_c, frame_c) = run(
        &mut scene,
        &mut memory,
        pointer_input(old_center, true),
        None,
    );
    assert_ne!(route_c, PointerRoute::Target(final_id));
    let before = passives(&frame_b);
    let after = passives(&frame_c);
    assert_eq!(
        before
            .iter()
            .map(|row| (&row.0, &row.1))
            .collect::<Vec<_>>(),
        after.iter().map(|row| (&row.0, &row.1)).collect::<Vec<_>>()
    );
    assert_eq!(before[0].2, after[0].2);
    for index in 1..before.len() {
        assert_eq!(
            (before[index].2.y - scene.metrics().row_height).to_bits(),
            after[index].2.y.to_bits()
        );
    }
    for (route, output, frame) in [
        (route_a, &output_a, &frame_a),
        (route_b, &output_b, &frame_b),
        (target_route, &target_output, &target_frame),
        (route_c, &output_c, &frame_c),
    ] {
        assert_passive(route, output, frame);
        assert!(!passives(frame).iter().any(|row| row.0 == final_id));
    }
    assert_eq!(memory.focused(), Some(final_id));
    assert_eq!(
        frame_c
            .semantics
            .focus_order()
            .into_iter()
            .filter(|id| *id != WidgetId::from_raw(73))
            .collect::<Vec<_>>(),
        [final_id]
    );
}
