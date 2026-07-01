mod validation;

use validation::{plan_workspace_snapshot_repair, validate_workspace_snapshot};
pub(crate) use validation::{restore_node, snapshot_node, validate_dock_snapshot};
pub use validation::{validate_dock_snapshot_diagnostics, validate_workspace_snapshot_diagnostics};

use super::{
    Axis, Dock, DockSplitPath, FrameId, Panel, PanelId, PanelInstanceId, PanelTypeDescriptor,
    PanelTypeId,
};
/// Persistable dock snapshot.
#[derive(Debug, Clone, PartialEq)]
pub struct DockSnapshot {
    /// Root-owned active frame identity.
    pub active_frame: Option<FrameId>,
    /// Root snapshot node.
    pub root: DockSnapshotNode,
}

impl DockSnapshot {
    /// Returns structured diagnostics for this snapshot.
    #[must_use]
    pub fn diagnostics(&self) -> DockSnapshotDiagnostics {
        validate_dock_snapshot_diagnostics(self)
    }
}

/// Snapshot node.
#[derive(Debug, Clone, PartialEq)]
pub enum DockSnapshotNode {
    /// Frame snapshot.
    Frame {
        /// Frame identity.
        id: FrameId,
        /// Panels.
        panels: Vec<Panel>,
        /// Active panel index.
        active: usize,
        /// Panels whose frame tabs expose close/dismiss affordances.
        dismissible_panels: Vec<PanelId>,
    },
    /// Split snapshot.
    Split {
        /// Split axis.
        axis: Axis,
        /// First child ratio.
        ratio: f32,
        /// Minimum first size.
        min_first: f32,
        /// Minimum second size.
        min_second: f32,
        /// First child.
        first: Box<DockSnapshotNode>,
        /// Second child.
        second: Box<DockSnapshotNode>,
    },
}

/// Snapshot restore error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DockRestoreError {
    /// Frame contains no panels.
    EmptyFrame,
    /// Active tab index is outside the panel list.
    InvalidActiveIndex,
    /// Two frames use the same stable frame identity.
    DuplicateFrameId,
    /// Two panels use the same stable panel identity.
    DuplicatePanelId,
    /// Dismissible panel policy references a panel missing from the frame.
    InvalidDismissiblePanel,
    /// Dismissible panel policy contains the same panel more than once.
    DuplicateDismissiblePanel,
    /// Active frame identity references a frame missing from the dock tree.
    InvalidActiveFrame,
    /// Split ratio is not finite or is outside the inclusive 0.0..=1.0 range.
    InvalidSplitRatio,
    /// Split minimum is not finite or is negative.
    InvalidSplitMinimum,
}

/// Snapshot diagnostic severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotDiagnosticSeverity {
    /// Validation error that prevents restore.
    Error,
    /// Non-fatal issue that should be visible to debug tooling.
    Warning,
}

/// Stable diagnostic code for dock snapshot validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DockSnapshotDiagnosticCode {
    /// Frame contains no panels.
    EmptyFrame,
    /// Two frames use the same stable frame identity.
    DuplicateFrameId,
    /// Two panels use the same stable panel identity.
    DuplicatePanelId,
    /// Active frame identity references a frame missing from the dock tree.
    InvalidActiveFrame,
    /// Active tab index is outside the panel list.
    InvalidActivePanelIndex,
    /// Split ratio is not finite or is outside the inclusive 0.0..=1.0 range.
    InvalidSplitRatio,
    /// Split minimum is not finite or is negative.
    InvalidSplitMinimum,
    /// Dismissible panel policy references a panel missing from the frame.
    InvalidDismissiblePanel,
    /// Dismissible panel policy contains the same panel more than once.
    DuplicateDismissiblePolicy,
}

impl DockSnapshotDiagnosticCode {
    /// Returns the stable string code for this diagnostic.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::EmptyFrame => "dock.empty_frame",
            Self::DuplicateFrameId => "dock.duplicate_frame_id",
            Self::DuplicatePanelId => "dock.duplicate_panel_id",
            Self::InvalidActiveFrame => "dock.invalid_active_frame",
            Self::InvalidActivePanelIndex => "dock.invalid_active_panel_index",
            Self::InvalidSplitRatio => "dock.invalid_split_ratio",
            Self::InvalidSplitMinimum => "dock.invalid_split_minimum",
            Self::InvalidDismissiblePanel => "dock.invalid_dismissible_panel",
            Self::DuplicateDismissiblePolicy => "dock.duplicate_dismissible_policy",
        }
    }
}

