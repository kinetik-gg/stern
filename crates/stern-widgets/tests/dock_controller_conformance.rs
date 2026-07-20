//! Public Dock controller input and persistence conformance tests.

use std::time::Duration;

use stern_core::{
    Axis, CursorShape, FrameContext, Key, KeyEvent, KeyState, KeyboardInput, Modifiers,
    MouseButton, PhysicalSize, PlatformRequest, Point, PointerOrder, Rect, RepaintRequest,
    ScaleFactor, Size, TimeInfo, UiInput, UiInputEvent, UiMemory, Vec2, ViewportInfo, WidgetId,
    default_dark_theme,
};
use stern_widgets::Ui;
use stern_widgets::dock::{
    Dock, DockController, DockControllerConfig, DockControllerOutput, DockNode, DockScene,
    DockSceneConfig, DockSnapshotNode, DockSplitterContextActionKind, Frame, FrameId, Panel,
    PanelId, PanelInstanceId, PanelInstanceLocation, PanelInstanceSnapshot, PanelTypeDescriptor,
    PanelTypeId,
};

const BOUNDS: Rect = Rect::new(0.0, 0.0, 600.0, 400.0);
const ROOT: WidgetId = WidgetId::from_raw(0xD0C0);

fn panel(id: u64, title: &str) -> Panel {
    Panel::new(PanelId::from_raw(id), title)
}

fn frame(id: u64, panels: Vec<Panel>) -> Frame {
    Frame::new(FrameId::from_raw(id), panels)
}

fn split_dock() -> Dock {
    split_dock_with_axis(Axis::Horizontal)
}

fn split_dock_with_axis(axis: Axis) -> Dock {
    Dock::new(DockNode::Split {
        axis,
        ratio: 0.5,
        min_first: 0.0,
        min_second: 0.0,
        first: Box::new(DockNode::Frame(frame(
            1,
            vec![
                panel(11, "Assets"),
                panel(12, "Inspector"),
                panel(13, "Details"),
            ],
        ))),
        second: Box::new(DockNode::Frame(frame(2, vec![panel(21, "Viewport")]))),
    })
}

fn replacement_split_dock() -> Dock {
    Dock::new(DockNode::Split {
        axis: Axis::Horizontal,
        ratio: 0.25,
        min_first: 24.0,
        min_second: 36.0,
        first: Box::new(DockNode::Frame(frame(
            7,
            vec![panel(71, "Replacement left")],
        ))),
        second: Box::new(DockNode::Split {
            axis: Axis::Vertical,
            ratio: 0.6,
            min_first: 18.0,
            min_second: 22.0,
            first: Box::new(DockNode::Frame(frame(
                8,
                vec![panel(81, "Replacement top")],
            ))),
            second: Box::new(DockNode::Frame(frame(
                9,
                vec![panel(91, "Replacement bottom")],
            ))),
        }),
    })
}

fn root_split_ratio(dock: &Dock) -> f32 {
    match dock.snapshot().root {
        DockSnapshotNode::Split { ratio, .. } => ratio,
        DockSnapshotNode::Frame { .. } => panic!("expected root split"),
    }
}

fn snapshot_bytes(dock: &Dock) -> Vec<u8> {
    format!("{:?}", dock.snapshot()).into_bytes()
}

fn context(input: UiInput) -> FrameContext {
    FrameContext::new(
        ViewportInfo::new(
            Size::new(600.0, 400.0),
            PhysicalSize::new(600, 400),
            ScaleFactor::ONE,
        ),
        input,
        TimeInfo::new(Duration::from_millis(500), Duration::from_millis(16), 1),
    )
}

fn scene(dock: &Dock) -> DockScene {
    scene_with_disabled(dock, false)
}

fn scene_with_disabled(dock: &Dock, disabled: bool) -> DockScene {
    DockScene::new(DockSceneConfig::new(ROOT, BOUNDS).disabled(disabled), dock)
}

fn run_frame(
    dock: &mut Dock,
    controller: &mut DockController,
    memory: &mut UiMemory,
    input: UiInput,
    new_frame: FrameId,
) -> (DockControllerOutput, stern_core::FrameOutput) {
    run_frame_with_disabled(dock, controller, memory, input, new_frame, false)
}

fn run_frame_with_disabled(
    dock: &mut Dock,
    controller: &mut DockController,
    memory: &mut UiMemory,
    input: UiInput,
    new_frame: FrameId,
    disabled: bool,
) -> (DockControllerOutput, stern_core::FrameOutput) {
    let prepared = scene_with_disabled(dock, disabled);
    run_frame_with_scene(dock, controller, memory, input, new_frame, &prepared)
}

