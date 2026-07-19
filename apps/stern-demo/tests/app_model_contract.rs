//! Shared application model and action identity contract.

use stern::core::{ActionContext, ActionDescriptor, ActionInvocation, ActionSource, Color};
use stern::widgets::gradient_editor::{GradientEditorIntent, GradientInterpolationSpace};
use stern_demo::{
    DemoActionAvailability, DemoActionRegistry, DemoApplicationModel, DemoColorSaveState,
    DemoJobPhase, DemoTaggedColor, DemoViewportTool, DemoWorkspace,
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
fn shared_action_availability_is_application_owned_and_fail_closed() {
    let mut registry = DemoActionRegistry::new();
    let mut model = DemoApplicationModel::new();

    for (availability, visible, enabled) in [
        (DemoActionAvailability::Available, true, true),
        (DemoActionAvailability::Unavailable, true, false),
        (DemoActionAvailability::Hidden, false, false),
    ] {
        model.set_apply_availability(availability);
        registry.project_apply_shared_state(model.apply_availability());
        assert_eq!(registry.apply_shared_state().state.visible, visible);
        assert_eq!(registry.apply_shared_state().state.enabled, enabled);

        let revision = model.applied_revision();
        assert_eq!(
            model.execute(&invocation(registry.apply_shared_state())),
            availability == DemoActionAvailability::Available
        );
        assert_eq!(
            model.applied_revision(),
            revision + u32::from(availability == DemoActionAvailability::Available)
        );
    }
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
            ("color-style.save", "Save Color Style"),
        ]
    );

    let library = include_str!("../src/lib.rs");
    let model = include_str!("../src/app_model.rs");
    assert!(!library.contains("ActionDescriptor::new"));
    assert_eq!(model.matches("ActionDescriptor::new").count(), 5);
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
        registry.save_color_style().icon,
        Some(stern_icons_phosphor::regular::FLOPPY_DISK.icon())
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

#[test]
fn color_gradient_and_failed_save_remain_application_owned() {
    let registry = DemoActionRegistry::new();
    let mut model = DemoApplicationModel::new();
    let original_color = model.tagged_color();
    let original_style = model.color_style().clone();
    let original_stops = model.gradient_stops().to_vec();
    let selected = model.selected_gradient_stop();

    model.commit_color(Color::rgb8(12, 34, 56));
    assert_eq!(
        model.tagged_color(),
        DemoTaggedColor::Srgb(Color::rgb8(12, 34, 56))
    );
    assert_ne!(model.tagged_color(), original_color);
    assert_ne!(model.color_style(), &original_style);
    assert_eq!(model.color_revision(), 1);
    assert_eq!(
        model
            .gradient_stops()
            .iter()
            .find(|stop| stop.id == selected)
            .expect("selected stop")
            .color,
        model.tagged_color().color()
    );
    assert_eq!(
        model.gradient_interpolation(),
        GradientInterpolationSpace::Srgb
    );

    model.apply_gradient_intents(&[GradientEditorIntent::MoveStop {
        id: selected,
        position: 0.35,
    }]);
    assert_eq!(
        model
            .gradient_stops()
            .iter()
            .find(|stop| stop.id == selected)
            .expect("selected stop")
            .position
            .to_bits(),
        0.35_f32.to_bits()
    );

    model.apply_gradient_intents(&[GradientEditorIntent::Reverse]);
    assert_eq!(model.selected_gradient_stop(), selected);
    assert_eq!(
        model
            .gradient_stops()
            .iter()
            .map(|stop| stop.id)
            .collect::<Vec<_>>(),
        original_stops
            .iter()
            .rev()
            .map(|stop| stop.id)
            .collect::<Vec<_>>()
    );

    let alternate = model
        .gradient_stops()
        .iter()
        .find(|stop| stop.id != selected)
        .expect("alternate stop")
        .id;
    model.apply_gradient_intents(&[GradientEditorIntent::SelectStop(alternate)]);
    let alternate_color = model
        .gradient_stops()
        .iter()
        .find(|stop| stop.id == alternate)
        .expect("selected alternate stop")
        .color;
    assert_eq!(model.selected_gradient_stop(), alternate);
    assert_eq!(model.tagged_color(), DemoTaggedColor::Srgb(alternate_color));

    assert!(model.execute(&invocation(registry.save_color_style())));
    assert_eq!(model.color_save_state(), DemoColorSaveState::Failed);
    assert_eq!(model.serialized_color_style(), None);
    assert!(model.execute(&invocation(registry.save_color_style())));
    assert_eq!(model.color_save_state(), DemoColorSaveState::Succeeded);
    let serialized = model.serialized_color_style().expect("serialized style");
    assert!(serialized.starts_with("color=srgb("));
    assert!(serialized.contains(";gradient=sRGB"));
    assert_eq!(serialized.matches("=srgb(").count(), 3);
}

fn invocation(action: &ActionDescriptor) -> ActionInvocation {
    ActionInvocation::new(action.id.clone(), ActionSource::Menu, ActionContext::Global)
}
