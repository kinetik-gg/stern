//! Windowless status bar conformance for reusable editor chrome contracts.

use kinetik_ui_widgets::{
    DiagnosticSource, DiagnosticStrip, DiagnosticStripItem, DiagnosticStripItemId,
    DiagnosticStripSeverity, JobCancel, JobList, JobPhase, JobProgress, JobRow, JobRowId,
    StatusBar, StatusItem, StatusItemId, StatusItemKind, StatusProgress,
};

use kinetik_ui_core::{ActionContext, ActionDescriptor, ActionId, ActionSource};

fn status_id(raw: u64) -> StatusItemId {
    StatusItemId::from_raw(raw)
}

fn job_id(raw: u64) -> JobRowId {
    JobRowId::from_raw(raw)
}

fn diagnostic_id(raw: u64) -> DiagnosticStripItemId {
    DiagnosticStripItemId::from_raw(raw)
}

fn assert_close(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < f32::EPSILON,
        "expected {actual} to equal {expected}"
    );
}

#[test]
fn status_bar_visible_items_preserve_order_and_filter_hidden_items() {
    let status_bar = StatusBar::from_items([
        StatusItem::new(
            status_id(1),
            "Ready",
            "Viewport ready",
            StatusItemKind::Ready,
        ),
        StatusItem::new(
            status_id(2),
            "Hidden",
            "Internal state",
            StatusItemKind::Message,
        )
        .with_visible(false),
        StatusItem::new(
            status_id(3),
            "Queued",
            "3 jobs queued",
            StatusItemKind::JobCount,
        )
        .with_count(3),
    ]);

    let visible = status_bar.visible_items();

    assert_eq!(visible.len(), 2);
    assert_eq!(visible[0].id, status_id(1));
    assert_eq!(visible[0].label, "Ready");
    assert_eq!(visible[1].id, status_id(3));
    assert_eq!(visible[1].count, Some(3));
    assert_eq!(
        status_bar.item(status_id(2)).map(|item| item.visible),
        Some(false)
    );
}

#[test]
fn status_bar_progress_values_sanitize_and_clamp_deterministically() {
    assert_close(StatusProgress::new(f32::NAN).value, 0.0);
    assert_close(StatusProgress::new(f32::INFINITY).value, 0.0);
    assert_close(StatusProgress::new(-0.25).value, 0.0);
    assert_close(StatusProgress::new(1.25).value, 1.0);
    assert_close(StatusProgress::from_fraction(5.0, 10.0).value, 0.5);
    assert_close(StatusProgress::from_fraction(5.0, 0.0).value, 0.0);

    let item = StatusItem::new(
        status_id(4),
        "Render",
        "Rendering preview",
        StatusItemKind::Progress,
    )
    .with_progress_value(1.8);

    assert_close(item.progress.expect("progress metadata").value, 1.0);
}

#[test]
fn status_bar_represents_ready_pending_stale_and_error_as_typed_metadata() {
    let status_bar = StatusBar::from_items([
        StatusItem::new(status_id(1), "Ready", "Ready", StatusItemKind::Ready),
        StatusItem::new(status_id(2), "Pending", "Loading", StatusItemKind::Pending),
        StatusItem::new(status_id(3), "Stale", "Out of date", StatusItemKind::Stale),
        StatusItem::new(status_id(4), "Error", "Failed", StatusItemKind::Error),
    ]);

    let kinds = status_bar
        .items()
        .iter()
        .map(|item| item.kind)
        .collect::<Vec<_>>();

    assert_eq!(
        kinds,
        vec![
            StatusItemKind::Ready,
            StatusItemKind::Pending,
            StatusItemKind::Stale,
            StatusItemKind::Error,
        ]
    );
}

