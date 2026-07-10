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
  -> renderer draws 2D UI and composites texture surfaces
  -> platform presents frame
```

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

kinetik-ui
  Facade crate that re-exports the common application stack.

kinetik-ui-showcase
  Showcase application and visual regression target.
```

Dependency direction:

```text
kinetik-ui-core
  <- kinetik-ui-widgets
  <- kinetik-ui-render
  <- kinetik-ui-vello
  <- kinetik-ui-winit
  <- kinetik-ui
```

`kinetik-ui-core` must remain platform-independent.

The UI runtime must not depend directly on Vello, wgpu, winit, or OS APIs.

Renderer backends must not know about component types such as `Button`, `Tabs`, or `PropertyGrid`. They should only consume render primitives and resources.

Application code should normally depend on the `kinetik-ui` facade crate. Custom
renderers should depend on `kinetik-ui-render`, Vello integrations on
`kinetik-ui-vello`, and winit shells on `kinetik-ui-winit`. Breaking crate graph
changes require migration notes; the `ef7c2f9` consolidation is documented in
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
6. Widgets resolve interactions immediately during their calls.
7. Widgets update UiMemory through stable WidgetIds.
8. Widgets emit render primitives.
9. Widgets emit semantic nodes.
10. Widgets may emit action invocations.
11. ui.end_frame() finalizes cleanup and returns FrameOutput.
12. Renderer draws primitives.
13. Platform presents the frame.
14. Redraw scheduling decides whether another frame is needed.
```

Interaction resolution should happen during widget calls, not in a hidden post-pass, unless a specific subsystem requires deferred resolution.

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
coordinates. Pointer positions use the complete inverse affine transform;
pointer movement and wheel vectors use its inverse linear portion. Semantic,
debug, and IME rectangles are exported in screen-logical coordinates.

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
    pointer: PointerInput,
    keyboard: KeyboardInput,
    text_events: Vec<TextInputEvent>,
    window_focused: bool,
    time: TimeInfo,
}
```

Input should be stored as a snapshot for the current frame. Widgets should query current input and prior `UiMemory` to produce responses.

The input model must support pointer capture. During a drag, the active widget
continues receiving drag updates after leaving its original rectangle while it
remains inside the effective clip. Outside the effective clip, interaction is
inert except that button-release edges remain available to clean up capture.

Text input has priority when a text editor is focused. Keyboard shortcuts should not steal ordinary typing from focused text fields.

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
