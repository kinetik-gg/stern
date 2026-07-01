use kinetik_ui_core::{ActionContext, ActionDescriptor, ActionInvocation, ActionSource};

use super::status_bar::StatusProgress;

/// Stable application-owned identity for a job row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct JobRowId(u64);

impl JobRowId {
    /// Creates a job row ID from raw bits.
    #[must_use]
    pub const fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    /// Returns the raw ID bits.
    #[must_use]
    pub const fn raw(self) -> u64 {
        self.0
    }
}

/// Application-supplied presentation phase for a job row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum JobPhase {
    /// Work is waiting to start.
    Queued,
    /// Work is currently running.
    Running,
    /// Cancellation has been requested and is being acknowledged by the application.
    Cancelling,
    /// Work finished successfully.
    Succeeded,
    /// Work finished with an application-owned failure state.
    Failed,
}

impl JobPhase {
    /// Returns true when the phase still represents active or pending work.
    #[must_use]
    pub const fn is_active(self) -> bool {
        matches!(self, Self::Queued | Self::Running | Self::Cancelling)
    }
}

/// Application-supplied progress metadata for a job row.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum JobProgress {
    /// Work has no deterministic fraction yet.
    Indeterminate,
    /// Work has a sanitized deterministic fraction.
    Determinate(StatusProgress),
}

impl JobProgress {
    /// Creates determinate progress metadata, replacing non-finite values with `0.0` and clamping.
    #[must_use]
    pub fn determinate(value: f32) -> Self {
        Self::Determinate(StatusProgress::new(value))
    }

    /// Creates determinate progress metadata from a completed/total pair.
    #[must_use]
    pub fn from_fraction(completed: f32, total: f32) -> Self {
        Self::Determinate(StatusProgress::from_fraction(completed, total))
    }

    /// Returns the determinate status progress value, when available.
    #[must_use]
    pub const fn status_progress(self) -> Option<StatusProgress> {
        match self {
            Self::Indeterminate => None,
            Self::Determinate(progress) => Some(progress),
        }
    }
}

/// Application-owned cancel action metadata for a job row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobCancel {
    /// Action metadata shared with menus, command palettes, shortcuts, and buttons.
    pub action: ActionDescriptor,
    /// Context emitted with the cancel action invocation.
    pub context: ActionContext,
}

impl JobCancel {
    /// Creates cancel metadata from an application-owned action descriptor.
    #[must_use]
    pub const fn new(action: ActionDescriptor, context: ActionContext) -> Self {
        Self { action, context }
    }

    /// Returns true when the cancel affordance should be presented.
    #[must_use]
    pub const fn visible(&self) -> bool {
        self.action.state.visible
    }

    /// Returns true when the cancel affordance can currently be invoked.
    #[must_use]
    pub const fn enabled(&self) -> bool {
        self.action.state.enabled
    }

    /// Returns true when this cancel action is both visible and enabled.
    #[must_use]
    pub const fn can_request(&self) -> bool {
        self.action.can_invoke()
    }

    /// Creates a cancel request for an enabled visible cancel action.
    #[must_use]
    pub fn request(&self, job_id: JobRowId) -> Option<JobCancelRequest> {
        self.can_request().then(|| {
            JobCancelRequest::new(
                job_id,
                ActionInvocation::new(
                    self.action.id.clone(),
                    ActionSource::Button,
                    self.context.clone(),
                ),
            )
        })
    }
}

/// Cancel request metadata emitted by a job presentation affordance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobCancelRequest {
    /// Stable job row identity supplied by the application.
    pub job_id: JobRowId,
    /// Action invocation for the application to execute.
    pub invocation: ActionInvocation,
}

impl JobCancelRequest {
    /// Creates cancel request metadata.
    #[must_use]
    pub const fn new(job_id: JobRowId, invocation: ActionInvocation) -> Self {
        Self { job_id, invocation }
    }
}

/// Data-only application-supplied job row snapshot.
#[derive(Debug, Clone, PartialEq)]
pub struct JobRow {
    /// Stable job row identity.
    pub id: JobRowId,
    /// Short label for compact presentation or accessibility.
    pub label: String,
    /// Current phase supplied by the application.
    pub phase: JobPhase,
    /// Progress metadata supplied by the application.
    pub progress: JobProgress,
    /// Optional secondary detail text supplied by the application.
    pub detail: Option<String>,
    /// Optional cancel action metadata supplied by the application.
    pub cancel: Option<JobCancel>,
}

impl JobRow {
    /// Creates a job row with indeterminate progress and no cancel metadata.
    #[must_use]
    pub fn new(id: JobRowId, label: impl Into<String>, phase: JobPhase) -> Self {
        Self {
            id,
            label: label.into(),
            phase,
            progress: JobProgress::Indeterminate,
            detail: None,
            cancel: None,
        }
    }

    /// Sets progress metadata.
    #[must_use]
    pub const fn with_progress(mut self, progress: JobProgress) -> Self {
        self.progress = progress;
        self
    }

    /// Sets secondary detail text.
    #[must_use]
    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    /// Sets cancel metadata.
    #[must_use]
    pub fn with_cancel(mut self, cancel: JobCancel) -> Self {
        self.cancel = Some(cancel);
        self
    }