/// Split value identified by a dock snapshot diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DockSnapshotSplitValue {
    /// Split ratio.
    Ratio,
    /// Minimum size for the first child.
    MinFirst,
    /// Minimum size for the second child.
    MinSecond,
}

/// Structured diagnostic for dock snapshot validation.
#[derive(Debug, Clone, PartialEq)]
pub struct DockSnapshotDiagnostic {
    /// Stable diagnostic code.
    pub code: DockSnapshotDiagnosticCode,
    /// Diagnostic severity.
    pub severity: SnapshotDiagnosticSeverity,
    /// Tree path to the split or frame where the diagnostic was found.
    pub path: DockSplitPath,
    /// Frame identity when the diagnostic is frame-scoped.
    pub frame: Option<FrameId>,
    /// Panel identity when the diagnostic is panel-scoped.
    pub panel: Option<PanelId>,
    /// Invalid active panel index when applicable.
    pub active_index: Option<usize>,
    /// Panel count used to judge an active panel index.
    pub panel_count: Option<usize>,
    /// Split value involved in split diagnostics.
    pub split_value: Option<DockSnapshotSplitValue>,
}

impl DockSnapshotDiagnostic {
    pub(super) fn new(code: DockSnapshotDiagnosticCode, path: DockSplitPath) -> Self {
        Self {
            code,
            severity: SnapshotDiagnosticSeverity::Error,
            path,
            frame: None,
            panel: None,
            active_index: None,
            panel_count: None,
            split_value: None,
        }
    }

    /// Returns the stable string code for this diagnostic.
    #[must_use]
    pub const fn stable_code(&self) -> &'static str {
        self.code.as_str()
    }
}

/// Structured dock snapshot diagnostics.
#[derive(Debug, Clone, PartialEq)]
pub struct DockSnapshotDiagnostics {
    /// Diagnostics in deterministic validation order.
    pub diagnostics: Vec<DockSnapshotDiagnostic>,
}

impl DockSnapshotDiagnostics {
    /// Returns true when no error diagnostics were emitted.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        !self.has_errors()
    }

    /// Returns true when at least one error diagnostic was emitted.
    #[must_use]
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == SnapshotDiagnosticSeverity::Error)
    }
}

/// Persistable metadata for one open panel instance.
///
/// This keeps the workspace shell typed while leaving panel content,
/// application state serialization, and factory behavior application-owned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PanelInstanceSnapshot {
    /// Stable identity for one open panel instance.
    pub id: PanelInstanceId,
    /// Developer-declared panel type identity for this instance.
    pub panel_type: PanelTypeId,
    /// Display title used by workspace tabs or persisted custom labels.
    pub title: String,
    /// Optional application-owned key for looking up persisted panel state.
    pub state_key: Option<String>,
}

impl PanelInstanceSnapshot {
    /// Creates a panel instance snapshot.
    #[must_use]
    pub fn new(id: PanelInstanceId, panel_type: PanelTypeId, title: impl Into<String>) -> Self {
        Self {
            id,
            panel_type,
            title: title.into(),
            state_key: None,
        }
    }

    /// Sets the optional application-owned state key.
    #[must_use]
    pub fn with_state_key(mut self, state_key: impl Into<String>) -> Self {
        self.state_key = Some(state_key.into());
        self
    }
}

/// Additive workspace snapshot shell around a dock snapshot.
///
/// `DockSnapshot` remains usable on its own. This type adds enough typed
/// metadata for applications to validate panel instance references without
/// introducing panel factories or app state serialization into the widget layer.
#[derive(Debug, Clone, PartialEq)]
pub struct WorkspaceSnapshot {
    /// Persisted dock tree and active frame state.
    pub dock: DockSnapshot,
    /// Persisted open panel instance records.
    pub panel_instances: Vec<PanelInstanceSnapshot>,
}

