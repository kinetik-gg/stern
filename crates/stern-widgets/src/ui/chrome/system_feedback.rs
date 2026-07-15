use stern_core::{
    Brush, ClipId, Color, ComponentState, Point, Primitive, Rect, RectPrimitive, RepaintRequest,
    SemanticAction, SemanticActionKind, SemanticNode, SemanticRole, SemanticValue, Stroke,
    TextPrimitive, TextRole,
};

use super::super::Ui;
use crate::chrome::{
    DiagnosticActionKind, DiagnosticStrip, DiagnosticStripSeverity, FeedbackKind, FeedbackStack,
    JobList, JobPhase, JobProgress, SystemFeedbackActionRow, SystemFeedbackOutput,
    SystemFeedbackResponse, SystemFeedbackRow, SystemFeedbackRowKind, SystemFeedbackScene,
    SystemFeedbackSceneConfig, SystemFeedbackSceneError, SystemFeedbackSurface,
    SystemFeedbackSurfaceLayout, SystemFeedbackTarget,
};
use crate::components::{ButtonFocusPlacement, button_surface_primitives};

impl Ui<'_> {
    /// Prepares jobs, diagnostics, and feedback using this frame's real time snapshot.
    ///
    /// # Errors
    ///
    /// Returns invalid geometry or the first repeated model identity.
    pub fn prepare_system_feedback<'a>(
        &self,
        config: SystemFeedbackSceneConfig,
        jobs: &'a JobList,
        diagnostics: &'a DiagnosticStrip,
        feedback: &'a FeedbackStack,
    ) -> Result<SystemFeedbackScene<'a>, SystemFeedbackSceneError> {
        SystemFeedbackScene::prepare(config, jobs, diagnostics, feedback, self.time().now)
    }

    /// Paints and evaluates one validated system-feedback scene.
    ///
    /// Call [`SystemFeedbackScene::declare_pointer_targets`] from the frame's
    /// closed pointer-plan prepass before evaluating this scene. Requests remain
    /// application-owned and their invocations are also queued on the frame.
    pub fn system_feedback(&mut self, scene: &SystemFeedbackScene<'_>) -> SystemFeedbackOutput {
        let repaint_request = scene.repaint_request();
        self.request_repaint(repaint_request);
        let mut output = SystemFeedbackOutput {
            repaint_request,
            ..SystemFeedbackOutput::default()
        };

        for surface in scene.layout() {
            self.paint_system_feedback_surface(&surface);
            let surface_children = surface.rows.iter().map(|row| row.id).collect::<Vec<_>>();
            self.register_id(surface.id);
            self.push_semantic_node(system_feedback_surface_semantics(
                &surface,
                surface_children,
            ));

            let clip = ClipId::from_raw(surface.id.child("clip").raw());
            self.primitive(Primitive::ClipBegin {
                id: clip,
                rect: surface.rect,
            });
            for row in surface.rows {
                self.register_id(row.id);
                self.paint_system_feedback_row(&row, scene.now());

                let mut action_children = Vec::with_capacity(row.actions.len());
                for action in &row.actions {
                    self.register_id(action.id);
                    let response = self.pressable_with_id(action.id, action.rect, !action.enabled);
                    self.paint_system_feedback_action(action, &response);
                    self.push_semantic_node(system_feedback_action_semantics(action, &response));
                    action_children.push(action.id);
                    output.responses.push(SystemFeedbackResponse {
                        target: action.target,
                        response,
                    });

                    if response.clicked
                        && let Some(request) = action.request.clone()
                    {
                        self.push_action(request.invocation().clone());
                        self.request_repaint(RepaintRequest::NextFrame);
                        output.requests.push(request);
                    }
                }
                self.push_semantic_node(system_feedback_row_semantics(&row, action_children));
            }
            self.primitive(Primitive::ClipEnd { id: clip });
        }
        output
    }

    fn paint_system_feedback_surface(&mut self, surface: &SystemFeedbackSurfaceLayout) {
        self.primitive(Primitive::Rect(RectPrimitive {
            rect: surface.rect,
            fill: Some(Brush::Solid(self.theme.colors.surface.sunken)),
            stroke: Some(Stroke::new(
                self.theme.strokes.hairline,
                Brush::Solid(self.theme.colors.border.subtle),
            )),
            radius: self.theme.radii.none,
        }));
    }

    fn paint_system_feedback_row(&mut self, row: &SystemFeedbackRow, now: std::time::Duration) {
        let tone = system_feedback_tone(self, row.kind);
        self.primitive(Primitive::Rect(RectPrimitive {
            rect: row.rect,
            fill: Some(Brush::Solid(self.theme.colors.surface.panel)),
            stroke: Some(Stroke::new(
                self.theme.strokes.hairline,
                Brush::Solid(self.theme.colors.border.subtle),
            )),
            radius: self.theme.radii.none,
        }));
        self.primitive(Primitive::Rect(RectPrimitive {
            rect: Rect::new(
                row.rect.x,
                row.rect.y,
                3.0_f32.min(row.rect.width),
                row.rect.height,
            ),
            fill: Some(Brush::Solid(tone)),
            stroke: None,
            radius: self.theme.radii.none,
        }));

        let content_clip = ClipId::from_raw(row.id.child("content-clip").raw());
        self.primitive(Primitive::ClipBegin {
            id: content_clip,
            rect: row.content_rect,
        });
        let font = self.theme.font(TextRole::Label);
        let baseline = row.rect.y + (row.rect.height - font.line_height).max(0.0) * 0.5 + font.size;
        let text = if row.detail.is_empty() {
            row.label.clone()
        } else {
            format!("{} — {}", row.label, row.detail)
        };
        self.primitive(Primitive::Text(TextPrimitive {
            layout: None,
            origin: Point::new(row.rect.x + self.theme.controls.padding_x + 3.0, baseline),
            text,
            family: font.family.to_owned(),
            size: font.size,
            line_height: font.line_height,
            brush: Brush::Solid(self.theme.colors.content.primary),
        }));
        self.primitive(Primitive::ClipEnd { id: content_clip });

        if let SystemFeedbackRowKind::Job { phase, progress } = row.kind {
            self.paint_job_progress(row.rect, phase, progress, tone, now);
        }
    }

    fn paint_job_progress(
        &mut self,
        row: Rect,
        phase: JobPhase,
        progress: JobProgress,
        tone: Color,
        now: std::time::Duration,
    ) {
        let track = Rect::new(row.x, row.max_y() - 3.0, row.width, 3.0_f32.min(row.height));
        let fill = match progress {
            JobProgress::Determinate(progress) => {
                Rect::new(track.x, track.y, track.width * progress.value, track.height)
            }
            JobProgress::Indeterminate if phase.is_active() => {
                let fraction = now.as_secs_f32().fract();
                let width = track.width * 0.25;
                Rect::new(
                    track.x + (track.width - width) * fraction,
                    track.y,
                    width,
                    track.height,
                )
            }
            JobProgress::Indeterminate => return,
        };
        self.primitive(Primitive::Rect(RectPrimitive {
            rect: track,
            fill: Some(Brush::Solid(self.theme.colors.border.subtle)),
            stroke: None,
            radius: self.theme.radii.none,
        }));
        if !fill.is_empty() {
            self.primitive(Primitive::Rect(RectPrimitive {
                rect: fill,
                fill: Some(Brush::Solid(tone)),
                stroke: None,
                radius: self.theme.radii.none,
            }));
        }
    }

    fn paint_system_feedback_action(
        &mut self,
        action: &SystemFeedbackActionRow,
        response: &stern_core::Response,
    ) {
        let state = ComponentState {
            hovered: response.state.hovered,
            pressed: response.state.pressed,
            focused: response.state.focused,
            disabled: !action.enabled,
            selected: false,
        };
        let recipe = self.theme.button(state);
        for primitive in button_surface_primitives(
            self.theme,
            &recipe,
            state,
            action.rect,
            recipe.radius,
            ButtonFocusPlacement::Inward,
        ) {
            self.primitive(primitive);
        }
        let font = self.theme.font(TextRole::Label);
        let baseline =
            action.rect.y + (action.rect.height - font.line_height).max(0.0) * 0.5 + font.size;
        self.primitive(Primitive::Text(TextPrimitive {
            layout: None,
            origin: Point::new(action.rect.x + self.theme.controls.padding_x, baseline),
            text: action.action.label.clone(),
            family: font.family.to_owned(),
            size: font.size,
            line_height: font.line_height,
            brush: Brush::Solid(recipe.foreground),
        }));
    }
}

