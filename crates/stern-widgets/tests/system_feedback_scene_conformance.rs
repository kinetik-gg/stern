//! Public prepared system-feedback scene conformance tests.

use std::time::Duration;

use stern_core::{
    ActionContext, ActionDescriptor, ActionId, ActionSource, FrameContext, FrameOutput,
    PhysicalSize, Point, PointerButtonState, PointerInput, PointerOrder, PointerTarget, Primitive,
    Rect, RepaintRequest, Response, ScaleFactor, SemanticActionKind, SemanticRole, SemanticValue,
    Size, TimeInfo, UiInput, UiMemory, ViewportInfo, WidgetId, default_dark_theme,
};
use stern_widgets::Ui;
use stern_widgets::chrome::{
    DiagnosticAction, DiagnosticActionKind, DiagnosticActionSet, DiagnosticStrip,
    DiagnosticStripItem, DiagnosticStripItemId, DiagnosticStripSeverity, FeedbackAction,
    FeedbackDismiss, FeedbackId, FeedbackItem, FeedbackKind, FeedbackStack, JobCancel, JobList,
    JobPhase, JobProgress, JobRow, JobRowId, SystemFeedbackOutput, SystemFeedbackRequest,
    SystemFeedbackScene, SystemFeedbackSceneConfig, SystemFeedbackSceneError,
    SystemFeedbackSurface, SystemFeedbackTarget,
};

const JOBS_RECT: Rect = Rect::new(0.0, 0.0, 240.0, 32.0);
const DIAGNOSTICS_RECT: Rect = Rect::new(0.0, 40.0, 240.0, 32.0);
const FEEDBACK_RECT: Rect = Rect::new(0.0, 80.0, 240.0, 32.0);
const LOWER_RECT: Rect = Rect::new(0.0, 0.0, 280.0, 140.0);
const FRAME_NOW: Duration = Duration::from_secs(5);

fn job_id(raw: u64) -> JobRowId {
    JobRowId::from_raw(raw)
}

fn diagnostic_id(raw: u64) -> DiagnosticStripItemId {
    DiagnosticStripItemId::from_raw(raw)
}

fn feedback_id(raw: u64) -> FeedbackId {
    FeedbackId::from_raw(raw)
}

fn action(id: &str, label: &str) -> ActionDescriptor {
    ActionDescriptor::new(id, label)
}

fn diagnostic(raw: u64, severity: DiagnosticStripSeverity) -> DiagnosticStripItem {
    DiagnosticStripItem::new(
        diagnostic_id(raw),
        severity,
        format!("STERN-{raw}"),
        format!("Diagnostic {raw}"),
    )
}

fn config() -> SystemFeedbackSceneConfig {
    SystemFeedbackSceneConfig::new(
        WidgetId::from_key("system-feedback-test"),
        JOBS_RECT,
        DIAGNOSTICS_RECT,
        FEEDBACK_RECT,
    )
    .with_row_height(32.0)
    .with_action_width(64.0)
    .with_action_gap(4.0)
}

fn actionable_config() -> SystemFeedbackSceneConfig {
    config().with_diagnostic_actions([DiagnosticActionSet::new(diagnostic_id(7))
        .with_dismiss(DiagnosticAction::new(
            action("diagnostic.dismiss", "Dismiss"),
            ActionContext::Global,
        ))
        .with_report(DiagnosticAction::new(
            action("diagnostic.report", "Report"),
            ActionContext::Editor,
        ))])
}

fn actionable_models() -> (JobList, DiagnosticStrip, FeedbackStack) {
    let jobs = JobList::from_rows([JobRow::new(job_id(7), "Render", JobPhase::Running)
        .with_progress(JobProgress::determinate(0.5))
        .with_cancel(JobCancel::new(
            action("job.cancel", "Cancel"),
            ActionContext::Global,
        ))]);
    let diagnostics = DiagnosticStrip::from_items([diagnostic(7, DiagnosticStripSeverity::Error)]);
    let feedback = FeedbackStack::from_items([FeedbackItem::pinned(
        feedback_id(7),
        FeedbackKind::Success,
        "Export ready",
        "The export can be opened",
    )
    .with_action(FeedbackAction::new(
        action("feedback.open", "Open"),
        ActionContext::Editor,
    ))
    .with_dismiss(FeedbackDismiss::new(
        action("feedback.dismiss", "Dismiss"),
        ActionContext::Global,
    ))]);
    (jobs, diagnostics, feedback)
}

