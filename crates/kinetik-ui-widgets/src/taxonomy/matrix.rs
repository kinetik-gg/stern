use super::evidence::{
    S10_ASSET_BROWSER_EVIDENCE, S10_COLLECTION_DRAG_CONTEXT_EVIDENCE,
    S10_COLLECTION_PROJECTION_EVIDENCE, S10_INLINE_EDIT_EVIDENCE, S10_OUTLINER_EVIDENCE,
    S11_PRESERVATION_EVIDENCE, S11_RULER_EVIDENCE, S11_SNAPPING_EVIDENCE, S11_TIMELINE_EVIDENCE,
    S11_TRANSPORT_EVIDENCE, S12_VIEWPORT_ACTION_ROUTING_EVIDENCE, S12_VIEWPORT_EVIDENCE,
    S12_VIEWPORT_OVERLAY_EVIDENCE, S12_VIEWPORT_TOOLS_EVIDENCE, S13_DIAGNOSTIC_STRIP_EVIDENCE,
    S13_FEEDBACK_STACK_EVIDENCE, S13_JOB_LIST_EVIDENCE, S13_PROGRESS_EVIDENCE,
};
use super::types::{
    ComponentCapabilityAxis, ComponentCapabilityEvidence, ComponentCategory,
    ComponentConformanceMatrixRow,
};

use ComponentCapabilityAxis::{Accessibility, Input, LiveWorkflow, Model, Paint, Platform};
use ComponentCategory::{Collection, Control, Display, System, TextEditing, Viewport};

const WORKFLOW_AXES: &[ComponentCapabilityAxis] =
    &[Model, Paint, Input, Accessibility, Platform, LiveWorkflow];

const S10_OUTLINER_CAPABILITY_EVIDENCE: &[ComponentCapabilityEvidence] =
    &[ComponentCapabilityEvidence::new(
        Model,
        "conformance.outliner-contracts",
    )];
const S10_ASSET_BROWSER_CAPABILITY_EVIDENCE: &[ComponentCapabilityEvidence] =
    &[ComponentCapabilityEvidence::new(
        Model,
        "conformance.asset-browser-contracts",
    )];
const S10_INLINE_EDIT_CAPABILITY_EVIDENCE: &[ComponentCapabilityEvidence] =
    &[ComponentCapabilityEvidence::new(
        Model,
        "conformance.inline-edit-contracts",
    )];
const S10_COLLECTION_DRAG_CONTEXT_CAPABILITY_EVIDENCE: &[ComponentCapabilityEvidence] =
    &[ComponentCapabilityEvidence::new(
        Model,
        "conformance.collection-drag-context-contracts",
    )];
const S10_COLLECTION_PROJECTION_CAPABILITY_EVIDENCE: &[ComponentCapabilityEvidence] =
    &[ComponentCapabilityEvidence::new(
        Model,
        "conformance.collection-projection-contracts",
    )];
const S11_TIMELINE_CAPABILITY_EVIDENCE: &[ComponentCapabilityEvidence] =
    &[ComponentCapabilityEvidence::new(
        Model,
        "conformance.timeline-contracts",
    )];
const S11_RULER_CAPABILITY_EVIDENCE: &[ComponentCapabilityEvidence] =
    &[ComponentCapabilityEvidence::new(
        Model,
        "conformance.timeline-ruler-contracts",
    )];
const S11_TRANSPORT_CAPABILITY_EVIDENCE: &[ComponentCapabilityEvidence] =
    &[ComponentCapabilityEvidence::new(
        Model,
        "conformance.timeline-transport-contracts",
    )];
const S11_SNAPPING_CAPABILITY_EVIDENCE: &[ComponentCapabilityEvidence] =
    &[ComponentCapabilityEvidence::new(
        Model,
        "conformance.timeline-snapping-contracts",
    )];
const S11_PRESERVATION_CAPABILITY_EVIDENCE: &[ComponentCapabilityEvidence] =
    &[ComponentCapabilityEvidence::new(
        Model,
        "conformance.timeline-preservation-contracts",
    )];
const S12_VIEWPORT_CAPABILITY_EVIDENCE: &[ComponentCapabilityEvidence] =
    &[ComponentCapabilityEvidence::new(
        Model,
        "conformance.viewport-surface-contracts",
    )];
