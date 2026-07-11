//! Scoped runtime coordinate and clipping conformance coverage.

use kinetik_ui_core::{
    ClipId, CursorShape, InputWheelDelta, Key, KeyEvent, KeyState, Modifiers, MouseButton,
    PlatformRequest, Point, PointerButtonState, Primitive, Rect, SemanticActionKind, SemanticNode,
    SemanticRole, TextInputEvent, Transform, UiInput, UiInputEvent, UiTestHarness, Vec2, WidgetId,
    draggable, pressable,
};

fn assert_point_close(actual: Point, expected: Point) {
    assert!((actual.x - expected.x).abs() < 1.0e-4, "x: {actual:?}");
    assert!((actual.y - expected.y).abs() < 1.0e-4, "y: {actual:?}");
}

fn assert_vec_close(actual: Vec2, expected: Vec2) {
    assert!((actual.x - expected.x).abs() < 1.0e-4, "x: {actual:?}");
    assert!((actual.y - expected.y).abs() < 1.0e-4, "y: {actual:?}");
}

#[test]
fn scoped_input_composes_affine_vectors_for_every_accessor_and_restores_parent() {
    let mut harness = UiTestHarness::new();
    harness.input_mut().pointer.position = Some(Point::new(12.0, 36.0));
    harness.input_mut().pointer.delta = Vec2::ZERO;
    harness.input_mut().pointer.primary = PointerButtonState::new(true, true, false);
    let owner = WidgetId::from_key("scaled-drag");

    let _ = harness.run_frame(|ui| {
        ui.register_id(owner);
        ui.push_primitive(Primitive::TransformBegin(Transform::translation(
            Vec2::new(10.0, 20.0),
        )));
        ui.push_primitive(Primitive::TransformBegin(Transform::scale(Vec2::new(
            2.0, 4.0,
        ))));
        let (input, memory) = ui.input_and_memory_mut();
        let response = draggable(owner, Rect::new(0.0, 0.0, 10.0, 10.0), input, memory, false);
        ui.push_primitive(Primitive::TransformEnd);
        ui.push_primitive(Primitive::TransformEnd);
        response
    });

    harness.input_mut().events.clear();
    harness.input_mut().pointer.position = Some(Point::new(20.0, 44.0));
    harness.input_mut().pointer.delta = Vec2::new(8.0, 8.0);
    harness.input_mut().pointer.wheel_delta = Vec2::new(2.0, 4.0);
    harness.input_mut().pointer.primary = PointerButtonState::new(true, false, false);

    let (response, output) = harness.run_frame(|ui| {
        ui.register_id(owner);
        let root = ui.input().clone();
        ui.push_primitive(Primitive::TransformBegin(Transform::translation(
            Vec2::new(10.0, 20.0),
        )));
        ui.push_primitive(Primitive::TransformBegin(Transform::scale(Vec2::new(
            2.0, 4.0,
        ))));

        let scoped = ui.input().clone();
        assert_eq!(ui.context().input, scoped);
        assert_point_close(
            scoped.pointer.position.expect("localized point"),
            Point::new(5.0, 6.0),
        );
        assert_vec_close(scoped.pointer.delta, Vec2::new(4.0, 2.0));
        assert_vec_close(scoped.pointer.wheel_delta, Vec2::new(1.0, 1.0));
        let from_split_borrow = ui.input_and_memory_mut().0.clone();
        assert_eq!(from_split_borrow, scoped);

        let (input, memory) = ui.input_and_memory_mut();
        let response = draggable(owner, Rect::new(0.0, 0.0, 10.0, 10.0), input, memory, false);
        assert!(response.dragged);
        assert_vec_close(response.drag_delta, Vec2::new(4.0, 2.0));
        assert_eq!(response.rect, Rect::new(0.0, 0.0, 10.0, 10.0));

        ui.push_primitive(Primitive::TransformEnd);
        ui.push_primitive(Primitive::TransformEnd);
        assert_eq!(ui.input(), &root);
        assert_eq!(&ui.context().input, &root);
        response
    });

    assert!(response.dragged);
    assert!(output.warnings.is_empty());
}

