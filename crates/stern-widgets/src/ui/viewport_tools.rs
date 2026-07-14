use stern_core::{
    Brush, ClipId, CursorShape, DomainDragGestureAction, DomainDragGesturePhase, Modifiers, Point,
    Primitive, Rect, RectPrimitive, RepaintRequest, Stroke,
};

use super::Ui;
use crate::viewport::{
    ViewportCursorShape, ViewportToolController, ViewportToolScene, ViewportToolSceneConfig,
    ViewportToolSceneOutput, ViewportTransformDragCapture, ViewportTransformHandleDescriptor,
    ViewportTransformHandleKind, ViewportTransformHandleResponse,
    ViewportTransformInteractionPhase, ViewportTransformInteractionRequest, ViewportWidget,
};

impl Ui<'_> {
    /// Prepares a transform-tool scene through a viewport's frozen geometry.
    #[must_use]
    pub fn prepare_viewport_tool_scene(
        &self,
        viewport: &ViewportWidget,
        config: ViewportToolSceneConfig,
    ) -> ViewportToolScene {
        ViewportToolScene::new(viewport, config)
    }

    /// Evaluates and paints one viewport transform-tool scene.
    ///
    /// Returned requests are data only. The application remains responsible
    /// for snapping, constraints, geometry mutation, undo, and persistence.
    #[allow(clippy::too_many_lines)]
    pub fn viewport_tool_scene(
        &mut self,
        scene: &ViewportToolScene,
        controller: &mut ViewportToolController,
    ) -> ViewportToolSceneOutput {
        let mut output = ViewportToolSceneOutput::default();
        let mut repaint = false;

        if let Some(capture) = controller.capture.clone()
            && scene.handle(capture.handle).is_none()
        {
            let id = scene.handle_widget_id(capture.handle);
            self.register_id(id);
            let gesture =
                self.runtime
                    .captured_domain_drag_gesture(id, capture.handle_screen_rect, true);
            let cancellation = gesture
                .actions
                .iter()
                .find(|action| matches!(action.phase, DomainDragGesturePhase::Cancel));
            let (ordinal, modifiers, point) = cancellation.map_or_else(
                || {
                    (
                        None,
                        self.input().keyboard.modifiers,
                        self.input().pointer.position,
                    )
                },
                |action| (action.ordinal, action.modifiers, action.position),
            );
            if controller.started {
                output.interactions.push(transform_interaction(
                    scene,
                    &capture,
                    ViewportTransformInteractionPhase::Cancelled,
                    ordinal,
                    modifiers,
                    point,
                ));
            }
            *controller = ViewportToolController::default();
            repaint = true;
        }

        for handle in scene.handles() {
            let id = scene.handle_widget_id(handle.id);
            self.register_id(id);
            let gesture = self.runtime.captured_domain_drag_gesture(
                id,
                handle.handle_screen_rect,
                scene.config().disabled,
            );
            let drag_crossed_threshold = gesture.response.dragged
                || self.memory().is_drag_source(id)
                || self.memory().released_drag_source() == Some(id);

            if gesture.response.state.hovered {
                output.hovered_handle = Some(handle.id);
            }
            let start_move = (!controller.started && drag_crossed_threshold)
                .then(|| {
                    gesture
                        .actions
                        .iter()
                        .rposition(|action| matches!(action.phase, DomainDragGesturePhase::Move))
                })
                .flatten();
            for (index, action) in gesture.actions.iter().enumerate() {
                repaint |= self.apply_viewport_handle_action(
                    scene,
                    handle,
                    controller,
                    action,
                    drag_crossed_threshold,
                    start_move == Some(index),
                    &mut output,
                );
            }

            if gesture.response.state.hovered
                || controller.captured_handle() == Some(handle.id)
                || self.memory().released_drag_source() == Some(id)
            {
                self.runtime.request_cursor_for(
                    id,
                    viewport_cursor_shape(&handle.cursor.shape, self.memory().is_drag_source(id)),
                );
            }

            repaint |= !gesture.actions.is_empty()
                || gesture.response.clicked
                || gesture.response.dragged
                || gesture.response.state.pressed;
            output
                .handle_responses
                .push(ViewportTransformHandleResponse {
                    handle: handle.id,
                    widget_id: id,
                    response: gesture.response,
                });
        }

        if output.hovered_handle.is_none() && controller.captured_handle().is_none() {
            self.request_active_tool_cursor(scene);
        }
        self.paint_viewport_tool_scene(scene, controller, &output);

        if repaint {
            self.request_repaint(RepaintRequest::NextFrame);
        }
        output
    }

    #[allow(clippy::too_many_arguments)]
    fn apply_viewport_handle_action(
        &mut self,
        scene: &ViewportToolScene,
        handle: &ViewportTransformHandleDescriptor,
        controller: &mut ViewportToolController,
        action: &DomainDragGestureAction,
        drag_crossed_threshold: bool,
        starts_transform: bool,
        output: &mut ViewportToolSceneOutput,
    ) -> bool {
        match action.phase {
            DomainDragGesturePhase::Press => {
                let point = action
                    .position
                    .unwrap_or_else(|| rect_center(handle.handle_screen_rect));
                controller.capture = Some(scene.capture_from_handle(handle, point));
                controller.started = false;
                self.runtime.memory_mut().focus(scene.viewport_id());
                true
            }
            DomainDragGesturePhase::Move => {
                let Some(capture) = matching_capture(controller, handle) else {
                    return false;
                };
                if !controller.started && !starts_transform {
                    return false;
                }
                let phase = if controller.started {
                    ViewportTransformInteractionPhase::Updated
                } else {
                    controller.started = true;
                    ViewportTransformInteractionPhase::Started
                };
                let fallback = Point::new(
                    capture.pointer_origin_screen.x + action.delta.x,
                    capture.pointer_origin_screen.y + action.delta.y,
                );
                output.interactions.push(transform_interaction(
                    scene,
                    &capture,
                    phase,
                    action.ordinal,
                    action.modifiers,
                    action.position.or(Some(fallback)),
                ));
                true
            }
            DomainDragGesturePhase::Release => {
                let Some(capture) = matching_capture(controller, handle) else {
                    return false;
                };
                if !controller.started && drag_crossed_threshold {
                    controller.started = true;
                    output.interactions.push(transform_interaction(
                        scene,
                        &capture,
                        ViewportTransformInteractionPhase::Started,
                        action.ordinal,
                        action.modifiers,
                        action.position,
                    ));
                }
                if controller.started {
                    output.interactions.push(transform_interaction(
                        scene,
                        &capture,
                        ViewportTransformInteractionPhase::Finished,
                        action.ordinal,
                        action.modifiers,
                        action.position,
                    ));
                }
                *controller = ViewportToolController::default();
                true
            }
            DomainDragGesturePhase::Cancel => {
                let Some(capture) = matching_capture(controller, handle) else {
                    return false;
                };
                if controller.started {
                    output.interactions.push(transform_interaction(
                        scene,
                        &capture,
                        ViewportTransformInteractionPhase::Cancelled,
                        action.ordinal,
                        action.modifiers,
                        action.position,
                    ));
                }
                *controller = ViewportToolController::default();
                true
            }
        }
    }

    fn request_active_tool_cursor(&mut self, scene: &ViewportToolScene) {
        if scene.config().disabled {
            return;
        }
        let Some(tool) = scene.config().active_tool.as_ref() else {
            return;
        };
        let Some(cursor) = tool.cursor_request() else {
            return;
        };
        if self
            .input()
            .pointer
            .position
            .is_some_and(|point| scene.surface().contains_screen_point(point))
        {
            self.runtime.request_cursor_for(
                scene.viewport_id(),
                viewport_cursor_shape(&cursor.shape, false),
            );
        }
    }

    fn paint_viewport_tool_scene(
        &mut self,
        scene: &ViewportToolScene,
        controller: &ViewportToolController,
        output: &ViewportToolSceneOutput,
    ) {
        let bounds = scene.surface().effective_bounds();
        if bounds.width <= 0.0 || bounds.height <= 0.0 {
            return;
        }

        let clip = ClipId::from_raw(scene.viewport_id().child("viewport-tools-clip").raw());
        self.primitive(Primitive::ClipBegin {
            id: clip,
            rect: bounds,
        });
        for outline in scene.outlines() {
            self.primitive(Primitive::Rect(RectPrimitive {
                rect: outline.screen_rect,
                fill: None,
                stroke: Some(Stroke::new(
                    self.theme.controls.border_width,
                    Brush::Solid(self.theme.colors.accent.default),
                )),
                radius: self.theme.radii.none,
            }));
        }
        for handle in scene.handles() {
            if handle.kind == ViewportTransformHandleKind::Move {
                continue;
            }
            let response = output
                .handle_responses
                .iter()
                .find(|response| response.handle == handle.id)
                .map(|response| &response.response);
            let active = controller.captured_handle() == Some(handle.id);
            let hovered = response.is_some_and(|response| response.state.hovered);
            let fill = if active {
                self.theme.colors.surface.control_pressed
            } else if hovered {
                self.theme.colors.surface.control_hover
            } else {
                self.theme.colors.surface.control
            };
            let border = if active || hovered {
                self.theme.colors.focus.ring
            } else {
                self.theme.colors.accent.default
            };
            self.primitive(Primitive::Rect(RectPrimitive {
                rect: handle.handle_screen_rect,
                fill: Some(Brush::Solid(fill)),
                stroke: Some(Stroke::new(
                    self.theme.controls.border_width,
                    Brush::Solid(border),
                )),
                radius: self.theme.radii.xs,
            }));
        }
        self.primitive(Primitive::ClipEnd { id: clip });
    }
}

