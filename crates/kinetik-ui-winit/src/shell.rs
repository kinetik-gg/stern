use core::fmt;

use kinetik_ui_core::{ClipboardText, RepaintRequest, WidgetId};

/// One ordered application-shell operation.
#[derive(PartialEq, Eq)]
pub enum WinitShellRequest {
    /// Write text, including an empty string, to the clipboard.
    CopyToClipboard(String),
    /// Read clipboard text for one requesting widget.
    RequestClipboardText {
        /// Widget that owns the pending paste request.
        target: WidgetId,
    },
    /// Open a validated HTTP or HTTPS URL.
    OpenUrl(String),
}

impl fmt::Debug for WinitShellRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CopyToClipboard(text) => formatter
                .debug_struct("CopyToClipboard")
                .field("bytes", &text.len())
                .finish(),
            Self::RequestClipboardText { target } => formatter
                .debug_struct("RequestClipboardText")
                .field("target", target)
                .finish(),
            Self::OpenUrl(url) => formatter
                .debug_struct("OpenUrl")
                .field("scheme", &redacted_url_scheme(url))
                .finish(),
        }
    }
}

/// Owned ordered shell work returned by window request application.
///
/// The batch is intentionally non-cloneable and is consumed by [`Self::execute`].
#[derive(Debug, Default, PartialEq, Eq)]
pub struct WinitShellRequests {
    /// Operations in original frame order.
    operations: Vec<WinitShellRequest>,
}

impl WinitShellRequests {
    /// Creates an owned batch from ordered operations.
    #[must_use]
    pub fn from_operations(operations: impl IntoIterator<Item = WinitShellRequest>) -> Self {
        Self {
            operations: operations.into_iter().collect(),
        }
    }

    /// Returns ordered operations without consuming the batch.
    #[must_use]
    pub fn operations(&self) -> &[WinitShellRequest] {
        &self.operations
    }

    /// Returns whether the batch contains no shell work.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.operations.is_empty()
    }

    pub(crate) fn push(&mut self, request: WinitShellRequest) {
        self.operations.push(request);
    }

    /// Executes the batch once, continuing after individual failures.
    #[must_use]
    pub fn execute(self, services: &mut dyn WinitShellServices) -> WinitShellOutcome {
        let mut outcome = WinitShellOutcome::default();
        for request in self.operations {
            match request {
                WinitShellRequest::CopyToClipboard(text) => {
                    if let Err(error) = services.write_clipboard_text(&text) {
                        outcome
                            .results
                            .push(WinitShellResult::Failure(WinitShellFailure::new(
                                WinitShellOperation::ClipboardWrite,
                                None,
                                error.into(),
                            )));
                    }
                }
                WinitShellRequest::RequestClipboardText { target } => {
                    match services.read_clipboard_text() {
                        Ok(text) => outcome.results.push(WinitShellResult::ClipboardText(
                            ClipboardText::new(target, text),
                        )),
                        Err(error) => {
                            outcome.results.push(WinitShellResult::Failure(
                                WinitShellFailure::new(
                                    WinitShellOperation::ClipboardRead,
                                    Some(target),
                                    error.into(),
                                ),
                            ));
                        }
                    }
                }
                WinitShellRequest::OpenUrl(url) => {
                    if !is_supported_web_url(&url) {
                        outcome
                            .results
                            .push(WinitShellResult::Failure(WinitShellFailure::new(
                                WinitShellOperation::OpenUrl,
                                None,
                                WinitShellFailureReason::UnsupportedUrlScheme,
                            )));
                    } else if let Err(error) = services.open_http_url(&url) {
                        outcome
                            .results
                            .push(WinitShellResult::Failure(WinitShellFailure::new(
                                WinitShellOperation::OpenUrl,
                                None,
                                error.into(),
                            )));
                    }
                }
            }
        }
        outcome
    }
}

