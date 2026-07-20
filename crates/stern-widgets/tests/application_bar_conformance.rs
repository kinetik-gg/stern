//! Windowless application-bar composition conformance.
use stern_core::{
    ActionDescriptor, Brush, FrameContext, Key, KeyEvent, KeyState, KeyboardInput, Modifiers,
    PhysicalSize, Point, PointerButtonState, PointerInput, PointerOrder, PointerTarget, Primitive,
    Rect, ScaleFactor, SemanticActionKind, SemanticRole, Size, Theme, TimeInfo, UiInput,
    UiInputEvent, UiMemory, ViewportInfo, WidgetId, default_dark_theme,
};
use stern_widgets::{
    ApplicationBar, ApplicationBarConfig, ApplicationBarIntent, MenuBar, MenuBarMenu,
    MenuBarMenuId, PreparedApplicationBar, Ui, WorkspaceTab, WorkspaceTabId,
};
const FILE: MenuBarMenuId = MenuBarMenuId::from_raw(1);
const VIEW: MenuBarMenuId = MenuBarMenuId::from_raw(3);
const W1: WorkspaceTabId = WorkspaceTabId::from_raw(10);
const W2: WorkspaceTabId = WorkspaceTabId::from_raw(20);
const W3: WorkspaceTabId = WorkspaceTabId::from_raw(30);
fn bar() -> ApplicationBar {
    let mut config = ApplicationBarConfig::new(
        WidgetId::from_key("app-bar"),
        Rect::new(0.25, 0.5, 360.5, 40.0),
    );
    config.menu_width = 50.25;
    config.workspace_width = 70.0;
    let mut disabled = WorkspaceTab::new(W2, "Editing", false);
    disabled.enabled = false;
    ApplicationBar::new(
        config,
        MenuBar::from_menus([
            MenuBarMenu::from_actions(FILE, "File", [ActionDescriptor::new("open", "Open")]),
            MenuBarMenu::from_actions(MenuBarMenuId::from_raw(2), "Empty", []),
            MenuBarMenu::from_actions(VIEW, "View", [ActionDescriptor::new("grid", "Grid")]),
        ]),
        [
            WorkspaceTab::new(W1, "Compositing", true),
            disabled,
            WorkspaceTab::new(W3, "Grading", false),
        ],
    )
}
fn pointer(point: Point, down: Option<bool>) -> UiInput {
    UiInput {
        pointer: PointerInput {
            position: Some(point),
            primary: down.map_or(PointerButtonState::default(), |down| {
                PointerButtonState::new(down, down, !down)
            }),
            ..PointerInput::default()
        },
        ..UiInput::default()
    }
}
fn key(key: Key) -> UiInput {
    UiInput {
        keyboard: KeyboardInput {
            events: vec![KeyEvent::new(
                key,
                KeyState::Pressed,
                Modifiers::default(),
                false,
            )],
            ..KeyboardInput::default()
        },
        ..UiInput::default()
    }
}
fn cancel(event: UiInputEvent) -> UiInput {
    let mut input = UiInput::default();
    input.push_event(event);
    input
}
fn run(
    bar: &mut ApplicationBar,
    memory: &mut UiMemory,
    input: UiInput,
) -> (stern_widgets::ApplicationBarOutput, stern_core::FrameOutput) {
    let theme = default_dark_theme();
    run_with_theme(bar, memory, input, &theme)
}
fn run_with_theme(
    bar: &mut ApplicationBar,
    memory: &mut UiMemory,
    input: UiInput,
    theme: &Theme,
) -> (stern_widgets::ApplicationBarOutput, stern_core::FrameOutput) {
    let context = FrameContext::new(
        ViewportInfo::new(
            Size::new(500.0, 200.0),
            PhysicalSize::new(500, 200),
            ScaleFactor::ONE,
        ),
        input,
        TimeInfo::default(),
    );
    let prepared = bar.prepare(theme);
    let mut ui = Ui::begin_frame(context, memory, theme);
    ui.resolve_pointer_targets(|plan| {
        if let Some(prepared) = &prepared {
            prepared.declare_pointer_targets(bar, theme, plan, PointerOrder::new(10));
        }
    })
    .unwrap();
    let output = prepared.as_ref().map_or_else(Default::default, |prepared| {
        ui.application_bar(bar, prepared)
    });
    (output, ui.finish_output())
}
fn assert_preparation_mismatch_is_inert(
    bar: &mut ApplicationBar,
    prepared: &PreparedApplicationBar,
    active_theme: &Theme,
) {
    let input = pointer(Point::new(325.0, 10.0), Some(true));
    let context = FrameContext::new(
        ViewportInfo::new(
            Size::new(500.0, 200.0),
            PhysicalSize::new(500, 200),
            ScaleFactor::ONE,
        ),
        input,
        TimeInfo::default(),
    );
    let probe_id = WidgetId::from_key("application-bar-mismatch-probe");
    let probe_rect = Rect::new(0.0, 0.0, 500.0, 200.0);
    let mut memory = UiMemory::new();
    let mut ui = Ui::begin_frame(context, &mut memory, active_theme);
    let mut next_order = PointerOrder::new(u64::MAX);
    ui.resolve_pointer_targets(|plan| {
        plan.target(PointerTarget::new(
            probe_id,
            probe_rect,
            PointerOrder::new(0),
        ));
        next_order =
            prepared.declare_pointer_targets(bar, active_theme, plan, PointerOrder::new(10));
    })
    .unwrap();
    let probe = ui.pressable_with_id(probe_id, probe_rect, false);
    let output = ui.application_bar(bar, prepared);
    let frame = ui.finish_output();
    assert_eq!(next_order, PointerOrder::new(10));
    assert!(probe.state.hovered && probe.state.pressed);
    assert!(output.responses.is_empty());
    assert!(output.intents.is_empty() && output.drag_safe_regions.is_empty());
    assert!(frame.primitives.is_empty() && frame.semantics.is_empty());
}
#[test]
fn composition_geometry_semantics_and_ids_are_exact() {
    let theme = default_dark_theme();
    let mut bar = bar();
    let w3 = bar.workspace_widget_id(W3);
    let (output, frame) = run(&mut bar, &mut UiMemory::new(), UiInput::default());
    assert_eq!(
        bar.config.bounds.height.to_bits(),
        theme.sizes.workspace_bar.to_bits()
    );
    assert_eq!(
        output.drag_safe_regions,
        vec![Rect::new(100.75, 0.5, 50.0, 40.0)]
    );
    assert!(
        matches!(&frame.primitives[0], Primitive::Rect(rect) if rect.fill == Some(Brush::Solid(theme.colors.surface.application)))
    );
    let root = frame.semantics.get(bar.config.root).unwrap();
    assert_eq!(
        (&root.role, root.label.as_deref()),
        (
            &SemanticRole::Custom("application-bar".into()),
            Some("Application bar")
        )
    );
    assert_eq!(root.children.len(), 2);
    let menu = frame.semantics.get(root.children[0]).unwrap();
    let workspaces = frame.semantics.get(root.children[1]).unwrap();
    assert_eq!(menu.bounds, Rect::new(0.25, 0.5, 100.5, 40.0));
    assert_eq!(workspaces.bounds, Rect::new(150.75, 0.5, 210.0, 40.0));
    assert_eq!(
        menu.children,
        vec![bar.menu_widget_id(FILE), bar.menu_widget_id(VIEW)]
    );
    assert_eq!(
        workspaces.children,
        vec![
            bar.workspace_widget_id(W1),
            bar.workspace_widget_id(W2),
            bar.workspace_widget_id(W3),
        ]
    );
    assert!(menu.bounds.max_x() <= workspaces.bounds.x);
    assert!(root.bounds.contains_rect(menu.bounds) && root.bounds.contains_rect(workspaces.bounds));
    assert!(menu.children.iter().all(|id| {
        menu.bounds
            .contains_rect(frame.semantics.get(*id).unwrap().bounds)
    }));
    assert!(workspaces.children.iter().all(|id| {
        workspaces
            .bounds
            .contains_rect(frame.semantics.get(*id).unwrap().bounds)
    }));
    assert_eq!(
        (&menu.role, menu.label.as_deref()),
        (
            &SemanticRole::Custom("menu-bar".into()),
            Some("Application menu")
        )
    );
    assert_eq!(
        (&workspaces.role, workspaces.label.as_deref()),
        (&SemanticRole::TabList, Some("Workspaces"))
    );
    assert_eq!(
        menu.children
            .iter()
            .filter(|id| frame.semantics.get(**id).unwrap().focusable)
            .count(),
        1
    );
    assert_eq!(
        workspaces
            .children
            .iter()
            .filter(|id| frame.semantics.get(**id).unwrap().focusable)
            .count(),
        1
    );
    for response in &output.responses {
        let node = frame.semantics.get(response.id).unwrap();
        assert_eq!(node.bounds, response.rect);
        assert!(bar.config.bounds.contains_rect(response.rect));
        assert!(response.rect.x.is_finite() && response.rect.max_x().is_finite());
    }
    assert!(
        output
            .responses
            .windows(2)
            .all(|pair| pair[0].rect.max_x() <= pair[1].rect.x)
    );
    let active = frame.semantics.get(bar.workspace_widget_id(W1)).unwrap();
    let disabled = frame.semantics.get(bar.workspace_widget_id(W2)).unwrap();
    assert!(active.state.selected && !active.state.focused);
    assert!(disabled.state.disabled && !disabled.focusable && disabled.actions.is_empty());
    assert_eq!(
        frame.semantics.get(w3).unwrap().actions[0].kind,
        SemanticActionKind::Invoke
    );
    bar.workspaces = vec![WorkspaceTab::new(W3, "Grading", true)];
    assert_eq!(bar.workspace_widget_id(W3), w3);
}
#[test]
fn pointer_transactions_open_replace_activate_and_cancel_exactly_once() {
    let mut menu_bar = bar();
    let mut memory = UiMemory::new();
    run(
        &mut menu_bar,
        &mut memory,
        pointer(Point::new(10.0, 10.0), Some(true)),
    );
    let (opened, _) = run(
        &mut menu_bar,
        &mut memory,
        pointer(Point::new(10.0, 10.0), Some(false)),
    );
    assert!(matches!(
        opened.intents.as_slice(),
        [ApplicationBarIntent::OpenMenu { menu: FILE, .. }]
    ));
    let (replaced, _) = run(
        &mut menu_bar,
        &mut memory,
        pointer(Point::new(75.0, 10.0), None),
    );
    assert!(matches!(
        replaced.intents.as_slice(),
        [ApplicationBarIntent::OpenMenu { menu: VIEW, .. }]
    ));
    assert_ne!(memory.focused(), Some(menu_bar.menu_widget_id(VIEW)));
    let press = Point::new(325.0, 10.0);
    let mut workspace_bar = bar();
    let mut memory = UiMemory::new();
    run(&mut workspace_bar, &mut memory, pointer(press, Some(true)));
    let (activated, _) = run(&mut workspace_bar, &mut memory, pointer(press, Some(false)));
    assert!(
        matches!(activated.intents.as_slice(), [ApplicationBarIntent::ActivateWorkspace(target)] if target.id == W3)
    );
    for release in [
        pointer(Point::new(185.0, 10.0), Some(false)),
        pointer(Point::new(125.0, 10.0), Some(false)),
        cancel(UiInputEvent::PointerReleaseAll {
            position: Some(press),
        }),
        cancel(UiInputEvent::WindowFocusChanged(false)),
    ] {
        let mut candidate = bar();
        let mut memory = UiMemory::new();
        run(&mut candidate, &mut memory, pointer(press, Some(true)));
        assert!(
            !run(&mut candidate, &mut memory, release)
                .0
                .intents
                .iter()
                .any(|intent| matches!(intent, ApplicationBarIntent::ActivateWorkspace(_)))
        );
    }
}
#[test]
fn keyboard_navigation_and_focus_repair_preserve_passive_active_state() {
    let mut bar = bar();
    let mut memory = UiMemory::new();
    memory.focus(bar.workspace_widget_id(W1));
    assert!(
        run(&mut bar, &mut memory, key(Key::ArrowRight))
            .0
            .intents
            .is_empty()
    );
    assert_eq!(memory.focused(), Some(bar.workspace_widget_id(W3)));
    for movement in [Key::Home, Key::End, Key::ArrowLeft, Key::ArrowRight] {
        run(&mut bar, &mut memory, key(movement));
    }
    for activation in [Key::Enter, Key::Space] {
        let (activated, _) = run(&mut bar, &mut memory, key(activation));
        assert!(
            matches!(activated.intents.as_slice(), [ApplicationBarIntent::ActivateWorkspace(target)] if target.id == W3)
        );
    }
    assert!(bar.workspaces[0].active && !bar.workspaces[2].active);
    let (opened, _) = run(&mut bar, &mut memory, key(Key::Function(10)));
    assert!(matches!(
        opened.intents.as_slice(),
        [ApplicationBarIntent::OpenMenu { menu: FILE, .. }]
    ));
    let (moved, _) = run(&mut bar, &mut memory, key(Key::ArrowLeft));
    assert!(matches!(
        moved.intents.as_slice(),
        [ApplicationBarIntent::OpenMenu { menu: VIEW, .. }]
    ));
    run(&mut bar, &mut memory, key(Key::ArrowRight));
    let (dismissed, _) = run(&mut bar, &mut memory, key(Key::Escape));
    assert!(matches!(
        dismissed.intents.as_slice(),
        [ApplicationBarIntent::DismissMenu { menu: FILE }]
    ));
    assert_eq!(memory.focused(), Some(bar.menu_widget_id(FILE)));
    memory.focus(bar.workspace_widget_id(W3));
    run(&mut bar, &mut memory, UiInput::default());
    bar.workspaces = vec![WorkspaceTab::new(W1, "Compositing", true)];
    run(&mut bar, &mut memory, UiInput::default());
    assert_eq!(memory.focused(), Some(bar.workspace_widget_id(W1)));
    bar.workspaces[0].enabled = false;
    run(&mut bar, &mut memory, UiInput::default());
    assert_eq!(memory.focused(), None);
}
#[test]
fn customized_theme_height_is_authoritative_everywhere() {
    let mut theme = default_dark_theme();
    theme.sizes.workspace_bar = 33.25;
    let mut candidate = bar();
    candidate.config.bounds.height = 5.0;
    let point = Point::new(325.0, 30.0);
    let mut memory = UiMemory::new();
    memory.focus(candidate.workspace_widget_id(W3));
    run_with_theme(
        &mut candidate,
        &mut memory,
        pointer(point, Some(true)),
        &theme,
    );
    let (output, frame) = run_with_theme(
        &mut candidate,
        &mut memory,
        pointer(point, Some(false)),
        &theme,
    );
    let expected = Rect::new(0.25, 0.5, 360.5, 33.25);
    assert!(matches!(
        frame.primitives.first(),
        Some(Primitive::Rect(rect)) if rect.rect == expected
    ));
    assert_eq!(
        frame.semantics.get(candidate.config.root).unwrap().bounds,
        expected
    );
    assert!(output.responses.iter().all(|response| {
        response.rect.y.to_bits() == expected.y.to_bits()
            && response.rect.height.to_bits() == expected.height.to_bits()
            && frame.semantics.get(response.id).unwrap().bounds == response.rect
            && frame.primitives.iter().any(
                |primitive| matches!(primitive, Primitive::Rect(rect) if rect.rect == response.rect),
            )
    }));
    assert!(output.drag_safe_regions.iter().all(|rect| {
        rect.y.to_bits() == expected.y.to_bits()
            && rect.height.to_bits() == expected.height.to_bits()
    }));
    assert!(matches!(
        output.intents.as_slice(),
        [ApplicationBarIntent::ActivateWorkspace(target)] if target.id == W3
    ));
    let active = frame
        .semantics
        .get(candidate.workspace_widget_id(W1))
        .unwrap();
    let focused = frame
        .semantics
        .get(candidate.workspace_widget_id(W3))
        .unwrap();
    assert!(active.state.selected && !active.state.focused);
    assert!(!focused.state.selected && focused.state.focused);
    theme.sizes.workspace_bar = f32::NAN;
    assert!(candidate.prepare(&theme).is_none());
}
#[test]
fn stale_bar_or_active_theme_preparation_is_inert() {
    let mut prepared_theme = default_dark_theme();
    prepared_theme.sizes.workspace_bar = 31.25;

    let mut changed_theme_bar = bar();
    let prepared = changed_theme_bar.prepare(&prepared_theme).unwrap();
    let mut active_theme = prepared_theme;
    active_theme.sizes.workspace_bar = 32.75;
    assert_preparation_mismatch_is_inert(&mut changed_theme_bar, &prepared, &active_theme);

    let mut invalid_theme_bar = bar();
    let prepared = invalid_theme_bar.prepare(&prepared_theme).unwrap();
    let mut invalid_theme = prepared_theme;
    invalid_theme.sizes.workspace_bar = f32::NAN;
    assert_preparation_mismatch_is_inert(&mut invalid_theme_bar, &prepared, &invalid_theme);

    let mut changed_bar = bar();
    let prepared = changed_bar.prepare(&prepared_theme).unwrap();
    changed_bar.config.workspace_width = 71.0;
    assert_preparation_mismatch_is_inert(&mut changed_bar, &prepared, &prepared_theme);
}
#[test]
fn invalid_duplicate_overflow_and_overlap_geometry_is_wholly_inert() {
    let mut cases = Vec::new();
    for (menu_width, workspace_width, point) in [
        (f32::NAN, 70.0, Point::new(325.0, 10.0)),
        (0.0, 70.0, Point::new(325.0, 10.0)),
        (50.25, f32::NAN, Point::new(10.0, 10.0)),
        (50.25, 0.0, Point::new(10.0, 10.0)),
        (200.0, 70.0, Point::new(325.0, 10.0)),
        (50.25, 130.0, Point::new(10.0, 10.0)),
        (100.0, 80.0, Point::new(10.0, 10.0)),
        (100.0, 80.0, Point::new(325.0, 10.0)),
    ] {
        let mut candidate = bar();
        candidate.config.menu_width = menu_width;
        candidate.config.workspace_width = workspace_width;
        cases.push((candidate, point));
    }
    let mut duplicate_menu = bar();
    duplicate_menu.menu_bar = MenuBar::from_menus([
        MenuBarMenu::from_actions(FILE, "File", [ActionDescriptor::new("open", "Open")]),
        MenuBarMenu::from_actions(FILE, "Again", [ActionDescriptor::new("save", "Save")]),
    ]);
    cases.push((duplicate_menu, Point::new(325.0, 10.0)));
    let mut duplicate_workspace = bar();
    duplicate_workspace.workspaces[1].id = W1;
    cases.push((duplicate_workspace, Point::new(10.0, 10.0)));
    for (mut candidate, point) in cases {
        let mut memory = UiMemory::new();
        for down in [true, false] {
            let (output, frame) = run(&mut candidate, &mut memory, pointer(point, Some(down)));
            assert!(output.responses.is_empty());
            assert!(output.intents.is_empty() && output.drag_safe_regions.is_empty());
            let root = frame.semantics.get(candidate.config.root).unwrap();
            assert!(root.children.is_empty());
            assert_eq!(frame.semantics.len(), 1);
        }
    }
    let mut invalid_root = bar();
    invalid_root.config.bounds = Rect::ZERO;
    let (_, frame) = run(&mut invalid_root, &mut UiMemory::new(), UiInput::default());
    assert!(frame.semantics.is_empty());
}