fn system_feedback_tone(ui: &Ui<'_>, kind: SystemFeedbackRowKind) -> Color {
    match kind {
        SystemFeedbackRowKind::Job { phase, .. } => match phase {
            JobPhase::Queued => ui.theme.colors.content.muted,
            JobPhase::Running => ui.theme.colors.accent.default,
            JobPhase::Cancelling => ui.theme.colors.status.warning.strong,
            JobPhase::Succeeded => ui.theme.colors.status.success.strong,
            JobPhase::Failed => ui.theme.colors.status.danger.strong,
        },
        SystemFeedbackRowKind::Diagnostic(severity) => match severity {
            DiagnosticStripSeverity::Error => ui.theme.colors.status.danger.strong,
            DiagnosticStripSeverity::Warning => ui.theme.colors.status.warning.strong,
            DiagnosticStripSeverity::Info => ui.theme.colors.status.info.strong,
        },
        SystemFeedbackRowKind::Feedback(kind) => match kind {
            FeedbackKind::Info => ui.theme.colors.status.info.strong,
            FeedbackKind::Success => ui.theme.colors.status.success.strong,
            FeedbackKind::Warning => ui.theme.colors.status.warning.strong,
            FeedbackKind::Error => ui.theme.colors.status.danger.strong,
        },
    }
}

