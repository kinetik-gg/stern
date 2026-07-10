use super::{
    BTreeSet, COMPONENT_METADATA, ComponentCategory, ComponentConformanceStatus,
    ComponentEvidenceCategory, component_conformance_matrix_by_stage, component_evidence,
    component_evidence_for, component_status_evidence, components_by_evidence_category, entry,
    evidence_categories, matrix_entry, matrix_evidence_categories,
};

#[test]
fn s10_s11_conformance_matrix_rows_report_experimental_data_only_coverage() {
    for (slug, stage, category, component_slug) in [
        (
            "s10-outliner-tree-selection-semantics",
            10,
            ComponentCategory::Collection,
            Some("outliner"),
        ),
        (
            "s10-asset-browser-grid-list-metadata",
            10,
            ComponentCategory::Collection,
            Some("asset-browser"),
        ),
        (
            "s10-inline-edit-rename-lifecycle",
            10,
            ComponentCategory::TextEditing,
            None,
        ),
        (
            "s10-collection-drag-drop-context",
            10,
            ComponentCategory::Collection,
            None,
        ),
        (
            "s10-collection-filter-sort-selection-preservation",
            10,
            ComponentCategory::Collection,
            None,
        ),
        (
            "s11-timeline-layout-coordinate-selection",
            11,
            ComponentCategory::Viewport,
            Some("timeline"),
        ),
        (
            "s11-ruler-ticks-timecode",
            11,
            ComponentCategory::Viewport,
            Some("ruler"),
        ),
        (
            "s11-transport-action-controls",
            11,
            ComponentCategory::Control,
            Some("transport-controls"),
        ),
        (
            "s11-timeline-snapping",
            11,
            ComponentCategory::Viewport,
            Some("timeline"),
        ),
        (
            "s11-timeline-preservation",
            11,
            ComponentCategory::Viewport,
            Some("timeline"),
        ),
    ] {
        let row = matrix_entry(slug);
        assert_eq!(row.stage, stage, "{slug} stage");
        assert_eq!(row.category, category, "{slug} category");
        assert_eq!(
            row.status,
            ComponentConformanceStatus::Experimental,
            "{slug} must not claim complete widget behavior"
        );
        assert_eq!(row.component_slug, component_slug, "{slug} component slug");

        let categories = matrix_evidence_categories(slug);
        for category in [
            ComponentEvidenceCategory::Status,
            ComponentEvidenceCategory::Stage,
            ComponentEvidenceCategory::Conformance,
            ComponentEvidenceCategory::Showcase,
        ] {
            assert!(
                categories.contains(&category),
                "{slug} missing {category:?}"
            );
        }
    }
}
#[test]
fn s10_s11_matrix_stage_filters_are_exact() {
    let stage_10 = component_conformance_matrix_by_stage(10)
        .map(|row| row.slug)
        .collect::<BTreeSet<_>>();
    let stage_11 = component_conformance_matrix_by_stage(11)
        .map(|row| row.slug)
        .collect::<BTreeSet<_>>();

    assert_eq!(
        stage_10,
        BTreeSet::from([
            "s10-asset-browser-grid-list-metadata",
            "s10-collection-drag-drop-context",
            "s10-collection-filter-sort-selection-preservation",
            "s10-inline-edit-rename-lifecycle",
            "s10-outliner-tree-selection-semantics",
        ])
    );
    assert_eq!(
        stage_11,
        BTreeSet::from([
            "s11-ruler-ticks-timecode",
            "s11-timeline-layout-coordinate-selection",
            "s11-timeline-preservation",
            "s11-timeline-snapping",
            "s11-transport-action-controls",
        ])
    );
}

