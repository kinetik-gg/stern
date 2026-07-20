//! Public prepared viewport transform-tool scene conformance tests.

use std::time::Duration;

use stern_core::{
    Brush, CursorShape, FrameContext, InputWheelDelta, Modifiers, MouseButton, PhysicalSize,
    PlatformRequest, Point, PointerInput, PointerOrder, Primitive, RadiusScale, Rect,
    RepaintRequest, ScaleFactor, SemanticRole, Size, StrokeScale, Theme, TimeInfo, UiInput,
    UiInputEvent, UiMemory, Vec2, ViewportInfo, WidgetId, default_dark_theme,
};
use stern_widgets::{
    PanZoom, Ui, ViewportSelectionTargetDescriptor, ViewportSelectionTargetId, ViewportSurface,
    ViewportToolController, ViewportToolScene, ViewportToolSceneConfig, ViewportToolSceneOutput,
    ViewportTransformDragStatus, ViewportTransformHandleId, ViewportTransformHandleKind,
    ViewportTransformHandleSet, ViewportTransformInteractionPhase, ViewportWidget,
    ViewportWidgetConfig, ViewportWidgetOutput, viewport_transform_handle_widget_id,
};

const VIEWPORT: WidgetId = WidgetId::from_raw(0x7001);
const BOUNDS: Rect = Rect::new(10.0, 20.0, 300.0, 200.0);

fn surface() -> ViewportSurface {
    let mut pan_zoom = PanZoom::default();
    pan_zoom.set_zoom(1.0);
    ViewportSurface {
        texture: stern_core::TextureId::from_raw(17),
        source_size: Size::new(100.0, 50.0),
        bounds: BOUNDS,
        pan_zoom,
    }
}

fn target(raw: u64) -> ViewportSelectionTargetDescriptor {
    ViewportSelectionTargetDescriptor::new(
        ViewportSelectionTargetId::from_raw(raw),
        Rect::new(20.0, 10.0, 40.0, 20.0),
    )
    .with_handle_size(10.0)
    .with_rotate_offset(20.0)
}

fn move_target(raw: u64) -> ViewportSelectionTargetDescriptor {
    target(raw).with_handles(ViewportTransformHandleSet::move_only())
}

fn context(input: UiInput, scale_factor: ScaleFactor) -> FrameContext {
    FrameContext::new(
        ViewportInfo::new(
            Size::new(640.0, 480.0),
            PhysicalSize::new(1280, 960),
            scale_factor,
        ),
        input,
        TimeInfo::new(Duration::from_millis(500), Duration::from_millis(16), 1),
    )
}

struct Run {
    scene: ViewportToolScene,
    viewport: ViewportWidgetOutput,
    tools: ViewportToolSceneOutput,
    frame: stern_core::FrameOutput,
}

fn run_frame(
    surface: ViewportSurface,
    tool_config: ViewportToolSceneConfig,
    controller: &mut ViewportToolController,
    pan_zoom: &mut PanZoom,
    memory: &mut UiMemory,
    input: UiInput,
    scale_factor: ScaleFactor,
) -> Run {
    let theme = default_dark_theme();
    run_frame_with_theme(
        surface,
        tool_config,
        controller,
        pan_zoom,
        memory,
        input,
        scale_factor,
        &theme,
    )
}

#[allow(clippy::too_many_arguments)]
fn run_frame_with_theme(
    surface: ViewportSurface,
    tool_config: ViewportToolSceneConfig,
    controller: &mut ViewportToolController,
    pan_zoom: &mut PanZoom,
    memory: &mut UiMemory,
    input: UiInput,
    scale_factor: ScaleFactor,
    theme: &Theme,
) -> Run {
    let mut ui = Ui::begin_frame(context(input, scale_factor), memory, theme);
    let viewport = ui.prepare_viewport_widget(ViewportWidgetConfig::new(VIEWPORT, surface));
    let scene = ui.prepare_viewport_tool_scene(&viewport, tool_config);
    ui.resolve_pointer_targets(|plan| {
        let next = viewport.declare_pointer_targets(plan, PointerOrder::new(100));
        scene.declare_pointer_targets(plan, next);
    })
    .expect("valid viewport tool pointer plan");
    let viewport_output = ui.viewport_widget(&viewport, pan_zoom, &[]);
    let tool_output = ui.viewport_tool_scene(&scene, controller);
    let frame = ui.finish_output();
    Run {
        scene,
        viewport: viewport_output,
        tools: tool_output,
        frame,
    }
}

