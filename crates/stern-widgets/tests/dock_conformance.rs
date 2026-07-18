//! Windowless Dock/Frame/Panel model conformance tests.

use stern_core::{ActionId, Axis, Point, Rect, Size, Vec2};
use stern_widgets::{
    DiagnosticFieldValue, DiagnosticSource, DiagnosticStrip, DiagnosticStripItemId,
    DiagnosticStripSeverity, Dock, DockChromeStyle, DockDropTarget, DockInteractionPolicy,
    DockNeighborDirection, DockNode, DockPathElement, DockPlacement, DockRestoreError,
    DockSnapshot, DockSnapshotDiagnosticCode, DockSnapshotNode, DockSnapshotSplitValue,
    DockSplitInsertion, DockSplitPath, DockSplitterContextAction, DockSplitterContextActionKind,
    DockSplitterSide, Frame, FrameId, FrameLayout, FrameNeighbors, FrameSplitAffordanceRequest,
    Panel, PanelAffordances, PanelClosePolicy, PanelDockHint, PanelDuplicatePolicy,
    PanelFloatPolicy, PanelId, PanelInstanceId, PanelInstanceLocation, PanelInstancePolicy,
    PanelInstanceSnapshot, PanelOpenActionMetadata, PanelOpenDecision, PanelPolicyContext,
    PanelPolicyMetadata, PanelPolicyUnavailableReason, PanelRegistry, PanelRegistryError,
    PanelTypeCategory, PanelTypeDescriptor, PanelTypeId, PanelWorkspaceContext,
    SnapshotDiagnosticSeverity, WorkspaceRepairAction, WorkspaceRepairActionCode,
    WorkspaceRestoreError, WorkspaceSnapshot, WorkspaceSnapshotDiagnosticCode, frame_neighbor,
    frame_tabs, resolve_dock_drop_target, resolve_dock_drop_target_with_policy,
    resolve_dock_join_request, resolve_dock_splitter_context_actions,
    resolve_dock_splitter_context_actions_with_policy, resolve_dock_swap_request,
    resolve_frame_drop_zone_with_policy, resolve_frame_split_affordance_request,
    resolve_frame_split_affordance_request_with_policy, resolve_panel_affordances,
    resolve_panel_close_request, resolve_panel_duplicate_request, resolve_panel_float_request,
    resolve_panel_open_decision, resolve_panel_policy_context, solve_dock_layout,
    solve_dock_neighbors, solve_dock_splitters, solve_dock_splitters_with_style,
    split_ratio_from_drag,
};

fn panel(id: u64, title: &str) -> Panel {
    Panel::new(PanelId::from_raw(id), title)
}

fn frame(id: u64, panels: Vec<Panel>) -> Frame {
    Frame::new(FrameId::from_raw(id), panels)
}

fn nested_dock() -> Dock {
    Dock::new(DockNode::Split {
        axis: Axis::Horizontal,
        ratio: 0.3,
        min_first: 80.0,
        min_second: 120.0,
        first: Box::new(DockNode::Frame(frame(1, vec![panel(1, "Media")]))),
        second: Box::new(DockNode::Split {
            axis: Axis::Vertical,
            ratio: 0.6,
            min_first: 90.0,
            min_second: 110.0,
            first: Box::new(DockNode::Frame(frame(
                2,
                vec![panel(2, "Viewport"), panel(3, "Inspector")],
            ))),
            second: Box::new(DockNode::Frame(frame(3, vec![panel(4, "Timeline")]))),
        }),
    })
}

fn assert_close(left: f32, right: f32) {
    assert!(
        (left - right).abs() <= 0.001,
        "expected {left} to be close to {right}"
    );
}

fn frame_rect(dock: &Dock, frame: u64, bounds: Rect) -> Rect {
    solve_dock_layout(dock, bounds)
        .into_iter()
        .find(|layout| layout.frame == FrameId::from_raw(frame))
        .expect("frame layout")
        .rect
}

fn neighbors_for(neighbors: &[FrameNeighbors], frame: u64) -> FrameNeighbors {
    neighbors
        .iter()
        .find(|neighbors| neighbors.frame == FrameId::from_raw(frame))
        .copied()
        .expect("frame neighbors")
}

fn panel_ids(frame: &Frame) -> Vec<PanelId> {
    frame.panels.iter().map(|panel| panel.id).collect()
}

fn field_value<'a>(
    fields: &'a [stern_widgets::DiagnosticField],
    name: &str,
) -> Option<&'a DiagnosticFieldValue> {
    fields
        .iter()
        .find(|field| field.name == name)
        .map(|field| &field.value)
}

fn splitter_context_action(
    actions: &[DockSplitterContextAction],
    kind: DockSplitterContextActionKind,
    source_side: DockSplitterSide,
) -> &DockSplitterContextAction {
    actions
        .iter()
        .find(|action| action.kind == kind && action.source_side == source_side)
        .expect("splitter context action")
}

fn workspace_panel_descriptors() -> Vec<PanelTypeDescriptor> {
    vec![
        PanelTypeDescriptor::new(PanelTypeId::from_raw(10), "Media"),
        PanelTypeDescriptor::new(PanelTypeId::from_raw(20), "Viewport"),
        PanelTypeDescriptor::new(PanelTypeId::from_raw(30), "Inspector"),
        PanelTypeDescriptor::new(PanelTypeId::from_raw(40), "Timeline"),
    ]
}

fn workspace_panel_instances() -> Vec<PanelInstanceSnapshot> {
    vec![
        PanelInstanceSnapshot::new(
            PanelInstanceId::from_raw(1),
            PanelTypeId::from_raw(10),
            "Media",
        )
        .with_state_key("media-state"),
        PanelInstanceSnapshot::new(
            PanelInstanceId::from_raw(2),
            PanelTypeId::from_raw(20),
            "Viewport",
        ),
        PanelInstanceSnapshot::new(
            PanelInstanceId::from_raw(3),
            PanelTypeId::from_raw(30),
            "Inspector",
        ),
        PanelInstanceSnapshot::new(
            PanelInstanceId::from_raw(4),
            PanelTypeId::from_raw(40),
            "Timeline",
        ),
    ]
}

include!("dock_conformance/registry.rs");
include!("dock_conformance/panel_policy.rs");
include!("dock_conformance/workspace_snapshot.rs");
include!("dock_conformance/diagnostics.rs");
include!("dock_conformance/layout_policy.rs");
include!("dock_conformance/neighbors_join_swap.rs");
include!("dock_conformance/interactions_snapshot.rs");