#[test]
fn s10_s11_matrix_evidence_points_to_public_contracts_and_tests() {
    for (slug, contract, test) in [
        (
            "s10-outliner-tree-selection-semantics",
            "OutlinerModel",
            "outliner_conformance::tree_visible_row_order_is_deterministic",
        ),
        (
            "s10-asset-browser-grid-list-metadata",
            "AssetBrowserLayout",
            "asset_browser_conformance::grid_layout_resolves_deterministic_materialized_items",
        ),
        (
            "s10-inline-edit-rename-lifecycle",
            "InlineEditSession",
            "inline_edit_conformance::draft_edit_commit_cancel_and_focus_loss_requests_are_deterministic",
        ),
        (
            "s10-collection-drag-drop-context",
            "CollectionDragSource",
            "collection_drag_context_conformance::drag_source_identity_is_stable_and_selection_aware",
        ),
        (
            "s10-collection-filter-sort-selection-preservation",
            "CollectionProjection",
            "collection_projection_conformance::selected_ids_survive_filtering_out_and_back_in",
        ),
        (
            "s11-timeline-layout-coordinate-selection",
            "TimelineLayout",
            "timeline_conformance::timeline_lane_visible_and_materialized_ranges_are_deterministic",
        ),
        (
            "s11-ruler-ticks-timecode",
            "TimelineRulerTickRequest",
            "timeline_conformance::ruler_ticks_are_deterministic_finite_and_ordered",
        ),
        (
            "s11-transport-action-controls",
            "TransportControls",
            "timeline_transport_conformance::transport_request_metadata_preserves_action_source_kind_and_timeline_context",
        ),
        (
            "s11-timeline-snapping",
            "timeline_snap_candidates",
            "timeline_conformance::snap_candidates_include_grid_markers_keyframes_clip_edges_and_range_boundaries",
        ),
        (
            "s11-timeline-preservation",
            "TimelineViewportState",
            "timeline_conformance::scroll_clamping_preserves_playhead_range_and_snap_metadata",
        ),
    ] {
        let row = matrix_entry(slug);
        assert!(
            row.public_contracts.contains(&contract),
            "{slug} missing public contract {contract}"
        );
        assert!(
            row.deterministic_tests.contains(&test),
            "{slug} missing deterministic test {test}"
        );
    }
}

#[test]
fn s12_s13_conformance_matrix_rows_report_experimental_data_only_coverage() {
    for (slug, stage, category, component_slug) in [
        (
            "s12-viewport-surface-overlays",
            12,
            ComponentCategory::Viewport,
            Some("viewport"),
        ),
        (
            "s12-viewport-tools-transform-handles",
            12,
            ComponentCategory::Viewport,
            Some("viewport-tools"),
        ),
        (
            "s12-viewport-action-routing",
            12,
            ComponentCategory::Viewport,
            Some("viewport-action-routing"),
        ),
        (
            "s12-viewport-guides-rulers-safe-areas-hud",
            12,
            ComponentCategory::Viewport,
            Some("viewport"),
        ),
        (
            "s13-progress-indicator-metadata",
            13,
            ComponentCategory::Display,
            Some("progress-indicator"),
        ),
        (
            "s13-job-list-progress-cancel",
            13,
            ComponentCategory::System,
            Some("job-list"),
        ),
        (
            "s13-diagnostic-strip-codes-fields-ordering",
            13,
            ComponentCategory::System,
            Some("diagnostic-strip"),
        ),
        (
            "s13-feedback-stack-lifetime-repaint",
            13,
            ComponentCategory::System,
            Some("feedback-stack"),
        ),
    ] {
        let row = matrix_entry(slug);
        assert_eq!(row.stage, stage, "{slug} stage");
        assert_eq!(row.category, category, "{slug} category");
        assert_eq!(
            row.status,
            ComponentConformanceStatus::Experimental,
            "{slug} must stay honest about incomplete component behavior"
        );
        assert_eq!(row.component_slug, component_slug, "{slug} component slug");

        let categories = matrix_evidence_categories(slug);
        for category in [
            ComponentEvidenceCategory::Status,
            ComponentEvidenceCategory::Stage,
            ComponentEvidenceCategory::Conformance,
            ComponentEvidenceCategory::Showcase,
        ] {
            assert!(
                categories.contains(&category),
                "{slug} missing {category:?}"
            );
        }
    }
}

#[test]
fn s12_s13_matrix_stage_filters_are_exact() {
    let stage_12 = component_conformance_matrix_by_stage(12)
        .map(|row| row.slug)
        .collect::<BTreeSet<_>>();
    let stage_13 = component_conformance_matrix_by_stage(13)
        .map(|row| row.slug)
        .collect::<BTreeSet<_>>();

    assert_eq!(
        stage_12,
        BTreeSet::from([
            "s12-viewport-action-routing",
            "s12-viewport-guides-rulers-safe-areas-hud",
            "s12-viewport-surface-overlays",
            "s12-viewport-tools-transform-handles",
        ])
    );
    assert_eq!(
        stage_13,
        BTreeSet::from([
            "s13-diagnostic-strip-codes-fields-ordering",
            "s13-feedback-stack-lifetime-repaint",
            "s13-job-list-progress-cancel",
            "s13-progress-indicator-metadata",
        ])
    );
}

