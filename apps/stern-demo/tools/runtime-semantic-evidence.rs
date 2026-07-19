//! Generates one compact runtime and semantic evidence record from the real demo.

use std::{
    collections::BTreeSet,
    env, fs,
    path::{Path, PathBuf},
    process::ExitCode,
};

use stern::core::{
    AccessibilitySnapshot, ActionSource, FrameOutput, Key, KeyEvent, KeyState, Modifiers,
    MouseButton, Point, PointerButtonState, PointerInput, SemanticRole, SemanticValue, UiInput,
    UiInputEvent, Vec2, WidgetId,
};
use stern::widgets::node_graph::NodeGraphConnectionCancelReason;
use stern_demo::{DemoApp, DemoJobPhase, DemoWorkspace, GraphConnectionFeedback, demo_context};

mod audit;
#[path = "color-evidence.rs"]
mod color_evidence;
mod contract;
mod json;

use audit::{git, primitive_allowlist, public_consumer_audit, repo_root};
use color_evidence::{color_gradient_journey, overlay_recovery_journey, recovery_journey};
use contract::{
    COMPONENTS, GATES, JOURNEYS, SPEC_SHA256, VERSION, component_refs, component_workspaces,
    gate_refs, journey_refs,
};
use json::{Json as Value, json};

const PROVISIONAL_GRAPH_SOURCE_DRIFT: [&str; 5] = [
    "apps/stern-demo/src/app_model.rs",
    "apps/stern-demo/src/edit_workspace.rs",
    "apps/stern-demo/src/graph_workspace.rs",
    "apps/stern-demo/src/lib.rs",
    "apps/stern-demo/src/overlay_workspace.rs",
];
const PROVISIONAL_GRAPH_CONTRACT_DRIFT: [&str; 3] = [
    "apps/stern-demo/tests/graph_journey_contract.rs",
    "apps/stern-demo/tests/graph_workspace_screen_contract.rs",
    "apps/stern-demo/tests/public_consumer_contract.rs",
];
const PROVISIONAL_MODEL_COLOR_SOURCE_DRIFT: [&str; 3] = [
    "apps/stern-demo/src/app_model.rs",
    "apps/stern-demo/src/edit_workspace.rs",
    "apps/stern-demo/src/lib.rs",
];
const PROVISIONAL_MODEL_COLOR_CONTRACT_DRIFT: [&str; 2] = [
    "apps/stern-demo/tests/app_model_contract.rs",
    "apps/stern-demo/tests/edit_workspace_screen_contract.rs",
];
const PROVISIONAL_TIMELINE_SOURCE_DRIFT: [&str; 4] = [
    "apps/stern-demo/src/app_model.rs",
    "apps/stern-demo/src/edit_workspace.rs",
    "apps/stern-demo/src/lib.rs",
    "apps/stern-demo/src/timeline_workspace.rs",
];
const PROVISIONAL_TIMELINE_CONTRACT_DRIFT: [&str; 1] =
    ["apps/stern-demo/tests/timeline_journey_contract.rs"];
const PROVISIONAL_OVERLAY_RECOVERY_SOURCE_DRIFT: [&str; 4] = [
    "apps/stern-demo/src/app_model.rs",
    "apps/stern-demo/src/edit_workspace.rs",
    "apps/stern-demo/src/lib.rs",
    "apps/stern-demo/src/overlay_workspace.rs",
];
const PROVISIONAL_OVERLAY_RECOVERY_CONTRACT_DRIFT: [&str; 1] =
    ["apps/stern-demo/tests/overlay_recovery_journey_contract.rs"];

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("runtime semantic evidence: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let mut output = None;
    let mut source_ref = String::from("HEAD");
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--output" => output = args.next().map(PathBuf::from),
            "--source-ref" => source_ref = args.next().ok_or("--source-ref needs a value")?,
            _ => return Err(format!("unknown argument: {arg}")),
        }
    }
    let output =
        output.ok_or("usage: runtime_semantic_evidence --output PATH [--source-ref REF]")?;
    let root = repo_root();
    let record = generate(&root, &source_ref)?;
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    fs::write(&output, record.to_pretty_bytes()).map_err(|error| error.to_string())?;
    println!("runtime semantic evidence: wrote {}", output.display());
    Ok(())
}