fn context(input: UiInput, now: Duration) -> FrameContext {
    FrameContext::new(
        ViewportInfo::new(
            Size::new(320.0, 180.0),
            PhysicalSize::new(320, 180),
            ScaleFactor::ONE,
        ),
        input,
        TimeInfo::new(now, Duration::from_millis(16), 9),
    )
}

fn primary_input(point: Point, down: bool, pressed: bool, released: bool) -> UiInput {
    UiInput {
        pointer: PointerInput {
            position: Some(point),
            primary: PointerButtonState::new(down, pressed, released),
            ..PointerInput::default()
        },
        ..UiInput::default()
    }
}

struct Run {
    captured_now: Duration,
    lower: Option<Response>,
    output: SystemFeedbackOutput,
    frame: FrameOutput,
}

#[allow(clippy::too_many_arguments)]
fn run_frame(
    config: SystemFeedbackSceneConfig,
    jobs: &JobList,
    diagnostics: &DiagnosticStrip,
    feedback: &FeedbackStack,
    memory: &mut UiMemory,
    input: UiInput,
    now: Duration,
    lower: bool,
) -> Run {
    let theme = default_dark_theme();
    let mut ui = Ui::begin_frame(context(input, now), memory, &theme);
    let scene = ui
        .prepare_system_feedback(config, jobs, diagnostics, feedback)
        .expect("valid system feedback scene");
    let captured_now = scene.now();
    let lower_id = ui.make_id("lower-content");
    ui.resolve_pointer_targets(|plan| {
        if lower {
            plan.target(PointerTarget::new(
                lower_id,
                LOWER_RECT,
                PointerOrder::new(10),
            ));
        }
        scene.declare_pointer_targets(plan, PointerOrder::new(100));
    })
    .expect("valid shared pointer plan");
    let lower = lower.then(|| ui.pressable_with_id(lower_id, LOWER_RECT, false));
    let output = ui.system_feedback(&scene);
    let frame = ui.finish_output();
    Run {
        captured_now,
        lower,
        output,
        frame,
    }
}

fn click(
    point: Point,
    config: SystemFeedbackSceneConfig,
    jobs: &JobList,
    diagnostics: &DiagnosticStrip,
    feedback: &FeedbackStack,
) -> Run {
    let mut memory = UiMemory::new();
    let _ = run_frame(
        config.clone(),
        jobs,
        diagnostics,
        feedback,
        &mut memory,
        primary_input(point, true, true, false),
        FRAME_NOW,
        false,
    );
    run_frame(
        config,
        jobs,
        diagnostics,
        feedback,
        &mut memory,
        primary_input(point, false, false, true),
        FRAME_NOW,
        false,
    )
}

fn prepared_scene<'a>(
    config: SystemFeedbackSceneConfig,
    jobs: &'a JobList,
    diagnostics: &'a DiagnosticStrip,
    feedback: &'a FeedbackStack,
) -> Result<SystemFeedbackScene<'a>, SystemFeedbackSceneError> {
    SystemFeedbackScene::prepare(config, jobs, diagnostics, feedback, FRAME_NOW)
}

#[test]
fn ui_preparation_captures_frame_time_and_nearest_feedback_deadline() {
    let jobs = JobList::from_rows([JobRow::new(job_id(1), "Render", JobPhase::Running)
        .with_progress(JobProgress::determinate(0.25))]);
    let diagnostics = DiagnosticStrip::new();
    let feedback = FeedbackStack::from_items([
        FeedbackItem::timed(
            feedback_id(1),
            FeedbackKind::Info,
            "Soon",
            "Expires first",
            Duration::from_secs(4),
            Duration::from_secs(3),
        ),
        FeedbackItem::timed(
            feedback_id(2),
            FeedbackKind::Warning,
            "Later",
            "Expires later",
            Duration::from_secs(4),
            Duration::from_secs(8),
        ),
    ]);
    let mut memory = UiMemory::new();

    let run = run_frame(
        config(),
        &jobs,
        &diagnostics,
        &feedback,
        &mut memory,
        UiInput::default(),
        FRAME_NOW,
        false,
    );

    assert_eq!(run.captured_now, FRAME_NOW);
    assert_eq!(
        run.output.repaint_request,
        RepaintRequest::After(Duration::from_secs(2))
    );
    assert_eq!(
        run.frame.repaint,
        RepaintRequest::After(Duration::from_secs(2))
    );
}