fn run_frame_with_scene(
    dock: &mut Dock,
    controller: &mut DockController,
    memory: &mut UiMemory,
    input: UiInput,
    new_frame: FrameId,
    prepared: &DockScene,
) -> (DockControllerOutput, stern_core::FrameOutput) {
    let theme = default_dark_theme();
    let mut ui = Ui::begin_frame(context(input), memory, &theme);
    ui.resolve_pointer_targets(|plan| {
        prepared.declare_pointer_targets(plan, PointerOrder::new(100));
    })
    .expect("valid Dock pointer plan");
    let output = ui.dock_controller(
        prepared,
        dock,
        controller,
        DockControllerConfig::new(new_frame),
    );
    let _ = ui.dock_scene(prepared, |_, _| ());
    let frame = ui.finish_output();
    (output, frame)
}

fn pointer_button(point: Point, button: MouseButton, down: bool) -> UiInput {
    let mut input = UiInput::default();
    input.push_event(UiInputEvent::PointerButton {
        button,
        down,
        click_count: 1,
        position: Some(point),
    });
    input
}

fn pointer_move(point: Point, delta: Vec2) -> UiInput {
    let mut input = UiInput::default();
    input.pointer.primary.down = true;
    input.push_event(UiInputEvent::PointerMoved {
        position: point,
        delta,
    });
    input
}

fn pointer_release_all(point: Point) -> UiInput {
    let mut input = UiInput::default();
    input.push_event(UiInputEvent::PointerReleaseAll {
        position: Some(point),
    });
    input
}

fn window_focus_lost() -> UiInput {
    let mut input = UiInput::default();
    input.push_event(UiInputEvent::WindowFocusChanged(false));
    input
}

fn pointer_hover(point: Point) -> UiInput {
    let mut input = UiInput::default();
    input.pointer.position = Some(point);
    input
}

fn key_input(key: Key, ctrl: bool) -> UiInput {
    let modifiers = Modifiers::new(false, ctrl, false, false);
    UiInput {
        keyboard: KeyboardInput {
            modifiers,
            events: vec![KeyEvent::new(key, KeyState::Pressed, modifiers, false)],
        },
        ..UiInput::default()
    }
}

fn key_event_input(key: Key, state: KeyState, modifiers: Modifiers) -> UiInput {
    UiInput {
        keyboard: KeyboardInput {
            modifiers,
            events: vec![KeyEvent::new(key, state, modifiers, false)],
        },
        ..UiInput::default()
    }
}

fn center(rect: Rect) -> Point {
    Point::new(rect.x + rect.width * 0.5, rect.y + rect.height * 0.5)
}

fn click(
    point: Point,
    dock: &mut Dock,
    controller: &mut DockController,
    memory: &mut UiMemory,
    new_frame: FrameId,
) -> (DockControllerOutput, stern_core::FrameOutput) {
    let _ = run_frame(
        dock,
        controller,
        memory,
        pointer_button(point, MouseButton::Primary, true),
        new_frame,
    );
    run_frame(
        dock,
        controller,
        memory,
        pointer_button(point, MouseButton::Primary, false),
        new_frame,
    )
}

fn drag(
    from: Point,
    to: Point,
    dock: &mut Dock,
    controller: &mut DockController,
    memory: &mut UiMemory,
    new_frame: FrameId,
) -> (DockControllerOutput, DockControllerOutput) {
    let _ = run_frame(
        dock,
        controller,
        memory,
        pointer_button(from, MouseButton::Primary, true),
        new_frame,
    );
    let moved = run_frame(
        dock,
        controller,
        memory,
        pointer_move(to, Vec2::new(to.x - from.x, to.y - from.y)),
        new_frame,
    )
    .0;
    let released = run_frame(
        dock,
        controller,
        memory,
        pointer_button(to, MouseButton::Primary, false),
        new_frame,
    )
    .0;
    (moved, released)
}

