//! Public prepared viewport-widget conformance tests.

use std::time::Duration;

use stern_core::{
    ActionContext, ActionDescriptor, ActionId, ActionSource, CursorShape, FrameContext,
    InputWheelDelta, MouseButton, PhysicalSize, PlatformRequest, Point, PointerInput, PointerOrder,
    PointerTarget, Primitive, Rect, RepaintRequest, Response, ScaleFactor, SemanticRole, Size,
    TimeInfo, UiInput, UiInputEvent, UiMemory, Vec2, ViewportInfo, WidgetId, default_dark_theme,
};
use stern_widgets::{
    PanZoom, Ui, ViewportActionDescriptor, ViewportActionKind, ViewportActionRequest,
    ViewportActionTarget, ViewportFit, ViewportSurface, ViewportWidget, ViewportWidgetConfig,
    ViewportWidgetOutput, viewport_action_widget_id,
};

const VIEWPORT: WidgetId = WidgetId::from_raw(0xBEEF);
const LOWER: WidgetId = WidgetId::from_raw(0xCAFE);
const BOUNDS: Rect = Rect::new(10.0, 20.0, 300.0, 200.0);

fn surface() -> ViewportSurface {
    let mut pan_zoom = PanZoom::default();
    pan_zoom.set_zoom(1.0);
    ViewportSurface {
        texture: stern_core::TextureId::from_raw(7),
        source_size: Size::new(400.0, 200.0),
        bounds: BOUNDS,
        pan_zoom,
    }
}

fn config() -> ViewportWidgetConfig {
    ViewportWidgetConfig::new(VIEWPORT, surface())
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
    widget: ViewportWidget,
    widget_clone: ViewportWidget,
    output: ViewportWidgetOutput,
    lower: Option<Response>,
    frame: stern_core::FrameOutput,
}

