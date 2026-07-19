//! Pure public-facade evidence for the bounded Edit workspace slice.

use std::{collections::BTreeSet, fs, path::PathBuf};

use stern::core::{
    ActionSource, FrameContext, FrameOutput, Key, KeyEvent, KeyState, KeyboardInput, Modifiers,
    MouseButton, PhysicalSize, Point, PointerButtonState, PointerInput, ScaleFactor, SemanticNode,
    SemanticRole, SemanticValue, Size, TimeInfo, UiInput, UiInputEvent, Vec2, ViewportInfo,
    WidgetId,
};
use stern::render::RenderDiagnostic;
use stern::widgets::node_graph::{EdgeId, NodeId, PortEndpoint, PortId};
use stern_demo::{
    DemoApp, DemoJobPhase, DemoViewportTool, DemoWorkspace, GraphConnectionFeedback, demo_context,
};

const REQUIRED_IDS: &str = concat!(
    "button text-field dropdown selection-controls value-controls progress-feedback ",
    "overlay-system virtual-list editor-frame workspace-chrome dock inspector-collections ",
    "node-graph timeline viewport color-picker gradient-editor content-structure-components ",
    "icon-shortcut-components toolbar-components menu-components command-palette-components ",
    "advanced-editor-fields choice-value-components feedback-status-components overlay-components ",
    "navigation-surface-components collection-components inspector-components ",
    "editor-chrome-components color-components timeline-components node-components viewport-components",
);
#[test]
fn edit_workspace_composes_chrome_dock_panels_and_toolbar_action() {
    let mut app = DemoApp::new();
    let initial = app.frame(demo_context(UiInput::default()));
    node(&initial, &SemanticRole::Dock, "Editor dock");
    for label in ["Assets", "Viewport", "Inspector"] {
        node(&initial, &SemanticRole::Panel, label);
    }
    assert!(has_label(&initial, "Application toolbar"));

    let output = click(
        &mut app,
        &initial,
        &SemanticRole::IconButton,
        "Apply Shared State",
    );
    let mut emitted = output.actions.clone();
    assert!(
        emitted
            .drain()
            .any(|action| action.action_id.as_str() == "shared.apply")
    );
    assert_eq!(app.applied_revision(), 1);
}

#[test]
fn collection_pointer_and_keyboard_selection_projects_inspector() {
    let mut app = DemoApp::new();
    let initial = app.frame(demo_context(UiInput::default()));
    let selected = click(&mut app, &initial, &SemanticRole::ListItem, "Character");
    assert!(has_label(&selected, "Vector layer"));
    assert!(
        node(&selected, &SemanticRole::ListItem, "Character")
            .state
            .selected
    );

    let moved = app.frame(demo_context(key(Key::ArrowDown)));
    assert!(has_label(&moved, "Adjustment layer"));
    assert!(
        node(&moved, &SemanticRole::ListItem, "Lighting")
            .state
            .selected
    );
}

#[test]
fn collection_to_inspector_edit_executes_all_three_assertions() {
    let evidence = collection_inspector_evidence();
    assert!(evidence.identity_survived_scroll_and_rename);
    assert!(evidence.inspector_projected_selected_record);
    assert!(evidence.edit_lifecycle_observed);
}

#[test]
fn viewport_texture_translates_without_missing_resource() {
    let mut app = DemoApp::new();
    let output = app.frame(demo_context(UiInput::default()));
    node(&output, &SemanticRole::Viewport, "Viewport");
    let translation =
        stern::render_vello::translate_primitives(&output.primitives, &app.render_resources());
    assert!(!translation.commands.is_empty());
    assert!(!translation.diagnostics.iter().any(|diagnostic| matches!(
        diagnostic,
        RenderDiagnostic::MissingTexture(_) | RenderDiagnostic::MissingTextureSnapshot(_)
    )));
}

#[test]
fn timeline_scrub_and_clip_edits_use_retained_preview_commit_and_cancel_lifecycles() {
    let mut app = DemoApp::new();
    let initial = app.frame(demo_context(UiInput::default()));
    let timeline = custom_node(&initial, "timeline", "Timeline");
    let ruler = Point::new(timeline.bounds.x + 110.0, timeline.bounds.y + 10.0);
    let committed_playhead = app.committed_playhead_frame();
    let _ = app.frame(demo_context(pointer(ruler, true, true, false)));
    let scrubbed = Point::new(ruler.x + 36.0, ruler.y);
    let _ = app.frame(demo_context(drag(scrubbed, 36.0)));
    assert_ne!(app.playhead_frame(), committed_playhead);
    let _ = app.frame(demo_context(pointer(scrubbed, false, false, true)));
    assert_eq!(app.playhead_frame(), app.committed_playhead_frame());
    assert_ne!(app.committed_playhead_frame(), committed_playhead);

    let committed_clip = app.committed_clip_frames();
    let frame = app.frame(demo_context(UiInput::default()));
    let clip = custom_node(&frame, "timeline-item", "Hero clip");
    let start = clip.bounds.center();
    let moved = Point::new(start.x + 24.0, start.y);
    let _ = app.frame(demo_context(pointer(start, true, true, false)));
    let _ = app.frame(demo_context(drag(moved, 24.0)));
    assert_ne!(app.clip_frames(), committed_clip);
    let _ = app.frame(demo_context(pointer(moved, false, false, true)));
    assert_eq!(app.clip_frames(), app.committed_clip_frames());
    assert_ne!(app.committed_clip_frames(), committed_clip);

    let committed_clip = app.committed_clip_frames();
    let frame = app.frame(demo_context(UiInput::default()));
    let clip = custom_node(&frame, "timeline-item", "Hero clip");
    let trim = Point::new(clip.bounds.x + 1.0, clip.bounds.center().y);
    let preview = Point::new(trim.x + 12.0, trim.y);
    let _ = app.frame(demo_context(pointer(trim, true, true, false)));
    let _ = app.frame(demo_context(drag(preview, 12.0)));
    assert_ne!(app.clip_frames(), committed_clip);
    let _ = app.frame(demo_context(escape_while_dragging(preview)));
    assert_eq!(app.clip_frames(), committed_clip);
    assert_eq!(app.committed_clip_frames(), committed_clip);

    let frame = app.frame(demo_context(UiInput::default()));
    let clip = custom_node(&frame, "timeline-item", "Hero clip");
    let start = clip.bounds.center();
    let preview = Point::new(start.x + 18.0, start.y);
    let _ = app.frame(demo_context(pointer(start, true, true, false)));
    let _ = app.frame(demo_context(drag(preview, 18.0)));
    assert_ne!(app.clip_frames(), committed_clip);
    let _ = app.frame(demo_context(capture_lost(preview)));
    assert_eq!(app.clip_frames(), committed_clip);
    assert_eq!(app.committed_clip_frames(), committed_clip);
}

