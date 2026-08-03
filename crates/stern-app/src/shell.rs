use std::time::Duration;

use stern_core::{
    CursorShape, FrameOutput, PlatformRequest, RepaintRequest, ScaleFactor, WidgetId,
};

/// Retained-focus change requested through [`ShellCtx`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShellFocusRequest {
    /// Focus the widget after the current frame is finished.
    Focus(WidgetId),
    /// Clear retained focus after the current frame is finished.
    Clear,
}

/// Per-frame application-shell context passed to [`crate::App`] hooks.
///
/// The context records intent only, as thin wrappers over the existing
/// [`PlatformRequest`] vocabulary plus runner-owned repaint, close, and
/// retained-focus state. The runner merges the recorded intent into the frame
/// output after [`crate::App::frame`] and [`crate::App::on_actions`] return,
/// so requests recorded in either hook take effect in the same frame.
/// Explicit shell requests are appended after widget-emitted requests, so the
/// final cursor or window title recorded here wins over widget requests.
#[derive(Debug, PartialEq)]
pub struct ShellCtx {
    scale_factor: ScaleFactor,
    close_requested: bool,
    requests: Vec<PlatformRequest>,
    repaint: RepaintRequest,
    focus: Option<ShellFocusRequest>,
}

impl ShellCtx {
    /// Creates an empty context for one frame at the given scale factor.
    #[must_use]
    pub const fn new(scale_factor: ScaleFactor) -> Self {
        Self {
            scale_factor,
            close_requested: false,
            requests: Vec::new(),
            repaint: RepaintRequest::None,
            focus: None,
        }
    }

    /// Returns the current window scale factor.
    #[must_use]
    pub const fn scale_factor(&self) -> ScaleFactor {
        self.scale_factor
    }

    /// Requests that the runner exit its event loop after this frame.
    pub const fn request_close(&mut self) {
        self.close_requested = true;
    }

    /// Returns whether the application requested to close.
    #[must_use]
    pub const fn close_requested(&self) -> bool {
        self.close_requested
    }

    /// Requests a window title update.
    pub fn set_window_title(&mut self, title: impl Into<String>) {
        self.requests
            .push(PlatformRequest::SetWindowTitle(title.into()));
    }

    /// Requests the final pointer cursor for this frame.
    pub fn set_cursor(&mut self, cursor: CursorShape) {
        self.requests.push(PlatformRequest::SetCursor(cursor));
    }

    /// Requests a clipboard write.
    pub fn copy_to_clipboard(&mut self, text: impl Into<String>) {
        self.requests
            .push(PlatformRequest::CopyToClipboard(text.into()));
    }

    /// Requests clipboard text as future input for the target widget.
    pub fn request_clipboard_text(&mut self, target: WidgetId) {
        self.requests
            .push(PlatformRequest::RequestClipboardText { target });
    }

    /// Requests one repaint as soon as the platform can present a frame.
    pub fn request_repaint(&mut self) {
        self.merge_repaint(RepaintRequest::NextFrame);
    }

    /// Requests a repaint after the provided delay.
    pub fn request_repaint_after(&mut self, delay: Duration) {
        self.merge_repaint(RepaintRequest::After(delay));
    }

    /// Requests continuous repainting until a later frame replaces it.
    pub fn request_continuous_repaint(&mut self) {
        self.merge_repaint(RepaintRequest::Continuous);
    }

    /// Requests retained focus for a widget after this frame is finished.
    ///
    /// The last focus request recorded during a frame wins.
    pub fn request_widget_focus(&mut self, target: WidgetId) {
        self.focus = Some(ShellFocusRequest::Focus(target));
    }

    /// Requests that retained focus is cleared after this frame is finished.
    pub fn clear_widget_focus(&mut self) {
        self.focus = Some(ShellFocusRequest::Clear);
    }

    /// Returns the recorded platform requests in order.
    #[must_use]
    pub fn requests(&self) -> &[PlatformRequest] {
        &self.requests
    }

    /// Returns the merged repaint request recorded so far.
    #[must_use]
    pub const fn repaint(&self) -> RepaintRequest {
        self.repaint
    }

    /// Returns the pending retained-focus request, when present.
    #[must_use]
    pub const fn focus_request(&self) -> Option<&ShellFocusRequest> {
        self.focus.as_ref()
    }