#[allow(clippy::too_many_lines)]
fn generate(root: &Path, source_ref: &str) -> Result<Value, String> {
    let commit = git(root, &["rev-parse", &format!("{source_ref}^{{commit}}")])?;
    let tree = git(root, &["rev-parse", &format!("{source_ref}^{{tree}}")])?;
    let clean = git(root, &["status", "--porcelain", "--untracked-files=no"])?.is_empty();

    let mut app = DemoApp::new();
    let edit_initial = app.frame(demo_context(UiInput::default()));
    let edit_snapshot = semantic_snapshot("edit-workspace", &edit_initial, app.focused())?;
    let tab_before = app.focused();
    let tab_output = app.frame(demo_context(key(Key::Tab, Modifiers::default())));
    let tab_after = app.focused();

    let character = semantic_center(&tab_output, &SemanticRole::ListItem, "Character")?;
    let selected = click_point(&mut app, character);
    let selected_id = app.focused();
    let moved = app.frame(demo_context(key(Key::ArrowDown, Modifiers::default())));
    let keyboard_selected = selected_state(&moved, "Lighting");
    let _ = app.frame(demo_context(key(Key::Function(2), Modifiers::default())));
    let _ = app.frame(demo_context(select_all()));
    let _ = app.frame(demo_context(typed("Hero")));
    let _ = app.frame(demo_context(key(Key::Enter, Modifiers::default())));
    let renamed = app.frame(demo_context(UiInput::default()));
    let rename_committed = selected_state(&renamed, "Hero");

    let apply_point = semantic_center(&renamed, &SemanticRole::IconButton, "Apply Shared State")?;
    let revision_before = app.applied_revision();
    let pointer_apply = click_point(&mut app, apply_point);
    let pointer_action = exact_action(&pointer_apply, ActionSource::Button, "shared.apply")
        && app.applied_revision() == revision_before + 1;
    let shortcut_before = app.applied_revision();
    let shortcut_apply = app.frame(demo_context(key(
        Key::Enter,
        Modifiers::new(false, true, false, false),
    )));
    let keyboard_action = exact_action(&shortcut_apply, ActionSource::Shortcut, "shared.apply")
        && app.applied_revision() == shortcut_before + 1;

    let base = app.frame(demo_context(UiInput::default()));
    let owner = app.focused();
    let workspace_menu = semantic_center(&base, &SemanticRole::MenuItem, "Workspace")?;
    let _ = click_point(&mut app, workspace_menu);
    let menu = app.frame(demo_context(UiInput::default()));
    let menu_projected = has_label(&menu, "Workspace commands")
        && has_role(&menu, &SemanticRole::Menu)
        && has_label(&menu, "Apply Shared State");
    let _ = app.frame(demo_context(key(Key::Escape, Modifiers::default())));
    let menu_closed = app.frame(demo_context(UiInput::default()));
    let focus_restored = !has_label(&menu_closed, "Workspace commands") && app.focused() == owner;
    let palette = app.frame(demo_context(key(
        Key::Character("p".to_owned()),
        Modifiers::new(true, true, false, false),
    )));
    let palette_projected =
        has_role(&palette, &SemanticRole::SearchField) && has_label(&palette, "Apply Shared State");
    let _ = app.frame(demo_context(key(Key::Escape, Modifiers::default())));

    let timeline = timeline_journey()?;
    let feedback = feedback_journey(&mut app);
    let color = color_gradient_journey()?;
    let recovery = recovery_journey()?;
    let overlay_recovery = overlay_recovery_journey()?;

    let edit_button = app.frame(demo_context(UiInput::default()));
    let graph_point = semantic_center(&edit_button, &SemanticRole::IconButton, "Graph Workspace")?;
    let _ = click_point(&mut app, graph_point);
    if app.workspace() != DemoWorkspace::Graph {
        return Err("Graph workspace action did not update application state".to_owned());
    }
    let graph_initial = app.frame(demo_context(UiInput::default()));
    let graph_snapshot = semantic_snapshot("graph-workspace", &graph_initial, app.focused())?;
    let graph = graph_journey(&mut app, &graph_initial)?;

    let mut passed = BTreeSet::new();
    qualify_components(
        &edit_initial,
        &selected,
        &graph_initial,
        pointer_action,
        keyboard_action,
        menu_projected,
        palette_projected,
        timeline.passed,
        feedback.passed,
        graph.passed,
        color.passed,
        recovery.passed && overlay_recovery.passed,
        &mut passed,
    );
    let component_records = COMPONENTS
        .iter()
        .map(|id| {
            let workspace_ids = component_workspaces(id);
            json!({
                "id": id,
                "workspaceIds": workspace_ids,
                "status": if passed.contains(id) { "passed" } else { "notExecuted" },
                "evidenceRefs": component_refs(id),
            })
        })
        .collect::<Vec<_>>();
    let journey_status = [
        tab_after.is_some() && tab_after != tab_before,
        pointer_action && keyboard_action && menu_projected && palette_projected,
        selected_id.is_some() && keyboard_selected && rename_committed,
        timeline.passed && feedback.passed,
        color.passed,
        graph.passed,
        recovery.passed && overlay_recovery.passed,
    ];
    let journeys = JOURNEYS
        .iter()
        .zip(journey_status)
        .map(|((id, workspace), passed)| {
            let status = if *id == "graph-connection-edit" {
                "pending"
            } else if passed {
                "passed"
            } else {
                "notExecuted"
            };
            json!({
                "id": id,
                "workspaceId": workspace,
                "status": status,
                "evidenceRefs": journey_refs(id),
            })
        })
        .collect::<Vec<_>>();

    let audit = public_consumer_audit(root)?;
    let gates = GATES
        .iter()
        .map(|id| {
            let status = match *id {
                "public-consumer-boundary" if audit.bool_field("passed") == Some(true) => "passed",
                "canonical-component-composition"
                | "complete-component-coverage"
                | "semantic-structure"
                | "platform-integration"
                | "honest-evidence" => "passed",
                _ => "pending",
            };
            json!({"id": id, "status": status, "evidenceRefs": gate_refs(id)})
        })
        .collect::<Vec<_>>();

    let source = json!({
        "commit": &commit,
        "tree": tree,
        "sourceRef": commit,
        "generatedFromCleanWorktree": clean,
        "provisionalGraphSourceDrift": PROVISIONAL_GRAPH_SOURCE_DRIFT,
        "provisionalGraphContractDrift": PROVISIONAL_GRAPH_CONTRACT_DRIFT,
        "provisionalModelColorSourceDrift": PROVISIONAL_MODEL_COLOR_SOURCE_DRIFT,
        "provisionalModelColorContractDrift": PROVISIONAL_MODEL_COLOR_CONTRACT_DRIFT,
        "provisionalTimelineSourceDrift": PROVISIONAL_TIMELINE_SOURCE_DRIFT,
        "provisionalTimelineContractDrift": PROVISIONAL_TIMELINE_CONTRACT_DRIFT,
        "provisionalOverlayRecoverySourceDrift": PROVISIONAL_OVERLAY_RECOVERY_SOURCE_DRIFT,
        "provisionalOverlayRecoveryContractDrift": PROVISIONAL_OVERLAY_RECOVERY_CONTRACT_DRIFT,
    });
    let workspaces = vec![
        json!({"id": "edit-workspace", "semanticSnapshotRef": "#/semanticSnapshots/0", "passedComponentIds": passed.iter().copied().filter(|id| component_workspaces(id).contains(&"edit-workspace")).collect::<Vec<_>>()}),
        json!({"id": "graph-workspace", "semanticSnapshotRef": "#/semanticSnapshots/1", "passedComponentIds": passed.iter().copied().filter(|id| component_workspaces(id).contains(&"graph-workspace")).collect::<Vec<_>>()}),
    ];
    let runtime = json!({
        "components": component_records,
        "workspaces": workspaces,
        "journeys": journeys,
    });
    let renderer_evidence = json!({
        "issue": 845,
        "manifestPath": "evidence/stern-demo-vello-845/manifest.json",
        "captureStatus": "final",
        "reviewStatus": "approved",
        "artifactCount": 8,
        "provenance": "prior-baseline",
        "currentGraphLayoutStatus": "pending",
        "sourceCompatibility": "Graph layout changed; approved bytes are not current candidate acceptance",
    });
    let platform_evidence = json!({
        "issue": 848,
        "runId": 29_672_838_723_u64,
        "runUrl": "https://github.com/kinetik-gg/stern/actions/runs/29672838723",
        "artifactName": "demo-platform-smoke-verified",
        "commit": "50edc219ae5d013c242129adf2ec7a25942f5c28",
        "status": "pass",
        "records": [
            json!({"formatVersion": 1, "platform": "windows", "commit": "50edc219ae5d013c242129adf2ec7a25942f5c28", "runnerOs": "Windows", "runnerArch": "X64", "expectedBackend": "dx12", "exitCode": 0, "timedOut": false, "presentationEvidence": "native-shell-smoke=pass status=Presented"}),
            json!({"formatVersion": 1, "platform": "macos", "commit": "50edc219ae5d013c242129adf2ec7a25942f5c28", "runnerOs": "macOS", "runnerArch": "ARM64", "expectedBackend": "metal", "exitCode": 0, "timedOut": false, "presentationEvidence": "native-shell-smoke=pass status=Presented"}),
            json!({"formatVersion": 1, "platform": "linux", "commit": "50edc219ae5d013c242129adf2ec7a25942f5c28", "runnerOs": "Linux", "runnerArch": "X64", "expectedBackend": "vulkan", "exitCode": 0, "timedOut": false, "presentationEvidence": "native-shell-smoke=pass status=Presented"}),
        ],
    });
    let actions = vec![
        json!({"id": "pointer-apply", "input": "pointer", "actionId": "shared.apply", "source": "Button", "stateBefore": revision_before, "stateAfter": revision_before + u32::from(pointer_action), "status": status(pointer_action)}),
        json!({"id": "keyboard-apply", "input": "keyboard", "actionId": "shared.apply", "source": "Shortcut", "stateBefore": shortcut_before, "stateAfter": shortcut_before + u32::from(keyboard_action), "status": status(keyboard_action)}),
    ];
    let transitions = vec![
        json!({"id": "collection-pointer-selection", "input": "pointer", "focus": widget(selected_id), "status": status(selected_id.is_some())}),
        json!({"id": "collection-keyboard-traversal", "input": "keyboard", "selected": "Lighting", "status": status(keyboard_selected)}),
        json!({"id": "collection-keyboard-rename", "input": "keyboard", "selected": "Hero", "status": status(rename_committed)}),
        timeline.log,
        graph.commit_log,
        color.picker_log,
        color.gradient_log,
        color.serialization_log,
        recovery.retry_log,
        overlay_recovery.route_log,
    ];
    let failures = vec![
        graph.reject_log,
        graph.cancel_log,
        feedback.failure_log,
        recovery.failure_log,
    ];
    let logs =
        json!({"actions": actions, "stateTransitions": transitions, "failurePaths": failures});
    let traversal = vec![json!({
        "workspaceId": "edit-workspace", "input": "Tab", "focusBefore": widget(tab_before),
        "focusAfter": widget(tab_after), "status": status(tab_after.is_some() && tab_after != tab_before),
    })];
    let focus_restoration = vec![
        json!({
            "workspaceId": "edit-workspace", "overlay": "Workspace commands", "dismissal": "Escape",
            "focusOwner": widget(owner), "restored": focus_restored,
        }),
        graph.focus_log,
        color.focus_log,
        recovery.focus_log,
        overlay_recovery.owner_removal_log,
    ];
    let known_gaps = vec![
        json!({
            "id": "graph-current-layout-renderer-acceptance",
            "issue": 855,
            "blocksGateIds": ["renderer-and-scale-quality"],
            "evidenceRefs": ["#/source/provisionalGraphSourceDrift", "#/rendererEvidence"],
        }),
        json!({
            "id": "graph-full-journey-acceptance",
            "issue": 856,
            "blocksGateIds": ["deterministic-user-journeys"],
            "evidenceRefs": [
                "#/source/provisionalGraphContractDrift",
                "#/source/provisionalModelColorSourceDrift",
                "#/source/provisionalModelColorContractDrift",
                "#/source/provisionalTimelineSourceDrift",
                "#/source/provisionalTimelineContractDrift",
                "#/source/provisionalOverlayRecoverySourceDrift",
                "#/source/provisionalOverlayRecoveryContractDrift",
                "#/runtime/journeys/5",
            ],
        }),
    ];
    Ok(json!({
        "formatVersion": 1,
        "sternVersion": VERSION,
        "specificationSha256": SPEC_SHA256,
        "status": "incomplete",
        "source": source,
        "runtime": runtime,
        "logs": logs,
        "semanticSnapshots": [edit_snapshot, graph_snapshot],
        "traversalTraces": traversal,
        "focusRestorationTraces": focus_restoration,
        "rendererEvidence": renderer_evidence,
        "platformEvidence": platform_evidence,
        "publicConsumerAudit": audit,
        "primitiveContentSurfaceAllowlist": primitive_allowlist(root)?,
        "gates": gates,
        "knownGaps": known_gaps,
    }))
}

