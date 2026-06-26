//! Windowless status bar conformance for reusable editor chrome contracts.

use kinetik_ui_widgets::{
    DiagnosticSource, DiagnosticStrip, DiagnosticStripItem, DiagnosticStripItemId,
    DiagnosticStripSeverity, StatusBar, StatusItem, StatusItemId, StatusItemKind, StatusProgress,
};

fn status_id(raw: u64) -> StatusItemId {
    StatusItemId::from_raw(raw)
}

fn diagnostic_id(raw: u64) -> DiagnosticStripItemId {
    DiagnosticStripItemId::from_raw(raw)
}

fn assert_close(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < f32::EPSILON,
        "expected {actual} to equal {expected}"
    );
}

#[test]
fn status_bar_visible_items_preserve_order_and_filter_hidden_items() {
    let status_bar = StatusBar::from_items([
        StatusItem::new(
            status_id(1),
            "Ready",
            "Viewport ready",
            StatusItemKind::Ready,
        ),
        StatusItem::new(
            status_id(2),
            "Hidden",
            "Internal state",
            StatusItemKind::Message,
        )
        .with_visible(false),
        StatusItem::new(
            status_id(3),
            "Queued",
            "3 jobs queued",
            StatusItemKind::JobCount,
        )
        .with_count(3),
    ]);

    let visible = status_bar.visible_items();

    assert_eq!(visible.len(), 2);
    assert_eq!(visible[0].id, status_id(1));
    assert_eq!(visible[0].label, "Ready");
    assert_eq!(visible[1].id, status_id(3));
    assert_eq!(visible[1].count, Some(3));
    assert_eq!(
        status_bar.item(status_id(2)).map(|item| item.visible),
        Some(false)
    );
}

#[test]
fn status_bar_progress_values_sanitize_and_clamp_deterministically() {
    assert_close(StatusProgress::new(f32::NAN).value, 0.0);
    assert_close(StatusProgress::new(f32::INFINITY).value, 0.0);
    assert_close(StatusProgress::new(-0.25).value, 0.0);
    assert_close(StatusProgress::new(1.25).value, 1.0);
    assert_close(StatusProgress::from_fraction(5.0, 10.0).value, 0.5);
    assert_close(StatusProgress::from_fraction(5.0, 0.0).value, 0.0);

    let item = StatusItem::new(
        status_id(4),
        "Render",
        "Rendering preview",
        StatusItemKind::Progress,
    )
    .with_progress_value(1.8);

    assert_close(item.progress.expect("progress metadata").value, 1.0);
}

#[test]
fn status_bar_represents_ready_pending_stale_and_error_as_typed_metadata() {
    let status_bar = StatusBar::from_items([
        StatusItem::new(status_id(1), "Ready", "Ready", StatusItemKind::Ready),
        StatusItem::new(status_id(2), "Pending", "Loading", StatusItemKind::Pending),
        StatusItem::new(status_id(3), "Stale", "Out of date", StatusItemKind::Stale),
        StatusItem::new(status_id(4), "Error", "Failed", StatusItemKind::Error),
    ]);

    let kinds = status_bar
        .items()
        .iter()
        .map(|item| item.kind)
        .collect::<Vec<_>>();

    assert_eq!(
        kinds,
        vec![
            StatusItemKind::Ready,
            StatusItemKind::Pending,
            StatusItemKind::Stale,
            StatusItemKind::Error,
        ]
    );
}

#[test]
fn status_bar_diagnostics_strip_orders_by_severity_and_preserves_insertion_order_within_severity() {
    let strip = DiagnosticStrip::from_items([
        DiagnosticStripItem::new(
            diagnostic_id(1),
            DiagnosticStripSeverity::Warning,
            "KUI-WARN-A",
            "First warning",
        ),
        DiagnosticStripItem::new(
            diagnostic_id(2),
            DiagnosticStripSeverity::Info,
            "KUI-INFO",
            "Informational note",
        ),
        DiagnosticStripItem::new(
            diagnostic_id(3),
            DiagnosticStripSeverity::Error,
            "KUI-ERR",
            "Error",
        )
        .with_source(DiagnosticSource::Renderer)
        .with_field("texture", "missing"),
        DiagnosticStripItem::new(
            diagnostic_id(4),
            DiagnosticStripSeverity::Warning,
            "KUI-WARN-B",
            "Second warning",
        ),
    ]);

    let ordered = strip.ordered_items();

    assert_eq!(
        ordered.iter().map(|item| item.id).collect::<Vec<_>>(),
        vec![
            diagnostic_id(3),
            diagnostic_id(1),
            diagnostic_id(4),
            diagnostic_id(2),
        ]
    );
    assert_eq!(ordered[0].source, Some(DiagnosticSource::Renderer));
    assert_eq!(ordered[0].fields[0].name, "texture");
}

#[test]
fn status_bar_diagnostics_strip_summary_counts_are_deterministic_for_empty_and_mixed_input() {
    assert_eq!(DiagnosticStrip::new().summary().total(), 0);

    let strip = DiagnosticStrip::from_items([
        DiagnosticStripItem::new(
            diagnostic_id(1),
            DiagnosticStripSeverity::Warning,
            "KUI-WARN-A",
            "First warning",
        ),
        DiagnosticStripItem::new(
            diagnostic_id(2),
            DiagnosticStripSeverity::Error,
            "KUI-ERR-A",
            "First error",
        ),
        DiagnosticStripItem::new(
            diagnostic_id(3),
            DiagnosticStripSeverity::Error,
            "KUI-ERR-B",
            "Second error",
        ),
        DiagnosticStripItem::new(
            diagnostic_id(4),
            DiagnosticStripSeverity::Info,
            "KUI-INFO",
            "Info",
        ),
    ]);
    let summary = strip.summary();

    assert_eq!(summary.errors, 2);
    assert_eq!(summary.warnings, 1);
    assert_eq!(summary.info, 1);
    assert_eq!(summary.total(), 4);
}
