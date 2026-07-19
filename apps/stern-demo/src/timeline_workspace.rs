use stern::core::{ActionContext, PointerOrder, PointerTarget, PointerTargetPlan, Rect, WidgetId};
use stern::widgets::chrome::{SystemFeedbackScene, SystemFeedbackSceneConfig};
use stern::widgets::{
    DiagnosticStrip, FeedbackId, FeedbackItem, FeedbackKind, FeedbackStack, JobList, JobPhase,
    JobProgress, JobRow, JobRowId, PanZoom, StatusItem, StatusItemId, StatusItemKind,
    TimelineClipEditController, TimelineClipEditIntent, TimelineClipEditRequest,
    TimelineDescriptor, TimelineFrame, TimelineFrameRate, TimelineFrameRounding,
    TimelineItemDescriptor, TimelineItemId, TimelineLaneDescriptor, TimelineLaneId, TimelineRange,
    TimelineScale, TimelineScrubController, TimelineScrubIntent, TimelineTime,
    TimelineViewportState, TimelineWidget, TimelineWidgetConfig, TimelineWidgetIntent,
    TimelineZoom, Ui, ViewportActionDescriptor, ViewportActionKind, ViewportActionTarget,
    ViewportCursorMetadata, ViewportCursorShape, ViewportSelectionTargetDescriptor,
    ViewportSelectionTargetId, ViewportToolController, ViewportToolDescriptor, ViewportToolId,
    ViewportToolScene, ViewportToolSceneConfig, ViewportTransformHandleSet, ViewportWidget,
};

use crate::{
    DemoActionRegistry, DemoApplicationModel, DemoColorSaveState, DemoJobPhase, DemoViewportTool,
};

const FRAME_RATE: TimelineFrameRate = TimelineFrameRate::integer(30);
const TIMELINE_LANE: TimelineLaneId = TimelineLaneId::from_raw(1);
const TIMELINE_CLIP: TimelineItemId = TimelineItemId::from_raw(1);
const VIEWPORT_TARGET: ViewportSelectionTargetId = ViewportSelectionTargetId::from_raw(1);
const SELECT_TOOL: ViewportToolId = ViewportToolId::from_raw(1);
const TRANSFORM_TOOL: ViewportToolId = ViewportToolId::from_raw(2);
const JOB_ID: JobRowId = JobRowId::from_raw(1);
const FEEDBACK_ID: FeedbackId = FeedbackId::from_raw(1);
const COLOR_FEEDBACK_ID: FeedbackId = FeedbackId::from_raw(2);

/// Retained public Stern state for the timeline, viewport tools, and feedback projection.
pub(crate) struct TimelineWorkspace {
    pub(crate) descriptor: TimelineDescriptor,
    pub(crate) viewport_state: TimelineViewportState,
    pub(crate) scrub: TimelineScrubController,
    pub(crate) clip_edit: TimelineClipEditController,
    pub(crate) pan_zoom: PanZoom,
    pub(crate) viewport_tools: ViewportToolController,
    pub(crate) jobs: JobList,
    pub(crate) diagnostics: DiagnosticStrip,
    pub(crate) feedback: FeedbackStack,
}

impl TimelineWorkspace {
    pub(crate) fn new() -> Self {
        let scale = TimelineScale::new(
            0.0,
            0.0,
            TimelineRange::seconds(0.0, 8.0),
            TimelineZoom::new(48.0),
            0.0,
        );
        Self {
            descriptor: descriptor((30, 90)),
            viewport_state: TimelineViewportState::new(scale)
                .with_playhead_time(FRAME_RATE.frame_to_time(TimelineFrame::from_raw(24))),
            scrub: TimelineScrubController::default(),
            clip_edit: TimelineClipEditController::default(),
            pan_zoom: PanZoom::default(),
            viewport_tools: ViewportToolController::default(),
            jobs: JobList::new(),
            diagnostics: DiagnosticStrip::new(),
            feedback: FeedbackStack::new(),
        }
    }

