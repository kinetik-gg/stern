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

#[test]
fn inspector_label_width_preserves_value_space_at_narrow_widths() {
    assert_eq!(inspector_label_width(120.0), 52.0);
    assert!((inspector_label_width(180.0) - 75.6).abs() < f32::EPSILON);
    assert_eq!(inspector_label_width(400.0), 96.0);
    assert_eq!(inspector_label_width(f32::NAN), 72.0);
}

#[test]
fn editor_chrome_metrics_follow_theme_controls() {
    let theme = default_dark_theme();
    let chrome = EditorChromeMetrics::from_theme(&theme);

    assert_eq!(
        chrome.toolbar_button,
        theme.controls.compact_control_height + theme.controls.padding_y
    );
    assert_eq!(
        chrome.toolbar_stride,
        chrome.toolbar_button + theme.controls.padding_x * 0.5
    );
    assert_eq!(chrome.toolbar_icon, theme.controls.icon_size);
    assert_eq!(chrome.asset_icon, theme.controls.icon_size);
    assert_eq!(chrome.toolbar_button, 26.0);
    assert_eq!(chrome.toolbar_stride, 30.0);
    assert_eq!(chrome.toolbar_icon, 16.0);
    assert_eq!(super::workspace_top(&theme), 68.0);
}

#[test]
fn editor_chrome_menu_bar_converts_active_menu_to_overlay_contract() {
    let mut editor = EditorShowcase::new();
    editor.open_menu = Some(EditorMenuKind::File);
    let viewport = Rect::new(0.0, 0.0, 1440.0, 900.0);
    let menu_bar = editor.menu_bar_model();

    assert_eq!(menu_bar.menus().len(), 7);
    assert_eq!(
        menu_bar.active_id(),
        Some(EditorMenuKind::File.menu_bar_id())
    );
    assert_eq!(
        menu_bar.active_menu().expect("active file menu").title,
        "File"
    );

    let overlay = editor.menu_overlay_model(EditorMenuKind::File, viewport);

    assert_eq!(overlay.entry.kind, OverlayKind::Menu);
    assert_eq!(
        overlay.entry.dismissal,
        OverlayDismissal::OutsideClickOrEscape
    );
    assert_eq!(overlay.source, ActionSource::Menu);
    assert_eq!(overlay.context, ActionContext::Editor);
    assert!(overlay.entry.rect.y > super::menu_anchor(EditorMenuKind::File).max_y());
    assert!(overlay.visible_items().iter().any(|item| matches!(
        item,
        MenuItem::Action(action)
            if action.id.as_str() == ACTION_SAVE
                && action.label == "Save Scene"
                && action.can_invoke()
    )));
    assert!(overlay.visible_items().iter().any(|item| matches!(
        item,
        MenuItem::Action(action) if action.label == "Quit" && !action.can_invoke()
    )));
}

#[test]
fn editor_chrome_toolbar_contract_tracks_checked_action_state() {
    let mut editor = EditorShowcase::new();
    let toolbar = editor.toolbar_model();
    let tools = toolbar
        .group(EditorToolbarGroupKind::Tools.id())
        .expect("tools group")
        .visible_items();

    assert_eq!(
        tools.iter().map(|item| item.label()).collect::<Vec<_>>(),
        ["Select", "Move", "Rotate", "Scale"]
    );
    assert_eq!(tools[1].action_id().as_str(), super::ACTION_TOOL_MOVE);
    assert_eq!(tools[1].checked(), Some(true));
    assert_eq!(tools[0].checked(), Some(false));
    assert_eq!(
        tools[1].icon().map(kinetik_ui::core::ActionIcon::as_str),
        Some("move")
    );

    let viewport_tools = toolbar
        .group(EditorToolbarGroupKind::Viewport.id())
        .expect("viewport group")
        .visible_items();
    assert_eq!(viewport_tools[0].action_id().as_str(), ACTION_GRID);
    assert_eq!(viewport_tools[0].checked(), Some(true));

    assert!(editor.apply_action(ACTION_PLAY));
    let toolbar = editor.toolbar_model();
    let run_items = toolbar
        .group(EditorToolbarGroupKind::Run.id())
        .expect("run group")
        .visible_items();
    assert_eq!(run_items[0].label(), "Play");
    assert_eq!(run_items[0].checked(), Some(true));
    assert_eq!(run_items[1].label(), "Pause");
    assert_eq!(run_items[1].checked(), Some(false));

    let invocation = toolbar
        .invocation_for_group_visible(EditorToolbarGroupKind::Run.id(), 0, ActionContext::Editor)
        .expect("run invocation");
    assert_eq!(invocation.action_id, ActionId::new(ACTION_PLAY));
    assert_eq!(invocation.source, ActionSource::Button);
    assert_eq!(invocation.context, ActionContext::Editor);
}

#[test]
fn editor_chrome_status_bar_contract_preserves_order_counts_and_progress() {
    let mut editor = EditorShowcase::new();
    editor.status = "Busy".to_owned();
    editor.running = true;
    editor.timeline = 1.5;

    let status_bar = editor.status_bar_model(12);
    let visible = status_bar.visible_items();

    assert_eq!(
        visible
            .iter()
            .map(|item| item.text.as_str())
            .collect::<Vec<_>>(),
        [
            "Busy",
            "Actions: 12",
            "Snap 1m",
            "Vello / winit",
            "Jobs: 2 active / 4 total",
            "Diagnostics: 1E 1W 1I",
            "Feedback: 2"
        ]
    );
    let actions = status_bar
        .item(EditorStatusItemKind::Actions.id())
        .expect("action count status");
    assert_eq!(actions.kind, StatusItemKind::ActionCount);
    assert_eq!(actions.count, Some(12));

    let jobs = status_bar
        .item(EditorStatusItemKind::Jobs.id())
        .expect("job status");
    assert_eq!(jobs.kind, StatusItemKind::JobCount);
    assert_eq!(jobs.count, Some(2));
    assert!((jobs.progress.expect("job progress").value - 0.4).abs() < f32::EPSILON);
    assert!(jobs.visible);

    let diagnostics = status_bar
        .item(EditorStatusItemKind::Diagnostics.id())
        .expect("diagnostics status");
    assert_eq!(diagnostics.kind, StatusItemKind::Error);
    assert_eq!(diagnostics.count, Some(3));

    let feedback = status_bar
        .item(EditorStatusItemKind::Feedback.id())
        .expect("feedback status");
    assert_eq!(feedback.kind, StatusItemKind::Message);
    assert_eq!(feedback.count, Some(2));

    let progress = status_bar
        .item(EditorStatusItemKind::Timeline.id())
        .expect("timeline progress status");
    assert_eq!(progress.kind, StatusItemKind::Progress);
    assert_eq!(progress.progress.expect("progress metadata").value, 1.0);
    assert!(!progress.visible);
}

#[test]
fn editor_showcase_job_fixture_is_deterministic_and_app_owned() {
    let jobs = EditorShowcase::showcase_job_list();
    let summary = jobs.summary();
    let progress = jobs.active_progress().expect("active fixture jobs");

    assert_eq!(jobs.rows().len(), 4);
    assert_eq!(
        jobs.rows()
            .iter()
            .map(|row| row.label.as_str())
            .collect::<Vec<_>>(),
        [
            "Active showcase job",
            "Queued showcase job",
            "Completed showcase job",
            "Failed showcase job"
        ]
    );
    assert_eq!(summary.running, 1);
    assert_eq!(summary.queued, 1);
    assert_eq!(summary.succeeded, 1);
    assert_eq!(summary.failed, 1);
    assert_eq!(summary.active(), 2);
    assert_eq!(progress.active, 2);
    assert_eq!(progress.determinate, 2);
    assert_eq!(progress.indeterminate, 0);
    assert!(
        (progress.status_progress().expect("status progress").value - 0.4).abs() < f32::EPSILON
    );
    assert_eq!(jobs.rows()[0].phase, JobPhase::Running);
    assert!(jobs.rows()[0].can_cancel());
    assert_eq!(
        jobs.cancel_request(super::job_row_id(1))
            .expect("cancel request")
            .invocation
            .action_id,
        ActionId::new(super::ACTION_CANCEL_ACTIVE_FIXTURE_JOB)
    );
}

