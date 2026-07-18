use super::{
    CommandPalette, Menu, OverlayDismissal, OverlayEntry, OverlayId, OverlayKind, OverlayStack,
    PopoverPlacement, PopoverRequest, overlay_semantics, place_popover,
};
use stern_core::{
    ActionContext, ActionDescriptor, ActionId, ActionQueue, Point, Rect, SemanticActionKind,
    SemanticRole, Size,
};

fn action(id: &str, label: &str) -> ActionDescriptor {
    ActionDescriptor::new(id, label)
}

#[test]
fn overlay_stack_preserves_order_and_replaces_ids() {
    let mut stack = OverlayStack::new();
    let first = OverlayEntry::new(
        OverlayId::from_raw(1),
        OverlayKind::Menu,
        Rect::new(0.0, 0.0, 10.0, 10.0),
    )
    .dismiss_on(OverlayDismissal::OutsideClick);
    let replacement = OverlayEntry {
        rect: Rect::new(1.0, 1.0, 10.0, 10.0),
        ..first.clone()
    };

    stack.open(first);
    stack.open(
        OverlayEntry::new(
            OverlayId::from_raw(2),
            OverlayKind::CommandPalette,
            Rect::new(0.0, 0.0, 20.0, 20.0),
        )
        .modal(true),
    );
    stack.open(replacement);

    assert_eq!(stack.entries().len(), 2);
    assert_eq!(stack.top().expect("top").id, OverlayId::from_raw(1));
    assert!(stack.has_modal());
}

#[test]
fn outside_click_requests_dismissible_overlays() {
    let mut stack = OverlayStack::new();
    stack.open(
        OverlayEntry::new(
            OverlayId::from_raw(1),
            OverlayKind::Popover,
            Rect::new(0.0, 0.0, 10.0, 10.0),
        )
        .dismiss_on(OverlayDismissal::OutsideClick),
    );

    assert_eq!(
        stack.outside_click_close_requests(Point::new(20.0, 20.0)),
        vec![OverlayId::from_raw(1)]
    );
}

#[test]
fn menu_invokes_enabled_visible_actions() {
    let mut disabled = action("disabled", "Disabled");
    disabled.state.enabled = false;
    let menu = Menu::from_actions([action("open", "Open"), disabled]);
    let mut queue = ActionQueue::new();

    assert!(menu.invoke_visible(0, &mut queue, ActionContext::Global));
    assert!(!menu.invoke_visible(1, &mut queue, ActionContext::Global));

    assert_eq!(
        queue.pop_front().expect("invocation").action_id,
        ActionId::new("open")
    );
}

#[test]
fn hidden_menu_actions_are_filtered() {
    let mut hidden = action("hidden", "Hidden");
    hidden.state.visible = false;
    let menu = Menu::from_actions([action("shown", "Shown"), hidden]);

    assert_eq!(menu.visible_items().len(), 1);
}

#[test]
fn popover_can_be_clamped_inside_viewport() {
    let rect = place_popover(
        PopoverRequest {
            anchor: Rect::new(90.0, 90.0, 10.0, 10.0),
            size: Size::new(40.0, 40.0),
            placement: PopoverPlacement::Below,
            offset: 4.0,
            fit_viewport: true,
        },
        Rect::new(0.0, 0.0, 100.0, 100.0),
    );

    assert!((rect.x - 60.0).abs() < f32::EPSILON);
    assert!((rect.y - 46.0).abs() < f32::EPSILON);
}

#[test]
fn popover_clamp_handles_overlay_larger_than_viewport() {
    let viewport = Rect::new(40.0, 30.0, 100.0, 80.0);
    let rect = place_popover(
        PopoverRequest {
            anchor: Rect::new(120.0, 90.0, 10.0, 10.0),
            size: Size::new(180.0, 160.0),
            placement: PopoverPlacement::Below,
            offset: 4.0,
            fit_viewport: true,
        },
        viewport,
    );

    assert_eq!(rect, viewport);
    assert!(viewport.contains_rect(rect));
    assert!(
        [rect.x, rect.y, rect.width, rect.height]
            .into_iter()
            .all(|value| value.is_finite() && value >= 0.0)
    );
}

