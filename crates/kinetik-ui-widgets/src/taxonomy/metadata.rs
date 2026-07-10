use super::evidence::{
    EXPERIMENTAL_TAXONOMY_EVIDENCE, S12_VIEWPORT_ACTION_ROUTING_EVIDENCE, S12_VIEWPORT_EVIDENCE,
    S12_VIEWPORT_TOOLS_EVIDENCE, S13_DIAGNOSTIC_STRIP_EVIDENCE, S13_FEEDBACK_STACK_EVIDENCE,
    S13_JOB_LIST_EVIDENCE, S13_PROGRESS_EVIDENCE, STAGE_10_EXPERIMENTAL_EVIDENCE,
    STAGE_11_EXPERIMENTAL_EVIDENCE, STAGE_13_EXPERIMENTAL_EVIDENCE,
};
use super::types::{
    ComponentCapabilityAxis, ComponentCategory, ComponentConformanceStatus, ComponentMetadata,
};

use ComponentCapabilityAxis::{Accessibility, Input, LiveWorkflow, Model, Paint, Platform};
use ComponentCategory::{
    Collection, Control, Display, Docking, Input as InputCategory, Inspector, Overlay, System,
    TextEditing, Viewport,
};

const DISPLAY_AXES: &[ComponentCapabilityAxis] = &[Model, Paint, Accessibility];
const CONTROL_AXES: &[ComponentCapabilityAxis] = &[Model, Paint, Input, Accessibility];
const SHELL_CONTROL_AXES: &[ComponentCapabilityAxis] =
    &[Model, Paint, Input, Accessibility, Platform];
const WORKFLOW_AXES: &[ComponentCapabilityAxis] =
    &[Model, Paint, Input, Accessibility, Platform, LiveWorkflow];

const fn experimental(
    name: &'static str,
    slug: &'static str,
    category: ComponentCategory,
    required_axes: &'static [ComponentCapabilityAxis],
) -> ComponentMetadata {
    ComponentMetadata::new(
        name,
        slug,
        category,
        ComponentConformanceStatus::Experimental,
    )
    .with_required_axes(required_axes)
}