#[test]
fn status_bar_diagnostics_strip_orders_by_severity_and_preserves_insertion_order_within_severity() {
    let strip = DiagnosticStrip::from_items([
        DiagnosticStripItem::new(
            diagnostic_id(1),
            DiagnosticStripSeverity::Warning,
            "KUI-WARN-A",
            "First warning",
        ),
        DiagnosticStripItem::new(
            diagnostic_id(2),
            DiagnosticStripSeverity::Info,
            "KUI-INFO",
            "Informational note",
        ),
        DiagnosticStripItem::new(
            diagnostic_id(3),
            DiagnosticStripSeverity::Error,
            "KUI-ERR",
            "Error",
        )
        .with_source(DiagnosticSource::Renderer)
        .with_field("texture", "missing"),
        DiagnosticStripItem::new(
            diagnostic_id(4),
            DiagnosticStripSeverity::Warning,
            "KUI-WARN-B",
            "Second warning",
        ),
    ]);

    let ordered = strip.ordered_items();

    assert_eq!(
        ordered.iter().map(|item| item.id).collect::<Vec<_>>(),
        vec![
            diagnostic_id(3),
            diagnostic_id(1),
            diagnostic_id(4),
            diagnostic_id(2),
        ]
    );
    assert_eq!(ordered[0].source, Some(DiagnosticSource::Renderer));
    assert_eq!(ordered[0].fields[0].name, "texture");
}

#[test]
fn status_bar_diagnostics_strip_summary_counts_are_deterministic_for_empty_and_mixed_input() {
    assert_eq!(DiagnosticStrip::new().summary().total(), 0);

    let strip = DiagnosticStrip::from_items([
        DiagnosticStripItem::new(
            diagnostic_id(1),
            DiagnosticStripSeverity::Warning,
            "KUI-WARN-A",
            "First warning",
        ),
        DiagnosticStripItem::new(
            diagnostic_id(2),
            DiagnosticStripSeverity::Error,
            "KUI-ERR-A",
            "First error",
        ),
        DiagnosticStripItem::new(
            diagnostic_id(3),
            DiagnosticStripSeverity::Error,
            "KUI-ERR-B",
            "Second error",
        ),
        DiagnosticStripItem::new(
            diagnostic_id(4),
            DiagnosticStripSeverity::Info,
            "KUI-INFO",
            "Info",
        ),
    ]);
    let summary = strip.summary();

    assert_eq!(summary.errors, 2);
    assert_eq!(summary.warnings, 1);
    assert_eq!(summary.info, 1);
    assert_eq!(summary.total(), 4);
}

#[test]
fn job_progress_clamps_and_sanitizes_determinate_values_without_affecting_indeterminate() {
    assert_eq!(
        JobProgress::Indeterminate.status_progress(),
        None,
        "indeterminate progress must not become fake zero percent"
    );
    assert_close(
        JobProgress::determinate(f32::NAN)
            .status_progress()
            .expect("determinate progress")
            .value,
        0.0,
    );
    assert_close(
        JobProgress::determinate(f32::INFINITY)
            .status_progress()
            .expect("determinate progress")
            .value,
        0.0,
    );
    assert_close(
        JobProgress::determinate(-0.25)
            .status_progress()
            .expect("determinate progress")
            .value,
        0.0,
    );
    assert_close(
        JobProgress::determinate(1.25)
            .status_progress()
            .expect("determinate progress")
            .value,
        1.0,
    );
    assert_close(
        JobProgress::from_fraction(2.0, 4.0)
            .status_progress()
            .expect("determinate progress")
            .value,
        0.5,
    );
}

#[test]
fn job_list_summary_counts_and_row_order_are_deterministic() {
    let jobs = JobList::from_rows([
        JobRow::new(job_id(30), "Queued", JobPhase::Queued),
        JobRow::new(job_id(10), "Running", JobPhase::Running)
            .with_progress(JobProgress::determinate(0.25)),
        JobRow::new(job_id(20), "Cancelling", JobPhase::Cancelling),
        JobRow::new(job_id(40), "Done", JobPhase::Succeeded),
        JobRow::new(job_id(50), "Failed", JobPhase::Failed),
    ]);

    assert_eq!(
        jobs.rows().iter().map(|row| row.id).collect::<Vec<_>>(),
        vec![job_id(30), job_id(10), job_id(20), job_id(40), job_id(50)]
    );

    let summary = jobs.summary();
    assert_eq!(summary.queued, 1);
    assert_eq!(summary.running, 1);
    assert_eq!(summary.cancelling, 1);
    assert_eq!(summary.succeeded, 1);
    assert_eq!(summary.failed, 1);
    assert_eq!(summary.active(), 3);
    assert_eq!(summary.total(), 5);
    assert_eq!(jobs.active_count(), 3);
}