#[test]
fn transformed_clip_uses_exact_region_instead_of_its_screen_aabb() {
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
    let clip = ClipId::from_raw(1);

    let mut outside = UiTestHarness::new();
    outside.set_pointer_position(Point::new(36.0, 51.0));
    outside.input_mut().pointer.delta = Vec2::new(8.0, 4.0);
    outside.wheel(Vec2::new(3.0, -2.0));
    outside.input_mut().pointer.primary = PointerButtonState::new(true, true, false);
    outside.input_mut().pointer.secondary = PointerButtonState::new(false, true, true);
    outside.input_mut().pointer.middle = PointerButtonState::new(true, true, false);
    outside.input_mut().pointer.other_buttons =
        vec![(8, PointerButtonState::new(true, true, false))];
    outside.input_mut().pointer.click_count = 2;
    outside.input_mut().events.clear();
    let ((), output) = outside.run_frame(|ui| {
        ui.push_primitive(Primitive::TransformBegin(rotated));
        ui.push_primitive(Primitive::ClipBegin {
            id: clip,
            rect: Rect::new(0.0, 0.0, 20.0, 20.0),
        });
        let direct = ui.input().clone();
        let context = ui.context().input.clone();
        let split = ui.input_and_memory_mut().0.clone();
        for scoped in [&direct, &context, &split] {
            assert_eq!(scoped.pointer.position, None);
            assert_eq!(scoped.pointer.delta, Vec2::ZERO);
            assert_eq!(scoped.pointer.wheel_delta, Vec2::ZERO);
            assert_eq!(scoped.pointer.primary, PointerButtonState::default());
            assert_eq!(scoped.pointer.secondary, PointerButtonState::default());
            assert_eq!(scoped.pointer.middle, PointerButtonState::default());
            assert!(scoped.pointer.other_buttons.is_empty());
            assert_eq!(scoped.pointer.click_count, 0);
        }
        ui.push_primitive(Primitive::ClipEnd { id: clip });
        ui.push_primitive(Primitive::TransformEnd);
        assert_eq!(ui.input().pointer.position, Some(Point::new(36.0, 51.0)));
        assert_eq!(
            ui.input().pointer.primary,
            PointerButtonState::new(true, true, false)
        );
        assert_eq!(ui.input().pointer.click_count, 2);
    });
    assert!(output.warnings.is_empty());

    let mut inside = UiTestHarness::new();
    inside.set_pointer_position(rotated.transform_point(Point::new(5.0, 5.0)));
    let (localized, output) = inside.run_frame(|ui| {
        ui.push_primitive(Primitive::TransformBegin(rotated));
        ui.push_primitive(Primitive::ClipBegin {
            id: clip,
            rect: Rect::new(0.0, 0.0, 20.0, 20.0),
        });
        let localized = ui
            .input()
            .pointer
            .position
            .expect("inside transformed clip");
        ui.push_primitive(Primitive::ClipEnd { id: clip });
        ui.push_primitive(Primitive::TransformEnd);
        localized
    });
    assert_point_close(localized, Point::new(5.0, 5.0));
    assert!(output.warnings.is_empty());
}

#[test]
fn finite_near_zero_transform_remains_invertible_and_spatially_visible() {
    let transform = Transform::scale(Vec2::new(1.0e-4, 1.0e-4));
    let clip = ClipId::from_raw(11);
    let node = WidgetId::from_key("tiny");
    let mut harness = UiTestHarness::new();
    harness.set_pointer_position(Point::new(5.0e-5, 5.0e-5));

    let (localized, output) = harness.run_frame(|ui| {
        ui.push_primitive(Primitive::TransformBegin(transform));
        ui.push_primitive(Primitive::ClipBegin {
            id: clip,
            rect: Rect::new(0.0, 0.0, 1.0, 1.0),
        });
        let localized = ui.input().pointer.position.expect("finite inverse");
        ui.push_semantic_node(SemanticNode::new(
            node,
            SemanticRole::Button,
            Rect::new(0.0, 0.0, 1.0, 1.0),
        ));
        ui.push_primitive(Primitive::ClipEnd { id: clip });
        ui.push_primitive(Primitive::TransformEnd);
        localized
    });

    assert_point_close(localized, Point::new(0.5, 0.5));
    assert_eq!(
        output.semantics.get(node).expect("tiny semantic").bounds,
        Rect::new(0.0, 0.0, 1.0e-4, 1.0e-4)
    );
}

