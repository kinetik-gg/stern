//! Public-consumer baseline for the Stern integration demo.

mod app_model;
mod edit_workspace;
mod graph_workspace;
mod overlay_workspace;
mod timeline_workspace;

use stern::UiState;
use stern::core::{
    ActionInvocation, ActionRoutingContext, FrameContext, FrameOutput, PhysicalSize,
    PlatformRequest, Rect, ScaleFactor, SemanticRole, Size, TimeInfo, UiInput, ViewportInfo,
    WidgetId, default_dark_theme,
};
use stern::render::RenderResources;
use stern::widgets::Ui;

pub use edit_workspace::DemoSelectedAssetSnapshot;
use edit_workspace::EditWorkspace;
use overlay_workspace::SharedOverlayRoute;

pub use app_model::{
    DemoActionAvailability, DemoActionRegistry, DemoApplicationModel, DemoColorSaveState,
    DemoJobPhase, DemoScenario, DemoTaggedColor, DemoTaggedColorStyle, DemoTimelineKeyframe,
    DemoTimelinePosition, DemoTimelineState, DemoTransportState, DemoViewportTool, DemoWorkspace,
};
pub use graph_workspace::{GraphConnectionFeedback, GraphWorkspaceState};

/// Canonical integration-demo title.
pub const DEMO_TITLE: &str = "Stern Integration Demo";

/// Application-owned state composed exclusively through the public `stern` facade.
pub struct DemoApp {
    ui_state: UiState,
    model: DemoApplicationModel,
    actions: DemoActionRegistry,
    edit_workspace: EditWorkspace,
    graph_workspace: GraphWorkspaceState,
    overlays: SharedOverlayRoute,
}

impl DemoApp {
    /// Creates the deterministic baseline fixture.
    #[must_use]
    pub fn new() -> Self {
        Self::for_scenario(DemoScenario::Default)
    }

    /// Creates the demo with explicit production journey diagnostics enabled.
    #[must_use]
    pub fn for_scenario(scenario: DemoScenario) -> Self {
        let model = DemoApplicationModel::for_scenario(scenario);
        let edit_workspace = EditWorkspace::new(&model);
        Self {
            ui_state: UiState::new(),
            model,
            actions: DemoActionRegistry::for_scenario(scenario),
            edit_workspace,
            graph_workspace: GraphWorkspaceState::for_scenario(scenario),
            overlays: SharedOverlayRoute::new(),
        }
    }

    /// Returns the explicit application scenario.
    #[must_use]
    pub const fn scenario(&self) -> DemoScenario {
        self.model.scenario()
    }

    /// Returns read-only application-owned timeline state.
    #[must_use]
    pub const fn timeline(&self) -> &DemoTimelineState {
        self.model.timeline()
    }

    /// Returns the exact shared frame/time projection.
    #[must_use]
    pub fn timeline_position(&self) -> DemoTimelinePosition {
        self.model.timeline().position()
    }

    /// Returns the application-owned playback state.
    #[must_use]
    pub const fn transport_state(&self) -> DemoTransportState {
        self.model.transport_state()
    }

    /// Returns the active application workspace.
    #[must_use]
    pub const fn workspace(&self) -> DemoWorkspace {
        self.model.workspace()
    }

    /// Returns the application-owned shared revision.
    #[must_use]
    pub const fn applied_revision(&self) -> u32 {
        self.model.applied_revision()
    }

    /// Returns the application-owned explicitly tagged color.
    #[must_use]
    pub const fn tagged_color(&self) -> DemoTaggedColor {
        self.model.tagged_color()
    }

    /// Returns the unified application-owned tagged color style.
    #[must_use]
    pub const fn color_style(&self) -> &DemoTaggedColorStyle {
        self.model.color_style()
    }

    /// Returns the number of committed color-picker changes.
    #[must_use]
    pub const fn color_revision(&self) -> u32 {
        self.model.color_revision()
    }

    /// Returns stable application-owned gradient stops.
    #[must_use]
    pub fn gradient_stops(&self) -> &[stern::widgets::gradient_editor::GradientEditorStop] {
        self.model.gradient_stops()
    }

    /// Returns the stable selected gradient stop identity.
    #[must_use]
    pub const fn selected_gradient_stop(
        &self,
    ) -> stern::widgets::gradient_editor::GradientEditorStopId {
        self.model.selected_gradient_stop()
    }

