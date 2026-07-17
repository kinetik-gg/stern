//! Public, windowless conformance for adjacent application-menu overlay replacement.

use std::time::Duration;
use stern_core::{
    ActionContext, ActionDescriptor, ActionId, ActionSource, FrameContext, PhysicalSize, Point,
    PointerButtonState, PointerInput, PointerOrder, PointerRoute, Rect, ScaleFactor,
    SemanticActionKind, Size, TimeInfo, UiInput, UiMemory, ViewportInfo, WidgetId,
    default_dark_theme,
};
use stern_widgets::{
    Menu, MenuBar, MenuBarMenu, MenuBarMenuId, MenuBarOverlayRequest, MenuItem, MenuOverlay,
    OverlayDismissal, OverlayEntry, OverlayId, OverlayKind, OverlayScene, OverlaySceneIntent,
    OverlaySceneOutput, OverlaySceneSurface, OverlayStack, PopoverPlacement, Ui,
};
const ROOT: OverlayId = OverlayId::from_raw(100);
const FILE: MenuBarMenuId = MenuBarMenuId::from_raw(1);
const EDIT: MenuBarMenuId = MenuBarMenuId::from_raw(2);
const HIDDEN: MenuBarMenuId = MenuBarMenuId::from_raw(3);
const EMPTY: MenuBarMenuId = MenuBarMenuId::from_raw(4);
const VIEW: MenuBarMenuId = MenuBarMenuId::from_raw(5);
fn action(id: impl Into<String>, label: impl Into<String>) -> ActionDescriptor {
    ActionDescriptor::new(id, label)
}
fn menu_definitions() -> Vec<MenuBarMenu> {
    let mut hidden = action("hidden.only", "Hidden");
    hidden.state.visible = false;
    vec![
        MenuBarMenu::from_actions(FILE, "File", [action("file.open", "Open")]),
        MenuBarMenu::from_actions(HIDDEN, "Hidden", [hidden]),
        MenuBarMenu::from_actions(EDIT, "Edit", [action("edit.copy", "Copy")]),
        MenuBarMenu::from_actions(EMPTY, "Empty", Vec::<ActionDescriptor>::new()),
        MenuBarMenu::from_actions(VIEW, "View", [action("view.guides", "Guides")]),
    ]
}
fn request(
    anchor_x: f32,
    placement: PopoverPlacement,
    dismissal: OverlayDismissal,
    source: ActionSource,
    context: ActionContext,
) -> MenuBarOverlayRequest {
    MenuBarOverlayRequest {
        overlay_id: ROOT,
        kind: OverlayKind::Menu,
        anchor: Rect::new(anchor_x, 0.0, 60.0, 24.0),
        size: Size::new(160.0, 64.0),
        placement,
        offset: 0.0,
        fit_viewport: false,
        viewport: Rect::new(0.0, 0.0, 640.0, 480.0),
        dismissal,
        source,
        context,
    }
}
fn menu_request(anchor_x: f32, context: ActionContext) -> MenuBarOverlayRequest {
    request(
        anchor_x,
        PopoverPlacement::Below,
        OverlayDismissal::Manual,
        ActionSource::Menu,
        context,
    )
}
fn unrelated_entry() -> OverlayEntry {
    OverlayEntry::new(
        OverlayId::from_raw(900),
        OverlayKind::Tooltip,
        Rect::new(500.0, 20.0, 80.0, 20.0),
    )
}
fn passive(raw: u64, parent: Option<OverlayId>, label: &str) -> OverlaySceneSurface {
    let mut entry = OverlayEntry::new(OverlayId::from_raw(raw), OverlayKind::Popover, Rect::ZERO);
    if let Some(parent) = parent {
        entry = entry.with_parent(parent);
    }
    OverlaySceneSurface::passive(entry, label, label)
}
fn project(
    bar: &MenuBar,
    stack: &mut OverlayStack,
    scene: &mut OverlayScene,
    request: MenuBarOverlayRequest,
) -> MenuOverlay {
    let label = bar.active_menu().expect("active menu").title.clone();
    let overlay = bar.active_overlay(request).expect("active overlay");
    overlay.open_in(stack);
    scene.push(OverlaySceneSurface::menu(label, overlay.clone()));
    overlay
}
fn child_overlay(raw: u64, parent: OverlayId, x: f32, action_id: String) -> MenuOverlay {
    MenuOverlay::new(
        OverlayEntry::new(
            OverlayId::from_raw(raw),
            OverlayKind::Menu,
            Rect::new(x, 20.0, 140.0, 64.0),
        )
        .with_parent(parent),
        Menu::from_actions([action(action_id, "Stale")]),
        ActionSource::Menu,
        ActionContext::Global,
    )
}
fn add_descendants(
    stack: &mut OverlayStack,
    scene: &mut OverlayScene,
    seed: u64,
) -> (OverlayId, OverlayId) {
    let child_id = OverlayId::from_raw(seed);
    let grandchild_id = OverlayId::from_raw(seed + 1);
    let child = child_overlay(seed, ROOT, 220.0, format!("stale.{seed}.child"));
    let grandchild = child_overlay(
        seed + 1,
        child_id,
        380.0,
        format!("stale.{seed}.grandchild"),
    );
    assert!(child.open_child_in(stack, ROOT));
    assert!(grandchild.open_child_in(stack, child_id));
    scene.push(OverlaySceneSurface::menu("Child", child));
    scene.push(OverlaySceneSurface::menu("Grandchild", grandchild));
    (child_id, grandchild_id)
}
fn stack_ids(stack: &OverlayStack) -> Vec<OverlayId> {
    stack.entries().iter().map(|entry| entry.id).collect()
}
fn scene_ids(scene: &OverlayScene) -> Vec<OverlayId> {
    scene
        .surfaces()
        .iter()
        .map(|surface| surface.entry().id)
        .collect()
}
fn scene_menu(scene: &OverlayScene, id: OverlayId) -> &MenuOverlay {
    let surface = scene
        .surfaces()
        .iter()
        .find(|surface| surface.entry().id == id)
        .expect("scene surface");
    let OverlaySceneSurface::Menu { overlay, .. } = surface else {
        panic!("menu surface");
    };
    overlay
}
fn action_ids(overlay: &MenuOverlay) -> Vec<ActionId> {
    overlay
        .visible_items_iter()
        .filter_map(|item| match item {
            MenuItem::Action(action) => Some(action.id.clone()),
            MenuItem::Label(_) | MenuItem::Separator => None,
        })
        .collect()
}
fn action_row(overlay: OverlayId, action: &str) -> WidgetId {
    WidgetId::from_raw(overlay.raw())
        .child("overlay-scene")
        .child(("overlay-action", action))
}
fn pointer_input(point: Point, state: Option<bool>) -> UiInput {
    UiInput {
        pointer: PointerInput {
            position: Some(point),
            primary: match state {
                Some(true) => PointerButtonState::new(true, true, false),
                Some(false) => PointerButtonState::new(false, false, true),
                None => PointerButtonState::default(),
            },
            ..PointerInput::default()
        },
        ..UiInput::default()
    }
}
fn run_frame(
    scene: &mut OverlayScene,
    memory: &mut UiMemory,
    input: UiInput,
) -> (PointerRoute, OverlaySceneOutput, stern_core::FrameOutput) {
    let context = FrameContext::new(
        ViewportInfo::new(
            Size::new(640.0, 480.0),
            PhysicalSize::new(640, 480),
            ScaleFactor::ONE,
        ),
        input,
        TimeInfo::new(Duration::from_millis(500), Duration::from_millis(16), 1),
    );
    let theme = default_dark_theme();
    let mut ui = Ui::begin_frame(context, memory, &theme);
    ui.resolve_pointer_targets(|plan| {
        scene.declare_pointer_targets(plan, PointerOrder::new(100));
    })
    .expect("valid pointer plan");
    let route = ui.memory().pointer_route();
    let output = ui.overlay_scene(scene);
    (route, output, ui.finish_output())
}

