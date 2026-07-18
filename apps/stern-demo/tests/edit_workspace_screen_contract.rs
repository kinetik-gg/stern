//! Pure public-facade evidence for the bounded Edit workspace slice.

use std::{collections::BTreeSet, fs, path::PathBuf};

use stern::core::{
    FrameContext, FrameOutput, Key, KeyEvent, KeyState, KeyboardInput, Modifiers, PhysicalSize,
    Point, PointerButtonState, PointerInput, ScaleFactor, SemanticNode, SemanticRole, Size,
    TimeInfo, UiInput, ViewportInfo, WidgetId,
};
use stern::render::RenderDiagnostic;
use stern_demo::{DemoApp, demo_context};

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
fn edit_workspace_reports_exact_ten_public_component_ids() {
    let trace = edit_workspace_trace();
    let observed = observed_component_ids(&trace);
    let expected = EXPECTED_COMPONENT_IDS.split_ascii_whitespace().collect();
    assert_eq!(observed, expected);
    let required = REQUIRED_IDS
        .split_ascii_whitespace()
        .collect::<BTreeSet<_>>();
    assert_eq!(required.len(), 34);
    assert!(observed.is_subset(&required));
    assert_eq!(required.difference(&observed).count(), 24);

    let assertions = runtime_journey_assertions(&trace);
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
    for ((id, required), assertions) in journeys.into_iter().zip(assertions) {
        let missing = required
            .split_ascii_whitespace()
            .filter(|component| !observed.contains(component))
            .collect::<Vec<_>>();
        let failed = assertions
            .into_iter()
            .enumerate()
            .filter_map(|(index, passes)| (!passes).then_some(index + 1))
            .collect::<Vec<_>>();
        if missing.is_empty() && failed.is_empty() {
            completed.push(id);
        }
        assert!(
            !missing.is_empty() || !failed.is_empty(),
            "{id} has no runtime-derived gap: missing={missing:?}, failed assertions={failed:?}",
        );
    }
    assert!(completed.is_empty(), "completed journeys: {completed:?}");
}

const EXPECTED_COMPONENT_IDS: &str = concat!(
    "button virtual-list workspace-chrome dock content-structure-components toolbar-components ",
    "navigation-surface-components collection-components inspector-components editor-chrome-components",
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
    moved: FrameOutput,
    invoked: FrameOutput,
    milestones: [bool; 4],
}
fn edit_workspace_trace() -> EditWorkspaceTrace {
    let mut app = DemoApp::new();
    let initial = app.frame(demo_context(UiInput::default()));
    let translation =
        stern::render_vello::translate_primitives(&initial.primitives, &app.render_resources());
    let selected = click(&mut app, &initial, &SemanticRole::ListItem, "Character");
    let moved = app.frame(demo_context(key(Key::ArrowDown)));
    let button = SemanticRole::IconButton;
    let invoked = click(&mut app, &moved, &button, "Apply Shared State");
    let action = has_action(&invoked, "shared.apply") && app.applied_revision() == 1;
    let focus = app.focused();
    let ids = dock_ids(&invoked);
    let resized = app.frame(resized_context(UiInput::default()));
    let stable = ids == dock_ids(&resized) && focus == app.focused();
    let focus = app.focused();
    let _ = app.frame(demo_context(key(Key::Escape)));
    let texture = !translation.commands.is_empty() && translation.diagnostics.is_empty();
    let restored = focus == app.focused();
    EditWorkspaceTrace {
        initial,
        selected,
        moved,
        invoked,
        milestones: [texture, action, stable, restored],
    }
}
fn observed_component_ids(trace: &EditWorkspaceTrace) -> BTreeSet<&'static str> {
    let action = trace.milestones[1];
    let list = has_role(&trace.initial, &SemanticRole::List) && has_label(&trace.initial, "Assets");
    let selected = node(&trace.selected, &SemanticRole::ListItem, "Character")
        .state
        .selected;
    let inspector = has_role(&trace.selected, &SemanticRole::Grid)
        && has_label(&trace.selected, "Vector layer");
    let dock = has_labels(&trace.initial, "Editor dock|Assets|Viewport|Inspector");
    let chrome = has_labels(
        &trace.initial,
        "Application menu|Application toolbar|Document tabs|Application status",
    );
    let navigation = has_role(&trace.initial, &SemanticRole::TabList)
        && has_role(&trace.initial, &SemanticRole::Tab);
    let structure = dock && has_role(&trace.initial, &SemanticRole::Frame) && trace.milestones[0];
    let toolbar = has_custom_role(&trace.initial, "toolbar") && action;
    EXPECTED_COMPONENT_IDS
        .split_ascii_whitespace()
        .zip([
            action,
            list,
            chrome,
            dock,
            structure,
            toolbar,
            navigation,
            list && selected,
            inspector,
            chrome && action,
        ])
        .filter_map(|(id, passes)| passes.then_some(id))
        .collect()
}

