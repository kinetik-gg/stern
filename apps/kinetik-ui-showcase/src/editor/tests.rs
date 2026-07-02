#![allow(clippy::float_cmp)]

use std::time::Duration;

use super::{
    ACTION_GRID, ACTION_PLAY, ACTION_SAVE, ACTION_STOP, ACTION_VIEWPORT_ACTUAL_SIZE,
    ACTION_VIEWPORT_FIT_CONTENT, ACTION_VIEWPORT_FIT_SELECTION, ACTION_VIEWPORT_FOCUS_SELECTED,
    ACTION_VIEWPORT_PAN, ACTION_VIEWPORT_ZOOM_IN, ACTION_VIEWPORT_ZOOM_OUT, EditorChromeMetrics,
    EditorMenuKind, EditorShowcase, EditorStatusItemKind, EditorTool, EditorToolbarGroupKind,
    FRAME_BOTTOM, FRAME_INSPECTOR, FRAME_VIEWPORT, PANEL_TIMELINE, TOOLBAR_Y, VIEWPORT_SIZE,
    frame_tab_rects, frame_tab_strip, icon_atlas_image, inspector_label_width, item_id,
    phosphor_icons, register_resources, rgb, rgba,
};
use kinetik_ui::core::{
    ActionContext, ActionDescriptor, ActionId, ActionSource, Brush, CursorShape, FrameContext,
    PhysicalSize, PlatformRequest, Point, PointerButtonState, PointerInput, Primitive, Rect,
    RepaintRequest, ScaleFactor, SemanticActionKind, SemanticRole, Size, TimeInfo, UiInput,
    UiMemory, Vec2, ViewportInfo, WidgetId, default_dark_theme,
};
use kinetik_ui::render::RenderResources;
use kinetik_ui::widgets::{
    DockSplitterContextActionKind, FeedbackKind, GraphVector, JobPhase, MenuItem, ModalActionRole,
    NodeFrameId, NodeGraphContextActionKind, NodeGraphContextTarget, NodeGraphHitTarget,
    NodeGraphLinkEditRequest, NodeGraphSelection, NodeGraphSelectionTarget, NodeId,
    OverlayDismissal, OverlayKind, PanZoom, PanelOpenDecision, PanelTypeCategory, PortEndpoint,
    PortId, StatusItemKind, TimelineDescriptor, TimelineFrameRate, TimelineId,
    TimelineItemDescriptor, TimelineItemId, TimelineKeyframeDescriptor, TimelineKeyframeId,
    TimelineLaneDescriptor, TimelineLaneId, TimelineLayout, TimelineMarkerDescriptor,
    TimelineMarkerId, TimelineRange, TimelineScale, TimelineSelection, TimelineSelectionTarget,
    TimelineSnapCandidate, TimelineSnapCandidateRequest, TimelineSnapSource, TimelineTime,
    TimelineTransportContext, TimelineViewportState, TimelineZoom, TransportActionRequest,
    TransportControlDescriptor, TransportControlId, TransportControlIntent, TransportControls, Ui,
    ViewportActionDescriptor, ViewportActionKind, ViewportActionRequest, ViewportActionTarget,
    ViewportCursorMetadata, ViewportCursorRequest, ViewportCursorRequestSource,
    ViewportCursorShape, ViewportOverlayDescriptor, ViewportOverlayId, ViewportOverlayKind,
    ViewportOverlaySpace, ViewportSelectionTargetId, ViewportSurface, ViewportToolDescriptor,
    ViewportToolId, hit_test_viewport_overlays, resolve_dock_splitter_context_actions_with_policy,
    solve_dock_layout, solve_dock_splitters_with_style, timeline_semantics,
    timeline_snap_candidates, viewport_action_requests, viewport_actions_semantics,
    viewport_cursor_request,
};

struct EditorTimelineFixture {
    descriptor: TimelineDescriptor,
    candidates: Vec<TimelineSnapCandidate>,
    transport_request: TransportActionRequest,
    state: TimelineViewportState,
    semantic_roles: Vec<SemanticRole>,
}

struct EditorViewportToolFixture {
    actions: Vec<ViewportActionDescriptor>,
    requests: Vec<ViewportActionRequest>,
    cursor_request: ViewportCursorRequest,
    semantic_roles: Vec<SemanticRole>,
}