#[test]
fn same_root_scene_replacement_closes_descendants_and_preserves_unrelated_roots() {
    let before = passive(900, None, "Before");
    let after = passive(901, None, "After");
    let mut scene = OverlayScene::new();
    scene.push(before.clone());
    scene.push(passive(102, Some(OverlayId::from_raw(101)), "Grandchild"));
    scene.push(passive(101, Some(ROOT), "Child"));
    scene.push(after.clone());
    scene.push(passive(ROOT.raw(), None, "First root"));
    assert_eq!(
        scene_ids(&scene),
        [900, 102, 101, 901, 100].map(OverlayId::from_raw)
    );

    let replacement = OverlaySceneSurface::passive(
        OverlayEntry::new(ROOT, OverlayKind::Popover, Rect::new(1.0, 2.0, 3.0, 4.0)),
        "Replacement",
        "Replacement",
    );
    scene.push(replacement.clone());
    assert_eq!(scene.surfaces(), &[before, after, replacement]);
}

#[test]
fn adjacent_hover_reuses_one_root_and_leaves_one_application_menu_chain() {
    let mut bar = MenuBar::from_menus(menu_definitions());
    let mut stack = OverlayStack::new();
    let mut scene = OverlayScene::new();
    assert!(bar.open(FILE));
    project(
        &bar,
        &mut stack,
        &mut scene,
        menu_request(0.0, ActionContext::Global),
    );
    add_descendants(&mut stack, &mut scene, 110);

    assert!(bar.hover_open(EDIT));
    let edit = project(
        &bar,
        &mut stack,
        &mut scene,
        menu_request(80.0, ActionContext::Editor),
    );
    assert_eq!(stack_ids(&stack), [ROOT]);
    assert_eq!(scene_ids(&scene), [ROOT]);
    assert_eq!(stack.focus_target(), Some(ROOT));
    assert_eq!(action_ids(&edit), [ActionId::new("edit.copy")]);
    assert_eq!(action_ids(scene_menu(&scene, ROOT)), action_ids(&edit));
}