fn pointer_at(point: Point) -> UiInput {
    UiInput {
        pointer: PointerInput {
            position: Some(point),
            ..PointerInput::default()
        },
        ..UiInput::default()
    }
}

fn pointer_button(point: Point, down: bool) -> UiInput {
    let mut input = UiInput::default();
    input.push_event(UiInputEvent::PointerButton {
        button: MouseButton::Primary,
        down,
        click_count: 1,
        position: Some(point),
    });
    input
}

fn pointer_move(point: Point, delta: Vec2, modifiers: Option<Modifiers>) -> UiInput {
    let mut input = UiInput::default();
    input.pointer.primary.down = true;
    if let Some(modifiers) = modifiers {
        input.push_event(UiInputEvent::ModifiersChanged(modifiers));
    }
    input.push_event(UiInputEvent::PointerMoved {
        position: point,
        delta,
    });
    input
}

fn focus_lost() -> UiInput {
    let mut input = UiInput::default();
    input.push_event(UiInputEvent::WindowFocusChanged(false));
    input
}

fn handle_center(scene: &ViewportToolScene, handle: ViewportTransformHandleId) -> Point {
    let rect = scene
        .handles()
        .iter()
        .find(|candidate| candidate.id == handle)
        .expect("resolved handle")
        .handle_screen_rect;
    Point::new(rect.x + rect.width * 0.5, rect.y + rect.height * 0.5)
}

fn prepared_scene(
    surface: ViewportSurface,
    scale_factor: ScaleFactor,
    config: ViewportToolSceneConfig,
) -> ViewportToolScene {
    let viewport = ViewportWidget::new(ViewportWidgetConfig::new(VIEWPORT, surface), scale_factor);
    ViewportToolScene::new(&viewport, config)
}

