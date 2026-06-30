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

/// Data-only conformance matrix row for a spec-stage capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ComponentConformanceMatrixRow {
    /// Public capability or pattern name.
    pub capability: &'static str,
    /// Stable lower-kebab capability identifier.
    pub slug: &'static str,
    /// Optional component slug this capability supports.
    pub component_slug: Option<&'static str>,
    /// Kinetik-owned category.
    pub category: ComponentCategory,
    /// Honest implementation status for the capability.
    pub status: ComponentConformanceStatus,
    /// Restarted editor-toolkit stage that owns this matrix row.
    pub stage: u8,
    /// Public data-only contracts that provide the capability surface.
    pub public_contracts: &'static [&'static str],
    /// Deterministic tests that prove the evidence claim.
    pub deterministic_tests: &'static [&'static str],
    /// Stable evidence identifiers backing the row.
    pub evidence_ids: &'static [&'static str],
}

impl ComponentConformanceMatrixRow {
    /// Creates a partial component conformance matrix row.
    #[must_use]
    pub const fn partial(
        capability: &'static str,
        slug: &'static str,
        category: ComponentCategory,
        stage: u8,
        public_contracts: &'static [&'static str],
        deterministic_tests: &'static [&'static str],
        evidence_ids: &'static [&'static str],
    ) -> Self {
        Self {
            capability,
            slug,
            component_slug: None,
            category,
            status: ComponentConformanceStatus::Partial,
            stage,
            public_contracts,
            deterministic_tests,
            evidence_ids,
        }
    }

    /// Associates this matrix row with a component metadata slug.
    #[must_use]
    pub const fn with_component_slug(mut self, component_slug: &'static str) -> Self {
        self.component_slug = Some(component_slug);
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
const S10_OUTLINER_EVIDENCE: &[&str] = &[
    "status.partial-public-contract",
    "stage.10-outliner-asset-browser",
    "conformance.outliner-contracts",
    "showcase.metadata-only",
];
const S10_ASSET_BROWSER_EVIDENCE: &[&str] = &[
    "status.partial-public-contract",
    "stage.10-outliner-asset-browser",
    "conformance.asset-browser-contracts",
    "showcase.metadata-only",
];
const S10_INLINE_EDIT_EVIDENCE: &[&str] = &[
    "status.partial-public-contract",
    "stage.10-outliner-asset-browser",
    "conformance.inline-edit-contracts",
    "showcase.metadata-only",
];
const S10_COLLECTION_DRAG_CONTEXT_EVIDENCE: &[&str] = &[
    "status.partial-public-contract",
    "stage.10-outliner-asset-browser",
    "conformance.collection-drag-context-contracts",
    "showcase.metadata-only",
];
const S10_COLLECTION_PROJECTION_EVIDENCE: &[&str] = &[
    "status.partial-public-contract",
    "stage.10-outliner-asset-browser",
    "conformance.collection-projection-contracts",
    "showcase.metadata-only",
];
const S11_TIMELINE_EVIDENCE: &[&str] = &[
    "status.partial-public-contract",
    "stage.11-timeline",
    "conformance.timeline-contracts",
    "showcase.metadata-only",
];
const S11_RULER_EVIDENCE: &[&str] = &[
    "status.partial-public-contract",
    "stage.11-timeline",
    "conformance.timeline-ruler-contracts",
    "showcase.metadata-only",
];
const S11_TRANSPORT_EVIDENCE: &[&str] = &[
    "status.partial-public-contract",
    "stage.11-timeline",
    "conformance.timeline-transport-contracts",
    "showcase.metadata-only",
];
const S11_SNAPPING_EVIDENCE: &[&str] = &[
    "status.partial-public-contract",
    "stage.11-timeline",
    "conformance.timeline-snapping-contracts",
    "showcase.metadata-only",
];
const S11_PRESERVATION_EVIDENCE: &[&str] = &[
    "status.partial-public-contract",
    "stage.11-timeline",
    "conformance.timeline-preservation-contracts",
    "showcase.metadata-only",
];

const S10_OUTLINER_CONTRACTS: &[&str] = &[
    "OutlinerModel",
    "OutlinerLayout",
    "OutlinerRow",
    "TreeExpansion",
    "Selection",
    "outliner_semantics",
];
const S10_ASSET_BROWSER_CONTRACTS: &[&str] = &[
    "AssetBrowserModel",
    "AssetBrowserLayout",
    "AssetBrowserViewMode",
    "AssetBrowserResolvedItem",
    "asset_browser_semantics",
];
const S10_INLINE_EDIT_CONTRACTS: &[&str] = &[
    "InlineEditSession",
    "InlineEditRequest",
    "InlineEditDraftPolicy",
    "inline_edit_widget_id",
];
const S10_COLLECTION_DRAG_CONTEXT_CONTRACTS: &[&str] = &[
    "CollectionDragSource",
    "CollectionContextTarget",
    "collection_context_actions",
    "OutlinerDropTarget",
    "AssetBrowserDropTarget",
];
const S10_COLLECTION_PROJECTION_CONTRACTS: &[&str] = &[
    "CollectionProjection",
    "SelectionProjectionPolicy",
    "AssetBrowserSort",
    "AssetBrowserSortKey",
    "Selection",
];
const S11_TIMELINE_CONTRACTS: &[&str] = &[
    "TimelineDescriptor",
    "TimelineLayout",
    "TimelineScale",
    "TimelineSelection",
    "timeline_semantics",
];
const S11_RULER_CONTRACTS: &[&str] = &[
    "TimelineRulerTickRequest",
    "TimelineFrameRate",
    "TimelineFrame",
    "timeline_timecode_label",
];
const S11_TRANSPORT_CONTRACTS: &[&str] = &[
    "TransportControls",
    "TransportControlDescriptor",
    "TimelineTransportContext",
    "transport_controls_semantics",
];
const S11_SNAPPING_CONTRACTS: &[&str] = &[
    "TimelineSnapCandidateRequest",
    "TimelineSnapCandidate",
    "TimelineSnapMetadata",
    "TimelineSnapSource",
    "timeline_snap_candidates",
    "timeline_snap_time",
];
const S11_PRESERVATION_CONTRACTS: &[&str] = &[
    "TimelineViewportState",
    "TimelineSelection",
    "TimelineRange",
    "TimelineSnapMetadata",
    "TimelineScale::zoom_around_anchor",
];

const S10_OUTLINER_TESTS: &[&str] = &[
    "outliner_conformance::tree_visible_row_order_is_deterministic",
    "outliner_conformance::expansion_preservation_is_deterministic",
    "outliner_conformance::semantics_and_list_metadata_are_stable",
];
const S10_ASSET_BROWSER_TESTS: &[&str] = &[
    "asset_browser_conformance::grid_layout_resolves_deterministic_materialized_items",
    "asset_browser_conformance::list_row_rectangles_are_stable",
    "asset_browser_conformance::semantics_preserve_view_mode_selection_and_disabled_state",
];
const S10_INLINE_EDIT_TESTS: &[&str] = &[
    "inline_edit_conformance::rename_starts_from_selected_outliner_item_and_preserves_selection",
    "inline_edit_conformance::draft_edit_commit_cancel_and_focus_loss_requests_are_deterministic",
    "inline_edit_conformance::outliner_and_asset_semantics_expose_rename_only_when_available",
];
const S10_COLLECTION_DRAG_CONTEXT_TESTS: &[&str] = &[
    "collection_drag_context_conformance::drag_source_identity_is_stable_and_selection_aware",
    "collection_drag_context_conformance::asset_browser_resolves_item_and_empty_space_drop_targets",
    "collection_drag_context_conformance::context_action_requests_are_metadata_only_and_selection_aware",
];
const S10_COLLECTION_PROJECTION_TESTS: &[&str] = &[
    "collection_projection_conformance::selected_ids_survive_filtering_out_and_back_in",
    "collection_projection_conformance::asset_sorting_uses_app_provided_keys_and_source_indices",
    "collection_projection_conformance::asset_grid_and_list_selection_survives_view_mode_switch",
];
const S11_TIMELINE_TESTS: &[&str] = &[
    "timeline_conformance::time_and_frame_screen_conversions_round_trip",
    "timeline_conformance::timeline_lane_visible_and_materialized_ranges_are_deterministic",
    "timeline_conformance::timeline_descriptor_state_and_semantics_are_exposed_without_renderer_dependencies",
];
const S11_RULER_TESTS: &[&str] = &[
    "timeline_conformance::ruler_ticks_are_deterministic_finite_and_ordered",
    "timeline_conformance::ruler_ticks_respect_max_tick_bound_for_large_ranges",
    "timeline_conformance::timecode_labels_are_stable_for_positive_negative_and_fractional_rates",
];
const S11_TRANSPORT_TESTS: &[&str] = &[
    "timeline_transport_conformance::transport_visible_controls_preserve_descriptor_order_and_omit_hidden_actions",
    "timeline_transport_conformance::transport_request_metadata_preserves_action_source_kind_and_timeline_context",
    "timeline_transport_conformance::transport_descriptors_reuse_action_surface_contracts_without_command_duplication",
];
const S11_SNAPPING_TESTS: &[&str] = &[
    "timeline_conformance::snap_candidates_include_grid_markers_keyframes_clip_edges_and_range_boundaries",
    "timeline_conformance::snap_metadata_reports_snapped_and_unsnapped_time_with_source_identity",
    "timeline_conformance::snap_resolution_uses_deterministic_priority_and_tie_breaking",
];
const S11_PRESERVATION_TESTS: &[&str] = &[
    "timeline_conformance::selection_targets_survive_lane_reorder_and_scroll_changes_by_stable_id",
    "timeline_conformance::range_selection_metadata_survives_zoom_and_normalizes_deterministically",
    "timeline_conformance::scroll_clamping_preserves_playhead_range_and_snap_metadata",
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
        "conformance.outliner-contracts",
        Conformance,
        "Covered by public outliner data contracts and deterministic outliner conformance tests.",
    ),
    ComponentEvidence::new(
        "conformance.asset-browser-contracts",
        Conformance,
        "Covered by public asset browser data contracts and deterministic asset browser conformance tests.",
    ),
    ComponentEvidence::new(
        "conformance.inline-edit-contracts",
        Conformance,
        "Covered by public inline edit session contracts and deterministic rename conformance tests.",
    ),
    ComponentEvidence::new(
        "conformance.collection-drag-context-contracts",
        Conformance,
        "Covered by public collection drag/drop/context contracts and deterministic conformance tests.",
    ),
    ComponentEvidence::new(
        "conformance.collection-projection-contracts",
        Conformance,
        "Covered by public collection projection, filter, sort, and selection preservation contracts.",
    ),
    ComponentEvidence::new(
        "conformance.timeline-contracts",
        Conformance,
        "Covered by public timeline layout, coordinate, selection, and semantic contracts.",
    ),
    ComponentEvidence::new(
        "conformance.timeline-ruler-contracts",
        Conformance,
        "Covered by public timeline ruler tick and timecode contracts.",
    ),
    ComponentEvidence::new(
        "conformance.timeline-transport-contracts",
        Conformance,
        "Covered by public timeline transport action and semantic contracts.",
    ),
    ComponentEvidence::new(
        "conformance.timeline-snapping-contracts",
        Conformance,
        "Covered by public timeline snap candidate and snap resolution contracts.",
    ),
    ComponentEvidence::new(
        "conformance.timeline-preservation-contracts",
        Conformance,
        "Covered by public timeline viewport state and selection preservation contracts.",
    ),
    ComponentEvidence::new(
        "showcase.metadata-only",
        Showcase,
        "Tracked by taxonomy metadata only; no interactive showcase behavior is claimed.",
    ),
];