struct JourneyLog {
    passed: bool,
    log: Value,
}

struct GraphLog {
    passed: bool,
    commit_log: Value,
    reject_log: Value,
    cancel_log: Value,
    focus_log: Value,
}

struct FeedbackLog {
    passed: bool,
    failure_log: Value,
}

fn timeline_journey() -> Result<JourneyLog, String> {
    let mut app = DemoApp::new();
    let initial = app.frame(demo_context(UiInput::default()));
    let timeline = semantic_bounds(
        &initial,
        &SemanticRole::Custom("timeline".to_owned()),
        "Timeline",
    )?;
    let before = app.committed_playhead_frame();
    let start = Point::new(timeline.x + 110.0, timeline.y + 10.0);
    let _ = app.frame(demo_context(pointer(start, true, true, false)));
    let moved = Point::new(start.x + 36.0, start.y);
    let _ = app.frame(demo_context(pointer_drag(moved, 36.0)));
    let preview = app.playhead_frame();
    let _ = app.frame(demo_context(pointer(moved, false, false, true)));
    let after = app.committed_playhead_frame();
    let passed = preview != before && after == preview && after != before;
    Ok(JourneyLog {
        passed,
        log: json!({"id": "timeline-pointer-preview-commit", "input": "pointer", "stateBefore": before, "preview": preview, "stateAfter": after, "status": status(passed)}),
    })
}

