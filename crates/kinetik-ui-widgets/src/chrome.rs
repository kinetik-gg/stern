//! Data-only editor chrome contracts.

mod diagnostics;
mod feedback;
mod jobs;
mod menu_bar;
mod overflow;
mod scene;
mod status_bar;
mod system_feedback;
mod tab_strip;
mod toolbar;

pub use diagnostics::{
    DiagnosticField, DiagnosticFieldValue, DiagnosticSource, DiagnosticStrip, DiagnosticStripItem,
    DiagnosticStripItemId, DiagnosticStripSeverity, DiagnosticStripSummary,
};
pub use feedback::{
    FeedbackAction, FeedbackActionRequest, FeedbackDismiss, FeedbackDismissRequest, FeedbackId,
    FeedbackItem, FeedbackKind, FeedbackLifetime, FeedbackStack,
};
pub use jobs::{
    ActiveJobProgress, JobCancel, JobCancelRequest, JobList, JobPhase, JobProgress, JobRow,
    JobRowId, JobSummaryCounts,
};
pub use menu_bar::{MenuBar, MenuBarMenu, MenuBarMenuId, MenuBarMove, MenuBarOverlayRequest};
pub use overflow::{
    ChromeOverflowItem, ChromeOverflowPlacement, ChromeOverflowProjection, ChromeOverflowTrigger,
    project_chrome_overflow,
};
pub use scene::{
    ChromeOverflowRequest, ChromeScene, ChromeSceneConfig, ChromeSceneIntent, ChromeSceneItemKey,
    ChromeSceneOutput, ChromeSurfaceKind,
};
pub(crate) use scene::{ChromeSceneRow, ChromeSceneRowKind};
pub use status_bar::{StatusBar, StatusItem, StatusItemId, StatusItemKind, StatusProgress};
pub use system_feedback::{
    DiagnosticAction, DiagnosticActionKind, DiagnosticActionRequest, DiagnosticActionSet,
    SystemFeedbackOutput, SystemFeedbackRequest, SystemFeedbackResponse, SystemFeedbackScene,
    SystemFeedbackSceneConfig, SystemFeedbackSceneError, SystemFeedbackSurface,
    SystemFeedbackTarget,
};
#[allow(unused_imports)] // Re-exported for the sibling UI renderer module.
pub(crate) use system_feedback::{
    SystemFeedbackActionRow, SystemFeedbackRow, SystemFeedbackRowKind, SystemFeedbackSurfaceLayout,
};
pub use tab_strip::{TabStrip, TabStripMove, TabStripTarget};
pub use toolbar::{Toolbar, ToolbarGroup, ToolbarGroupId, ToolbarItem, ToolbarItemPresentation};