    pub(crate) fn project(&mut self, model: &DemoApplicationModel) {
        self.descriptor = descriptor(model.clip_frames());
        self.viewport_state.playhead_time =
            Some(FRAME_RATE.frame_to_time(TimelineFrame::from_raw(model.playhead_frame())));
        let phase = match model.job_phase() {
            DemoJobPhase::Running => JobPhase::Running,
            DemoJobPhase::Succeeded => JobPhase::Succeeded,
            DemoJobPhase::Failed => JobPhase::Failed,
        };
        self.jobs
            .replace_rows([JobRow::new(JOB_ID, "Preview render", phase)
                .with_progress(JobProgress::from_fraction(
                    f32::from(model.job_progress_percent()),
                    100.0,
                ))
                .with_detail(job_detail(model))]);
        let mut feedback = match model.job_phase() {
            DemoJobPhase::Running => Vec::new(),
            DemoJobPhase::Succeeded => vec![FeedbackItem::pinned(
                FEEDBACK_ID,
                FeedbackKind::Success,
                "Preview complete",
                "Viewport reflects the committed timeline",
            )],
            DemoJobPhase::Failed => vec![FeedbackItem::pinned(
                FEEDBACK_ID,
                FeedbackKind::Error,
                "Preview failed",
                "The committed timeline remains unchanged",
            )],
        };
        match model.color_save_state() {
            DemoColorSaveState::Idle => {}
            DemoColorSaveState::Failed => feedback.push(FeedbackItem::pinned(
                COLOR_FEEDBACK_ID,
                FeedbackKind::Error,
                "Color style save failed",
                "No serialized color or gradient was committed; retry is available",
            )),
            DemoColorSaveState::Succeeded => feedback.push(FeedbackItem::pinned(
                COLOR_FEEDBACK_ID,
                FeedbackKind::Success,
                "Color style saved",
                "Explicit sRGB color and stable gradient stops were committed",
            )),
        }
        self.feedback.replace_items(feedback);
    }

    pub(crate) fn viewport_scene(
        ui: &Ui<'_>,
        viewport: &ViewportWidget,
        tool: DemoViewportTool,
    ) -> ViewportToolScene {
        let active = match tool {
            DemoViewportTool::Select => ViewportToolDescriptor::new(SELECT_TOOL, "Select Tool")
                .active(true)
                .with_cursor(ViewportCursorMetadata::new(ViewportCursorShape::Pointer)),
            DemoViewportTool::Transform => {
                ViewportToolDescriptor::new(TRANSFORM_TOOL, "Transform Tool")
                    .active(true)
                    .with_cursor(ViewportCursorMetadata::new(ViewportCursorShape::Move))
            }
        };
        let target = ViewportSelectionTargetDescriptor::new(
            VIEWPORT_TARGET,
            Rect::new(410.0, 240.0, 300.0, 180.0),
        )
        .with_label("Selected clip content")
        .with_handles(ViewportTransformHandleSet::move_only());
        ui.prepare_viewport_tool_scene(
            viewport,
            ViewportToolSceneConfig::new([target])
                .with_active_tool(active)
                .disabled(tool == DemoViewportTool::Select),
        )
    }

    pub(crate) fn status_items(model: &DemoApplicationModel) -> Vec<StatusItem> {
        let progress = f32::from(model.job_progress_percent()) / 100.0;
        let job = match model.job_phase() {
            DemoJobPhase::Running => StatusItem::new(
                StatusItemId::from_raw(2),
                "Preview progress",
                format!("Preview {}%", model.job_progress_percent()),
                StatusItemKind::Progress,
            )
            .with_progress_value(progress),
            DemoJobPhase::Succeeded => StatusItem::new(
                StatusItemId::from_raw(2),
                "Preview status",
                "Preview complete",
                StatusItemKind::Ready,
            ),
            DemoJobPhase::Failed => StatusItem::new(
                StatusItemId::from_raw(2),
                "Preview status",
                "Preview failed",
                StatusItemKind::Error,
            ),
        };
        vec![job]
    }
}

pub(crate) fn prepare_timeline<'a>(
    ui: &Ui<'_>,
    bounds: Rect,
    descriptor: &'a TimelineDescriptor,
    state: &'a TimelineViewportState,
) -> TimelineWidget<'a> {
    ui.prepare_timeline_widget(
        TimelineWidgetConfig::new(
            WidgetId::from_key("edit-workspace.timeline"),
            bounds,
            FRAME_RATE,
            descriptor,
            state,
        )
        .with_label("Timeline")
        .with_lane_header_width(72.0)
        .with_ruler_height(22.0),
    )
    .expect("deterministic demo timeline is valid")
}

pub(crate) fn prepare_feedback<'a>(
    ui: &Ui<'_>,
    bounds: Rect,
    jobs: &'a JobList,
    diagnostics: &'a DiagnosticStrip,
    feedback: &'a FeedbackStack,
) -> SystemFeedbackScene<'a> {
    let half = bounds.height * 0.5;
    ui.prepare_system_feedback(
        SystemFeedbackSceneConfig::new(
            WidgetId::from_key("edit-workspace.feedback"),
            Rect::new(bounds.x, bounds.y, bounds.width, half),
            Rect::ZERO,
            Rect::new(
                bounds.x,
                bounds.y + half,
                bounds.width,
                bounds.height - half,
            ),
        )
        .with_row_height(half.max(1.0)),
        jobs,
        diagnostics,
        feedback,
    )
    .expect("deterministic demo feedback is valid")
}

pub(crate) fn viewport_tool_rects(bounds: Rect) -> [Rect; 2] {
    [
        Rect::new(bounds.x, bounds.y, 92.0_f32.min(bounds.width), 26.0),
        Rect::new(
            bounds.x + 96.0,
            bounds.y,
            112.0_f32.min((bounds.width - 96.0).max(0.0)),
            26.0,
        ),
    ]
}

