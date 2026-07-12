# Kinetik UI Specification: Foundations

This file is part of the Kinetik UI architecture specification. The canonical entrypoint is [../specs.md](../specs.md).

Contained sections: 1-10.

## 1. Purpose

Kinetik UI is a reusable Rust UI toolkit for building fast, consistent, editor-style desktop applications.

The toolkit is designed for applications with dense tool surfaces, panels, media viewports, inspectors, tables, timelines, property grids, action bars, command palettes, and docked editor regions.

The primary design goals are:

- Crisp rendering across DPI scales.
- Low input latency.
- Deterministic frame behavior.
- A clear mental model for humans and LLM agents.
- A reusable component vocabulary for multiple applications.
- Strong separation between layout, behavior, styling, rendering, and application logic.
- UI code that is expressive without becoming a web/DOM clone.
- A testable architecture where most behavior can be validated without opening a window.
- A non-blocking application model where heavy work cannot freeze the UI.

The toolkit should be compact in concept, but not incomplete in intent. It is not scoped around an MVP mindset. Implementation can be phased, but the architecture should describe the desired complete direction.

## 2. Core Philosophy

Components are not primitives.

Primitives are:

- Layout boxes.
- Input regions.
- Interaction behaviors.
- Render commands.
- State slots.
- Semantic accessibility nodes.

Components are reusable compositions of primitives.

This distinction exists to avoid over-opinionated widgets. For example, a `Button` must not be the only reusable clickable thing. A tab header, menu item, list row, toolbar icon, disclosure row, transport button, and segmented control item are all button-like, but they should not inherit a single visual definition.

The toolkit should provide neutral behavior primitives:

- `pressable`
- `selectable`
- `draggable`
- `focusable`
- `scrollable`
- `text_editable`
- `drop_target`
- `tooltip_trigger`
- `context_menu_trigger`

Named components should then be built from those behaviors:

```text
Button = pressable + label/icon layout + button style recipe
TabHeader = selectable + pressable + tab style recipe
MenuItem = pressable + row layout + menu style recipe
ListRow = selectable + pressable + row style recipe
Slider = draggable + value mapping + track/thumb style recipe
TextField = focusable + text_editable + text layout + field style recipe
```

Behavior should be neutral.

Appearance should come from theme tokens and component style recipes.

Renderers should know about render primitives, not widget types.

Application logic should know about actions and domain state, not how widgets are painted.

## 3. System Overview

The system is divided into layers:

```text
Application
  Owns business state, documents, tools, commands, processing jobs, and domain renderers.

Kinetik UI Runtime
  Owns frame lifecycle, input normalization, layout, widget identity,
  UI memory, hit testing, focus, active/hover state, action dispatch,
  semantic nodes, and draw list generation.

Widgets / Components
  Compose layout + behavior + style recipes into reusable controls.

Text Subsystem
  Handles shaping, layout, wrapping, caret placement, selection,
  editing state, and text undo.

Renderer Boundary
  Consumes render primitives and draws them using a backend implementation.

Platform Adapter
  Provides window events, input events, DPI, cursor changes,
  clipboard access, redraw scheduling, and platform services.

Viewport / Media Renderer
  Produces GPU texture surfaces for video, images, 3D scenes,
  processed previews, or domain-specific render targets.
```

The intended high-level frame flow is:

```text
OS/window events
  -> platform adapter
  -> normalized UI input
  -> Kinetik UI frame
  -> application builds UI top-down
  -> layout + interaction + draw primitives + semantic nodes
  -> consume one owned platform batch and execute ordered shell work once
  -> renderer draws 2D UI and composites texture surfaces
  -> platform presents frame
  -> clear transient input, append targeted shell responses, schedule redraw
```

External side effects are frame-owned. A recoverable render failure may request
another frame, but it must not replay the already-consumed clipboard, URL,
title, cursor, or IME batch. Shell responses are appended only after transient
input is cleared so they become ordered input for exactly the next frame.

## 4. Crate Layout

The crate graph should derive from the `kinetik-ui` name and preserve clean dependency boundaries.

Recommended crates:

```text
kinetik-ui-core
  Pure core types and runtime concepts.
  No winit, no wgpu, no vello, no platform-specific dependencies.

kinetik-ui-widgets
  Core components and editor patterns built from kinetik-ui-core.

kinetik-ui-render
  Renderer traits, render primitive types, resource handles,
  and backend-independent rendering contracts.

kinetik-ui-vello
  First 2D renderer backend using Vello.

kinetik-ui-winit
  winit platform adapter, event normalization, window integration,
  cursor mapping, redraw scheduling, and DPI handling.

kinetik-ui-vello-winit
  Concrete one-window integration owning the Vello/wgpu surface, current
  device/queue, presentation order, and recovery policy. It does not own the
  event loop, input, shell requests, or application state.

kinetik-ui
  Facade crate that re-exports the common application stack.

kinetik-ui-showcase
  Showcase application and visual regression target.
```

Dependency direction:

```text
kinetik-ui-core <- kinetik-ui-widgets
kinetik-ui-core <- kinetik-ui-render <- kinetik-ui-vello
kinetik-ui-core <- kinetik-ui-winit
(kinetik-ui-core, kinetik-ui-render, kinetik-ui-vello)
  <- kinetik-ui-vello-winit
(all public library layers) <- kinetik-ui
```

`kinetik-ui-core` must remain platform-independent.

The UI runtime must not depend directly on Vello, wgpu, winit, or OS APIs.

Renderer backends must not know about component types such as `Button`, `Tabs`, or `PropertyGrid`. They should only consume render primitives and resources.

Application code should normally depend on the `kinetik-ui` facade crate. Custom
renderers should depend on `kinetik-ui-render`, Vello integrations on
`kinetik-ui-vello`, winit shells on `kinetik-ui-winit`, and the accepted live
Vello window path on `kinetik-ui-vello-winit`. The concrete presenter uses
Winit directly but does not depend on the input/shell adapter merely to share a
windowing library. Breaking crate graph changes require migration notes; the
`ef7c2f9` consolidation and later presenter addition are documented in
[`crate-migration.md`](../crate-migration.md).

## 5. Terminology

Use consistent terms throughout the codebase.

```text
WidgetId
  Stable identity for stateful UI elements.

Ui
  The main frame-building context passed through user UI code.

UiInput
  Normalized input state for the current frame.

UiMemory
  Retained interaction and widget state owned by the toolkit.

Response
  Result of an interaction primitive or component call.

Primitive
  Backend-independent render command.

SemanticNode
  Accessibility and semantic description of a widget or region.

Action
  User-invokable command described by the application.

ActionInvocation
  A request emitted by the UI indicating that an action was invoked.

Dock
  Editor layout manager that owns Frames.

Frame
  Docked/sub-window editor container.

Panel
  Passive content surface inside a Frame.

PanelTypeId
  Stable identity for a developer-declared kind of panel.

PanelInstanceId
  Stable identity for one open instance of a panel type. Existing `PanelId`
  vocabulary is compatibility terminology for panel instances.

WorkspaceSnapshot
  Additive persistence shell that wraps `DockSnapshot` with typed open panel
  instance records.

ViewportSurface
  UI-managed rectangle that displays domain-rendered texture content.
```

Avoid interchangeable synonyms unless a distinction is intentionally specified.

## 6. Frame Lifecycle

Kinetik UI uses an immediate-style API with retained UI memory.

The application builds the UI top-down each frame. Widgets are called during this pass. Stateful widgets use stable IDs to retrieve and update retained UI memory.

Frame lifecycle:

```text
1. Platform receives events.
2. Platform adapter normalizes events into UiInput.
3. Application calls ui.begin_frame(frame_context).
4. Application builds UI top-down.
5. Widgets measure and allocate layout rectangles.
6. Layered or overlapping regions predeclare one closed pointer target plan.
7. Widgets resolve interactions immediately during their calls.
8. Widgets update UiMemory through stable WidgetIds.
9. Widgets emit render primitives.
10. Widgets emit semantic nodes.
11. Widgets may emit action invocations.
12. ui.end_frame() finalizes cleanup and returns FrameOutput.
13. Renderer draws primitives.
14. Platform presents the frame.
15. Redraw scheduling decides whether another frame is needed.
```

