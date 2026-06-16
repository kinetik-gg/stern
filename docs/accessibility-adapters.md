# Accessibility Adapter Boundary

Kinetik UI exports accessibility data as a validated semantic snapshot. The
snapshot is backend-neutral: it contains roles, labels, descriptions, logical
bounds, state, values, actions, parent links, child order, focus order, and the
currently focused widget when that widget is focusable.

Render primitives stay out of this path. Painters consume `Primitive` values;
accessibility adapters consume `AccessibilitySnapshot`.

## Core Flow

Applications build semantic nodes during a frame and export a snapshot after the
frame is finalized:

```rust
use kinetik_ui_core::{
    AccessibilityAdapter, FrameOutput, SemanticTreeError, UiMemory,
};

enum SyncAccessibilityError<E> {
    InvalidTree(SemanticTreeError),
    Adapter(E),
}

fn sync_accessibility<A: AccessibilityAdapter>(
    output: &FrameOutput,
    memory: &UiMemory,
    adapter: &mut A,
) -> Result<(), SyncAccessibilityError<A::Error>> {
    let snapshot = output
        .accessibility_snapshot(memory.focused())
        .map_err(SyncAccessibilityError::InvalidTree)?;

    adapter
        .synchronize(&snapshot)
        .map_err(SyncAccessibilityError::Adapter)
}
```

Invalid semantic trees remain core runtime diagnostics. `Ui::end_frame()` adds a
`FrameWarning::InvalidSemanticTree` warning, and snapshot export returns the same
`SemanticTreeError` instead of silently dropping bad data.

## Winit Boundary

`kinetik-ui-winit` provides `WinitAccessibilityUpdate` as the first platform
handoff type:

```rust
use kinetik_ui_winit::WinitAccessibilityUpdate;

let update = WinitAccessibilityUpdate::from_frame_output(&output, memory.focused())?;
```

The update is still OS-service-free. A Windows, macOS, Linux, or test adapter can
translate `update.snapshot` into native accessibility objects without pulling OS
accessibility crates into `kinetik-ui-core`.

## Adapter Rules

- Build snapshots from validated `SemanticTree` data.
- Preserve semantic traversal order and focus order.
- Keep accessibility data independent from painting and renderer resources.
- Keep OS integration in platform/application adapters.
- Route platform-requested actions through semantic action invocations; the
  application still owns command execution.