#[test]
fn semantic_tree_projects_sheared_non_uniform_partial_clips_without_losing_edges() {
    let root = WidgetId::from_key("semantic-root");
    let child = WidgetId::from_key("semantic-child");
    let clip = ClipId::from_raw(2);
    let transform = Transform {
        m11: 1.0,
        m12: 0.0,
        m21: 1.0,
        m22: 2.0,
        dx: 10.0,
        dy: 20.0,
    };
    let mut harness = UiTestHarness::new();

    let ((), output) = harness.run_frame(|ui| {
        ui.push_primitive(Primitive::TransformBegin(transform));
        ui.push_primitive(Primitive::ClipBegin {
            id: clip,
            rect: Rect::new(0.0, 0.0, 10.0, 10.0),
        });
        ui.push_semantic_node(
            SemanticNode::new(root, SemanticRole::Root, Rect::new(0.0, 0.0, 10.0, 10.0))
                .with_children([child]),
        );
        ui.push_semantic_node(
            SemanticNode::new(child, SemanticRole::Button, Rect::new(5.0, 5.0, 10.0, 10.0))
                .focusable(true),
        );
        ui.push_primitive(Primitive::ClipEnd { id: clip });
        ui.push_primitive(Primitive::TransformEnd);
    });

    assert!(output.semantics.validate().is_ok());
    assert!(output.warnings.is_empty());
    assert_eq!(
        output.semantics.get(root).expect("root").bounds,
        Rect::new(10.0, 20.0, 20.0, 20.0)
    );
    assert_eq!(
        output.semantics.get(child).expect("child").bounds,
        Rect::new(20.0, 30.0, 10.0, 10.0)
    );
    assert_eq!(output.semantics.focus_order(), vec![child]);
}

#[test]
fn clipped_focused_text_blurs_and_emits_exactly_one_stop_in_the_same_frame() {
    let owner = WidgetId::from_key("clipped-field");
    let clip = ClipId::from_raw(3);
    let mut harness = UiTestHarness::new();
    harness.memory_mut().focus(owner);
    harness.memory_mut().set_text_input_owner(owner);

    let (started, output) = harness.run_frame(|ui| {
        ui.push_primitive(Primitive::ClipBegin {
            id: clip,
            rect: Rect::new(0.0, 0.0, 10.0, 10.0),
        });
        let started = ui.start_text_input(owner, Some(Rect::new(20.0, 20.0, 8.0, 8.0)));
        ui.push_semantic_node(
            SemanticNode::new(
                owner,
                SemanticRole::TextField,
                Rect::new(20.0, 20.0, 8.0, 8.0),
            )
            .focusable(true),
        );
        ui.push_platform_request(PlatformRequest::StartTextInput {
            rect: Some(Rect::new(20.0, 20.0, 8.0, 8.0)),
        });
        ui.push_primitive(Primitive::ClipEnd { id: clip });
        started
    });

    assert!(!started);
    assert_eq!(harness.memory().focused(), None);
    assert_eq!(harness.memory().text_input_owner(), None);
    assert_eq!(
        output.platform_requests,
        vec![PlatformRequest::StopTextInput]
    );
    let node = output.semantics.get(owner).expect("retained clipped node");
    assert_eq!(node.bounds, Rect::ZERO);
    assert!(!node.focusable);
    assert!(!node.state.focused);
    assert!(
        node.actions
            .iter()
            .all(|action| action.kind != SemanticActionKind::Focus)
    );
    assert!(output.semantics.validate().is_ok());
}

#[test]
fn helper_and_raw_ime_rectangles_share_transformed_partial_clip_projection() {
    let owner = WidgetId::from_key("ime-field");
    let clip = ClipId::from_raw(4);
    let mut harness = UiTestHarness::new();
    harness.memory_mut().focus(owner);

    let (started, output) = harness.run_frame(|ui| {
        ui.push_primitive(Primitive::TransformBegin(Transform::translation(
            Vec2::new(10.0, 20.0),
        )));
        ui.push_primitive(Primitive::ClipBegin {
            id: clip,
            rect: Rect::new(0.0, 0.0, 10.0, 10.0),
        });
        let started = ui.start_text_input(owner, Some(Rect::new(5.0, 5.0, 10.0, 10.0)));
        ui.push_platform_request(PlatformRequest::StartTextInput {
            rect: Some(Rect::new(5.0, 5.0, 10.0, 10.0)),
        });
        ui.push_primitive(Primitive::ClipEnd { id: clip });
        ui.push_primitive(Primitive::TransformEnd);
        started
    });

    let expected = PlatformRequest::StartTextInput {
        rect: Some(Rect::new(15.0, 25.0, 5.0, 5.0)),
    };
    assert!(started);
    assert_eq!(output.platform_requests, vec![expected.clone(), expected]);
}

