//! Data-only component taxonomy conformance tests.

use std::collections::BTreeSet;

use kinetik_ui_core::{
    ActionContext, ActionDescriptor, ActionId, ActionSource, Color, Key, KeyEvent, KeyState,
    KeyboardInput, Modifiers, Point, PointerButtonState, PointerInput, Rect, SemanticActionKind,
    SemanticRole, SemanticValue, Size, TextureId, UiInput, UiMemory, Vec2, WidgetId,
    default_dark_theme,
};
use kinetik_ui_text::TextEditState;
use kinetik_ui_widgets::{
    COMPONENT_CONFORMANCE_MATRIX, COMPONENT_EVIDENCE, COMPONENT_METADATA, ColorFieldConfig,
    ComponentCapabilityAxis, ComponentCapabilityEvidence, ComponentCategory,
    ComponentConformanceStatus, ComponentConformanceValidationError, ComponentEvidenceCategory,
    ComponentEvidenceProof, ComponentMetadata, DropdownCloseReason, DropdownItem, DropdownItemId,
    DropdownModel, DropdownOverlay, EdgeDescriptor, GraphRect, JobList, JobPhase, JobProgress,
    JobRow, JobRowId, NodeDescriptor, NodeGraphDescriptor, NodeGraphPanZoom, NodeGraphSelection,
    NodeGraphSelectionTarget, NodeGraphStaticView, NodeGraphViewport, NodeId,
    NumericScrubInputConfig, OverlayId, OverlayStack, PanZoom, PanelId, PopoverPlacement,
    PortDescriptor, PortDirection, PortEndpoint, PortId, PortTypeId, PropertyGridAffordanceLayout,
    PropertyGridLayout, PropertyGridRow, PropertyGridRowAffordances, PropertyGridRowState,
    PropertyGridRowStatus, PropertyGridStatusSeverity, RadioGroupChoice, SliderStep, TabStrip,
    TimelineDescriptor, TimelineFrameRate, TimelineId, TimelineItemDescriptor, TimelineItemId,
    TimelineLaneDescriptor, TimelineLaneId, TimelineRange, TimelineRulerTickRequest,
    TimelineSelection, TimelineSelectionTarget, TimelineSnapCandidateRequest, TimelineSnapSource,
    TimelineZoom, TransportControlIntent, TransportControls, Ui, VectorComponentLayout,
    VectorScrubInputConfig, ViewportActionDescriptor, ViewportActionKind, ViewportActionRequest,
    ViewportActionTarget, ViewportCursorMetadata, ViewportCursorRequest,
    ViewportCursorRequestSource, ViewportCursorShape, ViewportFit, ViewportGuideDescriptor,
    ViewportGuideId, ViewportGuideOrientation, ViewportGuidePlacement, ViewportOverlayDescriptor,
    ViewportOverlayId, ViewportOverlayKind, ViewportOverlaySpace, ViewportPanZoomHudDescriptor,
    ViewportRulerDescriptor, ViewportRulerEdge, ViewportRulerId, ViewportSafeAreaDescriptor,
    ViewportSafeAreaId, ViewportSafeAreaSpace, ViewportSurface, ViewportToolDescriptor,
    ViewportToolId, classify_numeric_input_draft, component_conformance_matrix_by_stage,
    component_conformance_matrix_row, component_evidence, component_evidence_for,
    component_metadata, component_status_evidence, components_by_category,
    components_by_evidence_category, hit_test_viewport_overlays, numeric_input,
    numeric_scrub_input, property_grid_row_affordance_controls, property_grid_row_affordance_rects,
    property_grid_row_status_semantics, slider_with_step, timeline_snap_candidates,
    validate_component_capability_claim, validate_component_conformance_matrix_row,
    validate_component_metadata, vector4_component_rects, viewport_action_requests,
    viewport_cursor_request, viewport_guides, viewport_rulers, viewport_safe_areas,
};

fn entry(name: &str) -> &'static ComponentMetadata {
    component_metadata(name).unwrap_or_else(|| panic!("missing metadata for {name}"))
}

fn assert_entry(name: &str, category: ComponentCategory, status: ComponentConformanceStatus) {
    let metadata = entry(name);
    assert_eq!(metadata.category, category, "{name} category");
    assert_eq!(metadata.status, status, "{name} status");
}

fn evidence_categories(metadata: &ComponentMetadata) -> BTreeSet<ComponentEvidenceCategory> {
    component_evidence_for(metadata)
        .map(|evidence| evidence.category)
        .collect()
}