#[test]
fn tab_click_selects_activates_focuses_and_close_only_emits_an_app_request() {
    let mut dock = split_dock();
    assert!(dock.set_active_frame(FrameId::from_raw(2)));
    let prepared = scene(&dock);
    let tab = prepared.layout().frames[0].tabs[1].clone();
    let mut controller = DockController::new();
    let mut memory = UiMemory::new();

    let (selected, frame) = click(
        center(tab.rect),
        &mut dock,
        &mut controller,
        &mut memory,
        FrameId::from_raw(90),
    );
    assert!(selected.changed);
    assert!(selected.focus_changed);
    assert_eq!(dock.active_frame(), Some(FrameId::from_raw(1)));
    assert_eq!(
        dock.frame(FrameId::from_raw(1))
            .and_then(Frame::active_panel)
            .map(|panel| panel.id),
        Some(PanelId::from_raw(12))
    );
    assert_eq!(
        controller.focused_tab().map(|focus| focus.panel),
        Some(PanelId::from_raw(12))
    );
    assert!(memory.is_focused(prepared.tab_widget_id(PanelId::from_raw(12))));
    assert_eq!(frame.repaint, RepaintRequest::NextFrame);

    let selected_snapshot = dock.snapshot();
    let close_rect = scene(&dock).layout().frames[0].tabs[1]
        .close_rect
        .expect("visible close affordance");
    let (closed, close_frame) = click(
        center(close_rect),
        &mut dock,
        &mut controller,
        &mut memory,
        FrameId::from_raw(90),
    );
    assert_eq!(dock.snapshot(), selected_snapshot);
    assert!(!closed.changed);
    assert_eq!(
        closed.close_requests,
        vec![PanelInstanceLocation::new(
            PanelInstanceId::from_raw(12),
            FrameId::from_raw(1),
        )]
    );
    assert_eq!(close_frame.repaint, RepaintRequest::NextFrame);
}

#[test]
fn keyboard_moves_within_tabs_and_spatially_between_frames() {
    let mut dock = split_dock();
    let prepared = scene(&dock);
    let mut controller = DockController::new();
    let mut memory = UiMemory::new();
    let _ = run_frame(
        &mut dock,
        &mut controller,
        &mut memory,
        UiInput::default(),
        FrameId::from_raw(90),
    );
    memory.focus(prepared.tab_widget_id(PanelId::from_raw(11)));

    for (key, ctrl, expected_frame, expected_panel) in [
        (Key::ArrowRight, false, 1, 12),
        (Key::End, false, 1, 13),
        (Key::Home, false, 1, 11),
        (Key::ArrowRight, true, 2, 21),
    ] {
        let (output, frame) = run_frame(
            &mut dock,
            &mut controller,
            &mut memory,
            key_input(key, ctrl),
            FrameId::from_raw(90),
        );
        assert!(output.changed || output.focus_changed);
        assert_eq!(dock.active_frame(), Some(FrameId::from_raw(expected_frame)));
        assert_eq!(
            controller
                .focused_tab()
                .map(|focus| (focus.frame, focus.panel)),
            Some((
                FrameId::from_raw(expected_frame),
                PanelId::from_raw(expected_panel),
            ))
        );
        assert_eq!(frame.repaint, RepaintRequest::NextFrame);
    }
}

#[test]
fn center_and_edge_drops_mutate_while_outside_and_colliding_drops_preserve_state() {
    let mut centered = split_dock();
    let initial = scene(&centered);
    let source = center(initial.layout().frames[0].tabs[0].rect);
    let target = center(initial.layout().frames[1].rect);
    let mut controller = DockController::new();
    let mut memory = UiMemory::new();
    let (preview, dropped) = drag(
        source,
        target,
        &mut centered,
        &mut controller,
        &mut memory,
        FrameId::from_raw(90),
    );
    assert!(preview.drop_preview.is_some());
    assert!(dropped.changed);
    assert!(
        centered
            .frame(FrameId::from_raw(2))
            .expect("target frame")
            .panels
            .iter()
            .any(|panel| panel.id == PanelId::from_raw(11))
    );

    let mut edged = split_dock();
    let edge_scene = scene(&edged);
    let source = center(edge_scene.layout().frames[0].tabs[0].rect);
    let target_frame = &edge_scene.layout().frames[1];
    let right_edge = Point::new(target_frame.rect.max_x() - 2.0, center(target_frame.rect).y);
    let mut edge_controller = DockController::new();
    let mut edge_memory = UiMemory::new();
    let (_, edge_drop) = drag(
        source,
        right_edge,
        &mut edged,
        &mut edge_controller,
        &mut edge_memory,
        FrameId::from_raw(90),
    );
    assert!(edge_drop.changed);
    assert_eq!(
        edged
            .frame(FrameId::from_raw(90))
            .and_then(Frame::active_panel)
            .map(|panel| panel.id),
        Some(PanelId::from_raw(11))
    );

    for (drop_point, new_frame) in [
        (Point::new(700.0, 200.0), FrameId::from_raw(90)),
        (right_edge, FrameId::from_raw(2)),
    ] {
        let mut invalid = split_dock();
        let invalid_scene = scene(&invalid);
        let source = center(invalid_scene.layout().frames[0].tabs[0].rect);
        let before = invalid.snapshot();
        let mut invalid_controller = DockController::new();
        let mut invalid_memory = UiMemory::new();
        let (moved, released) = drag(
            source,
            drop_point,
            &mut invalid,
            &mut invalid_controller,
            &mut invalid_memory,
            new_frame,
        );
        assert!(moved.drop_preview.is_none());
        assert!(!released.changed);
        assert_eq!(invalid.snapshot(), before);
    }
}

