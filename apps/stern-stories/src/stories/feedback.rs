//! System-feedback composition story: job rows and toast/status banners.

use stern::core::Rect;
use stern::widgets::Ui;
use stern::widgets::chrome::{
    DiagnosticStrip, FeedbackId, FeedbackItem, FeedbackKind, JobList, JobPhase, JobProgress,
    JobRow, JobRowId, SystemFeedbackSceneConfig,
};

use crate::story::{Story, StoryKind};

/// Feedback sheet: background-job rows and toast/status banners at the
/// visual-spec 07 heights (job rows 28, banner min-height 38). This is the
/// surface where AUDIT #941 defect 8 (hardcoded excess-height toast) lived.
#[must_use]
pub fn sheet() -> Story {
    Story {
        id: "feedback/sheet",
        title: "System feedback jobs and toasts",
        kind: StoryKind::Composition,
        compose,
    }
}

fn compose(ui: &mut Ui<'_>, rect: Rect) {
    ui.panel(rect);
    let bounds = rect.inset(16.0);
    ui.label(
        Rect::new(bounds.x, bounds.y, bounds.width, 16.0),
        "Background jobs",
    );
    // Two job rows at the spec's 28px row height and two toasts at the
    // spec's 38px banner min-height (visual-spec 07).
    let jobs_rect = Rect::new(bounds.x, bounds.y + 24.0, bounds.width, 56.0);
    ui.label(
        Rect::new(bounds.x, jobs_rect.max_y() + 12.0, bounds.width, 16.0),
        "Toasts",
    );
    let feedback_rect = Rect::new(bounds.x, jobs_rect.max_y() + 36.0, bounds.width, 76.0);

    let jobs = JobList::from_rows([
        JobRow::new(JobRowId::from_raw(1), "Preview render", JobPhase::Running)
            .with_progress(JobProgress::determinate(0.65)),
        JobRow::new(JobRowId::from_raw(2), "Asset import", JobPhase::Succeeded),
    ]);
    let diagnostics = DiagnosticStrip::default();
    let feedback = stern::widgets::chrome::FeedbackStack::from_items([
        FeedbackItem::pinned(
            FeedbackId::from_raw(1),
            FeedbackKind::Info,
            "Saved",
            "Scene saved to library",
        ),
        FeedbackItem::pinned(
            FeedbackId::from_raw(2),
            FeedbackKind::Error,
            "Export failed",
            "Target volume is read-only",
        ),
    ]);

    // Default row heights: jobs 28, toast/banner min-height 38 (spec 07).
    let scene = ui
        .prepare_system_feedback(
            SystemFeedbackSceneConfig::new(
                ui.make_id("story-feedback"),
                jobs_rect,
                Rect::ZERO,
                feedback_rect,
            ),
            &jobs,
            &diagnostics,
            &feedback,
        )
        .expect("deterministic story feedback scene is valid");
    let _ = ui.system_feedback(&scene);
}
