use std::time::Duration;

use kinetik_ui_core::{
    MouseButton, Point, PointerOrder, PointerTarget, UiTestHarness, draggable,
    draggable_transformed, drop_target, drop_target_transformed,
};

use crate::support::{
    drag_transformed_source_to, local_target_rect, press_transformed_source, source_id,
    source_rect, start_drag_over_target, start_transformed_drag_over_target, target_id,
    target_rect, transformed_source_transform, transformed_target_transform,
};

#[test]
fn pointer_interaction_drop_target_reports_active_drag_source_over_target() {
    let mut harness = UiTestHarness::new();
    let source = start_drag_over_target(&mut harness);

    let drop = harness
        .run_frame(|ui| {
            ui.register_id(source);
            let target = target_id(ui);
            let (input, memory) = ui.input_and_memory_mut();
            drop_target(target, target_rect(), input, memory, false)
        })
        .0;

    assert_eq!(drop.source, Some(source));
    assert!(!drop.dropped);
    assert!(drop.response.state.hovered);
    assert_eq!(harness.memory().drag_source(), Some(source));
    assert_eq!(harness.memory().released_drag_source(), None);
}

#[test]
fn pointer_interaction_drop_target_accepts_released_drag_source_over_target() {
    for source_first in [true, false] {
        let mut harness = UiTestHarness::new();
        let source = start_drag_over_target(&mut harness);

        harness.pointer_release(MouseButton::Primary);
        let (source_response, target_response) = harness
            .run_frame(|ui| {
                let source = source_id(ui);
                let target = target_id(ui);
                ui.resolve_pointer_targets(|plan| {
                    plan.target(
                        PointerTarget::new(source, source_rect(), PointerOrder::new(20))
                            .domain_drag_source(),
                    );
                    plan.target(
                        PointerTarget::new(target, target_rect(), PointerOrder::new(30))
                            .ordinary_owner(None)
                            .drop_owner(target),
                    );
                })
                .expect("valid release drop plan");
                let (input, memory) = ui.input_and_memory_mut();

                if source_first {
                    let source_response = draggable(source, source_rect(), input, memory, false);
                    let target_response = drop_target(target, target_rect(), input, memory, false);
                    (source_response, target_response)
                } else {
                    let target_response = drop_target(target, target_rect(), input, memory, false);
                    let source_response = draggable(source, source_rect(), input, memory, false);
                    (source_response, target_response)
                }
            })
            .0;

        assert_eq!(source_response.id, source);
        assert_eq!(target_response.source, Some(source));
        assert!(target_response.dropped);
        assert!(target_response.response.state.hovered);
        assert_eq!(harness.memory().drag_source(), None);
        assert_eq!(harness.memory().released_drag_source(), Some(source));

        harness.advance_frame(Duration::from_millis(16));
        let released_source = harness.run_frame(|ui| ui.memory().released_drag_source()).0;
        assert_eq!(released_source, None);
    }
}

