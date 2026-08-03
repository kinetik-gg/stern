//! Public-facade end-to-end coverage for the application bar.
//!
//! Exercises the complete public path: constructing and preparing an
//! [`ApplicationBar`], declaring its pointer targets through
//! [`PreparedApplicationBar::declare_pointer_targets`], and evaluating it
//! through [`Ui::application_bar`]. Follow-up to the focused internal
//! conformance matrix in `stern-widgets`, which this test does not duplicate.
use stern::{
    core::{
        ActionDescriptor, FrameContext, FrameOutput, PhysicalSize, Point, PointerButtonState,
        PointerInput, PointerOrder, PointerTarget, Rect, ScaleFactor, SemanticActionKind,
        SemanticRole, Size, Theme, TimeInfo, UiInput, UiMemory, ViewportInfo, WidgetId,
        default_dark_theme,
    },
    widgets::{
        ApplicationBar, ApplicationBarConfig, ApplicationBarIntent, ApplicationBarOutput,
        ApplicationMenuBar, MenuBarMenu, MenuBarMenuId, PreparedApplicationBar, Ui, WorkspaceTab,
        WorkspaceTabId,
    },
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
        ApplicationMenuBar::from_menus([
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

/// Runs one frame through the exact public path an application drives:
/// prepare, declare pointer targets, then evaluate through `Ui::application_bar`.
fn run(
    bar: &mut ApplicationBar,
    memory: &mut UiMemory,
    theme: &Theme,
    input: UiInput,
) -> (ApplicationBarOutput, FrameOutput) {
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

/// Proves that a preparation which no longer matches the bar or active theme
/// publishes neither pointer targets nor evaluation output, while unrelated
/// pointer targets declared in the same plan remain unaffected.
fn assert_publishes_nothing_through_the_public_facade(
    bar: &mut ApplicationBar,
    prepared: &PreparedApplicationBar,
    active_theme: &Theme,
) {
    let probe_id = WidgetId::from_key("application-bar-facade-mismatch-probe");
    let probe_rect = Rect::new(0.0, 0.0, 500.0, 200.0);
    let context = FrameContext::new(
        ViewportInfo::new(
            Size::new(500.0, 200.0),
            PhysicalSize::new(500, 200),
            ScaleFactor::ONE,
        ),
        pointer(Point::new(325.0, 10.0), Some(true)),
        TimeInfo::default(),
    );
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
fn application_bar_is_constructible_through_the_public_facade() {
    let menu = MenuBarMenu::from_actions(
        MenuBarMenuId::from_raw(1),
        "File",
        [ActionDescriptor::new("open", "Open")],
    );
    let bar = ApplicationBar::new(
        ApplicationBarConfig::new(WidgetId::from_key("bar"), Rect::new(0.0, 0.0, 320.0, 40.0)),
        ApplicationMenuBar::from_menus([menu]),
        [WorkspaceTab::new(
            WorkspaceTabId::from_raw(1),
            "Editing",
            true,
        )],
    );
    let prepared = bar.prepare(&default_dark_theme()).unwrap();
    assert_eq!(prepared.drag_safe_regions().len(), 1);
}

#[test]
fn application_bar_declares_and_evaluates_menu_and_workspace_through_the_public_facade() {
    let theme = default_dark_theme();
    let mut app_bar = bar();
    let mut memory = UiMemory::new();

    // Open the File menu through the public pointer-declaration path.
    run(
        &mut app_bar,
        &mut memory,
        &theme,
        pointer(Point::new(10.0, 10.0), Some(true)),
    );
    let (menu_output, menu_frame) = run(
        &mut app_bar,
        &mut memory,
        &theme,
        pointer(Point::new(10.0, 10.0), Some(false)),
    );
    assert!(matches!(
        menu_output.intents.as_slice(),
        [ApplicationBarIntent::OpenMenu { menu: FILE, .. }]
    ));
    assert_eq!(menu_output.responses.len(), 5);
    assert!(!menu_frame.primitives.is_empty());
    let root = menu_frame.semantics.get(app_bar.config.root).unwrap();
    assert_eq!(
        root.role,
        SemanticRole::Custom("application-bar".to_owned())
    );
    let menu_item = menu_frame
        .semantics
        .get(app_bar.menu_widget_id(FILE))
        .unwrap();
    assert_eq!(menu_item.role, SemanticRole::MenuItem);
    assert!(
        menu_item
            .actions
            .iter()
            .any(|action| action.kind == SemanticActionKind::Open)
    );
    assert!(menu_item.state.expanded == Some(true));

    // Dismiss the open menu before exercising workspace activation so the
    // subsequent intent is unambiguous.
    run(
        &mut app_bar,
        &mut memory,
        &theme,
        pointer(Point::new(10.0, 10.0), Some(true)),
    );
    run(
        &mut app_bar,
        &mut memory,
        &theme,
        pointer(Point::new(10.0, 10.0), Some(false)),
    );

    // Activate the disabled-adjacent, inactive workspace tab through the
    // public pointer-declaration path.
    run(
        &mut app_bar,
        &mut memory,
        &theme,
        pointer(Point::new(325.0, 10.0), Some(true)),
    );
    let (workspace_output, workspace_frame) = run(
        &mut app_bar,
        &mut memory,
        &theme,
        pointer(Point::new(325.0, 10.0), Some(false)),
    );
    assert!(matches!(
        workspace_output.intents.as_slice(),
        [ApplicationBarIntent::ActivateWorkspace(target)] if target.id == W3
    ));
    assert_eq!(workspace_output.responses.len(), 5);
    let workspace_root = workspace_frame.semantics.get(app_bar.config.root).unwrap();
    assert_eq!(workspace_root.children.len(), 2);
    let workspace_item = workspace_frame
        .semantics
        .get(app_bar.workspace_widget_id(W3))
        .unwrap();
    assert_eq!(workspace_item.role, SemanticRole::Tab);
    assert!(
        workspace_item
            .actions
            .iter()
            .any(|action| action.kind == SemanticActionKind::Invoke)
    );
}

#[test]
fn stale_bar_preparation_publishes_no_targets_or_outputs_through_the_public_facade() {
    let mut prepared_theme = default_dark_theme();
    prepared_theme.sizes.workspace_bar = 31.25;
    let mut app_bar = bar();
    let prepared = app_bar.prepare(&prepared_theme).unwrap();

    // The active theme has drifted since preparation: the same preparation
    // must publish nothing rather than presenting stale geometry.
    let mut active_theme = prepared_theme;
    active_theme.sizes.workspace_bar = 32.75;
    assert_publishes_nothing_through_the_public_facade(&mut app_bar, &prepared, &active_theme);
}

#[test]
fn invalid_active_theme_publishes_no_targets_or_outputs_through_the_public_facade() {
    let prepared_theme = default_dark_theme();
    let mut app_bar = bar();
    let prepared = app_bar.prepare(&prepared_theme).unwrap();

    // The active theme is non-finite: the preparation must fail closed
    // instead of publishing invalid geometry.
    let mut invalid_theme = prepared_theme;
    invalid_theme.sizes.workspace_bar = f32::NAN;
    assert_publishes_nothing_through_the_public_facade(&mut app_bar, &prepared, &invalid_theme);
}
