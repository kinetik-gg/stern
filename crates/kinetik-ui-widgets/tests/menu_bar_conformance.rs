//! Windowless menu-bar conformance for reusable editor chrome contracts.

use kinetik_ui_core::{
    ActionContext, ActionDescriptor, ActionId, ActionQueue, ActionSource, Rect, Size, WidgetId,
};
use kinetik_ui_widgets::{
    CommandPalette, MenuBar, MenuBarMenu, MenuBarMenuId, MenuBarOverlayRequest, OverlayDismissal,
    OverlayId, OverlayKind, OverlayStack, PopoverPlacement,
};

fn menu_id(raw: u64) -> MenuBarMenuId {
    MenuBarMenuId::from_raw(raw)
}

fn overlay_id(raw: u64) -> OverlayId {
    OverlayId::from_raw(raw)
}

fn action(id: &str, label: &str) -> ActionDescriptor {
    ActionDescriptor::new(id, label)
}

fn hidden_action(id: &str) -> ActionDescriptor {
    let mut action = ActionDescriptor::new(id, "Hidden");
    action.state.visible = false;
    action
}

fn disabled_action(id: &str, label: &str) -> ActionDescriptor {
    let mut action = ActionDescriptor::new(id, label);
    action.state.enabled = false;
    action
}

#[test]
fn menu_bar_opens_toggles_closes_and_ignores_unknown_ids() {
    let file = menu_id(1);
    let edit = menu_id(2);
    let missing = menu_id(99);
    let mut menu_bar = MenuBar::from_menus([
        MenuBarMenu::from_actions(file, "File", [action("file.open", "Open")]),
        MenuBarMenu::from_actions(edit, "Edit", [action("edit.undo", "Undo")]),
    ]);
    let queue = ActionQueue::new();

    assert_eq!(menu_bar.active_id(), None);
    assert!(menu_bar.open(file));
    assert_eq!(menu_bar.active_id(), Some(file));
    assert!(!menu_bar.open(missing));
    assert_eq!(menu_bar.active_id(), Some(file));

    assert!(menu_bar.hover_open(edit));
    assert_eq!(menu_bar.active_id(), Some(edit));
    assert!(menu_bar.toggle(edit));
    assert_eq!(menu_bar.active_id(), None);

    assert!(!menu_bar.hover_open(file));
    assert_eq!(menu_bar.active_id(), None);
    assert!(menu_bar.toggle(file));
    assert_eq!(menu_bar.close(), Some(file));
    assert_eq!(menu_bar.close(), None);
    assert!(queue.is_empty());
}

#[test]
fn menu_bar_navigation_skips_empty_and_all_hidden_menus_and_wraps() {
    let file = menu_id(1);
    let edit = menu_id(2);
    let view = menu_id(3);
    let empty = menu_id(4);
    let help = menu_id(5);
    let mut menu_bar = MenuBar::from_menus([
        MenuBarMenu::from_actions(file, "File", [action("file.open", "Open")]),
        MenuBarMenu::from_actions(edit, "Edit", [hidden_action("edit.hidden")]),
        MenuBarMenu::from_actions(view, "View", [disabled_action("view.grid", "Grid")]),
        MenuBarMenu::from_actions(empty, "Empty", Vec::<ActionDescriptor>::new()),
        MenuBarMenu::from_actions(help, "Help", [action("help.about", "About")]),
    ]);

    assert_eq!(menu_bar.move_next(), Some(file));
    assert_eq!(menu_bar.move_next(), Some(view));
    assert_eq!(menu_bar.move_next(), Some(help));
    assert_eq!(menu_bar.move_next(), Some(file));
    assert_eq!(menu_bar.move_previous(), Some(help));

    assert!(!menu_bar.open(edit));
    assert!(!menu_bar.open(empty));
    assert_eq!(menu_bar.active_id(), Some(help));
}