#[test]
fn indeterminate_active_work_is_continuous_while_determinate_and_terminal_work_are_idle() {
    let diagnostics = DiagnosticStrip::new();
    let pinned = FeedbackStack::from_items([FeedbackItem::pinned(
        feedback_id(1),
        FeedbackKind::Info,
        "Pinned",
        "No deadline",
    )]);
    let active = JobList::from_rows([JobRow::new(job_id(1), "Scanning", JobPhase::Running)]);
    let active_scene = prepared_scene(config(), &active, &diagnostics, &pinned).expect("scene");
    assert_eq!(active_scene.repaint_request(), RepaintRequest::Continuous);

    let idle = JobList::from_rows([
        JobRow::new(job_id(1), "Rendering", JobPhase::Running)
            .with_progress(JobProgress::determinate(0.75)),
        JobRow::new(job_id(2), "Done", JobPhase::Succeeded),
    ]);
    let idle_scene = prepared_scene(config(), &idle, &diagnostics, &pinned).expect("scene");
    assert_eq!(idle_scene.repaint_request(), RepaintRequest::None);
}

#[test]
fn every_action_click_emits_a_typed_request_and_the_same_frame_action() {
    let (jobs, diagnostics, feedback) = actionable_models();
    let cases = [
        (
            Point::new(208.0, 16.0),
            ActionId::new("job.cancel"),
            SystemFeedbackTarget::JobCancel(job_id(7)),
        ),
        (
            Point::new(208.0, 56.0),
            ActionId::new("diagnostic.dismiss"),
            SystemFeedbackTarget::DiagnosticAction {
                diagnostic_id: diagnostic_id(7),
                kind: DiagnosticActionKind::Dismiss,
            },
        ),
        (
            Point::new(140.0, 56.0),
            ActionId::new("diagnostic.report"),
            SystemFeedbackTarget::DiagnosticAction {
                diagnostic_id: diagnostic_id(7),
                kind: DiagnosticActionKind::Report,
            },
        ),
        (
            Point::new(208.0, 96.0),
            ActionId::new("feedback.open"),
            SystemFeedbackTarget::FeedbackAction(feedback_id(7)),
        ),
        (
            Point::new(140.0, 96.0),
            ActionId::new("feedback.dismiss"),
            SystemFeedbackTarget::FeedbackDismiss(feedback_id(7)),
        ),
    ];

    for (point, expected_action, expected_target) in cases {
        let mut run = click(point, actionable_config(), &jobs, &diagnostics, &feedback);
        assert_eq!(run.output.requests.len(), 1, "click at {point:?}");
        let request = &run.output.requests[0];
        assert_eq!(request.invocation().action_id, expected_action);
        assert_eq!(request.invocation().source, ActionSource::Button);
        assert!(
            run.output.responses.iter().any(|response| {
                response.target == expected_target && response.response.clicked
            })
        );
        assert_eq!(run.frame.actions.len(), 1);
        assert_eq!(
            run.frame.actions.pop_front(),
            Some(request.invocation().clone())
        );

        match (expected_target, request) {
            (SystemFeedbackTarget::JobCancel(id), SystemFeedbackRequest::JobCancel(actual)) => {
                assert_eq!(actual.job_id, id);
            }
            (
                SystemFeedbackTarget::DiagnosticAction {
                    diagnostic_id,
                    kind,
                },
                SystemFeedbackRequest::Diagnostic(actual),
            ) => {
                assert_eq!(actual.diagnostic_id, diagnostic_id);
                assert_eq!(actual.kind, kind);
            }
            (
                SystemFeedbackTarget::FeedbackAction(id),
                SystemFeedbackRequest::FeedbackAction(actual),
            ) => assert_eq!(actual.feedback_id, id),
            (
                SystemFeedbackTarget::FeedbackDismiss(id),
                SystemFeedbackRequest::FeedbackDismiss(actual),
            ) => assert_eq!(actual.feedback_id, id),
            _ => panic!("request kind did not match target {expected_target:?}"),
        }
    }
}