#[test]
fn editor_showcase_diagnostics_fixture_summarizes_ordered_app_metadata() {
    let diagnostics = EditorShowcase::showcase_diagnostics();
    let summary = diagnostics.summary();
    let ordered = diagnostics.ordered_items();

    assert_eq!(summary.errors, 1);
    assert_eq!(summary.warnings, 1);
    assert_eq!(summary.info, 1);
    assert_eq!(summary.total(), 3);
    assert_eq!(
        ordered
            .iter()
            .map(|item| item.code.as_str())
            .collect::<Vec<_>>(),
        [
            "showcase.fixture.error",
            "showcase.fixture.warning",
            "showcase.fixture.info"
        ]
    );
    assert!(
        diagnostics.items().iter().all(|item| {
            item.source == Some(kinetik_ui::widgets::DiagnosticSource::Application)
        })
    );
}

#[test]
fn editor_showcase_feedback_fixture_preserves_lifetime_action_and_dismiss_metadata() {
    let feedback = EditorShowcase::showcase_feedback_stack();
    let active = feedback.active_items(super::showcase_feedback_now());

    assert_eq!(feedback.items().len(), 3);
    assert_eq!(active.len(), 2);
    assert_eq!(
        active.iter().map(|item| item.kind).collect::<Vec<_>>(),
        [FeedbackKind::Success, FeedbackKind::Warning]
    );
    assert_eq!(
        feedback
            .item(super::feedback_id(1))
            .expect("timed feedback")
            .remaining_lifetime(super::showcase_feedback_now()),
        Some(Duration::from_secs(4))
    );
    assert_eq!(
        feedback
            .item(super::feedback_id(3))
            .expect("expired feedback")
            .remaining_lifetime(super::showcase_feedback_now()),
        None
    );
    assert_eq!(
        feedback
            .action_request(super::feedback_id(2), super::showcase_feedback_now())
            .expect("feedback action")
            .invocation
            .action_id,
        ActionId::new(super::ACTION_OPEN_FEEDBACK_REPORT)
    );
    assert_eq!(
        feedback
            .dismiss_request(super::feedback_id(2), super::showcase_feedback_now())
            .expect("feedback dismiss")
            .invocation
            .action_id,
        ActionId::new(super::ACTION_DISMISS_FEEDBACK_REPORT)
    );
}

#[test]
fn editor_showcase_frame_emits_no_core_warnings() {
    let theme = default_dark_theme();
    let mut memory = UiMemory::new();
    let context = editor_test_context(UiInput::default());
    let mut ui = Ui::begin_frame(context, &mut memory, &theme);
    let mut editor = EditorShowcase::new();

    editor.render(&mut ui, 0);
    let output = ui.finish_output();

    assert!(
        output.diagnostics().is_empty(),
        "{:?}",
        output.diagnostics()
    );
}

#[test]
fn editor_chrome_tab_strip_contract_preserves_frame_tab_targets() {
    let editor = EditorShowcase::new();
    let bottom = editor.dock.frame(FRAME_BOTTOM).expect("bottom frame");
    let strip = frame_tab_strip(bottom);
    let rects = frame_tab_rects(bottom, bottom_frame_rect(&editor), 26.0);

    assert_eq!(strip.len(), 3);
    assert_eq!(rects.len(), strip.len());
    assert_eq!(strip.tabs()[0].title, "Console");
    assert_eq!(strip.tabs()[1].title, "Timeline");
    assert_eq!(strip.active_panel(), Some(strip.tabs()[0].panel));
    assert_eq!(
        strip
            .activation_target_by_index(1)
            .expect("timeline target")
            .panel,
        PANEL_TIMELINE
    );
    assert_eq!(
        strip
            .drag_target_by_panel(PANEL_TIMELINE)
            .expect("timeline drag target")
            .index,
        1
    );
}

#[test]
fn editor_timeline_fixture_exposes_data_only_semantics_snap_and_transport_requests() {
    let fixture = editor_timeline_fixture();

    fixture
        .descriptor
        .validate()
        .expect("editor-owned timeline descriptors validate");
    assert!(
        fixture
            .semantic_roles
            .iter()
            .any(|role| *role == SemanticRole::Custom("timeline".to_owned()))
    );
    assert!(
        fixture
            .semantic_roles
            .iter()
            .any(|role| *role == SemanticRole::Custom("timeline-item".to_owned()))
    );
    assert!(
        fixture
            .candidates
            .iter()
            .any(|candidate| candidate.source == TimelineSnapSource::Frame)
    );
    assert!(
        fixture
            .candidates
            .iter()
            .any(|candidate| candidate.source == TimelineSnapSource::Marker)
    );
    assert!(
        fixture
            .candidates
            .iter()
            .any(|candidate| candidate.source == TimelineSnapSource::Keyframe)
    );
    assert_eq!(
        fixture.transport_request.action_id,
        ActionId::new(ACTION_PLAY)
    );
    assert_eq!(fixture.transport_request.source, ActionSource::Button);
    assert_eq!(
        fixture
            .transport_request
            .timeline_context
            .expect("transport context")
            .timeline,
        TimelineId::from_raw(9_000)
    );
    assert!(
        fixture
            .state
            .selection
            .contains(TimelineSelectionTarget::Item(TimelineItemId::from_raw(11)))
    );
}

#[test]
fn editor_viewport_tool_fixture_exercises_app_owned_action_routing() {
    let fixture = editor_viewport_tool_fixture();

    assert_eq!(fixture.actions.len(), 9);
    assert_eq!(
        fixture
            .requests
            .iter()
            .map(|request| request.kind)
            .collect::<Vec<_>>(),
        vec![
            ViewportActionKind::ActivateTool,
            ViewportActionKind::FocusSelected,
            ViewportActionKind::FitContent,
            ViewportActionKind::FitSelection,
            ViewportActionKind::ActualSize,
            ViewportActionKind::ZoomIn,
            ViewportActionKind::ZoomOut,
            ViewportActionKind::PanMode,
            ViewportActionKind::ToggleOverlay,
        ]
    );
    assert!(fixture.requests.iter().all(|request| {
        request.source == ActionSource::Button
            && matches!(request.context, ActionContext::Widget(_))
    }));
    assert_eq!(
        fixture.requests[0].action_id,
        ActionId::new(super::ACTION_TOOL_SELECT)
    );
    assert_eq!(fixture.requests[0].checked, Some(true));
    assert_eq!(
        fixture.requests[1].target.selection,
        Some(ViewportSelectionTargetId::from_raw(70))
    );
    assert_eq!(
        fixture.requests[8].target.overlay,
        Some(ViewportOverlayId::from_raw(12))
    );
    assert_eq!(fixture.requests[8].checked, Some(true));
    assert!(
        fixture
            .semantic_roles
            .iter()
            .any(|role| *role == SemanticRole::Custom("viewport-actions".to_owned()))
    );
    assert!(fixture.semantic_roles.contains(&SemanticRole::Toggle));
    assert!(fixture.semantic_roles.contains(&SemanticRole::Button));
    assert_eq!(
        fixture.cursor_request.source,
        ViewportCursorRequestSource::HoveredOverlay
    );
    assert_eq!(
        fixture.cursor_request.cursor.shape,
        ViewportCursorShape::Crosshair
    );
    assert_eq!(
        fixture.cursor_request.overlay,
        Some(ViewportOverlayId::from_raw(12))
    );
}