#[test]
fn nested_close_removes_descendants() {
    let mut stack = OverlayStack::new();
    let parent = OverlayId::from_raw(1);
    let child = OverlayId::from_raw(2);
    let grandchild = OverlayId::from_raw(3);

    stack.open(OverlayEntry::new(
        parent,
        OverlayKind::Menu,
        Rect::new(0.0, 0.0, 20.0, 20.0),
    ));
    assert!(stack.open_child(
        parent,
        OverlayEntry::new(
            child,
            OverlayKind::Popover,
            Rect::new(20.0, 0.0, 20.0, 20.0)
        )
    ));
    assert!(stack.open_child(
        child,
        OverlayEntry::new(
            grandchild,
            OverlayKind::ContextMenu,
            Rect::new(40.0, 0.0, 20.0, 20.0),
        )
    ));

    assert_eq!(stack.entries().len(), 3);
    assert_eq!(stack.close(parent).map(|entry| entry.id), Some(parent));
    assert!(stack.entries().is_empty());
}

#[test]
fn overlay_routing_prefers_topmost_hit_and_modal_capture() {
    let mut stack = OverlayStack::new();
    stack.open(OverlayEntry::new(
        OverlayId::from_raw(1),
        OverlayKind::Popover,
        Rect::new(0.0, 0.0, 100.0, 100.0),
    ));
    stack.open(
        OverlayEntry::new(
            OverlayId::from_raw(2),
            OverlayKind::Modal,
            Rect::new(10.0, 10.0, 20.0, 20.0),
        )
        .modal(true),
    );

    assert_eq!(
        stack.pointer_capture_target(Point::new(15.0, 15.0)),
        Some(OverlayId::from_raw(2))
    );
    assert_eq!(
        stack.pointer_capture_target(Point::new(90.0, 90.0)),
        Some(OverlayId::from_raw(2))
    );
    assert_eq!(
        stack.pointer_capture_target(Point::new(150.0, 150.0)),
        Some(OverlayId::from_raw(2))
    );
    assert_eq!(stack.focus_target(), Some(OverlayId::from_raw(2)));
}

#[test]
fn modal_capture_does_not_block_higher_overlay_hits() {
    let mut stack = OverlayStack::new();
    stack.open(OverlayEntry::new(
        OverlayId::from_raw(1),
        OverlayKind::Popover,
        Rect::new(0.0, 0.0, 100.0, 100.0),
    ));
    stack.open(
        OverlayEntry::new(
            OverlayId::from_raw(2),
            OverlayKind::Modal,
            Rect::new(10.0, 10.0, 20.0, 20.0),
        )
        .dismiss_on(OverlayDismissal::OutsideClickOrEscape),
    );
    stack.open(OverlayEntry::new(
        OverlayId::from_raw(3),
        OverlayKind::Tooltip,
        Rect::new(80.0, 80.0, 10.0, 10.0),
    ));

    assert_eq!(
        stack.pointer_capture_target(Point::new(85.0, 85.0)),
        Some(OverlayId::from_raw(3))
    );
    assert_eq!(
        stack.pointer_capture_target(Point::new(50.0, 50.0)),
        Some(OverlayId::from_raw(2))
    );
    assert_eq!(
        stack.outside_click_close_requests(Point::new(50.0, 50.0)),
        vec![OverlayId::from_raw(2)]
    );
    assert_eq!(
        stack.dismissal_requests(None, true),
        vec![OverlayId::from_raw(2)]
    );
}

#[test]
fn dismissal_requests_cover_escape_and_outside_click() {
    let mut stack = OverlayStack::new();
    stack.open(
        OverlayEntry::new(
            OverlayId::from_raw(1),
            OverlayKind::Menu,
            Rect::new(0.0, 0.0, 50.0, 50.0),
        )
        .dismiss_on(OverlayDismissal::OutsideClick),
    );
    stack.open(
        OverlayEntry::new(
            OverlayId::from_raw(2),
            OverlayKind::CommandPalette,
            Rect::new(10.0, 10.0, 50.0, 50.0),
        )
        .dismiss_on(OverlayDismissal::OutsideClickOrEscape),
    );

    assert_eq!(
        stack.dismissal_requests(Some(Point::new(80.0, 80.0)), false),
        vec![OverlayId::from_raw(2), OverlayId::from_raw(1)]
    );
    assert_eq!(
        stack.dismissal_requests(None, true),
        vec![OverlayId::from_raw(2)]
    );
}

