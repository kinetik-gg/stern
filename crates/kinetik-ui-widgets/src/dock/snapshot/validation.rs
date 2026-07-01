use std::collections::{BTreeMap, BTreeSet};

use super::super::{
    DockNode, DockPathElement, DockSplitPath, Frame, FrameId, Panel, PanelId, PanelInstanceId,
    PanelInstanceSnapshot, PanelTypeDescriptor, PanelTypeId,
};
use super::{
    DockRestoreError, DockSnapshot, DockSnapshotDiagnostic, DockSnapshotDiagnosticCode,
    DockSnapshotDiagnostics, DockSnapshotNode, DockSnapshotSplitValue, SnapshotDiagnosticSeverity,
    WorkspaceRepairAction, WorkspaceRepairActionCode, WorkspaceRepairPlan, WorkspaceRestoreError,
    WorkspaceSnapshot, WorkspaceSnapshotDiagnostic, WorkspaceSnapshotDiagnosticCode,
    WorkspaceSnapshotDiagnostics,
};

pub(crate) fn snapshot_node(node: &DockNode) -> DockSnapshotNode {
    match node {
        DockNode::Frame(frame) => DockSnapshotNode::Frame {
            id: frame.id,
            panels: frame.panels.clone(),
            active: frame.active,
            dismissible_panels: frame.dismissible_panels.iter().copied().collect(),
        },
        DockNode::Split {
            axis,
            ratio,
            min_first,
            min_second,
            first,
            second,
        } => DockSnapshotNode::Split {
            axis: *axis,
            ratio: *ratio,
            min_first: *min_first,
            min_second: *min_second,
            first: Box::new(snapshot_node(first)),
            second: Box::new(snapshot_node(second)),
        },
    }
}

pub(crate) fn restore_node(snapshot: DockSnapshotNode) -> DockNode {
    match snapshot {
        DockSnapshotNode::Frame {
            id,
            panels,
            active,
            dismissible_panels,
        } => DockNode::Frame(Frame {
            id,
            panels,
            active,
            dismissible_panels: dismissible_panels.into_iter().collect(),
        }),
        DockSnapshotNode::Split {
            axis,
            ratio,
            min_first,
            min_second,
            first,
            second,
        } => DockNode::Split {
            axis,
            ratio,
            min_first,
            min_second,
            first: Box::new(restore_node(*first)),
            second: Box::new(restore_node(*second)),
        },
    }
}

