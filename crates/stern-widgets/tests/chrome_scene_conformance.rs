//! Windowless paint, input, overflow, and semantic conformance for chrome scenes.

use std::time::Duration;

use stern_core::{
    ActionContext, ActionDescriptor, ActionId, ActionInvocation, ActionQueue, ActionSource,
    FrameContext, Key, KeyEvent, KeyState, KeyboardInput, Modifiers, PhysicalSize, Point,
    PointerButtonState, PointerInput, PointerOrder, PointerTarget, Primitive, Rect, Response,
    ScaleFactor, SemanticActionKind, SemanticRole, Size, TimeInfo, UiInput, UiMemory, ViewportInfo,
    WidgetId, default_dark_theme,
};
use stern_widgets::{
    ChromeOverflowRequest, ChromeScene, ChromeSceneConfig, ChromeSceneIntent, ChromeSceneItemKey,
    ChromeSurfaceKind, FrameTab, MenuBar, MenuBarMenu, MenuBarMenuId, MenuItem, PanelId, StatusBar,
    StatusItem, StatusItemId, StatusItemKind, TabStrip, Toolbar, ToolbarGroup, ToolbarGroupId, Ui,
};

const LOWER_RECT: Rect = Rect::new(0.0, 0.0, 320.0, 180.0);

fn action(id: &str, label: &str) -> ActionDescriptor {
    ActionDescriptor::new(id, label)
}

fn menu_bar() -> MenuBar {
    MenuBar::from_menus([
        MenuBarMenu::from_actions(
            MenuBarMenuId::from_raw(1),
            "File",
            [action("file.open", "Open")],
        ),
        MenuBarMenu::from_actions(
            MenuBarMenuId::from_raw(2),
            "Edit",
            [action("edit.copy", "Copy")],
        ),
    ])
}

fn toolbar() -> Toolbar {
    toolbar_from([
        action("file.open", "Open"),
        action("file.save", "Save"),
        action("file.export", "Export"),
    ])
}

fn toolbar_from(actions: impl IntoIterator<Item = ActionDescriptor>) -> Toolbar {
    Toolbar::from_groups([ToolbarGroup::from_actions(
        ToolbarGroupId::from_raw(10),
        "File",
        actions,
    )])
}

fn tab(panel: u64, title: &str, active: bool, close_visible: bool) -> FrameTab {
    FrameTab {
        panel: PanelId::from_raw(panel),
        title: title.to_owned(),
        active,
        close_visible,
        draggable: true,
    }
}

fn tab_strip() -> TabStrip {
    TabStrip::from_tabs([
        tab(100, "Scene", true, true),
        tab(101, "Inspector", false, false),
    ])
}

fn status_bar() -> StatusBar {
    StatusBar::from_items([
        StatusItem::new(
            StatusItemId::from_raw(20),
            "State",
            "Ready",
            StatusItemKind::Ready,
        ),
        StatusItem::new(
            StatusItemId::from_raw(21),
            "Jobs",
            "2 jobs",
            StatusItemKind::JobCount,
        ),
    ])
}

