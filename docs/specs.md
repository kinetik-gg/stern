# Kinetik UI Specification

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

DockArea
  Editor layout manager that owns Frames.

Frame
  Docked/sub-window editor container.

Panel
  Passive content surface inside a Frame.

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

The input model must support pointer capture. During a drag, the active widget should continue receiving drag updates even if the pointer leaves its original rect.

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

## 11. Layout Model

Layout is top-down and measurement-aware.

Parents receive an available rectangle and allocate rectangles to children.

Widgets may support measurement before final placement. This allows text, tables, tabs, wrapped rows, and fit-content containers to participate in layout without rendering first.

Each widget conceptually has:

```text
available size
desired size
final rect
optional clipping rect
```

Core layout primitives:

```text
Box
Row
Column
Stack
Grid
ScrollArea
SplitPane
DockArea
Frame
Panel
Overlay
Tabs
Spacer
Separator
Padding
Margin
Align
```

Sizing primitives:

```rust
enum SizeRule {
    Fixed(f32),
    Fit,
    Fill,
    Percent(f32),
    MinMax { min: f32, max: f32 },
    AspectRatio(f32),
}
```

Recommended layout contracts:

- `Row` lays children horizontally.
- `Column` lays children vertically.
- `Stack` overlays children in the same rect.
- `Grid` places children in rows and columns with fixed/fill/fit sizing.
- `ScrollArea` lays content in a scrollable viewport and clips children.
- `SplitPane` divides available space between two or more children.
- `DockArea` manages editor Frames.
- `Frame` owns editor-region behavior and chrome.
- `Panel` is a passive content surface.

Layout code should avoid web-like CSS complexity unless a feature is explicitly needed.

## 12. DockArea, Frame, And Panel

Editor-style applications need a hierarchy that separates workspace arrangement from content.

The hierarchy is:

```text
DockArea
  -> Frame
      -> Panel
          -> Components
              -> Primitives
```

### 12.1 DockArea

`DockArea` is the layout manager for editor frames.

Responsibilities:

- Own frame arrangement.
- Split regions horizontally or vertically.
- Manage tab groups.
- Manage active frame.
- Route focus to frames.
- Support frame insertion/removal.
- Support frame resizing.
- Support frame merging/tabbing.
- Support drag-to-dock behavior through explicit tab drag state and drop targets.

`DockArea` controls where frames live and how much space they receive.

Interactive docking remains model-owned and deterministic:

```text
frame tab drag -> DockTabDrag -> DockDropTarget -> DockArea mutation
splitter drag -> DockSplitPath + delta -> clamped split ratio
```

Splitters are addressed by `DockSplitPath`, and drop targets distinguish tab
merge from split insertion. These operations update the same dock tree that is
serialized by `DockArea::snapshot`.

### 12.2 Frame

`Frame` is a managed editor container. It is similar to a docked sub-window, not a floating modal.

Responsibilities:

- Frame title or tab label.
- Optional icon.
- Active/inactive visual state.
- Dismiss/close behavior.
- Resize handles when controlled by the dock layout.
- Drag handle for future docking interactions.
- Merge/tab behavior when supported.
- Focus region.
- Frame-level actions.

Frames are where editor-region behavior belongs.

### 12.3 Panel

`Panel` is a passive content surface.

Responsibilities:

- Visual surface.
- Padding.
- Optional header/body styling.
- Content clipping if requested.
- Local section styling.

Panels do not decide their own dock placement, outer size, drag behavior, dismissal, merge behavior, or workspace arrangement.

The same Panel should be reusable inside different Frames, tabs, split regions, showcase sections, or future modal-like contexts.

## 13. Interaction Primitives

Interaction primitives are behavior-only building blocks.

They must not hardcode component appearance.

Core primitives:

```text
hit_test
pressable
selectable
draggable
focusable
scrollable
text_editable
shortcut
context_menu_trigger
tooltip_trigger
drop_target
resizable
scrubbable
```

`Response` should be the common output:

