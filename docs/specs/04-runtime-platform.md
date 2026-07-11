# Kinetik UI Specification: Runtime, Platform, Accessibility, And Resources

This file is part of the Kinetik UI architecture specification. The canonical entrypoint is [../specs.md](../specs.md).

Contained sections: 23-28.

## 23. Non-Blocking Work Model

Heavy work must not run inside UI widget calls.

The UI runtime should remain responsive even when domain workloads are active.

Examples of heavy work:

- Video decoding.
- Image sequence loading.
- AI inference.
- Export.
- Preview rendering.
- 3D rendering.
- File scanning.
- Thumbnail generation.
- Large project loading.

Recommended model:

```text
UI thread
  owns window events, UI state, frame building, and presentation coordination

Worker threads/tasks
  own long-running CPU/domain work

Render jobs
  own GPU/domain rendering where appropriate

Message/resource queues
  move progress, results, errors, cancellation, and texture/resource handles
  back to the application/UI coordinator
```

The toolkit should support UI patterns for:

- Progress.
- Cancellation.
- Pending state.
- Ready/stale state.
- Error state.
- Disabled actions while work is unavailable.
- Non-blocking status messages.

The UI must not wait synchronously for heavy processing to complete.

External results delivered back to the UI coordinator use durable liveness
tokens. Marking an async owner present in each frame does not renew its token;
the token remains valid for the continuously active incarnation. Applications
explicitly restart an owner when same-ID work is replaced, cancel the exact
token when delivery should stop, or remove the owner when it is no longer
active. Widget-registration presence is not async-owner authority.

The coordinator, not a worker, owns validation and mutation. Applying a valid
token invokes that call's mutation once; this contract does not deduplicate
application result identities or reclaim worker resources. Apply-before-cancel
may commit once, while cancel-before-apply rejects the mutation. Tokens are
process-local and safe to transfer to workers, but registry mutation remains on
the UI coordinator. Observer queues preserve FIFO snapshot and reentrant
deferral semantics while validating the retained incarnation at drain time.

## 24. Redraw Scheduling

Rendering should be event-driven.

The application should not redraw continuously when idle.

Redraw should happen on:

- Input events.
- Window resize.
- DPI scale changes.
- State changes.
- Worker/job result updates.
- Texture updates.
- Animation ticks.
- Caret blink.
- Active drag.
- Active playback.
- Video preview playback.
- Viewport interaction.

Continuous redraw is allowed only while something is actively changing:

```text
dragging
animation
playback
video preview
active viewport render
caret blink timing
progress indicator animation
```

Frame output should include redraw requests:

```rust
struct FrameOutput {
    primitives: Vec<Primitive>,
    semantic_nodes: Vec<SemanticNode>,
    action_invocations: Vec<ActionInvocation>,
    platform_requests: Vec<PlatformRequest>,
    repaint: RepaintRequest,
}
```

`RepaintRequest` may include:

```text
None
NextFrame
After(Duration)
Continuous
```

The Winit adapter resolves these into `Idle`, `Immediate`, a concrete deadline,
or `Continuous`. Each completed frame replaces the previous schedule instead
of accumulating it. A targeted shell response promotes the schedule to at
least Immediate; an expired deadline fires and clears once; Continuous remains
only until a later frame replaces it. Unrepresentable deadlines fail closed to
Idle rather than panicking.

## 25. Platform Adapter

The platform adapter normalizes platform-specific events and services.

Expected first adapter: `winit`.

Responsibilities:

- Window creation.
- Window resize events.
- DPI scale changes.
- Pointer events.
- Keyboard events.
- Text input events.
- Window focus.
- Close requests.
- Cursor updates.
- Clipboard requests.
- Redraw scheduling.
- Optional drag/drop path.
- Optional file-dialog integration path.

The Winit adapter provides a timestamped automatic mouse-button path. A repeat
requires the same button, nondecreasing event time, no more than 500
milliseconds between presses, and squared logical press distance at most 16.
Press counts saturate; matching releases carry the active count and unmatched
releases report zero. Pointer leave, focus loss, mismatched transitions,
missing comparison evidence, backwards time, an actual sanitized DPI change,
or explicit-count input clears automatic continuation. A DPI change also
invalidates prior logical pointer evidence as a pointer-leave transition until a
new cursor event establishes the new coordinate basis. The explicit-count API remains available for
platforms and tests that already supply click counts and clears automatic
history before emitting the supplied count unchanged.

