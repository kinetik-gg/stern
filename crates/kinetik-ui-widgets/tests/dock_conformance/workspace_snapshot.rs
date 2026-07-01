#[test]
fn panel_id_and_panel_instance_id_convert_without_changing_existing_panel_usage() {
    let legacy = PanelId::from_raw(123);
    let instance = PanelInstanceId::from_raw(123);

    assert_eq!(legacy.instance_id(), instance);
    assert_eq!(PanelInstanceId::from(legacy), instance);
    assert_eq!(PanelId::from(instance), legacy);
    assert_eq!(PanelId::from_instance_id(instance), legacy);

    let panel = Panel::from_instance_id(instance, "Graph");
    assert_eq!(panel.id, legacy);
    assert_eq!(panel.instance_id(), instance);
    assert_eq!(Panel::new(legacy, "Graph"), panel);
}

#[test]
fn workspace_snapshot_panel_instance_references_survive_validation_and_restore() {
    let descriptors = workspace_panel_descriptors();
    let instances = workspace_panel_instances();
    let snapshot = nested_dock().workspace_snapshot(instances);

    snapshot
        .validate(&descriptors)
        .expect("workspace validates");
    assert_eq!(
        snapshot.panel_instances[0].state_key.as_deref(),
        Some("media-state")
    );

    let restored = Dock::restore_workspace(snapshot.clone(), &descriptors).expect("restore");
    assert_eq!(restored.snapshot(), snapshot.dock);

    let restored_workspace = restored.workspace_snapshot(snapshot.panel_instances.clone());
    assert_eq!(restored_workspace, snapshot);
    restored_workspace
        .validate(&descriptors)
        .expect("restored workspace validates");
}

#[test]
fn workspace_snapshot_rejects_missing_panel_instance_record() {
    let descriptors = workspace_panel_descriptors();
    let mut instances = workspace_panel_instances();
    instances.retain(|instance| instance.id != PanelInstanceId::from_raw(3));
    let snapshot = nested_dock().workspace_snapshot(instances);

    assert_eq!(
        snapshot
            .validate(&descriptors)
            .expect_err("missing instance"),
        WorkspaceRestoreError::MissingPanelInstance {
            panel_instance: PanelInstanceId::from_raw(3),
        }
    );
}

#[test]
fn workspace_snapshot_rejects_unknown_panel_type() {
    let descriptors = workspace_panel_descriptors();
    let mut snapshot = nested_dock().workspace_snapshot(workspace_panel_instances());
    snapshot.panel_instances[1].panel_type = PanelTypeId::from_raw(999);

    assert_eq!(
        snapshot
            .validate(&descriptors)
            .expect_err("unknown panel type"),
        WorkspaceRestoreError::UnknownPanelType {
            panel_instance: PanelInstanceId::from_raw(2),
            panel_type: PanelTypeId::from_raw(999),
        }
    );
}

#[test]
fn workspace_snapshot_rejects_duplicate_panel_instance_ids() {
    let descriptors = workspace_panel_descriptors();
    let mut snapshot = nested_dock().workspace_snapshot(workspace_panel_instances());
    snapshot.panel_instances[2].id = PanelInstanceId::from_raw(2);

    assert_eq!(
        snapshot
            .validate(&descriptors)
            .expect_err("duplicate panel instance"),
        WorkspaceRestoreError::DuplicatePanelInstanceId {
            panel_instance: PanelInstanceId::from_raw(2),
        }
    );
}

#[test]
fn workspace_snapshot_rejects_duplicate_panel_type_descriptors_deterministically() {
    let mut descriptors = workspace_panel_descriptors();
    descriptors.push(PanelTypeDescriptor::new(
        PanelTypeId::from_raw(20),
        "Second Viewport",
    ));
    let snapshot = nested_dock().workspace_snapshot(workspace_panel_instances());

    assert_eq!(
        snapshot
            .validate(&descriptors)
            .expect_err("duplicate descriptor"),
        WorkspaceRestoreError::DuplicatePanelTypeDescriptor {
            panel_type: PanelTypeId::from_raw(20),
        }
    );
}

#[test]
fn workspace_snapshot_rejects_stale_panel_instance_records_deterministically() {
    let descriptors = workspace_panel_descriptors();
    let mut snapshot = nested_dock().workspace_snapshot(workspace_panel_instances());
    snapshot.panel_instances.push(PanelInstanceSnapshot::new(
        PanelInstanceId::from_raw(99),
        PanelTypeId::from_raw(10),
        "Stale Media",
    ));

    assert_eq!(
        snapshot.validate(&descriptors).expect_err("stale instance"),
        WorkspaceRestoreError::StalePanelInstance {
            panel_instance: PanelInstanceId::from_raw(99),
        }
    );
}