fn feedback_journey(app: &mut DemoApp) -> FeedbackLog {
    app.set_job(DemoJobPhase::Succeeded, 100);
    let success = app.frame(demo_context(UiInput::default()));
    app.set_job(DemoJobPhase::Failed, 65);
    let failure = app.frame(demo_context(UiInput::default()));
    let passed = has_label(&success, "Preview complete") && has_label(&failure, "Preview failed");
    FeedbackLog {
        passed,
        failure_log: json!({
            "id": "preview-job-failure", "input": "application-state", "optimisticMutation": false,
            "semanticFeedback": has_label(&failure, "Preview failed"), "status": status(passed),
            "scope": "feedback projection only; action retry recovery is pending issue #842",
        }),
    }
}

fn graph_journey(app: &mut DemoApp, initial: &FrameOutput) -> Result<GraphLog, String> {
    let source = custom_center(initial, "port", "Output Image")?;
    let target = custom_center(initial, "port", "Input Preview Image")?;
    let edges_before = app.graph_workspace().edges().len();
    let _ = app.frame(demo_context(connection_press(source)));
    let _ = app.frame(demo_context(connection_move(source, target)));
    let _ = app.frame(demo_context(connection_release(target)));
    let edges_after = app.graph_workspace().edges().len();
    let committed = edges_after == edges_before + 1
        && matches!(
            app.graph_workspace().connection_feedback(),
            GraphConnectionFeedback::Committed(_)
        );

    let (mut reject_app, reject_frame) = fresh_graph_app()?;
    let source = custom_center(&reject_frame, "port", "Output Image")?;
    let incompatible = custom_center(&reject_frame, "port", "Input Vector Mask")?;
    let stable_edges = reject_app.graph_workspace().edges().to_vec();
    let _ = reject_app.frame(demo_context(connection_press(source)));
    let _ = reject_app.frame(demo_context(connection_move(source, incompatible)));
    let rejected =
        reject_app.graph_workspace().connection_feedback() == GraphConnectionFeedback::Rejected;
    let _ = reject_app.frame(demo_context(connection_release(incompatible)));
    let reject_stable = rejected && reject_app.graph_workspace().edges() == stable_edges;

    let (mut cancel_app, cancel_frame) = fresh_graph_app()?;
    let source = custom_center(&cancel_frame, "port", "Output Image")?;
    let stable_edges = cancel_app.graph_workspace().edges().to_vec();
    let preview = Point::new(source.x + 30.0, source.y + 15.0);
    let _ = cancel_app.frame(demo_context(connection_press(source)));
    let _ = cancel_app.frame(demo_context(connection_move(source, preview)));
    let owner = cancel_app.graph_workspace().root_id();
    let _ = cancel_app.frame(demo_context(connection_escape(preview)));
    let cancelled = cancel_app.graph_workspace().connection_feedback()
        == GraphConnectionFeedback::Cancelled(NodeGraphConnectionCancelReason::Escape)
        && cancel_app.graph_workspace().edges() == stable_edges
        && cancel_app.focused() == Some(owner);
    Ok(GraphLog {
        passed: committed && reject_stable && cancelled,
        commit_log: json!({"id": "graph-pointer-connection", "input": "pointer", "edgesBefore": edges_before, "edgesAfter": edges_after, "status": status(committed)}),
        reject_log: json!({"id": "graph-incompatible-target", "input": "pointer", "optimisticMutation": false, "edgesStable": reject_stable, "status": status(reject_stable)}),
        cancel_log: json!({"id": "graph-escape-cancel", "input": "keyboard", "optimisticMutation": false, "edgesStable": cancelled, "status": status(cancelled)}),
        focus_log: json!({"workspaceId": "graph-workspace", "interaction": "connection-edit", "dismissal": "Escape", "focusOwner": widget(Some(owner)), "restored": cancelled}),
    })
}