impl WorkspaceSnapshot {
    /// Creates a workspace snapshot shell.
    #[must_use]
    pub const fn new(dock: DockSnapshot, panel_instances: Vec<PanelInstanceSnapshot>) -> Self {
        Self {
            dock,
            panel_instances,
        }
    }

    /// Validates the workspace shell against supplied panel type descriptors.
    ///
    /// # Errors
    ///
    /// Returns [`WorkspaceRestoreError`] for invalid dock snapshots, duplicate
    /// descriptors, duplicate instances, missing records, unknown panel types,
    /// or stale records.
    pub fn validate(
        &self,
        descriptors: &[PanelTypeDescriptor],
    ) -> Result<(), WorkspaceRestoreError> {
        let dock_validation = validate_dock_snapshot(&self.dock)?;
        validate_workspace_snapshot(self, descriptors, &dock_validation)
    }

    /// Returns structured diagnostics for this workspace snapshot.
    #[must_use]
    pub fn diagnostics(&self, descriptors: &[PanelTypeDescriptor]) -> WorkspaceSnapshotDiagnostics {
        validate_workspace_snapshot_diagnostics(self, descriptors)
    }

    /// Returns an explicit metadata-only repair plan for this workspace snapshot.
    ///
    /// The plan keeps strict validation unchanged: hard identity or dock
    /// corruption yields no repaired snapshot. Recoverable stale, missing, or
    /// unknown panel metadata remains visible through diagnostics and actions.
    #[must_use]
    pub fn repair_plan(&self, descriptors: &[PanelTypeDescriptor]) -> WorkspaceRepairPlan {
        plan_workspace_snapshot_repair(self, descriptors)
    }

    /// Returns the metadata-only repaired workspace snapshot when planning found
    /// no hard repair error.
    ///
    /// # Errors
    ///
    /// Returns [`WorkspaceRestoreError`] when the dock snapshot is invalid,
    /// duplicate identity metadata exists, or a missing panel instance cannot be
    /// represented by safe placeholder metadata.
    pub fn repair_snapshot(
        &self,
        descriptors: &[PanelTypeDescriptor],
    ) -> Result<WorkspaceSnapshot, WorkspaceRestoreError> {
        self.repair_plan(descriptors).into_repaired_snapshot()
    }

    /// Validates this workspace snapshot and restores its dock.
    ///
    /// # Errors
    ///
    /// Returns [`WorkspaceRestoreError`] when validation fails or dock restore
    /// rejects the dock snapshot.
    pub fn restore_dock(
        self,
        descriptors: &[PanelTypeDescriptor],
    ) -> Result<Dock, WorkspaceRestoreError> {
        self.validate(descriptors)?;
        Dock::restore(self.dock).map_err(WorkspaceRestoreError::Dock)
    }
}

/// Workspace snapshot validation and restore error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceRestoreError {
    /// The wrapped dock snapshot is invalid.
    Dock(DockRestoreError),
    /// A dock panel does not have a matching panel instance record.
    MissingPanelInstance {
        /// Missing panel instance identity.
        panel_instance: PanelInstanceId,
    },
    /// A panel instance references a panel type absent from the supplied descriptors.
    UnknownPanelType {
        /// Panel instance with the unknown type.
        panel_instance: PanelInstanceId,
        /// Unknown panel type identity.
        panel_type: PanelTypeId,
    },
    /// Two panel instance records use the same stable identity.
    DuplicatePanelInstanceId {
        /// Duplicated panel instance identity.
        panel_instance: PanelInstanceId,
    },
    /// Two panel type descriptors use the same stable identity.
    DuplicatePanelTypeDescriptor {
        /// Duplicated panel type identity.
        panel_type: PanelTypeId,
    },
    /// A panel instance record is not referenced by the dock snapshot.
    StalePanelInstance {
        /// Stale panel instance identity.
        panel_instance: PanelInstanceId,
    },
}

/// Stable diagnostic code for workspace snapshot validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceSnapshotDiagnosticCode {
    /// Two panel instance records use the same stable identity.
    DuplicatePanelInstanceId,
    /// Two panel type descriptors use the same stable identity.
    DuplicatePanelTypeDescriptor,
    /// A dock panel does not have a matching panel instance record.
    MissingPanelInstance,
    /// A panel instance record is not referenced by the dock snapshot.
    StalePanelInstance,
    /// A panel instance references a panel type absent from the supplied descriptors.
    UnknownPanelType,
    /// A dock tab title differs from its panel instance title.
    PanelTitleDrift,
}

