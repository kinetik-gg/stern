use stern::core::{ActionContext, PointerOrder, PointerTarget, PointerTargetPlan, Rect, WidgetId};
use stern::widgets::chrome::{SystemFeedbackScene, SystemFeedbackSceneConfig};
use stern::widgets::{
    DiagnosticStrip, FeedbackId, FeedbackItem, FeedbackKind, FeedbackStack, JobList, JobPhase,
    JobProgress, JobRow, JobRowId, PanZoom, StatusItem, StatusItemId, StatusItemKind,
    TimelineClipEditController, TimelineClipEditIntent, TimelineClipEditRequest,
    TimelineDescriptor, TimelineFrame, TimelineFrameRate, TimelineFrameRounding,
    TimelineItemDescriptor, TimelineItemId, TimelineKeyframeDescriptor, TimelineKeyframeId,
    TimelineLaneDescriptor, TimelineLaneId, TimelineRange, TimelineScale, TimelineScrubController,
    TimelineScrubIntent, TimelineTime, TimelineViewportState, TimelineWidget, TimelineWidgetConfig,
    TimelineWidgetIntent, TimelineZoom, Ui, ViewportActionDescriptor, ViewportActionKind,
    ViewportActionTarget, ViewportCursorMetadata, ViewportCursorShape,
    ViewportSelectionTargetDescriptor, ViewportSelectionTargetId, ViewportToolController,
    ViewportToolDescriptor, ViewportToolId, ViewportToolScene, ViewportToolSceneConfig,
    ViewportTransformHandleSet, ViewportWidget,
};

use crate::{
    DemoActionRegistry, DemoApplicationModel, DemoColorSaveState, DemoJobPhase, DemoViewportTool,
};

const TIMELINE_LANE: TimelineLaneId = TimelineLaneId::from_raw(1);
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
    pub(crate) fn new(model: &DemoApplicationModel) -> Self {
        let position = model.timeline().position();
        Self {
            descriptor: descriptor(model),
            viewport_state: TimelineViewportState::new(scale(model))
                .with_playhead_time(position.time()),
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
        self.descriptor = descriptor(model);
        self.viewport_state.scale.content_range = scale(model).content_range;
        self.viewport_state.playhead_time = Some(model.timeline().position().time());
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
        model: &DemoApplicationModel,
    ) -> ViewportToolScene {
        let tool = model.viewport_tool();
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
        .with_label(if model.scenario().has_timeline_journey() {
            format!(
                "{} · {}",
                model.timeline().clip_label(),
                model.timeline().position().label()
            )
        } else {
            "Selected clip content".to_owned()
        })
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
        if model.scenario().has_timeline_journey() {
            vec![
                StatusItem::new(
                    StatusItemId::from_raw(3),
                    "Timeline position",
                    format!(
                        "{} · {}",
                        model.timeline().position().label(),
                        model.transport_state().label()
                    ),
                    StatusItemKind::Message,
                ),
                job,
            ]
        } else {
            vec![job]
        }
    }
}