```rust
struct Response {
    id: WidgetId,
    rect: Rect,
    hovered: bool,
    focused: bool,
    active: bool,
    pressed: bool,
    clicked: bool,
    double_clicked: bool,
    secondary_clicked: bool,
    dragged: bool,
    drag_delta: Vec2,
    disabled: bool,
}
```

Interaction state should support:

- Hover.
- Press.
- Release.
- Click.
- Double click.
- Context click.
- Drag start.
- Drag update.
- Drag end.
- Pointer capture.
- Focus gain/loss.
- Keyboard activation.
- Disabled state.

Examples:

```rust
let response = ui.pressable("tab_foreground", desired_size);

if response.clicked {
    preview_mode = PreviewMode::Foreground;
}

theme.paint_tab(ui, response.rect, response.state(), selected);
```

```rust
let response = ui.draggable("splitter", splitter_rect);

if response.dragged {
    layout.resize_split(response.drag_delta.x);
}
```

## 14. Action System

Kinetik UI should include an action presentation, shortcut routing, and dispatch system.

Actions are context-aware user-invokable commands.

The application owns action meaning and execution.

The UI toolkit owns action presentation and invocation mechanics.

Examples of actions:

```text
Analyze
Export
ImportMedia
Undo
Redo
PlayPause
StepForward
StepBackward
FitViewport
SetZoom100
SetPreviewMode(Foreground)
DeleteSelectedItem
RenameSelectedItem
OpenCommandPalette
```

The same action may be invoked through:

- Menu bar.
- Toolbar button.
- Context menu.
- Keyboard shortcut.
- Command palette.
- Inspector button.
- Timeline control.
- Viewport control.

Recommended structures:

```rust
struct ActionDescriptor<ActionId> {
    id: ActionId,
    label: String,
    icon: Option<IconId>,
    shortcut: Option<Shortcut>,
    enabled: bool,
    visible: bool,
    checked: Option<bool>,
    tooltip: Option<String>,
    keywords: Vec<String>,
}

struct ActionInvocation<ActionId> {
    id: ActionId,
    source: ActionSource,
    context: ActionContextSnapshot,
}
```

The UI should expose helpers:

```rust
ui.action_button(actions.get(AppAction::Analyze));
ui.menu_action(actions.get(AppAction::Export));
ui.context_menu_action(actions.get(AppAction::DeleteSelectedItem));
ui.command_palette(&actions);
```

At the end of the frame, the application handles invocations:

```rust
for invocation in ui.take_action_invocations() {
    app.handle_action(invocation.id);
}
```

Shortcut routing should be context-aware.

Priority:

```text
1. Active modal interaction.
2. Focused text editor.
3. Focused widget.
4. Focused Frame/Panel.
5. DockArea/editor context.
6. Global application actions.
```

Text fields must consume typing and text-editing shortcuts before global actions receive them.

## 15. Rendering Primitives

Widgets emit backend-independent render primitives.

Render primitives do not know about widget types.

Initial primitive set:

```text
Rect
RoundedRect
Stroke
Line
Polyline
Path
Text
GlyphRun
Image
Texture
ClipBegin
ClipEnd
LayerBegin
LayerEnd
TransformBegin
TransformEnd
Shadow
Gradient
```

Recommended shape:

```rust
enum Primitive {
    Rect(RectPrimitive),
    RoundedRect(RoundedRectPrimitive),
    Stroke(StrokePrimitive),
    Line(LinePrimitive),
    Path(PathPrimitive),
    Text(TextPrimitive),
    GlyphRun(GlyphRunPrimitive),
    Image(ImagePrimitive),
    Texture(TexturePrimitive),
    ClipBegin(ClipId, Rect),
    ClipEnd(ClipId),
    LayerBegin(LayerId),
    LayerEnd(LayerId),
    TransformBegin(Transform),
    TransformEnd,
}
```

`Image` is for decoded/static bitmap content.

`Texture` is for GPU-resident content:

- Video frames.
- 3D viewports.
- Processed previews.
- External render targets.
- Timeline thumbnails generated by a renderer.

Primitive emission should preserve order unless explicit layers reorder output.

Clipping and layering must be explicit enough for renderers to implement correctly.

## 16. Renderer Boundary

The renderer consumes frame output and draws it.