fn editor_timeline_fixture() -> EditorTimelineFixture {
    let timeline = TimelineId::from_raw(9_000);
    let descriptor = TimelineDescriptor::new(
        [
            TimelineLaneDescriptor::new(TimelineLaneId::from_raw(1), "Video"),
            TimelineLaneDescriptor::new(TimelineLaneId::from_raw(2), "Animation"),
        ],
        [
            TimelineItemDescriptor::new(
                TimelineItemId::from_raw(10),
                TimelineLaneId::from_raw(1),
                TimelineRange::seconds(0.0, 2.5),
                "Intro clip",
            ),
            TimelineItemDescriptor::new(
                TimelineItemId::from_raw(11),
                TimelineLaneId::from_raw(2),
                TimelineRange::seconds(1.0, 3.0),
                "Camera move",
            ),
        ],
        [TimelineMarkerDescriptor::new(
            TimelineMarkerId::from_raw(30),
            TimelineTime::from_seconds(1.5),
            "Beat",
        )],
        [TimelineKeyframeDescriptor::new(
            TimelineKeyframeId::from_raw(40),
            TimelineItemId::from_raw(11),
            TimelineTime::from_seconds(2.0),
        )],
    );
    let scale = TimelineScale::new(
        0.0,
        240.0,
        TimelineRange::seconds(0.0, 4.0),
        TimelineZoom::new(60.0),
        0.0,
    );
    let layout = TimelineLayout::new(24.0)
        .resolve(Rect::new(0.0, 0.0, 240.0, 48.0), scale, &descriptor, 0.0)
        .expect("editor timeline fixture resolves");
    let semantic_roles = timeline_semantics(
        WidgetId::from_key("editor.timeline.fixture"),
        layout.bounds,
        &layout,
        "Editor timeline",
    )
    .into_iter()
    .map(|node| node.role)
    .collect::<Vec<_>>();
    let candidates = timeline_snap_candidates(
        TimelineSnapCandidateRequest::new(
            timeline,
            scale.visible_range(),
            TimelineFrameRate::integer(24),
            &descriptor,
        )
        .with_selection_range(TimelineRange::seconds(0.5, 2.5))
        .with_playhead_time(TimelineTime::from_seconds(1.25)),
    );
    let selection = TimelineSelection::from_targets([TimelineSelectionTarget::Item(
        TimelineItemId::from_raw(11),
    )]);
    let state = TimelineViewportState::new(scale)
        .with_playhead_time(TimelineTime::from_seconds(1.25))
        .with_selection(selection)
        .with_selection_range(TimelineRange::seconds(0.5, 2.5));
    let transport = TransportControls::from_controls([
        TransportControlDescriptor::new(
            TransportControlId::from_raw(1),
            TransportControlIntent::PlayPause,
            ActionDescriptor::new(ACTION_PLAY, "Play"),
        ),
        TransportControlDescriptor::new(
            TransportControlId::from_raw(2),
            TransportControlIntent::Stop,
            ActionDescriptor::new(ACTION_STOP, "Stop"),
        ),
    ]);
    let transport_request = transport
        .request_for_visible(
            0,
            ActionSource::Button,
            Some(
                TimelineTransportContext::new(timeline)
                    .with_playhead_time(TimelineTime::from_seconds(1.25))
                    .with_selection_range(TimelineRange::seconds(0.5, 2.5)),
            ),
        )
        .expect("editor transport fixture emits metadata");

    EditorTimelineFixture {
        descriptor,
        candidates,
        transport_request,
        state,
        semantic_roles,
    }
}

