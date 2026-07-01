use std::hash::{Hash, Hasher};

use kinetik_ui_core::{
    DiagnosticCategory, DiagnosticLocation, DiagnosticSeverity as CoreDiagnosticSeverity,
    FrameDiagnostic,
};

use crate::{
    DockPathElement, DockSnapshotDiagnostic, DockSnapshotDiagnostics, DockSnapshotSplitValue,
    FrameId, PanelId, PanelInstanceId, PanelTypeId, SnapshotDiagnosticSeverity,
    WorkspaceSnapshotDiagnostic, WorkspaceSnapshotDiagnostics,
};

/// Stable identity for a diagnostics strip item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DiagnosticStripItemId(u64);

impl DiagnosticStripItemId {
    /// Creates a diagnostics strip item ID from raw bits.
    #[must_use]
    pub const fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    /// Returns the raw ID bits.
    #[must_use]
    pub const fn raw(self) -> u64 {
        self.0
    }
}

/// Severity ordering for diagnostics strip presentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DiagnosticStripSeverity {
    /// Error diagnostics should be presented first.
    Error,
    /// Warning diagnostics follow errors.
    Warning,
    /// Informational diagnostics follow warnings.
    Info,
}

impl DiagnosticStripSeverity {
    const fn sort_rank(self) -> u8 {
        match self {
            Self::Error => 0,
            Self::Warning => 1,
            Self::Info => 2,
        }
    }
}

impl From<CoreDiagnosticSeverity> for DiagnosticStripSeverity {
    fn from(severity: CoreDiagnosticSeverity) -> Self {
        match severity {
            CoreDiagnosticSeverity::Warning => Self::Warning,
        }
    }
}

impl From<SnapshotDiagnosticSeverity> for DiagnosticStripSeverity {
    fn from(severity: SnapshotDiagnosticSeverity) -> Self {
        match severity {
            SnapshotDiagnosticSeverity::Error => Self::Error,
            SnapshotDiagnosticSeverity::Warning => Self::Warning,
        }
    }
}

/// Structured diagnostic source suitable for later debug presentation.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DiagnosticSource {
    /// Core runtime diagnostics.
    Core,
    /// Dock snapshot or dock workspace diagnostics.
    Dock,
    /// Workspace snapshot shell diagnostics.
    Workspace,
    /// Renderer diagnostics.
    Renderer,
    /// Platform adapter diagnostics.
    Platform,
    /// Application-owned diagnostics.
    Application,
    /// Named external or future diagnostic source.
    Other(String),
}

/// Typed diagnostic context value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiagnosticFieldValue {
    /// Application, renderer, platform, or other free-form text.
    Text(String),
    /// Stable unsigned index or count.
    Usize(usize),
    /// Core runtime diagnostic category.
    CoreDiagnosticCategory(DiagnosticCategory),
    /// Core runtime diagnostic location.
    CoreDiagnosticLocation(DiagnosticLocation),
    /// Dock tree path elements.
    DockPath(Vec<DockPathElement>),
    /// Split value identified by dock snapshot validation.
    DockSplitValue(DockSnapshotSplitValue),
    /// Stable frame identity.
    FrameId(FrameId),
    /// Stable panel identity.
    PanelId(PanelId),
    /// Stable panel instance identity.
    PanelInstanceId(PanelInstanceId),
    /// Stable panel type identity.
    PanelTypeId(PanelTypeId),
}

impl Hash for DiagnosticFieldValue {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Self::Text(value) => {
                0_u8.hash(state);
                value.hash(state);
            }
            Self::Usize(value) => {
                1_u8.hash(state);
                value.hash(state);
            }
            Self::CoreDiagnosticCategory(category) => {
                2_u8.hash(state);
                category.hash(state);
            }
            Self::CoreDiagnosticLocation(location) => {
                3_u8.hash(state);
                location.hash(state);
            }
            Self::DockPath(path) => {
                4_u8.hash(state);
                path.hash(state);
            }
            Self::DockSplitValue(value) => {
                5_u8.hash(state);
                match value {
                    DockSnapshotSplitValue::Ratio => 0_u8.hash(state),
                    DockSnapshotSplitValue::MinFirst => 1_u8.hash(state),
                    DockSnapshotSplitValue::MinSecond => 2_u8.hash(state),
                }
            }
            Self::FrameId(frame) => {
                6_u8.hash(state);
                frame.hash(state);
            }
            Self::PanelId(panel) => {
                7_u8.hash(state);
                panel.hash(state);
            }
            Self::PanelInstanceId(panel_instance) => {
                8_u8.hash(state);
                panel_instance.hash(state);
            }
            Self::PanelTypeId(panel_type) => {
                9_u8.hash(state);
                panel_type.hash(state);
            }
        }
    }
}