#[test]
fn pointer_interaction_transformed_drop_target_reports_active_and_released_sources() {
    let mut harness = UiTestHarness::new();
    let source = start_transformed_drag_over_target(&mut harness);

    let active = harness
        .run_frame(|ui| {
            ui.register_id(source);
            let target = target_id(ui);
            let (input, memory) = ui.input_and_memory_mut();
            drop_target_transformed(
                target,
                local_target_rect(),
                transformed_target_transform(),
                input,
                memory,
                false,
            )
        })
        .0;

    assert_eq!(active.source, Some(source));
    assert!(!active.dropped);
    assert!(active.response.state.hovered);
    assert_eq!(harness.memory().pointer_capture(), Some(source));
    assert_eq!(harness.memory().drag_source(), Some(source));

    for source_first in [true, false] {
        let mut released = UiTestHarness::new();
        let source = start_transformed_drag_over_target(&mut released);

        released.pointer_release(MouseButton::Primary);
        let (source_response, target_response) = released
            .run_frame(|ui| {
                let source = source_id(ui);
                let target = target_id(ui);
                ui.resolve_pointer_targets(|plan| {
                    plan.with_transform(transformed_source_transform(), |plan| {
                        plan.target(
                            PointerTarget::new(source, source_rect(), PointerOrder::new(20))
                                .domain_drag_source(),
                        );
                    });
                    plan.with_transform(transformed_target_transform(), |plan| {
                        plan.target(
                            PointerTarget::new(target, local_target_rect(), PointerOrder::new(30))
                                .ordinary_owner(None)
                                .drop_owner(target),
                        );
                    });
                })
                .expect("valid transformed release drop plan");
                let (input, memory) = ui.input_and_memory_mut();

                if source_first {
                    let source_response = draggable_transformed(
                        source,
                        source_rect(),
                        transformed_source_transform(),
                        input,
                        memory,
                        false,
                    );
                    let target_response = drop_target_transformed(
                        target,
                        local_target_rect(),
                        transformed_target_transform(),
                        input,
                        memory,
                        false,
                    );
                    (source_response, target_response)
                } else {
                    let target_response = drop_target_transformed(
                        target,
                        local_target_rect(),
                        transformed_target_transform(),
                        input,
                        memory,
                        false,
                    );
                    let source_response = draggable_transformed(
                        source,
                        source_rect(),
                        transformed_source_transform(),
                        input,
                        memory,
                        false,
                    );
                    (source_response, target_response)
                }
            })
            .0;

        assert_eq!(source_response.id, source);
        assert_eq!(target_response.source, Some(source));
        assert!(target_response.dropped);
        assert!(target_response.response.state.hovered);
        assert_eq!(released.memory().released_drag_source(), Some(source));
    }
}

#[test]
fn pointer_interaction_drop_target_rejects_self_disabled_and_missed_releases() {
    let mut harness = UiTestHarness::new();

    harness.set_pointer_position(Point::new(10.0, 10.0));
    harness.pointer_press(MouseButton::Primary);
    let _ = harness.run_frame(|ui| {
        let source = source_id(ui);
        let (input, memory) = ui.input_and_memory_mut();
        draggable(source, source_rect(), input, memory, false)
    });
    harness.set_pointer_position(Point::new(20.0, 10.0));
    let _ = harness.run_frame(|ui| {
        let source = source_id(ui);
        let (input, memory) = ui.input_and_memory_mut();
        draggable(source, source_rect(), input, memory, false)
    });
    harness.pointer_release(MouseButton::Primary);
    let self_drop = harness
        .run_frame(|ui| {
            let source = source_id(ui);
            let (input, memory) = ui.input_and_memory_mut();
            let source_response = draggable(source, source_rect(), input, memory, false);
            let target_response = drop_target(source, source_rect(), input, memory, false);
            (source_response, target_response)
        })
        .0;
    assert_eq!(self_drop.1.source, None);
    assert!(!self_drop.1.dropped);

    let mut disabled = UiTestHarness::new();
    disabled.set_pointer_position(Point::new(10.0, 10.0));
    disabled.pointer_press(MouseButton::Primary);
    let _ = disabled.run_frame(|ui| {
        let source = source_id(ui);
        let (input, memory) = ui.input_and_memory_mut();
        draggable(source, source_rect(), input, memory, false)
    });
    disabled.set_pointer_position(Point::new(50.0, 10.0));
    let _ = disabled.run_frame(|ui| {
        let source = source_id(ui);
        let (input, memory) = ui.input_and_memory_mut();
        draggable(source, source_rect(), input, memory, false)
    });
    disabled.set_pointer_position(Point::new(90.0, 10.0));
    disabled.pointer_release(MouseButton::Primary);
    let disabled_drop = disabled
        .run_frame(|ui| {
            let source = source_id(ui);
            let target = target_id(ui);
            let (input, memory) = ui.input_and_memory_mut();
            draggable(source, source_rect(), input, memory, false);
            drop_target(target, target_rect(), input, memory, true)
        })
        .0;
    assert!(disabled_drop.response.state.disabled);
    assert_eq!(disabled_drop.source, None);
    assert!(!disabled_drop.dropped);

    let mut missed = UiTestHarness::new();
    missed.set_pointer_position(Point::new(10.0, 10.0));
    missed.pointer_press(MouseButton::Primary);
    let _ = missed.run_frame(|ui| {
        let source = source_id(ui);
        let (input, memory) = ui.input_and_memory_mut();
        draggable(source, source_rect(), input, memory, false)
    });
    missed.set_pointer_position(Point::new(50.0, 10.0));
    let _ = missed.run_frame(|ui| {
        let source = source_id(ui);
        let (input, memory) = ui.input_and_memory_mut();
        draggable(source, source_rect(), input, memory, false)
    });
    missed.set_pointer_position(Point::new(160.0, 10.0));
    missed.pointer_release(MouseButton::Primary);
    let (_, missed_drop) = missed
        .run_frame(|ui| {
            let source = source_id(ui);
            let target = target_id(ui);
            let (input, memory) = ui.input_and_memory_mut();
            (
                draggable(source, source_rect(), input, memory, false),
                drop_target(target, target_rect(), input, memory, false),
            )
        })
        .0;
    assert_eq!(missed_drop.source, None);
    assert!(!missed_drop.dropped);
}

