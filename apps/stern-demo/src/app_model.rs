use stern::core::{
    ActionBinding, ActionContext, ActionDescriptor, ActionInvocation, ActionPriority, ActionRouter,
    Key, Modifiers, Shortcut,
};
use stern_icons_phosphor as phosphor;

const EDIT_ACTION: &str = "workspace.edit";
const GRAPH_ACTION: &str = "workspace.graph";
const APPLY_ACTION: &str = "shared.apply";
const VIEWPORT_SELECT_ACTION: &str = "viewport.tool.select";
const VIEWPORT_TRANSFORM_ACTION: &str = "viewport.tool.transform";

/// Application-owned viewport tool selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DemoViewportTool {
    /// Neutral selection and navigation tool.
    Select,
    /// Transform-handle tool.
    Transform,
}

/// Application-owned background job phase shown by Stern feedback surfaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DemoJobPhase {
    /// Work is active with deterministic progress.
    Running,
    /// Work completed successfully.
    Succeeded,
    /// Work completed with an error.
    Failed,
}

/// Stable identity of a maintained demo workspace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DemoWorkspace {
    /// Document editing workspace.
    Edit,
    /// Graph editing workspace.
    Graph,
}

impl DemoWorkspace {
    /// Returns the pinned workspace identity.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::Edit => "edit-workspace",
            Self::Graph => "graph-workspace",
        }
    }
}

/// Shared deterministic application model used by every demo workspace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DemoApplicationModel {
    workspace: DemoWorkspace,
    applied_revision: u32,
    committed_playhead_frame: i64,
    preview_playhead_frame: Option<i64>,
    committed_clip_frames: (i64, i64),
    preview_clip_frames: Option<(i64, i64)>,
    viewport_tool: DemoViewportTool,
    job_phase: DemoJobPhase,
    job_progress_percent: u8,
}

impl DemoApplicationModel {
    /// Creates the deterministic initial application state.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            workspace: DemoWorkspace::Edit,
            applied_revision: 0,
            committed_playhead_frame: 24,
            preview_playhead_frame: None,
            committed_clip_frames: (30, 90),
            preview_clip_frames: None,
            viewport_tool: DemoViewportTool::Select,
            job_phase: DemoJobPhase::Running,
            job_progress_percent: 40,
        }
    }

    /// Returns the active workspace.
    #[must_use]
    pub const fn workspace(&self) -> DemoWorkspace {
        self.workspace
    }

    /// Returns the shared applied revision.
    #[must_use]
    pub const fn applied_revision(&self) -> u32 {
        self.applied_revision
    }

    /// Returns the playhead frame currently projected to the timeline.
    #[must_use]
    pub const fn playhead_frame(&self) -> i64 {
        match self.preview_playhead_frame {
            Some(frame) => frame,
            None => self.committed_playhead_frame,
        }
    }

    /// Returns the committed playhead frame.
    #[must_use]
    pub const fn committed_playhead_frame(&self) -> i64 {
        self.committed_playhead_frame
    }

    /// Stages a playhead preview without committing application state.
    pub const fn preview_playhead(&mut self, frame: i64) {
        self.preview_playhead_frame = Some(frame);
    }

    /// Commits a playhead frame and closes its preview transaction.
    pub const fn commit_playhead(&mut self, frame: i64) {
        self.committed_playhead_frame = frame;
        self.preview_playhead_frame = None;
    }

    /// Cancels the current playhead preview.
    pub const fn cancel_playhead_preview(&mut self) {
        self.preview_playhead_frame = None;
    }

    /// Returns the clip range currently projected to the timeline.
    #[must_use]
    pub const fn clip_frames(&self) -> (i64, i64) {
        match self.preview_clip_frames {
            Some(range) => range,
            None => self.committed_clip_frames,
        }
    }

    /// Returns the committed clip range.
    #[must_use]
    pub const fn committed_clip_frames(&self) -> (i64, i64) {
        self.committed_clip_frames
    }

    /// Stages a validated clip preview without committing application state.
    pub const fn preview_clip(&mut self, start: i64, end: i64) {
        self.preview_clip_frames = Some((start, end));
    }

    /// Commits a validated clip range and closes its preview transaction.
    pub const fn commit_clip(&mut self, start: i64, end: i64) {
        self.committed_clip_frames = (start, end);
        self.preview_clip_frames = None;
    }

    /// Cancels the current clip preview.
    pub const fn cancel_clip_preview(&mut self) {
        self.preview_clip_frames = None;
    }

    /// Returns the active application-owned viewport tool.
    #[must_use]
    pub const fn viewport_tool(&self) -> DemoViewportTool {
        self.viewport_tool
    }

    /// Returns the application-owned background job phase.
    #[must_use]
    pub const fn job_phase(&self) -> DemoJobPhase {
        self.job_phase
    }

    /// Returns deterministic job progress in the inclusive `0..=100` range.
    #[must_use]
    pub const fn job_progress_percent(&self) -> u8 {
        self.job_progress_percent
    }

    /// Replaces the deterministic job presentation state.
    pub fn set_job(&mut self, phase: DemoJobPhase, progress_percent: u8) {
        self.job_phase = phase;
        self.job_progress_percent = progress_percent.min(100);
    }

    /// Executes one recognized application action.
    pub fn execute(&mut self, invocation: &ActionInvocation) -> bool {
        match invocation.action_id.as_str() {
            EDIT_ACTION => self.workspace = DemoWorkspace::Edit,
            GRAPH_ACTION => self.workspace = DemoWorkspace::Graph,
            APPLY_ACTION => {
                self.applied_revision = self.applied_revision.saturating_add(1);
            }
            VIEWPORT_SELECT_ACTION => self.viewport_tool = DemoViewportTool::Select,
            VIEWPORT_TRANSFORM_ACTION => self.viewport_tool = DemoViewportTool::Transform,
            _ => return false,
        }
        true
    }
}

