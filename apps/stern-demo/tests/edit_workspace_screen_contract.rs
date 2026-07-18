//! Pure public-facade evidence for the bounded Edit workspace slice.

use std::{collections::BTreeSet, fs, path::PathBuf};

use stern::core::{
    ActionSource, FrameContext, FrameOutput, Key, KeyEvent, KeyState, KeyboardInput, Modifiers,
    PhysicalSize, Point, PointerButtonState, PointerInput, ScaleFactor, SemanticNode, SemanticRole,
    Size, TimeInfo, UiInput, ViewportInfo, WidgetId,
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
fn edit_workspace_reports_exact_fifteen_public_component_ids() {
    let trace = edit_workspace_trace();
    let observed = observed_component_ids(&trace);
    let expected = EXPECTED_COMPONENT_IDS.split_ascii_whitespace().collect();
    assert_eq!(observed, expected);
    let required = REQUIRED_IDS
        .split_ascii_whitespace()
        .collect::<BTreeSet<_>>();
    assert_eq!(required.len(), 34);
    assert!(observed.is_subset(&required));
    assert_eq!(required.difference(&observed).count(), 19);

    let evidence = runtime_journey_evidence(&trace);
    assert_eq!(
        evidence,
        [
            [RuntimeStepEvidence::Passed; 3],
            [RuntimeStepEvidence::Passed; 3],
            [
                RuntimeStepEvidence::NotExecuted,
                RuntimeStepEvidence::Passed,
                RuntimeStepEvidence::NotExecuted,
            ],
            [RuntimeStepEvidence::NotExecuted; 3],
            [RuntimeStepEvidence::NotExecuted; 3],
            [RuntimeStepEvidence::NotExecuted; 3],
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
    assert_eq!(completed, ["shared-action-projection"]);
}

const EXPECTED_COMPONENT_IDS: &str = concat!(
    "button virtual-list workspace-chrome dock content-structure-components toolbar-components ",
    "icon-shortcut-components menu-components command-palette-components overlay-system ",
    "overlay-components navigation-surface-components collection-components inspector-components ",
    "editor-chrome-components",
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
    inspector_projected: RuntimeStepEvidence,
}

fn edit_workspace_trace() -> EditWorkspaceTrace {
    let shared = shared_action_evidence();
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
    let inspector_projected = collection_traversed
        && has_labels(&selected, "Character|Vector layer")
        && has_labels(&moved, "Lighting|Adjustment layer");
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
            inspector_projected: RuntimeStepEvidence::executed(inspector_projected),
        },
    }
}

struct SharedActionEvidence {
    descriptor_projected: bool,
    activation_exact: bool,
    disabled_consistent: bool,
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
    let dock = has_labels(&trace.initial, "Editor dock|Assets|Viewport|Inspector");
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
    EXPECTED_COMPONENT_IDS
        .split_ascii_whitespace()
        .zip([
            action,
            list,
            chrome,
            dock,
            structure,
            toolbar,
            shared_surfaces,
            shared_surfaces,
            shared_surfaces,
            shared_surfaces,
            shared_surfaces,
            navigation,
            list && selected,
            inspector,
            chrome && action,
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
        [unexecuted, trace.evidence.inspector_projected, unexecuted],
        [unexecuted; 3],
        [unexecuted; 3],
        [unexecuted; 3],
        [unexecuted; 3],
    ]
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