#[test]
fn viewport_tool_and_feedback_surfaces_project_shared_application_state() {
    let mut app = DemoApp::new();
    let initial = app.frame(demo_context(UiInput::default()));
    custom_node(&initial, "timeline", "Timeline");
    node(&initial, &SemanticRole::Viewport, "Viewport");
    let job = custom_node(&initial, "job", "Preview render");
    assert!(matches!(
        &job.state.value,
        Some(SemanticValue::Number { current, .. }) if current.to_bits() == 0.4_f32.to_bits()
    ));
    assert!(has_label(&initial, "Preview 40%"));
    assert_eq!(app.viewport_tool(), DemoViewportTool::Select);

    let _ = click(&mut app, &initial, &SemanticRole::Button, "Transform Tool");
    let transformed = app.frame(demo_context(UiInput::default()));
    assert_eq!(app.viewport_tool(), DemoViewportTool::Transform);
    assert_eq!(
        node(&transformed, &SemanticRole::Toggle, "Transform Tool")
            .state
            .checked,
        Some(true)
    );

    app.set_job(DemoJobPhase::Succeeded, 100);
    let succeeded = app.frame(demo_context(UiInput::default()));
    custom_node(&succeeded, "notification", "Preview complete");
    assert!(has_label(&succeeded, "Preview complete"));

    app.set_job(DemoJobPhase::Failed, 65);
    let failed = app.frame(demo_context(UiInput::default()));
    custom_node(&failed, "notification", "Preview failed");
    assert!(has_label(&failed, "Preview failed"));
}

#[test]
fn dock_ids_remain_stable_across_resize_and_focus() {
    let mut app = DemoApp::new();
    let initial = app.frame(demo_context(UiInput::default()));
    let focused = click(&mut app, &initial, &SemanticRole::ListItem, "Backdrop");
    let focused_id = app.focused().expect("selected row owns focus");
    let before = dock_ids(&focused);

    let resized = app.frame(resized_context(UiInput::default()));
    assert_eq!(dock_ids(&resized), before);
    assert_eq!(app.focused(), Some(focused_id));
}

#[test]
fn shared_menu_escape_and_outside_press_preserve_focus_owner() {
    let mut app = DemoApp::new();
    let initial = app.frame(demo_context(UiInput::default()));
    let focused = click(&mut app, &initial, &SemanticRole::ListItem, "Backdrop");
    let owner = app.focused().expect("declared focus owner");

    let _ = click(&mut app, &focused, &SemanticRole::MenuItem, "Workspace");
    let shown = app.frame(demo_context(UiInput::default()));
    assert!(has_label(&shown, "Workspace commands"));
    let _ = app.frame(demo_context(key(Key::Escape)));
    let closed = app.frame(demo_context(UiInput::default()));
    assert!(!has_label(&closed, "Workspace commands"));
    assert_eq!(app.focused(), Some(owner));

    let _ = click(&mut app, &closed, &SemanticRole::MenuItem, "Workspace");
    let shown = app.frame(demo_context(UiInput::default()));
    assert!(has_label(&shown, "Workspace commands"));
    let outside = Point::new(8.0, 440.0);
    let _ = app.frame(demo_context(pointer(outside, true, true, false)));
    let closed = app.frame(demo_context(pointer(outside, false, false, true)));
    assert!(!has_label(&closed, "Workspace commands"));
    assert_eq!(app.focused(), Some(owner));
}

#[test]
fn global_runtime_reports_exact_thirty_one_public_component_ids() {
    let trace = edit_workspace_trace();
    let observed = observed_component_ids(&trace);
    let expected = EXPECTED_COMPONENT_IDS.split_ascii_whitespace().collect();
    assert_eq!(observed, expected);
    let required = REQUIRED_IDS
        .split_ascii_whitespace()
        .collect::<BTreeSet<_>>();
    assert_eq!(required.len(), 34);
    assert!(observed.is_subset(&required));
    assert_eq!(required.difference(&observed).count(), 3);

    let evidence = runtime_journey_evidence(&trace);
    assert_eq!(
        evidence,
        [
            [RuntimeStepEvidence::Passed; 3],
            [RuntimeStepEvidence::Passed; 3],
            [RuntimeStepEvidence::Passed; 3],
            [RuntimeStepEvidence::Passed; 3],
            [RuntimeStepEvidence::NotExecuted; 3],
            [RuntimeStepEvidence::Passed; 3],
            [RuntimeStepEvidence::NotExecuted; 3],
        ],
    );
    let journeys = JOURNEY_COMPONENTS
        .lines()
        .map(|line| line.split_once('|').expect("journey components"))
        .collect::<Vec<_>>();
    assert_eq!(
        journeys
            .iter()
            .map(|(_, required)| required.split_ascii_whitespace().count())
            .collect::<Vec<_>>(),
        [6, 5, 10, 6, 5, 5, 5]
    );
    let mut completed = Vec::new();
    for ((id, required), evidence) in journeys.into_iter().zip(evidence) {
        let missing = required
            .split_ascii_whitespace()
            .filter(|component| !observed.contains(component))
            .collect::<Vec<_>>();
        let unproven = evidence
            .into_iter()
            .enumerate()
            .filter_map(|(index, evidence)| {
                (evidence != RuntimeStepEvidence::Passed).then_some((index + 1, evidence))
            })
            .collect::<Vec<_>>();
        if missing.is_empty() && unproven.is_empty() {
            completed.push(id);
        }
    }
    assert_eq!(
        completed,
        [
            "workspace-boot-and-traversal",
            "shared-action-projection",
            "collection-to-inspector-edit",
            "timeline-and-viewport-edit",
            "graph-connection-edit",
        ]
    );
}