#[test]
#[allow(clippy::too_many_lines)]
fn scene_uses_theme_clip_hides_move_paint_and_keeps_stable_handle_ids() {
    let target = target(11).with_label("Layer 11");
    let config = ViewportToolSceneConfig::new([target.clone()]);
    let mut controller = ViewportToolController::default();
    let mut pan_zoom = surface().pan_zoom;
    let mut memory = UiMemory::new();
    let strokes = StrokeScale::from_values(0.75, 1.25, 2.5, 3.5, 4.5);
    let theme = default_dark_theme()
        .with_radii(RadiusScale::from_values(4.0, 11.0, 23.0, 777.0))
        .with_strokes(strokes);
    let run = run_frame_with_theme(
        surface(),
        config,
        &mut controller,
        &mut pan_zoom,
        &mut memory,
        UiInput::default(),
        ScaleFactor::ONE,
        &theme,
    );
    let mut focused_controller = ViewportToolController::default();
    let mut focused_pan_zoom = surface().pan_zoom;
    let mut focused_memory = UiMemory::new();
    focused_memory.focus(VIEWPORT);
    let focused = run_frame_with_theme(
        surface(),
        ViewportToolSceneConfig::new([target.clone()]),
        &mut focused_controller,
        &mut focused_pan_zoom,
        &mut focused_memory,
        UiInput::default(),
        ScaleFactor::ONE,
        &theme,
    );

    assert_eq!(run.scene.outlines(), focused.scene.outlines());
    assert_eq!(run.scene.handles(), focused.scene.handles());
    assert_eq!(run.viewport.response.rect, focused.viewport.response.rect);
    assert_eq!(
        run.tools
            .handle_responses
            .iter()
            .map(|response| (response.widget_id, response.response.rect))
            .collect::<Vec<_>>(),
        focused
            .tools
            .handle_responses
            .iter()
            .map(|response| (response.widget_id, response.response.rect))
            .collect::<Vec<_>>()
    );
    assert_eq!(run.frame.primitives.len(), focused.frame.primitives.len());
    assert_eq!(
        run.frame
            .primitives
            .iter()
            .filter_map(|primitive| match primitive {
                Primitive::Rect(rect) => Some((
                    rect.rect,
                    rect.radius,
                    rect.stroke.map(|stroke| stroke.width),
                )),
                _ => None,
            })
            .collect::<Vec<_>>(),
        focused
            .frame
            .primitives
            .iter()
            .filter_map(|primitive| match primitive {
                Primitive::Rect(rect) => Some((
                    rect.rect,
                    rect.radius,
                    rect.stroke.map(|stroke| stroke.width),
                )),
                _ => None,
            })
            .collect::<Vec<_>>()
    );
    assert_eq!(
        run.frame
            .semantics
            .nodes()
            .iter()
            .map(|node| (node.id, node.bounds))
            .collect::<Vec<_>>(),
        focused
            .frame
            .semantics
            .nodes()
            .iter()
            .map(|node| (node.id, node.bounds))
            .collect::<Vec<_>>()
    );

    assert_eq!(run.scene.outlines().len(), 1);
    assert_eq!(run.scene.handles().len(), 11);
    let move_handle = ViewportTransformHandleId::new(target.id, ViewportTransformHandleKind::Move);
    let move_rect = run
        .scene
        .handles()
        .iter()
        .find(|handle| handle.id == move_handle)
        .expect("move handle")
        .handle_screen_rect;
    let tool_clip = run
        .frame
        .primitives
        .iter()
        .rposition(
            |primitive| matches!(primitive, Primitive::ClipBegin { rect, .. } if *rect == BOUNDS),
        )
        .expect("tool clip");
    assert!(matches!(
        run.frame.primitives.last(),
        Some(Primitive::ClipEnd { .. })
    ));
    let tool_primitives = &run.frame.primitives[tool_clip + 1..run.frame.primitives.len() - 1];
    assert_eq!(
        tool_primitives
            .iter()
            .filter(|primitive| matches!(primitive, Primitive::Rect(rect) if rect.fill.is_some()))
            .count(),
        10,
        "Move is routed but not painted"
    );
    assert!(!tool_primitives.iter().any(|primitive| {
        matches!(primitive, Primitive::Rect(rect) if rect.rect == move_rect && rect.fill.is_some())
    }));
    let painted_handle_rects = tool_primitives
        .iter()
        .filter_map(|primitive| match primitive {
            Primitive::Rect(rect) if rect.fill.is_some() => Some(rect.rect),
            _ => None,
        })
        .collect::<Vec<_>>();
    let expected_handle_rects = run
        .scene
        .handles()
        .iter()
        .filter(|handle| handle.kind != ViewportTransformHandleKind::Move)
        .map(|handle| handle.handle_screen_rect)
        .collect::<Vec<_>>();
    assert_eq!(painted_handle_rects, expected_handle_rects);
    for handle in tool_primitives
        .iter()
        .filter_map(|primitive| match primitive {
            Primitive::Rect(rect) if rect.fill.is_some() => Some(rect),
            _ => None,
        })
    {
        assert_eq!(handle.radius, theme.radii.sm);
        assert_eq!(
            handle.stroke.map(|stroke| stroke.width),
            Some(strokes.default)
        );
    }
    let outline = tool_primitives
        .iter()
        .find_map(|primitive| match primitive {
            Primitive::Rect(rect) if rect.fill.is_none() => Some(rect),
            _ => None,
        })
        .expect("selection outline");
    assert_eq!(
        outline.stroke.as_ref().map(|stroke| &stroke.brush),
        Some(&Brush::Solid(theme.colors.accent.default))
    );
    assert_eq!(
        outline.stroke.map(|stroke| stroke.width),
        Some(strokes.default)
    );
    let viewport_surface = run
        .frame
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            Primitive::Rect(rect) if rect.rect == BOUNDS && rect.fill.is_some() => Some(rect),
            _ => None,
        })
        .expect("viewport structural surface");
    assert_eq!(
        viewport_surface.stroke.map(|stroke| stroke.width),
        Some(strokes.hairline)
    );
    let viewport_semantics = run
        .frame
        .semantics
        .get(VIEWPORT)
        .expect("viewport semantics");
    assert_eq!(viewport_semantics.role, SemanticRole::Viewport);
    assert_eq!(viewport_semantics.bounds, BOUNDS);

    let resize =
        ViewportTransformHandleId::new(target.id, ViewportTransformHandleKind::ResizeRight);
    let first_id = run.scene.handle_widget_id(resize);
    let mut changed_surface = surface();
    changed_surface.pan_zoom.pan_by(Vec2::new(12.0, -4.0));
    let changed = prepared_scene(
        changed_surface,
        ScaleFactor::new(1.5),
        ViewportToolSceneConfig::new([target.clone()]),
    );
    assert_eq!(changed.handle_widget_id(resize), first_id);
    assert_eq!(
        first_id,
        viewport_transform_handle_widget_id(VIEWPORT, resize)
    );

    let read_only = prepared_scene(
        surface(),
        ScaleFactor::ONE,
        ViewportToolSceneConfig::new([target.read_only(true)]),
    );
    assert_eq!(read_only.outlines().len(), 1);
    assert!(read_only.handles().is_empty());
}

