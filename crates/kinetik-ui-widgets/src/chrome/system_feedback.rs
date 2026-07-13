//! Deterministic frame-local scene for jobs, diagnostics, and user feedback.

use std::collections::HashSet;
use std::fmt;
use std::time::Duration;

use kinetik_ui_core::{
    ActionContext, ActionDescriptor, ActionInvocation, ActionSource, PointerOrder, PointerTarget,
    PointerTargetPlan, Rect, RepaintRequest, Response, WidgetId,
};

use super::{
    DiagnosticStrip, DiagnosticStripItemId, DiagnosticStripSeverity, FeedbackActionRequest,
    FeedbackDismissRequest, FeedbackId, FeedbackKind, FeedbackStack, JobCancelRequest, JobList,
    JobPhase, JobProgress, JobRowId,
};

/// One surface in the combined system-feedback scene.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SystemFeedbackSurface {
    /// Application-owned background jobs.
    Jobs,
    /// Aggregated diagnostics.
    Diagnostics,
    /// Transient or pinned user feedback.
    Feedback,
}

/// Stable identity for a row or action in a system-feedback scene.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SystemFeedbackTarget {
    /// Passive job row.
    Job(JobRowId),
    /// Job cancellation affordance.
    JobCancel(JobRowId),
    /// Passive diagnostic row.
    Diagnostic(DiagnosticStripItemId),
    /// Diagnostic action affordance.
    DiagnosticAction {
        /// Application-owned diagnostic identity.
        diagnostic_id: DiagnosticStripItemId,
        /// Kind of diagnostic action.
        kind: DiagnosticActionKind,
    },
    /// Passive active-feedback row.
    Feedback(FeedbackId),
    /// Primary feedback action affordance.
    FeedbackAction(FeedbackId),
    /// Feedback dismissal affordance.
    FeedbackDismiss(FeedbackId),
}

/// Supported application-owned diagnostic actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DiagnosticActionKind {
    /// Remove the diagnostic from application state.
    Dismiss,
    /// Open or send a diagnostic report through application code.
    Report,
}

/// Action metadata attached to one diagnostic affordance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticAction {
    /// Action metadata shared with other action surfaces.
    pub action: ActionDescriptor,
    /// Context emitted with the invocation.
    pub context: ActionContext,
}

impl DiagnosticAction {
    /// Creates diagnostic action metadata.
    #[must_use]
    pub const fn new(action: ActionDescriptor, context: ActionContext) -> Self {
        Self { action, context }
    }

    /// Returns true when the action should be painted.
    #[must_use]
    pub const fn visible(&self) -> bool {
        self.action.state.visible
    }

    /// Returns true when the painted action can be invoked.
    #[must_use]
    pub const fn enabled(&self) -> bool {
        self.action.state.enabled
    }

    fn request(
        &self,
        diagnostic_id: DiagnosticStripItemId,
        kind: DiagnosticActionKind,
    ) -> Option<DiagnosticActionRequest> {
        self.action.can_invoke().then(|| {
            DiagnosticActionRequest::new(
                diagnostic_id,
                kind,
                ActionInvocation::new(
                    self.action.id.clone(),
                    ActionSource::Button,
                    self.context.clone(),
                ),
            )
        })
    }
}

/// Optional dismiss and report actions for one diagnostic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticActionSet {
    /// Application-owned diagnostic identity.
    pub diagnostic_id: DiagnosticStripItemId,
    /// Optional dismissal action.
    pub dismiss: Option<DiagnosticAction>,
    /// Optional reporting action.
    pub report: Option<DiagnosticAction>,
}

impl DiagnosticActionSet {
    /// Creates an action set with no visible actions.
    #[must_use]
    pub const fn new(diagnostic_id: DiagnosticStripItemId) -> Self {
        Self {
            diagnostic_id,
            dismiss: None,
            report: None,
        }
    }

    /// Sets the optional dismissal action.
    #[must_use]
    pub fn with_dismiss(mut self, action: DiagnosticAction) -> Self {
        self.dismiss = Some(action);
        self
    }

    /// Sets the optional reporting action.
    #[must_use]
    pub fn with_report(mut self, action: DiagnosticAction) -> Self {
        self.report = Some(action);
        self
    }
}