#[test]
fn splitter_resize_and_context_metadata_use_existing_model_requests() {
    let mut dock = split_dock();
    let prepared = scene(&dock);
    let splitter = prepared.layout().splitters[0].clone();
    let start = center(splitter.rect);
    let mut controller = DockController::new();
    let mut memory = UiMemory::new();
    let before = dock.snapshot();

    let _ = run_frame(
        &mut dock,
        &mut controller,
        &mut memory,
        pointer_button(start, MouseButton::Primary, true),
        FrameId::from_raw(90),
    );
    let (resized, frame) = run_frame(
        &mut dock,
        &mut controller,
        &mut memory,
        pointer_move(Point::new(start.x + 40.0, start.y), Vec2::new(40.0, 0.0)),
        FrameId::from_raw(90),
    );
    assert!(resized.changed);
    assert_ne!(dock.snapshot(), before);
    assert_eq!(frame.repaint, RepaintRequest::NextFrame);

    let context_point = center(scene(&dock).layout().splitters[0].rect);
    let _ = run_frame(
        &mut dock,
        &mut controller,
        &mut memory,
        pointer_button(context_point, MouseButton::Primary, false),
        FrameId::from_raw(90),
    );
    let _ = run_frame(
        &mut dock,
        &mut controller,
        &mut memory,
        pointer_button(context_point, MouseButton::Secondary, true),
        FrameId::from_raw(90),
    );
    let (context_output, _) = run_frame(
        &mut dock,
        &mut controller,
        &mut memory,
        pointer_button(context_point, MouseButton::Secondary, false),
        FrameId::from_raw(90),
    );
    let request = context_output
        .splitter_context_requests
        .first()
        .expect("splitter context request");
    assert_eq!(request.path, splitter.path);
    let join = request
        .actions
        .iter()
        .find(|action| action.kind == DockSplitterContextActionKind::Join && action.enabled)
        .and_then(stern_widgets::dock::DockSplitterContextAction::join_request)
        .expect("enabled join request");
    let swap = request
        .actions
        .iter()
        .find(|action| action.kind == DockSplitterContextActionKind::Swap && action.enabled)
        .and_then(stern_widgets::dock::DockSplitterContextAction::swap_request)
        .expect("enabled swap request");
    let mut join_dock = dock.clone();
    let mut swap_dock = dock.clone();
    assert!(join_dock.apply_join_request(BOUNDS, join));
    assert!(swap_dock.apply_swap_request(BOUNDS, swap));
}

#[test]
fn splitter_drag_resizes_immediately_commits_on_release_and_keeps_capture_cursor() {
    let mut dock = split_dock();
    let splitter = scene(&dock).layout().splitters[0].clone();
    let start = center(splitter.rect);
    let mut controller = DockController::new();
    let mut memory = UiMemory::new();
    let initial = dock.snapshot();

    let (_, pressed) = run_frame(
        &mut dock,
        &mut controller,
        &mut memory,
        pointer_button(start, MouseButton::Primary, true),
        FrameId::from_raw(90),
    );
    assert_eq!(dock.snapshot(), initial);
    assert!(
        pressed
            .platform_requests
            .contains(&PlatformRequest::SetCursor(CursorShape::ResizeHorizontal))
    );

    let first_point = Point::new(start.x + 40.0, start.y);
    let (first_move, first_frame) = run_frame(
        &mut dock,
        &mut controller,
        &mut memory,
        pointer_move(first_point, Vec2::new(40.0, 0.0)),
        FrameId::from_raw(90),
    );
    assert!(first_move.changed);
    assert!((root_split_ratio(&dock) - (0.5 + 40.0 / 600.0)).abs() < 1e-6);
    assert!(
        first_frame
            .platform_requests
            .contains(&PlatformRequest::SetCursor(CursorShape::ResizeHorizontal))
    );

    let second_point = Point::new(start.x + 60.0, start.y);
    let (second_move, second_frame) = run_frame(
        &mut dock,
        &mut controller,
        &mut memory,
        pointer_move(second_point, Vec2::new(20.0, 0.0)),
        FrameId::from_raw(90),
    );
    assert!(second_move.changed);
    assert!((root_split_ratio(&dock) - 0.6).abs() < 1e-6);
    assert!(
        second_frame
            .platform_requests
            .contains(&PlatformRequest::SetCursor(CursorShape::ResizeHorizontal))
    );

    let _ = run_frame(
        &mut dock,
        &mut controller,
        &mut memory,
        pointer_button(second_point, MouseButton::Primary, false),
        FrameId::from_raw(90),
    );
    let committed = dock.snapshot();
    let (after_release, _) = run_frame(
        &mut dock,
        &mut controller,
        &mut memory,
        pointer_release_all(second_point),
        FrameId::from_raw(90),
    );
    assert_eq!(dock.snapshot(), committed);
    assert!(!after_release.changed);
    assert_eq!(memory.pointer_capture(), None);
}