#[test]
fn same_frame_pan_keeps_frozen_routing_and_reprojects_tool_presentation() {
    let selected = target(31);
    let config = ViewportToolSceneConfig::new([selected.clone()]);
    let mut controller = ViewportToolController::default();
    let mut pan_zoom = surface().pan_zoom;
    let mut memory = UiMemory::new();
    let pointer = Point::new(280.0, 180.0);
    let _ = run_frame(
        surface(),
        config.clone(),
        &mut controller,
        &mut pan_zoom,
        &mut memory,
        pointer_button(pointer, true),
        ScaleFactor::ONE,
    );
    let run = run_frame(
        surface(),
        config,
        &mut controller,
        &mut pan_zoom,
        &mut memory,
        pointer_move(Point::new(305.0, 170.0), Vec2::new(25.0, -10.0), None),
        ScaleFactor::ONE,
    );

    assert_eq!(run.scene.surface(), surface());
    assert_eq!(run.viewport.surface.pan_zoom.pan, Vec2::new(25.0, -10.0));
    let frozen_clone = run.scene.clone();
    let presented = run.scene.with_presented_surface(run.viewport.surface);
    assert_eq!(run.scene, frozen_clone);
    assert_ne!(presented.outlines(), run.scene.outlines());

    let handle_id =
        ViewportTransformHandleId::new(selected.id, ViewportTransformHandleKind::ResizeRight);
    let frozen_handle = run.scene.handle_widget_id(handle_id);
    let frozen_rect = run
        .scene
        .handles()
        .iter()
        .find(|handle| handle.id == handle_id)
        .expect("frozen handle")
        .handle_screen_rect;
    assert_eq!(
        run.tools
            .handle_responses
            .iter()
            .find(|response| response.widget_id == frozen_handle)
            .expect("routed handle")
            .response
            .rect,
        frozen_rect
    );
    let presented_handle = presented
        .handles()
        .iter()
        .find(|handle| handle.id == handle_id)
        .expect("presented handle");
    let presented_center = Point::new(
        presented_handle.handle_screen_rect.x + presented_handle.handle_screen_rect.width * 0.5,
        presented_handle.handle_screen_rect.y + presented_handle.handle_screen_rect.height * 0.5,
    );
    assert_eq!(
        presented
            .hit_test_handle(presented_center)
            .map(|handle| handle.id),
        Some(handle_id)
    );
    assert_eq!(run.scene.hit_test_handle(presented_center), None);
    assert_eq!(
        presented.presentation().screen_to_content(presented_center),
        run.viewport
            .screen_to_content_at(presented_center, ScaleFactor::ONE)
    );
    let presented_outline = presented.outlines()[0].screen_rect;
    assert!(run.frame.primitives.iter().any(
        |primitive| matches!(primitive, Primitive::Rect(rect) if rect.fill.is_none() && rect.rect == presented_outline)
    ));
}

