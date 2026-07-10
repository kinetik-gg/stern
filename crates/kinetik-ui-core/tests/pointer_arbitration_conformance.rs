//! Closed-world topmost pointer arbitration conformance.

use kinetik_ui_core::{
    ClipId, CursorShape, MouseButton, PlatformRequest, Point, PointerOrder, PointerPlanError,
    PointerRoute, PointerTarget, Rect, Size, Transform, UiTestHarness, Vec2, WidgetId, drop_target,
    pressable, scrollable,
};

const FULL: Rect = Rect::new(0.0, 0.0, 100.0, 100.0);

#[test]
fn explicit_paint_order_beats_behavior_evaluation_for_hover_press_click_and_cursor() {
    let base = WidgetId::from_key("base");
    let overlay = WidgetId::from_key("overlay");
    let mut harness = UiTestHarness::new();
    harness.set_pointer_position(Point::new(20.0, 20.0));
    harness.pointer_press(MouseButton::Primary);

    let ((base_press, overlay_press, base_cursor, overlay_cursor), output) =
        harness.run_frame(|ui| {
            ui.register_id(base);
            ui.register_id(overlay);
            ui.resolve_pointer_targets(|plan| {
                plan.target(PointerTarget::new(overlay, FULL, PointerOrder::new(20)));
                plan.target(PointerTarget::new(base, FULL, PointerOrder::new(10)));
            })
            .expect("valid target plan");

            let (input, memory) = ui.input_and_memory_mut();
            let base_press = pressable(base, FULL, input, memory, false);
            let (input, memory) = ui.input_and_memory_mut();
            let overlay_press = pressable(overlay, FULL, input, memory, false);
            let base_cursor = ui.request_cursor_for(base, CursorShape::PointingHand);
            let overlay_cursor = ui.request_cursor_for(overlay, CursorShape::Grabbing);
            (base_press, overlay_press, base_cursor, overlay_cursor)
        });

    assert!(!base_press.state.hovered);
    assert!(!base_press.state.active);
    assert!(overlay_press.state.hovered);
    assert!(overlay_press.state.active);
    assert_eq!(harness.memory().pointer_capture(), Some(overlay));
    assert!(!base_cursor);
    assert!(overlay_cursor);
    assert_eq!(
        output.platform_requests,
        vec![PlatformRequest::SetCursor(CursorShape::Grabbing)]
    );

    harness.pointer_release(MouseButton::Primary);
    let ((base_release, overlay_release), _) = harness.run_frame(|ui| {
        ui.register_id(base);
        ui.register_id(overlay);
        ui.resolve_pointer_targets(|plan| {
            plan.target(PointerTarget::new(base, FULL, PointerOrder::new(10)));
            plan.target(PointerTarget::new(overlay, FULL, PointerOrder::new(20)));
        })
        .expect("valid target plan");
        let (input, memory) = ui.input_and_memory_mut();
        let base_release = pressable(base, FULL, input, memory, false);
        let (input, memory) = ui.input_and_memory_mut();
        let overlay_release = pressable(overlay, FULL, input, memory, false);
        (base_release, overlay_release)
    });
    assert!(!base_release.clicked);
    assert!(overlay_release.clicked);
    assert_eq!(harness.memory().pointer_capture(), None);
}

#[test]
fn overlay_blocker_routes_submenu_and_blocks_viewport_wheel() {
    let viewport = WidgetId::from_key("viewport");
    let submenu = WidgetId::from_key("submenu");
    let submenu_rect = Rect::new(10.0, 10.0, 60.0, 60.0);
    let mut harness = UiTestHarness::new();
    harness.set_pointer_position(Point::new(20.0, 20.0));
    harness.input_mut().pointer.primary.pressed = true;
    harness.input_mut().pointer.primary.down = true;
    harness.wheel(Vec2::new(0.0, -30.0));

    let ((viewport_response, scroll, submenu_response), _) = harness.run_frame(|ui| {
        ui.register_id(viewport);
        ui.register_id(submenu);
        ui.resolve_pointer_targets(|plan| {
            plan.target(
                PointerTarget::new(viewport, FULL, PointerOrder::new(10)).wheel_owner(viewport),
            );
            plan.blocker(submenu_rect, PointerOrder::new(20));
            plan.target(PointerTarget::new(
                submenu,
                submenu_rect,
                PointerOrder::new(30),
            ));
        })
        .expect("valid overlay plan");
        let (input, memory) = ui.input_and_memory_mut();
        let viewport_response = pressable(viewport, FULL, input, memory, false);
        let (input, memory) = ui.input_and_memory_mut();
        let scroll = scrollable(
            viewport,
            FULL,
            Size::new(100.0, 300.0),
            input,
            memory,
            false,
        );
        let (input, memory) = ui.input_and_memory_mut();
        let submenu_response = pressable(submenu, submenu_rect, input, memory, false);
        (viewport_response, scroll, submenu_response)
    });

    assert!(!viewport_response.state.hovered);
    assert_eq!(scroll.delta, Vec2::ZERO);
    assert!(submenu_response.state.hovered);
    assert!(submenu_response.state.active);
}

