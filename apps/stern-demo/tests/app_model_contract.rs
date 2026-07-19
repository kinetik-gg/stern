//! Shared application model and action identity contract.

use stern::core::{ActionContext, ActionDescriptor, ActionInvocation, ActionSource};
use stern_demo::{
    DemoActionRegistry, DemoApplicationModel, DemoJobPhase, DemoViewportTool, DemoWorkspace,
};

#[test]
fn shared_model_preserves_pinned_workspace_identity_and_revision() {
    let registry = DemoActionRegistry::new();
    let mut model = DemoApplicationModel::new();

    assert_eq!(DemoWorkspace::Edit.id(), "edit-workspace");
    assert_eq!(DemoWorkspace::Graph.id(), "graph-workspace");
    assert_ne!(DemoWorkspace::Edit.id(), DemoWorkspace::Graph.id());
    assert_eq!(model.workspace(), DemoWorkspace::Edit);
    assert_eq!(model.applied_revision(), 0);
    assert_eq!(model.playhead_frame(), 24);
    assert_eq!(model.clip_frames(), (30, 90));
    assert_eq!(model.viewport_tool(), DemoViewportTool::Select);
    assert_eq!(model.job_phase(), DemoJobPhase::Running);
    assert_eq!(model.job_progress_percent(), 40);

    assert!(model.execute(&invocation(registry.graph_workspace())));
    assert_eq!(model.workspace(), DemoWorkspace::Graph);
    assert!(model.execute(&invocation(registry.apply_shared_state())));
    assert_eq!(model.applied_revision(), 1);
    assert!(model.execute(&invocation(registry.edit_workspace())));
    assert_eq!(model.workspace(), DemoWorkspace::Edit);
    assert_eq!(model.applied_revision(), 1);
}

#[test]
fn timeline_preview_commit_cancel_and_feedback_state_remain_application_owned() {
    let registry = DemoActionRegistry::new();
    let mut model = DemoApplicationModel::new();

    model.preview_playhead(48);
    assert_eq!(model.playhead_frame(), 48);
    assert_eq!(model.committed_playhead_frame(), 24);
    model.cancel_playhead_preview();
    assert_eq!(model.playhead_frame(), 24);
    model.commit_playhead(72);
    assert_eq!(model.committed_playhead_frame(), 72);

    model.preview_clip(45, 105);
    assert_eq!(model.clip_frames(), (45, 105));
    assert_eq!(model.committed_clip_frames(), (30, 90));
    model.cancel_clip_preview();
    assert_eq!(model.clip_frames(), (30, 90));
    model.commit_clip(60, 120);
    assert_eq!(model.committed_clip_frames(), (60, 120));

    assert!(model.execute(&invocation(registry.viewport_transform())));
    assert_eq!(model.viewport_tool(), DemoViewportTool::Transform);
    assert!(model.execute(&invocation(registry.viewport_select())));
    assert_eq!(model.viewport_tool(), DemoViewportTool::Select);

    model.set_job(DemoJobPhase::Succeeded, 140);
    assert_eq!(model.job_phase(), DemoJobPhase::Succeeded);
    assert_eq!(model.job_progress_percent(), 100);
    model.set_job(DemoJobPhase::Failed, 65);
    assert_eq!(model.job_phase(), DemoJobPhase::Failed);
    assert_eq!(model.job_progress_percent(), 65);
}

#[test]
fn single_registry_owns_exact_existing_action_descriptors() {
    let registry = DemoActionRegistry::new();
    let descriptors = registry
        .iter()
        .map(|action| (action.id.as_str(), action.label.as_str()))
        .collect::<Vec<_>>();
    assert_eq!(
        descriptors,
        [
            ("workspace.edit", "Edit Workspace"),
            ("workspace.graph", "Graph Workspace"),
            ("shared.apply", "Apply Shared State"),
        ]
    );

    let library = include_str!("../src/lib.rs");
    let model = include_str!("../src/app_model.rs");
    assert!(!library.contains("ActionDescriptor::new"));
    assert_eq!(model.matches("ActionDescriptor::new").count(), 4);
    assert_eq!(
        registry.edit_workspace().icon,
        Some(stern_icons_phosphor::regular::PENCIL_SIMPLE.icon())
    );
    assert_eq!(
        registry.graph_workspace().icon,
        Some(stern_icons_phosphor::regular::GRAPH.icon())
    );
    assert_eq!(
        registry.apply_shared_state().icon,
        Some(stern_icons_phosphor::regular::CHECK_CIRCLE.icon())
    );
    assert_eq!(
        registry.viewport_select().icon,
        Some(stern_icons_phosphor::regular::CURSOR.icon())
    );
    assert_eq!(
        registry.viewport_transform().icon,
        Some(stern_icons_phosphor::regular::ARROWS_OUT_CARDINAL.icon())
    );
    for forbidden in [
        "Rect",
        "Primitive",
        "Semantic",
        "UiState",
        "WidgetId",
        "FrameOutput",
        "UiInput",
        "stern::render",
        "stern::platform",
    ] {
        assert!(
            !model.contains(forbidden),
            "forbidden model API: {forbidden}"
        );
    }
}

fn invocation(action: &ActionDescriptor) -> ActionInvocation {
    ActionInvocation::new(action.id.clone(), ActionSource::Menu, ActionContext::Global)
}
