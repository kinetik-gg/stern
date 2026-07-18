//! Public, windowless painting/input/semantic conformance for overlay scenes.

use std::time::Duration;

use stern_core::{
    ActionContext, ActionDescriptor, ActionId, ActionSource, Brush, Color, FrameContext, Key,
    KeyEvent, KeyState, KeyboardInput, Modifiers, PhysicalSize, Point, PointerButtonState,
    PointerInput, PointerOrder, PointerTarget, Primitive, RadiusScale, Rect, Response, ScaleFactor,
    SemanticActionKind, SemanticRole, ShadowPrimitive, Size, StrokeScale, Theme, TimeInfo, UiInput,
    UiMemory, Vec2, ViewportInfo, WidgetId, default_dark_theme,
};
use stern_widgets::overlays::OverlayNavigationInput;
use stern_widgets::{
    CommandPaletteOverlay, DropdownItem, DropdownItemId, DropdownModel, DropdownOverlay, Menu,
    MenuItem, MenuOverlay, ModalAction, ModalActionRole, ModalDialog, ModalDialogOverlay,
    OverlayDismissal, OverlayEntry, OverlayId, OverlayKind, OverlayScene,
    OverlaySceneDismissReason, OverlaySceneIntent, OverlaySceneOutput, OverlaySceneSurface,
    PopoverPlacement, Ui,
};

const LOWER_RECT: Rect = Rect::new(0.0, 0.0, 320.0, 240.0);

fn action(id: &str, label: &str) -> ActionDescriptor {
    ActionDescriptor::new(id, label)
}

fn entry(raw: u64, kind: OverlayKind, rect: Rect) -> OverlayEntry {
    OverlayEntry::new(OverlayId::from_raw(raw), kind, rect)
}

fn menu_surface(raw: u64, kind: OverlayKind, menu: Menu) -> OverlaySceneSurface {
    OverlaySceneSurface::menu(
        "Commands",
        MenuOverlay::new(
            entry(raw, kind, Rect::new(20.0, 20.0, 180.0, 100.0))
                .dismiss_on(OverlayDismissal::OutsideClickOrEscape),
            menu,
            ActionSource::Menu,
            ActionContext::Global,
        ),
    )
}

fn pressed_at(x: f32, y: f32) -> UiInput {
    UiInput {
        pointer: PointerInput {
            position: Some(Point::new(x, y)),
            primary: PointerButtonState::new(true, true, false),
            ..PointerInput::default()
        },
        ..UiInput::default()
    }
}

fn released_at(x: f32, y: f32) -> UiInput {
    UiInput {
        pointer: PointerInput {
            position: Some(Point::new(x, y)),
            primary: PointerButtonState::new(false, false, true),
            ..PointerInput::default()
        },
        ..UiInput::default()
    }
}

fn pressed_key(key: Key) -> UiInput {
    UiInput {
        keyboard: KeyboardInput {
            modifiers: Modifiers::default(),
            events: vec![KeyEvent::new(
                key,
                KeyState::Pressed,
                Modifiers::default(),
                false,
            )],
        },
        ..UiInput::default()
    }
}

fn context(input: UiInput) -> FrameContext {
    FrameContext::new(
        ViewportInfo::new(
            Size::new(640.0, 480.0),
            PhysicalSize::new(640, 480),
            ScaleFactor::ONE,
        ),
        input,
        TimeInfo::new(Duration::from_millis(500), Duration::from_millis(16), 1),
    )
}

#[allow(clippy::float_cmp)]
fn assert_exact_overlay_shadow(shadow: &ShadowPrimitive, entry: &OverlayEntry, radius: f32) {
    let (offset_y, blur_radius, alpha) =
        if entry.modal || matches!(entry.kind, OverlayKind::Modal | OverlayKind::CommandPalette) {
            (12.0, 36.0, 0.52)
        } else if matches!(entry.kind, OverlayKind::Tooltip | OverlayKind::DragPreview) {
            (2.0, 6.0, 0.32)
        } else {
            (6.0, 18.0, 0.42)
        };

    assert_eq!(shadow.rect, entry.rect);
    assert_eq!(shadow.offset, Vec2::new(0.0, offset_y));
    assert_eq!(shadow.blur_radius, blur_radius);
    assert_eq!(shadow.spread, 0.0);
    assert_eq!(shadow.radius, radius);
    assert_eq!(shadow.color, Color::rgba(0.0, 0.0, 0.0, alpha));
}

