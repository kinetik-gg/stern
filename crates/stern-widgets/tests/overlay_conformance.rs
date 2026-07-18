//! Windowless overlay stack, placement, and semantic conformance tests.

use stern_core::{Point, Rect, SemanticActionKind, SemanticRole, Size};
use stern_widgets::{
    OverlayDismissal, OverlayEntry, OverlayId, OverlayKind, OverlayStack, PopoverPlacement,
    PopoverRequest, overlay_semantics, place_popover,
};

fn id(raw: u64) -> OverlayId {
    OverlayId::from_raw(raw)
}

fn entry(raw: u64, kind: OverlayKind, rect: Rect) -> OverlayEntry {
    OverlayEntry::new(id(raw), kind, rect)
}

#[test]
fn overlay_conformance_stack_order_replacement_and_descendant_closure_are_deterministic() {
    let mut stack = OverlayStack::new();

    stack.open(entry(
        1,
        OverlayKind::Menu,
        Rect::new(0.0, 0.0, 100.0, 100.0),
    ));
    assert!(
        stack.open_child(
            id(1),
            entry(2, OverlayKind::Popover, Rect::new(100.0, 0.0, 80.0, 60.0),)
                .dismiss_on(OverlayDismissal::OutsideClickOrEscape),
        )
    );
    stack.open(entry(
        3,
        OverlayKind::Tooltip,
        Rect::new(24.0, 24.0, 40.0, 20.0),
    ));
    stack.open(
        entry(
            2,
            OverlayKind::ContextMenu,
            Rect::new(40.0, 40.0, 90.0, 80.0),
        )
        .dismiss_on(OverlayDismissal::OutsideClick),
    );

    assert_eq!(
        stack
            .entries()
            .iter()
            .map(|overlay| overlay.id)
            .collect::<Vec<_>>(),
        vec![id(1), id(3), id(2)]
    );
    assert_eq!(
        stack.top().map(|overlay| overlay.kind),
        Some(OverlayKind::ContextMenu)
    );
    assert_eq!(stack.entries()[2].parent, None);

    assert!(stack.open_child(
        id(2),
        entry(4, OverlayKind::Menu, Rect::new(130.0, 40.0, 80.0, 80.0),),
    ));
    assert!(stack.open_child(
        id(4),
        entry(5, OverlayKind::Menu, Rect::new(210.0, 40.0, 80.0, 80.0),),
    ));

    assert_eq!(stack.close(id(2)).map(|overlay| overlay.id), Some(id(2)));
    assert_eq!(
        stack
            .entries()
            .iter()
            .map(|overlay| overlay.id)
            .collect::<Vec<_>>(),
        vec![id(1), id(3)]
    );
}

#[test]
fn overlay_conformance_children_require_present_parent_and_close_with_parent() {
    let mut stack = OverlayStack::new();

    assert!(!stack.open_child(
        id(1),
        entry(2, OverlayKind::Popover, Rect::new(20.0, 20.0, 80.0, 60.0),),
    ));
    assert!(stack.entries().is_empty());

    stack.open(entry(
        1,
        OverlayKind::Menu,
        Rect::new(0.0, 0.0, 100.0, 100.0),
    ));
    assert!(stack.open_child(
        id(1),
        entry(2, OverlayKind::Menu, Rect::new(100.0, 0.0, 80.0, 80.0),),
    ));
    assert!(stack.open_child(
        id(2),
        entry(
            3,
            OverlayKind::ContextMenu,
            Rect::new(180.0, 0.0, 80.0, 80.0),
        ),
    ));

    assert_eq!(
        stack
            .entries()
            .iter()
            .map(|overlay| (overlay.id, overlay.parent))
            .collect::<Vec<_>>(),
        vec![(id(1), None), (id(2), Some(id(1))), (id(3), Some(id(2)))]
    );

    stack.close(id(1));
    assert!(stack.entries().is_empty());
}