pub(crate) fn viewport_content_rect(bounds: Rect) -> Rect {
    Rect::new(
        bounds.x,
        bounds.y + 30.0,
        bounds.width,
        (bounds.height - 30.0).max(0.0),
    )
}

pub(crate) fn viewport_actions(
    actions: &DemoActionRegistry,
    viewport: WidgetId,
) -> [ViewportActionDescriptor; 2] {
    [
        ViewportActionDescriptor::new(
            actions.viewport_select().clone(),
            ViewportActionKind::ActivateTool,
            ViewportActionTarget::new(viewport).with_tool(SELECT_TOOL),
        ),
        ViewportActionDescriptor::new(
            actions.viewport_transform().clone(),
            ViewportActionKind::ActivateTool,
            ViewportActionTarget::new(viewport).with_tool(TRANSFORM_TOOL),
        ),
    ]
}

pub(crate) fn timeline_feedback_rects(bounds: Rect) -> (Rect, Rect) {
    let feedback_width = 180.0_f32.min(bounds.width * 0.38);
    (
        Rect::new(
            bounds.x,
            bounds.y,
            (bounds.width - feedback_width - 4.0).max(0.0),
            bounds.height,
        ),
        Rect::new(
            bounds.max_x() - feedback_width,
            bounds.y,
            feedback_width,
            bounds.height,
        ),
    )
}

pub(crate) fn declare_tool_actions(
    plan: &mut PointerTargetPlan,
    mut next: PointerOrder,
    root: WidgetId,
    actions: &DemoActionRegistry,
    rects: [Rect; 2],
) -> PointerOrder {
    for (action, rect) in [actions.viewport_select(), actions.viewport_transform()]
        .into_iter()
        .zip(rects)
    {
        plan.target(PointerTarget::new(
            root.child(action.id.as_str()),
            rect,
            next,
        ));
        next = PointerOrder::new(next.raw().saturating_add(1));
    }
    next
}

pub(crate) fn compose_tool_actions(
    ui: &mut Ui<'_>,
    actions: &DemoActionRegistry,
    rects: [Rect; 2],
) {
    for (action, rect) in [actions.viewport_select(), actions.viewport_transform()]
        .into_iter()
        .zip(rects)
    {
        let _ = ui.action_button(action.id.as_str(), rect, action, ActionContext::Editor);
    }
}

pub(crate) fn apply_timeline_output(
    model: &mut DemoApplicationModel,
    intent: Option<TimelineWidgetIntent>,
    scrub_intents: &[TimelineScrubIntent],
    clip_intents: &[TimelineClipEditIntent],
) {
    if let Some(TimelineWidgetIntent::Seek(request)) = intent {
        model.commit_playhead(request.frame.raw());
    }
    for intent in scrub_intents {
        match intent {
            TimelineScrubIntent::Begin(request) => {
                model.preview_playhead(frame(request.current_time));
            }
            TimelineScrubIntent::Update(request) => {
                model.preview_playhead(frame(request.current_time));
            }
            TimelineScrubIntent::End(request) => model.commit_playhead(frame(request.current_time)),
            TimelineScrubIntent::Cancel(_) => model.cancel_playhead_preview(),
        }
    }
    for intent in clip_intents {
        match intent {
            TimelineClipEditIntent::Begin(request) | TimelineClipEditIntent::Update(request) => {
                let (start, end) = clip_frames(*request);
                model.preview_clip(start, end);
            }
            TimelineClipEditIntent::End(request) => {
                let (start, end) = clip_frames(*request);
                model.commit_clip(start, end);
            }
            TimelineClipEditIntent::Cancel(_) => model.cancel_clip_preview(),
            TimelineClipEditIntent::Reject(_) => {}
        }
    }
}

fn descriptor(clip: (i64, i64)) -> TimelineDescriptor {
    TimelineDescriptor::new(
        [TimelineLaneDescriptor::new(TIMELINE_LANE, "Video")],
        [TimelineItemDescriptor::new(
            TIMELINE_CLIP,
            TIMELINE_LANE,
            TimelineRange::new(
                FRAME_RATE.frame_to_time(TimelineFrame::from_raw(clip.0)),
                FRAME_RATE.frame_to_time(TimelineFrame::from_raw(clip.1)),
            ),
            "Hero clip",
        )],
        [],
        [],
    )
}

fn clip_frames(request: TimelineClipEditRequest) -> (i64, i64) {
    let range = request.accepted_range();
    (frame(range.start), frame(range.end))
}

fn frame(time: TimelineTime) -> i64 {
    FRAME_RATE
        .time_to_frame(time, TimelineFrameRounding::Nearest)
        .raw()
}

fn job_detail(model: &DemoApplicationModel) -> String {
    match model.job_phase() {
        DemoJobPhase::Running => format!("{}%", model.job_progress_percent()),
        DemoJobPhase::Succeeded => "Succeeded".to_owned(),
        DemoJobPhase::Failed => "Failed".to_owned(),
    }
}