#[test]
fn editor_chrome_modal_contract_exposes_data_only_action_metadata() {
    let editor = EditorShowcase::new();
    let viewport = Rect::new(0.0, 0.0, 1440.0, 900.0);
    let before = editor.status.clone();
    let overlay = editor.about_modal_overlay_model(viewport);

    assert_eq!(overlay.entry.kind, OverlayKind::Modal);
    assert!(overlay.entry.modal);
    assert_eq!(
        overlay.entry.dismissal,
        OverlayDismissal::OutsideClickOrEscape
    );
    assert_eq!(overlay.context, ActionContext::Editor);
    assert_eq!(overlay.dialog.title, "About Kinetik Forge");
    assert_eq!(overlay.visible_actions().len(), 2);
    assert_eq!(
        overlay
            .visible_action_by_role(ModalActionRole::Cancel)
            .expect("cancel action")
            .action
            .label,
        "Close"
    );

    let invocation = overlay
        .invocation_for_role(ModalActionRole::Primary)
        .expect("primary modal action invocation");
    assert_eq!(invocation.action_id, ActionId::new(super::ACTION_PALETTE));
    assert_eq!(invocation.source, ActionSource::Button);
    assert_eq!(invocation.context, ActionContext::Editor);
    assert_eq!(editor.status, before);
}

#[test]
fn default_workspace_snapshot_validates_against_showcase_panel_descriptors() {
    let registry = super::editor_panel_registry();
    let snapshot = super::default_workspace_snapshot();
    let diagnostics = snapshot.diagnostics(registry.descriptors());

    assert!(diagnostics.is_valid(), "{diagnostics:?}");
    assert!(diagnostics.dock.diagnostics.is_empty(), "{diagnostics:?}");
    assert!(diagnostics.workspace.is_empty(), "{diagnostics:?}");
    snapshot
        .validate(registry.descriptors())
        .expect("workspace validates");
    assert_eq!(
        snapshot.panel_instances,
        super::editor_panel_instances(),
        "default workspace instances should be deterministic"
    );
}

#[test]
fn default_workspace_snapshot_round_trips_through_workspace_restore() {
    let registry = super::editor_panel_registry();
    let snapshot = super::default_workspace_snapshot();
    let restored =
        super::Dock::restore_workspace(snapshot.clone(), registry.descriptors()).expect("restore");

    assert_eq!(restored.snapshot(), snapshot.dock);
    assert_eq!(
        restored.workspace_snapshot(super::editor_panel_instances()),
        snapshot
    );
}

#[test]
fn editor_panel_registry_builds_unique_showcase_descriptors() {
    let registry = super::editor_panel_registry();

    assert_eq!(registry.descriptors().len(), 7);
    assert_eq!(
        registry.descriptors(),
        super::editor_panel_type_descriptors().as_slice()
    );
    assert_eq!(
        registry
            .descriptor(super::PANEL_TYPE_NODE_GRAPH)
            .expect("node graph descriptor")
            .title,
        "Node Graph"
    );
}

#[test]
fn registry_open_metadata_exposes_editor_vocabulary_in_stable_order() {
    let registry = super::editor_panel_registry();
    let metadata = super::editor_open_panel_metadata();
    let titles = metadata
        .iter()
        .map(|metadata| metadata.title.as_str())
        .collect::<Vec<_>>();
    let action_ids = metadata
        .iter()
        .map(|metadata| {
            metadata
                .default_open_action
                .as_ref()
                .expect("open action")
                .as_str()
        })
        .collect::<Vec<_>>();
    let categories = registry
        .categories()
        .into_iter()
        .map(super::panel_category_label)
        .collect::<Vec<_>>();

    assert_eq!(
        titles,
        [
            "Viewport",
            "Explorer",
            "Properties",
            "Asset Browser",
            "Timeline",
            "Console",
            "Node Graph",
        ]
    );
    assert_eq!(
        action_ids,
        [
            super::ACTION_OPEN_VIEWPORT,
            super::ACTION_OPEN_EXPLORER,
            super::ACTION_OPEN_PROPERTIES,
            super::ACTION_OPEN_ASSET_BROWSER,
            super::ACTION_OPEN_TIMELINE,
            super::ACTION_OPEN_CONSOLE,
            super::ACTION_OPEN_NODE_GRAPH,
        ]
    );
    assert_eq!(
        metadata
            .iter()
            .map(|metadata| metadata.category.clone())
            .collect::<Vec<_>>(),
        [
            PanelTypeCategory::Viewport,
            PanelTypeCategory::Hierarchy,
            PanelTypeCategory::Inspector,
            PanelTypeCategory::Assets,
            PanelTypeCategory::Timeline,
            PanelTypeCategory::Diagnostics,
            PanelTypeCategory::Timeline,
        ]
    );
    assert_eq!(
        categories,
        [
            "Viewport",
            "Hierarchy",
            "Inspector",
            "Assets",
            "Timeline",
            "Diagnostics",
        ]
    );
}

#[test]
fn default_workspace_snapshot_contains_roblox_blender_style_vocabulary() {
    let snapshot = super::default_workspace_snapshot();
    let titles = snapshot
        .panel_instances
        .iter()
        .map(|instance| instance.title.as_str())
        .collect::<Vec<_>>();
    let state_keys = snapshot
        .panel_instances
        .iter()
        .map(|instance| instance.state_key.as_deref().expect("state key"))
        .collect::<Vec<_>>();

    assert_eq!(
        titles,
        [
            "Explorer",
            "Asset Browser",
            "Viewport",
            "Console",
            "Timeline",
            "Properties",
            "Node Graph",
        ]
    );
    assert_eq!(
        state_keys,
        [
            "editor.explorer",
            "editor.asset-browser",
            "editor.viewport",
            "editor.console",
            "editor.timeline",
            "editor.properties",
            "editor.node-graph",
        ]
    );
}

#[test]
fn registry_open_or_focus_workflow_is_app_owned_and_deterministic() {
    let mut editor = EditorShowcase::new();
    let registry = super::editor_panel_registry();
    let instances = super::editor_panel_instances();
    let decision = registry
        .resolve_open_decision(
            super::PANEL_TYPE_NODE_GRAPH,
            &instances,
            &editor.dock,
            super::PanelWorkspaceContext::Docked,
        )
        .expect("open decision");

    assert!(matches!(decision, PanelOpenDecision::FocusExisting(_)));
    assert!(editor.open_or_focus_panel(super::PANEL_TYPE_NODE_GRAPH));
    assert_eq!(editor.status, "Focused Node Graph");
    assert_eq!(editor.dock.active_frame(), Some(FRAME_BOTTOM));
    assert_eq!(
        editor
            .dock
            .frame(FRAME_BOTTOM)
            .and_then(|frame| frame.active_panel())
            .map(|panel| panel.id),
        Some(super::PANEL_NODE_GRAPH)
    );

    assert!(editor.apply_action(super::ACTION_OPEN_PROPERTIES));
    assert_eq!(editor.status, "Focused Properties");
    assert_eq!(editor.dock.active_frame(), Some(FRAME_INSPECTOR));
}