#[test]
fn exact_nested_clip_skips_aabb_false_positive_and_singular_target() {
    let base = WidgetId::from_key("clip-base");
    let clipped = WidgetId::from_key("clipped-top");
    let singular = WidgetId::from_key("singular-top");
    let disabled = WidgetId::from_key("disabled-top");
    let fully_clipped = WidgetId::from_key("fully-clipped-top");
    let angle = std::f32::consts::FRAC_PI_4;
    let (sin, cos) = angle.sin_cos();
    let rotated = Transform {
        m11: cos,
        m12: sin,
        m21: -sin,
        m22: cos,
        dx: 50.0,
        dy: 50.0,
    };
    let mut harness = UiTestHarness::new();
    harness.set_pointer_position(Point::new(36.0, 51.0));

    let (routes, _) = harness.run_frame(|ui| {
        ui.resolve_pointer_targets(|plan| {
            plan.target(PointerTarget::new(base, FULL, PointerOrder::new(10)));
            plan.with_transform(rotated, |plan| {
                plan.with_clip(Rect::new(0.0, 0.0, 20.0, 20.0), |plan| {
                    plan.target(PointerTarget::new(
                        clipped,
                        Rect::new(0.0, 0.0, 20.0, 20.0),
                        PointerOrder::new(20),
                    ));
                });
            });
            plan.with_transform(Transform::scale(Vec2::new(0.0, 1.0)), |plan| {
                plan.target(PointerTarget::new(singular, FULL, PointerOrder::new(30)));
            });
            plan.target(PointerTarget::new(disabled, FULL, PointerOrder::new(40)).enabled(false));
            plan.with_clip(Rect::new(200.0, 200.0, 20.0, 20.0), |plan| {
                plan.target(PointerTarget::new(
                    fully_clipped,
                    FULL,
                    PointerOrder::new(50),
                ));
            });
        })
        .expect("valid nested spatial plan")
    });

    assert_eq!(routes.ordinary, PointerRoute::Target(base));
}