fn run_frame(
    scene: &mut OverlayScene,
    memory: &mut UiMemory,
    input: UiInput,
    lower: bool,
) -> (
    Option<Response>,
    OverlaySceneOutput,
    stern_core::FrameOutput,
) {
    let theme = default_dark_theme();
    run_frame_with_theme(scene, memory, input, lower, &theme)
}

fn run_frame_with_theme(
    scene: &mut OverlayScene,
    memory: &mut UiMemory,
    input: UiInput,
    lower: bool,
    theme: &Theme,
) -> (
    Option<Response>,
    OverlaySceneOutput,
    stern_core::FrameOutput,
) {
    let mut ui = Ui::begin_frame(context(input), memory, theme);
    let lower_id = ui.make_id("lower-button");
    ui.resolve_pointer_targets(|plan| {
        if lower {
            plan.target(PointerTarget::new(
                lower_id,
                LOWER_RECT,
                PointerOrder::new(10),
            ));
        }
        scene.declare_pointer_targets(plan, PointerOrder::new(100));
    })
    .expect("one valid pointer plan");
    let lower_response = lower.then(|| ui.pressable("lower-button", LOWER_RECT, false));
    let scene_output = ui.overlay_scene(scene);
    let frame = ui.finish_output();
    (lower_response, scene_output, frame)
}