#[test]
fn editor_node_graph_panel_exercises_stage9_contracts() {
    let mut editor = EditorShowcase::new();
    assert!(editor.open_or_focus_panel(super::PANEL_TYPE_NODE_GRAPH));

    let theme = default_dark_theme();
    let mut memory = UiMemory::new();
    let mut ui = Ui::begin_frame(editor_test_context(UiInput::default()), &mut memory, &theme);
    editor.render(&mut ui, 0);
    let frame = ui.finish_output();

    assert!(frame.semantics.nodes().iter().any(|node| {
        node.role == SemanticRole::Custom("node-graph".to_owned())
            && node.label.as_deref() == Some("Node graph")
    }));

    let body = Rect::new(20.0, 40.0, 480.0, 180.0);
    let viewport = super::EditorShowcase::showcase_node_graph_viewport(body);
    let graph = super::EditorShowcase::showcase_node_graph_descriptor();
    graph.validate().expect("showcase graph validates");

    let output = super::EditorShowcase::showcase_node_graph_output(
        WidgetId::from_key("showcase-node-graph"),
        viewport,
    )
    .expect("showcase graph emits static output");
    assert!(matches!(
        output.primitives.first(),
        Some(Primitive::ClipBegin { .. })
    ));
    assert!(matches!(
        output.primitives.last(),
        Some(Primitive::ClipEnd { .. })
    ));
    assert!(output.semantics.iter().any(|node| {
        node.role == SemanticRole::Custom("node".to_owned())
            && node.label.as_deref() == Some("Color Grade")
            && node.state.selected
    }));
    assert!(output.semantics.iter().any(|node| {
        node.role == SemanticRole::Custom("edge".to_owned())
            && node.label.as_deref() == Some("Edge 51: Color Grade Out to Output Surface")
            && node.state.selected
    }));
    assert!(output.semantics.iter().any(|node| {
        node.role == SemanticRole::Custom("port".to_owned())
            && node.label.as_deref() == Some("Input Mask")
            && node.description.as_deref() == Some("Incompatible port")
    }));

    let color_grade_center = viewport.graph_rect_to_screen(graph.nodes[1].rect).center();
    assert_eq!(
        graph
            .hit_test(viewport, color_grade_center)
            .expect("node hit target"),
        NodeGraphHitTarget::NodeBody(NodeId::from_raw(2))
    );

    let selection =
        NodeGraphSelection::new().replace(NodeGraphSelectionTarget::Node(NodeId::from_raw(2)));
    let context_actions = graph.context_actions(
        NodeGraphContextTarget::Node(NodeId::from_raw(2)),
        &selection,
    );
    assert!(
        context_actions
            .iter()
            .any(|action| { action.kind == NodeGraphContextActionKind::Delete && action.enabled })
    );
    assert!(context_actions.iter().any(|action| {
        action.kind == NodeGraphContextActionKind::FrameSelection && action.enabled
    }));

    let link_request = graph
        .create_link_request(
            PortEndpoint::new(NodeId::from_raw(1), PortId::from_raw(1)),
            PortEndpoint::new(NodeId::from_raw(3), PortId::from_raw(1)),
        )
        .expect("link request metadata");
    assert!(matches!(
        link_request,
        NodeGraphLinkEditRequest::CreateLink(_)
    ));

    let move_request = graph
        .move_frame_request(
            viewport,
            NodeFrameId::from_raw(1),
            GraphVector::new(20.0, -10.0),
        )
        .expect("frame move metadata");
    assert_eq!(move_request.children.len(), 2);
    assert_eq!(move_request.graph_delta, GraphVector::new(20.0, -10.0));
}

#[test]
fn inspector_snap_toggle_updates_status_same_frame() {
    let mut editor = EditorShowcase::new();
    let mut memory = UiMemory::new();
    let theme = default_dark_theme();

    let mut ui = Ui::begin_frame(
        editor_test_context(pointer_input_at(1290.0, 362.0, true, true, false)),
        &mut memory,
        &theme,
    );
    editor.render(&mut ui, 0);
    let _ = ui.finish_output();

    let mut ui = Ui::begin_frame(
        editor_test_context(pointer_input_at(1290.0, 362.0, false, false, true)),
        &mut memory,
        &theme,
    );
    editor.render(&mut ui, 0);
    let output = ui.finish_output();

    assert!(!editor.snap_enabled);
    assert!(output.primitives.iter().any(|primitive| {
        matches!(primitive, Primitive::Text(text) if text.text == "Snap off")
    }));
}

#[test]
fn toolbar_tool_selection_updates_status_same_frame() {
    let mut editor = EditorShowcase::new();
    let mut memory = UiMemory::new();
    let theme = default_dark_theme();

    let mut ui = Ui::begin_frame(
        editor_test_context(pointer_input_at(14.0, 40.0, true, true, false)),
        &mut memory,
        &theme,
    );
    editor.render(&mut ui, 0);
    let _ = ui.finish_output();
    let visible_tool_id =
        WidgetId::from_key("root").child(("editor.tool", super::ACTION_TOOL_SELECT));

    assert_eq!(memory.pressed(), Some(visible_tool_id));

    let mut ui = Ui::begin_frame(
        editor_test_context(pointer_input_at(14.0, 40.0, false, false, true)),
        &mut memory,
        &theme,
    );
    editor.render(&mut ui, 0);
    let output = ui.finish_output();

    assert_eq!(editor.selected_tool, EditorTool::Select);
    assert!(output.primitives.iter().any(|primitive| {
        matches!(primitive, Primitive::Text(text) if text.text == "Select tool active")
    }));
}

#[test]
fn toolbar_run_click_invokes_through_visible_identity() {
    let mut editor = EditorShowcase::new();
    let mut memory = UiMemory::new();
    let theme = default_dark_theme();
    let chrome = EditorChromeMetrics::from_theme(&theme);
    let viewport = Rect::new(0.0, 0.0, 1440.0, 900.0);
    let (index, _icon, _label, action, rect) = super::run_toolbar_buttons(viewport, chrome)[0];
    let point = rect.center();

    let mut ui = Ui::begin_frame(
        editor_test_context(pointer_input_at(point.x, point.y, true, true, false)),
        &mut memory,
        &theme,
    );
    let invocations = editor.render(&mut ui, 0);
    let _ = ui.finish_output();
    let visible_run_id = WidgetId::from_key("root").child(("editor.run", action, index));

    assert!(invocations.is_empty());
    assert_eq!(memory.pressed(), Some(visible_run_id));

    let mut ui = Ui::begin_frame(
        editor_test_context(pointer_input_at(point.x, point.y, false, false, true)),
        &mut memory,
        &theme,
    );
    let invocations = editor.render(&mut ui, 0);
    let output = ui.finish_output();

    assert_eq!(invocations.len(), 1);
    assert_eq!(invocations[0].action_id, ActionId::new(ACTION_PLAY));
    assert!(editor.running);
    assert!(output.primitives.iter().any(|primitive| {
        matches!(primitive, Primitive::Text(text) if text.text == "Play mode running")
    }));
}