#[test]
fn repair_plan_drops_stale_panel_instances_only_in_repair_mode_and_reports_action() {
    let descriptors = workspace_panel_descriptors();
    let mut snapshot = nested_dock().workspace_snapshot(workspace_panel_instances());
    snapshot.panel_instances.push(PanelInstanceSnapshot::new(
        PanelInstanceId::from_raw(99),
        PanelTypeId::from_raw(10),
        "Stale Media",
    ));
    let original_snapshot = snapshot.clone();

    assert_eq!(
        snapshot.validate(&descriptors).expect_err("stale instance"),
        WorkspaceRestoreError::StalePanelInstance {
            panel_instance: PanelInstanceId::from_raw(99),
        }
    );

    let plan = snapshot.repair_plan(&descriptors);

    assert!(plan.is_repairable());
    assert_eq!(plan.hard_error(), None);
    assert!(plan.repaired_snapshot().is_some());
    assert_eq!(snapshot, original_snapshot);
    assert_eq!(plan.actions.len(), 1);
    assert_eq!(
        plan.actions[0].code,
        WorkspaceRepairActionCode::DropStalePanelInstance
    );
    assert_eq!(
        plan.actions[0].stable_code(),
        "workspace_repair.drop_stale_panel_instance"
    );
    assert_eq!(
        plan.actions[0].panel_instance,
        Some(PanelInstanceId::from_raw(99))
    );

    let repaired = plan.into_repaired_snapshot().expect("repaired snapshot");
    assert!(
        !repaired
            .panel_instances
            .iter()
            .any(|instance| instance.id == PanelInstanceId::from_raw(99))
    );
    repaired
        .validate(&descriptors)
        .expect("stale metadata dropped");
}

#[test]
fn repair_plan_adds_missing_panel_instance_placeholder_from_dock_title_and_reports_action() {
    let descriptors = workspace_panel_descriptors();
    let mut instances = workspace_panel_instances();
    instances.retain(|instance| instance.id != PanelInstanceId::from_raw(3));
    let snapshot = nested_dock().workspace_snapshot(instances);
    let original_snapshot = snapshot.clone();

    assert_eq!(
        snapshot
            .validate(&descriptors)
            .expect_err("missing instance"),
        WorkspaceRestoreError::MissingPanelInstance {
            panel_instance: PanelInstanceId::from_raw(3),
        }
    );

    let plan = snapshot.repair_plan(&descriptors);

    assert!(plan.is_repairable());
    assert_eq!(plan.hard_error(), None);
    assert!(plan.repaired_snapshot().is_some());
    assert_eq!(snapshot, original_snapshot);
    assert_eq!(plan.actions.len(), 1);
    assert_eq!(
        plan.actions[0].code,
        WorkspaceRepairActionCode::AddMissingPanelInstancePlaceholder
    );
    assert_eq!(
        plan.actions[0].stable_code(),
        "workspace_repair.add_missing_panel_instance_placeholder"
    );
    assert_eq!(
        plan.actions[0].panel_instance,
        Some(PanelInstanceId::from_raw(3))
    );
    assert_eq!(plan.actions[0].panel_type, Some(PanelTypeId::from_raw(30)));
    assert_eq!(plan.actions[0].dock_title.as_deref(), Some("Inspector"));

    let repaired = plan.into_repaired_snapshot().expect("repaired snapshot");
    let placeholder = repaired
        .panel_instances
        .iter()
        .find(|instance| instance.id == PanelInstanceId::from_raw(3))
        .expect("placeholder instance");
    assert_eq!(placeholder.panel_type, PanelTypeId::from_raw(30));
    assert_eq!(placeholder.title, "Inspector");
    assert_eq!(placeholder.state_key, None);
    repaired
        .validate(&descriptors)
        .expect("placeholder metadata is valid");
}