#[test]
fn splitter_cancellation_restores_exact_start_at_clamps_and_emits_no_requests() {
    for (case, delta) in [
        ("release-all-min", -1_000.0),
        ("focus-loss-max", 1_000.0),
        ("disabled-owner", 120.0),
    ] {
        let mut dock = split_dock();
        let splitter = scene(&dock).layout().splitters[0].clone();
        let start = center(splitter.rect);
        let mut controller = DockController::new();
        let mut memory = UiMemory::new();
        let initial = dock.snapshot();

        let _ = run_frame(
            &mut dock,
            &mut controller,
            &mut memory,
            pointer_button(start, MouseButton::Primary, true),
            FrameId::from_raw(90),
        );
        let moved_point = Point::new(start.x + delta, start.y);
        let _ = run_frame(
            &mut dock,
            &mut controller,
            &mut memory,
            pointer_move(moved_point, Vec2::new(delta, 0.0)),
            FrameId::from_raw(90),
        );
        assert_ne!(dock.snapshot(), initial, "{case}");

        let (cancelled, frame) = match case {
            "release-all-min" => run_frame(
                &mut dock,
                &mut controller,
                &mut memory,
                pointer_release_all(moved_point),
                FrameId::from_raw(90),
            ),
            "focus-loss-max" => run_frame(
                &mut dock,
                &mut controller,
                &mut memory,
                window_focus_lost(),
                FrameId::from_raw(90),
            ),
            "disabled-owner" => run_frame_with_disabled(
                &mut dock,
                &mut controller,
                &mut memory,
                UiInput::default(),
                FrameId::from_raw(90),
                true,
            ),
            _ => unreachable!(),
        };

        assert_eq!(dock.snapshot(), initial, "{case}");
        assert_eq!(memory.pointer_capture(), None, "{case}");
        assert!(cancelled.close_requests.is_empty(), "{case}");
        assert!(cancelled.splitter_context_requests.is_empty(), "{case}");
        assert!(cancelled.drop_preview.is_none(), "{case}");
        assert!(
            frame
                .platform_requests
                .iter()
                .all(|request| !matches!(request, PlatformRequest::SetCursor(_))),
            "{case}"
        );
    }
}

