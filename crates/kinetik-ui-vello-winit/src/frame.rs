use kinetik_ui_render::{RenderFrameInput, RenderFrameOutput};

use crate::{
    VelloPresentReport, VelloPresentStatus, VelloPresenterError, VelloRedrawGuidance,
    device::CurrentDeviceEventOutcome, lifecycle::Extent,
};

pub(crate) enum AcquiredFrame<F> {
    Success(F),
    Suboptimal(F),
    Timeout,
    Occluded,
    Outdated,
    Lost,
    Validation,
}

pub(crate) trait PresentOperations {
    type Frame;
    type RenderError;

    fn acquire(&mut self) -> AcquiredFrame<Self::Frame>;
    fn acquired_extent(&mut self, frame: &Self::Frame) -> Extent;
    fn drop_frame(&mut self, frame: Self::Frame);
    fn reconfigure(&mut self);
    fn encode_scene(&mut self, input: RenderFrameInput<'_>) -> RenderFrameOutput;
    fn render_vello(&mut self) -> Result<(), Self::RenderError>;
    fn blit_submit(&mut self, frame: &Self::Frame);
    fn pre_present_notify(&mut self);
    fn present(&mut self, frame: Self::Frame);
    fn device_events_after_render_failure(&mut self) -> CurrentDeviceEventOutcome;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DrivenFrame {
    Presented {
        output: RenderFrameOutput,
        suboptimal: bool,
    },
    FrameExtentOutdated,
    AcquiredExtentOutdated,
    Timeout,
    Occluded,
    Outdated,
    Lost,
    Validation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DriveFailure<E> {
    Render(E),
    DeviceLostAfterRender,
    Actionable(VelloPresenterError),
}

pub(crate) fn drive_present<O: PresentOperations>(
    operations: &mut O,
    input: RenderFrameInput<'_>,
    configured_extent: Extent,
) -> Result<DrivenFrame, DriveFailure<O::RenderError>> {
    if Extent::from(input.viewport.physical_size) != configured_extent {
        return Ok(DrivenFrame::FrameExtentOutdated);
    }
    let (frame, suboptimal) = match operations.acquire() {
        AcquiredFrame::Success(frame) => (frame, false),
        AcquiredFrame::Suboptimal(frame) => (frame, true),
        AcquiredFrame::Timeout => return Ok(DrivenFrame::Timeout),
        AcquiredFrame::Occluded => return Ok(DrivenFrame::Occluded),
        AcquiredFrame::Outdated => {
            operations.reconfigure();
            return Ok(DrivenFrame::Outdated);
        }
        AcquiredFrame::Lost => return Ok(DrivenFrame::Lost),
        AcquiredFrame::Validation => return Ok(DrivenFrame::Validation),
    };

    if operations.acquired_extent(&frame) != configured_extent {
        operations.drop_frame(frame);
        operations.reconfigure();
        return Ok(DrivenFrame::AcquiredExtentOutdated);
    }

    let output = operations.encode_scene(input);
    if let Err(error) = operations.render_vello() {
        operations.drop_frame(frame);
        return match operations.device_events_after_render_failure() {
            CurrentDeviceEventOutcome::Lost => Err(DriveFailure::DeviceLostAfterRender),
            CurrentDeviceEventOutcome::Actionable(actionable) => {
                Err(DriveFailure::Actionable(actionable))
            }
            CurrentDeviceEventOutcome::None => Err(DriveFailure::Render(error)),
        };
    }
    operations.blit_submit(&frame);
    operations.pre_present_notify();
    operations.present(frame);

    Ok(DrivenFrame::Presented { output, suboptimal })
}

pub(crate) fn report_for_driven(
    driven: DrivenFrame,
    timeout_retry: std::time::Duration,
) -> Result<VelloPresentReport, VelloPresenterError> {
    let report = match driven {
        DrivenFrame::Presented { output, suboptimal } => VelloPresentReport::new(
            if suboptimal {
                VelloPresentStatus::PresentedSuboptimal
            } else {
                VelloPresentStatus::Presented
            },
            if suboptimal {
                VelloRedrawGuidance::NextFrame
            } else {
                VelloRedrawGuidance::UseApplicationRequest
            },
            Some(output),
        ),
        DrivenFrame::FrameExtentOutdated => VelloPresentReport::new(
            VelloPresentStatus::FrameExtentOutdated,
            VelloRedrawGuidance::NextFrame,
            None,
        ),
        DrivenFrame::AcquiredExtentOutdated => VelloPresentReport::new(
            VelloPresentStatus::AcquiredExtentOutdated,
            VelloRedrawGuidance::NextFrame,
            None,
        ),
        DrivenFrame::Timeout => VelloPresentReport::new(
            VelloPresentStatus::Timeout,
            VelloRedrawGuidance::Later(timeout_retry),
            None,
        ),
        DrivenFrame::Occluded => VelloPresentReport::new(
            VelloPresentStatus::Occluded,
            VelloRedrawGuidance::ExternalEvent,
            None,
        ),
        DrivenFrame::Outdated => VelloPresentReport::new(
            VelloPresentStatus::Outdated,
            VelloRedrawGuidance::NextFrame,
            None,
        ),
        DrivenFrame::Lost => VelloPresentReport::new(
            VelloPresentStatus::SurfaceLost,
            VelloRedrawGuidance::NextFrame,
            None,
        ),
        DrivenFrame::Validation => {
            return Err(VelloPresenterError::Validation {
                message: "wgpu rejected surface acquisition".into(),
            });
        }
    };
    Ok(report)
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FrameOperation {
    Acquire,
    ValidateAcquiredExtent,
    DropAcquired,
    Reconfigure,
    EncodeScene,
    VelloRenderSubmit,
    BlitSubmit,
    PrePresentNotify,
    Present,
    PollAfterRenderFailure,
}