impl DiagnosticFieldValue {
    /// Returns a presentation string without requiring downstream tools to parse it.
    #[must_use]
    pub fn display_value(&self) -> String {
        match self {
            Self::Text(value) => value.clone(),
            Self::Usize(value) => value.to_string(),
            Self::CoreDiagnosticCategory(category) => format!("{category:?}"),
            Self::CoreDiagnosticLocation(location) => format!("{location:?}"),
            Self::DockPath(path) => format!("{path:?}"),
            Self::DockSplitValue(value) => format!("{value:?}"),
            Self::FrameId(frame) => frame.raw().to_string(),
            Self::PanelId(panel) => panel.raw().to_string(),
            Self::PanelInstanceId(panel_instance) => panel_instance.raw().to_string(),
            Self::PanelTypeId(panel_type) => panel_type.raw().to_string(),
        }
    }
}

impl From<&str> for DiagnosticFieldValue {
    fn from(value: &str) -> Self {
        Self::Text(value.to_owned())
    }
}

impl From<String> for DiagnosticFieldValue {
    fn from(value: String) -> Self {
        Self::Text(value)
    }
}

impl From<usize> for DiagnosticFieldValue {
    fn from(value: usize) -> Self {
        Self::Usize(value)
    }
}

/// Typed diagnostic context field.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DiagnosticField {
    /// Stable field name.
    pub name: String,
    /// Typed field value for downstream tools and presentation.
    pub value: DiagnosticFieldValue,
}

impl DiagnosticField {
    /// Creates a diagnostic context field.
    #[must_use]
    pub fn new(name: impl Into<String>, value: impl Into<DiagnosticFieldValue>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
        }
    }

    /// Returns a presentation string for this field value.
    #[must_use]
    pub fn display_value(&self) -> String {
        self.value.display_value()
    }
}

/// Data-only diagnostics strip item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticStripItem {
    /// Stable diagnostic identity.
    pub id: DiagnosticStripItemId,
    /// Diagnostic severity.
    pub severity: DiagnosticStripSeverity,
    /// Stable diagnostic code.
    pub code: String,
    /// Short diagnostic message or label.
    pub message: String,
    /// Optional typed source metadata.
    pub source: Option<DiagnosticSource>,
    /// Optional typed context fields.
    pub fields: Vec<DiagnosticField>,
}