const EXPECTED_COMPONENT_IDS: &str = concat!(
    "button text-field dropdown selection-controls value-controls virtual-list workspace-chrome ",
    "dock inspector-collections content-structure-components toolbar-components ",
    "icon-shortcut-components menu-components command-palette-components advanced-editor-fields ",
    "choice-value-components overlay-system overlay-components navigation-surface-components ",
    "collection-components inspector-components editor-chrome-components editor-frame timeline ",
    "viewport progress-feedback feedback-status-components timeline-components viewport-components ",
    "node-graph node-components",
);
const JOURNEY_COMPONENTS: &str = "\
workspace-boot-and-traversal|editor-frame workspace-chrome dock editor-chrome-components navigation-surface-components content-structure-components
shared-action-projection|button toolbar-components menu-components command-palette-components icon-shortcut-components
collection-to-inspector-edit|virtual-list collection-components inspector-collections inspector-components text-field advanced-editor-fields dropdown selection-controls value-controls choice-value-components
timeline-and-viewport-edit|timeline timeline-components viewport viewport-components progress-feedback feedback-status-components
color-and-gradient-edit|color-picker gradient-editor color-components overlay-system overlay-components
graph-connection-edit|node-graph node-components dock inspector-components viewport-components
overlay-and-failure-recovery|overlay-system overlay-components menu-components command-palette-components feedback-status-components";
struct EditWorkspaceTrace {
    initial: FrameOutput,
    selected: FrameOutput,
    evidence: ExecutedEditEvidence,
}

struct ExecutedEditEvidence {
    shell_booted: RuntimeStepEvidence,
    collection_traversed: RuntimeStepEvidence,
    identity_preserved: RuntimeStepEvidence,
    shared_action_invoked: RuntimeStepEvidence,
    shared_descriptor_projected: RuntimeStepEvidence,
    shared_activation_exact: RuntimeStepEvidence,
    shared_disabled_consistent: RuntimeStepEvidence,
    collection_identity_preserved: RuntimeStepEvidence,
    inspector_projected_record: RuntimeStepEvidence,
    edit_lifecycle_observed: RuntimeStepEvidence,
    timeline_edit_lifecycle: RuntimeStepEvidence,
    viewport_tool_projected: RuntimeStepEvidence,
    feedback_states_projected: RuntimeStepEvidence,
    graph_surface_projected: RuntimeStepEvidence,
    graph_components_projected: RuntimeStepEvidence,
    graph_connection_lifecycle: RuntimeStepEvidence,
}

fn edit_workspace_trace() -> EditWorkspaceTrace {
    let shared = shared_action_evidence();
    let collection = collection_inspector_evidence();
    let timeline_viewport = timeline_viewport_evidence();
    let graph = graph_connection_evidence();
    let mut app = DemoApp::new();
    let initial = app.frame(demo_context(UiInput::default()));
    let translation =
        stern::render_vello::translate_primitives(&initial.primitives, &app.render_resources());
    let selected = click(&mut app, &initial, &SemanticRole::ListItem, "Character");
    let moved = app.frame(demo_context(key(Key::ArrowDown)));
    let button = SemanticRole::IconButton;
    let invoked = click(&mut app, &moved, &button, "Apply Shared State");
    let shared_action_invoked =
        action_count(&invoked, "shared.apply") == 1 && app.applied_revision() == 1;
    let focus = app.focused();
    let ids = dock_ids(&invoked);
    let resized = app.frame(resized_context(UiInput::default()));
    let item = &SemanticRole::ListItem;
    let character = node(&selected, item, "Character").state.selected;
    let lighting = node(&moved, item, "Lighting");
    let collection_traversed = character && lighting.state.selected && lighting.state.focused;
    let shell_booted = has_role(&initial, &SemanticRole::Dock)
        && has_role(&initial, &SemanticRole::Frame)
        && !translation.commands.is_empty()
        && translation.diagnostics.is_empty();
    EditWorkspaceTrace {
        initial,
        selected,
        evidence: ExecutedEditEvidence {
            shell_booted: RuntimeStepEvidence::executed(shell_booted),
            collection_traversed: RuntimeStepEvidence::executed(collection_traversed),
            identity_preserved: RuntimeStepEvidence::executed(
                ids == dock_ids(&resized) && focus == app.focused(),
            ),
            shared_action_invoked: RuntimeStepEvidence::executed(shared_action_invoked),
            shared_descriptor_projected: RuntimeStepEvidence::executed(shared.descriptor_projected),
            shared_activation_exact: RuntimeStepEvidence::executed(shared.activation_exact),
            shared_disabled_consistent: RuntimeStepEvidence::executed(shared.disabled_consistent),
            collection_identity_preserved: RuntimeStepEvidence::executed(
                collection.identity_survived_scroll_and_rename,
            ),
            inspector_projected_record: RuntimeStepEvidence::executed(
                collection.inspector_projected_selected_record,
            ),
            edit_lifecycle_observed: RuntimeStepEvidence::executed(
                collection.edit_lifecycle_observed,
            ),
            timeline_edit_lifecycle: RuntimeStepEvidence::executed(
                timeline_viewport.timeline_edit_lifecycle,
            ),
            viewport_tool_projected: RuntimeStepEvidence::executed(
                timeline_viewport.viewport_tool_projected,
            ),
            feedback_states_projected: RuntimeStepEvidence::executed(
                timeline_viewport.feedback_states_projected,
            ),
            graph_surface_projected: RuntimeStepEvidence::executed(graph.surface_projected),
            graph_components_projected: RuntimeStepEvidence::executed(graph.components_projected),
            graph_connection_lifecycle: RuntimeStepEvidence::executed(graph.connection_lifecycle),
        },
    }
}

struct SharedActionEvidence {
    descriptor_projected: bool,
    activation_exact: bool,
    disabled_consistent: bool,
}

struct CollectionInspectorEvidence {
    identity_survived_scroll_and_rename: bool,
    inspector_projected_selected_record: bool,
    edit_lifecycle_observed: bool,
}

struct TimelineViewportEvidence {
    timeline_edit_lifecycle: bool,
    viewport_tool_projected: bool,
    feedback_states_projected: bool,
}

struct GraphConnectionEvidence {
    surface_projected: bool,
    components_projected: bool,
    connection_lifecycle: bool,
}

