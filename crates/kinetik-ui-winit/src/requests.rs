use std::time::Duration;

use kinetik_ui_core::{CursorShape, FrameOutput, PlatformRequest, Rect, RepaintRequest, WidgetId};
use winit::dpi::{LogicalPosition, LogicalSize};
use winit::window::{CursorIcon, Window};

use crate::conversions::cursor_to_winit;
use crate::utils::sanitize_rect_for_platform;
/// Window operations used by the platform request applier.
pub trait WinitWindowOps {
    /// Requests a redraw from the host window.
    fn request_redraw(&mut self);
    /// Applies a cursor icon to the host window.
    fn set_cursor(&mut self, cursor: CursorIcon);
    /// Applies the host window title.
    fn set_title(&mut self, title: &str);
    /// Enables or disables IME/text input.
    fn set_ime_allowed(&mut self, allowed: bool);
    /// Sets the IME composition/caret area in logical window coordinates.
    fn set_ime_cursor_area(&mut self, rect: Rect);
}

impl WinitWindowOps for &Window {
    fn request_redraw(&mut self) {
        Window::request_redraw(self);
    }

    fn set_cursor(&mut self, cursor: CursorIcon) {
        Window::set_cursor(self, cursor);
    }

    fn set_title(&mut self, title: &str) {
        Window::set_title(self, title);
    }

    fn set_ime_allowed(&mut self, allowed: bool) {
        Window::set_ime_allowed(self, allowed);
    }

    fn set_ime_cursor_area(&mut self, rect: Rect) {
        let rect = sanitize_rect_for_platform(rect);
        Window::set_ime_cursor_area(
            self,
            LogicalPosition::new(f64::from(rect.x), f64::from(rect.y)),
            LogicalSize::new(f64::from(rect.width), f64::from(rect.height)),
        );
    }
}

/// Requests that remain application/shell responsibilities after applying
/// window-local effects.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WinitShellRequests {
    /// Text to write to the platform clipboard.
    pub clipboard_text: Option<String>,
    /// Text-input widget that should receive clipboard text read by the shell.
    pub request_clipboard_text: Option<WidgetId>,
    /// URLs the shell should open.
    pub open_urls: Vec<String>,
    /// Delay after which the shell should request another redraw.
    pub repaint_after: Option<Duration>,
    /// Whether the shell should keep requesting redraws.
    pub continuous_repaint: bool,
}

/// Text input request translated for a winit application shell.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WinitTextInputRequest {
    /// Start text input/IME at an optional logical rectangle.
    Start {
        /// Logical text-editing rectangle.
        rect: Option<Rect>,
    },
    /// Stop text input/IME.
    Stop,
}

/// Platform requests emitted by the adapter boundary.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct WinitPlatformRequests {
    /// Cursor shape to apply to the window.
    pub cursor: CursorShape,
    /// Redraw scheduling request.
    pub repaint: RepaintRequest,
    /// Text to write to the platform clipboard.
    pub clipboard_text: Option<String>,
    /// Text-input widget that should receive clipboard text read by the shell.
    pub request_clipboard_text: Option<WidgetId>,
    /// Text input/IME request.
    pub text_input: Option<WinitTextInputRequest>,
    /// Window title to apply.
    pub window_title: Option<String>,
    /// URLs the app shell should open.
    pub open_urls: Vec<String>,
}

impl WinitPlatformRequests {
    /// Translates core frame output into winit-facing platform requests.
    #[must_use]
    pub fn from_frame_output(output: &FrameOutput) -> Self {
        let mut requests = Self::default();
        requests.apply_frame_output(output);
        requests
    }

    /// Applies all requests from a core frame output.
    pub fn apply_frame_output(&mut self, output: &FrameOutput) {
        self.request_repaint(output.repaint);
        for request in &output.platform_requests {
            self.apply_platform_request(request);
        }
    }

    /// Updates the repaint request using core repaint priority rules.
    pub fn request_repaint(&mut self, repaint: RepaintRequest) {
        self.repaint = self.repaint.merge(repaint);
    }

    /// Applies one platform request.
    pub fn apply_platform_request(&mut self, request: &PlatformRequest) {
        match request {
            PlatformRequest::SetCursor(cursor) => {
                self.cursor = *cursor;
            }
            PlatformRequest::CopyToClipboard(text) => {
                self.clipboard_text = Some(text.clone());
            }
            PlatformRequest::RequestClipboardText { target } => {
                self.request_clipboard_text = Some(*target);
            }
            PlatformRequest::StartTextInput { rect } => {
                self.text_input = Some(WinitTextInputRequest::Start { rect: *rect });
            }
            PlatformRequest::StopTextInput => {
                self.text_input = Some(WinitTextInputRequest::Stop);
            }
            PlatformRequest::SetWindowTitle(title) => {
                self.window_title = Some(title.clone());
            }
            PlatformRequest::OpenUrl(url) => {
                self.open_urls.push(url.clone());
            }
        }
    }

    /// Applies window-local requests to a real winit window and returns shell work.
    #[must_use]
    pub fn apply_to_window(&self, window: &Window) -> WinitShellRequests {
        let mut window = window;
        self.apply_to_window_ops(&mut window)
    }

    /// Applies window-local requests to a target and returns shell work.
    #[must_use]
    pub fn apply_to_window_ops(&self, window: &mut impl WinitWindowOps) -> WinitShellRequests {
        window.set_cursor(cursor_to_winit(self.cursor));
        if let Some(title) = &self.window_title {
            window.set_title(title);
        }

        let mut shell = WinitShellRequests {
            clipboard_text: self.clipboard_text.clone(),
            request_clipboard_text: self.request_clipboard_text,
            open_urls: self.open_urls.clone(),
            ..WinitShellRequests::default()
        };

        match self.repaint {
            RepaintRequest::None => {}
            RepaintRequest::NextFrame => window.request_redraw(),
            RepaintRequest::After(delay) => {
                shell.repaint_after = Some(delay);
            }
            RepaintRequest::Continuous => {
                shell.continuous_repaint = true;
                window.request_redraw();
            }
        }

        match self.text_input {
            Some(WinitTextInputRequest::Start { rect }) => {
                window.set_ime_allowed(true);
                if let Some(rect) = rect {
                    window.set_ime_cursor_area(sanitize_rect_for_platform(rect));
                }
            }
            Some(WinitTextInputRequest::Stop) => {
                window.set_ime_allowed(false);
            }
            None => {}
        }

        shell
    }
}