#[derive(Default)]
pub(crate) struct DockSnapshotValidation {
    frame_ids: BTreeSet<FrameId>,
    panel_ids: BTreeSet<PanelId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DockPanelReference {
    panel: PanelId,
    frame: FrameId,
    title: String,
}

#[derive(Default)]
struct DockSnapshotDiagnosticState {
    frame_ids: BTreeSet<FrameId>,
    panel_ids: BTreeSet<PanelId>,
    panel_references: BTreeMap<PanelId, DockPanelReference>,
    diagnostics: Vec<DockSnapshotDiagnostic>,
}

/// Returns structured diagnostics for a dock snapshot.
#[must_use]
pub fn validate_dock_snapshot_diagnostics(snapshot: &DockSnapshot) -> DockSnapshotDiagnostics {
    let mut state = DockSnapshotDiagnosticState::default();
    collect_dock_snapshot_diagnostics(&snapshot.root, &DockSplitPath::root(), &mut state);
    if let Some(active_frame) = snapshot.active_frame
        && !state.frame_ids.contains(&active_frame)
    {
        let mut diagnostic = DockSnapshotDiagnostic::new(
            DockSnapshotDiagnosticCode::InvalidActiveFrame,
            DockSplitPath::root(),
        );
        diagnostic.frame = Some(active_frame);
        state.diagnostics.push(diagnostic);
    }
    DockSnapshotDiagnostics {
        diagnostics: state.diagnostics,
    }
}

/// Returns structured diagnostics for a workspace snapshot.
#[must_use]
pub fn validate_workspace_snapshot_diagnostics(
    snapshot: &WorkspaceSnapshot,
    descriptors: &[PanelTypeDescriptor],
) -> WorkspaceSnapshotDiagnostics {
    let dock = validate_dock_snapshot_diagnostics(&snapshot.dock);
    let dock_references = collect_dock_panel_references(&snapshot.dock.root);
    let workspace = collect_workspace_snapshot_diagnostics(snapshot, descriptors, &dock_references);
    WorkspaceSnapshotDiagnostics { dock, workspace }
}

pub(super) fn plan_workspace_snapshot_repair(
    snapshot: &WorkspaceSnapshot,
    descriptors: &[PanelTypeDescriptor],
) -> WorkspaceRepairPlan {
    let diagnostics = validate_workspace_snapshot_diagnostics(snapshot, descriptors);
    if let Some(error) = workspace_repair_hard_error(snapshot, descriptors, &diagnostics) {
        return WorkspaceRepairPlan::with_hard_error(diagnostics, error);
    }

    let mut actions = Vec::new();
    let stale_panel_instances = collect_stale_panel_instances(&diagnostics);
    let mut repaired = WorkspaceSnapshot::new(
        snapshot.dock.clone(),
        snapshot
            .panel_instances
            .iter()
            .filter(|instance| !stale_panel_instances.contains(&instance.id))
            .cloned()
            .collect(),
    );

    for diagnostic in &diagnostics.workspace {
        match diagnostic.code {
            WorkspaceSnapshotDiagnosticCode::MissingPanelInstance => {
                if let Some(placeholder) =
                    placeholder_panel_instance_from_diagnostic(diagnostic, descriptors)
                {
                    let mut action = WorkspaceRepairAction::new(
                        WorkspaceRepairActionCode::AddMissingPanelInstancePlaceholder,
                    );
                    action.panel_instance = Some(placeholder.id);
                    action.panel_type = Some(placeholder.panel_type);
                    action.frame = diagnostic.frame;
                    action.panel = diagnostic.panel;
                    action.dock_title.clone_from(&diagnostic.dock_title);
                    repaired.panel_instances.push(placeholder);
                    actions.push(action);
                }
            }
            WorkspaceSnapshotDiagnosticCode::StalePanelInstance => {
                let mut action =
                    WorkspaceRepairAction::new(WorkspaceRepairActionCode::DropStalePanelInstance);
                action.panel_instance = diagnostic.panel_instance;
                action.panel_type = diagnostic.panel_type;
                action.instance_title.clone_from(&diagnostic.instance_title);
                actions.push(action);
            }
            WorkspaceSnapshotDiagnosticCode::UnknownPanelType => {
                if diagnostic
                    .panel_instance
                    .is_some_and(|panel_instance| stale_panel_instances.contains(&panel_instance))
                {
                    continue;
                }
                let mut action =
                    WorkspaceRepairAction::new(WorkspaceRepairActionCode::KeepUnknownPanelType);
                action.panel_instance = diagnostic.panel_instance;
                action.panel_type = diagnostic.panel_type;
                actions.push(action);
            }
            WorkspaceSnapshotDiagnosticCode::DuplicatePanelInstanceId
            | WorkspaceSnapshotDiagnosticCode::DuplicatePanelTypeDescriptor
            | WorkspaceSnapshotDiagnosticCode::PanelTitleDrift => {}
        }
    }

    WorkspaceRepairPlan::repaired(diagnostics, actions, repaired)
}

fn workspace_repair_hard_error(
    snapshot: &WorkspaceSnapshot,
    descriptors: &[PanelTypeDescriptor],
    diagnostics: &WorkspaceSnapshotDiagnostics,
) -> Option<WorkspaceRestoreError> {
    if let Err(error) = validate_dock_snapshot(&snapshot.dock) {
        return Some(WorkspaceRestoreError::Dock(error));
    }

    for diagnostic in &diagnostics.workspace {
        match diagnostic.code {
            WorkspaceSnapshotDiagnosticCode::DuplicatePanelTypeDescriptor => {
                return diagnostic.panel_type.map(|panel_type| {
                    WorkspaceRestoreError::DuplicatePanelTypeDescriptor { panel_type }
                });
            }
            WorkspaceSnapshotDiagnosticCode::DuplicatePanelInstanceId => {
                return diagnostic.panel_instance.map(|panel_instance| {
                    WorkspaceRestoreError::DuplicatePanelInstanceId { panel_instance }
                });
            }
            WorkspaceSnapshotDiagnosticCode::MissingPanelInstance => {
                if placeholder_panel_instance_from_diagnostic(diagnostic, descriptors).is_none() {
                    return diagnostic.panel_instance.map(|panel_instance| {
                        WorkspaceRestoreError::MissingPanelInstance { panel_instance }
                    });
                }
            }
            WorkspaceSnapshotDiagnosticCode::StalePanelInstance
            | WorkspaceSnapshotDiagnosticCode::UnknownPanelType
            | WorkspaceSnapshotDiagnosticCode::PanelTitleDrift => {}
        }
    }

    None
}

fn collect_stale_panel_instances(
    diagnostics: &WorkspaceSnapshotDiagnostics,
) -> BTreeSet<PanelInstanceId> {
    diagnostics
        .workspace
        .iter()
        .filter_map(|diagnostic| {
            (diagnostic.code == WorkspaceSnapshotDiagnosticCode::StalePanelInstance)
                .then_some(diagnostic.panel_instance)
                .flatten()
        })
        .collect()
}

fn placeholder_panel_instance_from_diagnostic(
    diagnostic: &WorkspaceSnapshotDiagnostic,
    descriptors: &[PanelTypeDescriptor],
) -> Option<PanelInstanceSnapshot> {
    let panel_instance = diagnostic.panel_instance?;
    let dock_title = diagnostic.dock_title.as_ref()?;
    let panel_type = unique_panel_type_for_title(dock_title, descriptors)?;
    Some(PanelInstanceSnapshot::new(
        panel_instance,
        panel_type,
        dock_title.clone(),
    ))
}

fn unique_panel_type_for_title(
    title: &str,
    descriptors: &[PanelTypeDescriptor],
) -> Option<PanelTypeId> {
    let mut matches = descriptors
        .iter()
        .filter(|descriptor| descriptor.title == title)
        .map(|descriptor| descriptor.id);
    let panel_type = matches.next()?;
    matches.next().is_none().then_some(panel_type)
}

pub(crate) fn validate_dock_snapshot(
    snapshot: &DockSnapshot,
) -> Result<DockSnapshotValidation, DockRestoreError> {
    let mut validation = DockSnapshotValidation::default();
    validate_snapshot_node(&snapshot.root, &mut validation)?;
    if let Some(active_frame) = snapshot.active_frame
        && !validation.frame_ids.contains(&active_frame)
    {
        return Err(DockRestoreError::InvalidActiveFrame);
    }
    Ok(validation)
}

fn collect_workspace_snapshot_diagnostics(
    snapshot: &WorkspaceSnapshot,
    descriptors: &[PanelTypeDescriptor],
    dock_references: &BTreeMap<PanelId, DockPanelReference>,
) -> Vec<WorkspaceSnapshotDiagnostic> {
    let mut diagnostics = Vec::new();
    let mut panel_types = BTreeSet::new();
    for descriptor in descriptors {
        if !panel_types.insert(descriptor.id) {
            let mut diagnostic = WorkspaceSnapshotDiagnostic::new(
                WorkspaceSnapshotDiagnosticCode::DuplicatePanelTypeDescriptor,
            );
            diagnostic.panel_type = Some(descriptor.id);
            diagnostics.push(diagnostic);
        }
    }

    let mut snapshot_panel_instances = BTreeMap::new();
    for instance in &snapshot.panel_instances {
        match snapshot_panel_instances.entry(instance.id) {
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(instance);
            }
            std::collections::btree_map::Entry::Occupied(_) => {
                let mut diagnostic = WorkspaceSnapshotDiagnostic::new(
                    WorkspaceSnapshotDiagnosticCode::DuplicatePanelInstanceId,
                );
                diagnostic.panel_instance = Some(instance.id);
                diagnostic.panel_type = Some(instance.panel_type);
                diagnostics.push(diagnostic);
            }
        }
    }