fn graph_connection_evidence() -> GraphConnectionEvidence {
    let mut app = DemoApp::new();
    let _ = app.frame(demo_context(pointer(
        Point::new(180.0, 70.0),
        true,
        true,
        false,
    )));
    let _ = app.frame(demo_context(pointer(
        Point::new(180.0, 70.0),
        false,
        false,
        true,
    )));
    assert_eq!(app.workspace(), DemoWorkspace::Graph);
    let _ = app.frame(demo_context(graph_click(Point::new(100.0, 370.0))));
    let initial = app.frame(demo_context(UiInput::default()));
    let surface_projected = has_custom_role(&initial, "node-graph");
    let components_projected = ["node", "port", "edge"]
        .into_iter()
        .all(|role| has_custom_role(&initial, role));
    let source = custom_node(&initial, "port", "Output Image")
        .bounds
        .center();
    let target = custom_node(&initial, "port", "Input Preview Image")
        .bounds
        .center();
    let original_edges = app.graph_workspace().edges().to_vec();

    let _ = app.frame(demo_context(graph_connection_press(source)));
    let _ = app.frame(demo_context(graph_connection_move(source, target)));
    let preview_isolated = app.graph_workspace().connection_active()
        && app.graph_workspace().connection_feedback() == GraphConnectionFeedback::Previewing
        && app.graph_workspace().edges() == original_edges;

    let _ = app.frame(demo_context(graph_connection_release(target)));
    let committed = app.graph_workspace().edges().last();
    let commit_owned = !app.graph_workspace().connection_active()
        && app.graph_workspace().connection_feedback()
            == GraphConnectionFeedback::Committed(EdgeId::from_raw(2))
        && app.graph_workspace().edges().len() == original_edges.len() + 1
        && committed.is_some_and(|edge| {
            edge.id == EdgeId::from_raw(2)
                && edge.from == PortEndpoint::new(NodeId::from_raw(1), PortId::from_raw(1))
                && edge.to == PortEndpoint::new(NodeId::from_raw(2), PortId::from_raw(2))
        });

    GraphConnectionEvidence {
        surface_projected,
        components_projected,
        connection_lifecycle: preview_isolated && commit_owned,
    }
}

fn timeline_viewport_evidence() -> TimelineViewportEvidence {
    let mut app = DemoApp::new();
    let initial = app.frame(demo_context(UiInput::default()));
    let timeline = custom_node(&initial, "timeline", "Timeline");
    let viewport_present = has_role(&initial, &SemanticRole::Viewport);
    let progress_present = matches!(
        &custom_node(&initial, "job", "Preview render").state.value,
        Some(SemanticValue::Number { current, .. }) if current.to_bits() == 0.4_f32.to_bits()
    );

    let ruler = Point::new(timeline.bounds.x + 110.0, timeline.bounds.y + 10.0);
    let previous_playhead = app.committed_playhead_frame();
    let _ = app.frame(demo_context(pointer(ruler, true, true, false)));
    let scrubbed = Point::new(ruler.x + 36.0, ruler.y);
    let _ = app.frame(demo_context(drag(scrubbed, 36.0)));
    let scrub_previewed = app.playhead_frame() != previous_playhead;
    let _ = app.frame(demo_context(pointer(scrubbed, false, false, true)));
    let scrub_committed = app.playhead_frame() == app.committed_playhead_frame()
        && app.committed_playhead_frame() != previous_playhead;

    let previous_clip = app.committed_clip_frames();
    let frame = app.frame(demo_context(UiInput::default()));
    let clip = custom_node(&frame, "timeline-item", "Hero clip");
    let start = clip.bounds.center();
    let moved = Point::new(start.x + 24.0, start.y);
    let _ = app.frame(demo_context(pointer(start, true, true, false)));
    let _ = app.frame(demo_context(drag(moved, 24.0)));
    let move_previewed = app.clip_frames() != previous_clip;
    let _ = app.frame(demo_context(pointer(moved, false, false, true)));
    let move_committed = app.clip_frames() == app.committed_clip_frames()
        && app.committed_clip_frames() != previous_clip;

    let committed_clip = app.committed_clip_frames();
    let frame = app.frame(demo_context(UiInput::default()));
    let clip = custom_node(&frame, "timeline-item", "Hero clip");
    let trim = Point::new(clip.bounds.x + 1.0, clip.bounds.center().y);
    let preview = Point::new(trim.x + 12.0, trim.y);
    let _ = app.frame(demo_context(pointer(trim, true, true, false)));
    let _ = app.frame(demo_context(drag(preview, 12.0)));
    let trim_previewed = app.clip_frames() != committed_clip;
    let _ = app.frame(demo_context(escape_while_dragging(preview)));
    let escape_cancelled =
        app.clip_frames() == committed_clip && app.committed_clip_frames() == committed_clip;

    let frame = app.frame(demo_context(UiInput::default()));
    let clip = custom_node(&frame, "timeline-item", "Hero clip");
    let start = clip.bounds.center();
    let preview = Point::new(start.x + 18.0, start.y);
    let _ = app.frame(demo_context(pointer(start, true, true, false)));
    let _ = app.frame(demo_context(drag(preview, 18.0)));
    let capture_previewed = app.clip_frames() != committed_clip;
    let _ = app.frame(demo_context(capture_lost(preview)));
    let capture_cancelled =
        app.clip_frames() == committed_clip && app.committed_clip_frames() == committed_clip;

    let tool_frame = app.frame(demo_context(UiInput::default()));
    let _ = click(
        &mut app,
        &tool_frame,
        &SemanticRole::Button,
        "Transform Tool",
    );
    let transformed = app.frame(demo_context(UiInput::default()));
    let viewport_tool_projected = viewport_present
        && app.viewport_tool() == DemoViewportTool::Transform
        && node(&transformed, &SemanticRole::Toggle, "Transform Tool")
            .state
            .checked
            == Some(true);

    app.set_job(DemoJobPhase::Succeeded, 100);
    let succeeded = app.frame(demo_context(UiInput::default()));
    let success_present = has_label(&succeeded, "Preview complete");
    app.set_job(DemoJobPhase::Failed, 65);
    let failed = app.frame(demo_context(UiInput::default()));
    let failure_present = has_label(&failed, "Preview failed");

    TimelineViewportEvidence {
        timeline_edit_lifecycle: scrub_previewed
            && scrub_committed
            && move_previewed
            && move_committed
            && trim_previewed
            && escape_cancelled
            && capture_previewed
            && capture_cancelled,
        viewport_tool_projected,
        feedback_states_projected: progress_present && success_present && failure_present,
    }
}