#[test]
fn completed_job_remains_passive_until_application_removes_it() {
    let completed =
        JobList::from_rows([
            JobRow::new(job_id(3), "Export complete", JobPhase::Succeeded)
                .with_progress(JobProgress::determinate(1.0))
                .with_cancel(JobCancel::new(
                    action("job.cancel.stale", "Cancel"),
                    ActionContext::Global,
                )),
        ]);
    let diagnostics = DiagnosticStrip::new();
    let feedback = FeedbackStack::new();
    let scene = prepared_scene(config(), &completed, &diagnostics, &feedback).expect("scene");
    let row_id = scene.target_widget_id(SystemFeedbackTarget::Job(job_id(3)));
    let cancel_id = scene.target_widget_id(SystemFeedbackTarget::JobCancel(job_id(3)));
    let mut memory = UiMemory::new();
    let present = run_frame(
        config(),
        &completed,
        &diagnostics,
        &feedback,
        &mut memory,
        UiInput::default(),
        FRAME_NOW,
        false,
    );

    assert!(present.frame.semantics.get(row_id).is_some());
    assert!(present.frame.semantics.get(cancel_id).is_none());
    assert!(present.output.responses.is_empty());

    let removed = JobList::new();
    let mut memory = UiMemory::new();
    let absent = run_frame(
        config(),
        &removed,
        &diagnostics,
        &feedback,
        &mut memory,
        UiInput::default(),
        FRAME_NOW,
        false,
    );
    assert!(absent.frame.semantics.get(row_id).is_none());
}

#[test]
fn application_removal_drops_diagnostics_and_feedback_from_the_scene() {
    let jobs = JobList::new();
    let diagnostics =
        DiagnosticStrip::from_items([diagnostic(4, DiagnosticStripSeverity::Warning)]);
    let feedback = FeedbackStack::from_items([FeedbackItem::pinned(
        feedback_id(4),
        FeedbackKind::Warning,
        "Offline",
        "Connection lost",
    )]);
    let scene = prepared_scene(config(), &jobs, &diagnostics, &feedback).expect("scene");
    let diagnostic_widget =
        scene.target_widget_id(SystemFeedbackTarget::Diagnostic(diagnostic_id(4)));
    let feedback_widget = scene.target_widget_id(SystemFeedbackTarget::Feedback(feedback_id(4)));

    let mut memory = UiMemory::new();
    let present = run_frame(
        config(),
        &jobs,
        &diagnostics,
        &feedback,
        &mut memory,
        UiInput::default(),
        FRAME_NOW,
        false,
    );
    assert!(present.frame.semantics.get(diagnostic_widget).is_some());
    assert!(present.frame.semantics.get(feedback_widget).is_some());

    let mut memory = UiMemory::new();
    let absent = run_frame(
        config(),
        &jobs,
        &DiagnosticStrip::new(),
        &FeedbackStack::new(),
        &mut memory,
        UiInput::default(),
        FRAME_NOW,
        false,
    );
    assert!(absent.frame.semantics.get(diagnostic_widget).is_none());
    assert!(absent.frame.semantics.get(feedback_widget).is_none());
}