#[test]
#[rustfmt::skip]
fn left_right_switching_reuses_one_root_and_skips_unavailable_headings() {
    let unrelated = unrelated_entry();
    let unrelated_surface = OverlaySceneSurface::passive(unrelated.clone(), "Unrelated", "Unrelated");
    let mut bar = MenuBar::from_menus(menu_definitions());
    let mut stack = OverlayStack::new();
    let mut scene = OverlayScene::new();
    stack.open(unrelated.clone());
    scene.push(unrelated_surface.clone());
    assert!(bar.open(FILE));
    let transitions = [
        (EDIT, 80.0, (PopoverPlacement::Right, OverlayDismissal::Escape, ActionSource::Shortcut, ActionContext::Editor)),
        (VIEW, 120.0, (PopoverPlacement::Above, OverlayDismissal::OutsideClickOrEscape, ActionSource::Programmatic, ActionContext::Widget(WidgetId::from_key("view")))),
        (FILE, 40.0, (PopoverPlacement::Left, OverlayDismissal::Manual, ActionSource::Menu, ActionContext::Global)),
    ];
    for (active, x, policy) in transitions {
        assert_eq!(bar.move_next(), Some(active));
        let request = request(x, policy.0, policy.1, policy.2, policy.3);
        let expected = request.clone();
        let overlay = project(&bar, &mut stack, &mut scene, request);
        assert_eq!(stack_ids(&stack), [unrelated.id, ROOT]);
        assert_eq!(scene_ids(&scene), [unrelated.id, ROOT]);
        assert_eq!(overlay.entry.dismissal, expected.dismissal);
        assert_eq!(overlay.source, expected.source);
        assert_eq!(overlay.context, expected.context);
        assert_eq!(&stack.entries()[0], &unrelated);
        assert_eq!(&stack.entries()[1], &overlay.entry);
        assert_eq!(&scene.surfaces()[0], &unrelated_surface);
        assert_eq!(scene_menu(&scene, ROOT), &overlay);
    }
    assert_eq!(bar.move_previous(), Some(VIEW));
    let request = request(120.0, PopoverPlacement::Above, OverlayDismissal::OutsideClickOrEscape,
        ActionSource::Programmatic, ActionContext::Widget(WidgetId::from_key("view")));
    let expected = request.clone();
    let overlay = project(&bar, &mut stack, &mut scene, request);
    assert_eq!(stack_ids(&stack), [unrelated.id, ROOT]);
    assert_eq!(scene_ids(&scene), [unrelated.id, ROOT]);
    assert_eq!(overlay.entry.dismissal, expected.dismissal);
    assert_eq!(overlay.source, expected.source);
    assert_eq!(overlay.context, expected.context);
    assert_eq!(&stack.entries()[0], &unrelated);
    assert_eq!(&stack.entries()[1], &overlay.entry);
    assert_eq!(&scene.surfaces()[0], &unrelated_surface);
    assert_eq!(scene_menu(&scene, ROOT), &overlay);
}