#[allow(clippy::too_many_lines)]
fn collection_inspector_evidence() -> CollectionInspectorEvidence {
    let mut app = DemoApp::new();
    let initial = app.frame(demo_context(UiInput::default()));
    let selected = click(&mut app, &initial, &SemanticRole::ListItem, "Character");
    let list_bounds = node(&selected, &SemanticRole::List, "Assets").bounds;
    let character_y = node(&selected, &SemanticRole::ListItem, "Character")
        .bounds
        .y;
    let _ = app.frame(demo_context(wheel(list_bounds.center(), -28.0)));
    let scrolled = app.frame(demo_context(UiInput::default()));
    let character = node(&scrolled, &SemanticRole::ListItem, "Character");
    let selection_survived_scroll = has_semantic_text(&scrolled, "Character")
        && character.state.selected
        && character.bounds.y.to_bits() != character_y.to_bits();
    let _ = click(&mut app, &scrolled, &SemanticRole::ListItem, "Character");

    let _ = app.frame(demo_context(key(Key::Function(2))));
    let _ = app.frame(demo_context(select_all()));
    let _ = app.frame(demo_context(typed("Hero")));
    let _ = app.frame(demo_context(key(Key::Enter)));
    let renamed = app.frame(demo_context(UiInput::default()));
    let rename_committed = node(&renamed, &SemanticRole::ListItem, "Hero")
        .state
        .selected
        && has_semantic_text(&renamed, "Hero");

    let opened = click(&mut app, &renamed, &SemanticRole::Button, "Vector layer");
    let picker = if has_label(&opened, "Text layer") {
        opened
    } else {
        app.frame(demo_context(UiInput::default()))
    };
    let _ = click_label(&mut app, &picker, "Text layer");
    let kind_changed = app.frame(demo_context(UiInput::default()));

    let visibility_changed = click(&mut app, &kind_changed, &SemanticRole::CheckBox, "Visible");
    let visible_is_false = !node(&visibility_changed, &SemanticRole::CheckBox, "Visible")
        .state
        .selected;

    let opacity_point = numeric_node(&visibility_changed).bounds.center();
    let _ = click_point(&mut app, opacity_point);
    let _ = app.frame(demo_context(select_all()));
    let _ = app.frame(demo_context(typed("0.5")));
    let _ = app.frame(demo_context(key(Key::Enter)));
    let opacity_changed = app.frame(demo_context(UiInput::default()));

    let _ = click(&mut app, &opacity_changed, &SemanticRole::ListItem, "Hero");
    let _ = app.frame(demo_context(key(Key::ArrowUp)));
    let hero = app.frame(demo_context(key(Key::ArrowDown)));
    let projected_record = visible_is_false
        && has_description(&hero, "Hero", "Text layer")
        && !node(&hero, &SemanticRole::CheckBox, "Visible")
            .state
            .selected
        && numeric_value(&hero).to_bits() == 0.5_f32.to_bits();

    let _ = app.frame(demo_context(key(Key::Function(2))));
    let _ = app.frame(demo_context(select_all()));
    let _ = app.frame(demo_context(typed("Backdrop")));
    let _ = app.frame(demo_context(key(Key::Enter)));
    let duplicate = app.frame(demo_context(UiInput::default()));
    let duplicate_rejected =
        has_label(&duplicate, "Name already exists") && has_semantic_text(&duplicate, "Hero");
    let _ = app.frame(demo_context(key(Key::Escape)));
    let hero = app.frame(demo_context(UiInput::default()));

    let _ = click(&mut app, &hero, &SemanticRole::ListItem, "Hero");
    let _ = app.frame(demo_context(key(Key::Function(2))));
    let _ = app.frame(demo_context(select_all()));
    let _ = app.frame(demo_context(key(Key::Backspace)));
    let _ = app.frame(demo_context(key(Key::Enter)));
    let empty = app.frame(demo_context(UiInput::default()));
    let empty_rejected = has_label(&empty, "Name is required") && has_semantic_text(&empty, "Hero");
    let _ = app.frame(demo_context(key(Key::Escape)));
    let hero = app.frame(demo_context(UiInput::default()));

    let _ = click(&mut app, &hero, &SemanticRole::ListItem, "Hero");
    let _ = app.frame(demo_context(key(Key::Function(2))));
    let _ = app.frame(demo_context(select_all()));
    let _ = app.frame(demo_context(typed("Cancelled")));
    let _ = app.frame(demo_context(key(Key::Escape)));
    let cancelled = app.frame(demo_context(UiInput::default()));
    let cancel_preserved = has_label(&cancelled, "Hero");

    let reset_name = click_label(&mut app, &cancelled, "Reset Name to default");
    let reset_kind = click_label(&mut app, &reset_name, "Reset Kind to default");
    let reset_visible = click_label(&mut app, &reset_kind, "Reset Visible to default");
    let _ = click_label(&mut app, &reset_visible, "Reset Opacity to default");
    let reset = app.frame(demo_context(UiInput::default()));
    let defaults_restored = has_label(&reset, "Character")
        && has_description(&reset, "Character", "Vector layer")
        && node(&reset, &SemanticRole::CheckBox, "Visible")
            .state
            .selected
        && numeric_value(&reset).to_bits() == 0.9_f32.to_bits();

    CollectionInspectorEvidence {
        identity_survived_scroll_and_rename: selection_survived_scroll && rename_committed,
        inspector_projected_selected_record: projected_record,
        edit_lifecycle_observed: duplicate_rejected
            && empty_rejected
            && cancel_preserved
            && defaults_restored,
    }
}