#[test]
fn text_input_rect_updates_use_the_same_spatial_projection_without_restart() {
    let owner = WidgetId::from_key("ime-update-field");
    let clip = ClipId::from_raw(5);
    let mut harness = UiTestHarness::new();
    harness.memory_mut().focus(owner);
    harness.memory_mut().set_text_input_owner(owner);

    let (updated, output) = harness.run_frame(|ui| {
        ui.push_primitive(Primitive::TransformBegin(Transform::translation(
            Vec2::new(10.0, 20.0),
        )));
        ui.push_primitive(Primitive::ClipBegin {
            id: clip,
            rect: Rect::new(0.0, 0.0, 10.0, 10.0),
        });
        let updated = ui.start_text_input(owner, Some(Rect::new(5.0, 5.0, 10.0, 10.0)));
        ui.push_platform_request(PlatformRequest::UpdateTextInputRect {
            rect: Rect::new(5.0, 5.0, 10.0, 10.0),
        });
        ui.push_primitive(Primitive::ClipEnd { id: clip });
        ui.push_primitive(Primitive::TransformEnd);
        updated
    });

    let expected = PlatformRequest::UpdateTextInputRect {
        rect: Rect::new(15.0, 25.0, 5.0, 5.0),
    };
    assert!(updated);
    assert_eq!(output.platform_requests, vec![expected.clone(), expected]);
    assert!(
        !output
            .platform_requests
            .iter()
            .any(|request| matches!(request, PlatformRequest::StartTextInput { .. }))
    );
}

#[test]
fn singular_and_non_finite_scopes_are_inert_but_release_capture_and_restore_root_input() {
    let owner = WidgetId::from_key("captured-owner");
    let rect = Rect::new(0.0, 0.0, 20.0, 20.0);
    let mut harness = UiTestHarness::new();
    harness.set_pointer_position(Point::new(5.0, 5.0));
    harness.pointer_press(MouseButton::Primary);
    let (pressed, _) = harness.run_frame(|ui| {
        ui.register_id(owner);
        let (input, memory) = ui.input_and_memory_mut();
        draggable(owner, rect, input, memory, false)
    });
    assert!(pressed.state.active);
    assert_eq!(harness.memory().pointer_capture(), Some(owner));
    harness.memory_mut().focus(owner);
    harness.memory_mut().set_text_input_owner(owner);

    harness.set_pointer_position(Point::new(40.0, 40.0));
    harness.wheel(Vec2::new(9.0, -7.0));
    harness.pointer_release(MouseButton::Primary);
    let (response, output) = harness.run_frame(|ui| {
        let root = ui.input().clone();
        ui.push_primitive(Primitive::TransformBegin(Transform::scale(Vec2::new(
            0.0, 1.0,
        ))));
        assert_eq!(ui.input().pointer.position, None);
        assert_eq!(ui.input().pointer.delta, Vec2::ZERO);
        assert_eq!(ui.input().pointer.wheel_delta, Vec2::ZERO);
        assert_eq!(
            ui.input().pointer.primary,
            PointerButtonState::new(false, false, true)
        );
        assert_eq!(ui.input().pointer.secondary, PointerButtonState::default());
        let (input, memory) = ui.input_and_memory_mut();
        let response = draggable(owner, rect, input, memory, false);
        assert!(!response.clicked);
        assert!(!response.dragged);
        assert!(!ui.start_text_input(owner, Some(rect)));
        ui.push_semantic_node(
            SemanticNode::new(owner, SemanticRole::TextField, rect).focusable(true),
        );
        ui.push_platform_request(PlatformRequest::StartTextInput { rect: None });
        ui.push_primitive(Primitive::TransformEnd);
        assert_eq!(ui.input(), &root);

        ui.push_primitive(Primitive::TransformBegin(Transform {
            m11: f32::NAN,
            ..Transform::IDENTITY
        }));
        assert_eq!(ui.input().pointer.position, None);
        assert_eq!(ui.input().pointer.wheel_delta, Vec2::ZERO);
        ui.push_primitive(Primitive::TransformEnd);
        assert_eq!(ui.input(), &root);
        response
    });

    assert!(!response.clicked);
    assert!(!response.dragged);
    assert_eq!(harness.memory().pointer_capture(), None);
    assert_eq!(harness.memory().focused(), None);
    assert_eq!(harness.memory().text_input_owner(), None);
    assert_eq!(
        output.platform_requests,
        vec![PlatformRequest::StopTextInput]
    );
    assert!(output.warnings.is_empty());
}

