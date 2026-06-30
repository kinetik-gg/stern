//! Data-only taxonomy metadata for Kinetik UI widget components.

/// Kinetik-owned component category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ComponentCategory {
    /// Static display and decoration components.
    Display,
    /// Clickable, selectable, or adjustable controls.
    Control,
    /// Non-text input controls.
    Input,
    /// Text editing and text-query controls.
    TextEditing,
    /// Collection, virtualization, and structured data components.
    Collection,
    /// Docking, frame, and panel workspace components.
    Docking,
    /// Menus, popovers, command palettes, and other overlay surfaces.
    Overlay,
    /// Media, image, video, and editor viewport surfaces.
    Viewport,
    /// Property editing and inspector patterns.
    Inspector,
    /// System-level editor chrome and status patterns.
    System,
}

impl ComponentCategory {
    /// Returns a stable display name for the category.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Display => "Display",
            Self::Control => "Control",
            Self::Input => "Input",
            Self::TextEditing => "TextEditing",
            Self::Collection => "Collection",
            Self::Docking => "Docking",
            Self::Overlay => "Overlay",
            Self::Viewport => "Viewport",
            Self::Inspector => "Inspector",
            Self::System => "System",
        }
    }
}

/// Honest implementation status for a component or editor pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ComponentConformanceStatus {
    /// Public widget behavior exists for common usage.
    Implemented,
    /// Public models, helpers, or partial behavior exist, but the full component is incomplete.
    Partial,
    /// The component is part of the Kinetik vocabulary but is not implemented in this crate yet.
    Planned,
}

impl ComponentConformanceStatus {
    /// Returns a stable display name for the status.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Implemented => "Implemented",
            Self::Partial => "Partial",
            Self::Planned => "Planned",
        }
    }
}

/// Category for evidence attached to a component taxonomy entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ComponentEvidenceCategory {
    /// Evidence explaining the honest implementation status.
    Status,
    /// Evidence tying the entry to a restarted editor-toolkit stage.
    Stage,
    /// Evidence from deterministic conformance tests or contracts.
    Conformance,
    /// Evidence describing showcase/catalogue coverage without implying runtime behavior.
    Showcase,
}

impl ComponentEvidenceCategory {
    /// Returns a stable display name for the evidence category.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Status => "Status",
            Self::Stage => "Stage",
            Self::Conformance => "Conformance",
            Self::Showcase => "Showcase",
        }
    }
}

/// Stable evidence descriptor referenced by component taxonomy entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ComponentEvidence {
    /// Stable lower-kebab or dotted identifier.
    pub id: &'static str,
    /// Evidence category.
    pub category: ComponentEvidenceCategory,
    /// Short human-readable evidence summary.
    pub summary: &'static str,
}

impl ComponentEvidence {
    /// Creates a component taxonomy evidence descriptor.
    #[must_use]
    pub const fn new(
        id: &'static str,
        category: ComponentEvidenceCategory,
        summary: &'static str,
    ) -> Self {
        Self {
            id,
            category,
            summary,
        }
    }
}

/// Public component taxonomy entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ComponentMetadata {
    /// Public component or pattern name.
    pub name: &'static str,
    /// Stable lower-kebab identifier.
    pub slug: &'static str,
    /// Kinetik-owned category.
    pub category: ComponentCategory,
    /// Honest implementation status.
    pub status: ComponentConformanceStatus,
    /// Restarted editor-toolkit stage that currently owns the catalogue entry.
    pub stage: Option<u8>,
    /// Stable evidence identifiers backing the status and coverage metadata.
    pub evidence_ids: &'static [&'static str],
}

impl ComponentMetadata {
    /// Creates a component taxonomy entry.
    #[must_use]
    pub const fn new(
        name: &'static str,
        slug: &'static str,
        category: ComponentCategory,
        status: ComponentConformanceStatus,
    ) -> Self {
        Self {
            name,
            slug,
            category,
            status,
            stage: None,
            evidence_ids: &[],
        }
    }

    /// Sets the restarted editor-toolkit stage for this taxonomy entry.
    #[must_use]
    pub const fn with_stage(mut self, stage: u8) -> Self {
        self.stage = Some(stage);
        self
    }

    /// Sets the stable evidence identifiers for this taxonomy entry.
    #[must_use]
    pub const fn with_evidence(mut self, evidence_ids: &'static [&'static str]) -> Self {
        self.evidence_ids = evidence_ids;
        self
    }
}

use ComponentCategory::{
    Collection, Control, Display, Docking, Input, Inspector, Overlay, System, TextEditing, Viewport,
};
use ComponentConformanceStatus::{Implemented, Partial};
use ComponentEvidenceCategory::{Conformance, Showcase, Stage, Status};