fn shared_action_evidence() -> SharedActionEvidence {
    let mut app = DemoApp::new();
    let initial = app.frame(demo_context(UiInput::default()));
    let toolbar_projected = !node(&initial, &SemanticRole::IconButton, "Apply Shared State")
        .state
        .disabled;
    let toolbar = click(
        &mut app,
        &initial,
        &SemanticRole::IconButton,
        "Apply Shared State",
    );
    let toolbar_exact = exact_action(&toolbar, ActionSource::Button) && app.applied_revision() == 1;

    let base = app.frame(demo_context(UiInput::default()));
    let _ = click(&mut app, &base, &SemanticRole::MenuItem, "Workspace");
    let menu = app.frame(demo_context(UiInput::default()));
    let menu_projected = !node(&menu, &SemanticRole::MenuItem, "Apply Shared State")
        .state
        .disabled;
    let menu_action = click(
        &mut app,
        &menu,
        &SemanticRole::MenuItem,
        "Apply Shared State",
    );
    let menu_exact = exact_action(&menu_action, ActionSource::Menu) && app.applied_revision() == 2;

    let context_base = app.frame(demo_context(UiInput::default()));
    let context_point = node(&context_base, &SemanticRole::Viewport, "Viewport")
        .bounds
        .center();
    let _ = app.frame(demo_context(secondary(context_point, true, true, false)));
    let _ = app.frame(demo_context(secondary(context_point, false, false, true)));
    let context = app.frame(demo_context(UiInput::default()));
    let context_projected = !node(&context, &SemanticRole::MenuItem, "Apply Shared State")
        .state
        .disabled;
    let context_action = click(
        &mut app,
        &context,
        &SemanticRole::MenuItem,
        "Apply Shared State",
    );
    let context_exact =
        exact_action(&context_action, ActionSource::Menu) && app.applied_revision() == 3;

    let shortcut = app.frame(demo_context(key_with_modifiers(
        Key::Enter,
        Modifiers::new(false, true, false, false),
    )));
    let shortcut_exact =
        exact_action(&shortcut, ActionSource::Shortcut) && app.applied_revision() == 4;

    let palette = app.frame(demo_context(key_with_modifiers(
        Key::Character("p".to_owned()),
        Modifiers::new(true, true, false, false),
    )));
    let palette_projected = has_role(&palette, &SemanticRole::SearchField)
        && !node(&palette, &SemanticRole::MenuItem, "Apply Shared State")
            .state
            .disabled;
    let palette_action = app.frame(demo_context(key(Key::Enter)));
    let palette_exact =
        exact_action(&palette_action, ActionSource::CommandPalette) && app.applied_revision() == 5;

    SharedActionEvidence {
        descriptor_projected: toolbar_projected
            && menu_projected
            && context_projected
            && shortcut_exact
            && palette_projected,
        activation_exact: toolbar_exact
            && menu_exact
            && context_exact
            && shortcut_exact
            && palette_exact,
        disabled_consistent: disabled_projection_evidence(),
    }
}

fn disabled_projection_evidence() -> bool {
    let mut app = DemoApp::new();
    app.set_apply_enabled(false);
    let initial = app.frame(demo_context(UiInput::default()));
    let toolbar = node(&initial, &SemanticRole::IconButton, "Apply Shared State")
        .state
        .disabled;

    let _ = click(&mut app, &initial, &SemanticRole::MenuItem, "Workspace");
    let menu = app.frame(demo_context(UiInput::default()));
    let menu_disabled = node(&menu, &SemanticRole::MenuItem, "Apply Shared State")
        .state
        .disabled;
    let _ = app.frame(demo_context(key(Key::Escape)));

    let context_base = app.frame(demo_context(UiInput::default()));
    let context_point = node(&context_base, &SemanticRole::Viewport, "Viewport")
        .bounds
        .center();
    let _ = app.frame(demo_context(secondary(context_point, true, true, false)));
    let _ = app.frame(demo_context(secondary(context_point, false, false, true)));
    let context = app.frame(demo_context(UiInput::default()));
    let context_disabled = node(&context, &SemanticRole::MenuItem, "Apply Shared State")
        .state
        .disabled;
    let _ = app.frame(demo_context(key(Key::Escape)));

    let palette = app.frame(demo_context(key_with_modifiers(
        Key::Character("p".to_owned()),
        Modifiers::new(true, true, false, false),
    )));
    let palette_disabled = node(&palette, &SemanticRole::MenuItem, "Apply Shared State")
        .state
        .disabled;
    let palette_action = app.frame(demo_context(key(Key::Enter)));
    let _ = app.frame(demo_context(key(Key::Escape)));
    let shortcut = app.frame(demo_context(key_with_modifiers(
        Key::Enter,
        Modifiers::new(false, true, false, false),
    )));

    toolbar
        && menu_disabled
        && context_disabled
        && palette_disabled
        && action_count(&palette_action, "shared.apply") == 0
        && action_count(&shortcut, "shared.apply") == 0
        && app.applied_revision() == 0
}

fn exact_action(output: &FrameOutput, source: ActionSource) -> bool {
    let mut actions = output.actions.clone();
    let actions = actions.drain().collect::<Vec<_>>();
    matches!(actions.as_slice(), [action]
        if action.action_id.as_str() == "shared.apply"
            && action.source == source
            && action.context == stern::core::ActionContext::Editor)
}