#[test]
fn captured_owner_cannot_publish_cursor_outside_effective_clip() {
    let owner = WidgetId::from_key("captured-cursor-owner");
    let clip = ClipId::from_raw(41);
    let mut harness = UiTestHarness::new();
    harness.memory_mut().capture_pointer(owner);
    harness.set_pointer_position(Point::new(50.0, 50.0));

    let (published, output) = harness.run_frame(|ui| {
        ui.register_id(owner);
        ui.push_primitive(Primitive::ClipBegin {
            id: clip,
            rect: Rect::new(0.0, 0.0, 10.0, 10.0),
        });
        let published = ui.request_cursor_for(owner, CursorShape::Grabbing);
        ui.push_primitive(Primitive::ClipEnd { id: clip });
        published
    });

    assert!(!published);
    assert!(output.platform_requests.is_empty());
}

#[test]
fn invisible_secondary_release_only_cleans_up_existing_owner() {
    let owner = WidgetId::from_key("secondary-release-owner");
    let clip = ClipId::from_raw(42);
    let rect = Rect::new(0.0, 0.0, 10.0, 10.0);
    let mut harness = UiTestHarness::new();
    harness.memory_mut().press_secondary(owner);
    harness.set_pointer_position(Point::new(50.0, 50.0));
    harness.input_mut().pointer.primary = PointerButtonState::new(true, true, true);
    harness.input_mut().pointer.secondary = PointerButtonState::new(false, false, true);
    harness.input_mut().events.clear();

    let (response, output) = harness.run_frame(|ui| {
        ui.register_id(owner);
        ui.push_primitive(Primitive::ClipBegin { id: clip, rect });
        assert_eq!(ui.input().pointer.primary, PointerButtonState::default());
        assert_eq!(
            ui.input().pointer.secondary,
            PointerButtonState::new(false, false, true)
        );
        let (input, memory) = ui.input_and_memory_mut();
        let response = pressable(owner, rect, input, memory, false);
        ui.push_primitive(Primitive::ClipEnd { id: clip });
        response
    });

    assert!(!response.clicked);
    assert!(!response.secondary_clicked);
    assert_eq!(harness.memory().secondary_pressed(), None);
    assert!(output.platform_requests.is_empty());
}

fn ordered_spatial_input() -> UiInput {
    let mut input = UiInput::default();
    for event in [
        UiInputEvent::PointerMoved {
            position: Point::new(12.0, 24.0),
            delta: Vec2::new(2.0, 4.0),
        },
        UiInputEvent::PointerButton {
            button: MouseButton::Primary,
            down: true,
            click_count: 1,
            position: Some(Point::new(12.0, 24.0)),
        },
        UiInputEvent::PointerMoved {
            position: Point::new(50.0, 80.0),
            delta: Vec2::new(38.0, 56.0),
        },
        UiInputEvent::Wheel {
            delta: InputWheelDelta::Lines(Vec2::new(1.0, -1.0)),
            position: Some(Point::new(14.0, 28.0)),
        },
        UiInputEvent::Wheel {
            delta: InputWheelDelta::Pixels(Vec2::new(4.0, 8.0)),
            position: Some(Point::new(14.0, 28.0)),
        },
        UiInputEvent::PointerButton {
            button: MouseButton::Primary,
            down: false,
            click_count: 1,
            position: Some(Point::new(50.0, 80.0)),
        },
        UiInputEvent::PointerReleaseAll {
            position: Some(Point::new(50.0, 80.0)),
        },
        UiInputEvent::Key(KeyEvent::new(
            Key::Character("a".to_owned()),
            KeyState::Pressed,
            Modifiers::default(),
            false,
        )),
        UiInputEvent::Text(TextInputEvent::Commit("text".to_owned())),
    ] {
        input.push_event(event);
    }
    input
}

