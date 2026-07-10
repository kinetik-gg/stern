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

/// Evidence-backed conformance status for a component or editor pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ComponentConformanceStatus {
    /// Every declared capability axis has accepted behavioral evidence.
    Stable,
    /// A public surface exists, but one or more declared capability axes remain unproven.
    Experimental,
    /// The component is part of the Kinetik vocabulary but is not yet an active public surface.
    Planned,
}

impl ComponentConformanceStatus {
    /// Returns a stable display name for the status.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Stable => "Stable",
            Self::Experimental => "Experimental",
            Self::Planned => "Planned",
        }
    }

    /// Returns whether the status makes a stable capability claim.
    #[must_use]
    pub const fn is_stable(self) -> bool {
        matches!(self, Self::Stable)
    }
}

/// Independently provable capability axis for a component claim.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ComponentCapabilityAxis {
    /// Public data models, state transitions, or deterministic calculations.
    Model,
    /// Backend-independent visual output or renderer-backed presentation.
    Paint,
    /// Pointer, keyboard, focus, or editing interaction behavior.
    Input,
    /// Semantic output and accessible interaction behavior.
    Accessibility,
    /// Platform integration such as clipboard, IME, cursor, or window services.
    Platform,
    /// End-to-end behavior in a live application workflow.
    LiveWorkflow,
}

impl ComponentCapabilityAxis {
    /// Returns the stable short label used by conformance reports.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Model => "M",
            Self::Paint => "P",
            Self::Input => "I",
            Self::Accessibility => "A11y",
            Self::Platform => "PF",
            Self::LiveWorkflow => "LW",
        }
    }
}

/// Strength of a component evidence descriptor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ComponentEvidenceProof {
    /// Describes inventory, status, stage, or fixture reachability without proving behavior.
    MetadataOnly,
    /// References deterministic behavioral contracts that may prove an explicit axis mapping.
    Behavioral,
}

/// Claim-specific mapping from a capability axis to an attached evidence descriptor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ComponentCapabilityEvidence {
    /// Capability axis proved by the descriptor for this claim.
    pub axis: ComponentCapabilityAxis,
    /// Stable evidence identifier attached to the same claim.
    pub evidence_id: &'static str,
}

impl ComponentCapabilityEvidence {
    /// Creates a claim-specific capability evidence mapping.
    #[must_use]
    pub const fn new(axis: ComponentCapabilityAxis, evidence_id: &'static str) -> Self {
        Self { axis, evidence_id }
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
    /// Whether this descriptor can prove explicitly mapped behavior.
    pub proof: ComponentEvidenceProof,
    /// Short human-readable evidence summary.
    pub summary: &'static str,
}

impl ComponentEvidence {
    /// Creates a component taxonomy evidence descriptor.
    #[must_use]
    pub const fn new(
        id: &'static str,
        category: ComponentEvidenceCategory,
        proof: ComponentEvidenceProof,
        summary: &'static str,
    ) -> Self {
        Self {
            id,
            category,
            proof,
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
    /// Capability axes required before this component can be Stable.
    pub required_axes: &'static [ComponentCapabilityAxis],
    /// Claim-specific mappings from axes to attached behavioral evidence.
    pub capability_evidence: &'static [ComponentCapabilityEvidence],
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
            required_axes: &[],
            capability_evidence: &[],
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

    /// Sets the capability axes required before this component can be Stable.
    #[must_use]
    pub const fn with_required_axes(
        mut self,
        required_axes: &'static [ComponentCapabilityAxis],
    ) -> Self {
        self.required_axes = required_axes;
        self
    }

    /// Sets the claim-specific mappings from axes to attached behavioral evidence.
    #[must_use]
    pub const fn with_capability_evidence(
        mut self,
        capability_evidence: &'static [ComponentCapabilityEvidence],
    ) -> Self {
        self.capability_evidence = capability_evidence;
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
    /// Capability axes required before this row can be Stable.
    pub required_axes: &'static [ComponentCapabilityAxis],
    /// Claim-specific mappings from axes to attached behavioral evidence.
    pub capability_evidence: &'static [ComponentCapabilityEvidence],
}

impl ComponentConformanceMatrixRow {
    /// Creates an Experimental component conformance matrix row.
    #[must_use]
    pub const fn experimental(
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
            status: ComponentConformanceStatus::Experimental,
            stage,
            public_contracts,
            deterministic_tests,
            evidence_ids,
            required_axes: &[],
            capability_evidence: &[],
        }
    }

    /// Associates this matrix row with a component metadata slug.
    #[must_use]
    pub const fn with_component_slug(mut self, component_slug: &'static str) -> Self {
        self.component_slug = Some(component_slug);
        self
    }

    /// Sets the capability axes required before this row can be Stable.
    #[must_use]
    pub const fn with_required_axes(
        mut self,
        required_axes: &'static [ComponentCapabilityAxis],
    ) -> Self {
        self.required_axes = required_axes;
        self
    }

    /// Sets the claim-specific mappings from axes to attached behavioral evidence.
    #[must_use]
    pub const fn with_capability_evidence(
        mut self,
        capability_evidence: &'static [ComponentCapabilityEvidence],
    ) -> Self {
        self.capability_evidence = capability_evidence;
        self
    }
}