impl WorkspaceSnapshotDiagnosticCode {
    /// Returns the stable string code for this diagnostic.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::DuplicatePanelInstanceId => "workspace.duplicate_panel_instance_id",
            Self::DuplicatePanelTypeDescriptor => "workspace.duplicate_panel_type_descriptor",
            Self::MissingPanelInstance => "workspace.missing_panel_instance",
            Self::StalePanelInstance => "workspace.stale_panel_instance",
            Self::UnknownPanelType => "workspace.unknown_panel_type",
            Self::PanelTitleDrift => "workspace.panel_title_drift",
        }
    }
}

/// Structured diagnostic for workspace snapshot validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceSnapshotDiagnostic {
    /// Stable diagnostic code.
    pub code: WorkspaceSnapshotDiagnosticCode,
    /// Diagnostic severity.
    pub severity: SnapshotDiagnosticSeverity,
    /// Panel instance identity when the diagnostic is instance-scoped.
    pub panel_instance: Option<PanelInstanceId>,
    /// Panel type identity when the diagnostic is type-scoped.
    pub panel_type: Option<PanelTypeId>,
    /// Dock frame containing the panel instance when known.
    pub frame: Option<FrameId>,
    /// Legacy dock panel identity when known.
    pub panel: Option<PanelId>,
    /// Title stored on the dock panel when relevant.
    pub dock_title: Option<String>,
    /// Title stored on the panel instance when relevant.
    pub instance_title: Option<String>,
}

impl WorkspaceSnapshotDiagnostic {
    pub(super) fn new(code: WorkspaceSnapshotDiagnosticCode) -> Self {
        Self {
            code,
            severity: SnapshotDiagnosticSeverity::Error,
            panel_instance: None,
            panel_type: None,
            frame: None,
            panel: None,
            dock_title: None,
            instance_title: None,
        }
    }

    /// Returns the stable string code for this diagnostic.
    #[must_use]
    pub const fn stable_code(&self) -> &'static str {
        self.code.as_str()
    }
}

/// Structured workspace snapshot diagnostics.
#[derive(Debug, Clone, PartialEq)]
pub struct WorkspaceSnapshotDiagnostics {
    /// Diagnostics for the wrapped dock snapshot.
    pub dock: DockSnapshotDiagnostics,
    /// Diagnostics for the workspace panel instance shell.
    pub workspace: Vec<WorkspaceSnapshotDiagnostic>,
}

impl WorkspaceSnapshotDiagnostics {
    /// Returns true when no error diagnostics were emitted.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        !self.has_errors()
    }

    /// Returns true when at least one error diagnostic was emitted.
    #[must_use]
    pub fn has_errors(&self) -> bool {
        self.dock.has_errors()
            || self
                .workspace
                .iter()
                .any(|diagnostic| diagnostic.severity == SnapshotDiagnosticSeverity::Error)
    }
}

/// Stable repair action code for workspace snapshot repair planning.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceRepairActionCode {
    /// A missing panel instance record was filled with placeholder metadata.
    AddMissingPanelInstancePlaceholder,
    /// A panel instance record not referenced by the dock snapshot was dropped.
    DropStalePanelInstance,
    /// An unknown panel type was preserved as explicit unresolved metadata.
    KeepUnknownPanelType,
}

impl WorkspaceRepairActionCode {
    /// Returns the stable string code for this repair action.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AddMissingPanelInstancePlaceholder => {
                "workspace_repair.add_missing_panel_instance_placeholder"
            }
            Self::DropStalePanelInstance => "workspace_repair.drop_stale_panel_instance",
            Self::KeepUnknownPanelType => "workspace_repair.keep_unknown_panel_type",
        }
    }
}

