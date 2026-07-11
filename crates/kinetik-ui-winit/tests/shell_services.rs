//! Conformance tests for ordered, injectable Winit shell services.

use std::collections::VecDeque;

use kinetik_ui_core::{ClipboardText, RepaintRequest, UiInputEvent, WidgetId};
use kinetik_ui_winit::{
    WinitInputAdapter, WinitShellFailure, WinitShellFailureReason, WinitShellOperation,
    WinitShellRequest, WinitShellRequests, WinitShellResult, WinitShellServiceError,
    WinitShellServices,
};

#[derive(Debug, PartialEq, Eq)]
enum Call {
    Write(String),
    Read,
    Open(String),
}

#[derive(Default)]
struct FakeServices {
    calls: Vec<Call>,
    writes: VecDeque<Result<(), WinitShellServiceError>>,
    reads: VecDeque<Result<String, WinitShellServiceError>>,
    opens: VecDeque<Result<(), WinitShellServiceError>>,
}

impl WinitShellServices for FakeServices {
    fn write_clipboard_text(&mut self, text: &str) -> Result<(), WinitShellServiceError> {
        self.calls.push(Call::Write(text.to_owned()));
        self.writes.pop_front().unwrap_or(Ok(()))
    }

    fn read_clipboard_text(&mut self) -> Result<String, WinitShellServiceError> {
        self.calls.push(Call::Read);
        self.reads.pop_front().unwrap_or_else(|| Ok(String::new()))
    }

    fn open_http_url(&mut self, url: &str) -> Result<(), WinitShellServiceError> {
        self.calls.push(Call::Open(url.to_owned()));
        self.opens.pop_front().unwrap_or(Ok(()))
    }
}

#[test]
fn ordered_shell_operations_preserve_alternating_writes_reads_and_urls() {
    let first = WidgetId::from_key("first");
    let second = WidgetId::from_key("second");
    let requests = WinitShellRequests::from_operations([
        WinitShellRequest::CopyToClipboard("one".to_owned()),
        WinitShellRequest::RequestClipboardText { target: first },
        WinitShellRequest::CopyToClipboard(String::new()),
        WinitShellRequest::RequestClipboardText { target: second },
        WinitShellRequest::OpenUrl("https://example.com/one".to_owned()),
        WinitShellRequest::OpenUrl("http://example.com/two".to_owned()),
    ]);
    let mut services = FakeServices {
        reads: VecDeque::from([Ok("alpha".to_owned()), Ok("beta".to_owned())]),
        ..FakeServices::default()
    };

    let outcome = requests.execute(&mut services);
    let outcome_debug = format!("{outcome:?}");

    assert_eq!(
        services.calls,
        vec![
            Call::Write("one".to_owned()),
            Call::Read,
            Call::Write(String::new()),
            Call::Read,
            Call::Open("https://example.com/one".to_owned()),
            Call::Open("http://example.com/two".to_owned()),
        ]
    );
    assert!(!outcome_debug.contains("alpha"));
    assert!(!outcome_debug.contains("beta"));
    assert_eq!(
        outcome.results(),
        &[
            WinitShellResult::ClipboardText(ClipboardText::new(first, "alpha")),
            WinitShellResult::ClipboardText(ClipboardText::new(second, "beta")),
        ]
    );
}

#[test]
fn shell_failures_are_ordered_redacted_and_do_not_stop_later_operations() {
    let target = WidgetId::from_key("field");
    let requests = WinitShellRequests::from_operations([
        WinitShellRequest::CopyToClipboard("private clipboard payload".to_owned()),
        WinitShellRequest::RequestClipboardText { target },
        WinitShellRequest::OpenUrl("file:///private/path".to_owned()),
        WinitShellRequest::OpenUrl(
            "https://example.com/docs?secret=token#private-fragment".to_owned(),
        ),
        WinitShellRequest::CopyToClipboard("continued".to_owned()),
    ]);
    let request_debug = format!("{requests:?}");
    assert!(!request_debug.contains("payload"));
    assert!(!request_debug.contains("secret"));
    assert!(!request_debug.contains("private-fragment"));
    let mut services = FakeServices {
        writes: VecDeque::from([Err(WinitShellServiceError::Failed), Ok(())]),
        reads: VecDeque::from([Err(WinitShellServiceError::Unavailable)]),
        opens: VecDeque::from([Err(WinitShellServiceError::Failed)]),
        ..FakeServices::default()
    };

    let outcome = requests.execute(&mut services);

    assert_eq!(
        services.calls,
        vec![
            Call::Write("private clipboard payload".to_owned()),
            Call::Read,
            Call::Open("https://example.com/docs?secret=token#private-fragment".to_owned()),
            Call::Write("continued".to_owned()),
        ]
    );
    assert_eq!(
        outcome.results(),
        &[
            WinitShellResult::Failure(WinitShellFailure {
                operation: WinitShellOperation::ClipboardWrite,
                target: None,
                reason: WinitShellFailureReason::OperationFailed,
            }),
            WinitShellResult::Failure(WinitShellFailure {
                operation: WinitShellOperation::ClipboardRead,
                target: Some(target),
                reason: WinitShellFailureReason::ServiceUnavailable,
            }),
            WinitShellResult::Failure(WinitShellFailure {
                operation: WinitShellOperation::OpenUrl,
                target: None,
                reason: WinitShellFailureReason::UnsupportedUrlScheme,
            }),
            WinitShellResult::Failure(WinitShellFailure {
                operation: WinitShellOperation::OpenUrl,
                target: None,
                reason: WinitShellFailureReason::OperationFailed,
            }),
        ]
    );
    let diagnostics = outcome
        .results()
        .iter()
        .filter_map(|result| match result {
            WinitShellResult::Failure(failure) => Some(failure.to_string()),
            WinitShellResult::ClipboardText(_) => None,
        })
        .collect::<Vec<_>>()
        .join("\n");
    assert!(!diagnostics.contains("payload"));
    assert!(!diagnostics.contains("secret"));
    assert!(!diagnostics.contains("private-fragment"));
}

#[test]
fn targeted_shell_response_enters_ordered_input_once_and_requests_repaint() {
    let target = WidgetId::from_key("field");
    let requests =
        WinitShellRequests::from_operations([WinitShellRequest::RequestClipboardText { target }]);
    let mut services = FakeServices {
        reads: VecDeque::from([Ok("paste".to_owned())]),
        ..FakeServices::default()
    };
    let outcome = requests.execute(&mut services);
    assert_eq!(outcome.repaint_request(), RepaintRequest::NextFrame);
    let mut adapter = WinitInputAdapter::default();

    adapter.begin_frame();
    let failures = adapter.apply_shell_outcome(outcome);

    assert!(failures.is_empty());
    assert_eq!(
        adapter.input().events,
        vec![UiInputEvent::ClipboardText(ClipboardText::new(
            target, "paste"
        ))]
    );
    adapter.begin_frame();
    assert!(adapter.input().events.is_empty());
}

#[test]
fn empty_shell_batch_has_no_calls_results_or_repaint() {
    let mut services = FakeServices::default();

    let outcome = WinitShellRequests::default().execute(&mut services);

    assert!(services.calls.is_empty());
    assert!(outcome.results().is_empty());
    assert_eq!(outcome.repaint_request(), RepaintRequest::None);
}