/// Typed request emitted by a diagnostic action affordance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticActionRequest {
    /// Application-owned diagnostic identity.
    pub diagnostic_id: DiagnosticStripItemId,
    /// Kind of diagnostic action requested.
    pub kind: DiagnosticActionKind,
    /// Action invocation queued for application dispatch.
    pub invocation: ActionInvocation,
}

impl DiagnosticActionRequest {
    /// Creates a diagnostic action request.
    #[must_use]
    pub const fn new(
        diagnostic_id: DiagnosticStripItemId,
        kind: DiagnosticActionKind,
        invocation: ActionInvocation,
    ) -> Self {
        Self {
            diagnostic_id,
            kind,
            invocation,
        }
    }
}

/// Application-owned request emitted by the combined scene.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SystemFeedbackRequest {
    /// Cancel one active job.
    JobCancel(JobCancelRequest),
    /// Dismiss or report one diagnostic.
    Diagnostic(DiagnosticActionRequest),
    /// Invoke one active feedback item's primary action.
    FeedbackAction(FeedbackActionRequest),
    /// Dismiss one active feedback item.
    FeedbackDismiss(FeedbackDismissRequest),
}

impl SystemFeedbackRequest {
    /// Returns the action invocation also appended to the frame action queue.
    #[must_use]
    pub const fn invocation(&self) -> &ActionInvocation {
        match self {
            Self::JobCancel(request) => &request.invocation,
            Self::Diagnostic(request) => &request.invocation,
            Self::FeedbackAction(request) => &request.invocation,
            Self::FeedbackDismiss(request) => &request.invocation,
        }
    }
}

/// One evaluated action response and its stable scene target.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SystemFeedbackResponse {
    /// Stable row action identity.
    pub target: SystemFeedbackTarget,
    /// Neutral press/focus response.
    pub response: Response,
}

/// Result of painting and evaluating one system-feedback scene.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SystemFeedbackOutput {
    /// Action affordance responses in paint order.
    pub responses: Vec<SystemFeedbackResponse>,
    /// Application-owned requests in event order.
    pub requests: Vec<SystemFeedbackRequest>,
    /// Repaint scheduling derived from the same frozen scene snapshot.
    pub repaint_request: RepaintRequest,
}

/// Caller-owned geometry and diagnostic action metadata.
#[derive(Debug, Clone, PartialEq)]
pub struct SystemFeedbackSceneConfig {
    /// Stable root used to derive every surface, row, and action ID.
    pub root: WidgetId,
    /// Bounds for background-job rows.
    pub jobs_rect: Rect,
    /// Bounds for diagnostic rows.
    pub diagnostics_rect: Rect,
    /// Bounds for active feedback rows.
    pub feedback_rect: Rect,
    /// Fixed row height for this compact MVP surface.
    pub row_height: f32,
    /// Fixed width for each visible action affordance.
    pub action_width: f32,
    /// Gap between adjacent action affordances.
    pub action_gap: f32,
    /// Optional application-owned actions keyed by diagnostic identity.
    pub diagnostic_actions: Vec<DiagnosticActionSet>,
}

impl SystemFeedbackSceneConfig {
    /// Creates a scene config with caller-owned surface bounds.
    #[must_use]
    pub const fn new(
        root: WidgetId,
        jobs_rect: Rect,
        diagnostics_rect: Rect,
        feedback_rect: Rect,
    ) -> Self {
        Self {
            root,
            jobs_rect,
            diagnostics_rect,
            feedback_rect,
            row_height: 32.0,
            action_width: 72.0,
            action_gap: 4.0,
            diagnostic_actions: Vec::new(),
        }
    }

    /// Sets the fixed row height.
    #[must_use]
    pub const fn with_row_height(mut self, row_height: f32) -> Self {
        self.row_height = row_height;
        self
    }

    /// Sets the fixed action width.
    #[must_use]
    pub const fn with_action_width(mut self, action_width: f32) -> Self {
        self.action_width = action_width;
        self
    }

    /// Sets the gap between adjacent action affordances.
    #[must_use]
    pub const fn with_action_gap(mut self, action_gap: f32) -> Self {
        self.action_gap = action_gap;
        self
    }