    for (panel_instance, instance) in &snapshot_panel_instances {
        if !panel_types.contains(&instance.panel_type) {
            let mut diagnostic =
                WorkspaceSnapshotDiagnostic::new(WorkspaceSnapshotDiagnosticCode::UnknownPanelType);
            diagnostic.panel_instance = Some(*panel_instance);
            diagnostic.panel_type = Some(instance.panel_type);
            diagnostics.push(diagnostic);
        }
    }

    let dock_panel_instances: BTreeMap<_, _> = dock_references
        .iter()
        .map(|(panel, reference)| (panel.instance_id(), reference))
        .collect();
    for (panel_instance, reference) in &dock_panel_instances {
        if !snapshot_panel_instances.contains_key(panel_instance) {
            let mut diagnostic = WorkspaceSnapshotDiagnostic::new(
                WorkspaceSnapshotDiagnosticCode::MissingPanelInstance,
            );
            diagnostic.panel_instance = Some(*panel_instance);
            diagnostic.frame = Some(reference.frame);
            diagnostic.panel = Some(reference.panel);
            diagnostic.dock_title = Some(reference.title.clone());
            diagnostics.push(diagnostic);
        }
    }

    for panel_instance in snapshot_panel_instances.keys() {
        if !dock_panel_instances.contains_key(panel_instance) {
            let mut diagnostic = WorkspaceSnapshotDiagnostic::new(
                WorkspaceSnapshotDiagnosticCode::StalePanelInstance,
            );
            diagnostic.panel_instance = Some(*panel_instance);
            if let Some(instance) = snapshot_panel_instances.get(panel_instance) {
                diagnostic.panel_type = Some(instance.panel_type);
                diagnostic.instance_title = Some(instance.title.clone());
            }
            diagnostics.push(diagnostic);
        }
    }