/// Structured metadata-only action emitted by workspace repair planning.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceRepairAction {
    /// Stable repair action code.
    pub code: WorkspaceRepairActionCode,
    /// Panel instance identity affected by this action.
    pub panel_instance: Option<PanelInstanceId>,
    /// Panel type identity affected by this action.
    pub panel_type: Option<PanelTypeId>,
    /// Dock frame containing the panel instance when known.
    pub frame: Option<FrameId>,
    /// Legacy dock panel identity when known.
    pub panel: Option<PanelId>,
    /// Title stored on the dock panel when relevant.
    pub dock_title: Option<String>,
    /// Title stored on the panel instance when relevant.
    pub instance_title: Option<String>,
}

impl WorkspaceRepairAction {
    pub(super) fn new(code: WorkspaceRepairActionCode) -> Self {
        Self {
            code,
            panel_instance: None,
            panel_type: None,
            frame: None,
            panel: None,
            dock_title: None,
            instance_title: None,
        }
    }

    /// Returns the stable string code for this repair action.
    #[must_use]
    pub const fn stable_code(&self) -> &'static str {
        self.code.as_str()
    }
}

/// Deterministic report for explicit workspace snapshot repair planning.
#[derive(Debug, Clone, PartialEq)]
pub struct WorkspaceRepairPlan {
    /// Strict validation diagnostics collected before repair planning.
    pub diagnostics: WorkspaceSnapshotDiagnostics,
    /// Metadata-only repair actions the plan would apply.
    pub actions: Vec<WorkspaceRepairAction>,
    outcome: WorkspaceRepairPlanOutcome,
}

#[derive(Debug, Clone, PartialEq)]
enum WorkspaceRepairPlanOutcome {
    Repaired(WorkspaceSnapshot),
    HardError(WorkspaceRestoreError),
}

impl WorkspaceRepairPlan {
    pub(super) fn repaired(
        diagnostics: WorkspaceSnapshotDiagnostics,
        actions: Vec<WorkspaceRepairAction>,
        snapshot: WorkspaceSnapshot,
    ) -> Self {
        Self {
            diagnostics,
            actions,
            outcome: WorkspaceRepairPlanOutcome::Repaired(snapshot),
        }
    }

    pub(super) fn with_hard_error(
        diagnostics: WorkspaceSnapshotDiagnostics,
        error: WorkspaceRestoreError,
    ) -> Self {
        Self {
            diagnostics,
            actions: Vec::new(),
            outcome: WorkspaceRepairPlanOutcome::HardError(error),
        }
    }

    /// Returns true when this plan can produce a repaired snapshot.
    #[must_use]
    pub const fn is_repairable(&self) -> bool {
        matches!(self.outcome, WorkspaceRepairPlanOutcome::Repaired(_))
    }

    /// Returns true when repair planning found a hard error.
    #[must_use]
    pub const fn has_hard_error(&self) -> bool {
        matches!(self.outcome, WorkspaceRepairPlanOutcome::HardError(_))
    }

    /// Returns the repaired workspace snapshot when planning found no hard
    /// repair error.
    #[must_use]
    pub const fn repaired_snapshot(&self) -> Option<&WorkspaceSnapshot> {
        match &self.outcome {
            WorkspaceRepairPlanOutcome::Repaired(snapshot) => Some(snapshot),
            WorkspaceRepairPlanOutcome::HardError(_) => None,
        }
    }

    /// Returns the hard repair error when planning could not safely produce a
    /// repaired snapshot.
    #[must_use]
    pub const fn hard_error(&self) -> Option<&WorkspaceRestoreError> {
        match &self.outcome {
            WorkspaceRepairPlanOutcome::Repaired(_) => None,
            WorkspaceRepairPlanOutcome::HardError(error) => Some(error),
        }
    }

    /// Consumes the plan and returns the repaired snapshot.
    ///
    /// # Errors
    ///
    /// Returns the hard repair error when planning could not safely produce a
    /// repaired snapshot.
    pub fn into_repaired_snapshot(self) -> Result<WorkspaceSnapshot, WorkspaceRestoreError> {
        match self.outcome {
            WorkspaceRepairPlanOutcome::Repaired(snapshot) => Ok(snapshot),
            WorkspaceRepairPlanOutcome::HardError(error) => Err(error),
        }
    }
}

impl From<DockRestoreError> for WorkspaceRestoreError {
    fn from(value: DockRestoreError) -> Self {
        Self::Dock(value)
    }
}