Normal widget ID registration proves that an immediate-mode widget instance was
present during the frame and independently participates in duplicate-ID
diagnostics. Custom semantic nodes and evaluated text-input helpers provide
non-duplicating presence evidence for their owner. Deriving an ID with
`make_id` or declaring it in a pointer plan does not prove presence; custom or
intentionally hidden-but-retained widgets register their externally derived ID
explicitly.

At `end_frame`, persistent capture, active, pressed, secondary-pressed,
press-origin/threshold, drag, focus, and text/IME ownership is reconciled
against that frame-local presence set. A missing pointer owner cancels the
whole pointer transaction. Missing
focus and text owners are cleared, and text removal queues exactly one platform
stop intent. This cleanup requests a follow-up repaint but never rewrites a
current-frame response. Disabled, clipped, collapsed, or hidden-but-registered
widgets are present even when another contract makes them ineligible for input.
Evaluating a disabled current pointer owner cancels its transaction immediately;
presence alone does not keep an ineligible active gesture alive.

Interaction resolution should happen during widget calls, not in a hidden post-pass, unless a specific subsystem requires deferred resolution.

An immediate response cannot discover a visually later overlapping target.
After layout and before the first routed behavior call, layered UI therefore
predeclares one frame-local pointer plan. Unique explicit paint ordinals, not
declaration or behavior-evaluation order, select the top target. The plan is
closed-world: undeclared behavior IDs are inert, duplicate ordinals or IDs fail
safe, and no previous-frame hit tree or end-frame response rewrite is used.

The plan installs exact ordinary, drop, and wheel owners. These routes are
independent so a descendant may own press, its scroll ancestor may consume the
wheel exactly once, and a captured drag source may coexist with one drop
destination. Canonical cursor equivalence never grants activation. Capture-
lower and modal barriers cancel incompatible lower pointer ownership before
widget calls. Raw `hit_test` functions remain geometry queries rather than
routed activation APIs.

Ordinary and wheel routes use the frame-final pointer snapshot. When a retained
drag has an ordered terminating release, the drop route instead uses that first
release's event-time position. A ReleaseAll or focus loss before the release
blocks the route; cancellation after the release cannot rewrite it. The
captured source's effective clip must also accept the release, so a cleanup-only
edge cannot target a larger external drop region.

Wheel input stages the next retained scroll offset. Target geometry, paint,
semantics, debug bounds, and clipping retain the frame-start offset until
`end_frame`; the staged value becomes spatially observable in the next frame.
This prevents routing and visible geometry from using different scroll states.

Example:

```rust
if ui.button("Analyze").clicked() {
    ui.emit_action(AppAction::Analyze);
}
```

Internally this should:

```text
1. Resolve button ID.
2. Measure desired size.
3. Allocate final rect.
4. Run pressable behavior against current input and previous memory.
5. Emit button primitives using theme recipe.
6. Emit semantic node.
7. Return Response.
```

## 7. Coordinate And DPI Model

Layout uses logical units.

Rendering targets physical pixels.

```text
physical = logical * scale_factor
```

Logical units are the toolkit's cross-platform UI measurement. They are similar to CSS pixels, Windows device-independent pixels, Flutter logical pixels, Android dp, and macOS points.

All UI-facing dimensions should be expressed in logical units:

- Widget sizes.
- Padding.
- Margins.
- Font sizes.
- Border radii.
- Stroke widths.
- Table row heights.
- Panel widths.
- Timeline heights.

The platform adapter provides:

```rust
struct ViewportInfo {
    logical_size: Size,
    physical_size: PhysicalSize,
    scale_factor: f64,
}
```

The UI runtime lays out using `logical_size`.

The renderer receives logical primitives plus `scale_factor`, then draws scale-aware output to the physical render target.

Render transform and clip scopes also define the runtime coordinate scope.
Transforms compose in stream order as parent then child. Widget rectangles,
`Response::rect`, and all public `UiInput` accessors use current-scope logical
coordinates. Every ordered pointer event is localized from its own event-time
position. Pointer positions use the complete inverse affine transform; movement
and pixel-wheel vectors use its inverse linear portion, while line-wheel values
remain device-independent lines. Semantic, debug, and IME rectangles are
exported in screen-logical coordinates.