#[test]
fn splitter_owner_removal_rolls_back_but_stale_topology_is_not_resurrected() {
    let mut dock = split_dock();
    let splitter = scene(&dock).layout().splitters[0].clone();
    let start = center(splitter.rect);
    let mut controller = DockController::new();
    let mut memory = UiMemory::new();
    let initial = dock.snapshot();
    let _ = run_frame(
        &mut dock,
        &mut controller,
        &mut memory,
        pointer_button(start, MouseButton::Primary, true),
        FrameId::from_raw(90),
    );
    let _ = run_frame(
        &mut dock,
        &mut controller,
        &mut memory,
        pointer_move(Point::new(start.x + 80.0, start.y), Vec2::new(80.0, 0.0)),
        FrameId::from_raw(90),
    );
    assert_ne!(dock.snapshot(), initial);

    let owner_removed = DockScene::new(DockSceneConfig::new(ROOT, Rect::ZERO), &dock);
    let (_, removed_frame) = run_frame_with_scene(
        &mut dock,
        &mut controller,
        &mut memory,
        UiInput::default(),
        FrameId::from_raw(90),
        &owner_removed,
    );
    assert_eq!(dock.snapshot(), initial);
    assert_eq!(memory.pointer_capture(), None);
    assert!(removed_frame.platform_requests.is_empty());

    let mut stale_dock = split_dock();
    let stale_splitter = scene(&stale_dock).layout().splitters[0].clone();
    let stale_start = center(stale_splitter.rect);
    let mut stale_controller = DockController::new();
    let mut stale_memory = UiMemory::new();
    let _ = run_frame(
        &mut stale_dock,
        &mut stale_controller,
        &mut stale_memory,
        pointer_button(stale_start, MouseButton::Primary, true),
        FrameId::from_raw(90),
    );
    let _ = run_frame(
        &mut stale_dock,
        &mut stale_controller,
        &mut stale_memory,
        pointer_move(
            Point::new(stale_start.x + 80.0, stale_start.y),
            Vec2::new(80.0, 0.0),
        ),
        FrameId::from_raw(90),
    );
    assert!(stale_dock.merge_frames(FrameId::from_raw(1), FrameId::from_raw(2)));
    let merged = stale_dock.snapshot();
    let (stale, _) = run_frame(
        &mut stale_dock,
        &mut stale_controller,
        &mut stale_memory,
        UiInput::default(),
        FrameId::from_raw(90),
    );
    assert_eq!(stale_dock.snapshot(), merged);
    assert!(!stale.changed);
    assert_eq!(stale_memory.pointer_capture(), None);
}

#[test]
fn same_path_replacement_topology_never_receives_old_splitter_transaction() {
    for case in [
        "move",
        "release",
        "release-all",
        "focus-loss",
        "disabled-reconcile",
        "ordinary-reconcile",
    ] {
        let mut dock = split_dock();
        let splitter = scene(&dock).layout().splitters[0].clone();
        let start = center(splitter.rect);
        let moved = Point::new(start.x + 40.0, start.y);
        let mut controller = DockController::new();
        let mut memory = UiMemory::new();

        let _ = run_frame(
            &mut dock,
            &mut controller,
            &mut memory,
            pointer_button(start, MouseButton::Primary, true),
            FrameId::from_raw(90),
        );
        let _ = run_frame(
            &mut dock,
            &mut controller,
            &mut memory,
            pointer_move(moved, Vec2::new(40.0, 0.0)),
            FrameId::from_raw(90),
        );
        assert!(
            (root_split_ratio(&dock) - 0.5).abs() > f32::EPSILON,
            "{case}"
        );

        dock = replacement_split_dock();
        let replacement_snapshot = dock.snapshot();
        let replacement_bytes = snapshot_bytes(&dock);
        let (output, frame_output) = match case {
            "move" => run_frame(
                &mut dock,
                &mut controller,
                &mut memory,
                pointer_move(Point::new(moved.x + 20.0, moved.y), Vec2::new(20.0, 0.0)),
                FrameId::from_raw(90),
            ),
            "release" => run_frame(
                &mut dock,
                &mut controller,
                &mut memory,
                pointer_button(moved, MouseButton::Primary, false),
                FrameId::from_raw(90),
            ),
            "release-all" => run_frame(
                &mut dock,
                &mut controller,
                &mut memory,
                pointer_release_all(moved),
                FrameId::from_raw(90),
            ),
            "focus-loss" => run_frame(
                &mut dock,
                &mut controller,
                &mut memory,
                window_focus_lost(),
                FrameId::from_raw(90),
            ),
            "disabled-reconcile" => run_frame_with_disabled(
                &mut dock,
                &mut controller,
                &mut memory,
                UiInput::default(),
                FrameId::from_raw(90),
                true,
            ),
            "ordinary-reconcile" => run_frame(
                &mut dock,
                &mut controller,
                &mut memory,
                UiInput::default(),
                FrameId::from_raw(90),
            ),
            _ => unreachable!(),
        };

        assert_eq!(dock.snapshot(), replacement_snapshot, "{case}");
        assert_eq!(snapshot_bytes(&dock), replacement_bytes, "{case}");
        assert!(!output.changed, "{case}");
        assert!(output.close_requests.is_empty(), "{case}");
        assert!(output.splitter_context_requests.is_empty(), "{case}");
        assert!(output.drop_preview.is_none(), "{case}");
        assert_eq!(memory.pointer_capture(), None, "{case}");
        assert!(
            frame_output
                .platform_requests
                .iter()
                .all(|request| !matches!(request, PlatformRequest::SetCursor(_))),
            "{case}"
        );
    }
}

