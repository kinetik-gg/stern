//! Shared application model and action identity contract.

use stern::core::{ActionContext, ActionDescriptor, ActionInvocation, ActionSource};
use stern_demo::{DemoActionRegistry, DemoApplicationModel, DemoWorkspace};

#[test]
fn shared_model_preserves_pinned_workspace_identity_and_revision() {
    let registry = DemoActionRegistry::new();
    let mut model = DemoApplicationModel::new();

    assert_eq!(DemoWorkspace::Edit.id(), "edit-workspace");
    assert_eq!(DemoWorkspace::Graph.id(), "graph-workspace");
    assert_ne!(DemoWorkspace::Edit.id(), DemoWorkspace::Graph.id());
    assert_eq!(model.workspace(), DemoWorkspace::Edit);
    assert_eq!(model.applied_revision(), 0);

    assert!(model.execute(&invocation(registry.graph_workspace())));
    assert_eq!(model.workspace(), DemoWorkspace::Graph);
    assert!(model.execute(&invocation(registry.apply_shared_state())));
    assert_eq!(model.applied_revision(), 1);
    assert!(model.execute(&invocation(registry.edit_workspace())));
    assert_eq!(model.workspace(), DemoWorkspace::Edit);
    assert_eq!(model.applied_revision(), 1);
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
    assert_eq!(model.matches("ActionDescriptor::new").count(), 3);
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