#[test]
fn toolbar_tool_click_has_single_same_frame_selection_visual() {
    let mut editor = EditorShowcase::new();
    let mut memory = UiMemory::new();
    let theme = default_dark_theme();
    let chrome = EditorChromeMetrics::from_theme(&theme);
    let rotate = Point::new(
        10.0 + 2.0 * chrome.toolbar_stride + chrome.toolbar_button * 0.5,
        TOOLBAR_Y + chrome.toolbar_button * 0.5,
    );

    let mut ui = Ui::begin_frame(
        editor_test_context(pointer_input_at(rotate.x, rotate.y, true, true, false)),
        &mut memory,
        &theme,
    );
    editor.render(&mut ui, 0);
    let _ = ui.finish_output();

    let mut ui = Ui::begin_frame(
        editor_test_context(pointer_input_at(rotate.x, rotate.y, false, false, true)),
        &mut memory,
        &theme,
    );
    editor.render(&mut ui, 0);
    let output = ui.finish_output();
    let selected_fill = rgb(39, 69, 122);
    let selected_toolbar_buttons = output
        .primitives
        .iter()
        .filter(|primitive| match primitive {
            Primitive::Rect(rect) => {
                rect.rect.y == TOOLBAR_Y
                    && rect.rect.width == chrome.toolbar_button
                    && rect.rect.height == chrome.toolbar_button
                    && matches!(&rect.fill, Some(Brush::Solid(color)) if *color == selected_fill)
            }
            _ => false,
        })
        .count();

    assert_eq!(editor.selected_tool, EditorTool::Rotate);
    assert_eq!(selected_toolbar_buttons, 1);
}

#[test]
fn frame_tab_click_updates_body_same_frame() {
    let mut editor = EditorShowcase::new();
    let mut memory = UiMemory::new();
    let theme = default_dark_theme();
    let bottom = bottom_frame_rect(&editor);
    let timeline = editor
        .dock
        .frame(FRAME_BOTTOM)
        .and_then(|frame| {
            frame_tab_rects(frame, bottom, 26.0)
                .into_iter()
                .find(|(tab, _rect)| tab.panel == PANEL_TIMELINE)
                .map(|(_tab, rect)| rect)
        })
        .expect("timeline tab");
    let point = Point::new(
        timeline.x + timeline.width * 0.5,
        timeline.y + timeline.height * 0.5,
    );

    let mut ui = Ui::begin_frame(
        editor_test_context(pointer_input_at(point.x, point.y, true, true, false)),
        &mut memory,
        &theme,
    );
    editor.render(&mut ui, 0);
    let _ = ui.finish_output();

    let mut ui = Ui::begin_frame(
        editor_test_context(pointer_input_at(point.x, point.y, false, false, true)),
        &mut memory,
        &theme,
    );
    editor.render(&mut ui, 0);
    let output = ui.finish_output();

    assert_eq!(editor.dock.active_frame(), Some(FRAME_BOTTOM));
    assert_eq!(focused_frame_semantic_count(&output), 1);
    assert!(output.semantics.nodes().iter().any(|node| {
        node.role == SemanticRole::Frame
            && node.label.as_deref() == Some("Frame 4")
            && node.state.focused
    }));
    assert!(output.primitives.iter().any(|primitive| {
        matches!(primitive, Primitive::Text(text) if text.text == "Intro camera pan")
    }));
    assert!(
        !output.primitives.iter().any(|primitive| {
            matches!(primitive, Primitive::Text(text) if text.text == "Message")
        })
    );
}

#[test]
fn splitter_drag_routes_through_dock_resize_path() {
    let mut editor = EditorShowcase::new();
    let mut memory = UiMemory::new();
    let theme = default_dark_theme();
    let bounds = editor_workspace_bounds();
    let splitter =
        solve_dock_splitters_with_style(&editor.dock, bounds, super::editor_dock_chrome_style())
            .into_iter()
            .next()
            .expect("root splitter");
    let before = splitter.ratio;
    let press = splitter.rect.center();

    let mut ui = Ui::begin_frame(
        editor_test_context(pointer_input_at(press.x, press.y, true, true, false)),
        &mut memory,
        &theme,
    );
    editor.render(&mut ui, 0);
    let _ = ui.finish_output();

    let mut ui = Ui::begin_frame(
        editor_test_context(pointer_input_at_with_delta(
            press.x + 48.0,
            press.y,
            true,
            false,
            false,
            Vec2::new(48.0, 0.0),
        )),
        &mut memory,
        &theme,
    );
    editor.render(&mut ui, 0);
    let output = ui.finish_output();
    let after =
        solve_dock_splitters_with_style(&editor.dock, bounds, super::editor_dock_chrome_style())
            .into_iter()
            .next()
            .expect("root splitter")
            .ratio;

    assert!(after > before, "{after} should be greater than {before}");
    assert_eq!(output.repaint, RepaintRequest::NextFrame);
}

#[test]
fn editor_splitter_join_action_uses_context_metadata_and_apply_request() {
    let mut editor = EditorShowcase::new();
    let bounds = editor_workspace_bounds();
    let layout = solve_dock_layout(&editor.dock, bounds);
    let splitter =
        solve_dock_splitters_with_style(&editor.dock, bounds, super::editor_dock_chrome_style())
            .into_iter()
            .next()
            .expect("root splitter");
    let action = resolve_dock_splitter_context_actions_with_policy(
        &editor.dock,
        &layout,
        &splitter,
        super::editor_dock_interaction_policy(),
    )
    .into_iter()
    .find(|action| action.kind == DockSplitterContextActionKind::Join && action.enabled)
    .expect("enabled join action");
    let request = action.join_request().expect("join request");
    let source = request.source_frame();
    let target = request.target_frame();

    assert!(editor.apply_splitter_context_action(bounds, DockSplitterContextActionKind::Join));

    assert!(editor.dock.frame(source).is_none());
    assert!(editor.dock.frame(target).is_some());
    assert_eq!(editor.dock.active_frame(), Some(target));
    assert_eq!(
        editor.status,
        format!(
            "Dock splitter joined frame {} into frame {}",
            source.raw(),
            target.raw()
        )
    );
}

#[test]
fn editor_splitter_swap_action_uses_context_metadata_and_apply_request() {
    let mut editor = EditorShowcase::new();
    let bounds = editor_workspace_bounds();
    let layout = solve_dock_layout(&editor.dock, bounds);
    let splitter =
        solve_dock_splitters_with_style(&editor.dock, bounds, super::editor_dock_chrome_style())
            .into_iter()
            .next()
            .expect("root splitter");
    let action = resolve_dock_splitter_context_actions_with_policy(
        &editor.dock,
        &layout,
        &splitter,
        super::editor_dock_interaction_policy(),
    )
    .into_iter()
    .find(|action| action.kind == DockSplitterContextActionKind::Swap && action.enabled)
    .expect("enabled swap action");
    let request = action.swap_request().expect("swap request");
    let source = request.source_frame();
    let target = request.target_frame();
    let source_before = editor_frame_rect(&editor, source);
    let target_before = editor_frame_rect(&editor, target);

    assert!(editor.apply_splitter_context_action(bounds, DockSplitterContextActionKind::Swap));

    assert_eq!(editor_frame_rect(&editor, source), target_before);
    assert_eq!(editor_frame_rect(&editor, target), source_before);
    assert_eq!(
        editor.status,
        format!(
            "Dock splitter swapped frame {} with frame {}",
            source.raw(),
            target.raw()
        )
    );
}