const IMPLEMENTED_TAXONOMY_EVIDENCE: &[&str] = &[
    "status.implemented-public-api",
    "conformance.component-taxonomy",
    "showcase.metadata-only",
];
const PARTIAL_TAXONOMY_EVIDENCE: &[&str] = &[
    "status.partial-public-contract",
    "conformance.component-taxonomy",
    "showcase.metadata-only",
];
const STAGE_10_PARTIAL_EVIDENCE: &[&str] = &[
    "status.partial-public-contract",
    "stage.10-outliner-asset-browser",
    "conformance.component-taxonomy",
    "showcase.metadata-only",
];
const STAGE_11_PARTIAL_EVIDENCE: &[&str] = &[
    "status.partial-public-contract",
    "stage.11-timeline",
    "conformance.component-taxonomy",
    "showcase.metadata-only",
];
const STAGE_12_PARTIAL_EVIDENCE: &[&str] = &[
    "status.partial-public-contract",
    "stage.12-viewport-tools",
    "conformance.component-taxonomy",
    "showcase.metadata-only",
];
const STAGE_13_PARTIAL_EVIDENCE: &[&str] = &[
    "status.partial-public-contract",
    "stage.13-jobs-diagnostics",
    "conformance.component-taxonomy",
    "showcase.metadata-only",
];

/// Data-only registry of stable evidence descriptors.
pub const COMPONENT_EVIDENCE: &[ComponentEvidence] = &[
    ComponentEvidence::new(
        "status.implemented-public-api",
        Status,
        "Public widget behavior exists for common usage.",
    ),
    ComponentEvidence::new(
        "status.partial-public-contract",
        Status,
        "Public models, helpers, or partial behavior exist, but the full component is incomplete.",
    ),
    ComponentEvidence::new(
        "stage.10-outliner-asset-browser",
        Stage,
        "Restarted editor-toolkit Stage 10 outliner and asset browser catalogue coverage.",
    ),
    ComponentEvidence::new(
        "stage.11-timeline",
        Stage,
        "Restarted editor-toolkit Stage 11 timeline catalogue coverage.",
    ),
    ComponentEvidence::new(
        "stage.12-viewport-tools",
        Stage,
        "Restarted editor-toolkit Stage 12 viewport tools catalogue coverage.",
    ),
    ComponentEvidence::new(
        "stage.13-jobs-diagnostics",
        Stage,
        "Restarted editor-toolkit Stage 13 jobs and diagnostics catalogue coverage.",
    ),
    ComponentEvidence::new(
        "conformance.component-taxonomy",
        Conformance,
        "Covered by the component taxonomy conformance test matrix.",
    ),
    ComponentEvidence::new(
        "showcase.metadata-only",
        Showcase,
        "Tracked by taxonomy metadata only; no interactive showcase behavior is claimed.",
    ),
];