#[test]
fn replaced_descendant_loses_routes_focus_intents_and_actions() {
    let mut bar = MenuBar::from_menus(menu_definitions());
    let mut stack = OverlayStack::new();
    let mut scene = OverlayScene::new();
    assert!(bar.open(FILE));
    project(
        &bar,
        &mut stack,
        &mut scene,
        menu_request(0.0, ActionContext::Global),
    );
    let (child, _) = add_descendants(&mut stack, &mut scene, 120);
    let stale_row = action_row(child, "stale.120.child");
    let stale_point = Point::new(230.0, 30.0);
    let mut memory = UiMemory::new();
    let (route, before, frame) =
        run_frame(&mut scene, &mut memory, pointer_input(stale_point, None));
    assert_eq!(route, PointerRoute::Target(stale_row));
    assert!(
        before
            .responses
            .iter()
            .any(|response| response.id == stale_row)
    );
    assert!(frame.semantics.get(stale_row).is_some_and(|node| {
        node.actions
            .iter()
            .any(|action| action.kind == SemanticActionKind::Invoke)
    }));
    memory.focus(stale_row);

    assert!(bar.hover_open(EDIT));
    project(
        &bar,
        &mut stack,
        &mut scene,
        menu_request(80.0, ActionContext::Widget(WidgetId::from_key("editor"))),
    );
    let (route, after, frame) =
        run_frame(&mut scene, &mut memory, pointer_input(stale_point, None));
    assert_ne!(route, PointerRoute::Target(stale_row));
    assert!(
        after
            .responses
            .iter()
            .all(|response| response.id != stale_row)
    );
    assert!(frame.semantics.get(stale_row).is_none());
    assert_ne!(memory.focused(), Some(stale_row));
    assert!(after.intents.is_empty() && frame.actions.is_empty());
    assert_eq!(scene_menu(&scene, ROOT).menu.highlighted_action_id(), None);

    run_frame(
        &mut scene,
        &mut memory,
        pointer_input(Point::new(90.0, 30.0), Some(true)),
    );
    let (_, output, mut frame) = run_frame(
        &mut scene,
        &mut memory,
        pointer_input(Point::new(90.0, 30.0), Some(false)),
    );
    let expected = stern_core::ActionInvocation::new(
        ActionId::new("edit.copy"),
        ActionSource::Menu,
        ActionContext::Widget(WidgetId::from_key("editor")),
    );
    assert_eq!(
        output.intents,
        [OverlaySceneIntent::Action(expected.clone())]
    );
    assert_eq!(frame.actions.pop_front(), Some(expected));
    assert!(frame.actions.is_empty());
}

