fn invalid_dock_diagnostic_snapshot() -> DockSnapshot {
    DockSnapshot {
        active_frame: Some(FrameId::from_raw(99)),
        root: DockSnapshotNode::Split {
            axis: Axis::Horizontal,
            ratio: 1.25,
            min_first: -1.0,
            min_second: f32::INFINITY,
            first: Box::new(DockSnapshotNode::Frame {
                id: FrameId::from_raw(1),
                panels: vec![],
                active: 1,
                dismissible_panels: vec![],
            }),
            second: Box::new(DockSnapshotNode::Frame {
                id: FrameId::from_raw(1),
                panels: vec![panel(2, "A"), panel(2, "Duplicate")],
                active: 3,
                dismissible_panels: vec![PanelId::from_raw(9), PanelId::from_raw(9)],
            }),
        },
    }
}
#[test]
fn dock_snapshot_diagnostics_report_stable_codes() {
    let snapshot = invalid_dock_diagnostic_snapshot();

    let diagnostics = snapshot.diagnostics();
    let codes = diagnostics
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code.as_str())
        .collect::<Vec<_>>();

    assert!(diagnostics.has_errors());
    assert_eq!(
        codes,
        vec![
            "dock.invalid_split_ratio",
            "dock.invalid_split_minimum",
            "dock.invalid_split_minimum",
            "dock.empty_frame",
            "dock.invalid_active_panel_index",
            "dock.duplicate_frame_id",
            "dock.invalid_active_panel_index",
            "dock.duplicate_panel_id",
            "dock.invalid_dismissible_panel",
            "dock.duplicate_dismissible_policy",
            "dock.invalid_dismissible_panel",
            "dock.invalid_active_frame",
        ]
    );
}

#[test]
fn dock_snapshot_diagnostics_report_context() {
    let snapshot = invalid_dock_diagnostic_snapshot();

    let diagnostics = snapshot.diagnostics();

    assert_eq!(diagnostics.diagnostics[0].path, DockSplitPath::root());
    assert_eq!(
        diagnostics.diagnostics[0].split_value,
        Some(DockSnapshotSplitValue::Ratio)
    );
    assert_eq!(
        diagnostics.diagnostics[1].split_value,
        Some(DockSnapshotSplitValue::MinFirst)
    );
    assert_eq!(
        diagnostics.diagnostics[2].split_value,
        Some(DockSnapshotSplitValue::MinSecond)
    );
    assert_eq!(diagnostics.diagnostics[3].stable_code(), "dock.empty_frame");
    assert_eq!(diagnostics.diagnostics[3].frame, Some(FrameId::from_raw(1)));
    assert_eq!(
        diagnostics.diagnostics[3].path,
        DockSplitPath::root().child(DockPathElement::First)
    );

    let duplicate_frame = diagnostics
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == DockSnapshotDiagnosticCode::DuplicateFrameId)
        .expect("duplicate frame diagnostic");
    assert_eq!(duplicate_frame.frame, Some(FrameId::from_raw(1)));
    assert_eq!(
        duplicate_frame.path,
        DockSplitPath::root().child(DockPathElement::Second)
    );

    let duplicate_panel = diagnostics
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == DockSnapshotDiagnosticCode::DuplicatePanelId)
        .expect("duplicate panel diagnostic");
    assert_eq!(duplicate_panel.frame, Some(FrameId::from_raw(1)));
    assert_eq!(duplicate_panel.panel, Some(PanelId::from_raw(2)));

    let invalid_active_panel = diagnostics
        .diagnostics
        .iter()
        .find(|diagnostic| {
            diagnostic.code == DockSnapshotDiagnosticCode::InvalidActivePanelIndex
                && diagnostic.panel_count == Some(2)
        })
        .expect("invalid active panel diagnostic");
    assert_eq!(invalid_active_panel.active_index, Some(3));

    let invalid_dismissible = diagnostics
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == DockSnapshotDiagnosticCode::InvalidDismissiblePanel)
        .expect("invalid dismissible diagnostic");
    assert_eq!(invalid_dismissible.panel, Some(PanelId::from_raw(9)));

    let duplicate_dismissible = diagnostics
        .diagnostics
        .iter()
        .find(|diagnostic| {
            diagnostic.code == DockSnapshotDiagnosticCode::DuplicateDismissiblePolicy
        })
        .expect("duplicate dismissible diagnostic");
    assert_eq!(duplicate_dismissible.panel, Some(PanelId::from_raw(9)));

    let invalid_active_frame = diagnostics
        .diagnostics
        .last()
        .expect("invalid active frame diagnostic");
    assert_eq!(
        invalid_active_frame.code,
        DockSnapshotDiagnosticCode::InvalidActiveFrame
    );
    assert_eq!(invalid_active_frame.frame, Some(FrameId::from_raw(99)));
}