Effective clipping preserves transformed clip regions rather than reducing
them to screen-space bounding boxes. Input outside any active clip cannot
hover, press, click, wheel-scroll, or contribute drag movement. Fully clipped
semantic nodes remain structurally present but leave focus traversal, and a
clipped focused text owner is blurred and stops platform text input in the same
frame. Singular or non-finite scopes stay balanced in the primitive stream and
make descendant input and exported geometry inert until the parent scope is
restored.

Runtime-owned scrolling emits one clip plus one translation. Collection layout
may use the scroll offset to choose a virtualized materialization range, but
content geometry inside that scope must not subtract the same offset again.

Pixel alignment rules:

- Filled rectangles may use logical coordinates directly.
- One-pixel strokes should be aligned to physical pixel boundaries.
- Text should be shaped/rasterized for the target scale rather than rendered at one scale and bitmap-scaled.
- Texture viewports should support exact 1:1 mapping where one content pixel maps to one physical pixel.
- Rulers, guides, crosshairs, and viewport handles should be device-pixel aligned when appropriate.

## 8. Input Model

The platform adapter normalizes window and input events into `UiInput`.

Input categories:

```text
Pointer
  position, delta, buttons, clicks, double clicks, wheel, capture state

Keyboard
  physical key, logical key, modifiers, repeat, pressed/released

Text
  text input, composition events, IME path

Window
  resize, scale factor changes, focus, close request

Time
  current timestamp, delta time, frame counter
```

Recommended core input shape:

```rust
struct UiInput {
    events: Vec<UiInputEvent>,
    pointer: PointerInput,
    keyboard: KeyboardInput,
    text_events: Vec<TextInputEvent>,
    clipboard_text: Vec<ClipboardText>,
    window_focused: bool,
}
```

`UiInput.events` is the authoritative sequence for Winit, the test harness, and
other official producers. It records pointer movement, leave, button and
release-all transitions with event-time positions; typed line/pixel wheel
events; modifier and key events; text/IME events; targeted clipboard results;
IME availability; and window focus. `KeyEvent.text` carries layout-produced
hardware text. IME commits remain `TextInputEvent::Commit`, so adapters never
deduplicate by comparing strings.

`UiInput::push_event` appends exactly once and updates the compatibility
snapshots in the same call. `begin_frame` clears the ordered stream and all
transient projections together while preserving retained button-down,
modifier, pointer-position, and focus state. An empty stream is the explicit
legacy snapshot path; text consumption synthesizes the historical text-domain
order because legacy pointer order cannot be recovered. A non-empty stream
whose key, text, clipboard, focus, modifier, or pointer transient projections
were mutated inconsistently emits a structured frame warning and fails closed
for text editing. Root projection validation is recorded once before spatial
localization. Scoped text validation rechecks only unchanged non-pointer
projections, so legitimate local pointer coordinates do not block editing;
root conflicts remain out-of-band and preserve the canonical pointer snapshot
instead of healing from an inconsistent compatibility projection. Pointer
behaviors fail closed for new hover, activation, click, drag, and drop work on
any root conflict; a previously captured owner may consume only an ordered
release or cancellation edge to clean up retained state.

Winit IME Enabled/Disabled events describe availability, not active
composition. Non-empty preedit starts or updates composition, empty preedit
ends it, and commit ends active preedit before inserting. Hardware key text is
suppressed only while preedit is active. Focus loss ends composition, records
pointer release-all and leave, then records focus loss; later editing events in
that frame remain observable but do not mutate text.

