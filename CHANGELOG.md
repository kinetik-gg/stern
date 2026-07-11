# Changelog

All notable changes to Kinetik UI will be documented in this file. Categories
follow the repository release policy.

## [Unreleased]

`0.1.0-alpha.1` is the planned first prerelease. It has not been dated, tagged,
published, or accepted as an alpha release.

### Changed

- Prepared package metadata and dependency constraints for the planned
  prerelease archives.
- Made Winit platform batches owned and consuming, with ordered shell services,
  same-owner IME rectangle updates, and stateful repaint replacement. This is a
  provisional breaking API change: callers must use `WinitPlatformRequests`
  getters and consume `apply_to_window`/`apply_to_window_ops`, then split the
  returned `WinitAppliedRequests` with `into_parts`.
- Added `PlatformRequest::UpdateTextInputRect`; exhaustive matches over the
  provisional public enum must handle the new same-owner IME geometry request.
- Added native text clipboard and hardened HTTP/HTTPS browser services to the
  Winit adapter; image clipboard features remain disabled.
- Normalized ordered line-wheel events to a fixed 40-logical-unit step while
  preserving exact logical pixel deltas and the legacy empty-stream magnitude.
  Added timestamped Winit click sequencing; the explicit-count method remains
  available and resets automatic history.

### Documentation

- Distinguished current source-path use from future registry installation.

### Internal

- Recorded dependency-aware package verification and the seven-crate publish
  order.