fn matrix_entry(slug: &str) -> &'static kinetik_ui_widgets::ComponentConformanceMatrixRow {
    component_conformance_matrix_row(slug)
        .unwrap_or_else(|| panic!("missing conformance matrix row for {slug}"))
}

fn metadata_by_slug(slug: &str) -> Option<&'static ComponentMetadata> {
    COMPONENT_METADATA
        .iter()
        .find(|metadata| metadata.slug == slug)
}

fn matrix_evidence_categories(slug: &str) -> BTreeSet<ComponentEvidenceCategory> {
    matrix_entry(slug)
        .evidence_ids
        .iter()
        .filter_map(|id| component_evidence(id))
        .map(|evidence| evidence.category)
        .collect()
}

fn assert_stage_entry(
    name: &str,
    stage: u8,
    category: ComponentCategory,
    status: ComponentConformanceStatus,
) {
    let metadata = entry(name);
    assert_eq!(metadata.category, category, "{name} category");
    assert_eq!(metadata.status, status, "{name} status");
    assert_eq!(metadata.stage, Some(stage), "{name} stage");
    assert!(
        !component_status_evidence(metadata)
            .collect::<Vec<_>>()
            .is_empty(),
        "{name} status evidence"
    );

    let categories = evidence_categories(metadata);
    for category in [
        ComponentEvidenceCategory::Status,
        ComponentEvidenceCategory::Stage,
        ComponentEvidenceCategory::Conformance,
        ComponentEvidenceCategory::Showcase,
    ] {
        assert!(
            categories.contains(&category),
            "{name} missing {category:?}"
        );
    }
}

fn item(raw: u64, label: &str) -> DropdownItem {
    DropdownItem::new(DropdownItemId::from_raw(raw), label)
}

fn pressed_key(key: Key) -> UiInput {
    UiInput {
        keyboard: KeyboardInput {
            modifiers: Modifiers::default(),
            events: vec![KeyEvent::new(
                key,
                KeyState::Pressed,
                Modifiers::default(),
                false,
            )],
        },
        ..UiInput::default()
    }
}

fn assert_close(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < f32::EPSILON,
        "expected {actual} to equal {expected}"
    );
}

fn viewport_action_and_cursor_contracts(
    surface: ViewportSurface,
    viewport_id: WidgetId,
) -> (Vec<ViewportActionRequest>, ViewportCursorRequest) {
    let mut overlay_action = ActionDescriptor::new("viewport.overlay.grid", "Grid");
    overlay_action.state.checked = Some(true);
    let actions = [
        ViewportActionDescriptor::new(
            ActionDescriptor::new("viewport.fit.content", "Fit Content"),
            ViewportActionKind::FitContent,
            ViewportActionTarget::new(viewport_id),
        ),
        ViewportActionDescriptor::new(
            overlay_action,
            ViewportActionKind::ToggleOverlay,
            ViewportActionTarget::new(viewport_id).with_overlay(ViewportOverlayId::from_raw(1)),
        ),
    ];
    let action_requests = viewport_action_requests(
        &actions,
        ActionSource::Button,
        &ActionContext::Widget(viewport_id),
    );
    let tool = ViewportToolDescriptor::new(ViewportToolId::from_raw(1), "Pan")
        .active(true)
        .with_cursor(ViewportCursorMetadata::new(ViewportCursorShape::Grab));
    let overlay_hit = hit_test_viewport_overlays(
        surface,
        &[ViewportOverlayDescriptor::new(
            ViewportOverlayId::from_raw(1),
            ViewportOverlayKind::ToolRegion,
            Rect::new(12.0, 22.0, 120.0, 80.0),
            ViewportOverlaySpace::Screen,
        )
        .with_tool(ViewportToolId::from_raw(1))
        .with_cursor(ViewportCursorMetadata::new(ViewportCursorShape::Crosshair))],
        Point::new(20.0, 30.0),
    )
    .expect("taxonomy viewport overlay hit");
    let cursor_request =
        viewport_cursor_request(viewport_id, None, None, Some(&overlay_hit), Some(&tool))
            .expect("taxonomy cursor request");

    (action_requests, cursor_request)
}

#[path = "component_taxonomy_conformance/capabilities.rs"]
mod capabilities;
#[path = "component_taxonomy_conformance/controls.rs"]
mod controls;
#[path = "component_taxonomy_conformance/inspector_contracts.rs"]
mod inspector_contracts;
#[path = "component_taxonomy_conformance/matrix.rs"]
mod matrix;
#[path = "component_taxonomy_conformance/registry.rs"]
mod registry;
#[path = "component_taxonomy_conformance/stages.rs"]
mod stages;