#[test]
fn menu_bar_overlay_conversion_preserves_placement_source_dismissal_and_context() {
    let file = menu_id(1);
    let context = ActionContext::Frame(WidgetId::from_key("editor-frame"));
    let mut menu_bar = MenuBar::from_menus([MenuBarMenu::from_actions(
        file,
        "File",
        [
            action("file.open", "Open"),
            disabled_action("file.disabled", "Disabled"),
            hidden_action("file.hidden"),
        ],
    )]);
    let mut stack = OverlayStack::new();
    let mut queue = ActionQueue::new();

    assert!(menu_bar.open(file));
    let overlay = menu_bar
        .active_overlay(MenuBarOverlayRequest {
            overlay_id: overlay_id(10),
            kind: OverlayKind::Menu,
            anchor: Rect::new(10.0, 20.0, 80.0, 24.0),
            size: Size::new(120.0, 90.0),
            placement: PopoverPlacement::Below,
            offset: 2.0,
            fit_viewport: false,
            viewport: Rect::new(0.0, 0.0, 320.0, 240.0),
            dismissal: OverlayDismissal::OutsideClickOrEscape,
            source: ActionSource::Menu,
            context: context.clone(),
        })
        .expect("active menu overlay");

    overlay.open_in(&mut stack);

    assert_eq!(overlay.entry.id, overlay_id(10));
    assert_eq!(overlay.entry.kind, OverlayKind::Menu);
    assert_eq!(overlay.entry.rect, Rect::new(10.0, 46.0, 120.0, 90.0));
    assert_eq!(
        overlay.entry.dismissal,
        OverlayDismissal::OutsideClickOrEscape
    );
    assert_eq!(overlay.source, ActionSource::Menu);
    assert_eq!(overlay.context, context);
    assert_eq!(stack.focus_target(), Some(overlay_id(10)));
    assert_eq!(overlay.visible_items().len(), 2);

    assert!(overlay.invoke_visible(0, &mut queue));
    assert!(!overlay.invoke_visible(1, &mut queue));
    assert!(!overlay.invoke_visible(2, &mut queue));

    let invocation = queue.pop_front().expect("menu invocation");
    assert_eq!(invocation.action_id, ActionId::new("file.open"));
    assert_eq!(invocation.source, ActionSource::Menu);
    assert_eq!(
        invocation.context,
        ActionContext::Frame(WidgetId::from_key("editor-frame"))
    );
    assert!(queue.is_empty());
}

#[test]
fn menu_bar_hidden_disabled_behavior_matches_menu_and_command_palette_surfaces() {
    let root = menu_id(1);
    let visible = action("workspace.save", "Save Workspace");
    let disabled = disabled_action("workspace.disabled", "Disabled Workspace");
    let hidden = hidden_action("workspace.hidden");
    let menu_bar = MenuBar::from_menus([MenuBarMenu::from_actions(
        root,
        "Workspace",
        [visible.clone(), disabled.clone(), hidden.clone()],
    )]);
    let overlay = menu_bar.active_overlay(MenuBarOverlayRequest {
        overlay_id: overlay_id(20),
        kind: OverlayKind::Menu,
        anchor: Rect::new(0.0, 0.0, 60.0, 24.0),
        size: Size::new(160.0, 90.0),
        placement: PopoverPlacement::Below,
        offset: 0.0,
        fit_viewport: true,
        viewport: Rect::new(0.0, 0.0, 320.0, 240.0),
        dismissal: OverlayDismissal::OutsideClick,
        source: ActionSource::Menu,
        context: ActionContext::Global,
    });

    assert_eq!(overlay, None);

    let mut menu_bar = menu_bar;
    assert!(menu_bar.open(root));
    let overlay = menu_bar
        .active_overlay(MenuBarOverlayRequest {
            overlay_id: overlay_id(21),
            kind: OverlayKind::Menu,
            anchor: Rect::new(0.0, 0.0, 60.0, 24.0),
            size: Size::new(160.0, 90.0),
            placement: PopoverPlacement::Below,
            offset: 0.0,
            fit_viewport: true,
            viewport: Rect::new(0.0, 0.0, 320.0, 240.0),
            dismissal: OverlayDismissal::OutsideClick,
            source: ActionSource::Menu,
            context: ActionContext::Global,
        })
        .expect("active menu overlay");
    let mut queue = ActionQueue::new();

    assert_eq!(overlay.visible_items().len(), 2);
    assert!(overlay.invoke_visible(0, &mut queue));
    assert!(!overlay.invoke_visible(1, &mut queue));
    assert_eq!(
        queue.pop_front().expect("visible action").action_id,
        ActionId::new("workspace.save")
    );
    assert!(queue.is_empty());

    let mut palette = CommandPalette::from_actions(&[visible, disabled, hidden]);
    palette.query = "hidden".to_owned();
    assert!(palette.matches().is_empty());
    palette.query = "disabled".to_owned();
    assert_eq!(palette.matches().len(), 1);
    assert!(!palette.matches()[0].enabled);
    assert!(!palette.invoke_selected(&mut queue, ActionContext::Global));
    assert!(queue.is_empty());
}
