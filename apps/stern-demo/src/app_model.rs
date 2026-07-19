use std::fmt::Write as _;

use stern::core::{
    ActionBinding, ActionContext, ActionDescriptor, ActionInvocation, ActionPriority, ActionRouter,
    Color, Key, Modifiers, Shortcut,
};
use stern::widgets::gradient_editor::{
    GradientEditorIntent, GradientEditorStop, GradientEditorStopId,
};
use stern_icons_phosphor as phosphor;

const EDIT_ACTION: &str = "workspace.edit";
const GRAPH_ACTION: &str = "workspace.graph";
const APPLY_ACTION: &str = "shared.apply";
const VIEWPORT_SELECT_ACTION: &str = "viewport.tool.select";
const VIEWPORT_TRANSFORM_ACTION: &str = "viewport.tool.transform";
const SAVE_COLOR_STYLE_ACTION: &str = "color-style.save";

const PRIMARY_STOP: GradientEditorStopId = GradientEditorStopId::from_raw(1);
const SECONDARY_STOP: GradientEditorStopId = GradientEditorStopId::from_raw(2);

/// Color value paired with the color space required for serialization.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DemoTaggedColor {
    /// Color channels are encoded in the sRGB color space.
    Srgb(Color),
}

impl DemoTaggedColor {
    /// Returns the Stern color value carried by this tag.
    #[must_use]
    pub const fn color(self) -> Color {
        match self {
            Self::Srgb(color) => color,
        }
    }
}

/// Outcome of the latest application-owned color-style save attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DemoColorSaveState {
    /// No save has been attempted yet.
    Idle,
    /// The last save failed without changing the serialized value.
    Failed,
    /// The latest color and gradient were serialized successfully.
    Succeeded,
}

/// One-shot application request projected through Stern's public overlay scene.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DemoColorOverlayNotice {
    SaveFailed,
    SaveRecovered,
}

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
#[derive(Debug, Clone, PartialEq)]
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
    tagged_color: DemoTaggedColor,
    color_revision: u32,
    gradient_stops: Vec<GradientEditorStop>,
    selected_gradient_stop: GradientEditorStopId,
    color_save_state: DemoColorSaveState,
    fail_next_color_save: bool,
    serialized_color_style: Option<String>,
    color_overlay_notice: Option<DemoColorOverlayNotice>,
}