    /// Replaces diagnostic action metadata.
    #[must_use]
    pub fn with_diagnostic_actions(
        mut self,
        actions: impl IntoIterator<Item = DiagnosticActionSet>,
    ) -> Self {
        self.diagnostic_actions = actions.into_iter().collect();
        self
    }
}

/// Deterministic preparation failure for a frozen system-feedback scene.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemFeedbackSceneError {
    /// One surface has non-finite or negative geometry.
    InvalidBounds(SystemFeedbackSurface),
    /// The fixed row height is non-finite or non-positive.
    InvalidRowHeight,
    /// The action width is non-finite or non-positive.
    InvalidActionWidth,
    /// The action gap is non-finite or negative.
    InvalidActionGap,
    /// Two job rows reused one application identity.
    DuplicateJobId(JobRowId),
    /// Two diagnostic rows reused one application identity.
    DuplicateDiagnosticId(DiagnosticStripItemId),
    /// Two feedback rows reused one application identity.
    DuplicateFeedbackId(FeedbackId),
}

impl fmt::Display for SystemFeedbackSceneError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidBounds(surface) => write!(formatter, "invalid {surface:?} bounds"),
            Self::InvalidRowHeight => formatter.write_str("row height must be finite and positive"),
            Self::InvalidActionWidth => {
                formatter.write_str("action width must be finite and positive")
            }
            Self::InvalidActionGap => {
                formatter.write_str("action gap must be finite and non-negative")
            }
            Self::DuplicateJobId(id) => write!(formatter, "duplicate job id {}", id.raw()),
            Self::DuplicateDiagnosticId(id) => {
                write!(formatter, "duplicate diagnostic id {}", id.raw())
            }
            Self::DuplicateFeedbackId(id) => {
                write!(formatter, "duplicate feedback id {}", id.raw())
            }
        }
    }
}

impl std::error::Error for SystemFeedbackSceneError {}

/// Borrowed, validated scene over application-owned feedback snapshots.
#[derive(Debug)]
pub struct SystemFeedbackScene<'a> {
    config: SystemFeedbackSceneConfig,
    jobs: &'a JobList,
    diagnostics: &'a DiagnosticStrip,
    feedback: &'a FeedbackStack,
    now: Duration,
}

impl<'a> SystemFeedbackScene<'a> {
    /// Validates IDs and geometry, then freezes explicit frame time for evaluation.
    ///
    /// # Errors
    ///
    /// Returns the first invalid geometry field or repeated model identity.
    pub fn prepare(
        config: SystemFeedbackSceneConfig,
        jobs: &'a JobList,
        diagnostics: &'a DiagnosticStrip,
        feedback: &'a FeedbackStack,
        now: Duration,
    ) -> Result<Self, SystemFeedbackSceneError> {
        validate_config(&config)?;
        validate_unique_ids(jobs, diagnostics, feedback)?;
        Ok(Self {
            config,
            jobs,
            diagnostics,
            feedback,
            now,
        })
    }

    /// Returns the explicit frame time captured during preparation.
    #[must_use]
    pub const fn now(&self) -> Duration {
        self.now
    }

    /// Returns the stable widget ID for one scene surface.
    #[must_use]
    pub fn surface_widget_id(&self, surface: SystemFeedbackSurface) -> WidgetId {
        surface_widget_id(self.config.root, surface)
    }

    /// Returns the stable widget ID for one row or action target.
    #[must_use]
    pub fn target_widget_id(&self, target: SystemFeedbackTarget) -> WidgetId {
        target_widget_id(self.config.root, target)
    }

    /// Returns bounded redraw scheduling for timed feedback and indeterminate work.
    #[must_use]
    pub fn repaint_request(&self) -> RepaintRequest {
        let jobs = if self
            .jobs
            .rows()
            .iter()
            .any(|row| row.phase.is_active() && row.progress == JobProgress::Indeterminate)
        {
            RepaintRequest::Continuous
        } else {
            RepaintRequest::None
        };
        jobs.merge(self.feedback.repaint_request(self.now))
    }