#[test]
fn splitter_hover_and_capture_use_axis_cursor_and_cancel_suppresses_it() {
    for (axis, cursor) in [
        (Axis::Horizontal, CursorShape::ResizeHorizontal),
        (Axis::Vertical, CursorShape::ResizeVertical),
    ] {
        let mut dock = split_dock_with_axis(axis);
        let splitter = scene(&dock).layout().splitters[0].clone();
        let start = center(splitter.rect);
        let mut controller = DockController::new();
        let mut memory = UiMemory::new();

        let (_, hovered) = run_frame(
            &mut dock,
            &mut controller,
            &mut memory,
            pointer_hover(start),
            FrameId::from_raw(90),
        );
        assert!(
            hovered
                .platform_requests
                .contains(&PlatformRequest::SetCursor(cursor))
        );

        let _ = run_frame(
            &mut dock,
            &mut controller,
            &mut memory,
            pointer_button(start, MouseButton::Primary, true),
            FrameId::from_raw(90),
        );
        let outside = match axis {
            Axis::Horizontal => Point::new(start.x + 80.0, start.y),
            Axis::Vertical => Point::new(start.x, start.y + 80.0),
        };
        let delta = Vec2::new(outside.x - start.x, outside.y - start.y);
        let (_, captured) = run_frame(
            &mut dock,
            &mut controller,
            &mut memory,
            pointer_move(outside, delta),
            FrameId::from_raw(90),
        );
        assert!(
            captured
                .platform_requests
                .contains(&PlatformRequest::SetCursor(cursor))
        );

        let (_, cancelled) = run_frame(
            &mut dock,
            &mut controller,
            &mut memory,
            pointer_release_all(outside),
            FrameId::from_raw(90),
        );
        assert!(
            cancelled
                .platform_requests
                .iter()
                .all(|request| !matches!(request, PlatformRequest::SetCursor(_)))
        );
    }
}

#[test]
fn tab_drag_cancel_preserves_snapshot_and_clears_preview_without_requests() {
    let mut dock = split_dock();
    let tab = scene(&dock).layout().frames[0].tabs[0].clone();
    let start = center(tab.rect);
    let target = center(scene(&dock).layout().frames[1].rect);
    let initial = dock.snapshot();
    let mut controller = DockController::new();
    let mut memory = UiMemory::new();

    let _ = run_frame(
        &mut dock,
        &mut controller,
        &mut memory,
        pointer_button(start, MouseButton::Primary, true),
        FrameId::from_raw(90),
    );
    let (moved, _) = run_frame(
        &mut dock,
        &mut controller,
        &mut memory,
        pointer_move(target, Vec2::new(target.x - start.x, target.y - start.y)),
        FrameId::from_raw(90),
    );
    assert!(controller.tab_drag().is_some());
    assert!(moved.drop_preview.is_some());

    let (cancelled, _) = run_frame(
        &mut dock,
        &mut controller,
        &mut memory,
        pointer_release_all(target),
        FrameId::from_raw(90),
    );
    assert_eq!(dock.snapshot(), initial);
    assert!(controller.tab_drag().is_none());
    assert!(controller.drop_preview().is_none());
    assert!(cancelled.close_requests.is_empty());
    assert!(cancelled.splitter_context_requests.is_empty());
    assert!(cancelled.drop_preview.is_none());
}