const S12_VIEWPORT_TOOLS_CAPABILITY_EVIDENCE: &[ComponentCapabilityEvidence] =
    &[ComponentCapabilityEvidence::new(
        Model,
        "conformance.viewport-tool-contracts",
    )];
const S12_VIEWPORT_ACTION_ROUTING_CAPABILITY_EVIDENCE: &[ComponentCapabilityEvidence] =
    &[ComponentCapabilityEvidence::new(
        Model,
        "conformance.viewport-action-routing-contracts",
    )];
const S12_VIEWPORT_OVERLAY_CAPABILITY_EVIDENCE: &[ComponentCapabilityEvidence] =
    &[ComponentCapabilityEvidence::new(
        Model,
        "conformance.viewport-overlay-contracts",
    )];
const S13_PROGRESS_CAPABILITY_EVIDENCE: &[ComponentCapabilityEvidence] =
    &[ComponentCapabilityEvidence::new(
        Model,
        "conformance.progress-indicator-contracts",
    )];
const S13_JOB_LIST_CAPABILITY_EVIDENCE: &[ComponentCapabilityEvidence] =
    &[ComponentCapabilityEvidence::new(
        Model,
        "conformance.job-list-contracts",
    )];
const S13_DIAGNOSTIC_STRIP_CAPABILITY_EVIDENCE: &[ComponentCapabilityEvidence] =
    &[ComponentCapabilityEvidence::new(
        Model,
        "conformance.diagnostic-strip-contracts",
    )];