    for (panel_instance, reference) in &dock_panel_instances {
        let Some(instance) = snapshot_panel_instances.get(panel_instance) else {
            continue;
        };
        if reference.title != instance.title {
            let mut diagnostic =
                WorkspaceSnapshotDiagnostic::new(WorkspaceSnapshotDiagnosticCode::PanelTitleDrift);
            diagnostic.severity = SnapshotDiagnosticSeverity::Warning;
            diagnostic.panel_instance = Some(*panel_instance);
            diagnostic.panel_type = Some(instance.panel_type);
            diagnostic.frame = Some(reference.frame);
            diagnostic.panel = Some(reference.panel);
            diagnostic.dock_title = Some(reference.title.clone());
            diagnostic.instance_title = Some(instance.title.clone());
            diagnostics.push(diagnostic);
        }
    }

    diagnostics
}

pub(super) fn validate_workspace_snapshot(
    snapshot: &WorkspaceSnapshot,
    descriptors: &[PanelTypeDescriptor],
    dock_validation: &DockSnapshotValidation,
) -> Result<(), WorkspaceRestoreError> {
    let mut panel_types = BTreeSet::new();
    for descriptor in descriptors {
        if !panel_types.insert(descriptor.id) {
            return Err(WorkspaceRestoreError::DuplicatePanelTypeDescriptor {
                panel_type: descriptor.id,
            });
        }
    }

    let dock_panel_instances: BTreeSet<_> = dock_validation
        .panel_ids
        .iter()
        .map(|panel| panel.instance_id())
        .collect();
    let mut snapshot_panel_instances = BTreeMap::new();

    for instance in &snapshot.panel_instances {
        if snapshot_panel_instances
            .insert(instance.id, instance.panel_type)
            .is_some()
        {
            return Err(WorkspaceRestoreError::DuplicatePanelInstanceId {
                panel_instance: instance.id,
            });
        }
    }

    for (panel_instance, panel_type) in &snapshot_panel_instances {
        if !panel_types.contains(panel_type) {
            return Err(WorkspaceRestoreError::UnknownPanelType {
                panel_instance: *panel_instance,
                panel_type: *panel_type,
            });
        }
    }

    for panel_instance in &dock_panel_instances {
        if !snapshot_panel_instances.contains_key(panel_instance) {
            return Err(WorkspaceRestoreError::MissingPanelInstance {
                panel_instance: *panel_instance,
            });
        }
    }

    for panel_instance in snapshot_panel_instances.keys() {
        if !dock_panel_instances.contains(panel_instance) {
            return Err(WorkspaceRestoreError::StalePanelInstance {
                panel_instance: *panel_instance,
            });
        }
    }

    Ok(())
}