#[test]
fn same_frame_zoom_drives_tool_capture_domain_conversion() {
    let selected = target(32);
    let scene = prepared_scene(
        surface(),
        ScaleFactor::new(1.25),
        ViewportToolSceneConfig::new([selected.clone()]),
    );
    let handle_id =
        ViewportTransformHandleId::new(selected.id, ViewportTransformHandleKind::ResizeRight);
    let origin = handle_center(&scene, handle_id);
    let mut input = pointer_button(origin, true);
    input.push_event(UiInputEvent::Wheel {
        delta: InputWheelDelta::Lines(Vec2::new(0.0, 2.0)),
        position: Some(origin),
    });
    let mut controller = ViewportToolController::default();
    let mut pan_zoom = surface().pan_zoom;
    let mut memory = UiMemory::new();
    let pressed = run_frame(
        surface(),
        ViewportToolSceneConfig::new([selected.clone()]),
        &mut controller,
        &mut pan_zoom,
        &mut memory,
        input,
        ScaleFactor::new(1.25),
    );
    assert!(pressed.viewport.zoom_changed);
    assert_eq!(controller.captured_handle(), Some(handle_id));
    let expected_origin = pressed
        .viewport
        .screen_to_content_at(origin, ScaleFactor::new(1.25));

    let mut effective = surface();
    effective.pan_zoom = pan_zoom;
    let moved = run_frame(
        effective,
        ViewportToolSceneConfig::new([selected]),
        &mut controller,
        &mut pan_zoom,
        &mut memory,
        pointer_move(
            Point::new(origin.x + 12.0, origin.y),
            Vec2::new(12.0, 0.0),
            None,
        ),
        ScaleFactor::new(1.25),
    );
    let interaction = moved.tools.interactions.first().expect("started drag");
    assert_eq!(
        interaction.phase,
        ViewportTransformInteractionPhase::Started
    );
    assert_eq!(interaction.drag.pointer_origin_content, expected_origin);
    assert_eq!(
        interaction.drag.pointer_current_content,
        moved.viewport.screen_to_content_at(
            interaction.drag.pointer_current_screen,
            ScaleFactor::new(1.25)
        )
    );
}

#[test]
fn missing_same_frame_publication_falls_back_to_the_frozen_scene() {
    let scene = prepared_scene(
        surface(),
        ScaleFactor::new(1.5),
        ViewportToolSceneConfig::new([target(33)]),
    );
    let theme = default_dark_theme();
    let mut memory = UiMemory::new();
    let mut controller = ViewportToolController::default();
    let mut ui = Ui::begin_frame(
        context(UiInput::default(), ScaleFactor::new(1.5)),
        &mut memory,
        &theme,
    );
    ui.resolve_pointer_targets(|plan| {
        scene.declare_pointer_targets(plan, PointerOrder::new(100));
    })
    .expect("valid fallback plan");
    let output = ui.viewport_tool_scene(&scene, &mut controller);
    let frame = ui.finish_output();

    assert_eq!(output.handle_responses.len(), scene.handles().len());
    let outline = scene.outlines()[0].screen_rect;
    assert!(frame.primitives.iter().any(
        |primitive| matches!(primitive, Primitive::Rect(rect) if rect.fill.is_none() && rect.rect == outline)
    ));
}