fn system_feedback_surface_semantics(
    surface: &SystemFeedbackSurfaceLayout,
    children: Vec<stern_core::WidgetId>,
) -> SemanticNode {
    let label = match surface.kind {
        SystemFeedbackSurface::Jobs => "Background jobs",
        SystemFeedbackSurface::Diagnostics => "Diagnostics",
        SystemFeedbackSurface::Feedback => "Notifications",
    };
    SemanticNode::new(surface.id, SemanticRole::List, surface.rect)
        .with_label(label)
        .with_children(children)
}

fn system_feedback_row_semantics(
    row: &SystemFeedbackRow,
    children: Vec<stern_core::WidgetId>,
) -> SemanticNode {
    let role = match row.target {
        SystemFeedbackTarget::Job(_) => SemanticRole::Custom("job".to_owned()),
        SystemFeedbackTarget::Diagnostic(_) => SemanticRole::Custom("diagnostic".to_owned()),
        SystemFeedbackTarget::Feedback(_) => SemanticRole::Custom("notification".to_owned()),
        SystemFeedbackTarget::JobCancel(_)
        | SystemFeedbackTarget::DiagnosticAction { .. }
        | SystemFeedbackTarget::FeedbackAction(_)
        | SystemFeedbackTarget::FeedbackDismiss(_) => SemanticRole::ListItem,
    };
    let mut node = SemanticNode::new(row.id, role, row.rect)
        .with_label(&row.label)
        .with_children(children);
    if !row.detail.is_empty() {
        node.description = Some(row.detail.clone());
    }
    if let SystemFeedbackRowKind::Job {
        progress: JobProgress::Determinate(progress),
        ..
    } = row.kind
    {
        node.state.value = Some(SemanticValue::Number {
            current: progress.value,
            min: 0.0,
            max: 1.0,
        });
    }
    node
}

fn system_feedback_action_semantics(
    action: &SystemFeedbackActionRow,
    response: &stern_core::Response,
) -> SemanticNode {
    let mut node = SemanticNode::new(action.id, SemanticRole::Button, action.rect)
        .with_label(&action.action.label);
    node.state.disabled = !action.enabled;
    node.state.focused = response.state.focused;
    node.state.pressed = response.state.pressed;
    if action.enabled {
        node = node.focusable(true).with_action(SemanticAction {
            kind: semantic_action_kind(action.target),
            label: action.action.label.clone(),
            action_id: Some(action.action.id.clone()),
        });
    }
    node
}

fn semantic_action_kind(target: SystemFeedbackTarget) -> SemanticActionKind {
    match target {
        SystemFeedbackTarget::DiagnosticAction {
            kind: DiagnosticActionKind::Dismiss,
            ..
        }
        | SystemFeedbackTarget::FeedbackDismiss(_) => SemanticActionKind::Dismiss,
        SystemFeedbackTarget::Job(_)
        | SystemFeedbackTarget::JobCancel(_)
        | SystemFeedbackTarget::Diagnostic(_)
        | SystemFeedbackTarget::DiagnosticAction {
            kind: DiagnosticActionKind::Report,
            ..
        }
        | SystemFeedbackTarget::Feedback(_)
        | SystemFeedbackTarget::FeedbackAction(_) => SemanticActionKind::Invoke,
    }
}