fn observed_component_ids(trace: &EditWorkspaceTrace) -> BTreeSet<&'static str> {
    let action = trace.evidence.shared_action_invoked.passed();
    let shared_surfaces = trace.evidence.shared_descriptor_projected.passed();
    let list = has_role(&trace.initial, &SemanticRole::List) && has_label(&trace.initial, "Assets");
    let selected = node(&trace.selected, &SemanticRole::ListItem, "Character")
        .state
        .selected;
    let inspector = has_role(&trace.selected, &SemanticRole::Grid)
        && has_label(&trace.selected, "Vector layer");
    let text_field = has_role(&trace.initial, &SemanticRole::TextField);
    let dropdown = has_label(&trace.initial, "Raster layer")
        && has_role(&trace.initial, &SemanticRole::Button);
    let selection_control = has_role(&trace.initial, &SemanticRole::CheckBox);
    let value_control = numeric_value(&trace.initial).to_bits() == 1.0_f32.to_bits();
    let inspector_collection = list && inspector;
    let advanced_fields = text_field && value_control;
    let choice_values = dropdown && selection_control && value_control;
    let dock = has_labels(
        &trace.initial,
        "Editor dock|Assets|Viewport|Inspector|Timeline",
    );
    let chrome = has_labels(
        &trace.initial,
        "Application menu|Application toolbar|Document tabs|Application status",
    );
    let navigation = has_role(&trace.initial, &SemanticRole::TabList)
        && has_role(&trace.initial, &SemanticRole::Tab);
    let structure = dock
        && has_role(&trace.initial, &SemanticRole::Frame)
        && trace.evidence.shell_booted.passed();
    let toolbar = has_custom_role(&trace.initial, "toolbar") && action;
    let timeline =
        has_custom_role(&trace.initial, "timeline") && has_label(&trace.initial, "Hero clip");
    let viewport = has_role(&trace.initial, &SemanticRole::Viewport);
    let progress = matches!(
        &custom_node(&trace.initial, "job", "Preview render")
            .state
            .value,
        Some(SemanticValue::Number { current, .. }) if current.to_bits() == 0.4_f32.to_bits()
    );
    let feedback = progress && trace.evidence.feedback_states_projected.passed();
    let timeline_components = timeline && trace.evidence.timeline_edit_lifecycle.passed();
    let viewport_components = viewport && trace.evidence.viewport_tool_projected.passed();
    let node_graph = trace.evidence.graph_surface_projected.passed();
    let node_components = trace.evidence.graph_components_projected.passed();
    EXPECTED_COMPONENT_IDS
        .split_ascii_whitespace()
        .zip([
            action,
            text_field,
            dropdown,
            selection_control,
            value_control,
            list,
            chrome,
            dock,
            inspector_collection,
            structure,
            toolbar,
            shared_surfaces,
            shared_surfaces,
            shared_surfaces,
            advanced_fields,
            choice_values,
            shared_surfaces,
            shared_surfaces,
            navigation,
            list && selected,
            inspector,
            chrome && action,
            structure,
            timeline,
            viewport,
            progress,
            feedback,
            timeline_components,
            viewport_components,
            node_graph,
            node_components,
        ])
        .filter_map(|(id, passes)| passes.then_some(id))
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RuntimeStepEvidence {
    Passed,
    Failed,
    NotExecuted,
}

impl RuntimeStepEvidence {
    fn executed(passed: bool) -> Self {
        if passed { Self::Passed } else { Self::Failed }
    }

    fn passed(self) -> bool {
        self == Self::Passed
    }
}

fn runtime_journey_evidence(trace: &EditWorkspaceTrace) -> [[RuntimeStepEvidence; 3]; 7] {
    let unexecuted = RuntimeStepEvidence::NotExecuted;
    [
        [
            trace.evidence.shell_booted,
            trace.evidence.collection_traversed,
            trace.evidence.identity_preserved,
        ],
        [
            trace.evidence.shared_descriptor_projected,
            trace.evidence.shared_activation_exact,
            trace.evidence.shared_disabled_consistent,
        ],
        [
            trace.evidence.collection_identity_preserved,
            trace.evidence.inspector_projected_record,
            trace.evidence.edit_lifecycle_observed,
        ],
        [
            trace.evidence.timeline_edit_lifecycle,
            trace.evidence.viewport_tool_projected,
            trace.evidence.feedback_states_projected,
        ],
        [unexecuted; 3],
        [
            trace.evidence.graph_surface_projected,
            trace.evidence.graph_components_projected,
            trace.evidence.graph_connection_lifecycle,
        ],
        [unexecuted; 3],
    ]
}

#[test]
fn edit_workspace_source_uses_only_public_stern_surface() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let manifest = fs::read_to_string(root.join("Cargo.toml")).expect("manifest");
    let source = [
        "src/lib.rs",
        "src/edit_workspace.rs",
        "src/timeline_workspace.rs",
    ]
    .map(|path| fs::read_to_string(root.join(path)).expect("demo source"))
    .join("");
    assert!(manifest.contains("stern = {"));
    for private in
        "stern-core stern-render stern-text stern-vello stern-widgets".split_ascii_whitespace()
    {
        assert!(!manifest.contains(private), "private dependency: {private}");
    }
    let forbidden = concat!(
        "stern_core stern_render stern_widgets RectPrimitive TextPrimitive SemanticNode::new ",
        "push_semantic_node .primitive( mod_widgets mod_theme mod_renderer fn_paint_",
    );
    let normalized = source.replace(' ', "_");
    for forbidden in forbidden.split_ascii_whitespace() {
        assert!(
            !source.contains(forbidden) && !normalized.contains(forbidden),
            "forbidden demo surface: {forbidden}"
        );
    }
}

fn node<'a>(output: &'a FrameOutput, role: &SemanticRole, label: &str) -> &'a SemanticNode {
    output
        .semantics
        .nodes()
        .iter()
        .find(|node| &node.role == role && node.label.as_deref() == Some(label))
        .expect("semantic node")
}

fn custom_node<'a>(output: &'a FrameOutput, role: &str, label: &str) -> &'a SemanticNode {
    node(output, &SemanticRole::Custom(role.to_owned()), label)
}

fn center(output: &FrameOutput, role: &SemanticRole, label: &str) -> Point {
    node(output, role, label).bounds.center()
}

fn has_label(output: &FrameOutput, label: &str) -> bool {
    output
        .semantics
        .nodes()
        .iter()
        .any(|node| node.label.as_deref() == Some(label))
}

fn has_labels(output: &FrameOutput, labels: &str) -> bool {
    labels.split('|').all(|label| has_label(output, label))
}

fn has_semantic_text(output: &FrameOutput, value: &str) -> bool {
    output
        .semantics
        .nodes()
        .iter()
        .any(|node| matches!(&node.state.value, Some(SemanticValue::Text(text)) if text == value))
}

fn has_description(output: &FrameOutput, label: &str, description: &str) -> bool {
    output.semantics.nodes().iter().any(|node| {
        node.label.as_deref() == Some(label) && node.description.as_deref() == Some(description)
    })
}

fn numeric_node(output: &FrameOutput) -> &SemanticNode {
    output
        .semantics
        .nodes()
        .iter()
        .find(|node| matches!(&node.state.value, Some(SemanticValue::Number { .. })))
        .expect("numeric inspector field")
}

fn numeric_value(output: &FrameOutput) -> f32 {
    let Some(SemanticValue::Number { current, .. }) = &numeric_node(output).state.value else {
        unreachable!("numeric node has numeric value")
    };
    *current
}