#[test]
fn tab_drag_drop_uses_dock_drag_and_target_without_panel_metadata_mutation() {
    let mut editor = EditorShowcase::new();
    let mut memory = UiMemory::new();
    let theme = default_dark_theme();
    let bottom = bottom_frame_rect(&editor);
    let inspector = editor_frame_rect(&editor, FRAME_INSPECTOR);
    let timeline = editor
        .dock
        .frame(FRAME_BOTTOM)
        .and_then(|frame| {
            frame_tab_rects(frame, bottom, 26.0)
                .into_iter()
                .find(|(tab, _rect)| tab.panel == PANEL_TIMELINE)
                .map(|(_tab, rect)| rect)
        })
        .expect("timeline tab");
    let start = timeline.center();
    let target = inspector.center();

    let mut ui = Ui::begin_frame(
        editor_test_context(pointer_input_at(start.x, start.y, true, true, false)),
        &mut memory,
        &theme,
    );
    editor.render(&mut ui, 0);
    let _ = ui.finish_output();

    let drag_delta = Vec2::new(target.x - start.x, target.y - start.y);
    let mut ui = Ui::begin_frame(
        editor_test_context(pointer_input_at_with_delta(
            target.x, target.y, true, false, false, drag_delta,
        )),
        &mut memory,
        &theme,
    );
    editor.render(&mut ui, 0);
    let dragging_output = ui.finish_output();

    assert_eq!(editor.status, "Dragging Timeline tab");
    assert!(dragging_output.primitives.iter().any(|primitive| {
            matches!(
                primitive,
                Primitive::Rect(rect)
                    if matches!(&rect.fill, Some(Brush::Solid(color)) if *color == rgba(78, 142, 245, 0.18))
                        && inspector.contains_point(rect.rect.center())
            )
        }));

    let mut ui = Ui::begin_frame(
        editor_test_context(pointer_input_at(target.x, target.y, false, false, true)),
        &mut memory,
        &theme,
    );
    editor.render(&mut ui, 0);
    let output = ui.finish_output();
    let inspector_frame = editor.dock.frame(FRAME_INSPECTOR).expect("inspector frame");
    let timeline_panel = inspector_frame
        .panels
        .iter()
        .find(|panel| panel.id == PANEL_TIMELINE)
        .expect("moved timeline panel");

    assert_eq!(timeline_panel.title, "Timeline");
    assert_eq!(
        inspector_frame.active_panel().map(|panel| panel.id),
        Some(PANEL_TIMELINE)
    );
    assert_eq!(editor.dock.active_frame(), Some(FRAME_INSPECTOR));
    assert!(
        !editor
            .dock
            .frame(FRAME_BOTTOM)
            .expect("bottom frame")
            .panels
            .iter()
            .any(|panel| panel.id == PANEL_TIMELINE)
    );
    assert!(editor.status.contains("Dock tab merged into frame"));
    assert_eq!(output.repaint, RepaintRequest::NextFrame);

    let registry = super::editor_panel_registry();
    let moved_workspace = editor
        .dock
        .workspace_snapshot(super::editor_panel_instances());
    moved_workspace
        .validate(registry.descriptors())
        .expect("moved workspace validates");
    let moved_timeline = moved_workspace
        .panel_instances
        .iter()
        .find(|instance| instance.id == PANEL_TIMELINE.instance_id())
        .expect("timeline instance metadata");
    assert_eq!(moved_timeline.panel_type, super::PANEL_TYPE_TIMELINE);
    assert_eq!(moved_timeline.title, "Timeline");
    assert_eq!(moved_timeline.state_key.as_deref(), Some("editor.timeline"));
    let restored = super::Dock::restore_workspace(moved_workspace.clone(), registry.descriptors())
        .expect("moved workspace restores");
    assert_eq!(restored.snapshot(), moved_workspace.dock);
}

#[test]
fn viewport_selection_overlay_uses_scaled_content_mapping() {
    let mut editor = EditorShowcase::new();
    let mut memory = UiMemory::new();
    let theme = default_dark_theme();
    let viewport_frame = editor_frame_rect(&editor, FRAME_VIEWPORT);
    let viewport_body = frame_body_rect(viewport_frame);
    let surface_bounds = Rect::new(
        viewport_body.x + 8.0,
        viewport_body.y + 36.0,
        (viewport_body.width - 16.0).max(1.0),
        (viewport_body.height - 66.0).max(1.0),
    );
    let surface = ViewportSurface {
        texture: super::VIEWPORT_TEXTURE,
        source_size: VIEWPORT_SIZE,
        bounds: surface_bounds,
        pan_zoom: editor.viewport_pan_zoom,
    };
    let scale = ScaleFactor::new(1.25);
    let expected = surface
        .content_rect_to_screen_at(Rect::new(720.0, 210.0, 210.0, 280.0), scale)
        .expect("selection rect");

    let mut ui = Ui::begin_frame(
        editor_test_context_scaled(UiInput::default(), scale),
        &mut memory,
        &theme,
    );
    editor.render(&mut ui, 0);
    let output = ui.finish_output();
    let selection_fill = rgba(78, 142, 245, 0.12);
    let selection = output
            .primitives
            .iter()
            .find_map(|primitive| match primitive {
                Primitive::Rect(rect)
                    if matches!(&rect.fill, Some(Brush::Solid(color)) if *color == selection_fill) =>
                {
                    Some(rect.rect)
                }
                _ => None,
            })
            .expect("selection overlay rect");

    assert_eq!(selection, expected);
    let physical_x = f64::from(selection.x) * scale.value();
    let physical_width = f64::from(selection.width) * scale.value();
    assert!((physical_x - physical_x.round()).abs() < 0.001);
    assert!((physical_width - physical_width.round()).abs() < 0.001);
}

#[test]
fn scene_expander_flips_arrow_and_requests_repaint_same_frame() {
    let mut editor = EditorShowcase::new();
    let mut memory = UiMemory::new();
    let theme = default_dark_theme();
    let expander = Point::new(38.0, super::workspace_top(&theme) + 100.0);

    let mut ui = Ui::begin_frame(
        editor_test_context(pointer_input_at(expander.x, expander.y, true, true, false)),
        &mut memory,
        &theme,
    );
    editor.render(&mut ui, 0);
    let _ = ui.finish_output();

    let mut ui = Ui::begin_frame(
        editor_test_context(pointer_input_at(expander.x, expander.y, false, false, true)),
        &mut memory,
        &theme,
    );
    editor.render(&mut ui, 0);
    let output = ui.finish_output();

    assert_eq!(output.repaint, RepaintRequest::NextFrame);
    assert!(!editor.scene_expansion.is_expanded(item_id(2)));
    assert!(
        output
            .primitives
            .iter()
            .any(|primitive| { matches!(primitive, Primitive::Text(text) if text.text == ">") })
    );
}

#[test]
fn outside_click_dismisses_menu_and_requests_repaint() {
    let mut editor = EditorShowcase::new();
    editor.open_menu = Some(EditorMenuKind::File);
    let mut memory = UiMemory::new();
    let theme = default_dark_theme();

    let mut ui = Ui::begin_frame(
        editor_test_context(pointer_input_at(900.0, 700.0, false, false, true)),
        &mut memory,
        &theme,
    );
    editor.render(&mut ui, 0);
    let output = ui.finish_output();

    assert_eq!(editor.open_menu, None);
    assert_eq!(output.repaint, RepaintRequest::NextFrame);
}

#[test]
fn icon_atlas_duplicates_edge_pixels_into_gutters() {
    let first = phosphor_icons::ICON_ENTRIES
        .iter()
        .find(|entry| entry.logical_size == phosphor_icons::STANDARD_ICON_LOGICAL_SIZE)
        .expect("standard icon entry");
    let atlas = icon_atlas_image(first.physical_size).expect("atlas");
    let source = first.source;
    let left_gutter = atlas_pixel(
        &atlas.data,
        atlas.width,
        source.x as u32 - phosphor_icons::ICON_ATLAS_PADDING,
        source.y as u32,
    );
    let first_inner = atlas_pixel(&atlas.data, atlas.width, source.x as u32, source.y as u32);
    let bottom_gutter = atlas_pixel(
        &atlas.data,
        atlas.width,
        source.max_x() as u32,
        source.max_y() as u32,
    );
    let bottom_inner = atlas_pixel(
        &atlas.data,
        atlas.width,
        source.max_x() as u32 - 1,
        source.max_y() as u32 - 1,
    );
    let atlas_entry = phosphor_icons::ICON_ATLASES
        .iter()
        .find(|atlas| atlas.image == first.atlas)
        .expect("atlas entry");

    assert_eq!(atlas.width, atlas_entry.width);
    assert_eq!(atlas.height, atlas_entry.height);
    assert_eq!(left_gutter, first_inner);
    assert_eq!(bottom_gutter, bottom_inner);
}