#[test]
fn focus_target_skips_non_focusable_overlay_surfaces() {
    let mut stack = OverlayStack::new();

    assert_eq!(stack.focus_target(), None);

    stack.open(OverlayEntry::new(
        OverlayId::from_raw(1),
        OverlayKind::Tooltip,
        Rect::new(0.0, 0.0, 20.0, 20.0),
    ));
    assert_eq!(stack.focus_target(), None);

    stack.open(OverlayEntry::new(
        OverlayId::from_raw(2),
        OverlayKind::Menu,
        Rect::new(0.0, 0.0, 60.0, 60.0),
    ));
    stack.open(OverlayEntry::new(
        OverlayId::from_raw(3),
        OverlayKind::DragPreview,
        Rect::new(10.0, 10.0, 20.0, 20.0),
    ));
    assert_eq!(stack.focus_target(), Some(OverlayId::from_raw(2)));

    stack.open(
        OverlayEntry::new(
            OverlayId::from_raw(4),
            OverlayKind::Popover,
            Rect::new(20.0, 20.0, 20.0, 20.0),
        )
        .modal(true),
    );
    assert_eq!(stack.focus_target(), Some(OverlayId::from_raw(4)));
}

#[test]
fn popover_clamp_handles_non_origin_viewport_edges() {
    let rect = place_popover(
        PopoverRequest {
            anchor: Rect::new(180.0, 95.0, 10.0, 10.0),
            size: Size::new(50.0, 30.0),
            placement: PopoverPlacement::Right,
            offset: 4.0,
            fit_viewport: true,
        },
        Rect::new(100.0, 50.0, 100.0, 80.0),
    );

    assert_eq!(rect, Rect::new(126.0, 95.0, 50.0, 30.0));
}

#[test]
fn overlay_semantics_describe_surface_and_dismissal() {
    let entry = OverlayEntry::new(
        OverlayId::from_raw(7),
        OverlayKind::CommandPalette,
        Rect::new(0.0, 0.0, 100.0, 50.0),
    )
    .dismiss_on(OverlayDismissal::Escape);

    let node = overlay_semantics(&entry, "Commands");

    assert_eq!(node.role, SemanticRole::CommandPalette);
    assert_eq!(node.label.as_deref(), Some("Commands"));
    assert!(node.focusable);
    assert!(
        node.actions
            .iter()
            .any(|action| action.kind == SemanticActionKind::Dismiss)
    );
}

#[test]
fn overlay_semantics_expose_focusability_for_modal_and_non_focusable_surfaces() {
    let modal_popover = OverlayEntry::new(
        OverlayId::from_raw(8),
        OverlayKind::Popover,
        Rect::new(0.0, 0.0, 100.0, 50.0),
    )
    .modal(true)
    .dismiss_on(OverlayDismissal::OutsideClick);
    let tooltip = OverlayEntry::new(
        OverlayId::from_raw(9),
        OverlayKind::Tooltip,
        Rect::new(0.0, 0.0, 40.0, 20.0),
    );

    let modal_node = overlay_semantics(&modal_popover, "Inspector");
    let tooltip_node = overlay_semantics(&tooltip, "Tip");

    assert_eq!(modal_node.role, SemanticRole::Custom("popover".to_owned()));
    assert_eq!(modal_node.label.as_deref(), Some("Inspector"));
    assert!(modal_node.focusable);
    assert!(
        modal_node
            .actions
            .iter()
            .any(|action| action.kind == SemanticActionKind::Dismiss)
    );
    assert_eq!(
        tooltip_node.role,
        SemanticRole::Custom("tooltip".to_owned())
    );
    assert_eq!(tooltip_node.label.as_deref(), Some("Tip"));
    assert!(!tooltip_node.focusable);
    assert!(tooltip_node.actions.is_empty());
}

#[test]
fn command_palette_filters_by_label_and_keyword() {
    let mut save = action("save", "Save Project");
    save.keywords = vec!["write".to_owned()];
    let mut palette = CommandPalette::from_actions(&[save, action("export", "Export")]);

    palette.query = "wri".to_owned();

    assert_eq!(palette.matches()[0].action_id, ActionId::new("save"));
}

#[test]
fn command_palette_entries_preserve_checked_action_state() {
    let mut grid = action("view.grid", "Grid");
    grid.state.checked = Some(true);

    let palette = CommandPalette::from_actions(&[grid]);

    assert_eq!(palette.matches()[0].checked, Some(true));
}

#[test]
fn command_palette_moves_selection_and_invokes() {
    let mut palette =
        CommandPalette::from_actions(&[action("first", "First"), action("second", "Second")]);
    let mut queue = ActionQueue::new();

    palette.move_selection(1);
    assert!(palette.invoke_selected(&mut queue, ActionContext::Global));

    assert_eq!(
        queue.pop_front().expect("invocation").action_id,
        ActionId::new("second")
    );
}