#[test]
fn repair_plan_keeps_unknown_panel_type_visible_without_descriptor() {
    let descriptors = workspace_panel_descriptors();
    let mut snapshot = nested_dock().workspace_snapshot(workspace_panel_instances());
    snapshot.panel_instances[1].panel_type = PanelTypeId::from_raw(999);
    let original_snapshot = snapshot.clone();

    assert_eq!(
        snapshot
            .validate(&descriptors)
            .expect_err("unknown panel type"),
        WorkspaceRestoreError::UnknownPanelType {
            panel_instance: PanelInstanceId::from_raw(2),
            panel_type: PanelTypeId::from_raw(999),
        }
    );

    let plan = snapshot.repair_plan(&descriptors);

    assert!(plan.is_repairable());
    assert_eq!(plan.hard_error(), None);
    assert!(plan.repaired_snapshot().is_some());
    assert_eq!(snapshot, original_snapshot);
    assert_eq!(
        plan.diagnostics.workspace[0].code,
        WorkspaceSnapshotDiagnosticCode::UnknownPanelType
    );
    assert_eq!(plan.actions.len(), 1);
    assert_eq!(
        plan.actions[0].code,
        WorkspaceRepairActionCode::KeepUnknownPanelType
    );
    assert_eq!(
        plan.actions[0].stable_code(),
        "workspace_repair.keep_unknown_panel_type"
    );
    assert_eq!(plan.actions[0].panel_type, Some(PanelTypeId::from_raw(999)));

    let repaired = plan.into_repaired_snapshot().expect("repaired snapshot");
    assert_eq!(
        repaired.panel_instances[1].panel_type,
        PanelTypeId::from_raw(999)
    );
    assert_eq!(
        repaired
            .validate(&descriptors)
            .expect_err("unknown panel type remains visible"),
        WorkspaceRestoreError::UnknownPanelType {
            panel_instance: PanelInstanceId::from_raw(2),
            panel_type: PanelTypeId::from_raw(999),
        }
    );
}

#[test]
fn repair_plan_drops_stale_unknown_panel_instances_without_keep_action() {
    let descriptors = workspace_panel_descriptors();
    let mut snapshot = nested_dock().workspace_snapshot(workspace_panel_instances());
    snapshot.panel_instances.push(PanelInstanceSnapshot::new(
        PanelInstanceId::from_raw(99),
        PanelTypeId::from_raw(999),
        "Stale Unknown",
    ));
    let original_snapshot = snapshot.clone();

    assert_eq!(
        snapshot
            .validate(&descriptors)
            .expect_err("unknown stale panel type"),
        WorkspaceRestoreError::UnknownPanelType {
            panel_instance: PanelInstanceId::from_raw(99),
            panel_type: PanelTypeId::from_raw(999),
        }
    );

    let plan = snapshot.repair_plan(&descriptors);

    assert!(plan.is_repairable());
    assert_eq!(plan.hard_error(), None);
    assert!(plan.repaired_snapshot().is_some());
    assert_eq!(snapshot, original_snapshot);
    let diagnostic_codes = plan
        .diagnostics
        .workspace
        .iter()
        .map(|diagnostic| diagnostic.code)
        .collect::<Vec<_>>();
    assert_eq!(
        diagnostic_codes,
        vec![
            WorkspaceSnapshotDiagnosticCode::UnknownPanelType,
            WorkspaceSnapshotDiagnosticCode::StalePanelInstance,
        ]
    );
    assert_eq!(plan.actions.len(), 1);
    assert_eq!(
        plan.actions[0].code,
        WorkspaceRepairActionCode::DropStalePanelInstance
    );
    assert_eq!(
        plan.actions[0].stable_code(),
        "workspace_repair.drop_stale_panel_instance"
    );
    assert_eq!(
        plan.actions[0].panel_instance,
        Some(PanelInstanceId::from_raw(99))
    );
    assert_eq!(plan.actions[0].panel_type, Some(PanelTypeId::from_raw(999)));

    let repaired = plan.into_repaired_snapshot().expect("repaired snapshot");
    assert!(
        !repaired
            .panel_instances
            .iter()
            .any(|instance| instance.id == PanelInstanceId::from_raw(99))
    );
    repaired
        .validate(&descriptors)
        .expect("stale unknown metadata dropped");
}

#[test]
fn repair_plan_rejects_duplicate_panel_instance_ids_as_hard_error() {
    let descriptors = workspace_panel_descriptors();
    let mut snapshot = nested_dock().workspace_snapshot(workspace_panel_instances());
    snapshot.panel_instances[2].id = PanelInstanceId::from_raw(2);

    let plan = snapshot.repair_plan(&descriptors);

    assert!(plan.has_hard_error());
    assert_eq!(
        plan.hard_error(),
        Some(&WorkspaceRestoreError::DuplicatePanelInstanceId {
            panel_instance: PanelInstanceId::from_raw(2),
        })
    );
    assert_eq!(plan.repaired_snapshot(), None);
    assert!(plan.actions.is_empty());
}