impl DiagnosticStripItem {
    /// Creates a diagnostics strip item.
    #[must_use]
    pub fn new(
        id: DiagnosticStripItemId,
        severity: DiagnosticStripSeverity,
        code: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            id,
            severity,
            code: code.into(),
            message: message.into(),
            source: None,
            fields: Vec::new(),
        }
    }

    /// Creates a diagnostics strip item from a core frame diagnostic.
    #[must_use]
    pub fn from_frame_diagnostic(id: DiagnosticStripItemId, diagnostic: FrameDiagnostic) -> Self {
        Self::from_frame_diagnostic_ref(id, &diagnostic)
    }

    /// Creates a diagnostics strip item from a borrowed core frame diagnostic.
    #[must_use]
    pub fn from_frame_diagnostic_ref(
        id: DiagnosticStripItemId,
        diagnostic: &FrameDiagnostic,
    ) -> Self {
        Self::new(
            id,
            diagnostic.severity.into(),
            diagnostic.code,
            diagnostic.code,
        )
        .with_source(DiagnosticSource::Core)
        .with_field(
            "category",
            DiagnosticFieldValue::CoreDiagnosticCategory(diagnostic.category),
        )
        .with_field(
            "location",
            DiagnosticFieldValue::CoreDiagnosticLocation(diagnostic.location),
        )
    }

    /// Creates a diagnostics strip item from a dock snapshot diagnostic.
    #[must_use]
    pub fn from_dock_snapshot_diagnostic(
        id: DiagnosticStripItemId,
        diagnostic: &DockSnapshotDiagnostic,
    ) -> Self {
        let mut item = Self::new(
            id,
            diagnostic.severity.into(),
            diagnostic.stable_code(),
            diagnostic.stable_code(),
        )
        .with_source(DiagnosticSource::Dock)
        .with_field(
            "path",
            DiagnosticFieldValue::DockPath(diagnostic.path.elements().to_vec()),
        );

        if let Some(frame) = diagnostic.frame {
            item = item.with_field("frame", DiagnosticFieldValue::FrameId(frame));
        }
        if let Some(panel) = diagnostic.panel {
            item = item.with_field("panel", DiagnosticFieldValue::PanelId(panel));
        }
        if let Some(active_index) = diagnostic.active_index {
            item = item.with_field("active_index", active_index);
        }
        if let Some(panel_count) = diagnostic.panel_count {
            item = item.with_field("panel_count", panel_count);
        }
        if let Some(split_value) = diagnostic.split_value {
            item = item.with_field(
                "split_value",
                DiagnosticFieldValue::DockSplitValue(split_value),
            );
        }

        item
    }

    /// Creates a diagnostics strip item from a workspace snapshot diagnostic.
    #[must_use]
    pub fn from_workspace_snapshot_diagnostic(
        id: DiagnosticStripItemId,
        diagnostic: &WorkspaceSnapshotDiagnostic,
    ) -> Self {
        let mut item = Self::new(
            id,
            diagnostic.severity.into(),
            diagnostic.stable_code(),
            diagnostic.stable_code(),
        )
        .with_source(DiagnosticSource::Workspace);

        if let Some(panel_instance) = diagnostic.panel_instance {
            item = item.with_field(
                "panel_instance",
                DiagnosticFieldValue::PanelInstanceId(panel_instance),
            );
        }
        if let Some(panel_type) = diagnostic.panel_type {
            item = item.with_field("panel_type", DiagnosticFieldValue::PanelTypeId(panel_type));
        }
        if let Some(frame) = diagnostic.frame {
            item = item.with_field("frame", DiagnosticFieldValue::FrameId(frame));
        }
        if let Some(panel) = diagnostic.panel {
            item = item.with_field("panel", DiagnosticFieldValue::PanelId(panel));
        }
        if let Some(dock_title) = &diagnostic.dock_title {
            item = item.with_field("dock_title", dock_title.as_str());
        }
        if let Some(instance_title) = &diagnostic.instance_title {
            item = item.with_field("instance_title", instance_title.as_str());
        }

        item
    }

    /// Sets source metadata.
    #[must_use]
    pub fn with_source(mut self, source: DiagnosticSource) -> Self {
        self.source = Some(source);
        self
    }

    /// Appends a typed context field.
    #[must_use]
    pub fn with_field(
        mut self,
        name: impl Into<String>,
        value: impl Into<DiagnosticFieldValue>,
    ) -> Self {
        self.fields.push(DiagnosticField::new(name, value));
        self
    }

    /// Appends typed context fields.
    #[must_use]
    pub fn with_fields(mut self, fields: impl IntoIterator<Item = DiagnosticField>) -> Self {
        self.fields.extend(fields);
        self
    }
}

/// Summary counts for diagnostics strip presentation.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct DiagnosticStripSummary {
    /// Number of error diagnostics.
    pub errors: u32,
    /// Number of warning diagnostics.
    pub warnings: u32,
    /// Number of informational diagnostics.
    pub info: u32,
}

impl DiagnosticStripSummary {
    /// Returns the total number of diagnostics.
    #[must_use]
    pub const fn total(self) -> u32 {
        self.errors + self.warnings + self.info
    }
}

/// Data-only diagnostics strip model.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DiagnosticStrip {
    items: Vec<DiagnosticStripItem>,
}

impl DiagnosticStrip {
    /// Creates an empty diagnostics strip.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a diagnostics strip from item definitions.
    #[must_use]
    pub fn from_items(items: impl IntoIterator<Item = DiagnosticStripItem>) -> Self {
        Self {
            items: items.into_iter().collect(),
        }
    }

    /// Returns diagnostics in insertion order.
    #[must_use]
    pub fn items(&self) -> &[DiagnosticStripItem] {
        &self.items
    }

    /// Replaces diagnostics.
    pub fn replace_items(&mut self, items: impl IntoIterator<Item = DiagnosticStripItem>) {
        self.items = items.into_iter().collect();
    }