The UI runtime must not depend directly on any renderer implementation.

Renderer input:

```text
viewport logical size
viewport physical size
scale factor
render primitive list
texture handles
image handles
font/glyph resources
clip/layer commands
clear color
```

Renderer output:

```text
presented frame
resource upload results
recoverable warnings
fatal renderer errors
```

The first 2D backend should use Vello.

Vello should handle:

- 2D vector shapes.
- Paths.
- Fills.
- Strokes.
- Clips.
- Gradients.
- Images where suitable.
- UI surface drawing.

wgpu interop should support:

- Texture surfaces for viewports.
- Video frames.
- 3D render targets.
- Processed image previews.

The renderer backend should be replaceable. Alternative backends may use Skia, Cairo, tiny-skia, raw wgpu, or another renderer.

## 17. Text Subsystem

Text is a first-class subsystem.

The toolkit should use a dedicated text engine for shaping, layout, editing, and glyph positioning. The expected first choice is `cosmic-text`.

Text subsystem responsibilities:

- Font discovery.
- Font fallback.
- Text shaping.
- Glyph positioning.
- Multi-line layout.
- Wrapping.
- Ellipsis.
- Caret positioning.
- Hit testing position to character/index.
- Selection ranges.
- Copy/paste integration.
- Internal text field undo.
- Numeric field editing.
- Search field editing.
- Multi-line editing.
- IME path in the architecture.

Text rendering responsibilities:

- Draw text at the correct DPI scale.
- Avoid bitmap-scaling text from the wrong resolution.
- Cache shaped text where possible.
- Cache glyph resources where possible.
- Invalidate cache on text/font/size/width/style changes.

Text field variants:

```text
single-line
multi-line
numeric
search
password/masked later if needed
```

Text field undo should be local to text editing and should not conflict with application-level undo unless the application explicitly bridges them.

## 18. Styling And Theme Model

Styling uses tokens, semantic roles, and component recipes.

The style system should avoid becoming CSS.

### 18.1 Base Tokens

```text
Color
Spacing
Radius
Stroke
Font
FontSize
LineHeight
Opacity
Elevation
Cursor
Duration
```

### 18.2 Semantic Tokens

```text
surface
surface_hover
surface_active
surface_sunken
text
text_muted
text_disabled
accent
danger
warning
success
border
border_subtle
focus_ring
selection
disabled
overlay
viewport_background
```

### 18.3 Component Recipes

Component recipes map component variant + state to style values.

Examples:

```text
button.primary.normal
button.primary.hovered
button.primary.pressed
button.ghost.disabled
tab.active
tab.inactive.hovered
table.row.selected
table.row.hovered
slider.thumb.dragged
text_field.focused
frame.active
frame.inactive
```

### 18.4 Override Policy

Preferred customization order:

```text
1. Theme tokens.
2. Component variants.
3. Component recipe overrides.
4. Local style overrides.
5. Custom paint using behavior primitives.
```

Common components may be opinionated for convenience. Lower-level behavior primitives must remain neutral.

## 19. Components

Core component set:

```text
Label
RichLabel
Button
IconButton
Checkbox
RadioButton
Toggle
Slider
NumericInput
TextField
SearchField
Dropdown
MenuBar
Menu
MenuItem
ContextMenu
Tabs
Panel
Frame
DockArea
Toolbar
StatusBar
List
Grid
Table
Tree
PropertyGrid
Modal
Popover
Tooltip
CommandPalette
Viewport
Timeline
Ruler
TransportControls
ProgressIndicator
```

Components should be built from:

- Layout primitives.
- Interaction primitives.
- Theme recipes.
- Render primitives.
- Optional semantic nodes.
- Optional action descriptors.

Components should expose a concise common path and a configurable path.

Example:

```rust
if ui.button("Analyze").clicked() {
    ui.emit_action(AppAction::Analyze);
}
```

Builder-style configuration should be available for more complex cases:

```rust
Button::new("Analyze")
    .icon(IconId::Analyze)
    .variant(ButtonVariant::Primary)
    .enabled(can_analyze)
    .show(ui);
```