    /// Returns the explicit application-owned gradient interpolation space.
    #[must_use]
    pub const fn gradient_interpolation(
        &self,
    ) -> stern::widgets::gradient_editor::GradientInterpolationSpace {
        self.model.gradient_interpolation()
    }

    /// Returns the latest application-owned color-style save outcome.
    #[must_use]
    pub const fn color_save_state(&self) -> DemoColorSaveState {
        self.model.color_save_state()
    }

    /// Returns the last successful explicit-sRGB serialization.
    #[must_use]
    pub fn serialized_color_style(&self) -> Option<&str> {
        self.model.serialized_color_style()
    }

    /// Returns the application-owned projected playhead frame.
    #[must_use]
    pub const fn playhead_frame(&self) -> i64 {
        self.model.playhead_frame()
    }

    /// Returns the committed application-owned playhead frame.
    #[must_use]
    pub const fn committed_playhead_frame(&self) -> i64 {
        self.model.committed_playhead_frame()
    }

    /// Returns the application-owned projected clip range.
    #[must_use]
    pub const fn clip_frames(&self) -> (i64, i64) {
        self.model.clip_frames()
    }

    /// Returns the committed application-owned clip range.
    #[must_use]
    pub const fn committed_clip_frames(&self) -> (i64, i64) {
        self.model.committed_clip_frames()
    }

    /// Returns the active application-owned viewport tool.
    #[must_use]
    pub const fn viewport_tool(&self) -> DemoViewportTool {
        self.model.viewport_tool()
    }

    /// Replaces the deterministic preview-job state used by public feedback surfaces.
    pub fn set_job(&mut self, phase: DemoJobPhase, progress_percent: u8) {
        self.model.set_job(phase, progress_percent);
    }

    /// Enables or disables the shared action across every public projection.
    pub const fn set_apply_enabled(&mut self, enabled: bool) {
        self.model.set_apply_availability(if enabled {
            DemoActionAvailability::Available
        } else {
            DemoActionAvailability::Unavailable
        });
    }

    /// Replaces the shared action availability across every public projection.
    pub const fn set_apply_availability(&mut self, availability: DemoActionAvailability) {
        self.model.set_apply_availability(availability);
    }

    /// Returns the application-owned Graph workspace state.
    #[must_use]
    pub const fn graph_workspace(&self) -> &GraphWorkspaceState {
        &self.graph_workspace
    }

    /// Returns a read-only view over the selected canonical asset record.
    #[must_use]
    pub fn selected_asset(&self) -> Option<DemoSelectedAssetSnapshot<'_>> {
        self.edit_workspace.selected_asset()
    }

    /// Builds and dispatches one frame through public toolkit APIs.
    pub fn frame(&mut self, context: FrameContext) -> FrameOutput {
        let theme = default_dark_theme();
        let Self {
            ui_state,
            model,
            actions,
            edit_workspace,
            graph_workspace,
            overlays,
        } = self;
        let focus_return;
        let output = {
            let mut ui = ui_state.begin_frame(context, &theme);
            focus_return = compose_demo(
                &mut ui,
                actions,
                model,
                edit_workspace,
                graph_workspace,
                overlays,
            );
            ui.finish_output()
        };
        if let Some(focus_return) = focus_return {
            self.ui_state.memory_mut().focus(focus_return);
        }
        let mut actions = output.actions.clone();
        for invocation in actions.drain() {
            if let Some(target) = self.apply_action(&invocation) {
                self.ui_state.memory_mut().focus(target);
            }
        }
        output
    }

    /// Composes one demo frame into a caller-owned widget frame.
    ///
    /// This is the application-shell entry point: the caller owns the frame
    /// lifecycle and retained UI state, and must apply the returned
    /// retained-focus request after the frame is finished. Emitted action
    /// invocations must then be executed through [`Self::apply_action`].
    pub fn compose(&mut self, ui: &mut Ui<'_>) -> Option<WidgetId> {
        compose_demo(
            ui,
            &mut self.actions,
            &mut self.model,
            &mut self.edit_workspace,
            &mut self.graph_workspace,
            &mut self.overlays,
        )
    }

    /// Executes one emitted action invocation against application state.
    ///
    /// Returns a retained-focus request the caller must apply after the
    /// frame that emitted the invocation.
    pub fn apply_action(&mut self, invocation: &ActionInvocation) -> Option<WidgetId> {
        let target = (invocation.action_id.as_str() == self.actions.edit_workspace().id.as_str()
            || invocation.action_id.as_str() == self.actions.graph_workspace().id.as_str())
        .then(|| WidgetId::from_key("root").child(invocation.action_id.as_str()));
        if !self.graph_workspace.handle_action(invocation) {
            let _ = self.model.execute(invocation);
        }
        target
    }

    /// Registers demo-owned domain resources such as asset textures.
    ///
    /// Application shells that own text-layout caching register shaped text
    /// themselves and add only these domain resources.
    pub fn register_domain_resources(&self, resources: &mut RenderResources) {
        self.edit_workspace.register_resources(resources);
    }

    /// Returns renderer resources for the latest public frame.
    #[must_use]
    pub fn render_resources(&self) -> RenderResources {
        let mut resources = self.ui_state.text_render_resources();
        self.register_domain_resources(&mut resources);
        resources
    }

    /// Returns the retained focused widget.
    #[must_use]
    pub fn focused(&self) -> Option<WidgetId> {
        self.ui_state.memory().focused()
    }
}