    /// Appends one diagnostic in aggregation order.
    pub fn push_item(&mut self, item: DiagnosticStripItem) {
        self.items.push(item);
    }

    /// Appends diagnostics in aggregation order.
    pub fn extend_items(&mut self, items: impl IntoIterator<Item = DiagnosticStripItem>) {
        self.items.extend(items);
    }

    /// Appends core frame diagnostics using deterministic IDs from `first_id`.
    pub fn extend_frame_diagnostics(
        &mut self,
        first_id: DiagnosticStripItemId,
        diagnostics: impl IntoIterator<Item = FrameDiagnostic>,
    ) {
        self.items.extend(
            diagnostics
                .into_iter()
                .enumerate()
                .map(|(index, diagnostic)| {
                    DiagnosticStripItem::from_frame_diagnostic(
                        offset_diagnostic_id(first_id, index),
                        diagnostic,
                    )
                }),
        );
    }

    /// Appends borrowed core frame diagnostics using deterministic IDs from `first_id`.
    pub fn extend_frame_diagnostics_ref<'a>(
        &mut self,
        first_id: DiagnosticStripItemId,
        diagnostics: impl IntoIterator<Item = &'a FrameDiagnostic>,
    ) {
        self.items.extend(
            diagnostics
                .into_iter()
                .enumerate()
                .map(|(index, diagnostic)| {
                    DiagnosticStripItem::from_frame_diagnostic_ref(
                        offset_diagnostic_id(first_id, index),
                        diagnostic,
                    )
                }),
        );
    }

    /// Appends dock snapshot diagnostics in their deterministic validation order.
    pub fn extend_dock_snapshot_diagnostics(
        &mut self,
        first_id: DiagnosticStripItemId,
        diagnostics: &DockSnapshotDiagnostics,
    ) {
        self.items.extend(
            diagnostics
                .diagnostics
                .iter()
                .enumerate()
                .map(|(index, diagnostic)| {
                    DiagnosticStripItem::from_dock_snapshot_diagnostic(
                        offset_diagnostic_id(first_id, index),
                        diagnostic,
                    )
                }),
        );
    }

    /// Appends workspace diagnostics as dock diagnostics followed by workspace-shell diagnostics.
    pub fn extend_workspace_snapshot_diagnostics(
        &mut self,
        first_id: DiagnosticStripItemId,
        diagnostics: &WorkspaceSnapshotDiagnostics,
    ) {
        self.extend_dock_snapshot_diagnostics(first_id, &diagnostics.dock);
        let workspace_first_id = offset_diagnostic_id(first_id, diagnostics.dock.diagnostics.len());
        self.items.extend(
            diagnostics
                .workspace
                .iter()
                .enumerate()
                .map(|(index, diagnostic)| {
                    DiagnosticStripItem::from_workspace_snapshot_diagnostic(
                        offset_diagnostic_id(workspace_first_id, index),
                        diagnostic,
                    )
                }),
        );
    }

    /// Returns a diagnostic by stable identity.
    #[must_use]
    pub fn item(&self, id: DiagnosticStripItemId) -> Option<&DiagnosticStripItem> {
        self.items.iter().find(|item| item.id == id)
    }

    /// Returns diagnostics ordered by severity while preserving insertion order within severity.
    #[must_use]
    pub fn ordered_items(&self) -> Vec<&DiagnosticStripItem> {
        let mut ordered = self.items.iter().enumerate().collect::<Vec<_>>();
        ordered.sort_by_key(|(index, item)| (item.severity.sort_rank(), *index));
        ordered.into_iter().map(|(_, item)| item).collect()
    }

    /// Returns deterministic severity counts.
    #[must_use]
    pub fn summary(&self) -> DiagnosticStripSummary {
        let mut summary = DiagnosticStripSummary::default();
        for item in &self.items {
            match item.severity {
                DiagnosticStripSeverity::Error => summary.errors += 1,
                DiagnosticStripSeverity::Warning => summary.warnings += 1,
                DiagnosticStripSeverity::Info => summary.info += 1,
            }
        }
        summary
    }
}

fn offset_diagnostic_id(id: DiagnosticStripItemId, offset: usize) -> DiagnosticStripItemId {
    let offset = u64::try_from(offset).unwrap_or(u64::MAX);
    DiagnosticStripItemId::from_raw(id.raw().saturating_add(offset))
}
