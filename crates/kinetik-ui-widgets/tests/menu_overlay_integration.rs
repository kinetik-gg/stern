//! Windowless menu, context menu, dropdown, tooltip, popover, and palette integration tests.

use kinetik_ui_core::{
    ActionContext, ActionDescriptor, ActionId, ActionQueue, ActionSource, Point, Rect, Size,
    UiInput, UiMemory, WidgetId, default_dark_theme,
};
use kinetik_ui_widgets::{
    CommandPaletteOverlay, Menu, MenuOverlay, OverlayDismissal, OverlayEntry, OverlayId,
    OverlayKind, OverlayStack, PopoverPlacement, PopoverRequest, Ui, place_popover,
};

fn action(id: &str, label: &str) -> ActionDescriptor {
    ActionDescriptor::new(id, label)
}

fn overlay_id(raw: u64) -> OverlayId {
    OverlayId::from_raw(raw)
}

#[test]
fn menu_overlay_invokes_enabled_visible_actions_with_context_menu_context() {
    let mut hidden = action("scene.hidden", "Hidden");
    hidden.state.visible = false;
    let mut disabled = action("scene.disabled", "Disabled");
    disabled.state.enabled = false;
    let target = WidgetId::from_key("scene-node");
    let overlay = MenuOverlay::anchored(
        overlay_id(10),
        OverlayKind::ContextMenu,
        Menu::from_actions([action("scene.open", "Open"), hidden, disabled]),
        Rect::new(42.0, 38.0, 1.0, 1.0),
        Size::new(180.0, 92.0),
        PopoverPlacement::Below,
        4.0,
        true,
        Rect::new(0.0, 0.0, 320.0, 240.0),
        OverlayDismissal::OutsideClickOrEscape,
        ActionSource::Menu,
        ActionContext::Widget(target),
    );
    let mut stack = OverlayStack::new();
    let mut queue = ActionQueue::new();

    overlay.open_in(&mut stack);

    assert_eq!(stack.top().map(|entry| entry.id), Some(overlay_id(10)));
    assert_eq!(stack.focus_target(), Some(overlay_id(10)));
    assert_eq!(overlay.visible_items().len(), 2);
    assert!(overlay.invoke_visible(0, &mut queue));
    assert!(!overlay.invoke_visible(1, &mut queue));
    assert!(!overlay.invoke_visible(2, &mut queue));

    let invocation = queue.pop_front().expect("context menu invocation");
    assert_eq!(invocation.action_id, ActionId::new("scene.open"));
    assert_eq!(invocation.source, ActionSource::Menu);
    assert_eq!(invocation.context, ActionContext::Widget(target));
    assert!(queue.is_empty());
}

#[test]
fn dropdown_popover_and_tooltip_use_overlay_stack_placement_and_dismissal() {
    let dropdown = MenuOverlay::anchored(
        overlay_id(20),
        OverlayKind::Dropdown,
        Menu::from_actions([action("view.mode", "View Mode")]),
        Rect::new(260.0, 12.0, 48.0, 24.0),
        Size::new(90.0, 60.0),
        PopoverPlacement::Below,
        6.0,
        true,
        Rect::new(0.0, 0.0, 300.0, 180.0),
        OverlayDismissal::OutsideClickOrEscape,
        ActionSource::Menu,
        ActionContext::Global,
    );
    let popover_rect = place_popover(
        PopoverRequest {
            anchor: dropdown.entry.rect,
            size: Size::new(84.0, 42.0),
            placement: PopoverPlacement::Right,
            offset: 4.0,
            fit_viewport: true,
        },
        Rect::new(0.0, 0.0, 300.0, 180.0),
    );
    let popover = OverlayEntry::new(overlay_id(21), OverlayKind::Popover, popover_rect)
        .dismiss_on(OverlayDismissal::OutsideClick);
    let tooltip = OverlayEntry::new(
        overlay_id(22),
        OverlayKind::Tooltip,
        Rect::new(
            dropdown.entry.rect.x,
            dropdown.entry.rect.y - 24.0,
            100.0,
            20.0,
        ),
    );
    let mut stack = OverlayStack::new();

    dropdown.open_in(&mut stack);
    assert!(stack.open_child(dropdown.entry.id, popover.clone()));
    stack.open(tooltip);

    assert_eq!(
        stack
            .entries()
            .iter()
            .map(|entry| (entry.id, entry.kind, entry.parent))
            .collect::<Vec<_>>(),
        vec![
            (overlay_id(20), OverlayKind::Dropdown, None),
            (overlay_id(21), OverlayKind::Popover, Some(overlay_id(20))),
            (overlay_id(22), OverlayKind::Tooltip, None),
        ]
    );
    assert_eq!(
        stack.dismissal_requests(Some(Point::new(8.0, 170.0)), true),
        vec![overlay_id(21), overlay_id(20)]
    );
    assert_eq!(stack.escape_close_request(), Some(overlay_id(20)));
    assert!(stack.close(dropdown.entry.id).is_some());
    assert_eq!(
        stack.entries(),
        &[OverlayEntry::new(
            overlay_id(22),
            OverlayKind::Tooltip,
            Rect::new(
                dropdown.entry.rect.x,
                dropdown.entry.rect.y - 24.0,
                100.0,
                20.0
            )
        )]
    );
}