#[test]
fn overlap_priority_is_deterministic_and_handles_route_above_viewport_pan() {
    let lower = move_target(2).with_priority(1);
    let top = move_target(9).with_priority(5);
    let top_handle = ViewportTransformHandleId::new(top.id, ViewportTransformHandleKind::Move);
    let scene = prepared_scene(
        surface(),
        ScaleFactor::ONE,
        ViewportToolSceneConfig::new([lower.clone(), top.clone()]),
    );
    let point = handle_center(&scene, top_handle);

    for targets in [
        vec![lower.clone(), top.clone()],
        vec![top.clone(), lower.clone()],
    ] {
        let mut controller = ViewportToolController::default();
        let mut pan_zoom = surface().pan_zoom;
        let mut memory = UiMemory::new();
        let hovered = run_frame(
            surface(),
            ViewportToolSceneConfig::new(targets),
            &mut controller,
            &mut pan_zoom,
            &mut memory,
            pointer_at(point),
            ScaleFactor::ONE,
        );
        assert_eq!(hovered.tools.hovered_handle, Some(top_handle));
        assert!(!hovered.viewport.response.state.hovered);
    }

    let tie_scene = prepared_scene(
        surface(),
        ScaleFactor::ONE,
        ViewportToolSceneConfig::new([move_target(7), move_target(3)]),
    );
    let tie_point = handle_center(
        &tie_scene,
        ViewportTransformHandleId::new(
            ViewportSelectionTargetId::from_raw(3),
            ViewportTransformHandleKind::Move,
        ),
    );
    let mut tie_controller = ViewportToolController::default();
    let mut tie_pan = surface().pan_zoom;
    let mut tie_memory = UiMemory::new();
    let tie = run_frame(
        surface(),
        ViewportToolSceneConfig::new([move_target(7), move_target(3)]),
        &mut tie_controller,
        &mut tie_pan,
        &mut tie_memory,
        pointer_at(tie_point),
        ScaleFactor::ONE,
    );
    assert_eq!(
        tie.tools.hovered_handle.map(|handle| handle.target),
        Some(ViewportSelectionTargetId::from_raw(3))
    );

    let mut controller = ViewportToolController::default();
    let mut pan_zoom = surface().pan_zoom;
    let original_pan = pan_zoom.pan;
    let mut memory = UiMemory::new();
    let config = ViewportToolSceneConfig::new([lower, top]);
    let _ = run_frame(
        surface(),
        config.clone(),
        &mut controller,
        &mut pan_zoom,
        &mut memory,
        pointer_button(point, true),
        ScaleFactor::ONE,
    );
    let moved = run_frame(
        surface(),
        config,
        &mut controller,
        &mut pan_zoom,
        &mut memory,
        pointer_move(
            Point::new(point.x + 10.0, point.y),
            Vec2::new(10.0, 0.0),
            None,
        ),
        ScaleFactor::ONE,
    );
    assert_eq!(
        moved.tools.interactions[0].phase,
        ViewportTransformInteractionPhase::Started
    );
    assert_eq!(moved.tools.interactions[0].drag.target, top_handle.target);
    assert!(!moved.viewport.pan_changed);
    assert_eq!(pan_zoom.pan, original_pan);
    assert_eq!(
        moved.frame.platform_requests,
        vec![PlatformRequest::SetCursor(CursorShape::Grabbing)]
    );
}