    /// Adds surface blockers and visible action targets to a caller-owned pointer plan.
    ///
    /// The returned order is the first unused ordinal after this scene.
    pub fn declare_pointer_targets(
        &self,
        plan: &mut PointerTargetPlan,
        first_order: PointerOrder,
    ) -> PointerOrder {
        let mut ordinal = first_order.raw();
        for surface in self.layout() {
            plan.blocker(surface.rect, take_order(&mut ordinal));
            plan.with_clip(surface.rect, |plan| {
                for row in surface.rows {
                    for action in row.actions {
                        if action.enabled {
                            plan.target(PointerTarget::new(
                                action.id,
                                action.rect,
                                take_order(&mut ordinal),
                            ));
                        }
                    }
                }
            });
        }
        PointerOrder::new(ordinal)
    }

    pub(crate) fn layout(&self) -> Vec<SystemFeedbackSurfaceLayout> {
        [
            self.surface_layout(
                SystemFeedbackSurface::Jobs,
                self.config.jobs_rect,
                self.job_rows(),
            ),
            self.surface_layout(
                SystemFeedbackSurface::Diagnostics,
                self.config.diagnostics_rect,
                self.diagnostic_rows(),
            ),
            self.surface_layout(
                SystemFeedbackSurface::Feedback,
                self.config.feedback_rect,
                self.feedback_rows(),
            ),
        ]
        .into_iter()
        .flatten()
        .collect()
    }

    fn job_rows(&self) -> Vec<SystemFeedbackRowSource> {
        self.jobs
            .rows()
            .iter()
            .map(|job| {
                let mut actions = Vec::new();
                if job.phase.is_active()
                    && let Some(cancel) = job.cancel.as_ref().filter(|cancel| cancel.visible())
                {
                    actions.push(SystemFeedbackActionRowSource {
                        target: SystemFeedbackTarget::JobCancel(job.id),
                        action: cancel.action.clone(),
                        request: job.cancel_request().map(SystemFeedbackRequest::JobCancel),
                    });
                }
                SystemFeedbackRowSource {
                    target: SystemFeedbackTarget::Job(job.id),
                    label: job.label.clone(),
                    detail: job
                        .detail
                        .clone()
                        .unwrap_or_else(|| format!("{:?}", job.phase)),
                    kind: SystemFeedbackRowKind::Job {
                        phase: job.phase,
                        progress: job.progress,
                    },
                    actions,
                }
            })
            .collect()
    }

    fn diagnostic_rows(&self) -> Vec<SystemFeedbackRowSource> {
        self.diagnostics
            .ordered_items()
            .into_iter()
            .map(|item| {
                let mut actions = Vec::new();
                if let Some(set) = self
                    .config
                    .diagnostic_actions
                    .iter()
                    .find(|set| set.diagnostic_id == item.id)
                {
                    append_diagnostic_action(
                        &mut actions,
                        item.id,
                        DiagnosticActionKind::Dismiss,
                        set.dismiss.as_ref(),
                    );
                    append_diagnostic_action(
                        &mut actions,
                        item.id,
                        DiagnosticActionKind::Report,
                        set.report.as_ref(),
                    );
                }
                SystemFeedbackRowSource {
                    target: SystemFeedbackTarget::Diagnostic(item.id),
                    label: format!("{}: {}", item.code, item.message),
                    detail: item
                        .source
                        .as_ref()
                        .map_or_else(String::new, |source| format!("{source:?}")),
                    kind: SystemFeedbackRowKind::Diagnostic(item.severity),
                    actions,
                }
            })
            .collect()
    }

    fn feedback_rows(&self) -> Vec<SystemFeedbackRowSource> {
        self.feedback
            .active_items_iter(self.now)
            .map(|item| {
                let mut actions = Vec::new();
                if let Some(action) = item.action.as_ref().filter(|action| action.visible()) {
                    actions.push(SystemFeedbackActionRowSource {
                        target: SystemFeedbackTarget::FeedbackAction(item.id),
                        action: action.action.clone(),
                        request: item
                            .action_request()
                            .map(SystemFeedbackRequest::FeedbackAction),
                    });
                }
                if let Some(dismiss) = item.dismiss.as_ref().filter(|dismiss| dismiss.visible()) {
                    actions.push(SystemFeedbackActionRowSource {
                        target: SystemFeedbackTarget::FeedbackDismiss(item.id),
                        action: dismiss.action.clone(),
                        request: item
                            .dismiss_request()
                            .map(SystemFeedbackRequest::FeedbackDismiss),
                    });
                }
                SystemFeedbackRowSource {
                    target: SystemFeedbackTarget::Feedback(item.id),
                    label: item.label.clone(),
                    detail: item.text.clone(),
                    kind: SystemFeedbackRowKind::Feedback(item.kind),
                    actions,
                }
            })
            .collect()
    }