fn has_role(output: &FrameOutput, role: &SemanticRole) -> bool {
    output
        .semantics
        .nodes()
        .iter()
        .any(|node| &node.role == role)
}

fn has_custom_role(output: &FrameOutput, role: &str) -> bool {
    output
        .semantics
        .nodes()
        .iter()
        .any(|node| matches!(&node.role, SemanticRole::Custom(value) if value == role))
}

fn action_count(output: &FrameOutput, id: &str) -> usize {
    let mut actions = output.actions.clone();
    actions
        .drain()
        .filter(|action| action.action_id.as_str() == id)
        .count()
}

fn dock_ids(output: &FrameOutput) -> Vec<WidgetId> {
    output
        .semantics
        .nodes()
        .iter()
        .filter(|node| {
            [SemanticRole::Dock, SemanticRole::Frame, SemanticRole::Panel].contains(&node.role)
        })
        .map(|node| node.id)
        .collect()
}

fn click(app: &mut DemoApp, output: &FrameOutput, role: &SemanticRole, label: &str) -> FrameOutput {
    let point = center(output, role, label);
    let _ = app.frame(demo_context(pointer(point, true, true, false)));
    app.frame(demo_context(pointer(point, false, false, true)))
}

fn click_label(app: &mut DemoApp, output: &FrameOutput, label: &str) -> FrameOutput {
    let point = output
        .semantics
        .nodes()
        .iter()
        .find(|node| node.label.as_deref() == Some(label))
        .unwrap_or_else(|| panic!("semantic node {label}"))
        .bounds
        .center();
    click_point(app, point)
}

fn click_point(app: &mut DemoApp, point: Point) -> FrameOutput {
    let _ = app.frame(demo_context(pointer(point, true, true, false)));
    app.frame(demo_context(pointer(point, false, false, true)))
}

fn pointer(point: Point, down: bool, pressed: bool, released: bool) -> UiInput {
    UiInput {
        pointer: PointerInput {
            position: Some(point),
            primary: PointerButtonState::new(down, pressed, released),
            ..PointerInput::default()
        },
        ..UiInput::default()
    }
}

fn drag(point: Point, delta_x: f32) -> UiInput {
    UiInput {
        pointer: PointerInput {
            position: Some(point),
            delta: Vec2::new(delta_x, 0.0),
            primary: PointerButtonState::new(true, false, false),
            ..PointerInput::default()
        },
        ..UiInput::default()
    }
}

fn escape_while_dragging(point: Point) -> UiInput {
    let mut input = key(Key::Escape);
    input.pointer.position = Some(point);
    input.pointer.primary = PointerButtonState::new(true, false, false);
    input
        .events
        .push(stern::core::UiInputEvent::Key(KeyEvent::new(
            Key::Escape,
            KeyState::Pressed,
            Modifiers::default(),
            false,
        )));
    input
}

fn capture_lost(point: Point) -> UiInput {
    let mut input = UiInput {
        pointer: PointerInput {
            position: Some(point),
            primary: PointerButtonState::new(true, false, false),
            ..PointerInput::default()
        },
        ..UiInput::default()
    };
    input.push_event(stern::core::UiInputEvent::WindowFocusChanged(false));
    input
}

fn graph_click(point: Point) -> UiInput {
    let mut input = UiInput::default();
    input.push_event(UiInputEvent::PointerButton {
        button: MouseButton::Primary,
        down: true,
        click_count: 1,
        position: Some(point),
    });
    input.push_event(UiInputEvent::PointerButton {
        button: MouseButton::Primary,
        down: false,
        click_count: 1,
        position: Some(point),
    });
    input
}

fn graph_connection_press(point: Point) -> UiInput {
    let mut input = UiInput::default();
    input.pointer.primary.down = true;
    input.push_event(UiInputEvent::PointerButton {
        button: MouseButton::Primary,
        down: true,
        click_count: 1,
        position: Some(point),
    });
    input
}

fn graph_connection_move(from: Point, to: Point) -> UiInput {
    let mut input = UiInput::default();
    input.pointer.primary.down = true;
    input.push_event(UiInputEvent::PointerMoved {
        position: to,
        delta: Vec2::new(to.x - from.x, to.y - from.y),
    });
    input
}

fn graph_connection_release(point: Point) -> UiInput {
    let mut input = UiInput::default();
    input.push_event(UiInputEvent::PointerButton {
        button: MouseButton::Primary,
        down: false,
        click_count: 1,
        position: Some(point),
    });
    input
}

fn secondary(point: Point, down: bool, pressed: bool, released: bool) -> UiInput {
    UiInput {
        pointer: PointerInput {
            position: Some(point),
            secondary: PointerButtonState::new(down, pressed, released),
            ..PointerInput::default()
        },
        ..UiInput::default()
    }
}

fn wheel(point: Point, delta_y: f32) -> UiInput {
    UiInput {
        pointer: PointerInput {
            position: Some(point),
            wheel_delta: Vec2::new(0.0, delta_y),
            ..PointerInput::default()
        },
        ..UiInput::default()
    }
}

fn select_all() -> UiInput {
    key_with_modifiers(
        Key::Character("a".to_owned()),
        Modifiers::new(false, true, false, false),
    )
}

fn typed(text: &str) -> UiInput {
    let event = KeyEvent::new(
        Key::Character(text.to_owned()),
        KeyState::Pressed,
        Modifiers::default(),
        false,
    )
    .with_text(text);
    UiInput {
        keyboard: KeyboardInput {
            modifiers: Modifiers::default(),
            events: vec![event],
        },
        ..UiInput::default()
    }
}

fn key(key: Key) -> UiInput {
    key_with_modifiers(key, Modifiers::default())
}

fn key_with_modifiers(key: Key, modifiers: Modifiers) -> UiInput {
    let event = KeyEvent::new(key, KeyState::Pressed, modifiers, false);
    UiInput {
        keyboard: KeyboardInput {
            modifiers,
            events: vec![event],
        },
        ..UiInput::default()
    }
}

fn resized_context(input: UiInput) -> FrameContext {
    FrameContext::new(
        ViewportInfo::new(
            Size::new(960.0, 640.0),
            PhysicalSize::new(960, 640),
            ScaleFactor::ONE,
        ),
        input,
        TimeInfo::default(),
    )
}