    /// Consumes the pending retained-focus request.
    pub(crate) fn take_focus_request(&mut self) -> Option<ShellFocusRequest> {
        self.focus.take()
    }

    /// Merges the recorded requests and repaint intent into a frame output.
    pub(crate) fn merge_into(&mut self, output: &mut FrameOutput) {
        output.platform_requests.append(&mut self.requests);
        output.repaint = output.repaint.merge(std::mem::take(&mut self.repaint));
    }

    fn merge_repaint(&mut self, request: RepaintRequest) {
        self.repaint = self.repaint.merge(request);
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use stern_core::{
        CursorShape, FrameOutput, PlatformRequest, RepaintRequest, ScaleFactor, WidgetId,
    };

    use super::{ShellCtx, ShellFocusRequest};

    #[test]
    fn shell_requests_map_onto_the_existing_platform_request_vocabulary() {
        let target = WidgetId::from_key("paste-target");
        let mut shell = ShellCtx::new(ScaleFactor::new(2.0));
        shell.set_window_title("Retitled");
        shell.set_cursor(CursorShape::Text);
        shell.copy_to_clipboard("copied");
        shell.request_clipboard_text(target);

        assert_eq!(shell.scale_factor(), ScaleFactor::new(2.0));
        assert_eq!(
            shell.requests(),
            &[
                PlatformRequest::SetWindowTitle("Retitled".to_owned()),
                PlatformRequest::SetCursor(CursorShape::Text),
                PlatformRequest::CopyToClipboard("copied".to_owned()),
                PlatformRequest::RequestClipboardText { target },
            ]
        );
        assert!(!shell.close_requested());
    }

    #[test]
    fn repaint_requests_merge_toward_the_most_urgent_schedule() {
        let mut shell = ShellCtx::new(ScaleFactor::ONE);
        assert_eq!(shell.repaint(), RepaintRequest::None);

        shell.request_repaint_after(Duration::from_millis(250));
        assert_eq!(
            shell.repaint(),
            RepaintRequest::After(Duration::from_millis(250))
        );
        shell.request_repaint();
        assert_eq!(shell.repaint(), RepaintRequest::NextFrame);
        shell.request_repaint_after(Duration::from_millis(250));
        assert_eq!(shell.repaint(), RepaintRequest::NextFrame);
        shell.request_continuous_repaint();
        assert_eq!(shell.repaint(), RepaintRequest::Continuous);
    }

    #[test]
    fn merge_appends_shell_requests_after_widget_requests_and_merges_repaint() {
        let mut output = FrameOutput::new();
        output
            .platform_requests
            .push(PlatformRequest::SetCursor(CursorShape::PointingHand));
        output.repaint = RepaintRequest::After(Duration::from_secs(1));
        let mut shell = ShellCtx::new(ScaleFactor::ONE);
        shell.set_cursor(CursorShape::Grabbing);
        shell.request_repaint();

        shell.merge_into(&mut output);

        assert_eq!(
            output.platform_requests,
            [
                PlatformRequest::SetCursor(CursorShape::PointingHand),
                PlatformRequest::SetCursor(CursorShape::Grabbing),
            ]
        );
        assert_eq!(output.repaint, RepaintRequest::NextFrame);
        assert!(shell.requests().is_empty());
        assert_eq!(shell.repaint(), RepaintRequest::None);
    }

    #[test]
    fn close_and_focus_requests_are_recorded_and_last_focus_wins() {
        let first = WidgetId::from_key("first");
        let second = WidgetId::from_key("second");
        let mut shell = ShellCtx::new(ScaleFactor::ONE);
        shell.request_close();
        shell.request_widget_focus(first);
        shell.request_widget_focus(second);

        assert!(shell.close_requested());
        assert_eq!(
            shell.focus_request(),
            Some(&ShellFocusRequest::Focus(second))
        );
        assert_eq!(
            shell.take_focus_request(),
            Some(ShellFocusRequest::Focus(second))
        );
        assert_eq!(shell.take_focus_request(), None);

        shell.clear_widget_focus();
        assert_eq!(shell.take_focus_request(), Some(ShellFocusRequest::Clear));
    }
}