#[test]
fn command_palette_overlay_filters_and_invokes_without_executing_commands() {
    let mut save = action("workspace.save", "Save Workspace");
    save.keywords = vec!["write".to_owned(), "persist".to_owned()];
    let mut disabled = action("workspace.disabled", "Disabled Workspace");
    disabled.state.enabled = false;
    let mut hidden = action("workspace.hidden", "Hidden Workspace");
    hidden.state.visible = false;
    let mut overlay = CommandPaletteOverlay::anchored_from_actions(
        overlay_id(30),
        &[
            save,
            disabled,
            hidden,
            action("workspace.export", "Export Workspace"),
        ],
        Rect::new(120.0, 36.0, 80.0, 24.0),
        Size::new(240.0, 160.0),
        PopoverPlacement::Below,
        8.0,
        true,
        Rect::new(0.0, 0.0, 420.0, 320.0),
        OverlayDismissal::OutsideClickOrEscape,
        ActionContext::Global,
    );
    let mut stack = OverlayStack::new();
    let mut queue = ActionQueue::new();

    overlay.open_in(&mut stack);
    overlay.palette.query = "write".to_owned();

    assert!(stack.has_modal());
    assert_eq!(stack.focus_target(), Some(overlay_id(30)));
    assert_eq!(
        overlay.matches()[0].action_id,
        ActionId::new("workspace.save")
    );
    assert!(overlay.invoke_selected(&mut queue));

    let invocation = queue.pop_front().expect("palette invocation");
    assert_eq!(invocation.action_id, ActionId::new("workspace.save"));
    assert_eq!(invocation.source, ActionSource::CommandPalette);
    assert_eq!(invocation.context, ActionContext::Global);

    overlay.palette.query = "disabled".to_owned();
    overlay.palette.selected = 0;
    assert!(!overlay.invoke_selected(&mut queue));
    overlay.palette.query = "hidden".to_owned();
    assert!(overlay.matches().is_empty());
    assert!(queue.is_empty());
}

#[test]
fn ui_overlay_helpers_emit_invocations_to_frame_output_only() {
    let theme = default_dark_theme();
    let input = UiInput::default();
    let mut memory = UiMemory::new();
    let mut ui = Ui::new(&input, &mut memory, &theme);
    let menu_overlay = MenuOverlay::new(
        OverlayEntry::new(
            overlay_id(40),
            OverlayKind::Menu,
            Rect::new(0.0, 0.0, 120.0, 80.0),
        ),
        Menu::from_actions([action("file.save", "Save")]),
        ActionSource::Menu,
        ActionContext::Global,
    );
    let mut palette_overlay = CommandPaletteOverlay::from_actions(
        OverlayEntry::new(
            overlay_id(41),
            OverlayKind::CommandPalette,
            Rect::new(0.0, 90.0, 220.0, 120.0),
        ),
        &[action("file.open", "Open")],
        ActionContext::Global,
    );
    palette_overlay.palette.query = "open".to_owned();

    assert!(ui.invoke_menu_overlay_item(&menu_overlay, 0));
    assert!(ui.invoke_command_palette_overlay(&palette_overlay));
    assert!(!ui.invoke_menu_overlay_item(&menu_overlay, 1));
    let mut output = ui.finish_output();

    assert_eq!(
        output
            .actions
            .drain()
            .map(|invocation| (invocation.action_id, invocation.source))
            .collect::<Vec<_>>(),
        vec![
            (ActionId::new("file.save"), ActionSource::Menu),
            (ActionId::new("file.open"), ActionSource::CommandPalette),
        ]
    );
}