#[test]
fn duplicate_model_ids_fail_preparation_with_typed_errors() {
    let empty_jobs = JobList::new();
    let empty_diagnostics = DiagnosticStrip::new();
    let empty_feedback = FeedbackStack::new();

    let duplicate_jobs = JobList::from_rows([
        JobRow::new(job_id(9), "First", JobPhase::Queued),
        JobRow::new(job_id(9), "Second", JobPhase::Running),
    ]);
    assert_eq!(
        prepared_scene(
            config(),
            &duplicate_jobs,
            &empty_diagnostics,
            &empty_feedback
        )
        .expect_err("duplicate job ID"),
        SystemFeedbackSceneError::DuplicateJobId(job_id(9))
    );

    let duplicate_diagnostics = DiagnosticStrip::from_items([
        diagnostic(9, DiagnosticStripSeverity::Error),
        diagnostic(9, DiagnosticStripSeverity::Info),
    ]);
    assert_eq!(
        prepared_scene(
            config(),
            &empty_jobs,
            &duplicate_diagnostics,
            &empty_feedback
        )
        .expect_err("duplicate diagnostic ID"),
        SystemFeedbackSceneError::DuplicateDiagnosticId(diagnostic_id(9))
    );

    let duplicate_feedback = FeedbackStack::from_items([
        FeedbackItem::pinned(feedback_id(9), FeedbackKind::Info, "First", "First"),
        FeedbackItem::pinned(feedback_id(9), FeedbackKind::Error, "Second", "Second"),
    ]);
    assert_eq!(
        prepared_scene(
            config(),
            &empty_jobs,
            &empty_diagnostics,
            &duplicate_feedback
        )
        .expect_err("duplicate feedback ID"),
        SystemFeedbackSceneError::DuplicateFeedbackId(feedback_id(9))
    );
}

#[test]
fn target_ids_survive_reorder_and_categories_namespace_equal_raw_ids() {
    let first_jobs = JobList::from_rows([
        JobRow::new(job_id(1), "One", JobPhase::Queued),
        JobRow::new(job_id(2), "Two", JobPhase::Running),
    ]);
    let reordered_jobs = JobList::from_rows([
        JobRow::new(job_id(2), "Two", JobPhase::Running),
        JobRow::new(job_id(1), "One", JobPhase::Queued),
    ]);
    let diagnostics = DiagnosticStrip::from_items([diagnostic(1, DiagnosticStripSeverity::Info)]);
    let feedback = FeedbackStack::from_items([FeedbackItem::pinned(
        feedback_id(1),
        FeedbackKind::Info,
        "Info",
        "Same raw identity",
    )]);
    let first = prepared_scene(config(), &first_jobs, &diagnostics, &feedback).expect("scene");
    let reordered =
        prepared_scene(config(), &reordered_jobs, &diagnostics, &feedback).expect("scene");

    for id in [job_id(1), job_id(2)] {
        assert_eq!(
            first.target_widget_id(SystemFeedbackTarget::Job(id)),
            reordered.target_widget_id(SystemFeedbackTarget::Job(id))
        );
    }
    let job = first.target_widget_id(SystemFeedbackTarget::Job(job_id(1)));
    let diagnostic = first.target_widget_id(SystemFeedbackTarget::Diagnostic(diagnostic_id(1)));
    let feedback = first.target_widget_id(SystemFeedbackTarget::Feedback(feedback_id(1)));
    assert_ne!(job, diagnostic);
    assert_ne!(job, feedback);
    assert_ne!(diagnostic, feedback);
    assert_ne!(
        first.surface_widget_id(SystemFeedbackSurface::Jobs),
        first.surface_widget_id(SystemFeedbackSurface::Diagnostics)
    );
}

#[test]
fn hidden_actions_are_absent_and_disabled_actions_fail_closed() {
    let mut hidden_cancel = action("job.cancel.hidden", "Hidden cancel");
    hidden_cancel.state.visible = false;
    let jobs = JobList::from_rows([JobRow::new(job_id(2), "Scan", JobPhase::Running)
        .with_progress(JobProgress::determinate(0.2))
        .with_cancel(JobCancel::new(hidden_cancel, ActionContext::Global))]);
    let diagnostics = DiagnosticStrip::from_items([diagnostic(2, DiagnosticStripSeverity::Error)]);
    let feedback = FeedbackStack::new();
    let mut disabled_report = action("diagnostic.report.disabled", "Report");
    disabled_report.state.enabled = false;
    let inert_config =
        config().with_diagnostic_actions([DiagnosticActionSet::new(diagnostic_id(2)).with_report(
            DiagnosticAction::new(disabled_report, ActionContext::Editor),
        )]);
    let scene =
        prepared_scene(inert_config.clone(), &jobs, &diagnostics, &feedback).expect("scene");
    let hidden_id = scene.target_widget_id(SystemFeedbackTarget::JobCancel(job_id(2)));
    let disabled_id = scene.target_widget_id(SystemFeedbackTarget::DiagnosticAction {
        diagnostic_id: diagnostic_id(2),
        kind: DiagnosticActionKind::Report,
    });

    let run = click(
        Point::new(208.0, 56.0),
        inert_config,
        &jobs,
        &diagnostics,
        &feedback,
    );

    assert!(run.output.requests.is_empty());
    assert!(run.frame.actions.is_empty());
    assert!(run.frame.semantics.get(hidden_id).is_none());
    let disabled = run
        .frame
        .semantics
        .get(disabled_id)
        .expect("visible disabled diagnostic action");
    assert!(disabled.state.disabled);
    assert!(!disabled.focusable);
    assert!(disabled.actions.is_empty());
}