#[test]
fn dock_snapshot_diagnostics_are_repeatable() {
    let snapshot = DockSnapshot {
        active_frame: Some(FrameId::from_raw(99)),
        root: DockSnapshotNode::Frame {
            id: FrameId::from_raw(1),
            panels: vec![],
            active: 0,
            dismissible_panels: vec![],
        },
    };

    assert_eq!(snapshot.diagnostics(), snapshot.diagnostics());
}

#[test]
fn workspace_snapshot_diagnostics_report_stable_codes_and_context() {
    let mut descriptors = workspace_panel_descriptors();
    descriptors.push(PanelTypeDescriptor::new(
        PanelTypeId::from_raw(20),
        "Second Viewport",
    ));

    let mut instances = workspace_panel_instances();
    instances[0].title = "Renamed Media".to_owned();
    instances[1].panel_type = PanelTypeId::from_raw(999);
    instances[2].id = PanelInstanceId::from_raw(2);
    instances.push(PanelInstanceSnapshot::new(
        PanelInstanceId::from_raw(99),
        PanelTypeId::from_raw(10),
        "Stale Media",
    ));
    let snapshot = nested_dock().workspace_snapshot(instances);

    let diagnostics = snapshot.diagnostics(&descriptors);
    let codes = diagnostics
        .workspace
        .iter()
        .map(|diagnostic| diagnostic.code.as_str())
        .collect::<Vec<_>>();

    assert!(diagnostics.dock.is_valid());
    assert!(diagnostics.has_errors());
    assert_eq!(
        codes,
        vec![
            "workspace.duplicate_panel_type_descriptor",
            "workspace.duplicate_panel_instance_id",
            "workspace.unknown_panel_type",
            "workspace.missing_panel_instance",
            "workspace.stale_panel_instance",
            "workspace.panel_title_drift",
        ]
    );

    let duplicate_type = &diagnostics.workspace[0];
    assert_eq!(duplicate_type.panel_type, Some(PanelTypeId::from_raw(20)));

    let duplicate_instance = &diagnostics.workspace[1];
    assert_eq!(
        duplicate_instance.panel_instance,
        Some(PanelInstanceId::from_raw(2))
    );

    let unknown_type = &diagnostics.workspace[2];
    assert_eq!(
        unknown_type.panel_instance,
        Some(PanelInstanceId::from_raw(2))
    );
    assert_eq!(unknown_type.panel_type, Some(PanelTypeId::from_raw(999)));

    let missing_instance = &diagnostics.workspace[3];
    assert_eq!(
        missing_instance.panel_instance,
        Some(PanelInstanceId::from_raw(3))
    );
    assert_eq!(missing_instance.frame, Some(FrameId::from_raw(2)));
    assert_eq!(missing_instance.panel, Some(PanelId::from_raw(3)));
    assert_eq!(missing_instance.dock_title.as_deref(), Some("Inspector"));

    let stale_instance = &diagnostics.workspace[4];
    assert_eq!(
        stale_instance.panel_instance,
        Some(PanelInstanceId::from_raw(99))
    );
    assert_eq!(stale_instance.panel_type, Some(PanelTypeId::from_raw(10)));
    assert_eq!(
        stale_instance.instance_title.as_deref(),
        Some("Stale Media")
    );

    let title_drift = &diagnostics.workspace[5];
    assert_eq!(title_drift.severity, SnapshotDiagnosticSeverity::Warning);
    assert_eq!(title_drift.stable_code(), "workspace.panel_title_drift");
    assert_eq!(
        title_drift.panel_instance,
        Some(PanelInstanceId::from_raw(1))
    );
    assert_eq!(title_drift.frame, Some(FrameId::from_raw(1)));
    assert_eq!(title_drift.panel, Some(PanelId::from_raw(1)));
    assert_eq!(title_drift.dock_title.as_deref(), Some("Media"));
    assert_eq!(title_drift.instance_title.as_deref(), Some("Renamed Media"));
}

#[test]
fn workspace_snapshot_title_drift_is_a_warning_not_a_restore_error() {
    let descriptors = workspace_panel_descriptors();
    let mut instances = workspace_panel_instances();
    instances[0].title = "Renamed Media".to_owned();
    let snapshot = nested_dock().workspace_snapshot(instances);

    let diagnostics = snapshot.diagnostics(&descriptors);

    assert!(diagnostics.is_valid());
    assert_eq!(diagnostics.workspace.len(), 1);
    assert_eq!(
        diagnostics.workspace[0].code,
        WorkspaceSnapshotDiagnosticCode::PanelTitleDrift
    );
    assert_eq!(
        diagnostics.workspace[0].severity,
        SnapshotDiagnosticSeverity::Warning
    );
    snapshot
        .validate(&descriptors)
        .expect("title drift allowed");
}