impl Default for DemoApplicationModel {
    fn default() -> Self {
        Self::new()
    }
}

/// Single descriptor registry for the demo's existing application actions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DemoActionRegistry {
    descriptors: [ActionDescriptor; 3],
    viewport_tools: [ActionDescriptor; 2],
}

impl DemoActionRegistry {
    /// Creates the exact existing demo action set in stable order.
    #[must_use]
    pub fn new() -> Self {
        Self {
            descriptors: [
                ActionDescriptor::new(EDIT_ACTION, "Edit Workspace")
                    .with_icon(phosphor::regular::PENCIL_SIMPLE),
                ActionDescriptor::new(GRAPH_ACTION, "Graph Workspace")
                    .with_icon(phosphor::regular::GRAPH),
                apply_descriptor(),
            ],
            viewport_tools: [
                checkable_descriptor(
                    VIEWPORT_SELECT_ACTION,
                    "Select Tool",
                    phosphor::regular::CURSOR,
                    true,
                ),
                checkable_descriptor(
                    VIEWPORT_TRANSFORM_ACTION,
                    "Transform Tool",
                    phosphor::regular::ARROWS_OUT_CARDINAL,
                    false,
                ),
            ],
        }
    }

    /// Returns the Edit workspace action descriptor.
    #[must_use]
    pub const fn edit_workspace(&self) -> &ActionDescriptor {
        &self.descriptors[0]
    }

    /// Returns the Graph workspace action descriptor.
    #[must_use]
    pub const fn graph_workspace(&self) -> &ActionDescriptor {
        &self.descriptors[1]
    }

    /// Returns the shared-state apply action descriptor.
    #[must_use]
    pub const fn apply_shared_state(&self) -> &ActionDescriptor {
        &self.descriptors[2]
    }

    /// Enables or disables the shared action for every projected surface.
    pub const fn set_apply_shared_state_enabled(&mut self, enabled: bool) {
        self.descriptors[2].state.enabled = enabled;
    }

    /// Returns the select-tool action descriptor.
    #[must_use]
    pub const fn viewport_select(&self) -> &ActionDescriptor {
        &self.viewport_tools[0]
    }

    /// Returns the transform-tool action descriptor.
    #[must_use]
    pub const fn viewport_transform(&self) -> &ActionDescriptor {
        &self.viewport_tools[1]
    }

    /// Synchronizes checked tool presentation from application state.
    pub const fn project_viewport_tool(&mut self, active: DemoViewportTool) {
        self.viewport_tools[0].state.checked = Some(matches!(active, DemoViewportTool::Select));
        self.viewport_tools[1].state.checked = Some(matches!(active, DemoViewportTool::Transform));
    }

    /// Builds the application-owned shortcut router from the shared descriptors.
    #[must_use]
    pub fn shortcut_router(&self) -> ActionRouter {
        let mut router = ActionRouter::new();
        router.bind(ActionBinding::new(
            self.apply_shared_state().clone(),
            ActionContext::Editor,
            ActionPriority::Editor,
        ));
        router
    }

    /// Iterates over descriptors in stable registry order.
    #[must_use]
    pub fn iter(&self) -> impl ExactSizeIterator<Item = &ActionDescriptor> {
        self.descriptors.iter()
    }
}

impl Default for DemoActionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

fn apply_descriptor() -> ActionDescriptor {
    let mut descriptor = ActionDescriptor::new(APPLY_ACTION, "Apply Shared State")
        .with_icon(phosphor::regular::CHECK_CIRCLE);
    descriptor.shortcut = Some(Shortcut::new(
        Modifiers::new(false, true, false, false),
        Key::Enter,
    ));
    descriptor.keywords = vec!["apply".to_owned(), "shared state".to_owned()];
    descriptor
}

fn checkable_descriptor(
    id: &str,
    label: &str,
    icon: phosphor::PhosphorIcon,
    checked: bool,
) -> ActionDescriptor {
    let mut descriptor = ActionDescriptor::new(id, label).with_icon(icon);
    descriptor.state.checked = Some(checked);
    descriptor
}