/// Data-only registry of Kinetik widget components and editor patterns.
pub const COMPONENT_METADATA: &[ComponentMetadata] = &[
    experimental("Label", "label", Display, DISPLAY_AXES)
        .with_evidence(EXPERIMENTAL_TAXONOMY_EVIDENCE),
    experimental("Image", "image", Display, DISPLAY_AXES)
        .with_evidence(EXPERIMENTAL_TAXONOMY_EVIDENCE),
    experimental("Separator", "separator", Display, DISPLAY_AXES)
        .with_evidence(EXPERIMENTAL_TAXONOMY_EVIDENCE),
    experimental("Button", "button", Control, CONTROL_AXES)
        .with_evidence(EXPERIMENTAL_TAXONOMY_EVIDENCE),
    experimental("IconButton", "icon-button", Control, CONTROL_AXES)
        .with_evidence(EXPERIMENTAL_TAXONOMY_EVIDENCE),
    experimental("Checkbox", "checkbox", InputCategory, CONTROL_AXES)
        .with_evidence(EXPERIMENTAL_TAXONOMY_EVIDENCE),
    experimental("RadioButton", "radio-button", InputCategory, CONTROL_AXES)
        .with_evidence(EXPERIMENTAL_TAXONOMY_EVIDENCE),
    experimental("Toggle", "toggle", InputCategory, CONTROL_AXES)
        .with_evidence(EXPERIMENTAL_TAXONOMY_EVIDENCE),
    experimental("Slider", "slider", InputCategory, CONTROL_AXES)
        .with_evidence(EXPERIMENTAL_TAXONOMY_EVIDENCE),
    experimental(
        "NumericInput",
        "numeric-input",
        InputCategory,
        SHELL_CONTROL_AXES,
    )
    .with_evidence(EXPERIMENTAL_TAXONOMY_EVIDENCE),
    experimental(
        "NumericScrubInput",
        "numeric-scrub-input",
        InputCategory,
        SHELL_CONTROL_AXES,
    )
    .with_evidence(EXPERIMENTAL_TAXONOMY_EVIDENCE),
    experimental("TextField", "text-field", TextEditing, SHELL_CONTROL_AXES)
        .with_evidence(EXPERIMENTAL_TAXONOMY_EVIDENCE),
    experimental(
        "MultiLineTextField",
        "multi-line-text-field",
        TextEditing,
        SHELL_CONTROL_AXES,
    )
    .with_evidence(EXPERIMENTAL_TAXONOMY_EVIDENCE),
    experimental(
        "SearchField",
        "search-field",
        TextEditing,
        SHELL_CONTROL_AXES,
    )
    .with_evidence(EXPERIMENTAL_TAXONOMY_EVIDENCE),
    experimental("List", "list", Collection, CONTROL_AXES)
        .with_evidence(EXPERIMENTAL_TAXONOMY_EVIDENCE),
    experimental("Grid", "grid", Collection, CONTROL_AXES)
        .with_evidence(EXPERIMENTAL_TAXONOMY_EVIDENCE),
    experimental("Table", "table", Collection, CONTROL_AXES)
        .with_evidence(EXPERIMENTAL_TAXONOMY_EVIDENCE),
    experimental("Tree", "tree", Collection, CONTROL_AXES)
        .with_evidence(EXPERIMENTAL_TAXONOMY_EVIDENCE),
    experimental("Outliner", "outliner", Collection, WORKFLOW_AXES)
        .with_stage(10)
        .with_evidence(STAGE_10_EXPERIMENTAL_EVIDENCE),
    experimental("AssetBrowser", "asset-browser", Collection, WORKFLOW_AXES)
        .with_stage(10)
        .with_evidence(STAGE_10_EXPERIMENTAL_EVIDENCE),
    experimental("PropertyGrid", "property-grid", Inspector, WORKFLOW_AXES)
        .with_evidence(EXPERIMENTAL_TAXONOMY_EVIDENCE),
    experimental(
        "PropertyAffordanceControls",
        "property-affordance-controls",
        Inspector,
        WORKFLOW_AXES,
    )
    .with_evidence(EXPERIMENTAL_TAXONOMY_EVIDENCE),
    experimental("Vector2Field", "vector-two-field", Inspector, WORKFLOW_AXES)
        .with_evidence(EXPERIMENTAL_TAXONOMY_EVIDENCE),
    experimental(
        "Vector3Field",
        "vector-three-field",
        Inspector,
        WORKFLOW_AXES,
    )
    .with_evidence(EXPERIMENTAL_TAXONOMY_EVIDENCE),
    experimental(
        "Vector4Field",
        "vector-four-field",
        Inspector,
        WORKFLOW_AXES,
    )
    .with_evidence(EXPERIMENTAL_TAXONOMY_EVIDENCE),
    experimental("ColorField", "color-field", Inspector, WORKFLOW_AXES)
        .with_evidence(EXPERIMENTAL_TAXONOMY_EVIDENCE),
    experimental("SelectField", "select-field", Inspector, WORKFLOW_AXES)
        .with_evidence(EXPERIMENTAL_TAXONOMY_EVIDENCE),
    experimental(
        "AssetSlotField",
        "asset-slot-field",
        Inspector,
        WORKFLOW_AXES,
    )
    .with_evidence(EXPERIMENTAL_TAXONOMY_EVIDENCE),
    experimental("PathField", "path-field", Inspector, SHELL_CONTROL_AXES)
        .with_evidence(EXPERIMENTAL_TAXONOMY_EVIDENCE),
    experimental("Panel", "panel", Docking, DISPLAY_AXES)
        .with_evidence(EXPERIMENTAL_TAXONOMY_EVIDENCE),
    experimental("Frame", "frame", Docking, CONTROL_AXES)
        .with_evidence(EXPERIMENTAL_TAXONOMY_EVIDENCE),
    experimental("Dock", "dock", Docking, CONTROL_AXES)
        .with_evidence(EXPERIMENTAL_TAXONOMY_EVIDENCE),
    experimental("Menu", "menu", Overlay, CONTROL_AXES)
        .with_evidence(EXPERIMENTAL_TAXONOMY_EVIDENCE),
    experimental("MenuItem", "menu-item", Overlay, CONTROL_AXES)
        .with_evidence(EXPERIMENTAL_TAXONOMY_EVIDENCE),
    experimental("ContextMenu", "context-menu", Overlay, CONTROL_AXES)
        .with_evidence(EXPERIMENTAL_TAXONOMY_EVIDENCE),
    experimental("Popover", "popover", Overlay, CONTROL_AXES)
        .with_evidence(EXPERIMENTAL_TAXONOMY_EVIDENCE),
    experimental("Tooltip", "tooltip", Overlay, DISPLAY_AXES)
        .with_evidence(EXPERIMENTAL_TAXONOMY_EVIDENCE),
    experimental("CommandPalette", "command-palette", Overlay, CONTROL_AXES)
        .with_evidence(EXPERIMENTAL_TAXONOMY_EVIDENCE),
    experimental("Viewport", "viewport", Viewport, WORKFLOW_AXES)
        .with_stage(12)
        .with_evidence(S12_VIEWPORT_EVIDENCE),
    experimental("ViewportTools", "viewport-tools", Viewport, WORKFLOW_AXES)
        .with_stage(12)
        .with_evidence(S12_VIEWPORT_TOOLS_EVIDENCE),
    experimental(
        "ViewportActionRouting",
        "viewport-action-routing",
        Viewport,
        WORKFLOW_AXES,
    )
    .with_stage(12)
    .with_evidence(S12_VIEWPORT_ACTION_ROUTING_EVIDENCE),
    experimental("NodeGraph", "node-graph", Viewport, WORKFLOW_AXES)
        .with_evidence(EXPERIMENTAL_TAXONOMY_EVIDENCE),
    experimental("Ruler", "ruler", Viewport, DISPLAY_AXES)
        .with_stage(11)
        .with_evidence(STAGE_11_EXPERIMENTAL_EVIDENCE),
    experimental("Dropdown", "dropdown", Overlay, CONTROL_AXES)
        .with_evidence(EXPERIMENTAL_TAXONOMY_EVIDENCE),
    experimental("MenuBar", "menu-bar", Overlay, CONTROL_AXES)
        .with_evidence(EXPERIMENTAL_TAXONOMY_EVIDENCE),
    experimental("Tabs", "tabs", Docking, CONTROL_AXES)
        .with_evidence(EXPERIMENTAL_TAXONOMY_EVIDENCE),
    experimental("Toolbar", "toolbar", System, CONTROL_AXES)
        .with_evidence(EXPERIMENTAL_TAXONOMY_EVIDENCE),
    experimental("StatusBar", "status-bar", System, WORKFLOW_AXES)
        .with_stage(13)
        .with_evidence(STAGE_13_EXPERIMENTAL_EVIDENCE),
    experimental("Modal", "modal", Overlay, CONTROL_AXES)
        .with_evidence(EXPERIMENTAL_TAXONOMY_EVIDENCE),
    experimental("Timeline", "timeline", Viewport, WORKFLOW_AXES)
        .with_stage(11)
        .with_evidence(STAGE_11_EXPERIMENTAL_EVIDENCE),
    experimental(
        "TransportControls",
        "transport-controls",
        Control,
        WORKFLOW_AXES,
    )
    .with_stage(11)
    .with_evidence(STAGE_11_EXPERIMENTAL_EVIDENCE),
    experimental(
        "ProgressIndicator",
        "progress-indicator",
        Display,
        DISPLAY_AXES,
    )
    .with_stage(13)
    .with_evidence(S13_PROGRESS_EVIDENCE),
    experimental("JobList", "job-list", System, WORKFLOW_AXES)
        .with_stage(13)
        .with_evidence(S13_JOB_LIST_EVIDENCE),
    experimental("DiagnosticStrip", "diagnostic-strip", System, WORKFLOW_AXES)
        .with_stage(13)
        .with_evidence(S13_DIAGNOSTIC_STRIP_EVIDENCE),
    experimental("FeedbackStack", "feedback-stack", System, WORKFLOW_AXES)
        .with_stage(13)
        .with_evidence(S13_FEEDBACK_STACK_EVIDENCE),
];