const S13_FEEDBACK_STACK_CAPABILITY_EVIDENCE: &[ComponentCapabilityEvidence] =
    &[ComponentCapabilityEvidence::new(
        Model,
        "conformance.feedback-stack-contracts",
    )];

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
const S12_VIEWPORT_CONTRACTS: &[&str] = &[
    "ViewportSurface",
    "PanZoom",
    "ViewportOverlayDescriptor",
    "ViewportOverlayHit",
    "hit_test_viewport_overlays",
];
const S12_VIEWPORT_TOOLS_CONTRACTS: &[&str] = &[
    "ViewportToolDescriptor",
    "ViewportSelectionTargetDescriptor",
    "ViewportTransformHandleSet",
    "ViewportTransformDragCapture",
    "ViewportTransformDragRequest",
    "viewport_transform_handles",
    "hit_test_viewport_transform_handles",
];
const S12_VIEWPORT_ACTION_ROUTING_CONTRACTS: &[&str] = &[
    "ViewportActionDescriptor",
    "ViewportActionRequest",
    "ViewportActionTarget",
    "viewport_action_requests",
    "viewport_action_semantics",
    "viewport_cursor_request",
];
const S12_VIEWPORT_OVERLAY_CONTRACTS: &[&str] = &[
    "ViewportGuideDescriptor",
    "ViewportRulerDescriptor",
    "ViewportSafeAreaDescriptor",
    "ViewportPanZoomHudDescriptor",
    "viewport_guides",
    "viewport_rulers",
    "viewport_safe_areas",
];
const S13_PROGRESS_CONTRACTS: &[&str] = &[
    "StatusProgress",
    "JobProgress",
    "StatusItem",
    "JobList::active_status_progress",
];
const S13_JOB_LIST_CONTRACTS: &[&str] = &[
    "JobList",
    "JobRow",
    "JobRowId",
    "JobPhase",
    "JobProgress",
    "JobCancel",
];
const S13_DIAGNOSTIC_STRIP_CONTRACTS: &[&str] = &[
    "DiagnosticStrip",
    "DiagnosticStripItem",
    "DiagnosticStripItemId",
    "DiagnosticStripSeverity",
    "DiagnosticField",
    "DiagnosticSource",
];
const S13_FEEDBACK_STACK_CONTRACTS: &[&str] = &[
    "FeedbackStack",
    "FeedbackItem",
    "FeedbackId",
    "FeedbackLifetime",
    "FeedbackAction",
    "FeedbackDismiss",
    "RepaintRequest",
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
const S12_VIEWPORT_TESTS: &[&str] = &[
    "viewport_conformance::content_screen_point_and_rect_conversions_round_trip_under_pan_zoom",
    "viewport_conformance::overlay_hit_testing_transforms_content_targets_and_rejects_invalid_descriptors",
    "viewport_conformance::overlay_hit_priority_and_id_tie_breaking_are_descriptor_order_independent",
];
const S12_VIEWPORT_TOOLS_TESTS: &[&str] = &[
    "viewport_conformance::semantic_metadata_exposes_stable_viewport_and_tool_identity",
    "viewport_conformance::selection_outlines_and_transform_handles_track_content_screen_conversion",
    "viewport_conformance::transform_drag_capture_preserves_identity_and_reports_deltas_without_mutation",
    "viewport_conformance::stale_target_drag_requests_preserve_capture_metadata_as_noop_error_data",
];
const S12_VIEWPORT_ACTION_ROUTING_TESTS: &[&str] = &[
    "viewport_conformance::viewport_action_descriptors_preserve_order_state_and_context_metadata",
    "viewport_conformance::disabled_and_hidden_viewport_actions_do_not_emit_requests",
    "viewport_conformance::viewport_action_semantics_expose_button_toggle_and_action_metadata",
    "viewport_conformance::viewport_cursor_request_priority_is_active_handle_hovered_handle_overlay_then_tool",
];
const S12_VIEWPORT_OVERLAY_TESTS: &[&str] = &[
    "viewport_conformance::guide_descriptors_resolve_deterministically_and_reject_invalid_inputs",
    "viewport_conformance::safe_area_descriptors_clamp_to_content_and_viewport_bounds",
    "viewport_conformance::ruler_overlay_descriptors_emit_bounded_ticks_labels_and_origin_metadata",
    "viewport_conformance::pan_zoom_hud_reports_state_and_target_metadata_without_actions",
];
const S13_PROGRESS_TESTS: &[&str] = &[
    "status_bar_conformance::status_bar_progress_values_sanitize_and_clamp_deterministically",
    "status_bar_conformance::job_progress_clamps_and_sanitizes_determinate_values_without_affecting_indeterminate",
    "status_bar_conformance::job_list_active_progress_keeps_indeterminate_distinct_from_determinate_zero",
];
const S13_JOB_LIST_TESTS: &[&str] = &[
    "status_bar_conformance::job_list_summary_counts_and_row_order_are_deterministic",
    "status_bar_conformance::job_list_active_progress_keeps_indeterminate_distinct_from_determinate_zero",
    "status_bar_conformance::job_cancel_metadata_preserves_job_action_identity_and_availability",
];
const S13_DIAGNOSTIC_STRIP_TESTS: &[&str] = &[
    "status_bar_conformance::status_bar_diagnostics_strip_orders_by_severity_and_preserves_insertion_order_within_severity",
    "status_bar_conformance::status_bar_diagnostics_strip_summary_counts_are_deterministic_for_empty_and_mixed_input",
    "status_bar_conformance::status_bar_diagnostics_strip_aggregates_mixed_typed_diagnostics",
];
const S13_FEEDBACK_STACK_TESTS: &[&str] = &[
    "status_bar_conformance::feedback_timed_items_expire_from_explicit_time_inputs",
    "status_bar_conformance::feedback_stack_preserves_insertion_order_and_filters_inactive_items",
    "status_bar_conformance::feedback_repaint_after_is_bounded_to_next_active_timed_expiry",
    "status_bar_conformance::feedback_dismiss_and_action_metadata_preserve_feedback_and_action_identity",
];

/// Data-only conformance matrix for restarted editor-toolkit S10-S13 capabilities.
pub const COMPONENT_CONFORMANCE_MATRIX: &[ComponentConformanceMatrixRow] = &[
    ComponentConformanceMatrixRow::experimental(
        "Outliner tree, zones, selection, and semantics",
        "s10-outliner-tree-selection-semantics",
        Collection,
        10,
        S10_OUTLINER_CONTRACTS,
        S10_OUTLINER_TESTS,
        S10_OUTLINER_EVIDENCE,
    )
    .with_required_axes(WORKFLOW_AXES)
    .with_capability_evidence(S10_OUTLINER_CAPABILITY_EVIDENCE)
    .with_component_slug("outliner"),
    ComponentConformanceMatrixRow::experimental(
        "Asset browser grid/list layout and metadata",
        "s10-asset-browser-grid-list-metadata",
        Collection,
        10,
        S10_ASSET_BROWSER_CONTRACTS,
        S10_ASSET_BROWSER_TESTS,
        S10_ASSET_BROWSER_EVIDENCE,
    )
    .with_required_axes(WORKFLOW_AXES)
    .with_capability_evidence(S10_ASSET_BROWSER_CAPABILITY_EVIDENCE)
    .with_component_slug("asset-browser"),
    ComponentConformanceMatrixRow::experimental(
        "Inline edit rename lifecycle",
        "s10-inline-edit-rename-lifecycle",
        TextEditing,
        10,
        S10_INLINE_EDIT_CONTRACTS,
        S10_INLINE_EDIT_TESTS,
        S10_INLINE_EDIT_EVIDENCE,
    )
    .with_required_axes(WORKFLOW_AXES)
    .with_capability_evidence(S10_INLINE_EDIT_CAPABILITY_EVIDENCE),
    ComponentConformanceMatrixRow::experimental(
        "Collection drag, drop, and context routing",
        "s10-collection-drag-drop-context",
        Collection,
        10,
        S10_COLLECTION_DRAG_CONTEXT_CONTRACTS,
        S10_COLLECTION_DRAG_CONTEXT_TESTS,
        S10_COLLECTION_DRAG_CONTEXT_EVIDENCE,
    )
    .with_required_axes(WORKFLOW_AXES)
    .with_capability_evidence(S10_COLLECTION_DRAG_CONTEXT_CAPABILITY_EVIDENCE),
    ComponentConformanceMatrixRow::experimental(
        "Collection filter, sort, and selection preservation",
        "s10-collection-filter-sort-selection-preservation",
        Collection,
        10,
        S10_COLLECTION_PROJECTION_CONTRACTS,
        S10_COLLECTION_PROJECTION_TESTS,
        S10_COLLECTION_PROJECTION_EVIDENCE,
    )
    .with_required_axes(WORKFLOW_AXES)
    .with_capability_evidence(S10_COLLECTION_PROJECTION_CAPABILITY_EVIDENCE),
    ComponentConformanceMatrixRow::experimental(
        "Timeline layout, coordinates, selection, and semantics",
        "s11-timeline-layout-coordinate-selection",
        Viewport,
        11,
        S11_TIMELINE_CONTRACTS,
        S11_TIMELINE_TESTS,
        S11_TIMELINE_EVIDENCE,
    )
    .with_required_axes(WORKFLOW_AXES)
    .with_capability_evidence(S11_TIMELINE_CAPABILITY_EVIDENCE)
    .with_component_slug("timeline"),
    ComponentConformanceMatrixRow::experimental(
        "Timeline ruler ticks and timecode labels",
        "s11-ruler-ticks-timecode",
        Viewport,
        11,
        S11_RULER_CONTRACTS,
        S11_RULER_TESTS,
        S11_RULER_EVIDENCE,
    )
    .with_required_axes(WORKFLOW_AXES)
    .with_capability_evidence(S11_RULER_CAPABILITY_EVIDENCE)
    .with_component_slug("ruler"),
    ComponentConformanceMatrixRow::experimental(
        "Timeline transport action controls",
        "s11-transport-action-controls",
        Control,
        11,
        S11_TRANSPORT_CONTRACTS,
        S11_TRANSPORT_TESTS,
        S11_TRANSPORT_EVIDENCE,
    )
    .with_required_axes(WORKFLOW_AXES)
    .with_capability_evidence(S11_TRANSPORT_CAPABILITY_EVIDENCE)
    .with_component_slug("transport-controls"),
    ComponentConformanceMatrixRow::experimental(
        "Timeline snapping candidates and resolution",
        "s11-timeline-snapping",
        Viewport,
        11,
        S11_SNAPPING_CONTRACTS,
        S11_SNAPPING_TESTS,
        S11_SNAPPING_EVIDENCE,
    )
    .with_required_axes(WORKFLOW_AXES)
    .with_capability_evidence(S11_SNAPPING_CAPABILITY_EVIDENCE)
    .with_component_slug("timeline"),
    ComponentConformanceMatrixRow::experimental(
        "Timeline interaction state preservation",
        "s11-timeline-preservation",
        Viewport,
        11,
        S11_PRESERVATION_CONTRACTS,
        S11_PRESERVATION_TESTS,
        S11_PRESERVATION_EVIDENCE,
    )
    .with_required_axes(WORKFLOW_AXES)
    .with_capability_evidence(S11_PRESERVATION_CAPABILITY_EVIDENCE)
    .with_component_slug("timeline"),
    ComponentConformanceMatrixRow::experimental(
        "Viewport surface, pan/zoom, and overlay hit testing",
        "s12-viewport-surface-overlays",
        Viewport,
        12,
        S12_VIEWPORT_CONTRACTS,
        S12_VIEWPORT_TESTS,
        S12_VIEWPORT_EVIDENCE,
    )
    .with_required_axes(WORKFLOW_AXES)
    .with_capability_evidence(S12_VIEWPORT_CAPABILITY_EVIDENCE)
    .with_component_slug("viewport"),
    ComponentConformanceMatrixRow::experimental(
        "Viewport tools and transform manipulators",
        "s12-viewport-tools-transform-handles",
        Viewport,
        12,
        S12_VIEWPORT_TOOLS_CONTRACTS,
        S12_VIEWPORT_TOOLS_TESTS,
        S12_VIEWPORT_TOOLS_EVIDENCE,
    )
    .with_required_axes(WORKFLOW_AXES)
    .with_capability_evidence(S12_VIEWPORT_TOOLS_CAPABILITY_EVIDENCE)
    .with_component_slug("viewport-tools"),
    ComponentConformanceMatrixRow::experimental(
        "Viewport action request and cursor routing",
        "s12-viewport-action-routing",
        Viewport,
        12,
        S12_VIEWPORT_ACTION_ROUTING_CONTRACTS,
        S12_VIEWPORT_ACTION_ROUTING_TESTS,
        S12_VIEWPORT_ACTION_ROUTING_EVIDENCE,
    )
    .with_required_axes(WORKFLOW_AXES)
    .with_capability_evidence(S12_VIEWPORT_ACTION_ROUTING_CAPABILITY_EVIDENCE)
    .with_component_slug("viewport-action-routing"),
    ComponentConformanceMatrixRow::experimental(
        "Viewport guides, rulers, safe areas, and HUD metadata",
        "s12-viewport-guides-rulers-safe-areas-hud",
        Viewport,
        12,
        S12_VIEWPORT_OVERLAY_CONTRACTS,
        S12_VIEWPORT_OVERLAY_TESTS,
        S12_VIEWPORT_OVERLAY_EVIDENCE,
    )
    .with_required_axes(WORKFLOW_AXES)
    .with_capability_evidence(S12_VIEWPORT_OVERLAY_CAPABILITY_EVIDENCE)
    .with_component_slug("viewport"),
    ComponentConformanceMatrixRow::experimental(
        "Progress indicator metadata and active job progress",
        "s13-progress-indicator-metadata",
        Display,
        13,
        S13_PROGRESS_CONTRACTS,
        S13_PROGRESS_TESTS,
        S13_PROGRESS_EVIDENCE,
    )
    .with_required_axes(WORKFLOW_AXES)
    .with_capability_evidence(S13_PROGRESS_CAPABILITY_EVIDENCE)
    .with_component_slug("progress-indicator"),
    ComponentConformanceMatrixRow::experimental(
        "Job list summary, progress, and cancellation metadata",
        "s13-job-list-progress-cancel",
        System,
        13,
        S13_JOB_LIST_CONTRACTS,
        S13_JOB_LIST_TESTS,
        S13_JOB_LIST_EVIDENCE,
    )
    .with_required_axes(WORKFLOW_AXES)
    .with_capability_evidence(S13_JOB_LIST_CAPABILITY_EVIDENCE)
    .with_component_slug("job-list"),
    ComponentConformanceMatrixRow::experimental(
        "Diagnostic strip codes, fields, and ordering",
        "s13-diagnostic-strip-codes-fields-ordering",
        System,
        13,
        S13_DIAGNOSTIC_STRIP_CONTRACTS,
        S13_DIAGNOSTIC_STRIP_TESTS,
        S13_DIAGNOSTIC_STRIP_EVIDENCE,
    )
    .with_required_axes(WORKFLOW_AXES)
    .with_capability_evidence(S13_DIAGNOSTIC_STRIP_CAPABILITY_EVIDENCE)
    .with_component_slug("diagnostic-strip"),
    ComponentConformanceMatrixRow::experimental(
        "Feedback stack lifetime, action, and repaint metadata",
        "s13-feedback-stack-lifetime-repaint",
        System,
        13,
        S13_FEEDBACK_STACK_CONTRACTS,
        S13_FEEDBACK_STACK_TESTS,
        S13_FEEDBACK_STACK_EVIDENCE,
    )
    .with_required_axes(WORKFLOW_AXES)
    .with_capability_evidence(S13_FEEDBACK_STACK_CAPABILITY_EVIDENCE)
    .with_component_slug("feedback-stack"),
];