The UI core consumes platform-independent input and emits platform-independent requests.

Platform request examples:

```text
set cursor
copy to clipboard
read clipboard
request redraw
set window title
start text input
update text input rectangle
stop text input
open HTTP/HTTPS URL
```

Platform-specific behavior should be isolated behind adapter traits.

The first Winit boundary uses an owned, non-cloneable one-frame request batch.
`from_frame_output` or replacement translation starts from an empty batch;
applying it consumes the value, actively sets the final cursor (Default when
absent), applies an optional final title, preserves ordered Start/Update/Stop
IME operations, and returns ordered shell work plus the authoritative repaint
request.

Clipboard writes, targeted reads, and URL opens share one ordered shell vector.
Injected services execute every operation once and continue after individual
failure. Outcomes contain targeted clipboard responses and structured,
payload-free failures. Debug and display diagnostics must not include clipboard
contents or URL path, query, or fragment. Native services retain one clipboard
object for their lifetime, disable clipboard image support, and accept only
HTTP/HTTPS browser targets.

## 26. Accessibility And Semantic Nodes

Render primitives are visual.

Semantic nodes describe meaning.

The UI runtime should allow widgets to emit semantic nodes alongside render primitives.

Semantic node responsibilities:

- Role.
- Label.
- Description.
- Bounds.
- Enabled/disabled state.
- Focus state.
- Selected/checked state.
- Value.
- Action affordances.
- Parent/child relationships.

Example roles:

```text
Button
Checkbox
RadioButton
TextField
Slider
Tab
TabPanel
MenuItem
Table
Row
Cell
Tree
TreeItem
Viewport
```

The accessibility implementation may be phased, but the architecture should not require retrofitting semantics into visual primitives.

Accessibility export uses a validated, backend-neutral snapshot:

```text
SemanticTree -> AccessibilitySnapshot -> platform accessibility adapter
```

The snapshot preserves semantic traversal order, parent/child relationships,
focus order, focused widget identity, roles, labels, descriptions, values,
states, bounds, and action affordances. It is independent from render
primitives, renderer resources, and OS accessibility APIs.

`kinetik-ui-winit` exposes `WinitAccessibilityUpdate` as the platform handoff
point for future native accessibility backends. The handoff remains free of OS
accessibility services so tests can prove semantic preservation without a
window, GPU, or platform accessibility daemon.

## 27. Asset And Resource Model

Resources should be referenced by stable handles.

Core resource IDs:

```text
IconId
ImageId
TextureId
FontId
GlyphCacheKey
```

Icons:

- Prefer vector/path icons where possible.
- Support theme-aware icon color.
- Support bitmap icons when needed.

Images:

- Decoded bitmap resources.
- Used for previews, thumbnails, static assets.

Textures:

- GPU-resident surfaces.
- Used for video, 3D, viewport previews, processed frames.

Missing resource policy:

- Missing icons should render a fallback symbol in debug-friendly form.
- Missing images/textures should render a visible placeholder.
- Missing fonts should fall back to default font resolution.
- Recoverable resource issues should produce structured warnings.

## 28. Error And Debug Policy

Programmer errors:

- Duplicate IDs in the same scope.
- Unbalanced clip/layer stack.
- Invalid layout constraints.
- Invalid negative sizes.
- Invalid state access.

Runtime recoverable issues:

- Missing texture.
- Missing image.
- Missing font.
- Failed resource upload.
- Renderer temporary failure.

Recommended policy:

```text
debug_assert for clear programmer errors in debug builds
structured warnings for recoverable runtime issues
visible fallback UI for missing visual resources
error values for renderer/platform failures
no silent failures for layout or identity problems
```

Snapshot restore helpers may pair error-return validation with additive
structured diagnostics. Diagnostics should remain deterministic, use stable
codes, and provide typed context rather than string-only messages so future
debug tools can present them without parsing text.

The toolkit should include debug visualization modes:

- Layout rects.
- Hit boxes.
- Focus chain.
- Widget IDs.
- Clipping regions.
- Repaint reasons.
- Overdraw/layer ordering where possible.