#[test]
fn repair_plan_rejects_duplicate_panel_type_descriptors_as_hard_error() {
    let mut descriptors = workspace_panel_descriptors();
    descriptors.push(PanelTypeDescriptor::new(
        PanelTypeId::from_raw(20),
        "Second Viewport",
    ));
    let snapshot = nested_dock().workspace_snapshot(workspace_panel_instances());

    let plan = snapshot.repair_plan(&descriptors);

    assert!(plan.has_hard_error());
    assert_eq!(
        plan.hard_error(),
        Some(&WorkspaceRestoreError::DuplicatePanelTypeDescriptor {
            panel_type: PanelTypeId::from_raw(20),
        })
    );
    assert_eq!(plan.repaired_snapshot(), None);
    assert!(plan.actions.is_empty());
}

#[test]
fn repair_plan_rejects_invalid_dock_snapshots_as_hard_error() {
    let descriptors = workspace_panel_descriptors();
    let snapshot = WorkspaceSnapshot::new(
        invalid_dock_diagnostic_snapshot(),
        workspace_panel_instances(),
    );

    let plan = snapshot.repair_plan(&descriptors);

    assert!(plan.has_hard_error());
    assert_eq!(
        plan.hard_error(),
        Some(&WorkspaceRestoreError::Dock(
            DockRestoreError::InvalidSplitRatio,
        ))
    );
    assert_eq!(plan.repaired_snapshot(), None);
    assert!(plan.actions.is_empty());
}

#[test]
fn repair_plan_outcome_accessors_preserve_invariants_for_public_callers() {
    let descriptors = workspace_panel_descriptors();
    let mut repairable_snapshot = nested_dock().workspace_snapshot(workspace_panel_instances());
    repairable_snapshot
        .panel_instances
        .push(PanelInstanceSnapshot::new(
            PanelInstanceId::from_raw(99),
            PanelTypeId::from_raw(10),
            "Stale Media",
        ));

    let repairable_plan = repairable_snapshot.repair_plan(&descriptors);
    assert!(repairable_plan.is_repairable());
    assert!(!repairable_plan.has_hard_error());
    assert!(repairable_plan.repaired_snapshot().is_some());
    assert_eq!(repairable_plan.hard_error(), None);

    let mut hard_error_snapshot = nested_dock().workspace_snapshot(workspace_panel_instances());
    hard_error_snapshot.panel_instances[2].id = PanelInstanceId::from_raw(2);

    let hard_error_plan = hard_error_snapshot.repair_plan(&descriptors);
    assert!(!hard_error_plan.is_repairable());
    assert!(hard_error_plan.has_hard_error());
    assert_eq!(hard_error_plan.repaired_snapshot(), None);
    assert_eq!(
        hard_error_plan.hard_error(),
        Some(&WorkspaceRestoreError::DuplicatePanelInstanceId {
            panel_instance: PanelInstanceId::from_raw(2),
        })
    );
}

#[test]
fn repair_plan_is_deterministic_and_does_not_mutate_the_input_snapshot() {
    let descriptors = workspace_panel_descriptors();
    let mut instances = workspace_panel_instances();
    instances.retain(|instance| instance.id != PanelInstanceId::from_raw(3));
    instances[1].panel_type = PanelTypeId::from_raw(999);
    instances.push(PanelInstanceSnapshot::new(
        PanelInstanceId::from_raw(99),
        PanelTypeId::from_raw(10),
        "Stale Media",
    ));
    let snapshot = nested_dock().workspace_snapshot(instances);
    let original_snapshot = snapshot.clone();

    let first = snapshot.repair_plan(&descriptors);
    let second = snapshot.repair_plan(&descriptors);

    assert_eq!(first, second);
    assert_eq!(snapshot, original_snapshot);
    assert!(first.is_repairable());
    let action_codes = first
        .actions
        .iter()
        .map(WorkspaceRepairAction::stable_code)
        .collect::<Vec<_>>();
    assert_eq!(
        action_codes,
        vec![
            "workspace_repair.keep_unknown_panel_type",
            "workspace_repair.add_missing_panel_instance_placeholder",
            "workspace_repair.drop_stale_panel_instance",
        ]
    );
}