/// Data-only registry of Kinetik widget components and editor patterns.
pub const COMPONENT_METADATA: &[ComponentMetadata] = &[
    ComponentMetadata::new("Label", "label", Display, Implemented)
        .with_evidence(IMPLEMENTED_TAXONOMY_EVIDENCE),
    ComponentMetadata::new("Image", "image", Display, Implemented)
        .with_evidence(IMPLEMENTED_TAXONOMY_EVIDENCE),
    ComponentMetadata::new("Separator", "separator", Display, Implemented)
        .with_evidence(IMPLEMENTED_TAXONOMY_EVIDENCE),
    ComponentMetadata::new("Button", "button", Control, Implemented)
        .with_evidence(IMPLEMENTED_TAXONOMY_EVIDENCE),
    ComponentMetadata::new("IconButton", "icon-button", Control, Implemented)
        .with_evidence(IMPLEMENTED_TAXONOMY_EVIDENCE),
    ComponentMetadata::new("Checkbox", "checkbox", Input, Implemented)
        .with_evidence(IMPLEMENTED_TAXONOMY_EVIDENCE),
    ComponentMetadata::new("RadioButton", "radio-button", Input, Implemented)
        .with_evidence(IMPLEMENTED_TAXONOMY_EVIDENCE),
    ComponentMetadata::new("Toggle", "toggle", Input, Implemented)
        .with_evidence(IMPLEMENTED_TAXONOMY_EVIDENCE),
    ComponentMetadata::new("Slider", "slider", Input, Implemented)
        .with_evidence(IMPLEMENTED_TAXONOMY_EVIDENCE),
    ComponentMetadata::new("NumericInput", "numeric-input", Input, Implemented)
        .with_evidence(IMPLEMENTED_TAXONOMY_EVIDENCE),
    ComponentMetadata::new(
        "NumericScrubInput",
        "numeric-scrub-input",
        Input,
        Implemented,
    )
    .with_evidence(IMPLEMENTED_TAXONOMY_EVIDENCE),
    ComponentMetadata::new("TextField", "text-field", TextEditing, Implemented)
        .with_evidence(IMPLEMENTED_TAXONOMY_EVIDENCE),
    ComponentMetadata::new(
        "MultiLineTextField",
        "multi-line-text-field",
        TextEditing,
        Implemented,
    )
    .with_evidence(IMPLEMENTED_TAXONOMY_EVIDENCE),
    ComponentMetadata::new("SearchField", "search-field", TextEditing, Implemented)
        .with_evidence(IMPLEMENTED_TAXONOMY_EVIDENCE),
    ComponentMetadata::new("List", "list", Collection, Partial)
        .with_evidence(PARTIAL_TAXONOMY_EVIDENCE),
    ComponentMetadata::new("Grid", "grid", Collection, Partial)
        .with_evidence(PARTIAL_TAXONOMY_EVIDENCE),
    ComponentMetadata::new("Table", "table", Collection, Partial)
        .with_evidence(PARTIAL_TAXONOMY_EVIDENCE),
    ComponentMetadata::new("Tree", "tree", Collection, Partial)
        .with_evidence(PARTIAL_TAXONOMY_EVIDENCE),
    ComponentMetadata::new("Outliner", "outliner", Collection, Partial)
        .with_stage(10)
        .with_evidence(STAGE_10_PARTIAL_EVIDENCE),
    ComponentMetadata::new("AssetBrowser", "asset-browser", Collection, Partial)
        .with_stage(10)
        .with_evidence(STAGE_10_PARTIAL_EVIDENCE),
    ComponentMetadata::new("PropertyGrid", "property-grid", Inspector, Partial)
        .with_evidence(PARTIAL_TAXONOMY_EVIDENCE),
    ComponentMetadata::new(
        "PropertyAffordanceControls",
        "property-affordance-controls",
        Inspector,
        Implemented,
    )
    .with_evidence(IMPLEMENTED_TAXONOMY_EVIDENCE),
    ComponentMetadata::new("Vector2Field", "vector-two-field", Inspector, Implemented)
        .with_evidence(IMPLEMENTED_TAXONOMY_EVIDENCE),
    ComponentMetadata::new("Vector3Field", "vector-three-field", Inspector, Implemented)
        .with_evidence(IMPLEMENTED_TAXONOMY_EVIDENCE),
    ComponentMetadata::new("Vector4Field", "vector-four-field", Inspector, Implemented)
        .with_evidence(IMPLEMENTED_TAXONOMY_EVIDENCE),
    ComponentMetadata::new("ColorField", "color-field", Inspector, Implemented)
        .with_evidence(IMPLEMENTED_TAXONOMY_EVIDENCE),
    ComponentMetadata::new("SelectField", "select-field", Inspector, Implemented)
        .with_evidence(IMPLEMENTED_TAXONOMY_EVIDENCE),
    ComponentMetadata::new("AssetSlotField", "asset-slot-field", Inspector, Implemented)
        .with_evidence(IMPLEMENTED_TAXONOMY_EVIDENCE),
    ComponentMetadata::new("PathField", "path-field", Inspector, Implemented)
        .with_evidence(IMPLEMENTED_TAXONOMY_EVIDENCE),
    ComponentMetadata::new("Panel", "panel", Docking, Partial)
        .with_evidence(PARTIAL_TAXONOMY_EVIDENCE),
    ComponentMetadata::new("Frame", "frame", Docking, Partial)
        .with_evidence(PARTIAL_TAXONOMY_EVIDENCE),
    ComponentMetadata::new("Dock", "dock", Docking, Partial)
        .with_evidence(PARTIAL_TAXONOMY_EVIDENCE),
    ComponentMetadata::new("Menu", "menu", Overlay, Partial)
        .with_evidence(PARTIAL_TAXONOMY_EVIDENCE),
    ComponentMetadata::new("MenuItem", "menu-item", Overlay, Partial)
        .with_evidence(PARTIAL_TAXONOMY_EVIDENCE),
    ComponentMetadata::new("ContextMenu", "context-menu", Overlay, Partial)
        .with_evidence(PARTIAL_TAXONOMY_EVIDENCE),
    ComponentMetadata::new("Popover", "popover", Overlay, Partial)
        .with_evidence(PARTIAL_TAXONOMY_EVIDENCE),
    ComponentMetadata::new("Tooltip", "tooltip", Overlay, Partial)
        .with_evidence(PARTIAL_TAXONOMY_EVIDENCE),
    ComponentMetadata::new("CommandPalette", "command-palette", Overlay, Partial)
        .with_evidence(PARTIAL_TAXONOMY_EVIDENCE),
    ComponentMetadata::new("Viewport", "viewport", Viewport, Partial)
        .with_stage(12)
        .with_evidence(STAGE_12_PARTIAL_EVIDENCE),
    ComponentMetadata::new("ViewportTools", "viewport-tools", Viewport, Partial)
        .with_stage(12)
        .with_evidence(STAGE_12_PARTIAL_EVIDENCE),
    ComponentMetadata::new(
        "ViewportActionRouting",
        "viewport-action-routing",
        Viewport,
        Partial,
    )
    .with_stage(12)
    .with_evidence(STAGE_12_PARTIAL_EVIDENCE),
    ComponentMetadata::new("NodeGraph", "node-graph", Viewport, Partial)
        .with_evidence(PARTIAL_TAXONOMY_EVIDENCE),
    ComponentMetadata::new("Ruler", "ruler", Viewport, Partial)
        .with_stage(11)
        .with_evidence(STAGE_11_PARTIAL_EVIDENCE),
    ComponentMetadata::new("Dropdown", "dropdown", Overlay, Partial)
        .with_evidence(PARTIAL_TAXONOMY_EVIDENCE),
    ComponentMetadata::new("MenuBar", "menu-bar", Overlay, Partial)
        .with_evidence(PARTIAL_TAXONOMY_EVIDENCE),
    ComponentMetadata::new("Tabs", "tabs", Docking, Partial)
        .with_evidence(PARTIAL_TAXONOMY_EVIDENCE),
    ComponentMetadata::new("Toolbar", "toolbar", System, Partial)
        .with_evidence(PARTIAL_TAXONOMY_EVIDENCE),
    ComponentMetadata::new("StatusBar", "status-bar", System, Partial)
        .with_stage(13)
        .with_evidence(STAGE_13_PARTIAL_EVIDENCE),
    ComponentMetadata::new("Modal", "modal", Overlay, Partial)
        .with_evidence(PARTIAL_TAXONOMY_EVIDENCE),
    ComponentMetadata::new("Timeline", "timeline", Viewport, Partial)
        .with_stage(11)
        .with_evidence(STAGE_11_PARTIAL_EVIDENCE),
    ComponentMetadata::new("TransportControls", "transport-controls", Control, Partial)
        .with_stage(11)
        .with_evidence(STAGE_11_PARTIAL_EVIDENCE),
    ComponentMetadata::new("ProgressIndicator", "progress-indicator", Display, Partial)
        .with_stage(13)
        .with_evidence(STAGE_13_PARTIAL_EVIDENCE),
    ComponentMetadata::new("JobList", "job-list", System, Partial)
        .with_stage(13)
        .with_evidence(STAGE_13_PARTIAL_EVIDENCE),
    ComponentMetadata::new("DiagnosticStrip", "diagnostic-strip", System, Partial)
        .with_stage(13)
        .with_evidence(STAGE_13_PARTIAL_EVIDENCE),
    ComponentMetadata::new("FeedbackStack", "feedback-stack", System, Partial)
        .with_stage(13)
        .with_evidence(STAGE_13_PARTIAL_EVIDENCE),
];