#[test]
fn job_list_active_progress_keeps_indeterminate_distinct_from_determinate_zero() {
    let determinate = JobList::from_rows([
        JobRow::new(job_id(1), "First", JobPhase::Running)
            .with_progress(JobProgress::determinate(0.0)),
        JobRow::new(job_id(2), "Second", JobPhase::Queued)
            .with_progress(JobProgress::determinate(0.5)),
        JobRow::new(job_id(3), "Done", JobPhase::Succeeded)
            .with_progress(JobProgress::determinate(1.0)),
    ]);

    let progress = determinate
        .active_progress()
        .expect("active determinate progress");
    assert_eq!(progress.active, 2);
    assert_eq!(progress.determinate, 2);
    assert_eq!(progress.indeterminate, 0);
    assert_close(
        progress
            .status_progress()
            .expect("status progress for determinate active work")
            .value,
        0.25,
    );
    assert_close(
        determinate
            .active_status_progress()
            .expect("status progress")
            .value,
        0.25,
    );

    let mixed = JobList::from_rows([
        JobRow::new(job_id(1), "First", JobPhase::Running)
            .with_progress(JobProgress::determinate(0.0)),
        JobRow::new(job_id(2), "Second", JobPhase::Queued)
            .with_progress(JobProgress::Indeterminate),
    ]);
    let mixed_progress = mixed.active_progress().expect("active mixed progress");

    assert_eq!(mixed_progress.active, 2);
    assert_eq!(mixed_progress.determinate, 1);
    assert_eq!(mixed_progress.indeterminate, 1);
    assert_eq!(mixed_progress.progress, JobProgress::Indeterminate);
    assert_eq!(mixed.active_status_progress(), None);
}

#[test]
fn job_cancel_metadata_preserves_job_action_identity_and_availability() {
    let enabled_action = ActionDescriptor::new("job.cancel.render", "Cancel render");
    let mut disabled_action = ActionDescriptor::new("job.cancel.disabled", "Cancel disabled");
    disabled_action.state.enabled = false;
    let mut hidden_action = ActionDescriptor::new("job.cancel.hidden", "Cancel hidden");
    hidden_action.state.visible = false;

    let jobs = JobList::from_rows([
        JobRow::new(job_id(1), "Render", JobPhase::Running)
            .with_cancel(JobCancel::new(enabled_action, ActionContext::Global)),
        JobRow::new(job_id(2), "Bake", JobPhase::Running)
            .with_cancel(JobCancel::new(disabled_action, ActionContext::Editor)),
        JobRow::new(job_id(3), "Scan", JobPhase::Queued)
            .with_cancel(JobCancel::new(hidden_action, ActionContext::Global)),
        JobRow::new(job_id(4), "Upload", JobPhase::Running),
    ]);

    let request = jobs
        .cancel_request(job_id(1))
        .expect("enabled visible cancel request");

    assert_eq!(request.job_id, job_id(1));
    assert_eq!(
        request.invocation.action_id,
        ActionId::new("job.cancel.render")
    );
    assert_eq!(request.invocation.source, ActionSource::Button);
    assert_eq!(request.invocation.context, ActionContext::Global);

    let disabled = jobs.row(job_id(2)).expect("disabled cancel row");
    assert!(disabled.cancel.as_ref().is_some_and(JobCancel::visible));
    assert!(!disabled.can_cancel());
    assert_eq!(jobs.cancel_request(job_id(2)), None);

    let hidden = jobs.row(job_id(3)).expect("hidden cancel row");
    assert!(!hidden.cancel.as_ref().is_some_and(JobCancel::visible));
    assert_eq!(jobs.cancel_request(job_id(3)), None);
    assert_eq!(jobs.cancel_request(job_id(4)), None);
}
