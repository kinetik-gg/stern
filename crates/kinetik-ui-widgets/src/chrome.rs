//! Data-only editor chrome contracts.

mod diagnostics;
mod feedback;
mod jobs;
mod menu_bar;
mod status_bar;
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
pub use status_bar::{StatusBar, StatusItem, StatusItemId, StatusItemKind, StatusProgress};
pub use tab_strip::{TabStrip, TabStripMove, TabStripTarget};
pub use toolbar::{Toolbar, ToolbarGroup, ToolbarGroupId, ToolbarItem, ToolbarItemPresentation};
