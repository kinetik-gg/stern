//! Public-consumer baseline for the Stern integration demo.

mod app_model;
mod graph_workspace;

use stern::UiState;
use stern::core::{
    ActionContext, ActionInvocation, FrameContext, FrameOutput, PhysicalSize, PlatformRequest,
    Rect, ScaleFactor, SemanticRole, Size, TimeInfo, UiInput, ViewportInfo, WidgetId,
    default_dark_theme,
};
use stern::render::RenderResources;
use stern::text::TextEditState;

pub use app_model::{DemoActionRegistry, DemoApplicationModel, DemoWorkspace};
pub use graph_workspace::GraphWorkspaceState;

/// Canonical integration-demo title.
pub const DEMO_TITLE: &str = "Stern Integration Demo";

/// Application-owned state composed exclusively through the public `stern` facade.
pub struct DemoApp {
    ui_state: UiState,
    model: DemoApplicationModel,
    actions: DemoActionRegistry,
    document_name: TextEditState,
    graph_workspace: GraphWorkspaceState,
}

impl DemoApp {
    /// Creates the deterministic baseline fixture.
    #[must_use]
    pub fn new() -> Self {
        Self {
            ui_state: UiState::new(),
            model: DemoApplicationModel::new(),
            actions: DemoActionRegistry::new(),
            document_name: TextEditState::new("Untitled Stern Document"),
            graph_workspace: GraphWorkspaceState::new(),
        }
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

    /// Returns the application-owned Graph workspace state.
    #[must_use]
    pub const fn graph_workspace(&self) -> &GraphWorkspaceState {
        &self.graph_workspace
    }

    /// Builds and dispatches one frame through public toolkit APIs.
    pub fn frame(&mut self, context: FrameContext) -> FrameOutput {
        let logical_size = context.viewport.logical_size;
        let edit = self.actions.edit_workspace().clone();
        let graph = self.actions.graph_workspace().clone();
        let apply = self.actions.apply_shared_state().clone();
        let workspace = self.model.workspace();
        let theme = default_dark_theme();
        let output = {
            let mut ui = self.ui_state.begin_frame(context, &theme);
            let edit_rect = Rect::new(24.0, 56.0, 112.0, 30.0);
            let graph_rect = Rect::new(148.0, 56.0, 120.0, 30.0);
            let apply_rect = Rect::new(24.0, 188.0, 160.0, 30.0);
            ui.push_platform_request(PlatformRequest::SetWindowTitle(DEMO_TITLE.to_owned()));
            ui.label(Rect::new(24.0, 20.0, 320.0, 24.0), DEMO_TITLE);
            if workspace == DemoWorkspace::Graph {
                let graph_bounds = Rect::new(
                    24.0,
                    230.0,
                    (logical_size.width - 48.0).max(0.0),
                    (logical_size.height - 254.0).max(0.0),
                );
                let app_targets = [
                    (ui.make_id(edit.id.as_str()), edit_rect),
                    (ui.make_id(graph.id.as_str()), graph_rect),
                    (ui.make_id(apply.id.as_str()), apply_rect),
                ];
                self.graph_workspace
                    .compose(&mut ui, graph_bounds, &app_targets);
            }
            let _ = ui.action_button(edit.id.as_str(), edit_rect, &edit, ActionContext::Global);
            let _ = ui.action_button(graph.id.as_str(), graph_rect, &graph, ActionContext::Global);
            match workspace {
                DemoWorkspace::Edit => {
                    ui.label(Rect::new(24.0, 108.0, 180.0, 20.0), "Document name");
                    let _ = ui.text_field(
                        "document.name",
                        Rect::new(24.0, 136.0, 300.0, 30.0),
                        &mut self.document_name,
                        false,
                    );
                }
                DemoWorkspace::Graph => {}
            }
            let _ = ui.action_button(apply.id.as_str(), apply_rect, &apply, ActionContext::Global);
            ui.finish_output()
        };
        let mut actions = output.actions.clone();
        for invocation in actions.drain() {
            self.dispatch(&invocation);
        }
        output
    }

    /// Returns renderer resources for the latest public frame.
    #[must_use]
    pub fn render_resources(&self) -> RenderResources {
        self.ui_state.text_render_resources()
    }

    /// Returns the retained focused widget.
    #[must_use]
    pub fn focused(&self) -> Option<WidgetId> {
        self.ui_state.memory().focused()
    }

    fn dispatch(&mut self, invocation: &ActionInvocation) {
        if !self.graph_workspace.handle_action(invocation) {
            let _ = self.model.execute(invocation);
        }
    }
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
        .any(|node| node.role == SemanticRole::Button);
    let has_text = output
        .semantics
        .nodes()
        .iter()
        .any(|node| node.role == SemanticRole::TextField);
    has_button && has_text
}