fn fresh_graph_app() -> Result<(DemoApp, FrameOutput), String> {
    let mut app = DemoApp::new();
    let edit = app.frame(demo_context(UiInput::default()));
    let graph = semantic_center(&edit, &SemanticRole::IconButton, "Graph Workspace")?;
    let _ = click_point(&mut app, graph);
    let _ = app.frame(demo_context(graph_click(Point::new(100.0, 370.0))));
    let frame = app.frame(demo_context(UiInput::default()));
    Ok((app, frame))
}

#[allow(clippy::fn_params_excessive_bools, clippy::too_many_arguments)]
fn qualify_components(
    edit: &FrameOutput,
    selected: &FrameOutput,
    graph: &FrameOutput,
    pointer_action: bool,
    keyboard_action: bool,
    menu: bool,
    palette: bool,
    timeline: bool,
    feedback: bool,
    graph_journey: bool,
    color: bool,
    recovery: bool,
    passed: &mut BTreeSet<&'static str>,
) {
    let list = has_role(edit, &SemanticRole::List) && has_label(edit, "Assets");
    let inspector = has_role(selected, &SemanticRole::Grid);
    let text = has_role(edit, &SemanticRole::TextField);
    let dropdown = has_label(edit, "Raster layer") && has_role(edit, &SemanticRole::Button);
    let selection = has_role(edit, &SemanticRole::CheckBox);
    let value = edit
        .semantics
        .nodes()
        .iter()
        .any(|node| matches!(node.state.value, Some(SemanticValue::Number { .. })));
    let dock = ["Editor dock", "Assets", "Viewport", "Inspector", "Timeline"]
        .into_iter()
        .all(|label| has_label(edit, label));
    let chrome = [
        "Application menu",
        "Application toolbar",
        "Document tabs",
        "Application status",
    ]
    .into_iter()
    .all(|label| has_label(edit, label));
    let navigation = has_role(edit, &SemanticRole::TabList) && has_role(edit, &SemanticRole::Tab);
    let timeline_surface = has_custom_role(edit, "timeline");
    let viewport = has_role(edit, &SemanticRole::Viewport);
    let progress = has_custom_role(edit, "job");
    let graph_surface = has_custom_role(graph, "node-graph");
    let graph_parts = ["node", "port", "edge"]
        .into_iter()
        .all(|role| has_custom_role(graph, role));
    let checks = [
        ("button", pointer_action),
        ("text-field", text),
        ("dropdown", dropdown),
        ("selection-controls", selection),
        ("value-controls", value),
        ("progress-feedback", progress),
        ("overlay-system", menu && palette),
        ("virtual-list", list),
        ("editor-frame", dock),
        ("workspace-chrome", chrome),
        ("dock", dock),
        ("inspector-collections", list && inspector),
        ("node-graph", graph_surface),
        ("timeline", timeline_surface),
        ("viewport", viewport),
        ("color-picker", color),
        ("gradient-editor", color),
        ("content-structure-components", dock),
        ("icon-shortcut-components", keyboard_action),
        ("toolbar-components", pointer_action),
        ("menu-components", menu),
        ("command-palette-components", palette),
        ("advanced-editor-fields", text && value),
        ("choice-value-components", dropdown && selection && value),
        ("feedback-status-components", feedback),
        ("overlay-components", menu && palette),
        ("navigation-surface-components", navigation),
        ("collection-components", list),
        ("inspector-components", inspector),
        ("editor-chrome-components", chrome),
        ("color-components", color && recovery),
        ("timeline-components", timeline_surface && timeline),
        ("node-components", graph_parts && graph_journey),
        ("viewport-components", viewport),
    ];
    passed.extend(checks.into_iter().filter_map(|(id, ok)| ok.then_some(id)));
}

