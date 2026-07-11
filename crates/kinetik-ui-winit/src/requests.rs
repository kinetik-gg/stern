use core::fmt;

use kinetik_ui_core::{CursorShape, FrameOutput, PlatformRequest, Rect, RepaintRequest};
use winit::dpi::{LogicalPosition, LogicalSize};
use winit::window::{CursorIcon, Window};

use crate::conversions::cursor_to_winit;
use crate::shell::{WinitShellRequest, WinitShellRequests};
use crate::utils::sanitize_rect_for_platform;

/// Window operations used by the consumed platform-request batch.
pub trait WinitWindowOps {
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

/// Text-input operation translated for a Winit application shell.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WinitTextInputRequest {
    /// Start text input/IME at an optional logical rectangle.
    Start {
        /// Logical text-editing rectangle.
        rect: Option<Rect>,
    },
    /// Move the current IME candidate area without restarting text input.
    UpdateRect {
        /// Logical text-editing rectangle.
        rect: Rect,
    },
    /// Stop text input/IME.
    Stop,
}

/// Owned result of applying one consumed platform batch to a window.
#[derive(Debug, Default, PartialEq)]
pub struct WinitAppliedRequests {
    /// Ordered application-shell work that remains after window-local effects.
    shell: WinitShellRequests,
    /// The frame's sole repaint request.
    repaint: RepaintRequest,
}

impl WinitAppliedRequests {
    /// Returns the authoritative repaint request without consuming shell work.
    #[must_use]
    pub const fn repaint(&self) -> RepaintRequest {
        self.repaint
    }

    /// Returns the ordered shell operations without consuming them.
    #[must_use]
    pub fn shell(&self) -> &WinitShellRequests {
        &self.shell
    }

    /// Consumes the result into ordered shell work and repaint intent.
    #[must_use]
    pub fn into_parts(self) -> (WinitShellRequests, RepaintRequest) {
        (self.shell, self.repaint)
    }
}

/// Owned one-frame platform request batch.
///
/// This type deliberately does not implement [`Clone`]. Applying it consumes
/// the batch, which prevents clipboard, URL, title, or IME work from being
/// replayed after the frame that emitted it.
#[derive(Default, PartialEq)]
pub struct WinitPlatformRequests {
    /// Final cursor shape for this frame. The default actively resets a stale
    /// cursor when no widget requested one.
    cursor: CursorShape,
    /// Sole repaint request for this frame.
    repaint: RepaintRequest,
    /// Ordered text-input/IME operations.
    text_input: Vec<WinitTextInputRequest>,
    /// Final window title request, when present.
    window_title: Option<String>,
    /// Ordered application-shell operations.
    shell: WinitShellRequests,
}

impl fmt::Debug for WinitPlatformRequests {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("WinitPlatformRequests")
            .field("cursor", &self.cursor)
            .field("repaint", &self.repaint)
            .field("text_input", &self.text_input)
            .field(
                "window_title_bytes",
                &self.window_title.as_ref().map(String::len),
            )
            .field("shell", &self.shell)
            .finish()
    }
}

impl WinitPlatformRequests {
    /// Translates core frame output into a fresh one-frame batch.
    #[must_use]
    pub fn from_frame_output(output: &FrameOutput) -> Self {
        let mut requests = Self::default();
        requests.replace_frame_output(output);
        requests
    }

    /// Replaces this batch from one frame output without retaining old work.
    pub fn replace_frame_output(&mut self, output: &FrameOutput) {
        *self = Self::default();
        self.repaint = output.repaint;
        for request in &output.platform_requests {
            self.push_platform_request(request);
        }
    }

    /// Returns the final cursor for this frame.
    #[must_use]
    pub const fn cursor(&self) -> CursorShape {
        self.cursor
    }

    /// Returns the authoritative repaint request for this frame.
    #[must_use]
    pub const fn repaint(&self) -> RepaintRequest {
        self.repaint
    }

    /// Returns ordered text-input operations.
    #[must_use]
    pub fn text_input(&self) -> &[WinitTextInputRequest] {
        &self.text_input
    }

    /// Returns the final title request, when present.
    #[must_use]
    pub fn window_title(&self) -> Option<&str> {
        self.window_title.as_deref()
    }

    /// Returns ordered shell operations.
    #[must_use]
    pub fn shell(&self) -> &WinitShellRequests {
        &self.shell
    }

    fn push_platform_request(&mut self, request: &PlatformRequest) {
        match request {
            PlatformRequest::SetCursor(cursor) => self.cursor = *cursor,
            PlatformRequest::CopyToClipboard(text) => self
                .shell
                .push(WinitShellRequest::CopyToClipboard(text.clone())),
            PlatformRequest::RequestClipboardText { target } => self
                .shell
                .push(WinitShellRequest::RequestClipboardText { target: *target }),
            PlatformRequest::StartTextInput { rect } => {
                self.text_input
                    .push(WinitTextInputRequest::Start { rect: *rect });
            }
            PlatformRequest::UpdateTextInputRect { rect } => {
                self.text_input
                    .push(WinitTextInputRequest::UpdateRect { rect: *rect });
            }
            PlatformRequest::StopTextInput => self.text_input.push(WinitTextInputRequest::Stop),
            PlatformRequest::SetWindowTitle(title) => self.window_title = Some(title.clone()),
            PlatformRequest::OpenUrl(url) => {
                self.shell.push(WinitShellRequest::OpenUrl(url.clone()));
            }
        }
    }

    /// Consumes and applies this batch to a real Winit window.
    #[must_use]
    pub fn apply_to_window(self, window: &Window) -> WinitAppliedRequests {
        let mut window = window;
        self.apply_to_window_ops(&mut window)
    }

    /// Consumes and applies this batch to an injectable window target.
    #[must_use]
    pub fn apply_to_window_ops(self, window: &mut impl WinitWindowOps) -> WinitAppliedRequests {
        window.set_cursor(cursor_to_winit(self.cursor));
        if let Some(title) = &self.window_title {
            window.set_title(title);
        }

        for request in self.text_input {
            match request {
                WinitTextInputRequest::Start { rect } => {
                    window.set_ime_allowed(true);
                    if let Some(rect) = rect {
                        window.set_ime_cursor_area(sanitize_rect_for_platform(rect));
                    }
                }
                WinitTextInputRequest::UpdateRect { rect } => {
                    window.set_ime_cursor_area(sanitize_rect_for_platform(rect));
                }
                WinitTextInputRequest::Stop => window.set_ime_allowed(false),
            }
        }

        WinitAppliedRequests {
            shell: self.shell,
            repaint: self.repaint,
        }
    }
}
