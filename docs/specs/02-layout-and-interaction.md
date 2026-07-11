# Kinetik UI Specification: Layout And Interaction

This file is part of the Kinetik UI architecture specification. The canonical entrypoint is [../specs.md](../specs.md).

Contained sections: 11-14.

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
Dock
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
- `ScrollArea` lays content in a scrollable viewport, clips children, and owns
  the content translation. Virtualized children use the offset to choose their
  materialized range but emit content-coordinate rectangles without applying a
  second translation.
- `SplitPane` divides available space between two or more children.
- `Dock` manages editor Frames.
- `Frame` owns editor-region behavior and chrome.
- `Panel` is a passive content surface.

Layout code should avoid web-like CSS complexity unless a feature is explicitly needed.

## 12. Dock, Frame, And Panel

Editor-style applications need a hierarchy that separates workspace arrangement from content.

The hierarchy is:

```text
Dock
  -> Frame
      -> Panel
          -> Components
              -> Primitives
```

### 12.1 Dock

`Dock` is the layout manager for editor frames.

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

`Dock` controls where frames live and how much space they receive.

Interactive docking remains model-owned and deterministic:

```text
frame tab drag -> DockTabDrag -> DockDropTarget -> Dock mutation
splitter drag -> DockSplitPath + delta -> clamped split ratio
```

Splitters are addressed by `DockSplitPath`, and drop targets distinguish tab
merge from split insertion. These operations update the same dock tree that is
serialized by `Dock::snapshot`.

Dock topology queries should stay pure and windowless. Frame neighbor lookup is
derived from solved `FrameLayout` rectangles and supports left, right, up, and
down directions without mutating focus, shortcuts, actions, or dock state.
T-junctions and overlapping candidates use deterministic tie-breaking: nearest
edge distance, then greatest perpendicular edge overlap, then lowest raw
`FrameId`.

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

Panel vocabulary separates developer-declared panel kinds from open panel instances:

```text
PanelTypeId
  Stable ID for a panel kind such as Scene, Inspector, Viewport, Timeline, or Console.

PanelInstanceId
  Stable ID for one open instance of a panel type. `PanelId` remains supported
  as existing compatibility terminology for current dock callers.

PanelTypeDescriptor
  UI metadata for panel pickers, menus, palettes, tabs, and workspace policy.
```

`PanelTypeDescriptor` is toolkit-owned metadata only. It may describe title,
optional icon, category, singleton or multi-instance policy, default size,
allowed workspace contexts, dock placement hints, close/duplicate/future-float
affordance policy, and an optional application-owned default open action.

`WorkspaceSnapshot` wraps the existing `DockSnapshot` with
`PanelInstanceSnapshot` records. Validation must ensure each dock panel has a
matching panel instance, each panel instance references a known
`PanelTypeId`, panel instance IDs are unique, supplied panel type descriptors
are a deterministic set, and stale records are reported with structured
errors. The shell may carry an application-owned state key, but applications
still own panel content and state serialization.

Snapshot diagnostics are additive to restore validation. `DockSnapshot` and
`WorkspaceSnapshot` should expose structured diagnostics with stable codes and
typed context for frame IDs, panel instance IDs, panel type IDs, and split
paths where practical. Existing `Dock::restore`, `WorkspaceSnapshot::validate`,
and `WorkspaceSnapshot::restore_dock` error-return behavior remains compatible.
When a `PanelInstanceSnapshot` title drifts from the matching dock `Panel`
title, restoration remains allowed and diagnostics should report a warning.

Applications still own panel content, panel instance creation, action execution,
workspace persistence, and any domain-specific factories.

Panel policy helpers may combine descriptors, panel instance metadata, and
current Frame/Dock state to derive affordances or app-owned open, focus, close,
duplicate, and future-float requests. These helpers must stay pure: they do not
execute commands, create panels, remove panels, or create native windows.

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

Pointer capture remains authoritative outside the owner's original rectangle,
but never outside an active effective clip. A clipped captured owner receives
only button-release cleanup; it cannot hover, click, wheel, or emit drag
movement there. Transform and clip scopes localize all `Ui` input accessors,
while response rectangles stay local and semantic/debug/IME rectangles export
screen-logical geometry.

A primary gesture retains its press origin in the current logical scope. Net
displacement crosses the private drag threshold at four units inclusive and
then remains latched. The crossing update reports the full origin-to-current
displacement; later frames report only newly accumulated movement. A crossed
release never clicks, even after moving back. `pressable` uses the same latch
for click suppression but never becomes a domain drag; only `draggable` sets
`drag_source` and can produce a released source for drop targets. The retained
transaction records its gesture family. A composite resolves that family once:
numeric scrub fields resolve one DomainDrag response, then derive focus, caret,
and text-input behavior from that response without replaying pointer events.
Selection is isolated and cannot become or release a domain drag. Legacy
snapshot input starts a fresh press at the current position and does not
reinterpret that frame's aggregate pointer delta as post-press movement.

Text selection uses `Ui::captured_selection_gesture`, a visually neutral
capture seam that returns the common `Response` plus ordered Press, Move,
Release, and Cancel actions. Canonical actions retain their original root event
ordinal through transforms and clips; legacy snapshot actions have no ordinal.
Selection reports movement below the domain threshold and never publishes a
drag source. `Ui::claim_ordered_text_input_events` returns the single claimed
key, text, clipboard, modifier, IME, and focus stream with the same root
ordinals (or no ordinals for legacy synthesis). A field merges those events
with selection actions instead of parsing pointer input a second time.
Releases preserved outside an effective clip are cancellation-only, even when
their transformed point remains inside a larger widget rectangle. A canonical
release with no event-time position cannot click, cross a threshold, or drop.
Spatial localization preserves a same-frame outside cleanup edge when an
earlier accepted press created its potential owner. ReleaseAll survives every
spatial scope as a global ordered fence, even without a retained owner.
ReleaseAll and focus loss preserve earlier movement, wheel input, or a completed
drop while making later pointer transitions inert. An unrelated behavior defers
retained-owner cleanup so it cannot erase the owner's pre-fence output, and
focus cancellation never borrows a future event's position or click count.

Overlapping interaction uses a predeclared `PointerTargetPlan`. Each visual
target has one canonical identity, at most one ordinary event owner, at most
one drop owner, optional wheel ownership, and explicit cursor equivalents.
Only the exact event owner receives its route; equivalence cannot cause a
second press or click. Disabled, singular, or fully clipped declarations are
ineligible, allowing the next eligible painted target to win. A visual blocker
prevents ordinary, drop, and wheel fall-through, while a barrier also blocks
points outside its own rectangle.

`pressable`, `selectable`, `draggable`, `focusable`, context-menu, and tooltip
behavior use the ordinary route. Drop behavior with an active source uses the
drop route. `scrollable` uses ordinary routing for hover but the independent
wheel route for mutation. A planned draggable source must opt in with
`PointerTarget::domain_drag_source`; this makes target-first eligibility
explicit instead of speculating that every pressable is a drag source. For a
same-frame transaction the first causal press selects the ordinary owner, and
the first causal release supplies drop geometry. The source's declared
transform and clip validate the threshold and release before a target-first
commit can escape. Canonical drop commits fail closed without a matching plan;
empty-stream legacy drop behavior remains compatible. Other low-level
unplanned hover and press behavior remains available, while audited layered
components must install a complete plan before any behavior call.

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
5. Dock/editor context.
6. Global application actions.
```

Text fields must consume typing and text-editing shortcuts before global actions receive them.