#[test]
fn icon_manifest_entries_register_as_atlas_regions() {
    let mut resources = RenderResources::new();

    register_resources(&mut resources);

    for entry in phosphor_icons::ICON_ENTRIES {
        let resource = resources.image(entry.image).expect(entry.symbol);
        let region = resource.atlas_region.expect("icon atlas region");

        assert_eq!(
            resource.size,
            Size::new(entry.logical_size as f32, entry.logical_size as f32)
        );
        assert_eq!(
            resource.sampling,
            kinetik_ui::render::RenderImageSampling::UiIcon
        );
        assert_eq!(region.atlas, entry.atlas);
        assert_eq!(region.source, entry.source, "{}", entry.source_name);
    }
}

#[test]
fn icon_atlas_regions_target_inner_unpadded_cells() {
    let mut resources = RenderResources::new();

    register_resources(&mut resources);

    let entry = phosphor_icons::ICON_ENTRIES
        .iter()
        .find(|entry| {
            entry.icon == phosphor_icons::PhosphorIcon::Crosshair
                && entry.logical_size == phosphor_icons::STANDARD_ICON_LOGICAL_SIZE
                && entry.physical_size == 24
        })
        .expect("crosshair entry");
    let region = resources
        .image(entry.image)
        .and_then(|resource| resource.atlas_region)
        .expect("icon region");

    assert_eq!(region.source.width, entry.physical_size as f32);
    assert_eq!(region.source.height, entry.physical_size as f32);
    assert_eq!(region.source, entry.source);
    assert_eq!(entry.source_name, "crosshair");
}

#[test]
fn editor_structural_smoke_emits_dock_frame_panel_viewport_and_action_categories() {
    let theme = default_dark_theme();
    let mut memory = UiMemory::new();
    let context = editor_test_context(UiInput::default());
    let mut ui = Ui::begin_frame(context, &mut memory, &theme);
    let mut editor = EditorShowcase::new();

    let invocations = editor.render(&mut ui, 0);
    let output = ui.finish_output();

    assert!(invocations.is_empty());
    assert_eq!(output.warnings, Vec::new());
    assert!(output.primitives.len() > 200);
    assert!(
        count_primitives(&output.primitives, |primitive| matches!(
            primitive,
            Primitive::Rect(_)
        )) > 100
    );
    assert!(
        count_primitives(&output.primitives, |primitive| matches!(
            primitive,
            Primitive::Text(_)
        )) > 50
    );
    assert!(
        count_primitives(&output.primitives, |primitive| matches!(
            primitive,
            Primitive::Image(_)
        )) >= 24
    );
    assert!(
        count_primitives(&output.primitives, |primitive| matches!(
            primitive,
            Primitive::Texture(_)
        )) >= 1
    );
    assert!(
        count_primitives(&output.primitives, |primitive| matches!(
            primitive,
            Primitive::Line(_)
        )) >= 8
    );
    assert!(
        count_primitives(&output.primitives, |primitive| matches!(
            primitive,
            Primitive::ClipBegin { .. }
        )) >= 2
    );
    assert!(output.primitives.iter().any(|primitive| {
            matches!(primitive, Primitive::Texture(texture) if texture.texture == super::VIEWPORT_TEXTURE)
        }));
    assert!(output.primitives.iter().any(|primitive| {
        matches!(primitive, Primitive::Text(text) if text.text == "CameraPreview")
    }));

    assert_eq!(count_semantic_role(&output, &SemanticRole::Dock), 1);
    assert!(count_semantic_role(&output, &SemanticRole::Frame) >= 5);
    assert!(count_semantic_role(&output, &SemanticRole::Panel) >= 5);
    assert!(count_semantic_role(&output, &SemanticRole::Viewport) >= 1);
    assert!(count_semantic_role(&output, &SemanticRole::Tab) >= 6);
    assert!(count_semantic_role(&output, &SemanticRole::IconButton) >= 12);
    assert!(count_semantic_role(&output, &SemanticRole::Slider) >= 1);
    assert!(output.semantics.nodes().iter().any(|node| {
        node.role == SemanticRole::IconButton
            && node.label.as_deref() == Some("Play")
            && node
                .actions
                .iter()
                .any(|action| action.kind == SemanticActionKind::Invoke)
    }));
    assert!(output.semantics.nodes().iter().any(|node| {
        node.role == SemanticRole::Slider
            && node
                .actions
                .iter()
                .any(|action| action.kind == SemanticActionKind::SetValue)
    }));
}

#[test]
fn editor_uses_phosphor_atlas_primitives_for_visible_editor_icons() {
    let theme = default_dark_theme();
    let mut memory = UiMemory::new();
    let context = editor_test_context(UiInput::default());
    let mut ui = Ui::begin_frame(context, &mut memory, &theme);
    let mut editor = EditorShowcase::new();

    editor.render(&mut ui, 0);
    let output = ui.finish_output();
    let atlas_icon_count = output
        .primitives
        .iter()
        .filter(
            |primitive| matches!(primitive, Primitive::Image(image) if is_editor_icon(image.image)),
        )
        .count();

    assert!(
        atlas_icon_count >= 24,
        "visible Phosphor icon count was {atlas_icon_count}"
    );
}

#[test]
fn editor_toolbar_icons_use_tinted_bitmap_atlas() {
    let theme = default_dark_theme();
    let mut memory = UiMemory::new();
    let context = editor_test_context(UiInput::default());
    let mut ui = Ui::begin_frame(context, &mut memory, &theme);
    let mut editor = EditorShowcase::new();

    editor.render(&mut ui, 0);
    let output = ui.finish_output();
    let toolbar_bitmap_icons = output
        .primitives
        .iter()
        .filter_map(|primitive| match primitive {
            Primitive::Image(image)
                if is_editor_icon(image.image) && point_is_in_toolbar(image.rect.center()) =>
            {
                Some(image)
            }
            _ => None,
        })
        .collect::<Vec<_>>();

    assert!(toolbar_bitmap_icons.len() >= 12);
    assert!(
        toolbar_bitmap_icons
            .iter()
            .all(|image| image.tint.is_some())
    );
}

#[test]
fn editor_toolbar_atlas_icons_use_integer_logical_destinations() {
    let theme = default_dark_theme();
    let chrome = EditorChromeMetrics::from_theme(&theme);
    let mut memory = UiMemory::new();
    let context = editor_test_context_scaled(UiInput::default(), ScaleFactor::new(1.25));
    let mut ui = Ui::begin_frame(context, &mut memory, &theme);
    let mut editor = EditorShowcase::new();

    editor.render(&mut ui, 0);
    let output = ui.finish_output();
    let mut checked = 0;

    for primitive in &output.primitives {
        let Primitive::Image(image) = primitive else {
            continue;
        };
        if !is_editor_icon(image.image) || !point_is_in_toolbar(image.rect.center()) {
            continue;
        }
        assert_eq!(image.rect.x, image.rect.x.round());
        assert_eq!(image.rect.y, image.rect.y.round());
        assert_eq!(image.rect.width, chrome.toolbar_icon);
        assert_eq!(image.rect.height, chrome.toolbar_icon);
        checked += 1;
    }

    assert!(checked >= 12);
}