/// Data-only conformance matrix for restarted editor-toolkit S10-S11 capabilities.
pub const COMPONENT_CONFORMANCE_MATRIX: &[ComponentConformanceMatrixRow] = &[
    ComponentConformanceMatrixRow::partial(
        "Outliner tree, zones, selection, and semantics",
        "s10-outliner-tree-selection-semantics",
        Collection,
        10,
        S10_OUTLINER_CONTRACTS,
        S10_OUTLINER_TESTS,
        S10_OUTLINER_EVIDENCE,
    )
    .with_component_slug("outliner"),
    ComponentConformanceMatrixRow::partial(
        "Asset browser grid/list layout and metadata",
        "s10-asset-browser-grid-list-metadata",
        Collection,
        10,
        S10_ASSET_BROWSER_CONTRACTS,
        S10_ASSET_BROWSER_TESTS,
        S10_ASSET_BROWSER_EVIDENCE,
    )
    .with_component_slug("asset-browser"),
    ComponentConformanceMatrixRow::partial(
        "Inline edit rename lifecycle",
        "s10-inline-edit-rename-lifecycle",
        TextEditing,
        10,
        S10_INLINE_EDIT_CONTRACTS,
        S10_INLINE_EDIT_TESTS,
        S10_INLINE_EDIT_EVIDENCE,
    ),
    ComponentConformanceMatrixRow::partial(
        "Collection drag, drop, and context routing",
        "s10-collection-drag-drop-context",
        Collection,
        10,
        S10_COLLECTION_DRAG_CONTEXT_CONTRACTS,
        S10_COLLECTION_DRAG_CONTEXT_TESTS,
        S10_COLLECTION_DRAG_CONTEXT_EVIDENCE,
    ),
    ComponentConformanceMatrixRow::partial(
        "Collection filter, sort, and selection preservation",
        "s10-collection-filter-sort-selection-preservation",
        Collection,
        10,
        S10_COLLECTION_PROJECTION_CONTRACTS,
        S10_COLLECTION_PROJECTION_TESTS,
        S10_COLLECTION_PROJECTION_EVIDENCE,
    ),
    ComponentConformanceMatrixRow::partial(
        "Timeline layout, coordinates, selection, and semantics",
        "s11-timeline-layout-coordinate-selection",
        Viewport,
        11,
        S11_TIMELINE_CONTRACTS,
        S11_TIMELINE_TESTS,
        S11_TIMELINE_EVIDENCE,
    )
    .with_component_slug("timeline"),
    ComponentConformanceMatrixRow::partial(
        "Timeline ruler ticks and timecode labels",
        "s11-ruler-ticks-timecode",
        Viewport,
        11,
        S11_RULER_CONTRACTS,
        S11_RULER_TESTS,
        S11_RULER_EVIDENCE,
    )
    .with_component_slug("ruler"),
    ComponentConformanceMatrixRow::partial(
        "Timeline transport action controls",
        "s11-transport-action-controls",
        Control,
        11,
        S11_TRANSPORT_CONTRACTS,
        S11_TRANSPORT_TESTS,
        S11_TRANSPORT_EVIDENCE,
    )
    .with_component_slug("transport-controls"),
    ComponentConformanceMatrixRow::partial(
        "Timeline snapping candidates and resolution",
        "s11-timeline-snapping",
        Viewport,
        11,
        S11_SNAPPING_CONTRACTS,
        S11_SNAPPING_TESTS,
        S11_SNAPPING_EVIDENCE,
    )
    .with_component_slug("timeline"),
    ComponentConformanceMatrixRow::partial(
        "Timeline interaction state preservation",
        "s11-timeline-preservation",
        Viewport,
        11,
        S11_PRESERVATION_CONTRACTS,
        S11_PRESERVATION_TESTS,
        S11_PRESERVATION_EVIDENCE,
    )
    .with_component_slug("timeline"),
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

/// Looks up a conformance matrix row by exact stable row slug.
#[must_use]
pub fn component_conformance_matrix_row(
    slug: &str,
) -> Option<&'static ComponentConformanceMatrixRow> {
    COMPONENT_CONFORMANCE_MATRIX
        .iter()
        .find(|row| row.slug == slug)
}

/// Returns all conformance matrix rows for a restarted editor-toolkit stage.
pub fn component_conformance_matrix_by_stage(
    stage: u8,
) -> impl Iterator<Item = &'static ComponentConformanceMatrixRow> {
    COMPONENT_CONFORMANCE_MATRIX
        .iter()
        .filter(move |row| row.stage == stage)
}