#[test]
fn fitted_menu_surfaces_and_outer_clips_preserve_contained_state() {
    let viewport = Rect::new(32.0, 24.0, 180.0, 96.0);
    let owner = WidgetId::from_key("fitted-menu-owner");
    let oversized = MenuOverlay::anchored(
        OverlayId::from_raw(23),
        OverlayKind::Menu,
        Menu::from_actions([action("oversized", "Oversized")]),
        Rect::new(190.0, 100.0, 12.0, 12.0),
        Size::new(400.0, 300.0),
        PopoverPlacement::Below,
        4.0,
        true,
        viewport,
        OverlayDismissal::OutsideClickOrEscape,
        ActionSource::Menu,
        ActionContext::Widget(owner),
    );
    let fitting_size = Size::new(80.0, 48.0);
    let fitting = MenuOverlay::anchored(
        OverlayId::from_raw(24),
        OverlayKind::ContextMenu,
        Menu::from_actions([action("fitting", "Fitting")]),
        Rect::new(204.0, 92.0, 8.0, 8.0),
        fitting_size,
        PopoverPlacement::Right,
        4.0,
        true,
        viewport,
        OverlayDismissal::Manual,
        ActionSource::Programmatic,
        ActionContext::Editor,
    );
    assert_eq!(oversized.entry.rect, viewport);
    assert!(viewport.contains_rect(fitting.entry.rect));
    assert_eq!(fitting.entry.rect.size(), fitting_size);

    let mut scene = OverlayScene::new();
    scene.push(OverlaySceneSurface::menu("Oversized", oversized.clone()));
    scene.push(OverlaySceneSurface::menu("Fitting", fitting.clone()));
    assert_eq!(
        scene
            .surfaces()
            .iter()
            .map(|surface| surface.entry().id)
            .collect::<Vec<_>>(),
        vec![oversized.entry.id, fitting.entry.id]
    );
    let mut memory = UiMemory::new();
    let (_, output, frame) = run_frame(&mut scene, &mut memory, UiInput::default(), false);

    let clip_rects = frame
        .primitives
        .iter()
        .filter_map(|primitive| match primitive {
            Primitive::ClipBegin { rect, .. } => Some(*rect),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(clip_rects, vec![oversized.entry.rect, fitting.entry.rect]);
    let OverlaySceneSurface::Menu { overlay: first, .. } = &scene.surfaces()[0] else {
        panic!("oversized menu surface");
    };
    let OverlaySceneSurface::Menu {
        overlay: second, ..
    } = &scene.surfaces()[1]
    else {
        panic!("fitting menu surface");
    };
    assert_eq!(first, &oversized);
    assert_eq!(second, &fitting);
    assert!(output.intents.is_empty());
    assert!(frame.actions.is_empty());
}

#[test]
fn overlay_row_wins_pointer_arbitration_and_mouse_emits_the_action() {
    let icon = stern_icons_phosphor::regular::FOLDER_OPEN;
    let mut scene = OverlayScene::new();
    scene.push(menu_surface(
        1,
        OverlayKind::Menu,
        Menu::from_actions([action("file.open", "Open").with_icon(icon)]),
    ));
    let mut memory = UiMemory::new();

    let (lower_press, press, _) = run_frame(&mut scene, &mut memory, pressed_at(30.0, 30.0), true);
    assert!(!lower_press.expect("lower response").state.hovered);
    assert!(press.responses[0].state.pressed);
    assert!(press.intents.is_empty());

    let (lower_release, release, mut frame) =
        run_frame(&mut scene, &mut memory, released_at(30.0, 30.0), true);
    assert!(!lower_release.expect("lower response").clicked);
    let OverlaySceneIntent::Action(invocation) = &release.intents[0] else {
        panic!("expected action intent");
    };
    assert_eq!(invocation.action_id, ActionId::new("file.open"));
    assert_eq!(invocation.source, ActionSource::Menu);
    assert!(frame.primitives.iter().any(
        |primitive| matches!(primitive, Primitive::Icon(painted) if painted.icon == icon.icon())
    ));
    assert!(
        frame
            .primitives
            .iter()
            .any(|primitive| matches!(primitive, Primitive::Text(text) if text.text == "Open"))
    );
    assert_eq!(frame.actions.pop_front(), Some(invocation.clone()));
}

#[test]
fn outside_dismissal_blocks_the_lower_click() {
    let mut scene = OverlayScene::new();
    scene.push(menu_surface(
        2,
        OverlayKind::ContextMenu,
        Menu::from_actions([action("edit.copy", "Copy")]),
    ));
    let mut memory = UiMemory::new();

    let (lower_press, _, _) = run_frame(&mut scene, &mut memory, pressed_at(4.0, 4.0), true);
    assert!(!lower_press.expect("lower response").state.hovered);
    let (lower_release, release, _) =
        run_frame(&mut scene, &mut memory, released_at(4.0, 4.0), true);
    assert!(!lower_release.expect("lower response").clicked);
    assert_eq!(
        release.intents,
        vec![OverlaySceneIntent::Dismiss(
            stern_widgets::OverlaySceneDismissRequest {
                overlay_id: OverlayId::from_raw(2),
                reason: OverlaySceneDismissReason::OutsideClick,
                focus_return: None,
            }
        )]
    );
}

#[test]
fn keyboard_enter_matches_mouse_action_and_disabled_rows_stay_inert() {
    let mut disabled = action("file.disabled", "Disabled");
    disabled.state.enabled = false;
    let menu = Menu::from_actions([disabled, action("file.save", "Save")]);

    let mut mouse_scene = OverlayScene::new();
    mouse_scene.push(menu_surface(3, OverlayKind::Menu, menu.clone()));
    let mut mouse_memory = UiMemory::new();
    let (_, disabled_press, _) = run_frame(
        &mut mouse_scene,
        &mut mouse_memory,
        pressed_at(30.0, 30.0),
        false,
    );
    assert!(
        disabled_press
            .responses
            .iter()
            .all(|response| !response.state.hovered && !response.state.pressed)
    );
    let (_, disabled_release, _) = run_frame(
        &mut mouse_scene,
        &mut mouse_memory,
        released_at(30.0, 30.0),
        false,
    );
    assert!(disabled_release.intents.is_empty());

    let (_, _, _) = run_frame(
        &mut mouse_scene,
        &mut mouse_memory,
        pressed_at(30.0, 58.0),
        false,
    );
    let (_, mouse_release, _) = run_frame(
        &mut mouse_scene,
        &mut mouse_memory,
        released_at(30.0, 58.0),
        false,
    );

    let mut keyboard_scene = OverlayScene::new();
    keyboard_scene.push(menu_surface(4, OverlayKind::Menu, menu));
    let mut keyboard_memory = UiMemory::new();
    keyboard_memory.focus(
        WidgetId::from_raw(4)
            .child("overlay-scene")
            .child(("overlay-action", "file.save")),
    );
    let (_, keyboard_output, keyboard_frame) = run_frame(
        &mut keyboard_scene,
        &mut keyboard_memory,
        pressed_key(Key::Enter),
        false,
    );

    let OverlaySceneIntent::Action(mouse) = &mouse_release.intents[0] else {
        panic!("mouse action");
    };
    let OverlaySceneIntent::Action(keyboard) = &keyboard_output.intents[0] else {
        panic!("keyboard action");
    };
    assert_eq!(mouse.action_id, keyboard.action_id);
    assert_eq!(mouse.source, keyboard.source);
    assert_eq!(mouse.context, keyboard.context);
    assert_eq!(keyboard_frame.actions.len(), 1);
    assert_eq!(keyboard_output.intents.len(), 1);
}

#[test]
fn clipped_overflow_rows_emit_no_responses_or_semantics() {
    let mut scene = OverlayScene::new();
    scene.push(OverlaySceneSurface::menu(
        "Short menu",
        MenuOverlay::new(
            entry(18, OverlayKind::Menu, Rect::new(20.0, 20.0, 180.0, 36.0)),
            Menu::from_actions([
                action("visible", "Visible"),
                action("overflow.one", "Overflow one"),
                action("overflow.two", "Overflow two"),
            ]),
            ActionSource::Menu,
            ActionContext::Global,
        ),
    ));
    let root = WidgetId::from_raw(18).child("overlay-scene");
    let visible = root.child(("overlay-action", "visible"));
    let overflow_one = root.child(("overlay-action", "overflow.one"));
    let overflow_two = root.child(("overlay-action", "overflow.two"));
    let mut memory = UiMemory::new();

    let (_, output, frame) = run_frame(&mut scene, &mut memory, UiInput::default(), false);

    assert_eq!(
        output
            .responses
            .iter()
            .map(|response| response.id)
            .collect::<Vec<_>>(),
        vec![visible]
    );
    assert!(frame.semantics.get(visible).is_some());
    assert!(frame.semantics.get(overflow_one).is_none());
    assert!(frame.semantics.get(overflow_two).is_none());
    assert_eq!(
        frame
            .semantics
            .get(WidgetId::from_raw(18))
            .expect("surface semantics")
            .children,
        vec![visible]
    );
}

#[test]
#[allow(clippy::float_cmp)]
fn overlay_stroke_roles_preserve_focused_geometry_order_hits_and_semantics() {
    let mut menu = Menu::new();
    menu.push(MenuItem::Action(action("first", "First")));
    menu.push(MenuItem::Separator);
    menu.push(MenuItem::Action(action("second", "Second")));
    let mut scene = OverlayScene::new();
    scene.push(menu_surface(22, OverlayKind::Menu, menu));
    let strokes = StrokeScale::from_values(0.75, 1.25, 2.5, 3.5, 4.5);
    let theme = default_dark_theme().with_strokes(strokes);
    let focused_id = WidgetId::from_raw(22)
        .child("overlay-scene")
        .child(("overlay-action", "first"));

    let mut unfocused_memory = UiMemory::new();
    let (_, unfocused_output, unfocused_frame) = run_frame_with_theme(
        &mut scene,
        &mut unfocused_memory,
        UiInput::default(),
        false,
        &theme,
    );
    let mut focused_memory = UiMemory::new();
    focused_memory.focus(focused_id);
    let (_, focused_output, focused_frame) = run_frame_with_theme(
        &mut scene,
        &mut focused_memory,
        UiInput::default(),
        false,
        &theme,
    );

    let response_rects = |output: &OverlaySceneOutput| {
        output
            .responses
            .iter()
            .map(|response| (response.id, response.rect))
            .collect::<Vec<_>>()
    };
    let rect_geometry = |frame: &stern_core::FrameOutput| {
        frame
            .primitives
            .iter()
            .filter_map(|primitive| match primitive {
                Primitive::Rect(rect) => Some((
                    rect.rect,
                    rect.radius,
                    rect.stroke.map(|stroke| stroke.width),
                )),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    let semantic_geometry = |frame: &stern_core::FrameOutput| {
        frame
            .semantics
            .nodes()
            .iter()
            .map(|node| (node.id, node.bounds))
            .collect::<Vec<_>>()
    };
    assert_eq!(
        unfocused_frame.primitives.len(),
        focused_frame.primitives.len()
    );
    assert_eq!(
        rect_geometry(&unfocused_frame),
        rect_geometry(&focused_frame)
    );
    assert_eq!(
        response_rects(&unfocused_output),
        response_rects(&focused_output)
    );
    assert_eq!(
        semantic_geometry(&unfocused_frame),
        semantic_geometry(&focused_frame)
    );

    let overlay_rect = scene.surfaces()[0].entry().rect;
    let surface = unfocused_frame
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            Primitive::Rect(rect) if rect.rect == overlay_rect => Some(rect),
            _ => None,
        })
        .expect("overlay surface");
    assert_eq!(
        surface.stroke.map(|stroke| stroke.width),
        Some(strokes.default)
    );
    let separator = unfocused_frame
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            Primitive::Rect(rect)
                if rect.fill == Some(Brush::Solid(theme.colors.border.subtle))
                    && rect.rect.height == strokes.hairline =>
            {
                Some(rect)
            }
            _ => None,
        })
        .expect("overlay separator");
    assert_eq!(separator.rect.height, strokes.hairline);
}

#[test]
fn menu_navigation_overrides_stale_focused_row_on_enter() {
    let mut scene = OverlayScene::new();
    scene.push(menu_surface(
        19,
        OverlayKind::Menu,
        Menu::from_actions([action("first", "First"), action("second", "Second")]),
    ));
    let OverlaySceneSurface::Menu { overlay, .. } = &mut scene.surfaces_mut()[0] else {
        panic!("menu surface");
    };
    overlay.navigate(OverlayNavigationInput::First);
    let mut memory = UiMemory::new();
    memory.focus(
        WidgetId::from_raw(19)
            .child("overlay-scene")
            .child(("overlay-action", "first")),
    );
    let input = UiInput {
        keyboard: KeyboardInput {
            modifiers: Modifiers::default(),
            events: [Key::ArrowDown, Key::Enter]
                .into_iter()
                .map(|key| KeyEvent::new(key, KeyState::Pressed, Modifiers::default(), false))
                .collect(),
        },
        ..UiInput::default()
    };

    let (_, output, mut frame) = run_frame(&mut scene, &mut memory, input, false);

    assert_eq!(output.intents.len(), 1);
    let OverlaySceneIntent::Action(invocation) = &output.intents[0] else {
        panic!("menu action");
    };
    assert_eq!(invocation.action_id, ActionId::new("second"));
    assert_eq!(frame.actions.len(), 1);
    assert_eq!(
        frame.actions.pop_front().map(|action| action.action_id),
        Some(ActionId::new("second"))
    );
}

#[test]
fn repeated_enter_does_not_reinvoke_an_overlay_action() {
    let mut scene = OverlayScene::new();
    scene.push(menu_surface(
        20,
        OverlayKind::Menu,
        Menu::from_actions([action("once", "Once")]),
    ));
    let OverlaySceneSurface::Menu { overlay, .. } = &mut scene.surfaces_mut()[0] else {
        panic!("menu surface");
    };
    overlay.navigate(OverlayNavigationInput::First);
    let mut memory = UiMemory::new();
    let input = UiInput {
        keyboard: KeyboardInput {
            modifiers: Modifiers::default(),
            events: vec![KeyEvent::new(
                Key::Enter,
                KeyState::Pressed,
                Modifiers::default(),
                true,
            )],
        },
        ..UiInput::default()
    };

    let (_, output, frame) = run_frame(&mut scene, &mut memory, input, false);

    assert!(output.intents.is_empty());
    assert!(frame.actions.is_empty());
}

#[test]
fn hiding_an_earlier_action_preserves_label_and_separator_ids() {
    fn scene_with_prefix_visibility(visible: bool) -> OverlayScene {
        let mut prefix = action("prefix", "Prefix");
        prefix.state.visible = visible;
        let mut menu = Menu::new();
        menu.push(MenuItem::Action(prefix));
        menu.push(MenuItem::Label("Group".to_owned()));
        menu.push(MenuItem::Separator);
        menu.push(MenuItem::Action(action("remaining", "Remaining")));
        let mut scene = OverlayScene::new();
        scene.push(menu_surface(21, OverlayKind::Menu, menu));
        scene
    }

    let mut visible_scene = scene_with_prefix_visibility(true);
    let mut visible_memory = UiMemory::new();
    let (_, _, visible_frame) = run_frame(
        &mut visible_scene,
        &mut visible_memory,
        UiInput::default(),
        false,
    );
    let mut hidden_scene = scene_with_prefix_visibility(false);
    let mut hidden_memory = UiMemory::new();
    let (_, _, hidden_frame) = run_frame(
        &mut hidden_scene,
        &mut hidden_memory,
        UiInput::default(),
        false,
    );
    let surface_id = WidgetId::from_raw(21);
    let visible_children = &visible_frame
        .semantics
        .get(surface_id)
        .expect("visible surface")
        .children;
    let hidden_children = &hidden_frame
        .semantics
        .get(surface_id)
        .expect("hidden surface")
        .children;

    assert_eq!(visible_children[1], hidden_children[0]);
    assert_eq!(visible_children[2], hidden_children[1]);
    assert_eq!(
        hidden_frame
            .semantics
            .get(hidden_children[0])
            .expect("label")
            .role,
        SemanticRole::Label
    );
    assert!(matches!(
        &hidden_frame
            .semantics
            .get(hidden_children[1])
            .expect("separator")
            .role,
        SemanticRole::Custom(role) if role == "separator"
    ));
}

#[test]
fn submenu_intent_and_semantics_preserve_identity_source_and_context() {
    let mut menu = Menu::new();
    let mut checked = action("view.guides", "Guides");
    checked.state.checked = Some(true);
    menu.push_submenu(checked, Menu::from_actions([action("view.grid", "Grid")]));
    let owner = WidgetId::from_key("viewport");
    let mut scene = OverlayScene::new();
    scene.push(OverlaySceneSurface::menu(
        "View",
        MenuOverlay::new(
            entry(5, OverlayKind::Menu, Rect::new(20.0, 20.0, 180.0, 100.0)),
            menu,
            ActionSource::Menu,
            ActionContext::Widget(owner),
        ),
    ));
    let OverlaySceneSurface::Menu { overlay, .. } = &mut scene.surfaces_mut()[0] else {
        panic!("menu surface");
    };
    overlay.navigate(OverlayNavigationInput::First);
    let mut memory = UiMemory::new();
    let (_, result, frame) = run_frame(&mut scene, &mut memory, pressed_key(Key::Enter), false);

    let OverlaySceneIntent::OpenSubmenu(intent) = &result.intents[0] else {
        panic!("submenu intent");
    };
    assert_eq!(intent.parent_overlay, OverlayId::from_raw(5));
    assert_eq!(intent.trigger_action, ActionId::new("view.guides"));
    assert_eq!(intent.source, ActionSource::Menu);
    assert_eq!(intent.context, ActionContext::Widget(owner));

    let row_id = WidgetId::from_raw(5)
        .child("overlay-scene")
        .child(("overlay-action", "view.guides"));
    let row = frame.semantics.get(row_id).expect("submenu semantics");
    assert_eq!(row.state.checked, Some(true));
    assert_eq!(row.state.expanded, Some(false));
    assert!(
        row.actions
            .iter()
            .any(|action| action.kind == SemanticActionKind::Open)
    );
}

#[test]
fn dropdown_keyboard_selection_and_escape_return_trigger_focus() {
    let trigger = WidgetId::from_key("mode-trigger");
    let dropdown = DropdownOverlay::new(
        entry(
            6,
            OverlayKind::Dropdown,
            Rect::new(20.0, 20.0, 180.0, 100.0),
        )
        .dismiss_on(OverlayDismissal::OutsideClickOrEscape),
        trigger,
        DropdownModel::from_items([
            DropdownItem::new(DropdownItemId::from_raw(1), "Source"),
            DropdownItem::new(DropdownItemId::from_raw(2), "Composite"),
        ]),
    );
    let mut scene = OverlayScene::new();
    scene.push(OverlaySceneSurface::dropdown("Mode", dropdown));
    let OverlaySceneSurface::Dropdown { overlay, .. } = &mut scene.surfaces_mut()[0] else {
        panic!("dropdown surface");
    };
    overlay.navigate(OverlayNavigationInput::Last);
    let mut memory = UiMemory::new();
    let (_, selected, _) = run_frame(&mut scene, &mut memory, pressed_key(Key::Enter), false);
    let OverlaySceneIntent::SelectDropdown(selected) = selected.intents[0] else {
        panic!("dropdown selection");
    };
    assert_eq!(selected.overlay_id, OverlayId::from_raw(6));
    assert_eq!(selected.item_id, DropdownItemId::from_raw(2));
    assert_eq!(selected.focus_return, trigger);

    let (_, escaped, _) = run_frame(&mut scene, &mut memory, pressed_key(Key::Escape), false);
    let OverlaySceneIntent::Dismiss(request) = escaped.intents[0] else {
        panic!("dropdown dismissal");
    };
    assert_eq!(request.overlay_id, OverlayId::from_raw(6));
    assert_eq!(request.reason, OverlaySceneDismissReason::Escape);
    assert_eq!(request.focus_return, Some(trigger));
}

#[test]
fn command_palette_paints_query_and_invokes_the_selected_match() {
    let icon = stern_icons_phosphor::regular::FLOPPY_DISK;
    let mut save = action("file.save", "Save");
    save.keywords.push("write".to_owned());
    save.icon = Some(icon.into());
    let mut palette = CommandPaletteOverlay::from_actions(
        entry(
            7,
            OverlayKind::CommandPalette,
            Rect::new(20.0, 20.0, 220.0, 120.0),
        )
        .modal(true)
        .dismiss_on(OverlayDismissal::OutsideClickOrEscape),
        &[save, action("file.open", "Open")],
        ActionContext::Global,
    );
    palette.palette.query = "write".to_owned();
    let mut scene = OverlayScene::new();
    scene.push(OverlaySceneSurface::command_palette("Commands", palette));
    let mut memory = UiMemory::new();
    let (_, result, frame) = run_frame(&mut scene, &mut memory, pressed_key(Key::Enter), false);

    assert!(
        frame
            .primitives
            .iter()
            .any(|primitive| matches!(primitive, Primitive::Text(text) if text.text == "> write"))
    );
    assert!(
        frame
            .primitives
            .iter()
            .any(|primitive| matches!(primitive, Primitive::Text(text) if text.text == "Save"))
    );
    assert!(frame.primitives.iter().any(
        |primitive| matches!(primitive, Primitive::Icon(painted) if painted.icon == icon.icon())
    ));
    let OverlaySceneIntent::Action(invocation) = &result.intents[0] else {
        panic!("palette action");
    };
    assert_eq!(invocation.action_id, ActionId::new("file.save"));
    assert_eq!(invocation.source, ActionSource::CommandPalette);
}

#[test]
fn command_palette_escape_clears_query_before_dismissal() {
    let overlay_id = OverlayId::from_raw(71);
    let mut palette = CommandPaletteOverlay::from_actions(
        OverlayEntry::new(
            overlay_id,
            OverlayKind::CommandPalette,
            Rect::new(20.0, 20.0, 220.0, 120.0),
        )
        .modal(true)
        .dismiss_on(OverlayDismissal::OutsideClickOrEscape),
        &[action("file.save", "Save"), action("file.open", "Open")],
        ActionContext::Global,
    );
    palette.palette.query = "save".to_owned();
    palette.palette.selected = usize::MAX;
    let mut scene = OverlayScene::new();
    scene.push(OverlaySceneSurface::command_palette("Commands", palette));
    let mut memory = UiMemory::new();

    let (_, armed, armed_frame) = run_frame(&mut scene, &mut memory, pressed_at(30.0, 60.0), false);
    assert!(armed.intents.is_empty());
    assert!(armed_frame.actions.is_empty());
    let mut consumed_input = released_at(30.0, 60.0);
    consumed_input.keyboard = KeyboardInput {
        modifiers: Modifiers::default(),
        events: [Key::Escape, Key::Enter]
            .into_iter()
            .map(|key| KeyEvent::new(key, KeyState::Pressed, Modifiers::default(), false))
            .collect(),
    };
    let (_, first, first_frame) = run_frame(&mut scene, &mut memory, consumed_input, false);
    assert!(first.intents.is_empty());
    assert!(first_frame.actions.is_empty());
    assert_eq!(first_frame.repaint, stern_core::RepaintRequest::NextFrame);
    assert_eq!(scene.surfaces().len(), 1);
    let OverlaySceneSurface::CommandPalette { overlay, .. } = &scene.surfaces()[0] else {
        panic!("command palette surface");
    };
    assert_eq!(overlay.entry.id, overlay_id);
    assert!(overlay.palette.query.is_empty());
    assert_eq!(overlay.palette.selected, 1);

    let (_, second, second_frame) =
        run_frame(&mut scene, &mut memory, pressed_key(Key::Escape), false);
    assert!(second_frame.actions.is_empty());
    assert_eq!(second.intents.len(), 1);
    let OverlaySceneIntent::Dismiss(request) = second.intents[0] else {
        panic!("command palette dismissal");
    };
    assert_eq!(request.overlay_id, overlay_id);
    assert_eq!(request.reason, OverlaySceneDismissReason::Escape);
}

#[test]
#[allow(clippy::too_many_lines)]
fn every_overlay_kind_paints_an_ordered_themed_surface_and_children() {
    let mut scene = OverlayScene::new();
    scene.push(OverlaySceneSurface::menu(
        "Menu",
        MenuOverlay::new(
            entry(10, OverlayKind::Menu, Rect::new(20.0, 20.0, 160.0, 80.0)),
            Menu::from_actions([action("menu", "Menu")]),
            ActionSource::Menu,
            ActionContext::Global,
        ),
    ));
    scene.push(OverlaySceneSurface::menu(
        "Context menu",
        MenuOverlay::new(
            entry(
                11,
                OverlayKind::ContextMenu,
                Rect::new(20.0, 110.0, 160.0, 80.0),
            ),
            Menu::from_actions([action("context", "Context")]),
            ActionSource::Menu,
            ActionContext::Global,
        ),
    ));
    scene.push(OverlaySceneSurface::dropdown(
        "Dropdown",
        DropdownOverlay::new(
            entry(
                12,
                OverlayKind::Dropdown,
                Rect::new(220.0, 20.0, 160.0, 80.0),
            ),
            WidgetId::from_key("dropdown"),
            DropdownModel::from_items([DropdownItem::new(DropdownItemId::from_raw(1), "One")]),
        ),
    ));
    scene.push(OverlaySceneSurface::command_palette(
        "Palette",
        CommandPaletteOverlay::from_actions(
            entry(
                13,
                OverlayKind::CommandPalette,
                Rect::new(400.0, 20.0, 180.0, 100.0),
            ),
            &[action("palette", "Palette")],
            ActionContext::Global,
        ),
    ));
    let modal =
        ModalDialog::new(WidgetId::from_key("dialog"), "Confirm").with_actions([ModalAction::new(
            action("accept", "Accept"),
            ModalActionRole::Primary,
        )]);
    scene.push(OverlaySceneSurface::modal(ModalDialogOverlay::placed(
        OverlayId::from_raw(14),
        Rect::new(20.0, 150.0, 180.0, 100.0),
        modal,
        OverlayDismissal::Escape,
        ActionContext::Global,
    )));
    for (raw, kind, label, x) in [
        (15, OverlayKind::Popover, "Popover", 220.0),
        (16, OverlayKind::Tooltip, "Tooltip", 340.0),
        (17, OverlayKind::DragPreview, "Drag preview", 460.0),
    ] {
        scene.push(OverlaySceneSurface::passive(
            entry(raw, kind, Rect::new(x, 150.0, 110.0, 60.0)),
            label,
            label,
        ));
    }
    scene.push(OverlaySceneSurface::passive(
        entry(
            18,
            OverlayKind::Tooltip,
            Rect::new(400.0, 240.0, 150.0, 60.0),
        )
        .modal(true),
        "Modal tooltip override",
        "Modal tooltip override",
    ));
    let non_flagged_modal = entry(19, OverlayKind::Modal, Rect::new(220.0, 240.0, 150.0, 60.0));
    assert!(!non_flagged_modal.modal);
    scene.push(OverlaySceneSurface::passive(
        non_flagged_modal,
        "Modal kind without flag",
        "Modal kind without flag",
    ));
    let entries = scene
        .surfaces()
        .iter()
        .map(|surface| surface.entry().clone())
        .collect::<Vec<_>>();
    let mut memory = UiMemory::new();
    let mut theme =
        default_dark_theme().with_radii(RadiusScale::from_values(4.0, 11.0, 23.0, 777.0));
    theme.colors.surface.overlay = Color::rgb8(1, 2, 3);
    theme.colors.overlay.scrim = Color::rgb8(4, 5, 6);
    let (_, _, frame) =
        run_frame_with_theme(&mut scene, &mut memory, UiInput::default(), false, &theme);

    let mut previous = None;
    for entry in &entries {
        let position = frame
            .primitives
            .iter()
            .position(|primitive| {
                matches!(primitive, Primitive::Rect(rect)
                    if rect.rect == entry.rect
                        && rect.fill == Some(Brush::Solid(theme.colors.surface.overlay)))
            })
            .expect("themed overlay surface");
        let Primitive::Rect(surface_primitive) = &frame.primitives[position] else {
            panic!("overlay surface primitive");
        };
        assert_eq!(surface_primitive.radius, theme.radii.md);
        assert_ne!(surface_primitive.radius, theme.radii.lg);
        let matching_shadows = frame
            .primitives
            .iter()
            .enumerate()
            .filter_map(|(index, primitive)| match primitive {
                Primitive::Shadow(shadow) if shadow.rect == entry.rect => Some((index, shadow)),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(matching_shadows.len(), 1, "one shadow per overlay surface");
        let (shadow_position, shadow) = matching_shadows[0];
        assert!(
            shadow_position < position,
            "shadow paints before its surface"
        );
        assert_exact_overlay_shadow(shadow, entry, theme.radii.md.top_left);
        if let Some(previous) = previous {
            assert!(position > previous, "surfaces remain bottom-to-top");
        }
        previous = Some(position);

        let surface = frame
            .semantics
            .get(WidgetId::from_raw(entry.id.raw()))
            .expect("surface semantics");
        assert!(!surface.children.is_empty());
        assert_eq!(surface.bounds, entry.rect);
        for child in &surface.children {
            let child = frame
                .semantics
                .get(*child)
                .expect("owned overlay child semantics");
            assert!(
                child.bounds.intersection(entry.rect).is_some(),
                "owned child remains within its overlay surface"
            );
        }
    }
    assert!(frame.primitives.iter().any(|primitive| {
        matches!(primitive, Primitive::Rect(rect)
        if rect.rect == Rect::new(0.0, 0.0, 640.0, 480.0)
            && rect.fill == Some(Brush::Solid(
                theme
                    .colors
                    .overlay
                    .scrim
                    .with_alpha(theme.opacity.overlay_scrim)
            )))
    }));
    assert!(
        frame
            .semantics
            .nodes()
            .iter()
            .any(|node| node.role == SemanticRole::CommandPalette)
    );
}