fn config(width: f32) -> ChromeSceneConfig {
    ChromeSceneConfig::new(
        WidgetId::from_key("chrome-test"),
        Rect::new(0.0, 0.0, width, 28.0),
        Rect::new(0.0, 32.0, width, 28.0),
        Rect::new(0.0, 64.0, width, 28.0),
        Rect::new(0.0, 96.0, width, 28.0),
        ActionContext::Editor,
    )
    .with_overflow_trigger_width(20.0)
    .with_tab_close_width(18.0)
    .with_widths([
        (ChromeSceneItemKey::Menu(MenuBarMenuId::from_raw(1)), 50.0),
        (ChromeSceneItemKey::Menu(MenuBarMenuId::from_raw(2)), 50.0),
        (
            ChromeSceneItemKey::Toolbar {
                group: ToolbarGroupId::from_raw(10),
                action: ActionId::new("file.open"),
            },
            50.0,
        ),
        (
            ChromeSceneItemKey::Toolbar {
                group: ToolbarGroupId::from_raw(10),
                action: ActionId::new("file.save"),
            },
            50.0,
        ),
        (
            ChromeSceneItemKey::Toolbar {
                group: ToolbarGroupId::from_raw(10),
                action: ActionId::new("file.export"),
            },
            50.0,
        ),
        (ChromeSceneItemKey::Tab(PanelId::from_raw(100)), 70.0),
        (ChromeSceneItemKey::Tab(PanelId::from_raw(101)), 70.0),
        (ChromeSceneItemKey::Status(StatusItemId::from_raw(20)), 50.0),
        (ChromeSceneItemKey::Status(StatusItemId::from_raw(21)), 50.0),
    ])
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

fn run_frame(
    scene: &ChromeScene<'_>,
    memory: &mut UiMemory,
    input: UiInput,
    lower: bool,
) -> (
    Option<Response>,
    stern_widgets::ChromeSceneOutput,
    stern_core::FrameOutput,
) {
    let theme = default_dark_theme();
    let mut ui = Ui::begin_frame(context(input), memory, &theme);
    let lower_id = ui.make_id("lower");
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
    .expect("valid shared pointer plan");
    let lower_response = lower.then(|| ui.pressable("lower", LOWER_RECT, false));
    let output = ui.chrome_scene(scene);
    let frame = ui.finish_output();
    (lower_response, output, frame)
}

fn toolbar_overflow_request(scene: &ChromeScene<'_>) -> ChromeOverflowRequest {
    let mut memory = UiMemory::new();
    let _ = run_frame(scene, &mut memory, pressed_at(55.0, 40.0), false);
    let (_, output, _) = run_frame(scene, &mut memory, released_at(55.0, 40.0), false);
    let [ChromeSceneIntent::OpenOverflow(request)] = output.intents.as_slice() else {
        panic!("one toolbar overflow request");
    };
    request.clone()
}

#[test]
fn full_width_paints_all_surfaces_in_order_with_stable_semantics() {
    let menu = menu_bar();
    let toolbar = toolbar();
    let tabs = tab_strip();
    let status = status_bar();
    let scene = ChromeScene::new(config(240.0), &menu, &toolbar, &tabs, &status);
    let mut memory = UiMemory::new();

    let (_, output, frame) = run_frame(&scene, &mut memory, UiInput::default(), false);

    assert_eq!(output.responses.len(), 8);
    assert!(output.intents.is_empty());
    let mut previous = None;
    for kind in [
        ChromeSurfaceKind::MenuBar,
        ChromeSurfaceKind::Toolbar,
        ChromeSurfaceKind::TabStrip,
        ChromeSurfaceKind::StatusBar,
    ] {
        let id = scene.surface_widget_id(kind);
        let node = frame.semantics.get(id).expect("surface semantics");
        assert!(!node.children.is_empty());
        let position = frame
            .primitives
            .iter()
            .position(|primitive| matches!(primitive, Primitive::ClipBegin { id: clip, .. } if clip.raw() == id.child("clip").raw()))
            .expect("surface clip");
        if let Some(previous) = previous {
            assert!(position > previous);
        }
        previous = Some(position);
        assert!(
            frame
                .semantics
                .get(scene.overflow_widget_id(kind))
                .is_none()
        );
    }
    assert_eq!(
        frame
            .semantics
            .get(scene.surface_widget_id(ChromeSurfaceKind::TabStrip))
            .expect("tab list")
            .role,
        SemanticRole::TabList
    );
    let file = frame
        .semantics
        .get(scene.item_widget_id(&ChromeSceneItemKey::Menu(MenuBarMenuId::from_raw(1))))
        .expect("menu heading");
    assert_eq!(file.role, SemanticRole::MenuItem);
    assert!(
        file.actions
            .iter()
            .any(|action| action.kind == SemanticActionKind::Open)
    );
}

#[test]
fn chrome_surface_blocks_lower_input_and_menu_click_returns_anchor() {
    let menu = menu_bar();
    let toolbar = toolbar();
    let tabs = tab_strip();
    let status = status_bar();
    let scene = ChromeScene::new(config(240.0), &menu, &toolbar, &tabs, &status);
    let mut memory = UiMemory::new();

    let (lower_press, _, _) = run_frame(&scene, &mut memory, pressed_at(10.0, 10.0), true);
    assert!(!lower_press.expect("lower").state.hovered);
    let (lower_release, output, _) = run_frame(&scene, &mut memory, released_at(10.0, 10.0), true);
    assert!(!lower_release.expect("lower").clicked);
    assert_eq!(
        output.intents,
        vec![ChromeSceneIntent::OpenMenu {
            menu: MenuBarMenuId::from_raw(1),
            anchor: Rect::new(0.0, 0.0, 50.0, 28.0),
        }]
    );
}

#[test]
fn toolbar_mouse_and_keyboard_queue_the_same_action_exactly_once() {
    let menu = menu_bar();
    let toolbar = toolbar();
    let tabs = tab_strip();
    let status = status_bar();
    let scene = ChromeScene::new(config(240.0), &menu, &toolbar, &tabs, &status);
    let key = ChromeSceneItemKey::Toolbar {
        group: ToolbarGroupId::from_raw(10),
        action: ActionId::new("file.open"),
    };

    let mut mouse_memory = UiMemory::new();
    let _ = run_frame(&scene, &mut mouse_memory, pressed_at(10.0, 40.0), false);
    let (_, mouse_output, mut mouse_frame) =
        run_frame(&scene, &mut mouse_memory, released_at(10.0, 40.0), false);

    let mut keyboard_memory = UiMemory::new();
    keyboard_memory.focus(scene.item_widget_id(&key));
    let (_, keyboard_output, mut keyboard_frame) =
        run_frame(&scene, &mut keyboard_memory, pressed_key(Key::Enter), false);

    assert_eq!(mouse_output.intents.len(), 1);
    assert_eq!(keyboard_output.intents.len(), 1);
    let ChromeSceneIntent::Action(mouse) = &mouse_output.intents[0] else {
        panic!("mouse toolbar action");
    };
    let ChromeSceneIntent::Action(keyboard) = &keyboard_output.intents[0] else {
        panic!("keyboard toolbar action");
    };
    assert_eq!(mouse, keyboard);
    assert_eq!(mouse.action_id, ActionId::new("file.open"));
    assert_eq!(mouse.source, ActionSource::Button);
    assert_eq!(mouse.context, ActionContext::Editor);
    assert_eq!(mouse_frame.actions.len(), 1);
    assert_eq!(keyboard_frame.actions.len(), 1);
    assert_eq!(mouse_frame.actions.pop_front(), Some(mouse.clone()));
    assert_eq!(keyboard_frame.actions.pop_front(), Some(keyboard.clone()));
}

#[test]
fn compact_surfaces_expose_typed_source_order_overflow_requests() {
    let menu = menu_bar();
    let toolbar = toolbar();
    let tabs = tab_strip();
    let status = status_bar();
    let scene = ChromeScene::new(config(70.0), &menu, &toolbar, &tabs, &status);
    let cases = [
        (
            ChromeSurfaceKind::MenuBar,
            Point::new(55.0, 10.0),
            vec![ChromeSceneItemKey::Menu(MenuBarMenuId::from_raw(2))],
        ),
        (
            ChromeSurfaceKind::Toolbar,
            Point::new(55.0, 40.0),
            vec![
                ChromeSceneItemKey::Toolbar {
                    group: ToolbarGroupId::from_raw(10),
                    action: ActionId::new("file.save"),
                },
                ChromeSceneItemKey::Toolbar {
                    group: ToolbarGroupId::from_raw(10),
                    action: ActionId::new("file.export"),
                },
            ],
        ),
        (
            ChromeSurfaceKind::TabStrip,
            Point::new(5.0, 74.0),
            vec![
                ChromeSceneItemKey::Tab(PanelId::from_raw(100)),
                ChromeSceneItemKey::Tab(PanelId::from_raw(101)),
            ],
        ),
        (
            ChromeSurfaceKind::StatusBar,
            Point::new(55.0, 105.0),
            vec![ChromeSceneItemKey::Status(StatusItemId::from_raw(21))],
        ),
    ];

    let mut initial_memory = UiMemory::new();
    let (_, _, initial) = run_frame(&scene, &mut initial_memory, UiInput::default(), false);
    for kind in [
        ChromeSurfaceKind::MenuBar,
        ChromeSurfaceKind::Toolbar,
        ChromeSurfaceKind::TabStrip,
        ChromeSurfaceKind::StatusBar,
    ] {
        assert!(
            initial
                .semantics
                .get(scene.overflow_widget_id(kind))
                .is_some()
        );
    }

    for (kind, point, expected) in cases {
        let mut memory = UiMemory::new();
        let _ = run_frame(&scene, &mut memory, pressed_at(point.x, point.y), false);
        let (_, output, _) = run_frame(&scene, &mut memory, released_at(point.x, point.y), false);
        let ChromeSceneIntent::OpenOverflow(request) = &output.intents[0] else {
            panic!("overflow request");
        };
        assert_eq!(request.surface, kind);
        assert_eq!(request.items, expected);
        assert!(request.trigger_rect.width > 0.0);
    }
}

#[test]
fn toolbar_overflow_projects_shared_descriptors_and_menu_activation() {
    let mut save = action("file.save", "Save");
    save.state.checked = Some(true);
    let mut export = action("file.export", "Export");
    export.state.enabled = false;
    let toolbar = toolbar_from([action("file.open", "Open"), save.clone(), export.clone()]);
    let menu_bar = menu_bar();
    let tabs = tab_strip();
    let status = status_bar();
    let scene = ChromeScene::new(config(70.0), &menu_bar, &toolbar, &tabs, &status);
    let request = toolbar_overflow_request(&scene);

    let projected = scene
        .toolbar_overflow_menu(&request)
        .expect("current toolbar overflow menu");
    assert_eq!(
        projected
            .visible_items()
            .into_iter()
            .cloned()
            .collect::<Vec<_>>(),
        vec![MenuItem::Action(save), MenuItem::Action(export)]
    );
    let mut queue = ActionQueue::new();
    assert!(projected.invoke_visible(0, &mut queue, ActionContext::Editor));
    assert!(!projected.invoke_visible(1, &mut queue, ActionContext::Editor));
    assert_eq!(
        queue.pop_front(),
        Some(ActionInvocation::new(
            ActionId::new("file.save"),
            ActionSource::Menu,
            ActionContext::Editor,
        ))
    );
    assert!(queue.is_empty());
}

#[test]
fn toolbar_overflow_projection_fails_closed_for_noncurrent_requests() {
    let menu_bar = menu_bar();
    let toolbar = toolbar();
    let tabs = tab_strip();
    let status = status_bar();
    let scene = ChromeScene::new(config(70.0), &menu_bar, &toolbar, &tabs, &status);
    let current = toolbar_overflow_request(&scene);

    let mut invalid = Vec::new();
    for mutate in [
        |request: &mut ChromeOverflowRequest| request.surface = ChromeSurfaceKind::MenuBar,
        |request: &mut ChromeOverflowRequest| request.items.clear(),
        |request: &mut ChromeOverflowRequest| {
            request.items[0] = ChromeSceneItemKey::Menu(MenuBarMenuId::from_raw(1));
        },
        |request: &mut ChromeOverflowRequest| {
            request.items[0] = ChromeSceneItemKey::Toolbar {
                group: ToolbarGroupId::from_raw(99),
                action: ActionId::new("missing"),
            };
        },
        |request: &mut ChromeOverflowRequest| {
            let duplicate = request.items[0].clone();
            request.items[1] = duplicate;
        },
        |request: &mut ChromeOverflowRequest| request.items.swap(0, 1),
    ] {
        let mut request = current.clone();
        mutate(&mut request);
        invalid.push(request);
    }
    assert!(
        invalid
            .iter()
            .all(|request| scene.toolbar_overflow_menu(request).is_none())
    );

    let wide = ChromeScene::new(config(200.0), &menu_bar, &toolbar, &tabs, &status);
    let reordered_toolbar = toolbar_from([
        action("file.open", "Open"),
        action("file.export", "Export"),
        action("file.save", "Save"),
    ]);
    let reordered = ChromeScene::new(config(70.0), &menu_bar, &reordered_toolbar, &tabs, &status);
    let mut hidden = action("file.export", "Export");
    hidden.state.visible = false;
    let hidden_toolbar = toolbar_from([
        action("file.open", "Open"),
        action("file.save", "Save"),
        hidden,
    ]);
    let hidden = ChromeScene::new(config(70.0), &menu_bar, &hidden_toolbar, &tabs, &status);
    for stale_scene in [&wide, &reordered, &hidden] {
        assert!(stale_scene.toolbar_overflow_menu(&current).is_none());
    }
}

#[test]
fn tab_body_and_close_emit_distinct_stable_targets() {
    let menu = menu_bar();
    let toolbar = toolbar();
    let tabs = tab_strip();
    let status = status_bar();
    let scene = ChromeScene::new(config(240.0), &menu, &toolbar, &tabs, &status);

    let mut activate_memory = UiMemory::new();
    let _ = run_frame(&scene, &mut activate_memory, pressed_at(10.0, 72.0), false);
    let (_, activated, _) = run_frame(&scene, &mut activate_memory, released_at(10.0, 72.0), false);
    assert!(matches!(
        activated.intents.as_slice(),
        [ChromeSceneIntent::ActivateTab(target)] if target.panel == PanelId::from_raw(100) && target.index == 0
    ));

    let mut close_memory = UiMemory::new();
    let _ = run_frame(&scene, &mut close_memory, pressed_at(65.0, 72.0), false);
    let (_, closed, frame) = run_frame(&scene, &mut close_memory, released_at(65.0, 72.0), false);
    assert!(matches!(
        closed.intents.as_slice(),
        [ChromeSceneIntent::CloseTab(target)] if target.panel == PanelId::from_raw(100) && target.index == 0
    ));
    assert!(
        frame
            .semantics
            .get(scene.tab_close_widget_id(PanelId::from_raw(100)))
            .expect("close semantics")
            .label
            .as_deref()
            .is_some_and(|label| label == "Close Scene")
    );
}

#[test]
fn hidden_and_disabled_toolbar_items_are_inert() {
    let mut hidden = action("hidden", "Hidden");
    hidden.state.visible = false;
    let mut disabled = action("disabled", "Disabled");
    disabled.state.enabled = false;
    let toolbar = Toolbar::from_groups([ToolbarGroup::from_actions(
        ToolbarGroupId::from_raw(11),
        "State",
        [hidden, disabled],
    )]);
    let menu = MenuBar::new();
    let tabs = TabStrip::new();
    let status = StatusBar::new();
    let config = ChromeSceneConfig::new(
        WidgetId::from_key("inert-chrome"),
        Rect::ZERO,
        Rect::new(0.0, 0.0, 100.0, 28.0),
        Rect::ZERO,
        Rect::ZERO,
        ActionContext::Global,
    )
    .with_widths([
        (
            ChromeSceneItemKey::Toolbar {
                group: ToolbarGroupId::from_raw(11),
                action: ActionId::new("hidden"),
            },
            50.0,
        ),
        (
            ChromeSceneItemKey::Toolbar {
                group: ToolbarGroupId::from_raw(11),
                action: ActionId::new("disabled"),
            },
            50.0,
        ),
    ]);
    let scene = ChromeScene::new(config, &menu, &toolbar, &tabs, &status);
    let mut memory = UiMemory::new();

    let (_, output, frame) = run_frame(&scene, &mut memory, released_at(10.0, 10.0), false);

    assert!(output.intents.is_empty());
    assert!(frame.actions.is_empty());
    let hidden_id = scene.item_widget_id(&ChromeSceneItemKey::Toolbar {
        group: ToolbarGroupId::from_raw(11),
        action: ActionId::new("hidden"),
    });
    assert!(frame.semantics.get(hidden_id).is_none());
    let disabled_id = scene.item_widget_id(&ChromeSceneItemKey::Toolbar {
        group: ToolbarGroupId::from_raw(11),
        action: ActionId::new("disabled"),
    });
    let disabled = frame.semantics.get(disabled_id).expect("disabled item");
    assert!(disabled.state.disabled);
    assert!(!disabled.focusable);
}

#[test]
fn item_ids_survive_model_reorder_and_removal() {
    let first = menu_bar();
    let reordered = MenuBar::from_menus([
        MenuBarMenu::from_actions(
            MenuBarMenuId::from_raw(2),
            "Edit",
            [action("edit.copy", "Copy")],
        ),
        MenuBarMenu::from_actions(
            MenuBarMenuId::from_raw(1),
            "File",
            [action("file.open", "Open")],
        ),
    ]);
    let removed = MenuBar::from_menus([MenuBarMenu::from_actions(
        MenuBarMenuId::from_raw(1),
        "File",
        [action("file.open", "Open")],
    )]);
    let toolbar = Toolbar::new();
    let tabs = TabStrip::new();
    let status = StatusBar::new();
    let first_scene = ChromeScene::new(config(240.0), &first, &toolbar, &tabs, &status);
    let second_scene = ChromeScene::new(config(240.0), &reordered, &toolbar, &tabs, &status);
    let removed_scene = ChromeScene::new(config(240.0), &removed, &toolbar, &tabs, &status);
    let file = ChromeSceneItemKey::Menu(MenuBarMenuId::from_raw(1));
    let edit = ChromeSceneItemKey::Menu(MenuBarMenuId::from_raw(2));

    assert_eq!(
        first_scene.item_widget_id(&file),
        second_scene.item_widget_id(&file)
    );
    assert_eq!(
        first_scene.item_widget_id(&edit),
        second_scene.item_widget_id(&edit)
    );
    assert_eq!(
        first_scene.item_widget_id(&file),
        removed_scene.item_widget_id(&file)
    );

    let mut first_memory = UiMemory::new();
    let (_, _, first_frame) = run_frame(&first_scene, &mut first_memory, UiInput::default(), false);
    let mut second_memory = UiMemory::new();
    let (_, _, second_frame) =
        run_frame(&second_scene, &mut second_memory, UiInput::default(), false);
    let mut removed_memory = UiMemory::new();
    let (_, _, removed_frame) = run_frame(
        &removed_scene,
        &mut removed_memory,
        UiInput::default(),
        false,
    );
    assert!(
        first_frame
            .semantics
            .get(first_scene.item_widget_id(&file))
            .is_some()
    );
    assert!(
        second_frame
            .semantics
            .get(second_scene.item_widget_id(&file))
            .is_some()
    );
    assert!(
        removed_frame
            .semantics
            .get(removed_scene.item_widget_id(&file))
            .is_some()
    );
    assert!(
        removed_frame
            .semantics
            .get(removed_scene.item_widget_id(&edit))
            .is_none()
    );
}

#[test]
fn invalid_or_empty_surface_bounds_emit_nothing_and_do_not_capture_lower_input() {
    let menu = menu_bar();
    let toolbar = toolbar();
    let tabs = tab_strip();
    let status = status_bar();
    let config = ChromeSceneConfig::new(
        WidgetId::from_key("invalid-chrome"),
        Rect::new(f32::NAN, 0.0, 100.0, 28.0),
        Rect::new(0.0, 0.0, -1.0, 28.0),
        Rect::new(f32::MAX, 0.0, f32::MAX, 28.0),
        Rect::ZERO,
        ActionContext::Global,
    );
    let scene = ChromeScene::new(config, &menu, &toolbar, &tabs, &status);
    let mut memory = UiMemory::new();

    let (lower, output, frame) = run_frame(&scene, &mut memory, pressed_at(10.0, 10.0), true);

    assert!(lower.expect("lower").state.hovered);
    assert!(output.responses.is_empty());
    assert!(output.intents.is_empty());
    assert!(frame.primitives.is_empty());
    assert!(frame.semantics.is_empty());
}

fn assert_chrome_button_focus(frame: &stern_core::FrameOutput, rect: Rect) {
    let theme = default_dark_theme();
    let base = frame
        .primitives
        .iter()
        .position(|primitive| {
            matches!(primitive, Primitive::Rect(base) if base.rect == rect && base.stroke.is_some())
        })
        .expect("chrome button base");
    let Primitive::Rect(base_surface) = &frame.primitives[base] else {
        unreachable!()
    };
    assert_eq!(
        base_surface.stroke.expect("neutral border").brush,
        stern_core::Brush::Solid(theme.colors.border.default)
    );
    for (primitive, brush) in [
        (
            &frame.primitives[base + 1],
            theme.focus_ring(true).unwrap().primary.brush,
        ),
        (
            &frame.primitives[base + 2],
            theme.focus_ring(true).unwrap().separator.brush,
        ),
    ] {
        let Primitive::Path(path) = primitive else {
            panic!("chrome focus must be an inward compound path");
        };
        assert_eq!(path.fill, Some(brush));
        assert_eq!(path.stroke, None);
        for point in path.elements.iter().flat_map(|element| match *element {
            stern_core::PathElement::MoveTo(point) | stern_core::PathElement::LineTo(point) => {
                vec![point]
            }
            stern_core::PathElement::QuadTo { ctrl, to } => vec![ctrl, to],
            stern_core::PathElement::CubicTo { ctrl1, ctrl2, to } => vec![ctrl1, ctrl2, to],
            stern_core::PathElement::Close => Vec::new(),
        }) {
            assert!(point.x >= rect.min_x() && point.x <= rect.max_x());
            assert!(point.y >= rect.min_y() && point.y <= rect.max_y());
        }
    }
    assert!(matches!(frame.primitives[base + 3], Primitive::Text(_)));
}

#[test]
fn all_four_chrome_button_row_kinds_use_clip_contained_inward_focus() {
    let menu = menu_bar();
    let toolbar = toolbar();
    let tabs = tab_strip();
    let status = status_bar();
    let full = ChromeScene::new(config(240.0), &menu, &toolbar, &tabs, &status);
    let full_ids = [
        full.item_widget_id(&ChromeSceneItemKey::Menu(MenuBarMenuId::from_raw(1))),
        full.item_widget_id(&ChromeSceneItemKey::Toolbar {
            group: ToolbarGroupId::from_raw(10),
            action: ActionId::new("file.open"),
        }),
        full.tab_close_widget_id(PanelId::from_raw(100)),
    ];
    for id in full_ids {
        let mut memory = UiMemory::new();
        memory.focus(id);
        let (_, _, frame) = run_frame(&full, &mut memory, UiInput::default(), false);
        let rect = frame.semantics.get(id).expect("focused chrome row").bounds;
        assert_chrome_button_focus(&frame, rect);
    }

    let compact = ChromeScene::new(config(70.0), &menu, &toolbar, &tabs, &status);
    let overflow = compact.overflow_widget_id(ChromeSurfaceKind::Toolbar);
    let mut memory = UiMemory::new();
    memory.focus(overflow);
    let (_, _, frame) = run_frame(&compact, &mut memory, UiInput::default(), false);
    let rect = frame.semantics.get(overflow).expect("overflow row").bounds;
    assert!((rect.max_x() - 70.0).abs() <= f32::EPSILON);
    assert_chrome_button_focus(&frame, rect);
}

fn assert_chrome_tab_focus_pair(
    focused: &stern_core::FrameOutput,
    unfocused: &stern_core::FrameOutput,
    id: WidgetId,
    strip_rect: Rect,
    selected: bool,
) {
    let theme = default_dark_theme();
    let rect = focused.semantics.get(id).expect("focused tab").bounds;
    let base = focused
        .primitives
        .iter()
        .position(|primitive| {
            matches!(primitive, Primitive::Rect(base) if base.rect == rect && base.stroke.is_some())
        })
        .expect("chrome tab base");
    let unfocused_base = unfocused
        .primitives
        .iter()
        .position(|primitive| {
            matches!(primitive, Primitive::Rect(base) if base.rect == rect && base.stroke.is_some())
        })
        .expect("unfocused chrome tab base");
    assert_eq!(
        focused.primitives[base],
        unfocused.primitives[unfocused_base]
    );
    let Primitive::Rect(surface) = &focused.primitives[base] else {
        unreachable!()
    };
    assert_eq!(surface.radius, theme.radii.none);
    assert_eq!(
        surface.stroke.expect("neutral border").brush,
        stern_core::Brush::Solid(theme.colors.border.default)
    );
    assert_eq!(
        surface.fill,
        Some(stern_core::Brush::Solid(if selected {
            theme.colors.surface.control_pressed
        } else {
            theme.colors.surface.panel
        }))
    );
    let expected = theme
        .focus_ring(true)
        .expect("focus recipe")
        .inward_annulus_primitives(
            rect,
            surface.radius,
            surface.stroke.expect("neutral border").width,
        );
    assert_eq!(focused.primitives[base + 1], expected[0]);
    assert_eq!(focused.primitives[base + 2], expected[1]);
    for primitive in &focused.primitives[base + 1..base + 3] {
        let Primitive::Path(path) = primitive else {
            panic!("chrome tab focus path");
        };
        assert_eq!(path.stroke, None);
        for point in path.elements.iter().flat_map(|element| match *element {
            stern_core::PathElement::MoveTo(point) | stern_core::PathElement::LineTo(point) => {
                vec![point]
            }
            stern_core::PathElement::QuadTo { ctrl, to } => vec![ctrl, to],
            stern_core::PathElement::CubicTo { ctrl1, ctrl2, to } => vec![ctrl1, ctrl2, to],
            stern_core::PathElement::Close => Vec::new(),
        }) {
            assert!(point.x >= rect.min_x() && point.x <= rect.max_x());
            assert!(point.y >= rect.min_y() && point.y <= rect.max_y());
            assert!(point.x >= strip_rect.min_x() && point.x <= strip_rect.max_x());
            assert!(point.y >= strip_rect.min_y() && point.y <= strip_rect.max_y());
        }
    }
    assert!(matches!(focused.primitives[base + 3], Primitive::Text(_)));
    let mut stripped = focused.primitives.clone();
    stripped.drain(base + 1..base + 3);
    assert_eq!(stripped, unfocused.primitives);
    let focused_node = focused.semantics.get(id).expect("focused tab semantic");
    let unfocused_node = unfocused.semantics.get(id).expect("unfocused tab semantic");
    assert!(focused_node.state.focused);
    assert_eq!(focused_node.role, SemanticRole::Tab);
    assert_eq!(focused_node.bounds, unfocused_node.bounds);
    assert_eq!(focused_node.state.selected, unfocused_node.state.selected);
}

#[test]
fn first_last_and_close_truncated_chrome_tabs_use_neutral_contained_inward_focus() {
    let menu = menu_bar();
    let toolbar = toolbar();
    let tabs = tab_strip();
    let status = status_bar();
    let scene = ChromeScene::new(config(140.0), &menu, &toolbar, &tabs, &status);
    let strip_id = scene.surface_widget_id(ChromeSurfaceKind::TabStrip);
    let mut memory = UiMemory::new();
    let (_, _, unfocused) = run_frame(&scene, &mut memory, UiInput::default(), false);
    let strip_rect = unfocused.semantics.get(strip_id).expect("tab strip").bounds;
    let cases = [
        (
            scene.item_widget_id(&ChromeSceneItemKey::Tab(PanelId::from_raw(100))),
            true,
        ),
        (
            scene.item_widget_id(&ChromeSceneItemKey::Tab(PanelId::from_raw(101))),
            false,
        ),
    ];
    let first_rect = unfocused
        .semantics
        .get(cases[0].0)
        .expect("first tab")
        .bounds;
    let last_rect = unfocused
        .semantics
        .get(cases[1].0)
        .expect("last tab")
        .bounds;
    assert!((first_rect.min_x() - strip_rect.min_x()).abs() <= f32::EPSILON);
    assert!(
        first_rect.width < 70.0,
        "close affordance truncates the body"
    );
    assert!((last_rect.max_x() - strip_rect.max_x()).abs() <= f32::EPSILON);
    assert!(first_rect.max_x() <= last_rect.min_x());

    for (id, selected) in cases {
        let mut memory = UiMemory::new();
        memory.focus(id);
        let (_, _, focused) = run_frame(&scene, &mut memory, UiInput::default(), false);
        assert_chrome_tab_focus_pair(&focused, &unfocused, id, strip_rect, selected);
    }
    assert!(unfocused.primitives.iter().all(|primitive| {
        !matches!(
            primitive,
            Primitive::Rect(rect)
                if rect.fill == Some(stern_core::Brush::Solid(default_dark_theme().colors.accent.default))
        )
    }));
}