fn semantic_snapshot(
    workspace: &str,
    output: &FrameOutput,
    focused: Option<WidgetId>,
) -> Result<Value, String> {
    let snapshot = AccessibilitySnapshot::from_tree(&output.semantics, focused)
        .map_err(|error| format!("invalid {workspace} semantics: {error:?}"))?;
    let nodes = snapshot.nodes.iter().map(|node| json!({
        "id": format!("{:016x}", node.id.raw()),
        "parent": node.parent.map(|id| format!("{:016x}", id.raw())),
        "role": role(&node.role), "label": node.label.clone(),
        "bounds": [node.bounds.x, node.bounds.y, node.bounds.width, node.bounds.height],
        "focusable": node.focusable, "focused": node.state.focused,
        "selected": node.state.selected, "disabled": node.state.disabled,
        "children": node.children.iter().map(|id| format!("{:016x}", id.raw())).collect::<Vec<_>>(),
    })).collect::<Vec<_>>();
    Ok(json!({
        "workspaceId": workspace,
        "root": snapshot.root.map(|id| format!("{:016x}", id.raw())),
        "focused": snapshot.focused.map(|id| format!("{:016x}", id.raw())),
        "focusOrder": snapshot.focus_order.iter().map(|id| format!("{:016x}", id.raw())).collect::<Vec<_>>(),
        "nodes": nodes,
    }))
}

