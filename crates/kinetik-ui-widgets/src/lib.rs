//! Reusable widgets built from Kinetik UI core primitives.

pub mod asset_browser;
pub mod chrome;
pub mod collection_actions;
pub mod collections;
mod components;
pub mod dock;
pub mod inline_edit;
pub mod inspector;
pub mod node_graph;
pub mod outliner;
pub mod overlays;
pub mod taxonomy;
pub mod timeline;
pub mod ui;
pub mod viewport;

// Root exports are explicit compatibility exports. New advanced or staging APIs
// should remain module-qualified unless they are intentionally promoted here or
// into `prelude`.
pub use asset_browser::{
    AssetBrowserDropTarget, AssetBrowserDropTargetKind, AssetBrowserItem, AssetBrowserItemRect,
    AssetBrowserItemState, AssetBrowserLayout, AssetBrowserLayoutResult, AssetBrowserModel,
    AssetBrowserModelError, AssetBrowserResolvedItem, AssetBrowserSelectionOperation,
    AssetBrowserSelectionRequest, AssetBrowserSort, AssetBrowserSortKey, AssetBrowserViewMode,
    AssetIconFallback, asset_browser_item_widget_id, asset_browser_semantics,
};
pub use chrome::{
    ActiveJobProgress, ChromeOverflowItem, ChromeOverflowPlacement, ChromeOverflowProjection,
    ChromeOverflowRequest, ChromeOverflowTrigger, ChromeScene, ChromeSceneConfig,
    ChromeSceneIntent, ChromeSceneItemKey, ChromeSceneOutput, ChromeSurfaceKind, DiagnosticField,
    DiagnosticFieldValue, DiagnosticSource, DiagnosticStrip, DiagnosticStripItem,
    DiagnosticStripItemId, DiagnosticStripSeverity, DiagnosticStripSummary, FeedbackAction,
    FeedbackActionRequest, FeedbackDismiss, FeedbackDismissRequest, FeedbackId, FeedbackItem,
    FeedbackKind, FeedbackLifetime, FeedbackStack, JobCancel, JobCancelRequest, JobList, JobPhase,
    JobProgress, JobRow, JobRowId, JobSummaryCounts, MenuBar, MenuBarMenu, MenuBarMenuId,
    MenuBarMove, MenuBarOverlayRequest, StatusBar, StatusItem, StatusItemId, StatusItemKind,
    StatusProgress, TabStrip, TabStripMove, TabStripTarget, Toolbar, ToolbarGroup, ToolbarGroupId,
    ToolbarItem, ToolbarItemPresentation, project_chrome_overflow,
};
pub use collection_actions::{
    CollectionBackgroundContextTarget, CollectionContextAction, CollectionContextActionRequest,
    CollectionContextTarget, CollectionDragSource, CollectionItemContextTarget,
    CollectionSelectionContextTarget, collection_context_actions,
};
pub use collections::{
    CollectionCursor, CollectionCursorMove, CollectionCursorTarget, CollectionProjectedItem,
    CollectionProjection, GridColumns, GridLayout, ItemId, ItemRect, ListLayout, Selection,
    SelectionProjection, SelectionProjectionPolicy, SortDirection, TableCellRect, TableColumn,
    TableColumnConstraints, TableHeaderRect, TableLayout, TableSort, TreeExpansion, TreeItem,
    TreeLayout, TreeModel, TreeModelError, TreeRow, TreeRowRect, VirtualList, VirtualListConfig,
    VirtualListItemResponse, VirtualListOutput, VirtualListRow, VirtualListSelectionMode,
    VirtualRangeRequest, VirtualTree, VirtualTreeConfig, VirtualTreeItemResponse,
    VirtualTreeOutput, VirtualTreeRow, VirtualTreeSelectionMode, VirtualWindow,
    VirtualWindowRequest, clamp_virtual_scroll_offset, virtual_content_extent,
    virtual_max_scroll_offset, virtual_range, virtual_window,
};
pub use components::{
    AssetSlotAsset, AssetSlotConfig, AssetSlotOutput, ColorFieldConfig, ColorFieldOutput,
    IconGraphic, IconLibrary, IconPath, MultiLineTextFieldOutput, NumericInputDraft,
    NumericInputOutput, NumericInputPolicy, NumericScrubInputConfig, NumericScrubInputOutput,
    PanelFrame, PathFieldConfig, PathFieldOutput, SearchFieldOutput, SelectFieldConfig,
    SelectFieldOutput, SliderStep, TextFieldAccess, TextFieldOutput, VectorScrubInputConfig,
    VectorScrubInputOutput, WidgetOutput, asset_slot_field, button, button_semantics, checkbox,
    checkbox_semantics, checkbox_with_label, checkbox_with_label_target, choice_label_target_rect,
    classify_numeric_input_draft, color_field, icon_button, icon_button_semantics,
    icon_button_with_label, icon_button_with_library, image, image_icon_button,
    image_icon_button_sized, image_icon_selectable_button, image_icon_selectable_button_sized,
    image_semantics, label, label_semantics, list_row, multi_line_text_field,
    multi_line_text_field_with_text_layouts, numeric_input, numeric_input_with_text_layouts,
    numeric_scrub_input, numeric_scrub_input_with_text_layouts, panel, panel_semantics, path_field,
    path_field_with_text_layouts, radio_button, radio_button_semantics, radio_button_with_label,
    radio_button_with_label_target, restore_text_draft, search_field, search_field_semantics,
    search_field_with_text_layouts, select_field, separator, slider, slider_semantics,
    slider_with_label, slider_with_label_and_step, slider_with_step, tab_button, text_field,
    text_field_semantics, text_field_with_text_layouts, toggle, toggle_semantics,
    toggle_with_label, toggle_with_label_target, vector2_scrub_input, vector3_scrub_input,
    vector4_scrub_input,
};
pub(crate) use components::{
    multi_line_text_field_with_access_runtime,
    multi_line_text_field_with_text_layouts_and_caret_visibility,
    numeric_input_with_access_runtime, numeric_input_with_text_layouts_and_caret_visibility,
    numeric_scrub_input_with_runtime, numeric_scrub_input_with_text_layouts_and_caret_visibility,
    path_field_with_access_runtime, path_field_with_text_layouts_and_caret_visibility,
    search_field_with_access_runtime, search_field_with_text_layouts_and_caret_visibility,
    text_field_with_access_runtime, text_field_with_text_layouts_and_caret_visibility,
    vector_scrub_input_with_runtime, vector_scrub_input_with_text_layouts_and_caret_visibility,
};
pub use dock::{
    Dock, DockChromeStyle, DockDropTarget, DockDropTargetPolicy, DockDropZone,
    DockInteractionPolicy, DockJoinRequest, DockNeighborDirection, DockNode, DockPathElement,
    DockPlacement, DockRestoreError, DockSnapshot, DockSnapshotDiagnostic,
    DockSnapshotDiagnosticCode, DockSnapshotDiagnostics, DockSnapshotNode, DockSnapshotSplitValue,
    DockSplitInsertion, DockSplitPath, DockSplitter, DockSplitterActionContext,
    DockSplitterContextAction, DockSplitterContextActionKind, DockSplitterInteractionPolicy,
    DockSplitterSide, DockSwapRequest, DockTabDrag, Frame, FrameId, FrameLayout, FrameNeighbors,
    FrameSplitAffordanceRequest, FrameTab, Panel, PanelAffordances, PanelClosePolicy,
    PanelCloseRequest, PanelDockHint, PanelDuplicatePolicy, PanelDuplicateRequest,
    PanelFloatPolicy, PanelFloatRequest, PanelFocusRequest, PanelId, PanelInstanceId,
    PanelInstanceLocation, PanelInstancePolicy, PanelInstanceSnapshot, PanelOpenActionMetadata,
    PanelOpenDecision, PanelOpenRequest, PanelPolicyContext, PanelPolicyMetadata,
    PanelPolicyResolution, PanelPolicyUnavailableReason, PanelRegistry, PanelRegistryError,
    PanelTypeCategory, PanelTypeDescriptor, PanelTypeId, PanelWorkspaceContext,
    SnapshotDiagnosticSeverity, WorkspaceRepairAction, WorkspaceRepairActionCode,
    WorkspaceRepairPlan, WorkspaceRestoreError, WorkspaceSnapshot, WorkspaceSnapshotDiagnostic,
    WorkspaceSnapshotDiagnosticCode, WorkspaceSnapshotDiagnostics, frame_neighbor, frame_tabs,
    locate_panel_instance, resolve_dock_drop_target, resolve_dock_drop_target_with_policy,
    resolve_dock_join_request, resolve_dock_splitter_context_actions,
    resolve_dock_splitter_context_actions_with_policy, resolve_dock_swap_request,
    resolve_frame_drop_zone, resolve_frame_drop_zone_with_policy,
    resolve_frame_split_affordance_request, resolve_frame_split_affordance_request_with_policy,
    resolve_panel_affordances, resolve_panel_close_request, resolve_panel_duplicate_request,
    resolve_panel_float_request, resolve_panel_open_decision, resolve_panel_policy_context,
    solve_dock_layout, solve_dock_neighbors, solve_dock_splitters, solve_dock_splitters_with_style,
    split_ratio_from_drag, validate_dock_snapshot_diagnostics,
    validate_workspace_snapshot_diagnostics,
};
pub use inline_edit::{
    InlineEditBeginRequest, InlineEditCancelReason, InlineEditCancelRequest,
    InlineEditCommitReason, InlineEditCommitRequest, InlineEditDraftDisposition,
    InlineEditDraftEdit, InlineEditDraftPolicy, InlineEditDraftStatus, InlineEditEligibility,
    InlineEditFocusLossPolicy, InlineEditRequest, InlineEditResolution, InlineEditSession,
    classify_inline_edit_draft, inline_edit_widget_id,
};
pub use inspector::{
    PropertyGridAffordanceLayout, PropertyGridAffordanceOutput, PropertyGridAffordanceRects,
    PropertyGridError, PropertyGridKeyframeAffordance, PropertyGridLayout,
    PropertyGridResetAffordance, PropertyGridRow, PropertyGridRowAffordances, PropertyGridRowKind,
    PropertyGridRowRect, PropertyGridRowState, PropertyGridRowStatus,
    PropertyGridStatusPresentation, PropertyGridStatusSeverity, VectorComponentLayout,
    VectorComponentRect, property_grid_row_affordance_controls, property_grid_row_affordance_rects,
    property_grid_row_status_semantics, vector2_component_rects, vector3_component_rects,
    vector4_component_rects,
};
pub use kinetik_ui_core::IconId;
pub use node_graph::{
    DEFAULT_NODE_GRAPH_CONTEXT_ACTION_KINDS, DEFAULT_NODE_GRAPH_EDGE_HIT_TOLERANCE,
    DEFAULT_NODE_GRAPH_PORT_HIT_SIZE, DEFAULT_NODE_GRAPH_REROUTE_HIT_SIZE,
    DEFAULT_NODE_GRAPH_TITLE_BAR_HEIGHT, EdgeDescriptor, EdgeEndpointRole, EdgeId,
    EdgeResolutionError, GraphPoint, GraphRect, GraphVector, NodeDescriptor, NodeFrameDescriptor,
    NodeFrameId, NodeGraphAddNodeDescriptorId, NodeGraphAddNodeRequest,
    NodeGraphAddNodeSearchEntry, NodeGraphAddNodeSearchHighlight, NodeGraphAddNodeSearchMatch,
    NodeGraphAddNodeSearchSelection, NodeGraphAnnotationField, NodeGraphAnnotationRequest,
    NodeGraphBoxSelection, NodeGraphBoxSelectionMode, NodeGraphBoxSelectionRequest,
    NodeGraphCanvasPanRequest, NodeGraphCollapseLinkMetadata, NodeGraphCollapseRequest,
    NodeGraphCollapseTarget, NodeGraphContextAction, NodeGraphContextActionKind,
    NodeGraphContextActionRequest, NodeGraphContextActionUnavailableReason,
    NodeGraphContextCanvasOperation, NodeGraphContextCanvasRequest,
    NodeGraphContextDetachEndpointRequest, NodeGraphContextDisconnectRequest,
    NodeGraphContextDisconnectTarget, NodeGraphContextOrganizationOperation,
    NodeGraphContextOrganizationRequest, NodeGraphContextSelectionRequest, NodeGraphContextTarget,
    NodeGraphCreateLinkRequest, NodeGraphCutLinkRequest, NodeGraphDescriptor,
    NodeGraphDetachLinkRequest, NodeGraphEdgeRoutePoint, NodeGraphEmissionError,
    NodeGraphFrameMove, NodeGraphFrameMoveRequest, NodeGraphGridStyle, NodeGraphHitTarget,
    NodeGraphHitTestConfig, NodeGraphHitTestError, NodeGraphLinkDraft, NodeGraphLinkDraftCancelled,
    NodeGraphLinkDraftCompleted, NodeGraphLinkDraftCompletionError, NodeGraphLinkDraftEndpoint,
    NodeGraphLinkDraftEndpointError, NodeGraphLinkDraftOutcome, NodeGraphLinkDraftPortTarget,
    NodeGraphLinkDraftRejected, NodeGraphLinkDraftTarget, NodeGraphLinkDraftTargetError,
    NodeGraphLinkEditEdgeContext, NodeGraphLinkEditRequest, NodeGraphLinkEditRequestError,
    NodeGraphNodeMove, NodeGraphNodeStateAction, NodeGraphNodeStateRequest,
    NodeGraphOrganizationRequestError, NodeGraphOrganizationTarget, NodeGraphPanZoom,
    NodeGraphPortState, NodeGraphPortStyle, NodeGraphReconnectLinkSourceRequest,
    NodeGraphReconnectLinkTargetRequest, NodeGraphSelectedNodeMoveRequest, NodeGraphSelection,
    NodeGraphSelectionIntent, NodeGraphSelectionOperation, NodeGraphSelectionTarget,
    NodeGraphStaticOutput, NodeGraphStaticView, NodeGraphStyle, NodeGraphValidationError,
    NodeGraphViewport, NodeGroupDescriptor, NodeGroupId, NodeId, PortCompatibilityError,
    PortDescriptor, PortDirection, PortEndpoint, PortId, PortTypeId, RerouteDescriptor, RerouteId,
    ResolvedEdge, ResolvedEdgeRoutePoint, ResolvedEndpoint,
    filter_node_graph_add_node_search_entries, hit_test_node_graph,
    hit_test_node_graph_with_config, node_graph_drag_delta, node_graph_snap_delta,
    node_graph_snap_point, node_graph_snap_rect, ports_are_compatible, resolve_node_graph_edges,
    validate_node_graph_descriptors, validate_port_compatibility,
};
pub use outliner::{
    OutlinerDropTarget, OutlinerDropZoneKind, OutlinerExpansionRequest, OutlinerItem,
    OutlinerLayout, OutlinerLockToggleRequest, OutlinerModel, OutlinerResourceMetadata,
    OutlinerRow, OutlinerRowFlags, OutlinerRowZoneKind, OutlinerRowZones,
    OutlinerSelectionOperation, OutlinerSelectionRequest, OutlinerVisibilityToggleRequest,
    outliner_context_target_at, outliner_row_widget_id, outliner_semantics,
};
pub use overlays::{
    CommandPalette, CommandPaletteEntry, CommandPaletteOverlay, DropdownCloseReason,
    DropdownCloseResult, DropdownHighlightMove, DropdownItem, DropdownItemId, DropdownModel,
    DropdownOverlay, DropdownTriggerPresentation, DropdownVisibleRange, Menu, MenuItem,
    MenuOverlay, ModalAction, ModalActionRole, ModalCloseReason, ModalCloseRequest, ModalDialog,
    ModalDialogBody, ModalDialogOverlay, ModalFocusContainment, OverlayDismissal, OverlayEntry,
    OverlayId, OverlayKind, OverlayScene, OverlaySceneDismissReason, OverlaySceneDismissRequest,
    OverlaySceneDropdownSelection, OverlaySceneIntent, OverlaySceneMetrics, OverlaySceneOutput,
    OverlaySceneSurface, OverlayStack, PopoverPlacement, PopoverRequest, dropdown_visible_range,
    overlay_semantics, place_popover,
};
pub use taxonomy::{
    COMPONENT_CONFORMANCE_MATRIX, COMPONENT_EVIDENCE, COMPONENT_METADATA, ComponentCapabilityAxis,
    ComponentCapabilityEvidence, ComponentCategory, ComponentConformanceMatrixRow,
    ComponentConformanceStatus, ComponentConformanceValidationError, ComponentEvidence,
    ComponentEvidenceCategory, ComponentEvidenceProof, ComponentMetadata,
    component_conformance_matrix_by_stage, component_conformance_matrix_row, component_evidence,
    component_evidence_for, component_metadata, component_status_evidence, components_by_category,
    components_by_evidence_category, validate_component_capability_claim,
    validate_component_conformance_matrix_row, validate_component_metadata,
};
pub use timeline::{
    DEFAULT_TIMELINE_PIXELS_PER_SECOND, DEFAULT_TIMELINE_RULER_MAX_TICKS,
    MAX_TIMELINE_PIXELS_PER_SECOND, MIN_TIMELINE_PIXELS_PER_SECOND, ResolvedTimelineItem,
    ResolvedTimelineKeyframe, ResolvedTimelineLane, ResolvedTimelineMarker, TimelineClipDescriptor,
    TimelineClipId, TimelineClipMoveRequest, TimelineClipSelectionRequest, TimelineClipTrimRequest,
    TimelineDescriptor, TimelineDescriptorError, TimelineDescriptorState, TimelineFrame,
    TimelineFrameRate, TimelineFrameRounding, TimelineHitMetadata, TimelineHitTarget,
    TimelineHitTestConfig, TimelineId, TimelineItemDescriptor, TimelineItemId,
    TimelineKeyframeDescriptor, TimelineKeyframeId, TimelineKeyframeSelectionRequest,
    TimelineLaneDescriptor, TimelineLaneId, TimelineLayout, TimelineLayoutResult,
    TimelineMarkerContextRequest, TimelineMarkerDescriptor, TimelineMarkerId,
    TimelineMarkerSelectionRequest, TimelinePlayheadSeekRequest, TimelineRange,
    TimelineRangeSelectionBeginRequest, TimelineRangeSelectionEndRequest,
    TimelineRangeSelectionUpdateRequest, TimelineRulerId, TimelineRulerTick, TimelineRulerTickKind,
    TimelineRulerTickRequest, TimelineScale, TimelineScrubBeginRequest, TimelineScrubEndRequest,
    TimelineScrubUpdateRequest, TimelineSelection, TimelineSelectionOperation,
    TimelineSelectionTarget, TimelineSnapCandidate, TimelineSnapCandidateRequest,
    TimelineSnapMetadata, TimelineSnapSource, TimelineTime, TimelineTrackDescriptor,
    TimelineTrackId, TimelineTransportContext, TimelineTrimEdge, TimelineViewportState,
    TimelineViewportZoomResult, TimelineZoom, TimelineZoomAnchorResult, TransportActionRequest,
    TransportControlDescriptor, TransportControlId, TransportControlIntent, TransportControlKind,
    TransportControlSemanticRect, TransportControls, clamp_timeline_scroll_offset,
    max_timeline_scroll_offset, sanitize_timeline_zoom, timeline_clip_widget_id,
    timeline_item_semantics, timeline_item_widget_id, timeline_keyframe_semantics,
    timeline_keyframe_widget_id, timeline_lane_semantics, timeline_lane_widget_id,
    timeline_marker_semantics, timeline_marker_widget_id, timeline_root_semantics,
    timeline_ruler_ticks, timeline_semantics, timeline_snap_candidates, timeline_snap_time,
    timeline_timecode_label, transport_control_semantics, transport_control_widget_id,
    transport_controls_semantics,
};
pub use ui::{RadioGroupChoice, RadioGroupOutput, ScrollAreaOutput, Ui};
pub use viewport::{
    Crosshair, Guide, PanZoom, ViewportActionDescriptor, ViewportActionKind, ViewportActionRequest,
    ViewportActionTarget, ViewportComposition, ViewportCursorMetadata, ViewportCursorRequest,
    ViewportCursorRequestSource, ViewportCursorShape, ViewportFit, ViewportGuideDescriptor,
    ViewportGuideId, ViewportGuideOrientation, ViewportGuidePlacement, ViewportOverlayDescriptor,
    ViewportOverlayHit, ViewportOverlayId, ViewportOverlayKind, ViewportOverlaySpace,
    ViewportPanZoomHud, ViewportPanZoomHudDescriptor, ViewportResolvedGuide, ViewportResolvedRuler,
    ViewportResolvedSafeArea, ViewportRulerDescriptor, ViewportRulerEdge, ViewportRulerId,
    ViewportRulerTick, ViewportSafeAreaDescriptor, ViewportSafeAreaId, ViewportSafeAreaSpace,
    ViewportSelectionOutlineDescriptor, ViewportSelectionTargetDescriptor,
    ViewportSelectionTargetId, ViewportSelectionTargetState, ViewportSurface,
    ViewportToolDescriptor, ViewportToolId, ViewportToolSurfaceDescriptor,
    ViewportTransformDragCapture, ViewportTransformDragRequest, ViewportTransformDragStatus,
    ViewportTransformHandleDescriptor, ViewportTransformHandleHit, ViewportTransformHandleId,
    ViewportTransformHandleKind, ViewportTransformHandleSet, guide_primitives,
    hit_test_viewport_overlays, hit_test_viewport_overlays_at, hit_test_viewport_transform_handles,
    hit_test_viewport_transform_handles_at, ruler_ticks, viewport_action_requests,
    viewport_action_semantics, viewport_action_widget_id, viewport_actions_semantics,
    viewport_cursor_request, viewport_guide_widget_id, viewport_guides, viewport_guides_at,
    viewport_overlay_widget_id, viewport_ruler_widget_id, viewport_rulers, viewport_rulers_at,
    viewport_safe_area_widget_id, viewport_safe_areas, viewport_safe_areas_at,
    viewport_selection_outlines, viewport_selection_outlines_at, viewport_tool_semantics,
    viewport_tool_widget_id, viewport_transform_handles, viewport_transform_handles_at,
};