The canonical internal implementation should still use shared behavior primitives.

## 20. Lists, Tables, Trees, And Virtualization

Internal tools often display large collections.

Virtualization should be part of the architecture early, not an afterthought.

Collection primitives:

```text
CollectionView
SelectionModel
SortModel
FilterModel
ColumnModel
VirtualList
VirtualTable
TreeModel
ExpansionState
```

List responsibilities:

- Visible range calculation.
- Scroll offset.
- Selection.
- Keyboard navigation.
- Optional multi-select.
- Row keys.
- Row hover/press actions.

Table responsibilities:

- Column definitions.
- Column widths.
- Header rows.
- Visible row virtualization.
- Cell layout.
- Selection.
- Sorting state.
- Optional resizing.
- Optional horizontal scroll.

Tree responsibilities:

- Expansion state.
- Indentation.
- Virtualized visible node list.
- Selection.
- Keyboard navigation.

The toolkit should support thousands of rows without laying out or painting every row each frame.

## 21. Menus, Popovers, And Overlays

Menus are custom-rendered.

Overlay primitives must be first-class because menus, tooltips, command palettes, dropdowns, context menus, drag previews, and popovers often escape normal parent clipping.

Overlay responsibilities:

- Z-order.
- Focus routing.
- Outside-click dismissal.
- Escape-key dismissal.
- Placement relative to anchor rect.
- Screen-edge collision handling.
- Optional modality.
- Nested menu behavior.
- Semantic nodes.

Overlay types:

```text
Tooltip
Popover
Dropdown
ContextMenu
Menu
CommandPalette
Modal
DragPreview
```

Overlays should use the same render primitive pipeline, but their layer ordering and input capture must be handled explicitly.

## 22. Viewport Surfaces

Viewport-like work is part of the toolkit's UI domain as a container and interaction surface, but domain rendering must remain separate.

The UI toolkit owns:

- Viewport rectangle.
- Clipping.
- Focus and hover routing.
- Cursor behavior.
- Pan/zoom/fit helpers.
- Screen/content coordinate conversion.
- Overlay primitives.
- Viewport toolbar controls.
- Rulers and guides.
- Gesture routing.

The domain renderer owns:

- Video decoding.
- Image processing.
- 3D rendering.
- Color management.
- Effects rendering.
- Render targets.
- GPU texture production.

Viewport coordinate spaces:

```text
window space
UI logical space
viewport local space
content space
layer/object space
```

Required conversion helpers:

```rust
viewport.screen_to_content(pos)
viewport.content_to_screen(point)
viewport.viewport_to_content(pos)
viewport.content_rect_to_screen(rect)
```

Viewport display modes:

```text
fit
fill
contain
actual size / 100%
custom zoom
pan
```

Texture handling:

- Texture surfaces should be represented by stable `TextureId` or `TextureHandle`.
- UI repaint should not force texture re-upload.
- Video frames should upload only when frame content changes.
- 3D/render previews should render to GPU targets and pass handles to UI.

Overlay examples:

- Rulers.
- Center guides.
- Safe areas.
- Transform boxes.
- Selection outlines.
- Bezier masks.
- Handles.
- Crosshairs.
- Timeline scrub overlays.

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
At(Instant)
ContinuousUntil(condition)
```

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

The UI core consumes platform-independent input and emits platform-independent requests.

Platform request examples:

```text
set cursor
copy to clipboard
read clipboard
request redraw
set window title
start text input
stop text input
```

Platform-specific behavior should be isolated behind adapter traits.

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

The toolkit should include debug visualization modes:

- Layout rects.
- Hit boxes.
- Focus chain.
- Widget IDs.
- Clipping regions.
- Repaint reasons.
- Overdraw/layer ordering where possible.

## 29. Testing Strategy

Most UI behavior should be testable without opening a window.

Unit-testable areas:

```text
geometry math
DPI conversions
layout calculations
measurement
widget ID stability
hit testing
hover/press/click transitions
drag behavior
pointer capture
focus movement
shortcut routing
action dispatch
scroll clamping
slider value mapping
table virtualization
tree visible node calculation
theme token resolution
primitive emission
text edit state
text undo
viewport coordinate conversion
overlay placement
redraw scheduling
```

Core tests should not require GPU, window creation, or platform services.

Primitive snapshot tests may verify generated draw lists:

```rust
let output = run_ui_test(|ui| {
    ui.button("Analyze");
});