#[test]
#[rustfmt::skip]
fn repeated_switching_and_menu_reconciliation_remain_deterministic() {
    let unrelated = unrelated_entry();
    let mut bar = MenuBar::from_menus(menu_definitions());
    let mut stack = OverlayStack::new();
    let mut scene = OverlayScene::new();
    stack.open(unrelated.clone());
    scene.push(OverlaySceneSurface::passive(unrelated.clone(), "Unrelated", "Unrelated"));
    assert!(bar.open(FILE));
    project(&bar, &mut stack, &mut scene, menu_request(0.0, ActionContext::Global));
    for (seed, movement, active, x) in [(130, 0, VIEW, 120.0), (140, -1, EDIT, 80.0), (150, 1, VIEW, 120.0)] {
        let stale = add_descendants(&mut stack, &mut scene, seed);
        assert!(match movement {
            0 => bar.hover_open(active),
            -1 => bar.move_previous() == Some(active),
            _ => bar.move_next() == Some(active),
        });
        let overlay = project(&bar, &mut stack, &mut scene, menu_request(x, ActionContext::Global));
        assert_eq!(stack_ids(&stack), [unrelated.id, ROOT]);
        assert_eq!(scene_ids(&scene), [unrelated.id, ROOT]);
        assert_eq!(scene_menu(&scene, ROOT), &overlay);
        assert!(!stack_ids(&stack).contains(&stale.0));
        assert!(!scene_ids(&scene).contains(&stale.1));
    }
    let mut definitions = bar.menus().to_vec();
    definitions.reverse();
    let stack_before = stack.clone();
    let scene_before = scene.clone();
    bar.replace_menus(definitions);
    assert_eq!(bar.active_id(), Some(VIEW));
    assert_eq!(stack, stack_before);
    assert_eq!(scene, scene_before);
    bar.replace_menus(bar.menus().iter().filter(|menu| menu.id != FILE).cloned().collect::<Vec<_>>());
    assert_eq!(bar.active_id(), Some(VIEW));
    assert!(!bar.open(MenuBarMenuId::from_raw(999)) && !bar.open(HIDDEN) && !bar.open(EMPTY));
    assert_eq!(stack, stack_before);
    assert_eq!(scene, scene_before);
    bar.replace_menus(bar.menus().iter().filter(|menu| menu.id != VIEW).cloned().collect::<Vec<_>>());
    assert_eq!(bar.active_id(), None);
    assert_eq!(stack, stack_before);
    assert_eq!(scene, scene_before);
    assert!(bar.open(EDIT));
    let stale = add_descendants(&mut stack, &mut scene, 160);
    let request = request(80.0, PopoverPlacement::Below, OverlayDismissal::OutsideClickOrEscape,
        ActionSource::Menu, ActionContext::Editor);
    project(&bar, &mut stack, &mut scene, request);
    assert!(!stack_ids(&stack).contains(&stale.0));
    assert!(!scene_ids(&scene).contains(&stale.1));
    assert_eq!(scene_ids(&scene), [unrelated.id, ROOT]);
    let final_row = action_row(ROOT, "edit.copy");
    let obsolete_rows = [action_row(ROOT, "file.open"), action_row(ROOT, "view.guides"),
        action_row(stale.0, "stale.160.child"), action_row(stale.1, "stale.160.grandchild")];
    let mut memory = UiMemory::new();
    let point = Point::new(90.0, 30.0);
    let (route, pressed, frame) = run_frame(&mut scene, &mut memory, pointer_input(point, Some(true)));
    assert_eq!(route, PointerRoute::Target(final_row));
    assert!(pressed.responses.iter().any(|response| response.id == final_row && response.state.pressed));
    assert!(pressed.intents.is_empty() && frame.actions.is_empty());
    let (route, released, mut frame) = run_frame(&mut scene, &mut memory, pointer_input(point, Some(false)));
    assert_eq!(route, PointerRoute::Target(final_row));
    let response = released.responses.iter().find(|response| response.id == final_row).expect("final Edit response");
    assert!(response.clicked && !response.state.pressed);
    assert!(released.responses.iter().all(|response| !obsolete_rows.contains(&response.id)));
    let expected = stern_core::ActionInvocation::new(ActionId::new("edit.copy"), ActionSource::Menu, ActionContext::Editor);
    assert_eq!(released.intents, [OverlaySceneIntent::Action(expected.clone())]);
    assert_eq!(frame.actions.pop_front(), Some(expected));
    assert!(frame.actions.is_empty());
}
