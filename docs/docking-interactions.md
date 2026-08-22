# Docking Interactions

Docking interactions are deterministic model operations in `stern-widgets`.
`Dock` owns frame arrangement, split ratios, tab merges, and split insertion.
`Panel` remains passive content metadata.

## Splitter Resize

Use solved splitter hit rectangles with the neutral draggable primitive. Feed the
drag delta back into the dock model:

```rust
use stern_core::{Rect, Vec2};
use stern_widgets::{Dock, solve_dock_splitters};

fn drag_splitter(area: &mut Dock, bounds: Rect, drag_delta: Vec2) {
    let splitters = solve_dock_splitters(area, bounds, 6.0);
    if let Some(splitter) = splitters.first() {
        area.resize_split(&splitter.path, bounds, drag_delta);
    }
}
```

`resize_split` clamps ratios to the split minimums and updates the same
serializable dock tree used by `Dock::snapshot`.

## Tab Drag And Drop

Frame chrome starts tab drags; panels do not own drag behavior:

```rust
use stern_core::Point;
use stern_widgets::{
    Dock, FrameId, PanelId, resolve_dock_drop_target, solve_dock_layout,
};

fn drop_tab(area: &mut Dock, bounds: stern_core::Rect, pointer: Point) {
    let Some(drag) = area.begin_tab_drag(FrameId::from_raw(2), PanelId::from_raw(3)) else {
        return;
    };

    let frames = solve_dock_layout(area, bounds);
    let Some(target) = resolve_dock_drop_target(&frames, pointer, FrameId::from_raw(9)) else {
        return;
    };

    area.drop_tab(drag, target);
}
```

Dropping in the frame center merges the panel into the target tab group.
Dropping near an edge inserts the panel as a new frame split adjacent to the
target frame. The operation preserves panel dismissible policy and remains
round-trippable through snapshots.

## Tab Strip Insertion

Tab drags can also reorder within their own frame or insert at a precise
position in another frame. Tab-strip targeting is pure, keyed by stable
identities, and takes priority over generic edge-split targeting whenever
the pointer is over a prepared frame's tab strip:

```rust
use stern_core::{Point, Rect};
use stern_widgets::{
    Dock, DockDropTarget, DockTabStripGeometry, FrameId, PanelId,
    dock_tab_strip_contains_point, resolve_dock_drop_target, resolve_dock_tab_strip_target,
};

fn drop_target(
    area: &Dock,
    strips: &[DockTabStripGeometry],
    frames: &[stern_widgets::FrameLayout],
    pointer: Point,
    source_frame: FrameId,
    panel: PanelId,
    new_frame: FrameId,
) -> Option<DockDropTarget> {
    let drag = area.begin_tab_drag(source_frame, panel)?;
    match resolve_dock_tab_strip_target(strips, pointer, drag) {
        Some(target) => {
            let (frame, anchor) = target.to_anchor_target(strips, drag)?;
            // One validation query guards preview and commit alike.
            area.tab_insertion_is_current(drag, frame, anchor)
                .then_some(DockDropTarget::Insert { frame, anchor })
        }
        // Over a strip but idle (the dragged tab's own slot): no affirmative
        // preview, and never fall through into an edge split.
        None if dock_tab_strip_contains_point(strips, pointer) => None,
        // Outside every strip: center-merge and edge-split keep resolving.
        None => resolve_dock_drop_target(frames, pointer, new_frame),
    }
}
```

Resolution is deterministic. Strips are scanned in slice order (dock-tree
order); within the source strip the slot counts remaining tabs whose center
is at or left of the pointer; hovering another frame's tab anchors before
that tab, and gaps or empty tails append (`anchor: None`). A slot equal to
the dragged tab's current position resolves to no target.

Commit reuses the same validation query: `Dock::drop_tab` routes
`DockDropTarget::Insert` through `Dock::apply_tab_insertion`, which refuses
no-op placements, missing anchors, missing frames, and removed source panels
without mutating the committed tree. Same-frame reorders preserve the active
panel identity; cross-frame insertions activate the moved tab like every
other docking move. Escape, capture loss, focus loss, disablement, source
removal, and stale target state clear transient drag state and leave the
last committed dock snapshot unchanged.

### Prerelease exhaustive-match migration

This change added public enum variants in a pre-alpha release:
`DockDropTarget::Insert { frame, anchor }` and
`DockScenePreviewKind::Insert`. Exhaustive matches over either enum must add
those arms or a wildcard to keep compiling; constructors and existing
variants are unchanged, so only match sites migrate. See
`docs/public-api-policy.md` for the provisional-API context.