#[test]
fn editor_icons_pick_exact_physical_atlas_for_dpi_scale() {
    let theme = default_dark_theme();
    let chrome = EditorChromeMetrics::from_theme(&theme);
    let dense = phosphor_icons::icon_image(
        phosphor_icons::PhosphorIcon::Search,
        super::DENSE_ICON_SIZE,
        1.25,
    );
    let toolbar = phosphor_icons::icon_image(
        phosphor_icons::PhosphorIcon::Cursor,
        chrome.toolbar_icon,
        1.5,
    );
    let dense_entry = icon_entry(dense);
    let toolbar_entry = icon_entry(toolbar);

    assert_eq!(dense_entry.logical_size, 16);
    assert_eq!(dense_entry.physical_size, 20);
    assert_eq!(toolbar_entry.logical_size, 16);
    assert_eq!(toolbar_entry.physical_size, 24);

    let fallback = phosphor_icons::icon_image(
        phosphor_icons::PhosphorIcon::Search,
        super::DENSE_ICON_SIZE,
        1.33,
    );
    assert_eq!(icon_entry(fallback).physical_size, 24);
}

#[test]
fn toolbar_icon_size_leaves_padding_inside_button() {
    let theme = default_dark_theme();
    let chrome = EditorChromeMetrics::from_theme(&theme);
    let mut memory = UiMemory::new();
    let context = editor_test_context(UiInput::default());
    let mut ui = Ui::begin_frame(context, &mut memory, &theme);
    let mut editor = EditorShowcase::new();
    let first_button = Rect::new(
        10.0,
        TOOLBAR_Y,
        chrome.toolbar_button,
        chrome.toolbar_button,
    );

    editor.render(&mut ui, 0);
    let output = ui.finish_output();
    let first_icon = output
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            Primitive::Image(image)
                if is_editor_icon(image.image)
                    && first_button.contains_point(image.rect.center()) =>
            {
                Some(image)
            }
            _ => None,
        })
        .expect("first toolbar icon");

    assert!(first_icon.rect.x >= first_button.x + 4.0);
    assert!(first_icon.rect.max_x() <= first_button.max_x() - 4.0);
    assert!(first_icon.rect.y >= first_button.y + 4.0);
    assert!(first_icon.rect.max_y() <= first_button.max_y() - 4.0);
}
#[test]
fn editor_toolbar_atlas_icons_preserve_icon_button_semantics() {
    let theme = default_dark_theme();
    let mut memory = UiMemory::new();
    let context = editor_test_context(UiInput::default());
    let mut ui = Ui::begin_frame(context, &mut memory, &theme);
    let mut editor = EditorShowcase::new();

    editor.render(&mut ui, 0);
    let output = ui.finish_output();
    let toolbar_labels = [
        "Select",
        "Move",
        "Rotate",
        "Scale",
        "Toggle grid",
        "Frame selected",
        "Reset view",
        "Play",
        "Pause",
        "Stop",
        "Build",
        "Export",
    ];

    for label in toolbar_labels {
        assert!(
            output.semantics.nodes().iter().any(|node| {
                node.role == SemanticRole::IconButton
                    && node.label.as_deref() == Some(label)
                    && node.focusable
            }),
            "missing toolbar icon semantics for {label}"
        );
    }
}

#[test]
fn editor_toolbar_atlas_icons_request_hover_cursor() {
    let theme = default_dark_theme();
    let mut memory = UiMemory::new();
    let context = editor_test_context(pointer_input_at(20.0, 44.0, false, false, false));
    let mut ui = Ui::begin_frame(context, &mut memory, &theme);
    let mut editor = EditorShowcase::new();

    editor.render(&mut ui, 0);
    let output = ui.finish_output();

    assert!(
        output
            .platform_requests
            .contains(&PlatformRequest::SetCursor(CursorShape::PointingHand))
    );
}

fn editor_test_context(input: UiInput) -> FrameContext {
    editor_test_context_scaled(input, ScaleFactor::ONE)
}

fn editor_test_context_scaled(input: UiInput, scale_factor: ScaleFactor) -> FrameContext {
    FrameContext::new(
        ViewportInfo::new(
            Size::new(1440.0, 900.0),
            PhysicalSize::new(
                (1440.0 * scale_factor.value()).round() as u32,
                (900.0 * scale_factor.value()).round() as u32,
            ),
            scale_factor,
        ),
        input,
        TimeInfo::default(),
    )
}

fn bottom_frame_rect(editor: &EditorShowcase) -> Rect {
    editor_frame_rect(editor, FRAME_BOTTOM)
}

fn editor_frame_rect(editor: &EditorShowcase, frame: super::FrameId) -> Rect {
    solve_dock_layout(&editor.dock, editor_workspace_bounds())
        .into_iter()
        .find(|layout| layout.frame == frame)
        .map(|layout| layout.rect.inset(2.0))
        .expect("editor frame")
}

fn editor_workspace_bounds() -> Rect {
    let viewport = Rect::new(0.0, 0.0, 1440.0, 900.0);
    let theme = default_dark_theme();
    let workspace_top = super::workspace_top(&theme);
    Rect::new(
        4.0,
        workspace_top,
        viewport.width - 8.0,
        viewport.height - workspace_top - 28.0,
    )
}

fn frame_body_rect(frame_rect: Rect) -> Rect {
    let tab_height = 26.0;
    Rect::new(
        frame_rect.x + 1.0,
        frame_rect.y + tab_height + 2.0,
        (frame_rect.width - 2.0).max(0.0),
        (frame_rect.height - tab_height - 3.0).max(0.0),
    )
}

fn point_is_in_toolbar(point: Point) -> bool {
    let chrome = EditorChromeMetrics::from_theme(&default_dark_theme());
    point.y >= TOOLBAR_Y && point.y <= TOOLBAR_Y + chrome.toolbar_button
}

fn count_primitives(primitives: &[Primitive], predicate: impl Fn(&Primitive) -> bool) -> usize {
    primitives
        .iter()
        .filter(|primitive| predicate(primitive))
        .count()
}

fn count_semantic_role(output: &kinetik_ui::core::FrameOutput, role: &SemanticRole) -> usize {
    output
        .semantics
        .nodes()
        .iter()
        .filter(|node| &node.role == role)
        .count()
}

fn focused_frame_semantic_count(output: &kinetik_ui::core::FrameOutput) -> usize {
    output
        .semantics
        .nodes()
        .iter()
        .filter(|node| node.role == SemanticRole::Frame && node.state.focused)
        .count()
}

fn pointer_input_at(x: f32, y: f32, down: bool, pressed: bool, released: bool) -> UiInput {
    pointer_input_at_with_delta(x, y, down, pressed, released, Vec2::ZERO)
}

fn pointer_input_at_with_delta(
    x: f32,
    y: f32,
    down: bool,
    pressed: bool,
    released: bool,
    delta: Vec2,
) -> UiInput {
    UiInput {
        pointer: PointerInput {
            position: Some(Point::new(x, y)),
            delta,
            primary: PointerButtonState::new(down, pressed, released),
            ..PointerInput::default()
        },
        ..UiInput::default()
    }
}

fn atlas_pixel(data: &[u8], width: u32, x: u32, y: u32) -> &[u8] {
    let start = ((y * width + x) * 4) as usize;
    &data[start..start + 4]
}

fn is_editor_icon(image: kinetik_ui::core::ImageId) -> bool {
    phosphor_icons::ICON_ENTRIES
        .iter()
        .any(|entry| entry.image == image)
}

fn icon_entry(image: kinetik_ui::core::ImageId) -> &'static phosphor_icons::PhosphorIconEntry {
    phosphor_icons::ICON_ENTRIES
        .iter()
        .find(|entry| entry.image == image)
        .expect("icon entry")
}