fn run_frame(
    config: ViewportWidgetConfig,
    pan_zoom: &mut PanZoom,
    memory: &mut UiMemory,
    input: UiInput,
    scale_factor: ScaleFactor,
    action_requests: &[ViewportActionRequest],
    lower: bool,
) -> Run {
    let theme = default_dark_theme();
    let mut ui = Ui::begin_frame(context(input, scale_factor), memory, &theme);
    let widget = ui.prepare_viewport_widget(config);
    let widget_clone = widget.clone();
    ui.resolve_pointer_targets(|plan| {
        if lower {
            plan.target(PointerTarget::new(
                LOWER,
                widget.surface().effective_bounds(),
                PointerOrder::new(10),
            ));
        }
        widget.declare_pointer_targets(plan, PointerOrder::new(100));
    })
    .expect("valid viewport pointer plan");
    let lower = lower.then(|| ui.pressable_with_id(LOWER, BOUNDS, false));
    let output = ui.viewport_widget(&widget, pan_zoom, action_requests);
    let frame = ui.finish_output();
    Run {
        widget,
        widget_clone,
        output,
        lower,
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

fn pointer_move(point: Point, delta: Vec2) -> UiInput {
    let mut input = UiInput::default();
    input.pointer.primary.down = true;
    input.push_event(UiInputEvent::PointerMoved {
        position: point,
        delta,
    });
    input
}

fn wheel(point: Point, lines: f32) -> UiInput {
    let mut input = UiInput::default();
    input.push_event(UiInputEvent::Wheel {
        delta: InputWheelDelta::Lines(Vec2::new(0.0, lines)),
        position: Some(point),
    });
    input
}

fn request(id: &str, kind: ViewportActionKind, target: WidgetId) -> ViewportActionRequest {
    ViewportActionRequest::new(
        ActionId::new(id),
        kind,
        ActionSource::Shortcut,
        ActionContext::Editor,
        ViewportActionTarget::new(target),
        None,
    )
}

#[test]
fn scale_aware_texture_is_painted_inside_the_viewport_clip() {
    let mut surface = surface();
    surface.pan_zoom.actual_size();
    let config = ViewportWidgetConfig::new(VIEWPORT, surface);
    let mut pan_zoom = surface.pan_zoom;
    let mut memory = UiMemory::new();
    let run = run_frame(
        config,
        &mut pan_zoom,
        &mut memory,
        pointer_at(Point::new(110.0, 95.0)),
        ScaleFactor::new(2.0),
        &[],
        false,
    );

    assert_eq!(run.frame.primitives.len(), 4);
    assert!(matches!(run.frame.primitives[0], Primitive::Rect(_)));
    assert!(matches!(
        run.frame.primitives[1],
        Primitive::ClipBegin { rect, .. } if rect == BOUNDS
    ));
    let Primitive::Texture(texture) = run.frame.primitives[2] else {
        panic!("viewport texture");
    };
    assert_eq!(texture.texture, surface.texture);
    assert_eq!(texture.rect, Rect::new(60.0, 70.0, 200.0, 100.0));
    assert!(matches!(run.frame.primitives[3], Primitive::ClipEnd { .. }));
    assert_eq!(run.output.surface, surface);
    let content = Point::new(100.0, 50.0);
    let screen = Point::new(110.0, 95.0);
    assert_eq!(run.widget.content_to_screen(content), Some(screen));
    assert_eq!(run.widget.screen_to_content(screen), Some(content));
    assert_eq!(run.output.content_pointer, Some(content));
}

#[test]
fn pointer_plan_blocks_lower_content_and_click_focuses_with_grab_cursors() {
    let point = Point::new(100.0, 100.0);
    let config = config();
    let mut pan_zoom = config.surface.pan_zoom;
    let mut memory = UiMemory::new();

    let hovered = run_frame(
        config.clone(),
        &mut pan_zoom,
        &mut memory,
        pointer_at(point),
        ScaleFactor::ONE,
        &[],
        true,
    );
    assert!(hovered.output.response.state.hovered);
    assert!(!hovered.lower.expect("lower response").state.hovered);
    assert_eq!(
        hovered.frame.platform_requests,
        vec![PlatformRequest::SetCursor(CursorShape::Grab)]
    );

    let pressed = run_frame(
        config.clone(),
        &mut pan_zoom,
        &mut memory,
        pointer_button(point, true),
        ScaleFactor::ONE,
        &[],
        true,
    );
    assert!(!pressed.lower.expect("lower press").state.pressed);
    let clicked = run_frame(
        config.clone(),
        &mut pan_zoom,
        &mut memory,
        pointer_button(point, false),
        ScaleFactor::ONE,
        &[],
        true,
    );
    assert!(clicked.output.response.clicked);
    assert!(clicked.output.response.state.focused);
    assert!(memory.is_focused(VIEWPORT));
    assert_eq!(clicked.frame.repaint, RepaintRequest::NextFrame);

    let mut drag_pan_zoom = config.surface.pan_zoom;
    let mut drag_memory = UiMemory::new();
    let drag_pressed = run_frame(
        config.clone(),
        &mut drag_pan_zoom,
        &mut drag_memory,
        pointer_button(point, true),
        ScaleFactor::ONE,
        &[],
        false,
    );
    assert!(drag_pressed.output.response.state.focused);
    assert!(drag_memory.is_focused(VIEWPORT));
    let dragged = run_frame(
        config.clone(),
        &mut drag_pan_zoom,
        &mut drag_memory,
        pointer_move(Point::new(120.0, 110.0), Vec2::new(20.0, 10.0)),
        ScaleFactor::ONE,
        &[],
        false,
    );
    assert!(dragged.output.response.dragged);
    assert_eq!(
        dragged.frame.platform_requests,
        vec![PlatformRequest::SetCursor(CursorShape::Grabbing)]
    );
    let released = run_frame(
        config,
        &mut drag_pan_zoom,
        &mut drag_memory,
        pointer_button(Point::new(120.0, 110.0), false),
        ScaleFactor::ONE,
        &[],
        false,
    );
    assert_eq!(
        released.frame.platform_requests,
        vec![PlatformRequest::SetCursor(CursorShape::Grab)]
    );
    assert_eq!(released.frame.repaint, RepaintRequest::NextFrame);
}

#[test]
#[allow(clippy::too_many_lines)]
fn captured_pan_and_routed_wheel_present_clamped_state_in_the_same_frame() {
    let point = Point::new(100.0, 100.0);
    let config = config().with_zoom_range(0.5, 1.5);
    let frozen = config.surface;
    let mut pan_zoom = frozen.pan_zoom;
    let mut memory = UiMemory::new();
    let _ = run_frame(
        config.clone(),
        &mut pan_zoom,
        &mut memory,
        pointer_button(point, true),
        ScaleFactor::ONE,
        &[],
        false,
    );
    let panned = run_frame(
        config.clone(),
        &mut pan_zoom,
        &mut memory,
        pointer_move(Point::new(125.0, 90.0), Vec2::new(25.0, -10.0)),
        ScaleFactor::ONE,
        &[],
        false,
    );
    assert_eq!(panned.widget.surface(), frozen);
    assert_eq!(panned.widget, panned.widget_clone);
    assert_eq!(panned.output.next_pan_zoom.pan, Vec2::new(25.0, -10.0));
    assert_eq!(panned.output.surface.pan_zoom, panned.output.next_pan_zoom);
    assert_eq!(pan_zoom, panned.output.next_pan_zoom);
    assert!(panned.output.pan_changed);
    assert_eq!(panned.frame.repaint, RepaintRequest::NextFrame);
    let Primitive::Texture(texture) = panned.frame.primitives[2] else {
        panic!("viewport texture");
    };
    assert_eq!(
        texture.rect,
        panned
            .output
            .presentation_at(ScaleFactor::ONE)
            .content_rect()
    );
    let frozen_content = panned
        .widget
        .screen_to_content(point)
        .expect("frozen conversion");
    assert_eq!(panned.widget.content_to_screen(frozen_content), Some(point));

    let anchor = Point::new(161.0, 120.0);
    for scale_factor in [1.0, 1.25, 1.5, 2.0].map(ScaleFactor::new) {
        for initial_pan in [-0.000_1, 0.0, 0.000_1, 0.25] {
            let mut crossing = frozen;
            crossing.pan_zoom.pan.x = initial_pan;
            let crossing_config =
                ViewportWidgetConfig::new(VIEWPORT, crossing).with_zoom_range(0.5, 1.5);
            let prepared_clone = ViewportWidget::new(crossing_config.clone(), scale_factor).clone();
            let content_anchor = prepared_clone
                .screen_to_content(anchor)
                .expect("frozen anchor");
            let mut zoom_memory = UiMemory::new();
            let mut zoom = crossing.pan_zoom;
            let zoomed = run_frame(
                crossing_config,
                &mut zoom,
                &mut zoom_memory,
                wheel(anchor, 20.0),
                scale_factor,
                &[],
                false,
            );
            assert_eq!(prepared_clone.surface(), crossing);
            assert_eq!(zoomed.widget.surface(), crossing);
            assert_eq!(zoomed.output.surface.pan_zoom, zoomed.output.next_pan_zoom);
            assert_eq!(
                zoomed.output.next_pan_zoom.zoom.to_bits(),
                1.5_f32.to_bits()
            );
            assert!(zoomed.output.zoom_changed);
            let projected = zoomed
                .output
                .content_to_screen_at(content_anchor, scale_factor)
                .expect("effective anchor");
            assert!((projected.x - anchor.x).abs() <= 0.001);
            assert!((projected.y - anchor.y).abs() <= 0.001);
            if initial_pan.abs() > 0.01 {
                assert_eq!(
                    zoomed.output.next_pan_zoom.pan.x.is_sign_positive(),
                    initial_pan.is_sign_negative()
                );
            }
            let reconstructed = ViewportWidget::new(
                ViewportWidgetConfig::new(VIEWPORT, zoomed.output.surface),
                scale_factor,
            );
            assert_eq!(
                reconstructed.presentation(),
                zoomed.output.presentation_at(scale_factor)
            );
        }
    }

    let mut min_memory = UiMemory::new();
    let mut min_zoom = frozen.pan_zoom;
    let zoomed_out = run_frame(
        config,
        &mut min_zoom,
        &mut min_memory,
        wheel(point, -20.0),
        ScaleFactor::ONE,
        &[],
        false,
    );
    assert_eq!(
        zoomed_out.output.next_pan_zoom.zoom.to_bits(),
        0.5_f32.to_bits()
    );
}

#[test]
fn generic_actions_update_navigation_and_forward_only_targeted_app_requests() {
    let cases = [
        (ViewportActionKind::FitContent, ViewportFit::Fit),
        (ViewportActionKind::ActualSize, ViewportFit::ActualSize),
        (ViewportActionKind::ZoomIn, ViewportFit::Zoom),
        (ViewportActionKind::ZoomOut, ViewportFit::Zoom),
    ];
    for (kind, expected_fit) in cases {
        let config = config();
        let original_zoom = config.surface.pan_zoom.zoom;
        let mut pan_zoom = config.surface.pan_zoom;
        let mut memory = UiMemory::new();
        let run = run_frame(
            config,
            &mut pan_zoom,
            &mut memory,
            UiInput::default(),
            ScaleFactor::ONE,
            &[request("viewport.generic", kind, VIEWPORT)],
            false,
        );
        assert_eq!(run.output.next_pan_zoom.fit, expected_fit);
        assert_eq!(run.output.surface.pan_zoom, run.output.next_pan_zoom);
        let Primitive::Texture(texture) = run.frame.primitives[2] else {
            panic!("viewport texture");
        };
        assert_eq!(
            texture.rect,
            run.output.presentation_at(ScaleFactor::ONE).content_rect()
        );
        match kind {
            ViewportActionKind::ZoomIn => assert!(run.output.next_pan_zoom.zoom > original_zoom),
            ViewportActionKind::ZoomOut => assert!(run.output.next_pan_zoom.zoom < original_zoom),
            _ => {}
        }
        assert!(run.output.action_requests.is_empty());
        assert!(run.output.changed());
    }

    let unhandled = request(
        "viewport.fit-selection",
        ViewportActionKind::FitSelection,
        VIEWPORT,
    );
    let foreign = request(
        "viewport.foreign",
        ViewportActionKind::FitSelection,
        WidgetId::from_raw(99),
    );
    let config = config();
    let mut pan_zoom = config.surface.pan_zoom;
    let mut memory = UiMemory::new();
    let run = run_frame(
        config,
        &mut pan_zoom,
        &mut memory,
        UiInput::default(),
        ScaleFactor::ONE,
        &[unhandled.clone(), foreign],
        false,
    );
    assert_eq!(run.output.action_requests, vec![unhandled]);
    assert!(!run.output.changed());
}

#[test]
fn disabled_and_invalid_viewports_are_inert_and_safe() {
    let point = Point::new(100.0, 100.0);
    let disabled = config().disabled(true);
    let mut pan_zoom = disabled.surface.pan_zoom;
    let mut memory = UiMemory::new();
    let disabled_request = request("viewport.fit", ViewportActionKind::FitContent, VIEWPORT);
    let disabled_run = run_frame(
        disabled,
        &mut pan_zoom,
        &mut memory,
        pointer_button(point, true),
        ScaleFactor::ONE,
        std::slice::from_ref(&disabled_request),
        false,
    );
    assert!(disabled_run.output.response.state.disabled);
    assert!(!disabled_run.output.changed());
    assert_eq!(disabled_run.output.action_requests, vec![disabled_request]);
    assert!(disabled_run.frame.platform_requests.is_empty());

    let invalid_surface = ViewportSurface {
        texture: stern_core::TextureId::from_raw(8),
        source_size: Size::new(f32::NAN, 100.0),
        bounds: Rect::new(0.0, 0.0, f32::NAN, 100.0),
        pan_zoom: PanZoom::default(),
    };
    let invalid = ViewportWidgetConfig::new(VIEWPORT, invalid_surface);
    let mut invalid_pan_zoom = invalid.surface.pan_zoom;
    let mut invalid_memory = UiMemory::new();
    let invalid_run = run_frame(
        invalid,
        &mut invalid_pan_zoom,
        &mut invalid_memory,
        pointer_at(Point::new(0.0, 0.0)),
        ScaleFactor::new(0.0),
        &[],
        false,
    );
    assert!(invalid_run.output.response.state.disabled);
    assert!(!invalid_run.output.changed());
    assert!(invalid_run.frame.primitives.is_empty());
    let root = invalid_run
        .frame
        .semantics
        .get(VIEWPORT)
        .expect("root semantics");
    assert!(root.state.disabled);
    assert!(!root.focusable);
}

#[test]
fn root_and_action_semantic_ids_stay_stable_across_view_changes() {
    let fit = ViewportActionDescriptor::new(
        ActionDescriptor::new("viewport.fit", "Fit"),
        ViewportActionKind::FitContent,
        ViewportActionTarget::new(VIEWPORT),
    );
    let overlay = ViewportActionDescriptor::new(
        ActionDescriptor::new("viewport.overlay", "Overlay"),
        ViewportActionKind::ToggleOverlay,
        ViewportActionTarget::new(VIEWPORT),
    );
    let actions = vec![fit, overlay];
    let first_config = config().with_label("Canvas").with_actions(actions.clone());
    let mut first_state = first_config.surface.pan_zoom;
    let mut first_memory = UiMemory::new();
    let first = run_frame(
        first_config,
        &mut first_state,
        &mut first_memory,
        UiInput::default(),
        ScaleFactor::ONE,
        &[],
        false,
    );

    let mut changed_surface = surface();
    changed_surface.pan_zoom.actual_size();
    changed_surface.pan_zoom.pan_by(Vec2::new(20.0, 10.0));
    let second_config = ViewportWidgetConfig::new(VIEWPORT, changed_surface)
        .with_label("Canvas")
        .with_actions(actions);
    let mut second_state = changed_surface.pan_zoom;
    let mut second_memory = UiMemory::new();
    let second = run_frame(
        second_config,
        &mut second_state,
        &mut second_memory,
        UiInput::default(),
        ScaleFactor::new(1.5),
        &[],
        false,
    );

    let expected = vec![
        VIEWPORT,
        viewport_action_widget_id(VIEWPORT, &ActionId::new("viewport.fit")),
        viewport_action_widget_id(VIEWPORT, &ActionId::new("viewport.overlay")),
    ];
    assert_eq!(first.frame.semantics.traversal_order(), expected);
    assert_eq!(second.frame.semantics.traversal_order(), expected);
    let root = first.frame.semantics.get(VIEWPORT).expect("viewport root");
    assert_eq!(root.role, SemanticRole::Viewport);
    assert_eq!(root.label.as_deref(), Some("Canvas"));
    assert_eq!(root.children, expected[1..]);
    assert_eq!(
        first
            .frame
            .semantics
            .get(expected[1])
            .expect("fit action")
            .role,
        SemanticRole::Button
    );
    assert_eq!(
        first
            .frame
            .semantics
            .get(expected[2])
            .expect("overlay action")
            .role,
        SemanticRole::Toggle
    );
}