Input therefore exposes both an ordered canonical stream and compatibility
snapshots for the current frame. Scroll consumption reads individual canonical
wheel events when the stream is nonempty: each line component uses a private
40-logical-unit current-scope step, logical pixel components remain exact after
platform DPI conversion and spatial localization, nonfinite components become
zero, and direction inverts once. An empty stream preserves the raw logical
compatibility magnitude. Press, secondary press, drag, release, and captured
selection consume canonical pointer transitions once in order whenever that
stream is nonempty; they never replay compatibility snapshot edges. The empty
stream remains the legacy snapshot path. Spatial localization privately keeps
each surviving event paired with its original root-stream ordinal, including
gaps created by clipping, plus whether a captured release survived only for
cleanup, without adding metadata to public `UiInput`. Ordered focus-loss and
release-all cancellation is deferred until behaviors can observe preceding
transitions; `end_frame` performs the same cleanup if no owner participates.

Captured selection actions also carry the modifier state effective at their
original root ordinal. The runtime folds the root stream once from a retained
cross-frame baseline: modifier changes and key-carried modifiers replace the
running state, spatial filtering keeps the root association, and the final
compatibility snapshot is never guessed backward onto earlier pointer events.
An empty-stream focused frame uses and retains its modifier snapshot. Focus loss
reports the pre-loss state, clears the baseline, and suspends later modifier/key
updates until a valid focus-gain event. The official release-all, pointer-leave,
focus-loss sequence therefore emits one selection cancellation at release-all
while the independent modifier fold still reaches the focus-loss reset. A
conflicted stream cannot apply modifier/key changes, though its focus-loss fence
still performs the safety reset.

Captured DomainDrag actions use the same root-ordinal modifier lookup but a
DomainDrag-specific public action type. Each Release records whether that exact
transition contributed the aggregate pointer click, so multiple transactions
in one frame remain causally distinguishable. Action ordinals are observation
metadata only: release/drop authority always comes from the private scoped root
pointer sidecar. No standalone captured adapter or local ordinal namespace is
defined.

The runtime retains one logical text-input owner with an explicit
`TextInputOwnerMode`. Editable and ReadOnly owners may each claim the canonical
ordered editing stream once. ReadOnly ownership permits navigation, selection,
scrolling, and copy but cannot mutate text, request or accept paste, cut, or
activate native IME. Editable ownership may do both logical editing and native
IME work. Disabled fields do not become logical owners. The native platform
text-input state is separate from logical ownership, so changing between
Editable and ReadOnly does not invent an owner handoff or a spurious platform
stop.

Text fields merge pointer and editing actions by original root ordinal. A
retained selection anchor preserves multiframe drag identity, while exact
release-click provenance distinguishes caret placement from a threshold-crossed
domain drag. Authoritative editable numeric scrub resolves DomainDrag exactly
once; it previews ordered editing on cloned state, validates the arithmetic and
pointer transaction, consumes the exact cached claim, and commits once. Other
text selection consumes the neutral Selection facade. Aggregate response flags
cannot substitute for causal pointer metadata.

The input model must support pointer capture. During a drag, the active widget
continues receiving drag updates after leaving its original rectangle while it
remains inside the effective clip. Outside the effective clip, interaction is
inert except that button-release edges remain available to clean up capture.
Primary presses retain their current-scope logical origin and use a private
four-unit inclusive threshold. Crossing latches even if the pointer moves back;
the crossing response reports the full origin displacement and later frames
report only subsequent movement. Any crossed release suppresses a pointer
click. Only the `draggable` primitive publishes a domain drag source.
Drop targets use canonical release-time geometry rather than the frame-final
pointer snapshot; missing canonical button positions fail closed.

Text input has priority when a text editor is focused. Keyboard shortcuts should not steal ordinary typing from focused text fields.

Starting native text input and moving its candidate area are distinct platform
operations. Only an Editable logical owner may activate native IME. Initial
activation emits `StartTextInput`; the same focused editable owner emits
`UpdateTextInputRect` with its visible clipped caret rectangle in
screen-logical coordinates, without restarting IME or composition. Owner
handoff remains ordered Stop then Start. If a frozen text viewport hides the
caret, the runtime retains the previous platform rectangle, stages caret reveal,
and publishes the new rectangle with the following frame's geometry.

## 9. Widget Identity

Every stateful widget has a stable `WidgetId`.

IDs must remain deterministic across frames.

IDs may be derived from:

- Explicit user-provided IDs.
- Parent ID stack.
- Component-local keys.
- Stable row/item keys for lists, trees, and tables.
- Stable tab/frame identifiers.