#[test]
#[allow(clippy::too_many_lines)]
fn captured_handle_survives_outside_moves_and_finishes_or_cancels_when_removed() {
    let target = move_target(11);
    let handle = ViewportTransformHandleId::new(target.id, ViewportTransformHandleKind::Move);
    let scene = prepared_scene(
        surface(),
        ScaleFactor::ONE,
        ViewportToolSceneConfig::new([target.clone()]),
    );
    let origin = handle_center(&scene, handle);
    let config = ViewportToolSceneConfig::new([target.clone()]);
    let mut controller = ViewportToolController::default();
    let mut pan_zoom = surface().pan_zoom;
    let mut memory = UiMemory::new();
    let _ = run_frame(
        surface(),
        config.clone(),
        &mut controller,
        &mut pan_zoom,
        &mut memory,
        pointer_button(origin, true),
        ScaleFactor::ONE,
    );
    assert_eq!(controller.captured_handle(), Some(handle));

    let outside = Point::new(400.0, 260.0);
    let started = run_frame(
        surface(),
        config.clone(),
        &mut controller,
        &mut pan_zoom,
        &mut memory,
        pointer_move(
            outside,
            Vec2::new(outside.x - origin.x, outside.y - origin.y),
            None,
        ),
        ScaleFactor::ONE,
    );
    assert_eq!(
        started.tools.interactions[0].phase,
        ViewportTransformInteractionPhase::Started
    );
    assert!(controller.transform_started());
    let later = Point::new(420.0, 280.0);
    let updated = run_frame(
        surface(),
        config.clone(),
        &mut controller,
        &mut pan_zoom,
        &mut memory,
        pointer_move(later, Vec2::new(20.0, 20.0), None),
        ScaleFactor::ONE,
    );
    assert_eq!(
        updated.tools.interactions[0].phase,
        ViewportTransformInteractionPhase::Updated
    );
    let finished = run_frame(
        surface(),
        config.clone(),
        &mut controller,
        &mut pan_zoom,
        &mut memory,
        pointer_button(later, false),
        ScaleFactor::ONE,
    );
    assert_eq!(
        finished.tools.interactions[0].phase,
        ViewportTransformInteractionPhase::Finished
    );
    assert_eq!(controller.captured_handle(), None);
    assert_eq!(finished.frame.repaint, RepaintRequest::NextFrame);

    let mut release_controller = ViewportToolController::default();
    let mut release_pan = surface().pan_zoom;
    let mut release_memory = UiMemory::new();
    let _ = run_frame(
        surface(),
        config.clone(),
        &mut release_controller,
        &mut release_pan,
        &mut release_memory,
        pointer_button(origin, true),
        ScaleFactor::ONE,
    );
    let release_only = run_frame(
        surface(),
        config.clone(),
        &mut release_controller,
        &mut release_pan,
        &mut release_memory,
        pointer_button(Point::new(origin.x + 10.0, origin.y), false),
        ScaleFactor::ONE,
    );
    assert_eq!(
        release_only
            .tools
            .interactions
            .iter()
            .map(|interaction| interaction.phase)
            .collect::<Vec<_>>(),
        vec![
            ViewportTransformInteractionPhase::Started,
            ViewportTransformInteractionPhase::Finished,
        ]
    );

    let mut cancel_controller = ViewportToolController::default();
    let mut cancel_pan = surface().pan_zoom;
    let mut cancel_memory = UiMemory::new();
    let _ = run_frame(
        surface(),
        config.clone(),
        &mut cancel_controller,
        &mut cancel_pan,
        &mut cancel_memory,
        pointer_button(origin, true),
        ScaleFactor::ONE,
    );
    let pre_threshold_cancel = run_frame(
        surface(),
        config.clone(),
        &mut cancel_controller,
        &mut cancel_pan,
        &mut cancel_memory,
        focus_lost(),
        ScaleFactor::ONE,
    );
    assert!(pre_threshold_cancel.tools.interactions.is_empty());
    assert_eq!(cancel_controller.captured_handle(), None);

    let mut removed_controller = ViewportToolController::default();
    let mut removed_pan = surface().pan_zoom;
    let mut removed_memory = UiMemory::new();
    let _ = run_frame(
        surface(),
        config.clone(),
        &mut removed_controller,
        &mut removed_pan,
        &mut removed_memory,
        pointer_button(origin, true),
        ScaleFactor::ONE,
    );
    let _ = run_frame(
        surface(),
        config,
        &mut removed_controller,
        &mut removed_pan,
        &mut removed_memory,
        pointer_move(
            Point::new(origin.x + 10.0, origin.y),
            Vec2::new(10.0, 0.0),
            None,
        ),
        ScaleFactor::ONE,
    );
    let cancelled = run_frame(
        surface(),
        ViewportToolSceneConfig::new([]),
        &mut removed_controller,
        &mut removed_pan,
        &mut removed_memory,
        UiInput::default(),
        ScaleFactor::ONE,
    );
    assert_eq!(
        cancelled.tools.interactions[0].phase,
        ViewportTransformInteractionPhase::Cancelled
    );
    assert_eq!(
        cancelled.tools.interactions[0].drag.status,
        ViewportTransformDragStatus::StaleTarget
    );
    assert_eq!(removed_controller.captured_handle(), None);
}