#[test]
fn ordered_pointer_events_localize_individually_and_clips_keep_only_release_cleanup() {
    let owner = WidgetId::from_key("ordered-capture");
    let clip = ClipId::from_raw(51);
    let input = ordered_spatial_input();
    assert_eq!(input.validate_event_stream(), Ok(()));

    let mut harness = UiTestHarness::new();
    *harness.input_mut() = input;
    harness.memory_mut().capture_pointer(owner);
    let (localized, output) = harness.run_frame(|ui| {
        ui.register_id(owner);
        ui.memory_mut().capture_pointer(owner);
        ui.push_primitive(Primitive::TransformBegin(Transform::translation(
            Vec2::new(10.0, 20.0),
        )));
        ui.push_primitive(Primitive::TransformBegin(Transform::scale(Vec2::new(
            2.0, 4.0,
        ))));
        ui.push_primitive(Primitive::ClipBegin {
            id: clip,
            rect: Rect::new(0.0, 0.0, 10.0, 10.0),
        });
        let localized = ui.input().clone();
        ui.push_primitive(Primitive::ClipEnd { id: clip });
        ui.push_primitive(Primitive::TransformEnd);
        ui.push_primitive(Primitive::TransformEnd);
        localized
    });

    assert_eq!(
        localized.validate_event_stream(),
        Err(kinetik_ui_core::InputStreamConflict::Pointer),
        "localized input: {localized:#?}"
    );
    assert_eq!(localized.pointer.position, None);
    assert_vec_close(localized.pointer.delta, Vec2::new(1.0, 1.0));
    assert_vec_close(localized.pointer.wheel_delta, Vec2::new(3.0, 1.0));
    assert!(localized.pointer.primary.pressed);
    assert!(localized.pointer.primary.released);
    assert!(localized
        .events
        .iter()
        .all(|event| !matches!(event, UiInputEvent::PointerMoved { position, .. } if *position == Point::new(20.0, 15.0))));

    let wheels = localized
        .events
        .iter()
        .filter_map(|event| match event {
            UiInputEvent::Wheel { delta, position } => Some((*delta, *position)),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        wheels,
        vec![
            (
                InputWheelDelta::Lines(Vec2::new(1.0, -1.0)),
                Some(Point::new(2.0, 2.0)),
            ),
            (
                InputWheelDelta::Pixels(Vec2::new(2.0, 2.0)),
                Some(Point::new(2.0, 2.0)),
            ),
        ]
    );
    assert!(localized.events.iter().any(|event| matches!(
        event,
        UiInputEvent::PointerButton {
            down: false,
            position: Some(Point { x: 20.0, y: 15.0 }),
            ..
        }
    )));
    assert!(
        localized
            .events
            .iter()
            .any(|event| matches!(event, UiInputEvent::PointerReleaseAll { .. }))
    );
    let non_pointer = localized
        .events
        .iter()
        .filter(|event| matches!(event, UiInputEvent::Key(_) | UiInputEvent::Text(_)))
        .cloned()
        .collect::<Vec<_>>();
    assert!(matches!(non_pointer[0], UiInputEvent::Key(_)));
    assert!(matches!(non_pointer[1], UiInputEvent::Text(_)));
    assert!(output.warnings.is_empty());
}

#[test]
fn spatial_localization_never_heals_a_root_pointer_projection_conflict() {
    let mut harness = UiTestHarness::new();
    harness.set_pointer_position(Point::new(8.0, 12.0));
    harness.input_mut().pointer.delta = Vec2::new(99.0, 99.0);
    harness.input_mut().pointer.click_count = 7;

    let ((localized, conflict), output) = harness.run_frame(|ui| {
        ui.push_primitive(Primitive::TransformBegin(Transform::scale(Vec2::new(
            2.0, 2.0,
        ))));
        let localized = ui.input().clone();
        let conflict = localized.validate_event_stream();
        ui.push_primitive(Primitive::TransformEnd);
        (localized, conflict)
    });

    assert_eq!(conflict, Err(kinetik_ui_core::InputStreamConflict::Pointer));
    assert_eq!(localized.pointer.position, Some(Point::new(4.0, 6.0)));
    assert_vec_close(localized.pointer.delta, Vec2::new(49.5, 49.5));
    assert_eq!(localized.pointer.click_count, 7);
    assert!(matches!(
        output.warnings.as_slice(),
        [kinetik_ui_core::FrameWarning::InputStreamConflict {
            conflict: kinetik_ui_core::InputStreamConflict::Pointer
        }]
    ));
}