fn collect_dock_panel_references(
    snapshot: &DockSnapshotNode,
) -> BTreeMap<PanelId, DockPanelReference> {
    let mut state = DockSnapshotDiagnosticState::default();
    collect_dock_snapshot_diagnostics(snapshot, &DockSplitPath::root(), &mut state);
    state.panel_references
}

fn collect_dock_snapshot_diagnostics(
    snapshot: &DockSnapshotNode,
    path: &DockSplitPath,
    state: &mut DockSnapshotDiagnosticState,
) {
    match snapshot {
        DockSnapshotNode::Frame {
            id,
            panels,
            active,
            dismissible_panels,
        } => collect_frame_snapshot_diagnostics(
            *id,
            panels,
            *active,
            dismissible_panels,
            path,
            state,
        ),
        DockSnapshotNode::Split {
            ratio,
            min_first,
            min_second,
            first,
            second,
            ..
        } => collect_split_snapshot_diagnostics(
            *ratio,
            *min_first,
            *min_second,
            first,
            second,
            path,
            state,
        ),
    }
}

fn collect_frame_snapshot_diagnostics(
    id: FrameId,
    panels: &[Panel],
    active: usize,
    dismissible_panels: &[PanelId],
    path: &DockSplitPath,
    state: &mut DockSnapshotDiagnosticState,
) {
    if !state.frame_ids.insert(id) {
        let mut diagnostic =
            DockSnapshotDiagnostic::new(DockSnapshotDiagnosticCode::DuplicateFrameId, path.clone());
        diagnostic.frame = Some(id);
        state.diagnostics.push(diagnostic);
    }
    if panels.is_empty() {
        let mut diagnostic =
            DockSnapshotDiagnostic::new(DockSnapshotDiagnosticCode::EmptyFrame, path.clone());
        diagnostic.frame = Some(id);
        state.diagnostics.push(diagnostic);
    }
    if active >= panels.len() {
        let mut diagnostic = DockSnapshotDiagnostic::new(
            DockSnapshotDiagnosticCode::InvalidActivePanelIndex,
            path.clone(),
        );
        diagnostic.frame = Some(id);
        diagnostic.active_index = Some(active);
        diagnostic.panel_count = Some(panels.len());
        state.diagnostics.push(diagnostic);
    }

    let frame_panel_ids = collect_frame_panel_diagnostics(id, panels, path, state);
    collect_frame_dismissible_diagnostics(id, dismissible_panels, &frame_panel_ids, path, state);
}

fn collect_frame_panel_diagnostics(
    frame: FrameId,
    panels: &[Panel],
    path: &DockSplitPath,
    state: &mut DockSnapshotDiagnosticState,
) -> BTreeSet<PanelId> {
    let mut frame_panel_ids = BTreeSet::new();
    for panel in panels {
        if !frame_panel_ids.insert(panel.id) || !state.panel_ids.insert(panel.id) {
            let mut diagnostic = DockSnapshotDiagnostic::new(
                DockSnapshotDiagnosticCode::DuplicatePanelId,
                path.clone(),
            );
            diagnostic.frame = Some(frame);
            diagnostic.panel = Some(panel.id);
            state.diagnostics.push(diagnostic);
        }
        state
            .panel_references
            .entry(panel.id)
            .or_insert_with(|| DockPanelReference {
                panel: panel.id,
                frame,
                title: panel.title.clone(),
            });
    }
    frame_panel_ids
}