#[test]
fn workspace_snapshot_diagnostics_are_repeatable() {
    let descriptors = workspace_panel_descriptors();
    let mut snapshot = nested_dock().workspace_snapshot(workspace_panel_instances());
    snapshot.panel_instances.push(PanelInstanceSnapshot::new(
        PanelInstanceId::from_raw(99),
        PanelTypeId::from_raw(10),
        "Stale Media",
    ));

    assert_eq!(
        snapshot.diagnostics(&descriptors),
        snapshot.diagnostics(&descriptors)
    );
}

#[test]
fn dock_snapshot_diagnostics_adapt_to_strip_with_stable_typed_context() {
    let diagnostics = invalid_dock_diagnostic_snapshot().diagnostics();
    let mut strip = DiagnosticStrip::new();

    strip.extend_dock_snapshot_diagnostics(DiagnosticStripItemId::from_raw(100), &diagnostics);

    assert_eq!(
        strip.summary().errors,
        u32::try_from(diagnostics.diagnostics.len()).expect("diagnostic count fits status summary")
    );
    assert_eq!(strip.items()[0].id, DiagnosticStripItemId::from_raw(100));
    assert_eq!(strip.items()[0].code, "dock.invalid_split_ratio");
    assert_eq!(strip.items()[0].source, Some(DiagnosticSource::Dock));
    assert_eq!(strip.items()[0].severity, DiagnosticStripSeverity::Error);
    assert_eq!(
        field_value(&strip.items()[0].fields, "path"),
        Some(&DiagnosticFieldValue::DockPath(Vec::new()))
    );
    assert_eq!(
        field_value(&strip.items()[0].fields, "split_value"),
        Some(&DiagnosticFieldValue::DockSplitValue(
            DockSnapshotSplitValue::Ratio
        ))
    );

    let duplicate_panel = strip
        .items()
        .iter()
        .find(|item| item.code == "dock.duplicate_panel_id")
        .expect("duplicate panel item");
    assert_eq!(
        field_value(&duplicate_panel.fields, "frame"),
        Some(&DiagnosticFieldValue::FrameId(FrameId::from_raw(1)))
    );
    assert_eq!(
        field_value(&duplicate_panel.fields, "panel"),
        Some(&DiagnosticFieldValue::PanelId(PanelId::from_raw(2)))
    );
}

#[test]
fn workspace_snapshot_diagnostics_adapt_to_strip_after_dock_diagnostics() {
    let descriptors = workspace_panel_descriptors();
    let mut instances = workspace_panel_instances();
    instances[0].title = "Renamed Media".to_owned();
    instances.push(PanelInstanceSnapshot::new(
        PanelInstanceId::from_raw(99),
        PanelTypeId::from_raw(10),
        "Stale Media",
    ));
    let diagnostics = nested_dock()
        .workspace_snapshot(instances)
        .diagnostics(&descriptors);
    let mut strip = DiagnosticStrip::new();

    strip.extend_workspace_snapshot_diagnostics(DiagnosticStripItemId::from_raw(200), &diagnostics);

    assert_eq!(strip.summary().errors, 1);
    assert_eq!(strip.summary().warnings, 1);
    assert_eq!(
        strip
            .items()
            .iter()
            .map(|item| item.code.as_str())
            .collect::<Vec<_>>(),
        vec![
            "workspace.stale_panel_instance",
            "workspace.panel_title_drift",
        ]
    );
    assert_eq!(strip.items()[0].source, Some(DiagnosticSource::Workspace));
    assert_eq!(
        field_value(&strip.items()[0].fields, "panel_instance"),
        Some(&DiagnosticFieldValue::PanelInstanceId(
            PanelInstanceId::from_raw(99)
        ))
    );
    assert_eq!(
        field_value(&strip.items()[0].fields, "panel_type"),
        Some(&DiagnosticFieldValue::PanelTypeId(PanelTypeId::from_raw(
            10
        )))
    );

    let drift = &strip.items()[1];
    assert_eq!(drift.severity, DiagnosticStripSeverity::Warning);
    assert_eq!(
        field_value(&drift.fields, "frame"),
        Some(&DiagnosticFieldValue::FrameId(FrameId::from_raw(1)))
    );
    assert_eq!(
        field_value(&drift.fields, "panel"),
        Some(&DiagnosticFieldValue::PanelId(PanelId::from_raw(1)))
    );
    assert_eq!(
        field_value(&drift.fields, "dock_title"),
        Some(&DiagnosticFieldValue::Text("Media".to_owned()))
    );
    assert_eq!(
        field_value(&drift.fields, "instance_title"),
        Some(&DiagnosticFieldValue::Text("Renamed Media".to_owned()))
    );
}