#[test]
fn pointer_interaction_transformed_drop_target_rejects_self_source() {
    let mut harness = UiTestHarness::new();
    press_transformed_source(&mut harness, Point::new(110.0, 10.0));
    drag_transformed_source_to(&mut harness, Point::new(120.0, 10.0));
    harness.pointer_release(MouseButton::Primary);

    let (_, target_response) = harness
        .run_frame(|ui| {
            let source = source_id(ui);
            let (input, memory) = ui.input_and_memory_mut();
            (
                draggable_transformed(
                    source,
                    source_rect(),
                    transformed_source_transform(),
                    input,
                    memory,
                    false,
                ),
                drop_target_transformed(
                    source,
                    source_rect(),
                    transformed_source_transform(),
                    input,
                    memory,
                    false,
                ),
            )
        })
        .0;

    assert_eq!(target_response.source, None);
    assert!(!target_response.dropped);
}

#[test]
fn pointer_interaction_transformed_drop_target_rejects_disabled_target() {
    let mut harness = UiTestHarness::new();
    let _ = start_transformed_drag_over_target(&mut harness);
    harness.pointer_release(MouseButton::Primary);

    let disabled_drop = harness
        .run_frame(|ui| {
            let source = source_id(ui);
            let target = target_id(ui);
            let (input, memory) = ui.input_and_memory_mut();
            draggable_transformed(
                source,
                source_rect(),
                transformed_source_transform(),
                input,
                memory,
                false,
            );
            drop_target_transformed(
                target,
                local_target_rect(),
                transformed_target_transform(),
                input,
                memory,
                true,
            )
        })
        .0;

    assert!(disabled_drop.response.state.disabled);
    assert_eq!(disabled_drop.source, None);
    assert!(!disabled_drop.dropped);
}

#[test]
fn pointer_interaction_transformed_drop_target_rejects_missed_target() {
    let mut harness = UiTestHarness::new();
    press_transformed_source(&mut harness, Point::new(110.0, 10.0));
    drag_transformed_source_to(&mut harness, Point::new(400.0, 10.0));
    harness.pointer_release(MouseButton::Primary);

    let (_, missed_drop) = harness
        .run_frame(|ui| {
            let source = source_id(ui);
            let target = target_id(ui);
            let (input, memory) = ui.input_and_memory_mut();
            (
                draggable_transformed(
                    source,
                    source_rect(),
                    transformed_source_transform(),
                    input,
                    memory,
                    false,
                ),
                drop_target_transformed(
                    target,
                    local_target_rect(),
                    transformed_target_transform(),
                    input,
                    memory,
                    false,
                ),
            )
        })
        .0;

    assert_eq!(missed_drop.source, None);
    assert!(!missed_drop.dropped);
}