pub(crate) fn prepare_timeline<'a>(
    ui: &Ui<'_>,
    bounds: Rect,
    descriptor: &'a TimelineDescriptor,
    state: &'a TimelineViewportState,
    frame_rate: TimelineFrameRate,
) -> TimelineWidget<'a> {
    ui.prepare_timeline_widget(
        TimelineWidgetConfig::new(
            WidgetId::from_key("edit-workspace.timeline"),
            bounds,
            frame_rate,
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

pub(crate) fn timeline_transport_layout(bounds: Rect) -> ([Rect; 2], Rect) {
    let controls_height = 26.0_f32.min(bounds.height);
    let controls = [
        Rect::new(
            bounds.x,
            bounds.y,
            76.0_f32.min(bounds.width),
            controls_height,
        ),
        Rect::new(
            bounds.x + 80.0,
            bounds.y,
            76.0_f32.min((bounds.width - 80.0).max(0.0)),
            controls_height,
        ),
    ];
    let gap = 4.0_f32.min((bounds.height - controls_height).max(0.0));
    (
        controls,
        Rect::new(
            bounds.x,
            bounds.y + controls_height + gap,
            bounds.width,
            (bounds.height - controls_height - gap).max(0.0),
        ),
    )
}

pub(crate) fn declare_transport_actions(
    plan: &mut PointerTargetPlan,
    mut next: PointerOrder,
    root: WidgetId,
    actions: &DemoActionRegistry,
    rects: [Rect; 2],
) -> PointerOrder {
    for (action, rect) in [actions.transport_play_pause(), actions.transport_stop()]
        .into_iter()
        .zip(rects)
    {
        plan.target(
            PointerTarget::new(root.child(action.id.as_str()), rect, next)
                .enabled(action.can_invoke()),
        );
        next = PointerOrder::new(next.raw().saturating_add(1));
    }
    next
}

pub(crate) fn compose_transport_actions(
    ui: &mut Ui<'_>,
    actions: &DemoActionRegistry,
    rects: [Rect; 2],
) {
    for (action, rect) in [actions.transport_play_pause(), actions.transport_stop()]
        .into_iter()
        .zip(rects)
    {
        let _ = ui.action_button(action.id.as_str(), rect, action, ActionContext::Editor);
    }
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
    let frame_rate = model.timeline().frame_rate();
    if let Some(TimelineWidgetIntent::Seek(request)) = intent {
        model.commit_playhead(request.frame.raw());
    }
    for intent in scrub_intents {
        match intent {
            TimelineScrubIntent::Begin(request) => {
                model.preview_playhead(frame(request.current_time, frame_rate));
            }
            TimelineScrubIntent::Update(request) => {
                model.preview_playhead(frame(request.current_time, frame_rate));
            }
            TimelineScrubIntent::End(request) => {
                model.commit_playhead(frame(request.current_time, frame_rate));
            }
            TimelineScrubIntent::Cancel(_) => model.cancel_playhead_preview(),
        }
    }
    for intent in clip_intents {
        match intent {
            TimelineClipEditIntent::Begin(request) | TimelineClipEditIntent::Update(request) => {
                let (start, end) = clip_frames(*request, frame_rate);
                model.preview_clip(start, end);
            }
            TimelineClipEditIntent::End(request) => {
                let (start, end) = clip_frames(*request, frame_rate);
                model.commit_clip(start, end);
            }
            TimelineClipEditIntent::Cancel(_) => model.cancel_clip_preview(),
            TimelineClipEditIntent::Reject(_) => {}
        }
    }
}

fn descriptor(model: &DemoApplicationModel) -> TimelineDescriptor {
    let timeline = model.timeline();
    let frame_rate = timeline.frame_rate();
    let clip = timeline.clip_frames();
    let clip_id = TimelineItemId::from_raw(timeline.clip_id());
    TimelineDescriptor::new(
        [TimelineLaneDescriptor::new(TIMELINE_LANE, "Video")],
        [TimelineItemDescriptor::new(
            clip_id,
            TIMELINE_LANE,
            TimelineRange::new(
                frame_rate.frame_to_time(TimelineFrame::from_raw(clip.0)),
                frame_rate.frame_to_time(TimelineFrame::from_raw(clip.1)),
            ),
            timeline.clip_label(),
        )],
        [],
        timeline
            .keyframes()
            .iter()
            .map(|keyframe| {
                TimelineKeyframeDescriptor::new(
                    TimelineKeyframeId::from_raw(keyframe.id()),
                    clip_id,
                    frame_rate.frame_to_time(TimelineFrame::from_raw(keyframe.frame())),
                )
                .with_label(format!(
                    "{} · frame {}",
                    keyframe.label(),
                    keyframe.frame()
                ))
            })
            .collect::<Vec<_>>(),
    )
}

fn clip_frames(request: TimelineClipEditRequest, frame_rate: TimelineFrameRate) -> (i64, i64) {
    let range = request.accepted_range();
    (frame(range.start, frame_rate), frame(range.end, frame_rate))
}

fn frame(time: TimelineTime, frame_rate: TimelineFrameRate) -> i64 {
    frame_rate
        .time_to_frame(time, TimelineFrameRounding::Nearest)
        .raw()
}

fn scale(model: &DemoApplicationModel) -> TimelineScale {
    let timeline = model.timeline();
    let (start, end) = timeline.frame_range();
    TimelineScale::new(
        0.0,
        0.0,
        TimelineRange::new(
            timeline
                .frame_rate()
                .frame_to_time(TimelineFrame::from_raw(start)),
            timeline
                .frame_rate()
                .frame_to_time(TimelineFrame::from_raw(end)),
        ),
        TimelineZoom::new(48.0),
        0.0,
    )
}

fn job_detail(model: &DemoApplicationModel) -> String {
    match model.job_phase() {
        DemoJobPhase::Running => format!("{}%", model.job_progress_percent()),
        DemoJobPhase::Succeeded => "Succeeded".to_owned(),
        DemoJobPhase::Failed => "Failed".to_owned(),
    }
}