/// Looks up evidence metadata by exact stable evidence identifier.
#[must_use]
pub fn component_evidence(id: &str) -> Option<&'static ComponentEvidence> {
    COMPONENT_EVIDENCE.iter().find(|evidence| evidence.id == id)
}

/// Returns resolved evidence descriptors for a component taxonomy entry.
pub fn component_evidence_for(
    metadata: &ComponentMetadata,
) -> impl Iterator<Item = &'static ComponentEvidence> + '_ {
    metadata
        .evidence_ids
        .iter()
        .filter_map(|id| component_evidence(id))
}

/// Returns status evidence descriptors for a component taxonomy entry.
pub fn component_status_evidence(
    metadata: &ComponentMetadata,
) -> impl Iterator<Item = &'static ComponentEvidence> + '_ {
    component_evidence_for(metadata)
        .filter(|evidence| evidence.category == ComponentEvidenceCategory::Status)
}

/// Looks up component metadata by exact public name.
#[must_use]
pub fn component_metadata(name: &str) -> Option<&'static ComponentMetadata> {
    COMPONENT_METADATA
        .iter()
        .find(|metadata| metadata.name == name)
}

/// Returns all component metadata entries for a category.
pub fn components_by_category(
    category: ComponentCategory,
) -> impl Iterator<Item = &'static ComponentMetadata> {
    COMPONENT_METADATA
        .iter()
        .filter(move |metadata| metadata.category == category)
}

/// Returns all component metadata entries carrying evidence in a category.
pub fn components_by_evidence_category(
    category: ComponentEvidenceCategory,
) -> impl Iterator<Item = &'static ComponentMetadata> {
    COMPONENT_METADATA.iter().filter(move |metadata| {
        component_evidence_for(metadata).any(|evidence| evidence.category == category)
    })
}