#[test]
fn transform_requests_keep_snap_modifiers_and_raw_dpi_aware_deltas_without_mutation() {
    let target = move_target(21);
    let original = target.clone();
    let config = ViewportToolSceneConfig::new([target.clone()]).with_snap_tolerance(6.0);
    let scene = prepared_scene(surface(), ScaleFactor::new(2.0), config.clone());
    let handle = ViewportTransformHandleId::new(target.id, ViewportTransformHandleKind::Move);
    let origin = handle_center(&scene, handle);
    let mut controller = ViewportToolController::default();
    let mut pan_zoom = surface().pan_zoom;
    let mut memory = UiMemory::new();
    let _ = run_frame(
        surface(),
        config.clone(),
        &mut controller,
        &mut pan_zoom,
        &mut memory,
        pointer_button(origin, true),
        ScaleFactor::new(2.0),
    );
    let modifiers = Modifiers::new(true, false, false, false);
    let moved = run_frame(
        surface(),
        config,
        &mut controller,
        &mut pan_zoom,
        &mut memory,
        pointer_move(
            Point::new(origin.x + 10.0, origin.y + 5.0),
            Vec2::new(10.0, 5.0),
            Some(modifiers),
        ),
        ScaleFactor::new(2.0),
    );
    let interaction = &moved.tools.interactions[0];
    assert_eq!(
        interaction.phase,
        ViewportTransformInteractionPhase::Started
    );
    assert_eq!(interaction.event_ordinal, Some(1));
    assert_eq!(interaction.modifiers, modifiers);
    assert_eq!(interaction.snap_tolerance, Some(6.0));
    assert_eq!(interaction.drag.status, ViewportTransformDragStatus::Active);
    assert_eq!(interaction.drag.screen_delta, Vec2::new(10.0, 5.0));
    assert_eq!(interaction.drag.content_delta, Vec2::new(20.0, 10.0));
    assert_eq!(
        interaction.drag.current_content_rect,
        Some(target.content_rect)
    );
    assert_eq!(target, original);
}

#[test]
fn disabled_and_noninteractive_targets_fail_safely() {
    let interactive = move_target(31);
    let disabled_parent = ViewportWidget::new(
        ViewportWidgetConfig::new(VIEWPORT, surface()).disabled(true),
        ScaleFactor::ONE,
    );
    let inherited = ViewportToolScene::new(
        &disabled_parent,
        ViewportToolSceneConfig::new([interactive.clone()]),
    );
    assert!(inherited.config().disabled);
    let scene = prepared_scene(
        surface(),
        ScaleFactor::ONE,
        ViewportToolSceneConfig::new([interactive.clone()]),
    );
    let handle = ViewportTransformHandleId::new(interactive.id, ViewportTransformHandleKind::Move);
    let point = handle_center(&scene, handle);
    let mut controller = ViewportToolController::default();
    let mut pan_zoom = surface().pan_zoom;
    let mut memory = UiMemory::new();
    let disabled = ViewportToolSceneConfig::new([interactive.clone()]).disabled(true);
    let _ = run_frame(
        surface(),
        disabled.clone(),
        &mut controller,
        &mut pan_zoom,
        &mut memory,
        pointer_button(point, true),
        ScaleFactor::ONE,
    );
    let moved = run_frame(
        surface(),
        disabled,
        &mut controller,
        &mut pan_zoom,
        &mut memory,
        pointer_move(
            Point::new(point.x + 10.0, point.y),
            Vec2::new(10.0, 0.0),
            None,
        ),
        ScaleFactor::ONE,
    );
    assert!(moved.tools.interactions.is_empty());
    assert_eq!(controller.captured_handle(), None);

    let cases = [
        interactive.clone().enabled(false),
        interactive.clone().available(false),
        interactive.clone().read_only(true),
        interactive.selected(false),
    ];
    for target in cases {
        let scene = prepared_scene(
            surface(),
            ScaleFactor::ONE,
            ViewportToolSceneConfig::new([target.clone()]),
        );
        assert!(scene.handles().is_empty());
        assert_eq!(
            scene.outlines().is_empty(),
            !target.can_show_selection(),
            "outlines follow selection visibility independently of transform availability"
        );
    }
}