fn editor_viewport_tool_fixture() -> EditorViewportToolFixture {
    let viewport = WidgetId::from_key("editor.viewport.fixture");
    let selected = ViewportSelectionTargetId::from_raw(70);
    let overlay = ViewportOverlayId::from_raw(12);
    let select_tool = ViewportToolId::from_raw(1);
    let pan_tool = ViewportToolId::from_raw(2);
    let mut select_action = ActionDescriptor::new(super::ACTION_TOOL_SELECT, "Select");
    select_action.state.checked = Some(true);
    let mut pan_action = ActionDescriptor::new(ACTION_VIEWPORT_PAN, "Pan");
    pan_action.state.checked = Some(false);
    let mut grid_action = ActionDescriptor::new(ACTION_GRID, "Show Grid");
    grid_action.state.checked = Some(true);
    let actions = vec![
        ViewportActionDescriptor::new(
            select_action,
            ViewportActionKind::ActivateTool,
            ViewportActionTarget::new(viewport).with_tool(select_tool),
        ),
        ViewportActionDescriptor::new(
            ActionDescriptor::new(ACTION_VIEWPORT_FOCUS_SELECTED, "Focus Selected"),
            ViewportActionKind::FocusSelected,
            ViewportActionTarget::new(viewport).with_selection(selected),
        ),
        ViewportActionDescriptor::new(
            ActionDescriptor::new(ACTION_VIEWPORT_FIT_CONTENT, "Fit Content"),
            ViewportActionKind::FitContent,
            ViewportActionTarget::new(viewport),
        ),
        ViewportActionDescriptor::new(
            ActionDescriptor::new(ACTION_VIEWPORT_FIT_SELECTION, "Fit Selection"),
            ViewportActionKind::FitSelection,
            ViewportActionTarget::new(viewport).with_selection(selected),
        ),
        ViewportActionDescriptor::new(
            ActionDescriptor::new(ACTION_VIEWPORT_ACTUAL_SIZE, "Actual Size"),
            ViewportActionKind::ActualSize,
            ViewportActionTarget::new(viewport),
        ),
        ViewportActionDescriptor::new(
            ActionDescriptor::new(ACTION_VIEWPORT_ZOOM_IN, "Zoom In"),
            ViewportActionKind::ZoomIn,
            ViewportActionTarget::new(viewport),
        ),
        ViewportActionDescriptor::new(
            ActionDescriptor::new(ACTION_VIEWPORT_ZOOM_OUT, "Zoom Out"),
            ViewportActionKind::ZoomOut,
            ViewportActionTarget::new(viewport),
        ),
        ViewportActionDescriptor::new(
            pan_action,
            ViewportActionKind::PanMode,
            ViewportActionTarget::new(viewport).with_tool(pan_tool),
        ),
        ViewportActionDescriptor::new(
            grid_action,
            ViewportActionKind::ToggleOverlay,
            ViewportActionTarget::new(viewport).with_overlay(overlay),
        ),
    ];
    let requests = viewport_action_requests(
        &actions,
        ActionSource::Button,
        &ActionContext::Widget(viewport),
    );
    let semantic_roles = viewport_actions_semantics(
        viewport.child("actions"),
        Rect::new(0.0, 0.0, 280.0, 28.0),
        "Viewport tool actions",
        &actions,
        actions.iter().enumerate().map(|(index, action)| {
            (
                action.action.id.clone(),
                Rect::new(index as f32 * 28.0, 0.0, 24.0, 24.0),
            )
        }),
    )
    .into_iter()
    .map(|node| node.role)
    .collect::<Vec<_>>();
    let mut pan_zoom = PanZoom::default();
    pan_zoom.set_zoom(1.0);
    let surface = ViewportSurface {
        texture: super::VIEWPORT_TEXTURE,
        source_size: VIEWPORT_SIZE,
        bounds: Rect::new(0.0, 0.0, 320.0, 180.0),
        pan_zoom,
    };
    let overlay_hit = hit_test_viewport_overlays(
        surface,
        &[ViewportOverlayDescriptor::new(
            overlay,
            ViewportOverlayKind::ToolRegion,
            Rect::new(12.0, 12.0, 80.0, 40.0),
            ViewportOverlaySpace::Screen,
        )
        .with_tool(pan_tool)
        .with_cursor(ViewportCursorMetadata::new(ViewportCursorShape::Crosshair))],
        Point::new(24.0, 20.0),
    )
    .expect("editor viewport fixture overlay hit");
    let tool = ViewportToolDescriptor::new(pan_tool, "Pan")
        .active(true)
        .with_cursor(ViewportCursorMetadata::new(ViewportCursorShape::Grab));
    let cursor_request =
        viewport_cursor_request(viewport, None, None, Some(&overlay_hit), Some(&tool))
            .expect("editor viewport fixture cursor request");

    EditorViewportToolFixture {
        actions,
        requests,
        cursor_request,
        semantic_roles,
    }
}

include!("tests/chrome_fixtures.rs");
include!("tests/workspace_node_graph.rs");
include!("tests/interactions.rs");
include!("tests/viewport_icons.rs");
include!("tests/toolbar_helpers.rs");