fn collect_frame_dismissible_diagnostics(
    frame: FrameId,
    dismissible_panels: &[PanelId],
    frame_panel_ids: &BTreeSet<PanelId>,
    path: &DockSplitPath,
    state: &mut DockSnapshotDiagnosticState,
) {
    let mut frame_dismissible_ids = BTreeSet::new();
    for panel in dismissible_panels {
        if !frame_dismissible_ids.insert(*panel) {
            let mut diagnostic = DockSnapshotDiagnostic::new(
                DockSnapshotDiagnosticCode::DuplicateDismissiblePolicy,
                path.clone(),
            );
            diagnostic.frame = Some(frame);
            diagnostic.panel = Some(*panel);
            state.diagnostics.push(diagnostic);
        }
        if !frame_panel_ids.contains(panel) {
            let mut diagnostic = DockSnapshotDiagnostic::new(
                DockSnapshotDiagnosticCode::InvalidDismissiblePanel,
                path.clone(),
            );
            diagnostic.frame = Some(frame);
            diagnostic.panel = Some(*panel);
            state.diagnostics.push(diagnostic);
        }
    }
}

fn collect_split_snapshot_diagnostics(
    ratio: f32,
    min_first: f32,
    min_second: f32,
    first: &DockSnapshotNode,
    second: &DockSnapshotNode,
    path: &DockSplitPath,
    state: &mut DockSnapshotDiagnosticState,
) {
    if !ratio.is_finite() || !(0.0..=1.0).contains(&ratio) {
        let mut diagnostic = DockSnapshotDiagnostic::new(
            DockSnapshotDiagnosticCode::InvalidSplitRatio,
            path.clone(),
        );
        diagnostic.split_value = Some(DockSnapshotSplitValue::Ratio);
        state.diagnostics.push(diagnostic);
    }
    if !min_first.is_finite() || min_first < 0.0 {
        let mut diagnostic = DockSnapshotDiagnostic::new(
            DockSnapshotDiagnosticCode::InvalidSplitMinimum,
            path.clone(),
        );
        diagnostic.split_value = Some(DockSnapshotSplitValue::MinFirst);
        state.diagnostics.push(diagnostic);
    }
    if !min_second.is_finite() || min_second < 0.0 {
        let mut diagnostic = DockSnapshotDiagnostic::new(
            DockSnapshotDiagnosticCode::InvalidSplitMinimum,
            path.clone(),
        );
        diagnostic.split_value = Some(DockSnapshotSplitValue::MinSecond);
        state.diagnostics.push(diagnostic);
    }
    collect_dock_snapshot_diagnostics(first, &path.child(DockPathElement::First), state);
    collect_dock_snapshot_diagnostics(second, &path.child(DockPathElement::Second), state);
}

fn validate_snapshot_node(
    snapshot: &DockSnapshotNode,
    validation: &mut DockSnapshotValidation,
) -> Result<(), DockRestoreError> {
    match snapshot {
        DockSnapshotNode::Frame {
            id,
            panels,
            active,
            dismissible_panels,
        } => {
            if !validation.frame_ids.insert(*id) {
                return Err(DockRestoreError::DuplicateFrameId);
            }
            if panels.is_empty() {
                return Err(DockRestoreError::EmptyFrame);
            }
            if *active >= panels.len() {
                return Err(DockRestoreError::InvalidActiveIndex);
            }

            let mut frame_panel_ids = BTreeSet::new();
            for panel in panels {
                if !frame_panel_ids.insert(panel.id) || !validation.panel_ids.insert(panel.id) {
                    return Err(DockRestoreError::DuplicatePanelId);
                }
            }

            let mut frame_dismissible_ids = BTreeSet::new();
            for id in dismissible_panels {
                if !frame_dismissible_ids.insert(*id) {
                    return Err(DockRestoreError::DuplicateDismissiblePanel);
                }
                if !frame_panel_ids.contains(id) {
                    return Err(DockRestoreError::InvalidDismissiblePanel);
                }
            }
            Ok(())
        }
        DockSnapshotNode::Split {
            ratio,
            min_first,
            min_second,
            first,
            second,
            ..
        } => {
            if !ratio.is_finite() || !(0.0..=1.0).contains(ratio) {
                return Err(DockRestoreError::InvalidSplitRatio);
            }
            if !min_first.is_finite()
                || !min_second.is_finite()
                || *min_first < 0.0
                || *min_second < 0.0
            {
                return Err(DockRestoreError::InvalidSplitMinimum);
            }
            validate_snapshot_node(first, validation)?;
            validate_snapshot_node(second, validation)
        }
    }
}