impl DemoApplicationModel {
    /// Creates the deterministic initial application state.
    #[must_use]
    pub fn new() -> Self {
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
            tagged_color: DemoTaggedColor::Srgb(Color::rgb8(58, 137, 246)),
            color_revision: 0,
            gradient_stops: vec![
                GradientEditorStop::new(PRIMARY_STOP, 0.2, Color::rgb8(58, 137, 246)),
                GradientEditorStop::new(SECONDARY_STOP, 0.8, Color::rgb8(229, 108, 238)),
            ],
            selected_gradient_stop: PRIMARY_STOP,
            color_save_state: DemoColorSaveState::Idle,
            fail_next_color_save: true,
            serialized_color_style: None,
            color_overlay_notice: None,
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

    /// Returns the application-owned color with its explicit color-space tag.
    #[must_use]
    pub const fn tagged_color(&self) -> DemoTaggedColor {
        self.tagged_color
    }

    /// Returns the number of committed picker color changes.
    #[must_use]
    pub const fn color_revision(&self) -> u32 {
        self.color_revision
    }

    /// Returns the application-owned gradient stops in presentation order.
    #[must_use]
    pub fn gradient_stops(&self) -> &[GradientEditorStop] {
        &self.gradient_stops
    }

    /// Returns the stable selected gradient stop identity.
    #[must_use]
    pub const fn selected_gradient_stop(&self) -> GradientEditorStopId {
        self.selected_gradient_stop
    }

    /// Returns the latest save outcome.
    #[must_use]
    pub const fn color_save_state(&self) -> DemoColorSaveState {
        self.color_save_state
    }

    /// Returns the last successfully serialized explicit-sRGB value.
    #[must_use]
    pub fn serialized_color_style(&self) -> Option<&str> {
        self.serialized_color_style.as_deref()
    }

    pub(crate) fn take_color_overlay_notice(&mut self) -> Option<DemoColorOverlayNotice> {
        self.color_overlay_notice.take()
    }

    /// Replaces the deterministic job presentation state.
    pub fn set_job(&mut self, phase: DemoJobPhase, progress_percent: u8) {
        self.job_phase = phase;
        self.job_progress_percent = progress_percent.min(100);
    }

    /// Commits one picker result as an explicitly tagged sRGB color.
    pub fn commit_color(&mut self, color: Color) {
        let next = DemoTaggedColor::Srgb(color);
        if self.tagged_color != next {
            self.tagged_color = next;
            self.color_revision = self.color_revision.saturating_add(1);
            self.color_save_state = DemoColorSaveState::Idle;
        }
    }

    /// Applies public gradient-editor intents to stable application-owned stop IDs.
    pub fn apply_gradient_intents(&mut self, intents: &[GradientEditorIntent]) {
        for intent in intents {
            match *intent {
                GradientEditorIntent::SelectStop(id)
                    if self.gradient_stops.iter().any(|stop| stop.id == id) =>
                {
                    self.selected_gradient_stop = id;
                }
                GradientEditorIntent::MoveStop { id, position } => {
                    if let Some(stop) = self.gradient_stops.iter_mut().find(|stop| stop.id == id) {
                        stop.position = position.clamp(0.0, 1.0);
                    }
                }
                GradientEditorIntent::RemoveStop(id) if self.gradient_stops.len() > 2 => {
                    self.gradient_stops.retain(|stop| stop.id != id);
                    if !self
                        .gradient_stops
                        .iter()
                        .any(|stop| stop.id == self.selected_gradient_stop)
                    {
                        self.selected_gradient_stop = self.gradient_stops[0].id;
                    }
                }
                GradientEditorIntent::Reverse => {
                    for stop in &mut self.gradient_stops {
                        stop.position = 1.0 - stop.position;
                    }
                    self.gradient_stops
                        .sort_by(|left, right| left.position.total_cmp(&right.position));
                }
                GradientEditorIntent::SelectStop(_) | GradientEditorIntent::RemoveStop(_) => {}
            }
        }
        if !intents.is_empty() {
            self.color_save_state = DemoColorSaveState::Idle;
        }
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
            SAVE_COLOR_STYLE_ACTION => self.save_color_style(),
            _ => return false,
        }
        true
    }

    fn save_color_style(&mut self) {
        if self.fail_next_color_save {
            self.fail_next_color_save = false;
            self.color_save_state = DemoColorSaveState::Failed;
            self.color_overlay_notice = Some(DemoColorOverlayNotice::SaveFailed);
            return;
        }
        self.serialized_color_style = Some(serialize_color_style(
            self.tagged_color,
            &self.gradient_stops,
        ));
        self.color_save_state = DemoColorSaveState::Succeeded;
        self.color_overlay_notice = Some(DemoColorOverlayNotice::SaveRecovered);
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
    descriptors: [ActionDescriptor; 4],
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
                ActionDescriptor::new(SAVE_COLOR_STYLE_ACTION, "Save Color Style")
                    .with_icon(phosphor::regular::FLOPPY_DISK),
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

    /// Returns the application-owned color-style save action descriptor.
    #[must_use]
    pub const fn save_color_style(&self) -> &ActionDescriptor {
        &self.descriptors[3]
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

fn serialize_color_style(color: DemoTaggedColor, stops: &[GradientEditorStop]) -> String {
    let DemoTaggedColor::Srgb(color) = color;
    let mut serialized = format!(
        "color=srgb({:.3},{:.3},{:.3},{:.3});gradient={}",
        color.r, color.g, color.b, color.a, "sRGB"
    );
    for stop in stops {
        write!(
            &mut serialized,
            ";{}@{:.3}=srgb({:.3},{:.3},{:.3},{:.3})",
            stop.id.raw(),
            stop.position,
            stop.color.r,
            stop.color.g,
            stop.color.b,
            stop.color.a,
        )
        .expect("writing to a String cannot fail");
    }
    serialized
}