IDs must not rely only on call order when widgets can be reordered, filtered, virtualized, or conditionally displayed.

Examples:

```rust
ui.text_field("project_name", &mut project.name);
ui.slider("screen_strength", &mut settings.screen_strength, 0.0..=1.0);
ui.table("media_table", media.items(), |table| {
    table.row_key(|item| item.id);
});
```

The runtime should provide ID stack helpers:

```rust
ui.with_id("settings_panel", |ui| {
    ui.text_field("search", &mut search);
});
```

Debug policy:

- Duplicate IDs in the same active scope should produce a structured debug warning.
- In debug builds, severe ID misuse may trigger `debug_assert`.
- Release builds should degrade gracefully where possible.

## 10. UI Memory

Application state belongs to the application.

UI memory belongs to Kinetik UI.

Application state examples:

```text
current project
loaded media
selected clip
timeline position
effect parameters
document dirty state
processing job status
viewport zoom/pan if domain-owned
```

UI memory examples:

```text
hovered widget
focused widget
active widget
pressed widget
scroll offsets
text edit state
selection state for toolkit-owned selections
open popovers
open menus
drag state
animation state
layout cache
text layout cache
measure cache
```

Recommended shape:

```rust
struct UiMemory {
    hovered: Option<WidgetId>,
    focused: Option<WidgetId>,
    active: Option<WidgetId>,
    pressed: Option<WidgetId>,
    scroll_offsets: StateMap<WidgetId, Vec2>,
    text_editors: StateMap<WidgetId, TextEditState>,
    selections: StateMap<WidgetId, SelectionState>,
    open_popovers: StateSet<WidgetId>,
    drags: DragMemory,
    animations: AnimationMemory,
    layout_cache: LayoutCache,
    text_cache: TextCache,
}
```

Widgets should access memory through controlled runtime APIs rather than directly mutating global structures.

Only the current `text_input_owner` may claim a frame's ordered editing stream,
and only one claim succeeds. Ownership handoff before the claim routes the
stream to the new owner; handoff after a claim cannot replay it. The transient
claim clears at frame start alongside other frame-local memory.

Owner reconciliation clears only runtime ownership handles. It must not prune
application documents, values, selections, domain drag state, scroll offsets,
popover state, caches, or async incarnation/liveness state. Reusing a removed
widget ID later does not restore ownership that cleanup already cleared.

Async liveness has two separate truths. Presence records that an owner was seen
in the current frame; an incarnation remains active across continuously present
frames and authorizes external results through an opaque registry-scoped token.
Beginning a frame clears presence evidence without invalidating the active
incarnation. Omission retires only at frame finalization, so a result arriving
before that boundary may still apply.

First activation, same-ID reentry after retirement, and explicit restart each
allocate a checked registry-wide monotonic `LivenessIncarnation`. Repeated
presence marks allocate nothing and return the identical token. Validation
rejects foreign registry scopes as stale targets, reports a different latest
incarnation as `StaleIncarnation`, accepts the exact active incarnation, and
preserves an exact `Cancelled` result only while that cancelled incarnation is
still the latest retained record. A replacement incarnation therefore takes
precedence over an older cancellation and cannot be cancelled by the old token.

Cancellation, explicit removal, and omission create typed tombstones at the
current checked frame epoch. A tombstone survives the retirement frame plus one
complete following frame and prunes at that following frame's finalization;
repeated cancellation does not extend it. Pruning never resets the private
registry scope or incarnation allocator, preventing same-ID ABA after cleanup.
Tombstone storage is temporally bounded by this policy rather than by a hard
count cap.

`UiMemory`, `LivenessRegistry`, `ObserverRegistry`, and the test harness that
owns memory are non-cloneable authority holders. Their retained state remains
observationally comparable: private registry scope is ignored by state
`PartialEq`, while direct token equality and hashing include it. Equal state
does not make tokens interchangeable. Observer subscriptions retain one token
for one incarnation and validate on drain; cancellation, stale target, and
stale incarnation are observable skip reasons. Replacement work requires a new
subscription instead of per-frame token refresh.