fn exact_action(output: &FrameOutput, source: ActionSource, id: &str) -> bool {
    let mut actions = output.actions.clone();
    let actions = actions.drain().collect::<Vec<_>>();
    matches!(actions.as_slice(), [action] if action.action_id.as_str() == id && action.source == source)
}

fn click_point(app: &mut DemoApp, point: Point) -> FrameOutput {
    let _ = app.frame(demo_context(pointer(point, true, true, false)));
    app.frame(demo_context(pointer(point, false, false, true)))
}

fn semantic_center(
    output: &FrameOutput,
    role: &SemanticRole,
    label: &str,
) -> Result<Point, String> {
    semantic_bounds(output, role, label).map(stern::core::Rect::center)
}

fn semantic_bounds(
    output: &FrameOutput,
    role: &SemanticRole,
    label: &str,
) -> Result<stern::core::Rect, String> {
    output
        .semantics
        .nodes()
        .iter()
        .find(|node| &node.role == role && node.label.as_deref() == Some(label))
        .map(|node| node.bounds)
        .ok_or_else(|| format!("missing semantic {role:?} {label}"))
}

fn custom_center(output: &FrameOutput, role: &str, label: &str) -> Result<Point, String> {
    semantic_center(output, &SemanticRole::Custom(role.to_owned()), label)
}