/// Composes the maintained demo frame and resolves application shortcuts.
fn compose_demo(
    ui: &mut Ui<'_>,
    actions: &mut DemoActionRegistry,
    model: &mut DemoApplicationModel,
    edit_workspace: &mut EditWorkspace,
    graph_workspace: &mut GraphWorkspaceState,
    overlays: &mut SharedOverlayRoute,
) -> Option<WidgetId> {
    let keyboard = ui.input().keyboard.clone();
    let logical_size = ui.viewport().logical_size;
    actions.project_apply_shared_state(model.apply_availability());
    actions.project_viewport_tool(model.viewport_tool());
    let workspace = model.workspace();
    let bounds = logical_size;
    actions.project_transport_state(model.transport_state());
    let shortcut_enabled = !overlays.is_open();
    ui.push_platform_request(PlatformRequest::SetWindowTitle(DEMO_TITLE.to_owned()));
    let focus_return = match workspace {
        DemoWorkspace::Edit => {
            edit_workspace.compose(ui, actions, workspace, model, overlays, bounds)
        }
        DemoWorkspace::Graph => {
            let graph_bounds = Rect::new(
                24.0,
                202.0,
                (logical_size.width - 48.0).max(0.0),
                (logical_size.height - 226.0).max(0.0),
            );
            graph_workspace.compose(ui, graph_bounds, bounds, actions, model, overlays)
        }
    };
    if shortcut_enabled {
        let routing = ActionRoutingContext::new().with_editor();
        let mut shortcuts = actions
            .shortcut_router()
            .resolve_shortcuts_in_context(&keyboard, routing);
        for invocation in shortcuts.drain() {
            ui.push_action(invocation);
        }
    }
    focus_return
}

impl Default for DemoApp {
    fn default() -> Self {
        Self::new()
    }
}

/// Creates a deterministic frame context for tests and evidence capture.
#[must_use]
pub fn demo_context(input: UiInput) -> FrameContext {
    let logical = Size::new(720.0, 480.0);
    FrameContext::new(
        ViewportInfo::new(logical, PhysicalSize::new(720, 480), ScaleFactor::ONE),
        input,
        TimeInfo::default(),
    )
}

/// Reports whether output contains real component semantics.
#[must_use]
pub fn has_component_semantics(output: &FrameOutput) -> bool {
    let has_button = output
        .semantics
        .nodes()
        .iter()
        .any(|node| matches!(node.role, SemanticRole::Button | SemanticRole::IconButton));
    let has_dock = output
        .semantics
        .nodes()
        .iter()
        .any(|node| node.role == SemanticRole::Dock);
    let has_collection = output
        .semantics
        .nodes()
        .iter()
        .any(|node| node.role == SemanticRole::List);
    let has_inspector = output
        .semantics
        .nodes()
        .iter()
        .any(|node| node.role == SemanticRole::Grid);
    has_button && has_dock && has_collection && has_inspector
}