#[test]
fn closed_plan_cancels_non_captured_or_ineligible_owners_before_transition() {
    let lower = WidgetId::from_key("noncaptured-lower");
    let overlay = WidgetId::from_key("noncaptured-overlay");
    let mut harness = UiTestHarness::new();
    harness.set_pointer_position(Point::new(10.0, 10.0));
    harness.memory_mut().activate(lower);
    harness.memory_mut().press(lower);
    harness.memory_mut().press_secondary(lower);
    harness.input_mut().pointer.primary.released = true;
    harness.input_mut().pointer.secondary.released = true;

    let ((routes, response), _) = harness.run_frame(|ui| {
        ui.register_id(lower);
        ui.register_id(overlay);
        let routes = ui
            .resolve_pointer_targets(|plan| {
                plan.target(PointerTarget::new(lower, FULL, PointerOrder::new(10)));
                plan.target(PointerTarget::new(overlay, FULL, PointerOrder::new(20)));
            })
            .expect("valid transition plan");
        let (input, memory) = ui.input_and_memory_mut();
        let response = pressable(overlay, FULL, input, memory, false);
        (routes, response)
    });
    assert_eq!(routes.ordinary, PointerRoute::Target(overlay));
    assert!(!response.clicked);
    assert!(!response.secondary_clicked);
    assert!(
        !response.state.hovered,
        "cancellation suppresses this frame"
    );
    assert_eq!(harness.memory().active(), None);
    assert_eq!(harness.memory().pressed(), None);
    assert_eq!(harness.memory().secondary_pressed(), None);

    harness.memory_mut().capture_pointer(lower);
    harness.memory_mut().activate(lower);
    let (response, _) = harness.run_frame(|ui| {
        ui.register_id(lower);
        ui.register_id(overlay);
        ui.resolve_pointer_targets(|plan| {
            plan.target(PointerTarget::new(lower, FULL, PointerOrder::new(10)).enabled(false));
            plan.target(PointerTarget::new(overlay, FULL, PointerOrder::new(20)));
        })
        .expect("valid disabled transition plan");
        let (input, memory) = ui.input_and_memory_mut();
        pressable(overlay, FULL, input, memory, false)
    });
    assert!(!response.state.hovered);
    assert_eq!(harness.memory().pointer_capture(), None);

    harness.memory_mut().capture_pointer(lower);
    harness.memory_mut().activate(lower);
    harness.memory_mut().press(lower);
    harness.memory_mut().press_secondary(lower);
    harness.input_mut().pointer.primary.released = true;
    harness.input_mut().pointer.secondary.released = true;
    let (response, _) = harness.run_frame(|ui| {
        ui.register_id(overlay);
        ui.resolve_pointer_targets(|plan| {
            plan.target(PointerTarget::new(overlay, FULL, PointerOrder::new(20)));
        })
        .expect("valid removed-owner transition plan");
        let (input, memory) = ui.input_and_memory_mut();
        pressable(overlay, FULL, input, memory, false)
    });
    assert!(!response.clicked);
    assert!(!response.secondary_clicked);
    assert!(!response.state.hovered);
    assert_eq!(harness.memory().pointer_capture(), None);
    assert_eq!(harness.memory().active(), None);
    assert_eq!(harness.memory().pressed(), None);
    assert_eq!(harness.memory().secondary_pressed(), None);
}

#[test]
fn modal_barrier_cancels_lower_owners_and_blocks_click_through() {
    let lower = WidgetId::from_key("lower");
    let modal = WidgetId::from_key("modal");
    let child = WidgetId::from_key("modal-child");
    let dialog = Rect::new(30.0, 30.0, 40.0, 40.0);
    let child_rect = Rect::new(40.0, 40.0, 20.0, 20.0);
    let mut harness = UiTestHarness::new();
    harness.set_pointer_position(Point::new(10.0, 10.0));
    harness.pointer_press(MouseButton::Primary);
    let _ = harness.run_frame(|ui| {
        ui.register_id(lower);
        let (input, memory) = ui.input_and_memory_mut();
        pressable(lower, FULL, input, memory, false)
    });
    assert_eq!(harness.memory().pointer_capture(), Some(lower));
    harness.memory_mut().press_secondary(lower);

    harness.set_pointer_position(Point::new(50.0, 50.0));
    let ((routes, lower_response, child_response), _) = harness.run_frame(|ui| {
        ui.register_id(lower);
        ui.register_id(modal);
        ui.register_id(child);
        let routes = ui
            .resolve_pointer_targets(|plan| {
                plan.target(PointerTarget::new(lower, FULL, PointerOrder::new(10)));
                plan.capture_lower_layers(PointerOrder::new(100));
                plan.target(PointerTarget::new(modal, dialog, PointerOrder::new(110)));
                plan.target(PointerTarget::new(
                    child,
                    child_rect,
                    PointerOrder::new(120),
                ));
            })
            .expect("valid modal plan");
        let (input, memory) = ui.input_and_memory_mut();
        let lower_response = pressable(lower, FULL, input, memory, false);
        let (input, memory) = ui.input_and_memory_mut();
        let child_response = pressable(child, child_rect, input, memory, false);
        (routes, lower_response, child_response)
    });
    assert_eq!(routes.ordinary, PointerRoute::Target(child));
    assert!(!lower_response.state.hovered);
    assert!(
        !child_response.state.hovered,
        "cancellation suppresses this frame"
    );
    assert_eq!(harness.memory().pointer_capture(), None);
    assert_eq!(harness.memory().secondary_pressed(), None);

    harness.pointer_release(MouseButton::Primary);
    harness.set_pointer_position(Point::new(10.0, 10.0));
    let (lower_release, _) = harness.run_frame(|ui| {
        ui.register_id(lower);
        ui.register_id(modal);
        ui.resolve_pointer_targets(|plan| {
            plan.target(PointerTarget::new(lower, FULL, PointerOrder::new(10)));
            plan.capture_lower_layers(PointerOrder::new(100));
            plan.target(PointerTarget::new(modal, dialog, PointerOrder::new(110)));
        })
        .expect("valid modal guard");
        let (input, memory) = ui.input_and_memory_mut();
        pressable(lower, FULL, input, memory, false)
    });
    assert!(!lower_release.clicked);
    assert!(!lower_release.state.hovered);
}