/// Common stable imports for application widget code.
///
/// Advanced editor models and staging surfaces remain available from their
/// modules, for example `kinetik_ui_widgets::dock::Dock` or
/// `kinetik_ui_widgets::node_graph::NodeGraphDescriptor`.
pub mod prelude {
    pub use crate::{
        AssetSlotAsset, AssetSlotConfig, AssetSlotOutput, ColorFieldConfig, ColorFieldOutput,
        IconGraphic, IconLibrary, IconPath, MultiLineTextFieldOutput, NumericInputDraft,
        NumericInputOutput, NumericInputPolicy, NumericScrubInputConfig, NumericScrubInputOutput,
        PanelFrame, PathFieldConfig, PathFieldOutput, RadioGroupChoice, RadioGroupOutput,
        ScrollAreaOutput, SearchFieldOutput, SelectFieldConfig, SelectFieldOutput, SliderStep,
        TextFieldOutput, Ui, VectorScrubInputConfig, VectorScrubInputOutput, ViewportSurface,
        WidgetOutput, asset_slot_field, button, checkbox, checkbox_with_label, color_field,
        icon_button, image, image_icon_button, image_icon_selectable_button, label,
        multi_line_text_field, numeric_input, numeric_scrub_input, panel, path_field, radio_button,
        radio_button_with_label, search_field, select_field, separator, slider, tab_button,
        text_field, toggle, toggle_with_label, vector2_scrub_input, vector3_scrub_input,
        vector4_scrub_input,
    };
}
