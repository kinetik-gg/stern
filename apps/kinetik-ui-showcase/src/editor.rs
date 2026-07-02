//! DCC-style editor showcase surface.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::too_many_lines
)]

mod showcase;
#[cfg(test)]
mod tests;

use std::time::Duration;

use kinetik_ui::core::{
    ActionContext, ActionDescriptor, ActionIcon, ActionId, ActionInvocation, ActionQueue,
    ActionSource, Axis, Brush, ClipId, Color, CornerRadius, CursorShape, ImagePrimitive, Key,
    KeyState, LinePrimitive, Modifiers, PlatformRequest, Point, Primitive, Rect, RectPrimitive,
    RepaintRequest, Response, SemanticNode, SemanticRole, Shortcut, Size, Stroke, TextPrimitive,
    TextureId, Theme, Vec2, WidgetId,
};
use kinetik_ui::render::{
    ImageAtlasRegion, ImageResource, RenderImage, RenderImageSampling, RenderResources,
    TextureResource,
};
use kinetik_ui::text::TextEditState;
use kinetik_ui::widgets::{
    AssetSlotAsset, AssetSlotConfig, DiagnosticSource, DiagnosticStrip, DiagnosticStripItem,
    DiagnosticStripItemId, DiagnosticStripSeverity, Dock, DockChromeStyle, DockDropTarget,
    DockDropZone, DockInteractionPolicy, DockNode, DockPlacement, DockSplitterContextActionKind,
    DockTabDrag, DropdownItem, DropdownItemId, DropdownModel, EdgeDescriptor, EdgeId,
    FeedbackAction, FeedbackDismiss, FeedbackId, FeedbackItem, FeedbackKind, FeedbackStack, Frame,
    FrameId, FrameLayout, FrameTab, GraphPoint, GraphRect, GraphVector, GridColumns, GridLayout,
    Guide, ItemId, JobCancel, JobList, JobPhase, JobProgress, JobRow, JobRowId, ListLayout, Menu,
    MenuBar, MenuBarMenu, MenuBarMenuId, MenuBarOverlayRequest, MenuItem, MenuOverlay, ModalAction,
    ModalActionRole, ModalDialog, ModalDialogOverlay, NodeDescriptor, NodeFrameDescriptor,
    NodeFrameId, NodeGraphDescriptor, NodeGraphEdgeRoutePoint, NodeGraphEmissionError,
    NodeGraphPanZoom, NodeGraphSelection, NodeGraphSelectionTarget, NodeGraphStaticOutput,
    NodeGraphStaticView, NodeGraphViewport, NodeGroupDescriptor, NodeGroupId, NodeId,
    NumericScrubInputConfig, OverlayDismissal, OverlayId, OverlayKind, OverlayStack, PanZoom,
    Panel, PanelId, PanelInstanceId, PanelInstancePolicy, PanelInstanceSnapshot,
    PanelOpenActionMetadata, PanelOpenDecision, PanelRegistry, PanelTypeCategory,
    PanelTypeDescriptor, PanelTypeId, PanelWorkspaceContext, PathFieldConfig, PopoverPlacement,
    PortDescriptor, PortDirection, PortEndpoint, PortId, PortTypeId, PropertyGridAffordanceLayout,
    PropertyGridLayout, PropertyGridRow, PropertyGridRowStatus, RerouteDescriptor, RerouteId,
    SelectFieldConfig, StatusBar, StatusItem, StatusItemId, StatusItemKind, StatusProgress,
    TabStrip, TableColumn, TableLayout, Toolbar, ToolbarGroup, ToolbarGroupId, ToolbarItem,
    ToolbarItemPresentation, TreeExpansion, TreeItem, TreeLayout, TreeModel, Ui,
    VectorScrubInputConfig, ViewportComposition, ViewportFit, ViewportSurface, WorkspaceSnapshot,
    classify_numeric_input_draft, frame_tabs, icon_button_semantics,
    property_grid_row_affordance_rects, resolve_dock_splitter_context_actions_with_policy,
    resolve_frame_drop_zone_with_policy, solve_dock_layout, solve_dock_splitters_with_style,
};

include!("editor/root_state.rs");
include!("editor/resources.rs");
include!("editor/models.rs");
include!("editor/fixtures_paint.rs");