#[test]
fn ordinary_press_wheel_viewport_and_drop_destination_have_independent_routes() {
    let row = WidgetId::from_key("row");
    let outer = WidgetId::from_key("outer-scroll");
    let inner = WidgetId::from_key("inner-scroll");
    let source = WidgetId::from_key("drag-source");
    let lower_drop = WidgetId::from_key("lower-drop");
    let upper_drop = WidgetId::from_key("upper-drop");
    let source_rect = Rect::new(120.0, 0.0, 20.0, 20.0);
    let mut harness = UiTestHarness::new();
    harness.memory_mut().capture_pointer(source);
    harness.memory_mut().activate(source);
    harness.memory_mut().press(source);
    harness.memory_mut().start_drag(source);
    harness.set_pointer_position(Point::new(20.0, 20.0));
    harness.input_mut().pointer.primary.down = true;
    harness.input_mut().pointer.delta = Vec2::new(2.0, 0.0);
    harness.wheel(Vec2::new(0.0, -25.0));

    let ((routes, outer_scroll, inner_scroll, lower, upper), _) = harness.run_frame(|ui| {
        ui.register_id(source);
        ui.register_id(outer);
        ui.register_id(inner);
        ui.register_id(lower_drop);
        ui.register_id(upper_drop);
        let routes = ui
            .resolve_pointer_targets(|plan| {
                plan.target(PointerTarget::new(
                    source,
                    source_rect,
                    PointerOrder::new(50),
                ));
                plan.target(
                    PointerTarget::new(lower_drop, FULL, PointerOrder::new(10))
                        .ordinary_owner(None)
                        .drop_owner(lower_drop),
                );
                plan.target(
                    PointerTarget::new(row, FULL, PointerOrder::new(30)).drop_owner(upper_drop),
                );
                plan.target(PointerTarget::wheel_only(outer, FULL, PointerOrder::new(5)));
                plan.target(PointerTarget::wheel_only(
                    inner,
                    FULL,
                    PointerOrder::new(25),
                ));
            })
            .expect("valid independent routes");
        let (input, memory) = ui.input_and_memory_mut();
        let outer_scroll = scrollable(outer, FULL, Size::new(100.0, 300.0), input, memory, false);
        let (input, memory) = ui.input_and_memory_mut();
        let inner_scroll = scrollable(inner, FULL, Size::new(100.0, 300.0), input, memory, false);
        let (input, memory) = ui.input_and_memory_mut();
        let lower = drop_target(lower_drop, FULL, input, memory, false);
        let (input, memory) = ui.input_and_memory_mut();
        let upper = drop_target(upper_drop, FULL, input, memory, false);
        (routes, outer_scroll, inner_scroll, lower, upper)
    });

    assert_eq!(routes.ordinary, PointerRoute::Target(source));
    assert_eq!(routes.drop, PointerRoute::Target(upper_drop));
    assert_eq!(routes.wheel, PointerRoute::Target(inner));
    assert_eq!(outer_scroll.delta, Vec2::ZERO);
    assert_eq!(inner_scroll.delta, Vec2::new(0.0, 25.0));
    assert!(!lower.response.state.hovered);
    assert_eq!(lower.source, None);
    assert!(upper.response.state.hovered);
    assert_eq!(upper.source, Some(source));
}