#[test]
fn s12_s13_matrix_evidence_points_to_public_contracts_and_tests() {
    for (slug, contract, test) in [
        (
            "s12-viewport-surface-overlays",
            "ViewportSurface",
            "viewport_conformance::overlay_hit_priority_and_id_tie_breaking_are_descriptor_order_independent",
        ),
        (
            "s12-viewport-tools-transform-handles",
            "ViewportTransformDragRequest",
            "viewport_conformance::transform_drag_capture_preserves_identity_and_reports_deltas_without_mutation",
        ),
        (
            "s12-viewport-action-routing",
            "ViewportActionRequest",
            "viewport_conformance::viewport_action_descriptors_preserve_order_state_and_context_metadata",
        ),
        (
            "s12-viewport-guides-rulers-safe-areas-hud",
            "ViewportPanZoomHudDescriptor",
            "viewport_conformance::pan_zoom_hud_reports_state_and_target_metadata_without_actions",
        ),
        (
            "s13-progress-indicator-metadata",
            "StatusProgress",
            "status_bar_conformance::job_progress_clamps_and_sanitizes_determinate_values_without_affecting_indeterminate",
        ),
        (
            "s13-job-list-progress-cancel",
            "JobCancel",
            "status_bar_conformance::job_cancel_metadata_preserves_job_action_identity_and_availability",
        ),
        (
            "s13-diagnostic-strip-codes-fields-ordering",
            "DiagnosticStripItemId",
            "status_bar_conformance::status_bar_diagnostics_strip_orders_by_severity_and_preserves_insertion_order_within_severity",
        ),
        (
            "s13-feedback-stack-lifetime-repaint",
            "RepaintRequest",
            "status_bar_conformance::feedback_repaint_after_is_bounded_to_next_active_timed_expiry",
        ),
    ] {
        let row = matrix_entry(slug);
        assert!(
            row.public_contracts.contains(&contract),
            "{slug} missing public contract {contract}"
        );
        assert!(
            row.deterministic_tests.contains(&test),
            "{slug} missing deterministic test {test}"
        );
    }
}

#[test]
fn s12_s13_matrix_evidence_uses_specific_conformance_descriptors() {
    for (name, evidence_id) in [
        ("Viewport", "conformance.viewport-surface-contracts"),
        ("ViewportTools", "conformance.viewport-tool-contracts"),
        (
            "ViewportActionRouting",
            "conformance.viewport-action-routing-contracts",
        ),
        (
            "ProgressIndicator",
            "conformance.progress-indicator-contracts",
        ),
        ("JobList", "conformance.job-list-contracts"),
        ("DiagnosticStrip", "conformance.diagnostic-strip-contracts"),
        ("FeedbackStack", "conformance.feedback-stack-contracts"),
    ] {
        let evidence = component_evidence_for(entry(name))
            .map(|evidence| evidence.id)
            .collect::<BTreeSet<_>>();

        assert!(
            evidence.contains(evidence_id),
            "{name} missing specific evidence {evidence_id}"
        );
        assert!(
            component_evidence(evidence_id).is_some(),
            "{evidence_id} must resolve to a stable evidence descriptor"
        );
    }
}

#[test]
fn evidence_helpers_resolve_lookup_filters_and_status_metadata() {
    let stage_12 = component_evidence("stage.12-viewport-tools").expect("stage 12 evidence");
    assert_eq!(stage_12.category, ComponentEvidenceCategory::Stage);

    let viewport = entry("Viewport");
    let viewport_evidence = component_evidence_for(viewport)
        .map(|evidence| evidence.id)
        .collect::<BTreeSet<_>>();
    assert!(viewport_evidence.contains("status.experimental-public-surface"));
    assert!(viewport_evidence.contains("stage.12-viewport-tools"));
    assert!(viewport_evidence.contains("conformance.viewport-surface-contracts"));
    assert!(viewport_evidence.contains("showcase.metadata-only"));

    let status_evidence = component_status_evidence(viewport).collect::<Vec<_>>();
    assert_eq!(status_evidence.len(), 1);
    assert_eq!(
        status_evidence[0].category,
        ComponentEvidenceCategory::Status
    );

    let staged = components_by_evidence_category(ComponentEvidenceCategory::Stage)
        .map(|metadata| metadata.name)
        .collect::<BTreeSet<_>>();
    assert!(staged.contains("Outliner"));
    assert!(staged.contains("Timeline"));
    assert!(staged.contains("Viewport"));
    assert!(staged.contains("JobList"));

    let showcase =
        components_by_evidence_category(ComponentEvidenceCategory::Showcase).collect::<Vec<_>>();
    assert_eq!(showcase.len(), COMPONENT_METADATA.len());
    assert!(showcase.iter().all(|metadata| {
        evidence_categories(metadata).contains(&ComponentEvidenceCategory::Showcase)
    }));
}