#[test]
fn overlay_conformance_overlay_stack_open_child_rejects_parent_cycles_without_mutation() {
    let mut stack = OverlayStack::new();

    stack.open(entry(
        1,
        OverlayKind::Menu,
        Rect::new(0.0, 0.0, 100.0, 100.0),
    ));
    assert!(stack.open_child(
        id(1),
        entry(2, OverlayKind::Menu, Rect::new(100.0, 0.0, 80.0, 80.0),),
    ));
    assert!(stack.open_child(
        id(2),
        entry(3, OverlayKind::Menu, Rect::new(180.0, 0.0, 80.0, 80.0),),
    ));

    let before_self_parent = stack.entries().to_vec();
    assert!(!stack.open_child(
        id(2),
        entry(2, OverlayKind::Popover, Rect::new(200.0, 0.0, 80.0, 60.0),),
    ));
    assert_eq!(stack.entries(), before_self_parent.as_slice());

    let before_ancestor_replacement = stack.entries().to_vec();
    assert!(!stack.open_child(
        id(3),
        entry(
            1,
            OverlayKind::ContextMenu,
            Rect::new(260.0, 0.0, 80.0, 80.0),
        ),
    ));
    assert_eq!(stack.entries(), before_ancestor_replacement.as_slice());
}

#[test]
fn overlay_conformance_overlay_stack_open_child_allows_non_ancestor_replacement() {
    let mut stack = OverlayStack::new();

    stack.open(entry(
        1,
        OverlayKind::Menu,
        Rect::new(0.0, 0.0, 100.0, 100.0),
    ));
    assert!(stack.open_child(
        id(1),
        entry(2, OverlayKind::Menu, Rect::new(100.0, 0.0, 80.0, 80.0),),
    ));
    assert!(stack.open_child(
        id(2),
        entry(4, OverlayKind::Tooltip, Rect::new(120.0, 20.0, 40.0, 20.0),),
    ));
    assert!(stack.open_child(
        id(1),
        entry(3, OverlayKind::Menu, Rect::new(180.0, 0.0, 80.0, 80.0),),
    ));

    assert!(stack.open_child(
        id(3),
        entry(
            2,
            OverlayKind::ContextMenu,
            Rect::new(260.0, 0.0, 80.0, 80.0),
        ),
    ));

    assert_eq!(
        stack
            .entries()
            .iter()
            .map(|overlay| (overlay.id, overlay.parent, overlay.kind))
            .collect::<Vec<_>>(),
        vec![
            (id(1), None, OverlayKind::Menu),
            (id(3), Some(id(1)), OverlayKind::Menu),
            (id(2), Some(id(3)), OverlayKind::ContextMenu),
        ]
    );
}

#[test]
fn overlay_conformance_modal_blocks_lower_layers_but_not_higher_hits() {
    let mut stack = OverlayStack::new();

    stack.open(entry(
        1,
        OverlayKind::Popover,
        Rect::new(0.0, 0.0, 200.0, 200.0),
    ));
    stack.open(
        entry(2, OverlayKind::Modal, Rect::new(40.0, 40.0, 80.0, 80.0))
            .dismiss_on(OverlayDismissal::OutsideClickOrEscape),
    );
    stack.open(entry(
        3,
        OverlayKind::Tooltip,
        Rect::new(160.0, 160.0, 24.0, 24.0),
    ));

    assert_eq!(
        stack
            .topmost_at(Point::new(170.0, 170.0))
            .map(|overlay| overlay.id),
        Some(id(3))
    );
    assert_eq!(
        stack.pointer_capture_target(Point::new(170.0, 170.0)),
        Some(id(3))
    );
    assert_eq!(
        stack.pointer_capture_target(Point::new(10.0, 10.0)),
        Some(id(2))
    );
    assert_eq!(
        stack.pointer_capture_target(Point::new(300.0, 300.0)),
        Some(id(2))
    );
}

#[test]
fn overlay_conformance_dismissal_requests_are_top_to_bottom_and_deduplicated() {
    let mut stack = OverlayStack::new();

    stack.open(
        entry(1, OverlayKind::Menu, Rect::new(0.0, 0.0, 80.0, 80.0))
            .dismiss_on(OverlayDismissal::OutsideClick),
    );
    stack.open(
        entry(2, OverlayKind::Dropdown, Rect::new(90.0, 0.0, 80.0, 80.0))
            .dismiss_on(OverlayDismissal::OutsideClickOrEscape),
    );
    stack.open(
        entry(
            3,
            OverlayKind::CommandPalette,
            Rect::new(180.0, 0.0, 80.0, 80.0),
        )
        .dismiss_on(OverlayDismissal::OutsideClickOrEscape),
    );

    assert_eq!(
        stack.outside_click_close_requests(Point::new(300.0, 300.0)),
        vec![id(3), id(2), id(1)]
    );
    assert_eq!(stack.escape_close_request(), Some(id(3)));
    assert_eq!(
        stack.dismissal_requests(Some(Point::new(300.0, 300.0)), true),
        vec![id(3), id(2), id(1)]
    );

    stack.close(id(3));
    assert_eq!(
        stack.dismissal_requests(Some(Point::new(300.0, 300.0)), true),
        vec![id(2), id(1)]
    );
}