#[test]
fn cursor_equivalence_is_non_activating_and_plan_validation_fails_closed() {
    let behavior = WidgetId::from_key("behavior");
    let cursor_alias = WidgetId::from_key("cursor-alias");
    let unrelated = WidgetId::from_key("unrelated");
    let conflicting = WidgetId::from_key("conflicting");
    let mut harness = UiTestHarness::new();
    harness.set_pointer_position(Point::new(10.0, 10.0));
    harness.pointer_press(MouseButton::Primary);

    let ((behavior_response, alias_response, alias_cursor), _) = harness.run_frame(|ui| {
        ui.register_id(behavior);
        ui.register_id(cursor_alias);
        ui.resolve_pointer_targets(|plan| {
            plan.target(
                PointerTarget::new(behavior, FULL, PointerOrder::new(10))
                    .cursor_equivalent(cursor_alias),
            );
        })
        .expect("valid alias plan");
        let (input, memory) = ui.input_and_memory_mut();
        let behavior_response = pressable(behavior, FULL, input, memory, false);
        let (input, memory) = ui.input_and_memory_mut();
        let alias_response = pressable(cursor_alias, FULL, input, memory, false);
        let alias_cursor = ui.request_cursor_for(cursor_alias, CursorShape::Grabbing);
        (behavior_response, alias_response, alias_cursor)
    });
    assert!(behavior_response.state.active);
    assert!(!alias_response.state.hovered);
    assert!(!alias_response.state.active);
    assert!(alias_cursor);

    harness.pointer_release(MouseButton::Primary);
    let (errors, _) = harness.run_frame(|ui| {
        let duplicate_order = ui.resolve_pointer_targets(|plan| {
            plan.target(PointerTarget::new(behavior, FULL, PointerOrder::new(10)));
            plan.target(PointerTarget::new(unrelated, FULL, PointerOrder::new(10)));
        });
        let second_plan = ui.resolve_pointer_targets(|_| {});
        (duplicate_order, second_plan)
    });
    assert_eq!(
        errors.0,
        Err(PointerPlanError::DuplicateOrder(PointerOrder::new(10)))
    );
    assert_eq!(errors.1, Err(PointerPlanError::AlreadyInstalled));
    assert_eq!(harness.memory().pointer_route(), PointerRoute::Blocked);

    let (conflict, _) = harness.run_frame(|ui| {
        ui.resolve_pointer_targets(|plan| {
            plan.target(
                PointerTarget::new(behavior, FULL, PointerOrder::new(10))
                    .cursor_equivalent(conflicting),
            );
            plan.target(
                PointerTarget::new(unrelated, FULL, PointerOrder::new(20))
                    .cursor_equivalent(conflicting),
            );
        })
    });
    assert_eq!(
        conflict,
        Err(PointerPlanError::ConflictingWidgetId(conflicting))
    );
    assert_eq!(harness.memory().pointer_route(), PointerRoute::Blocked);

    let (unplanned_response, _) = harness.run_frame(|ui| {
        ui.register_id(unrelated);
        assert_eq!(ui.memory().pointer_route(), PointerRoute::Unplanned);
        let (input, memory) = ui.input_and_memory_mut();
        pressable(unrelated, FULL, input, memory, false)
    });
    assert!(unplanned_response.state.hovered);
}

#[test]
fn plan_clip_scope_can_start_inside_the_runtime_spatial_scope() {
    let owner = WidgetId::from_key("scoped-plan");
    let clip = ClipId::from_raw(99);
    let mut harness = UiTestHarness::new();
    harness.set_pointer_position(Point::new(15.0, 15.0));

    let (route, _) = harness.run_frame(|ui| {
        ui.push_primitive(kinetik_ui_core::Primitive::ClipBegin {
            id: clip,
            rect: Rect::new(10.0, 10.0, 20.0, 20.0),
        });
        let route = ui
            .resolve_pointer_targets(|plan| {
                plan.target(PointerTarget::new(
                    owner,
                    Rect::new(10.0, 10.0, 20.0, 20.0),
                    PointerOrder::new(1),
                ));
            })
            .expect("plan inherits runtime scope")
            .ordinary;
        ui.push_primitive(kinetik_ui_core::Primitive::ClipEnd { id: clip });
        route
    });
    assert_eq!(route, PointerRoute::Target(owner));
}