fn matching_capture(
    controller: &ViewportToolController,
    handle: &ViewportTransformHandleDescriptor,
) -> Option<ViewportTransformDragCapture> {
    controller
        .capture
        .as_ref()
        .filter(|capture| capture.handle == handle.id)
        .cloned()
}

fn transform_interaction(
    scene: &ViewportToolScene,
    capture: &ViewportTransformDragCapture,
    phase: ViewportTransformInteractionPhase,
    event_ordinal: Option<usize>,
    modifiers: Modifiers,
    point: Option<Point>,
) -> ViewportTransformInteractionRequest {
    let point = point.unwrap_or(capture.pointer_origin_screen);
    ViewportTransformInteractionRequest {
        phase,
        event_ordinal,
        modifiers,
        snap_tolerance: scene.config().snap_tolerance,
        drag: crate::viewport::ViewportTransformDragRequest::update_at(
            scene.surface(),
            scene.targets(),
            capture,
            point,
            scene.scale_factor(),
        ),
    }
}

fn viewport_cursor_shape(shape: &ViewportCursorShape, active: bool) -> CursorShape {
    match shape {
        ViewportCursorShape::Default | ViewportCursorShape::Custom(_) => CursorShape::Default,
        ViewportCursorShape::Pointer => CursorShape::PointingHand,
        ViewportCursorShape::Crosshair | ViewportCursorShape::Rotate => CursorShape::Crosshair,
        ViewportCursorShape::Grab => CursorShape::Grab,
        ViewportCursorShape::Grabbing => CursorShape::Grabbing,
        ViewportCursorShape::Text => CursorShape::Text,
        ViewportCursorShape::Move => {
            if active {
                CursorShape::Grabbing
            } else {
                CursorShape::Grab
            }
        }
        ViewportCursorShape::ResizeHorizontal => CursorShape::ResizeHorizontal,
        ViewportCursorShape::ResizeVertical => CursorShape::ResizeVertical,
        ViewportCursorShape::ResizeTopLeftBottomRight => CursorShape::ResizeTopLeftBottomRight,
        ViewportCursorShape::ResizeTopRightBottomLeft => CursorShape::ResizeTopRightBottomLeft,
    }
}

fn rect_center(rect: Rect) -> Point {
    Point::new(rect.x + rect.width * 0.5, rect.y + rect.height * 0.5)
}