/// Injectable operating-system services used by the Winit shell executor.
pub trait WinitShellServices {
    /// Writes clipboard text.
    ///
    /// # Errors
    ///
    /// Returns a safe structured service error without including the text.
    fn write_clipboard_text(&mut self, text: &str) -> Result<(), WinitShellServiceError>;

    /// Reads clipboard text.
    ///
    /// # Errors
    ///
    /// Returns a safe structured service error when the clipboard is unavailable.
    fn read_clipboard_text(&mut self) -> Result<String, WinitShellServiceError>;

    /// Opens a prevalidated HTTP or HTTPS URL.
    ///
    /// # Errors
    ///
    /// Returns a safe structured service error without including the URL.
    fn open_http_url(&mut self, url: &str) -> Result<(), WinitShellServiceError>;
}

/// Safe failure reported by an injected shell service.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WinitShellServiceError {
    /// The service could not be initialized or is unavailable.
    Unavailable,
    /// The service was available but the requested operation failed.
    Failed,
}

impl fmt::Display for WinitShellServiceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Unavailable => "service unavailable",
            Self::Failed => "operation failed",
        })
    }
}

impl std::error::Error for WinitShellServiceError {}

/// Kind of shell operation associated with a failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WinitShellOperation {
    /// Clipboard write.
    ClipboardWrite,
    /// Clipboard read.
    ClipboardRead,
    /// Browser URL open.
    OpenUrl,
}

/// Redacted reason for a shell operation failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WinitShellFailureReason {
    /// The service is unavailable.
    ServiceUnavailable,
    /// The platform operation failed.
    OperationFailed,
    /// The requested URL was not an HTTP or HTTPS URL.
    UnsupportedUrlScheme,
}

impl From<WinitShellServiceError> for WinitShellFailureReason {
    fn from(error: WinitShellServiceError) -> Self {
        match error {
            WinitShellServiceError::Unavailable => Self::ServiceUnavailable,
            WinitShellServiceError::Failed => Self::OperationFailed,
        }
    }
}

/// Structured, redacted failure from one shell operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WinitShellFailure {
    /// Failed operation kind.
    pub operation: WinitShellOperation,
    /// Clipboard-read target, when applicable.
    pub target: Option<WidgetId>,
    /// Redacted failure reason.
    pub reason: WinitShellFailureReason,
}

impl WinitShellFailure {
    const fn new(
        operation: WinitShellOperation,
        target: Option<WidgetId>,
        reason: WinitShellFailureReason,
    ) -> Self {
        Self {
            operation,
            target,
            reason,
        }
    }
}

impl fmt::Display for WinitShellFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "shell operation {:?} failed: {:?}",
            self.operation, self.reason
        )
    }
}

impl std::error::Error for WinitShellFailure {}

/// Ordered result produced by a shell batch.
#[derive(PartialEq, Eq)]
pub enum WinitShellResult {
    /// Targeted clipboard response for the next input snapshot.
    ClipboardText(ClipboardText),
    /// Redacted operation failure.
    Failure(WinitShellFailure),
}

impl fmt::Debug for WinitShellResult {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ClipboardText(clipboard) => formatter
                .debug_struct("ClipboardText")
                .field("target", &clipboard.target)
                .field("bytes", &clipboard.text.len())
                .finish(),
            Self::Failure(failure) => formatter.debug_tuple("Failure").field(failure).finish(),
        }
    }
}

/// Ordered results from one consumed shell batch.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct WinitShellOutcome {
    /// Responses and failures in operation order.
    results: Vec<WinitShellResult>,
}

impl WinitShellOutcome {
    /// Creates an outcome from ordered responses and failures.
    #[must_use]
    pub fn from_results(results: impl IntoIterator<Item = WinitShellResult>) -> Self {
        Self {
            results: results.into_iter().collect(),
        }
    }

    /// Returns ordered responses and failures.
    #[must_use]
    pub fn results(&self) -> &[WinitShellResult] {
        &self.results
    }