    fn surface_layout(
        &self,
        kind: SystemFeedbackSurface,
        rect: Rect,
        rows: impl IntoIterator<Item = SystemFeedbackRowSource>,
    ) -> Option<SystemFeedbackSurfaceLayout> {
        if rect.is_empty() {
            return None;
        }
        let mut y = rect.y;
        let rows = rows
            .into_iter()
            .filter_map(|row| {
                let row_rect = Rect::new(rect.x, y, rect.width, self.config.row_height);
                y += self.config.row_height;
                row_rect.intersection(rect).map(|visible_rect| {
                    layout_row(
                        self.config.root,
                        visible_rect,
                        self.config.action_width,
                        self.config.action_gap,
                        row,
                    )
                })
            })
            .collect();
        Some(SystemFeedbackSurfaceLayout {
            id: surface_widget_id(self.config.root, kind),
            kind,
            rect,
            rows,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SystemFeedbackSurfaceLayout {
    pub(crate) id: WidgetId,
    pub(crate) kind: SystemFeedbackSurface,
    pub(crate) rect: Rect,
    pub(crate) rows: Vec<SystemFeedbackRow>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SystemFeedbackRow {
    pub(crate) id: WidgetId,
    pub(crate) target: SystemFeedbackTarget,
    pub(crate) rect: Rect,
    pub(crate) content_rect: Rect,
    pub(crate) label: String,
    pub(crate) detail: String,
    pub(crate) kind: SystemFeedbackRowKind,
    pub(crate) actions: Vec<SystemFeedbackActionRow>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum SystemFeedbackRowKind {
    Job {
        phase: JobPhase,
        progress: JobProgress,
    },
    Diagnostic(DiagnosticStripSeverity),
    Feedback(FeedbackKind),
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SystemFeedbackActionRow {
    pub(crate) id: WidgetId,
    pub(crate) target: SystemFeedbackTarget,
    pub(crate) rect: Rect,
    pub(crate) action: ActionDescriptor,
    pub(crate) enabled: bool,
    pub(crate) request: Option<SystemFeedbackRequest>,
}

struct SystemFeedbackRowSource {
    target: SystemFeedbackTarget,
    label: String,
    detail: String,
    kind: SystemFeedbackRowKind,
    actions: Vec<SystemFeedbackActionRowSource>,
}

struct SystemFeedbackActionRowSource {
    target: SystemFeedbackTarget,
    action: ActionDescriptor,
    request: Option<SystemFeedbackRequest>,
}

fn append_diagnostic_action(
    output: &mut Vec<SystemFeedbackActionRowSource>,
    diagnostic_id: DiagnosticStripItemId,
    kind: DiagnosticActionKind,
    action: Option<&DiagnosticAction>,
) {
    let Some(action) = action.filter(|action| action.visible()) else {
        return;
    };
    output.push(SystemFeedbackActionRowSource {
        target: SystemFeedbackTarget::DiagnosticAction {
            diagnostic_id,
            kind,
        },
        action: action.action.clone(),
        request: action
            .request(diagnostic_id, kind)
            .map(SystemFeedbackRequest::Diagnostic),
    });
}

fn layout_row(
    root: WidgetId,
    rect: Rect,
    action_width: f32,
    action_gap: f32,
    source: SystemFeedbackRowSource,
) -> SystemFeedbackRow {
    let reserved = source
        .actions
        .iter()
        .fold((0.0, true), |(width, first), _| {
            (
                width + action_width + if first { 0.0 } else { action_gap },
                false,
            )
        })
        .0
        .min(rect.width);
    let content_rect = Rect::new(
        rect.x,
        rect.y,
        (rect.width - reserved).max(0.0),
        rect.height,
    );
    let mut right = rect.max_x();
    let actions = source
        .actions
        .into_iter()
        .filter_map(|action| {
            Rect::new(right - action_width, rect.y, action_width, rect.height)
                .intersection(rect)
                .map(|action_rect| {
                    right -= action_width + action_gap;
                    SystemFeedbackActionRow {
                        id: target_widget_id(root, action.target),
                        target: action.target,
                        rect: action_rect,
                        enabled: action.request.is_some(),
                        action: action.action,
                        request: action.request,
                    }
                })
        })
        .collect();
    SystemFeedbackRow {
        id: target_widget_id(root, source.target),
        target: source.target,
        rect,
        content_rect,
        label: source.label,
        detail: source.detail,
        kind: source.kind,
        actions,
    }
}

fn validate_config(config: &SystemFeedbackSceneConfig) -> Result<(), SystemFeedbackSceneError> {
    for (surface, rect) in [
        (SystemFeedbackSurface::Jobs, config.jobs_rect),
        (SystemFeedbackSurface::Diagnostics, config.diagnostics_rect),
        (SystemFeedbackSurface::Feedback, config.feedback_rect),
    ] {
        if !valid_optional_rect(rect) {
            return Err(SystemFeedbackSceneError::InvalidBounds(surface));
        }
    }
    if !config.row_height.is_finite() || config.row_height <= 0.0 {
        return Err(SystemFeedbackSceneError::InvalidRowHeight);
    }
    if !config.action_width.is_finite() || config.action_width <= 0.0 {
        return Err(SystemFeedbackSceneError::InvalidActionWidth);
    }
    if !config.action_gap.is_finite() || config.action_gap < 0.0 {
        return Err(SystemFeedbackSceneError::InvalidActionGap);
    }
    Ok(())
}

fn validate_unique_ids(
    jobs: &JobList,
    diagnostics: &DiagnosticStrip,
    feedback: &FeedbackStack,
) -> Result<(), SystemFeedbackSceneError> {
    let mut seen = HashSet::new();
    for row in jobs.rows() {
        if !seen.insert(row.id) {
            return Err(SystemFeedbackSceneError::DuplicateJobId(row.id));
        }
    }
    let mut seen = HashSet::new();
    for item in diagnostics.items() {
        if !seen.insert(item.id) {
            return Err(SystemFeedbackSceneError::DuplicateDiagnosticId(item.id));
        }
    }
    let mut seen = HashSet::new();
    for item in feedback.items() {
        if !seen.insert(item.id) {
            return Err(SystemFeedbackSceneError::DuplicateFeedbackId(item.id));
        }
    }
    Ok(())
}

fn valid_optional_rect(rect: Rect) -> bool {
    rect.x.is_finite()
        && rect.y.is_finite()
        && rect.width.is_finite()
        && rect.height.is_finite()
        && rect.width >= 0.0
        && rect.height >= 0.0
        && rect.max_x().is_finite()
        && rect.max_y().is_finite()
}

fn surface_widget_id(root: WidgetId, surface: SystemFeedbackSurface) -> WidgetId {
    root.child(("system-feedback-surface", surface))
}

fn target_widget_id(root: WidgetId, target: SystemFeedbackTarget) -> WidgetId {
    match target {
        SystemFeedbackTarget::Job(id) => {
            surface_widget_id(root, SystemFeedbackSurface::Jobs).child(("job", id.raw()))
        }
        SystemFeedbackTarget::JobCancel(id) => {
            target_widget_id(root, SystemFeedbackTarget::Job(id)).child("cancel")
        }
        SystemFeedbackTarget::Diagnostic(id) => {
            surface_widget_id(root, SystemFeedbackSurface::Diagnostics)
                .child(("diagnostic", id.raw()))
        }
        SystemFeedbackTarget::DiagnosticAction {
            diagnostic_id,
            kind,
        } => target_widget_id(root, SystemFeedbackTarget::Diagnostic(diagnostic_id))
            .child(("action", kind)),
        SystemFeedbackTarget::Feedback(id) => {
            surface_widget_id(root, SystemFeedbackSurface::Feedback).child(("feedback", id.raw()))
        }
        SystemFeedbackTarget::FeedbackAction(id) => {
            target_widget_id(root, SystemFeedbackTarget::Feedback(id)).child("action")
        }
        SystemFeedbackTarget::FeedbackDismiss(id) => {
            target_widget_id(root, SystemFeedbackTarget::Feedback(id)).child("dismiss")
        }
    }
}

fn take_order(ordinal: &mut u64) -> PointerOrder {
    let order = PointerOrder::new(*ordinal);
    *ordinal = ordinal.saturating_add(1);
    order
}