#[test]
fn splitter_context_pointer_menu_key_and_shift_f10_converge() {
    let new_frame = FrameId::from_raw(90);
    let mut requests = Vec::new();

    {
        let mut dock = split_dock();
        let splitter = scene(&dock).layout().splitters[0].clone();
        let point = center(splitter.rect);
        let before = dock.snapshot();
        let mut controller = DockController::new();
        let mut memory = UiMemory::new();
        let (pressed, _) = run_frame(
            &mut dock,
            &mut controller,
            &mut memory,
            pointer_button(point, MouseButton::Secondary, true),
            new_frame,
        );
        assert!(pressed.splitter_context_requests.is_empty());
        let (mut released, _) = run_frame(
            &mut dock,
            &mut controller,
            &mut memory,
            pointer_button(point, MouseButton::Secondary, false),
            new_frame,
        );
        assert_eq!(released.splitter_context_requests.len(), 1);
        requests.push(
            released
                .splitter_context_requests
                .pop()
                .expect("pointer request"),
        );
        assert_eq!(dock.snapshot(), before);
    }

    for (key, modifiers) in [
        (Key::ContextMenu, Modifiers::default()),
        (Key::Function(10), Modifiers::new(true, false, false, false)),
    ] {
        let mut dock = split_dock();
        let splitter = scene(&dock).layout().splitters[0].clone();
        let before = dock.snapshot();
        let mut controller = DockController::new();
        let mut memory = UiMemory::new();
        memory.focus(splitter.id);
        let (mut output, _) = run_frame(
            &mut dock,
            &mut controller,
            &mut memory,
            key_event_input(key, KeyState::Pressed, modifiers),
            new_frame,
        );
        assert_eq!(output.splitter_context_requests.len(), 1);
        requests.push(
            output
                .splitter_context_requests
                .pop()
                .expect("keyboard request"),
        );
        assert_eq!(dock.snapshot(), before);
    }

    assert_eq!(requests.len(), 3);
    assert_eq!(requests[1], requests[0]);
    assert_eq!(requests[2], requests[0]);

    for (focused, disabled, state) in [
        (true, false, KeyState::Released),
        (false, false, KeyState::Pressed),
        (true, true, KeyState::Pressed),
    ] {
        let mut dock = split_dock();
        let splitter = scene(&dock).layout().splitters[0].clone();
        let before = dock.snapshot();
        let mut controller = DockController::new();
        let mut memory = UiMemory::new();
        if focused {
            memory.focus(splitter.id);
        }
        let (output, _) = run_frame_with_disabled(
            &mut dock,
            &mut controller,
            &mut memory,
            key_event_input(Key::ContextMenu, state, Modifiers::default()),
            new_frame,
            disabled,
        );
        assert!(output.splitter_context_requests.is_empty());
        assert_eq!(dock.snapshot(), before);
    }
}

#[test]
fn disappearing_source_repairs_drag_and_controller_owned_focus() {
    let mut dock = split_dock();
    let prepared = scene(&dock);
    let tab = prepared.layout().frames[0].tabs[0].clone();
    let mut controller = DockController::new();
    let mut memory = UiMemory::new();
    let _ = click(
        center(tab.rect),
        &mut dock,
        &mut controller,
        &mut memory,
        FrameId::from_raw(90),
    );
    let _ = run_frame(
        &mut dock,
        &mut controller,
        &mut memory,
        pointer_button(center(tab.rect), MouseButton::Primary, true),
        FrameId::from_raw(90),
    );
    let _ = run_frame(
        &mut dock,
        &mut controller,
        &mut memory,
        pointer_move(Point::new(200.0, 20.0), Vec2::new(180.0, 0.0)),
        FrameId::from_raw(90),
    );
    assert!(controller.tab_drag().is_some());

    assert!(dock.merge_frames(FrameId::from_raw(1), FrameId::from_raw(2)));
    let (repaired, _) = run_frame(
        &mut dock,
        &mut controller,
        &mut memory,
        UiInput::default(),
        FrameId::from_raw(90),
    );
    assert!(controller.tab_drag().is_none());
    assert!(controller.drop_preview().is_none());
    assert_eq!(
        controller
            .focused_tab()
            .map(|focus| (focus.frame, focus.panel)),
        Some((FrameId::from_raw(2), PanelId::from_raw(11)))
    );
    assert!(
        repaired.focus_changed
            || memory.is_focused(scene(&dock).tab_widget_id(PanelId::from_raw(11)))
    );
}

#[test]
fn dock_and_validated_workspace_snapshots_round_trip() {
    let dock = split_dock();
    let snapshot = dock.snapshot();
    let restored = Dock::restore(snapshot.clone()).expect("valid Dock snapshot");
    assert_eq!(restored.snapshot(), snapshot);

    let panel_type = PanelTypeId::from_raw(7);
    let descriptors = vec![PanelTypeDescriptor::new(panel_type, "Editor panel")];
    let instances = [
        (11, "Assets"),
        (12, "Inspector"),
        (13, "Details"),
        (21, "Viewport"),
    ]
    .into_iter()
    .map(|(id, title)| PanelInstanceSnapshot::new(PanelInstanceId::from_raw(id), panel_type, title))
    .collect::<Vec<_>>();
    let workspace = dock.workspace_snapshot(instances);
    workspace.validate(&descriptors).expect("valid workspace");
    let restored_workspace =
        Dock::restore_workspace(workspace.clone(), &descriptors).expect("workspace restore");
    assert_eq!(restored_workspace.snapshot(), workspace.dock);
    assert_eq!(
        restored_workspace.workspace_snapshot(workspace.panel_instances.clone()),
        workspace
    );
}
