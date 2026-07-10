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