#[test]
fn paint_is_clipped_and_semantics_expose_job_progress_and_actions() {
    let (jobs, diagnostics, feedback) = actionable_models();
    let scene = prepared_scene(actionable_config(), &jobs, &diagnostics, &feedback).expect("scene");
    let job_widget = scene.target_widget_id(SystemFeedbackTarget::Job(job_id(7)));
    let dismiss_widget = scene.target_widget_id(SystemFeedbackTarget::DiagnosticAction {
        diagnostic_id: diagnostic_id(7),
        kind: DiagnosticActionKind::Dismiss,
    });
    let mut memory = UiMemory::new();
    let run = run_frame(
        actionable_config(),
        &jobs,
        &diagnostics,
        &feedback,
        &mut memory,
        UiInput::default(),
        FRAME_NOW,
        false,
    );

    for (surface, rect) in [
        (SystemFeedbackSurface::Jobs, JOBS_RECT),
        (SystemFeedbackSurface::Diagnostics, DIAGNOSTICS_RECT),
        (SystemFeedbackSurface::Feedback, FEEDBACK_RECT),
    ] {
        let surface_id = scene.surface_widget_id(surface);
        let node = run
            .frame
            .semantics
            .get(surface_id)
            .expect("surface semantics");
        assert_eq!(node.role, SemanticRole::List);
        assert!(run.frame.primitives.iter().any(
            |primitive| matches!(primitive, Primitive::ClipBegin { rect: actual, .. } if *actual == rect)
        ));
    }

    let job = run.frame.semantics.get(job_widget).expect("job semantics");
    assert_eq!(
        job.state.value,
        Some(SemanticValue::Number {
            current: 0.5,
            min: 0.0,
            max: 1.0,
        })
    );
    let dismiss = run
        .frame
        .semantics
        .get(dismiss_widget)
        .expect("dismiss semantics");
    assert_eq!(dismiss.role, SemanticRole::Button);
    assert!(
        dismiss
            .actions
            .iter()
            .any(|action| action.kind == SemanticActionKind::Dismiss)
    );
    assert!(run.frame.primitives.iter().any(|primitive| {
        matches!(primitive, Primitive::Rect(rect) if rect.rect == Rect::new(0.0, 29.0, 120.0, 3.0))
    }));
}

#[test]
fn feedback_surfaces_block_pointer_input_from_lower_content() {
    let jobs = JobList::from_rows([JobRow::new(job_id(1), "Done", JobPhase::Succeeded)]);
    let diagnostics = DiagnosticStrip::new();
    let feedback = FeedbackStack::new();
    let mut memory = UiMemory::new();
    let pressed = run_frame(
        config(),
        &jobs,
        &diagnostics,
        &feedback,
        &mut memory,
        primary_input(Point::new(20.0, 16.0), true, true, false),
        FRAME_NOW,
        true,
    );
    assert!(!pressed.lower.expect("lower response").state.hovered);

    let released = run_frame(
        config(),
        &jobs,
        &diagnostics,
        &feedback,
        &mut memory,
        primary_input(Point::new(20.0, 16.0), false, false, true),
        FRAME_NOW,
        true,
    );
    assert!(!released.lower.expect("lower response").clicked);
    assert!(released.output.requests.is_empty());
}