assert_snapshot!(output.primitives);
```

Renderer snapshot tests should prefer deterministic resource inventories and
backend command streams over pixel images. Backend-neutral resource snapshots
belong in `kinetik-ui-render`; Vello command snapshots belong in
`kinetik-ui-vello`. See [render-snapshots.md](render-snapshots.md).

Interaction tests may simulate input:

```rust
let mut harness = UiTestHarness::new();
harness.pointer_move(pos);
harness.pointer_down(MouseButton::Primary);
harness.pointer_up(MouseButton::Primary);

assert!(harness.response("analyze_button").clicked);
```

Visual tests may render showcase scenes to images and compare snapshots.

Pixel-perfect visual tests should be used carefully because they can be brittle, but they are useful for catching major regressions.

## 30. Continuous Integration

The repository should include GitHub Actions workflows for formatting, linting, testing, building, docs, examples, and showcase checks.

CI should run on:

```text
pull_request events
push events to main
```

CI should not run on every feature-branch push unless that branch has an open pull request.

Default workflow trigger:

```yaml
on:
  pull_request:
  push:
    branches: [main]
```

Required checks:

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo build --workspace --all-features
cargo check --workspace --examples --all-features
cargo doc --workspace --all-features --no-deps
```

Recommended platform matrix:

```text
Windows
Linux
macOS
```

At minimum, core tests should run on every supported CI platform.

Renderer-related checks should be split:

```text
core tests
  no GPU/window required

renderer smoke tests
  backend-specific, may be headless/offscreen where possible

showcase build
  compile showcase app without opening a window

visual regression tests
  optional, controlled, and stable enough to avoid noisy failures
```

Example workflow:

```yaml
name: CI

on:
  pull_request:
  push:
    branches: [main]

jobs:
  test:
    strategy:
      matrix:
        os: [ubuntu-latest, windows-latest, macos-latest]

    runs-on: ${{ matrix.os }}

    steps:
      - uses: actions/checkout@v4

      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy

      - uses: Swatinem/rust-cache@v2

      - run: cargo fmt --all -- --check

      - run: cargo clippy --workspace --all-targets --all-features -- -D warnings

      - run: cargo test --workspace --all-features

      - run: cargo build --workspace --all-features

      - run: cargo check --workspace --examples --all-features

      - run: cargo doc --workspace --all-features --no-deps
        env:
          RUSTDOCFLAGS: -D warnings
```

## 31. Showcase Application

The toolkit should include a showcase application that acts as:

- Living documentation.
- Component gallery.
- Interaction testbed.
- Theme tuning surface.
- Visual regression target.
- LLM-readable usage reference.
- Realistic editor-layout exercise.

Showcase areas:

```text
Component gallery
  labels, buttons, icon buttons, toggles, checkboxes, radios,
  sliders, dropdowns, fields, tabs, menus, overlays

Inspector/property grid
  dense labeled rows, numeric controls, dropdowns, sections,
  disabled states, validation states

Media/editor layout
  DockArea -> Frames -> Panels layout similar to editor applications

Viewport demo
  texture surface, zoom/pan, rulers, crosshair, overlays, fit/100% controls

Table stress test
  virtualized table with many rows, selection, sorting, scrolling

Text input states
  single-line, multi-line, numeric, search, undo, selection, focus

Action system demo
  same actions invoked through menu, toolbar, shortcut, command palette

Theme states
  normal, hover, pressed, focused, disabled, selected, danger, warning

DPI checks
  logical scaling, text crispness, stroke alignment
```

The showcase should use ordinary toolkit APIs, not special internal shortcuts.

## 32. Performance Requirements

The toolkit should be designed for crisp editor interaction.

Performance principles:

- Avoid continuous redraw when idle.
- Redraw immediately on input.
- Avoid blocking the UI thread.
- Cache text shaping and layout.
- Cache glyph resources.
- Virtualize large lists/tables/trees.
- Avoid per-frame full texture uploads.
- Avoid unnecessary allocation in hot paths.
- Batch or group render primitives where the backend benefits.
- Keep layout deterministic and simple.
- Expose profiling hooks early.

Performance targets should be validated in the showcase and tests:

```text
idle UI
  no unnecessary redraw loop

pointer hover
  visible feedback by next presented frame

dragging
  display-refresh-rate redraw where possible

large table
  only visible rows are measured/painted

viewport playback
  texture updates only when content changes

text fields
  typing should not reshape unrelated text
```

## 33. Implementation Workflow

The architecture should support phased issue-based PRs.

Phases describe implementation sequencing, not product scope judgment.

Recommended PR sequence:

```text
Phase 1: Repository and CI
  workspace crates
  formatting/lint/test/doc workflow
  basic crate boundaries

Phase 2: Core Geometry and DPI
  Rect, Point, Size, Vec2
  logical/physical conversion
  scale factor handling
  tests

Phase 3: Input and Frame Runtime
  UiInput
  begin_frame/end_frame
  UiMemory skeleton
  FrameOutput
  redraw requests

Phase 4: Widget Identity
  WidgetId
  ID stack
  scoped IDs
  duplicate detection
  tests

Phase 5: Measurement and Layout
  sizing rules
  row/column/stack/grid
  padding/align/spacer/separator
  measurement tests

Phase 6: Render Primitives
  primitive enum
  clipping/layer commands
  primitive snapshots
  renderer trait

Phase 7: Interaction Primitives
  hit_test
  pressable
  focusable
  draggable
  selectable
  pointer capture
  tests

Phase 8: Theme System
  base tokens
  semantic tokens
  component recipes
  default theme
  theme resolution tests

Phase 9: Basic Components
  label
  button
  icon button
  checkbox
  radio
  toggle
  slider
  panel

Phase 10: Text Subsystem
  cosmic-text integration boundary
  text layout cache
  single-line field
  numeric field
  search field
  text undo model

Phase 11: Platform Adapter
  winit integration
  window events
  input normalization
  cursor requests
  redraw scheduling

Phase 12: Vello Renderer
  render primitive translation
  text/glyph rendering path
  clipping/layers
  image support
  renderer smoke tests

Phase 13: Action System
  action descriptors
  shortcut routing
  action surfaces
  invocation queue
  command palette foundation

Phase 14: Overlays and Menus
  overlay layer
  tooltips
  popovers
  dropdowns
  custom menus
  context menus

Phase 15: DockArea, Frame, Panel
  DockArea layout model
  Frames
  passive Panels
  split/tab behavior
  frame focus

Phase 16: Collections
  virtual list
  table
  tree foundation
  selection model
  sorting/filtering hooks

Phase 17: Viewport Surfaces
  TextureId/TextureHandle
  viewport component
  fit/zoom/pan helpers
  coordinate conversion
  overlay drawing

Phase 18: Accessibility Semantics
  semantic node tree
  roles/states/labels
  action affordances
  adapter integration path

Phase 19: Showcase App
  component gallery
  editor layout
  viewport demo
  table stress
  action demo
  theme states

Phase 20: Performance and Debug Tools
  profiling hooks
  layout debug overlay
  hitbox/focus debug overlay
  repaint reason tracking
  cache metrics
```

Each PR should include tests for deterministic behavior when the implemented area is testable.

Agent-oriented issue templates should include:

```text
Goal
Relevant spec sections
Expected APIs
Non-goals
Tests required
Example usage
Acceptance criteria
```

## 34. Open Design Questions

These questions are intentionally left for later design decisions:

```text
Should native OS menus ever be supported as an alternate action surface?
How deep should built-in animation support go?
How should application-level undo integrate with text-field undo?
Which native accessibility backend should the winit handoff translate to first?
What exact icon format should be preferred?
How much rich text is needed beyond labels and text fields?
Should layout eventually include wrap/flex-like behavior?
```

Open questions should become focused design notes or issues before implementation reaches the affected subsystem.