#[test]
fn overlay_conformance_non_focusable_overlays_do_not_steal_focus_target() {
    let mut stack = OverlayStack::new();

    stack.open(entry(
        1,
        OverlayKind::Tooltip,
        Rect::new(0.0, 0.0, 80.0, 20.0),
    ));
    assert_eq!(stack.focus_target(), None);

    stack.open(entry(
        2,
        OverlayKind::Menu,
        Rect::new(0.0, 24.0, 120.0, 120.0),
    ));
    stack.open(entry(
        3,
        OverlayKind::DragPreview,
        Rect::new(20.0, 40.0, 32.0, 32.0),
    ));
    assert_eq!(stack.focus_target(), Some(id(2)));

    stack.open(entry(4, OverlayKind::Popover, Rect::new(40.0, 40.0, 80.0, 80.0)).modal(true));
    assert_eq!(stack.focus_target(), Some(id(4)));
}

#[test]
fn overlay_conformance_popover_flips_and_clamps_in_non_origin_viewport() {
    let rect = place_popover(
        PopoverRequest {
            anchor: Rect::new(185.0, 70.0, 10.0, 10.0),
            size: Size::new(40.0, 32.0),
            placement: PopoverPlacement::Right,
            offset: 6.0,
            fit_viewport: true,
        },
        Rect::new(100.0, 50.0, 100.0, 90.0),
    );

    assert_eq!(rect, Rect::new(139.0, 70.0, 40.0, 32.0));
}

#[test]
fn overlay_conformance_popover_oversize_outputs_remain_deterministic() {
    let rect = place_popover(
        PopoverRequest {
            anchor: Rect::new(180.0, 120.0, 10.0, 10.0),
            size: Size::new(140.0, 120.0),
            placement: PopoverPlacement::Below,
            offset: 4.0,
            fit_viewport: true,
        },
        Rect::new(100.0, 50.0, 80.0, 70.0),
    );

    assert_eq!(rect, Rect::new(100.0, 50.0, 80.0, 70.0));
}

#[test]
fn overlay_conformance_semantics_expose_role_focusability_and_dismiss_action() {
    let command_palette = entry(
        1,
        OverlayKind::CommandPalette,
        Rect::new(20.0, 20.0, 320.0, 200.0),
    )
    .dismiss_on(OverlayDismissal::Escape);
    let tooltip = entry(2, OverlayKind::Tooltip, Rect::new(20.0, 230.0, 120.0, 24.0));
    let modal_popover = entry(3, OverlayKind::Popover, Rect::new(40.0, 40.0, 160.0, 120.0))
        .modal(true)
        .dismiss_on(OverlayDismissal::OutsideClick);

    let command_node = overlay_semantics(&command_palette, "Commands");
    let tooltip_node = overlay_semantics(&tooltip, "Name hint");
    let modal_node = overlay_semantics(&modal_popover, "Inspector options");

    assert_eq!(command_node.role, SemanticRole::CommandPalette);
    assert_eq!(command_node.label.as_deref(), Some("Commands"));
    assert!(command_node.focusable);
    assert!(
        command_node
            .actions
            .iter()
            .any(|action| action.kind == SemanticActionKind::Focus)
    );
    assert!(
        command_node
            .actions
            .iter()
            .any(|action| action.kind == SemanticActionKind::Dismiss)
    );

    assert_eq!(
        tooltip_node.role,
        SemanticRole::Custom("tooltip".to_owned())
    );
    assert_eq!(tooltip_node.label.as_deref(), Some("Name hint"));
    assert!(!tooltip_node.focusable);
    assert!(tooltip_node.actions.is_empty());

    assert_eq!(modal_node.role, SemanticRole::Custom("popover".to_owned()));
    assert!(modal_node.focusable);
    assert!(
        modal_node
            .actions
            .iter()
            .any(|action| action.kind == SemanticActionKind::Dismiss)
    );
}