    /// Returns whether the outcome contains input that requires another frame.
    #[must_use]
    pub fn has_input_response(&self) -> bool {
        self.results
            .iter()
            .any(|result| matches!(result, WinitShellResult::ClipboardText(_)))
    }

    /// Returns the repaint request implied by targeted input responses.
    #[must_use]
    pub fn repaint_request(&self) -> RepaintRequest {
        if self.has_input_response() {
            RepaintRequest::NextFrame
        } else {
            RepaintRequest::None
        }
    }

    /// Consumes the outcome and returns its ordered results.
    #[must_use]
    pub fn into_results(self) -> Vec<WinitShellResult> {
        self.results
    }
}

/// Native long-lived clipboard and browser services.
pub struct NativeWinitShellServices {
    clipboard: Option<arboard::Clipboard>,
}

impl NativeWinitShellServices {
    /// Initializes native services. Clipboard initialization failure is retained
    /// as an unavailable service so browser work can still continue.
    #[must_use]
    pub fn new() -> Self {
        Self {
            clipboard: arboard::Clipboard::new().ok(),
        }
    }
}

impl Default for NativeWinitShellServices {
    fn default() -> Self {
        Self::new()
    }
}

impl WinitShellServices for NativeWinitShellServices {
    fn write_clipboard_text(&mut self, text: &str) -> Result<(), WinitShellServiceError> {
        let clipboard = self
            .clipboard
            .as_mut()
            .ok_or(WinitShellServiceError::Unavailable)?;
        clipboard
            .set_text(text.to_owned())
            .map_err(|_| WinitShellServiceError::Failed)
    }

    fn read_clipboard_text(&mut self) -> Result<String, WinitShellServiceError> {
        let clipboard = self
            .clipboard
            .as_mut()
            .ok_or(WinitShellServiceError::Unavailable)?;
        clipboard
            .get_text()
            .map_err(|_| WinitShellServiceError::Failed)
    }

    fn open_http_url(&mut self, url: &str) -> Result<(), WinitShellServiceError> {
        webbrowser::open(url).map_err(|_| WinitShellServiceError::Failed)
    }
}

fn redacted_url_scheme(url: &str) -> &'static str {
    let Some((scheme, _)) = url.split_once(':') else {
        return "missing";
    };
    if scheme.eq_ignore_ascii_case("https") {
        "https"
    } else if scheme.eq_ignore_ascii_case("http") {
        "http"
    } else {
        "unsupported"
    }
}

fn is_supported_web_url(url: &str) -> bool {
    if url.trim() != url || url.chars().any(char::is_control) {
        return false;
    }
    let Some((raw_scheme, raw_authority_and_path)) = url.split_once("://") else {
        return false;
    };
    if (!raw_scheme.eq_ignore_ascii_case("http") && !raw_scheme.eq_ignore_ascii_case("https"))
        || raw_authority_and_path.starts_with('/')
    {
        return false;
    }
    let Ok(parsed) = url::Url::parse(url) else {
        return false;
    };
    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return false;
    }
    parsed.host_str().is_some_and(|host| !host.is_empty())
}

#[cfg(test)]
mod tests {
    use super::is_supported_web_url;

    #[test]
    fn web_url_validation_allows_only_http_and_https_with_authority() {
        assert!(is_supported_web_url("https://example.com/docs?q=1#topic"));
        assert!(is_supported_web_url("HTTP://example.com"));
        assert!(!is_supported_web_url("file:///tmp/example"));
        assert!(!is_supported_web_url("javascript:alert(1)"));
        assert!(!is_supported_web_url("https://"));
        assert!(!is_supported_web_url(" https://example.com"));
        assert!(!is_supported_web_url("https://   /private"));
        assert!(!is_supported_web_url("https://@/path"));
        assert!(!is_supported_web_url("https:///missing-host"));
        assert!(!is_supported_web_url("https://example.com\nprivate"));
    }
}
