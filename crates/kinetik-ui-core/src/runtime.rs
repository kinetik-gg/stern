//! UI frame runtime boundary types.

use std::time::Duration;

use crate::input::UiInput;
use crate::{
    ActionContext, ActionId, ActionInvocation, ActionQueue, ActionSource, PhysicalSize,
    ScaleFactor, Size,
};

/// Information about the current rendering viewport.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewportInfo {
    /// Size used by UI layout.
    pub logical_size: Size,
    /// Size of the physical render target.
    pub physical_size: PhysicalSize,
    /// Scale factor between logical and physical units.
    pub scale_factor: ScaleFactor,
}

impl ViewportInfo {
    /// Creates viewport information.
    #[must_use]
    pub const fn new(
        logical_size: Size,
        physical_size: PhysicalSize,
        scale_factor: ScaleFactor,
    ) -> Self {
        Self {
            logical_size,
            physical_size,
            scale_factor,
        }
    }
}

/// Time information for one UI frame.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TimeInfo {
    /// Monotonic timestamp relative to the application-defined start.
    pub now: Duration,
    /// Time since the previous frame.
    pub delta: Duration,
    /// Sequential frame number.
    pub frame_index: u64,
}

impl TimeInfo {
    /// Creates frame time information.
    #[must_use]
    pub const fn new(now: Duration, delta: Duration, frame_index: u64) -> Self {
        Self {
            now,
            delta,
            frame_index,
        }
    }
}

/// Context provided to the UI runtime at the beginning of a frame.
#[derive(Debug, Clone, PartialEq)]
pub struct FrameContext {
    /// Viewport and DPI information.
    pub viewport: ViewportInfo,
    /// Input snapshot for this frame.
    pub input: UiInput,
    /// Time snapshot for this frame.
    pub time: TimeInfo,
}

impl FrameContext {
    /// Creates a frame context.
    #[must_use]
    pub const fn new(viewport: ViewportInfo, input: UiInput, time: TimeInfo) -> Self {
        Self {
            viewport,
            input,
            time,
        }
    }
}

/// Request for when the platform adapter should schedule another redraw.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum RepaintRequest {
    /// No repaint is currently needed.
    #[default]
    None,
    /// Repaint as soon as the platform can present another frame.
    NextFrame,
    /// Repaint after the provided delay.
    After(Duration),
    /// Continue repainting while an external active condition remains true.
    Continuous,
}

impl RepaintRequest {
    /// Combines two repaint requests, preserving the more urgent request.
    #[must_use]
    pub fn merge(self, other: Self) -> Self {
        match (self, other) {
            (Self::Continuous, _) | (_, Self::Continuous) => Self::Continuous,
            (Self::NextFrame, _) | (_, Self::NextFrame) => Self::NextFrame,
            (Self::After(a), Self::After(b)) => Self::After(a.min(b)),
            (Self::After(delay), Self::None) | (Self::None, Self::After(delay)) => {
                Self::After(delay)
            }
            (Self::None, Self::None) => Self::None,
        }
    }
}

/// Output produced by a UI frame.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FrameOutput {
    /// Repaint scheduling request.
    pub repaint: RepaintRequest,
    /// Action invocations emitted during the frame.
    pub actions: ActionQueue,
}

impl FrameOutput {
    /// Creates empty frame output.
    #[must_use]
    pub fn new() -> Self {
        Self {
            repaint: RepaintRequest::None,
            actions: ActionQueue::new(),
        }
    }

    /// Requests repaint scheduling.
    pub fn request_repaint(&mut self, request: RepaintRequest) {
        self.repaint = self.repaint.merge(request);
    }

    /// Adds an action invocation to the frame output.
    pub fn push_action(&mut self, invocation: ActionInvocation) {
        self.actions.push(invocation);
    }

    /// Adds an action invocation from simple parts.
    pub fn invoke_action(
        &mut self,
        action_id: ActionId,
        source: ActionSource,
        context: ActionContext,
    ) {
        self.actions.invoke(action_id, source, context);
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::time::Duration;

    use super::{FrameContext, FrameOutput, RepaintRequest, TimeInfo, ViewportInfo};
    use crate::input::UiInput;
    use crate::{ActionContext, ActionId, ActionSource, PhysicalSize, ScaleFactor, Size};

    #[test]
    fn creates_viewport_info() {
        let viewport = ViewportInfo::new(
            Size::new(800.0, 600.0),
            PhysicalSize::new(1600, 1200),
            ScaleFactor::new(2.0),
        );

        assert_eq!(viewport.logical_size, Size::new(800.0, 600.0));
        assert_eq!(viewport.physical_size, PhysicalSize::new(1600, 1200));
        assert_eq!(viewport.scale_factor.value(), 2.0);
    }

    #[test]
    fn creates_frame_context() {
        let viewport = ViewportInfo::new(
            Size::new(100.0, 50.0),
            PhysicalSize::new(100, 50),
            ScaleFactor::ONE,
        );
        let time = TimeInfo::new(Duration::from_millis(16), Duration::from_millis(16), 1);
        let context = FrameContext::new(viewport, UiInput::default(), time);

        assert_eq!(context.viewport, viewport);
        assert_eq!(context.time.frame_index, 1);
    }

    #[test]
    fn frame_output_defaults_to_no_repaint() {
        let output = FrameOutput::new();

        assert_eq!(output.repaint, RepaintRequest::None);
        assert!(output.actions.is_empty());
    }

    #[test]
    fn repaint_request_merge_keeps_most_urgent_request() {
        assert_eq!(
            RepaintRequest::After(Duration::from_secs(2))
                .merge(RepaintRequest::After(Duration::from_secs(1))),
            RepaintRequest::After(Duration::from_secs(1))
        );
        assert_eq!(
            RepaintRequest::After(Duration::from_secs(1)).merge(RepaintRequest::NextFrame),
            RepaintRequest::NextFrame
        );
        assert_eq!(
            RepaintRequest::NextFrame.merge(RepaintRequest::Continuous),
            RepaintRequest::Continuous
        );
    }

    #[test]
    fn frame_output_accumulates_repaint_requests() {
        let mut output = FrameOutput::new();

        output.request_repaint(RepaintRequest::After(Duration::from_secs(5)));
        output.request_repaint(RepaintRequest::After(Duration::from_secs(1)));

        assert_eq!(
            output.repaint,
            RepaintRequest::After(Duration::from_secs(1))
        );
    }

    #[test]
    fn frame_output_accumulates_actions() {
        let mut output = FrameOutput::new();

        output.invoke_action(
            ActionId::new("file.save"),
            ActionSource::Shortcut,
            ActionContext::Global,
        );

        assert_eq!(output.actions.len(), 1);
        assert_eq!(
            output.actions.pop_front().expect("action").action_id,
            ActionId::new("file.save")
        );
    }
}