    /// Returns true when this row can emit a cancel request.
    #[must_use]
    pub fn can_cancel(&self) -> bool {
        self.cancel.as_ref().is_some_and(JobCancel::can_request)
    }

    /// Creates cancel request metadata for this row when cancellation is available and enabled.
    #[must_use]
    pub fn cancel_request(&self) -> Option<JobCancelRequest> {
        self.cancel.as_ref()?.request(self.id)
    }
}

/// Deterministic summary counts for a job list snapshot.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct JobSummaryCounts {
    /// Number of queued jobs.
    pub queued: u32,
    /// Number of running jobs.
    pub running: u32,
    /// Number of cancelling jobs.
    pub cancelling: u32,
    /// Number of succeeded jobs.
    pub succeeded: u32,
    /// Number of failed jobs.
    pub failed: u32,
}

impl JobSummaryCounts {
    /// Returns active or pending jobs.
    #[must_use]
    pub const fn active(self) -> u32 {
        self.queued + self.running + self.cancelling
    }

    /// Returns the total number of jobs in the summary.
    #[must_use]
    pub const fn total(self) -> u32 {
        self.active() + self.succeeded + self.failed
    }
}

/// Active job progress metadata suitable for status bar presentation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ActiveJobProgress {
    /// Number of active or pending jobs.
    pub active: u32,
    /// Number of active or pending jobs with determinate progress.
    pub determinate: u32,
    /// Number of active or pending jobs with indeterminate progress.
    pub indeterminate: u32,
    /// Aggregate active progress without inventing percentages for indeterminate work.
    pub progress: JobProgress,
}

impl ActiveJobProgress {
    /// Returns determinate status progress only when all active work is determinate.
    #[must_use]
    pub const fn status_progress(self) -> Option<StatusProgress> {
        match self.progress {
            JobProgress::Determinate(progress) if self.indeterminate == 0 => Some(progress),
            JobProgress::Determinate(_) | JobProgress::Indeterminate => None,
        }
    }
}

/// Data-only job list model made of ordered application-owned rows.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct JobList {
    rows: Vec<JobRow>,
}

impl JobList {
    /// Creates an empty job list.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a job list from ordered row snapshots.
    #[must_use]
    pub fn from_rows(rows: impl IntoIterator<Item = JobRow>) -> Self {
        Self {
            rows: rows.into_iter().collect(),
        }
    }

    /// Returns job rows in application-supplied presentation order.
    #[must_use]
    pub fn rows(&self) -> &[JobRow] {
        &self.rows
    }

    /// Replaces job rows while preserving application-supplied order.
    pub fn replace_rows(&mut self, rows: impl IntoIterator<Item = JobRow>) {
        self.rows = rows.into_iter().collect();
    }

    /// Returns a job row by stable identity.
    #[must_use]
    pub fn row(&self, id: JobRowId) -> Option<&JobRow> {
        self.rows.iter().find(|row| row.id == id)
    }

    /// Returns summary counts by phase.
    #[must_use]
    pub fn summary(&self) -> JobSummaryCounts {
        let mut summary = JobSummaryCounts::default();
        for row in &self.rows {
            match row.phase {
                JobPhase::Queued => summary.queued += 1,
                JobPhase::Running => summary.running += 1,
                JobPhase::Cancelling => summary.cancelling += 1,
                JobPhase::Succeeded => summary.succeeded += 1,
                JobPhase::Failed => summary.failed += 1,
            }
        }
        summary
    }

    /// Returns active or pending job count suitable for `StatusItemKind::JobCount`.
    #[must_use]
    pub fn active_count(&self) -> u32 {
        self.summary().active()
    }

    /// Returns aggregate progress metadata for active or pending jobs.
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn active_progress(&self) -> Option<ActiveJobProgress> {
        let mut active = 0_u32;
        let mut determinate = 0_u32;
        let mut indeterminate = 0_u32;
        let mut progress_sum = 0.0_f32;

        for row in &self.rows {
            if !row.phase.is_active() {
                continue;
            }
            active += 1;
            match row.progress {
                JobProgress::Indeterminate => indeterminate += 1,
                JobProgress::Determinate(progress) => {
                    determinate += 1;
                    progress_sum += progress.value;
                }
            }
        }

        if active == 0 {
            return None;
        }

        let progress = if indeterminate == 0 && determinate > 0 {
            JobProgress::determinate(progress_sum / determinate as f32)
        } else {
            JobProgress::Indeterminate
        };

        Some(ActiveJobProgress {
            active,
            determinate,
            indeterminate,
            progress,
        })
    }

    /// Returns determinate status progress only when all active work has determinate progress.
    #[must_use]
    pub fn active_status_progress(&self) -> Option<StatusProgress> {
        self.active_progress()?.status_progress()
    }

    /// Creates cancel request metadata for a row by stable job identity.
    #[must_use]
    pub fn cancel_request(&self, id: JobRowId) -> Option<JobCancelRequest> {
        self.row(id)?.cancel_request()
    }
}