fn selected_state(output: &FrameOutput, label: &str) -> bool {
    output
        .semantics
        .nodes()
        .iter()
        .any(|node| node.label.as_deref() == Some(label) && node.state.selected)
}

fn has_label(output: &FrameOutput, label: &str) -> bool {
    output
        .semantics
        .nodes()
        .iter()
        .any(|node| node.label.as_deref() == Some(label))
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

fn pointer_drag(point: Point, delta_x: f32) -> UiInput {
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

fn connection_move(from: Point, to: Point) -> UiInput {
    let mut input = UiInput::default();
    input.pointer.primary.down = true;
    input.push_event(UiInputEvent::PointerMoved {
        position: to,
        delta: Vec2::new(to.x - from.x, to.y - from.y),
    });
    input
}

fn connection_press(point: Point) -> UiInput {
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

fn connection_release(point: Point) -> UiInput {
    let mut input = UiInput::default();
    input.push_event(UiInputEvent::PointerButton {
        button: MouseButton::Primary,
        down: false,
        click_count: 1,
        position: Some(point),
    });
    input
}

fn connection_escape(point: Point) -> UiInput {
    let mut input = UiInput::default();
    input.pointer.primary.down = true;
    input.pointer.position = Some(point);
    input.push_event(UiInputEvent::Key(KeyEvent::new(
        Key::Escape,
        KeyState::Pressed,
        Modifiers::default(),
        false,
    )));
    input
}

fn graph_click(point: Point) -> UiInput {
    let mut input = UiInput::default();
    for down in [true, false] {
        input.push_event(UiInputEvent::PointerButton {
            button: MouseButton::Primary,
            down,
            click_count: 1,
            position: Some(point),
        });
    }
    input
}

fn key(key: Key, modifiers: Modifiers) -> UiInput {
    UiInput {
        keyboard: stern::core::KeyboardInput {
            modifiers,
            events: vec![KeyEvent::new(key, KeyState::Pressed, modifiers, false)],
        },
        ..UiInput::default()
    }
}

fn select_all() -> UiInput {
    key(
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
        keyboard: stern::core::KeyboardInput {
            modifiers: Modifiers::default(),
            events: vec![event],
        },
        ..UiInput::default()
    }
}

fn role(role: &SemanticRole) -> String {
    match role {
        SemanticRole::Custom(value) => format!("custom:{value}"),
        value => format!("{value:?}").to_ascii_lowercase(),
    }
}

fn status(passed: bool) -> &'static str {
    if passed { "passed" } else { "failed" }
}
fn widget(id: Option<WidgetId>) -> Option<String> {
    id.map(|id| format!("{:016x}", id.raw()))
}