fn runtime_journey_assertions(trace: &EditWorkspaceTrace) -> [[bool; 3]; 7] {
    let item = &SemanticRole::ListItem;
    let character = node(&trace.selected, item, "Character").state.selected;
    let lighting = node(&trace.moved, item, "Lighting");
    let traversal = character && lighting.state.selected && lighting.state.focused;
    let nodes = trace.initial.semantics.nodes();
    let projections = nodes
        .iter()
        .filter(|node| has_semantic_action(node, "shared.apply"))
        .map(|node| node.state.disabled)
        .collect::<Vec<_>>();
    let consistent =
        projections.len() == 5 && projections.windows(2).all(|states| states[0] == states[1]);
    let timeline = has_custom_role(&trace.initial, "timeline");
    let graph = has_custom_role(&trace.initial, "node-graph");
    let overlay = has_role(&trace.initial, &SemanticRole::Menu)
        && has_role(&trace.initial, &SemanticRole::CommandPalette);
    let text = has_role(&trace.moved, &SemanticRole::TextField);
    let inspector = traversal
        && has_labels(&trace.selected, "Character|Vector layer")
        && has_labels(&trace.moved, "Lighting|Adjustment layer");
    let feedback = has_custom_role(&trace.invoked, "feedback-status");
    let color = has_custom_role(&trace.initial, "color-picker");
    let gradient = has_custom_role(&trace.initial, "gradient-editor");
    let valued = nodes
        .iter()
        .filter(|node| node.state.value.is_some())
        .count();
    let actions = !trace.invoked.actions.is_empty();
    let edit_outcomes = text && feedback && trace.milestones[1];
    let shell = has_role(&trace.initial, &SemanticRole::Dock)
        && has_role(&trace.initial, &SemanticRole::Frame);
    [
        [shell && trace.milestones[0], traversal, trace.milestones[2]],
        [projections.len() == 5, trace.milestones[1], consistent],
        [traversal && text, inspector, edit_outcomes],
        [
            timeline && has_role(&trace.initial, &SemanticRole::Viewport) && trace.milestones[0],
            timeline && actions,
            timeline && feedback && actions,
        ],
        [
            color && gradient && valued >= 2,
            gradient && valued > 0 && actions,
            color && overlay && trace.milestones[3],
        ],
        [
            graph && has_role(&trace.initial, &SemanticRole::Viewport) && trace.milestones[0],
            graph && overlay && actions,
            graph && trace.milestones[2],
        ],
        [
            overlay && !projections.is_empty(),
            overlay && trace.milestones[3],
            feedback && actions,
        ],
    ]
}

fn has_semantic_action(node: &SemanticNode, id: &str) -> bool {
    node.actions
        .iter()
        .filter_map(|action| action.action_id.as_ref())
        .any(|action| action.as_str() == id)
}

#[test]
fn edit_workspace_source_uses_only_public_stern_surface() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let manifest = fs::read_to_string(root.join("Cargo.toml")).expect("manifest");
    let source = ["src/lib.rs", "src/edit_workspace.rs"]
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

fn has_action(output: &FrameOutput, id: &str) -> bool {
    let mut actions = output.actions.clone();
    actions
        .drain()
        .any(|action| action.action_id.as_str() == id)
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

fn key(key: Key) -> UiInput {
    let event = KeyEvent::new(key, KeyState::Pressed, Modifiers::default(), false);
    UiInput {
        keyboard: KeyboardInput {
            modifiers: Modifiers::default(),
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
