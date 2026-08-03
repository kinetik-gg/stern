# RFC 0001: Layout Engine

- Status: Draft
- Tracking: KNOWN-GAPS.md item 1 (framework pillar: layout engine)
- Related: KNOWN-GAPS.md items 5 (input re-localization cost) and 8 (flat widget API)
- Spec anchors: `docs/specs/02-layout-and-interaction.md` §11, `docs/specs/01-foundations.md` §6 and §10, `docs/specs/05-quality-workflow.md` Phase 5

This RFC proposes a design for stern's layout engine. It is a design document
only; no code changes accompany it. Every claim about current code cites
file:line as of the tree this RFC was written against; if a reference drifts,
trust the code (see KNOWN-GAPS.md, lines 14-17).

## 1. Goals and non-goals

Goals, in priority order:

1. **Determinism.** Same tree + same constraints + same text store contents
   must produce bit-identical rectangles, on every platform, every frame. This
   is what makes the existing exact-equality test style possible
   (`crates/stern-core/src/layout.rs:793-828` asserts exact `Rect` values) and
   what the conformance apparatus depends on.
2. **Window-free tests.** The full measure→arrange path must run headless with
   no window, GPU, or platform services, matching AGENTS.md ("Core tests must
   not require a window, GPU, or platform services", AGENTS.md:107) and the
   existing `Ui::begin_frame`/`end_frame` test idiom.
3. **Editor density.** The consumers are docked panels, property grids,
   toolbars, virtual tables — fixed-height rows, pixel-snapped splitters,
   ellipsized labels. The engine must make *those* cheap and exact, not
   arbitrary documents.
4. **Content-driven sizing.** `SizeRule::Fit` must finally mean "the widget's
   real measured content," including shaped text via `stern-text`, not a
   caller-invented `Measurement` (today's reality, §2).
5. **Same-frame geometry before behavior.** The closed-world pointer plan
   requires final geometry for every interactive target before the first
   routed behavior call (`crates/stern-core/src/runtime/ui.rs:428-447`). The
   layout engine must be able to hand the plan solved rectangles in the same
   frame, with no frame lag.

Non-goals:

- **Not a web-flexbox clone.** The spec is explicit: "Layout code should avoid
  web-like CSS complexity unless a feature is explicitly needed"
  (`docs/specs/02-layout-and-interaction.md:94`). No `flex-basis`, no
  `min-content`/`max-content` keyword algebra, no CSS block formatting
  contexts.
- **Not a retained widget tree.** stern stays immediate-mode; the layout tree
  proposed here is frame-local, with an optional retained *cache*, not a
  retained scene graph.
- **Not a dock/table rewrite.** `solve_dock_layout`
  (`crates/stern-widgets/src/dock/layout.rs:71`) and
  `VirtualTable::prepare` (`crates/stern-widgets/src/collections/virtual_table.rs:435`)
  are already pure, deterministic, domain-specific solvers. They remain layout
  *owners* that consume an outer rect; the engine does not subsume them.
- **Not multi-window.** `UiMemory` is single-window (KNOWN-GAPS item 11); this
  RFC does not change that.

## 2. Current state

### 2.1 The "layout module" is a rect-splitting calculator

`crates/stern-core/src/layout.rs` (855 lines including tests) contains the
whole engine today:

- `SizeRule` with `Fixed/Fit/Fill/Percent/MinMax/AspectRatio`
  (`layout.rs:16-34`) — the vocabulary the spec promised
  (`docs/specs/02-layout-and-interaction.md:48-57`).
- `Measurement { desired: Size }` (`layout.rs:118-122`) — but nothing produces
  it. Callers hand-construct measurements; `Fit` resolves against whatever
  number the caller supplied (`layout.rs:53`).
- `row_layout`/`column_layout` (`layout.rs:368-376`) delegate to
  `linear_layout` (`layout.rs:547-612`), a single forward pass: reserve
  non-fill sizes, divide the remainder among `Fill` items, walk a cursor.
  `grid_layout` (`layout.rs:385-444`) does the same per track. There is no
  second pass, no re-measurement under the resolved size, no minimum
  propagation (a `Fit` child that doesn't fit simply overflows), no baseline
  concept, and no cache.
- Free-function splitters `split_leading`/`split_trailing`/`pad_rect`/`fit_box`
  (`layout.rs:245-356`) are the tools consumers actually reach for.

`stern-widgets` wraps these one-to-one: `Ui::row/column/grid/padding/stack`
(`crates/stern-widgets/src/ui/layout.rs:15-97`) take a caller-computed `rect`
plus caller-supplied `&[LayoutItem]` and immediately materialize child rects.
The wrapper adds ID scoping, nothing else.

### 2.2 Every widget takes a caller-computed Rect

The widget `Ui` surface has 137 `pub fn` methods across
`crates/stern-widgets/src/ui/*.rs` (+`ui/chrome/`), of which 126 take a
`rect: Rect` parameter (counted on this tree; KNOWN-GAPS item 8 says 135-150).
Representative signatures:

- `button(key, rect, text, disabled)`
  (`crates/stern-widgets/src/ui/basic_controls.rs:66-72`).
- `checkbox_with_label_target(key, rect, label_rect, label, checked, disabled)`
  (`crates/stern-widgets/src/ui/choice_controls.rs:131-139`) — *two* positional
  rects, plus a trailing positional bool.
- The combinatorial families (`checkbox`/`checkbox_value`/`checkbox_with_label`/
  `checkbox_value_with_label`/`checkbox_with_label_target`/
  `checkbox_value_with_label_target`, `choice_controls.rs:64-167`) exist mostly
  because rect-threading and positional flags cannot compose.

Text flows *backwards* through this API: `button` first receives its rect,
then derives the label width from it and asks the text store for an ellipsized
layout (`basic_controls.rs:81-93`). Content never influences size; size
truncates content.

### 2.3 Consumers hand-place everything

`apps/stern-demo/src/edit_workspace.rs` is the honest picture of consumer
life:

- `WorkspaceLayout::new` hard-codes the chrome bands: menu at y=0 h=28,
  toolbar at y=28 h=32, tabs at y=60, dock from y=88 to height-24
  (`edit_workspace.rs:781-795`).
- Panel content rects are derived by arithmetic on dock output:
  `panel_bounds(...).map(|rect| rect.inset(8.0))` (`edit_workspace.rs:249-257`),
  `inspector_component_bounds` splits by a hard-coded 146-pixel grid height
  (`edit_workspace.rs:921-933`), the picker popover is a hard-coded 260×164
  rect (`edit_workspace.rs:996-1002`), the overlay help button is
  `Rect::new((bounds.width - 120.0).max(0.0), 4.0, 112.0, 20.0)`
  (`edit_workspace.rs:540-542`).
- Crucially, all of this geometry is computed *before* widget composition so
  it can be fed to `declare_workspace_targets` →
  `ui.resolve_pointer_targets(...)` (`edit_workspace.rs:453-538`), and then
  the *same* rects are re-used in `compose_workspace_panels`
  (`edit_workspace.rs:545-628`). The demo already lives in a
  prepare-geometry → declare-targets → compose-widgets pipeline; it just
  computes the geometry by hand.

The larger widgets have internalized the same shape: `prepare_virtual_table`
solves all header/row/cell geometry into a frozen `VirtualTable` before any
behavior runs (`crates/stern-widgets/src/ui/virtual_table.rs:28-36`), and
`DockScene::new` + `dock_controller`/`dock_scene` split solving from
evaluation/painting (`crates/stern-widgets/src/ui/dock.rs:61-73,561-566`).

### 2.4 Runtime facts the engine must respect

- **Frame lifecycle.** `Ui::begin_frame(context, memory)` snapshots and
  validates input (`crates/stern-core/src/runtime/ui.rs:60-128`); widgets push
  primitives; `end_frame` reconciles focus/semantics and returns a
  deterministic `FrameOutput` (`ui.rs:777-827`). There is no phase between
  begin and the first widget call where geometry could be solved today — the
  engine has to create one, or piggyback on the existing consumer-side
  "prepare" phase.
- **Spatial scopes.** `push_primitive` feeds every primitive through
  `SpatialStack::observe_primitive` and then re-localizes the *entire* input
  snapshot via `refresh_scoped_input` (`ui.rs:551-555`, `843-866`), which
  clones the full `UiInput` (`crates/stern-core/src/runtime/spatial.rs:45`)
  — the `O(primitives × events)` cost of KNOWN-GAPS item 5. Clip/transform
  scopes are declared by primitives (`ClipBegin`/`TransformBegin`,
  `spatial.rs:19-36`), i.e. by widgets, not by layout.
- **Pointer plan.** `resolve_pointer_targets` may run once per frame and must
  be complete "before the first routed behavior call" (`ui.rs:428-447`); an
  unplanned route allows every widget under the point
  (`crates/stern-core/src/memory.rs:24-31`, KNOWN-GAPS item 9). Plans are
  declared in screen-space rects captured at declaration time
  (`ui.rs:469-476` snapshots the current `SpatialStack`).
- **Retained state precedent.** `UiMemory` already retains per-widget maps
  (`scroll_offsets`/`pending_scroll_offsets`,
  `crates/stern-core/src/memory.rs:199-200`) with a staged
  commit-at-end-of-frame pattern (`memory.rs:239-245`). The spec explicitly
  reserves "layout cache … measure cache" slots in `UiMemory`
  (`docs/specs/01-foundations.md:630-632`).
- **Text measurement.** `stern-text` can already answer "how big is this
  text": `CosmicTextEngine::shape_text` returns a `ShapedTextLayout` whose
  `size` is the measured extent (`crates/stern-text/src/engine.rs:35-178`,
  result at `172-177`), and `TextLayoutStore` caches shaped layouts with a
  strict 32 MiB payload bound and 120-generation idle eviction
  (`crates/stern-text/src/store.rs:13-14`), exposing both cached
  (`try_layout_id`, `store.rs:181-188`) and transient (`shape_transient`,
  `store.rs:176-178`) entry points. Widgets already hold
  `Option<&mut TextLayoutStore>` (`crates/stern-widgets/src/ui.rs:56-61`).
  Measurement requires `&mut` access (cosmic-text shapes against a mutable
  `FontSystem`), which constrains where the measure pass can run.
- **Spec promise.** §11 promises "top-down and measurement-aware" layout with
  available size → desired size → final rect per widget
  (`docs/specs/02-layout-and-interaction.md:9-22`), and the frame-lifecycle
  narrative promises `ui.button("Analyze")` with "2. Measure desired size.
  3. Allocate final rect." (`docs/specs/01-foundations.md:334-335`). Phase 5
  of the implementation workflow is this work
  (`docs/specs/05-quality-workflow.md:293-297`).
- **Determinism footnote.** `WidgetId` currently hashes with
  `DefaultHasher` (KNOWN-GAPS item 13). Any retained layout cache keyed by
  `WidgetId` inherits that in-process-only stability; fine for a per-session
  cache, unacceptable for persisted layout state.

## 3. Candidate models

All three candidates keep the public `SizeRule` vocabulary
(`layout.rs:16-34`) — it is spec-blessed and already exported. They differ in
who computes `Measurement` and when rects become final.

### 3.a Adopt taffy

Embed [taffy](https://github.com/DioxusLabs/taffy) (0.12.2 as of 2026-07-15,
MIT license, ~16K SLoC plus required deps `arrayvec`, `grid`, `slotmap`) as
the constraint solver behind a stern-owned facade. Taffy implements CSS
Block, Flexbox, and Grid, supports measure-function leaves (a closure that
returns a size given known/available dimensions — the natural hook for
`TextLayoutStore`), keeps a per-node layout cache in its retained
`TaffyTree`, and is proven in Servo, Blitz, Bevy, Slint, Zed's GPUI, and
Floem (Lapce). One honest correction to the prompt for this RFC: **iced does
not embed taffy** — iced maintains its own `layout::Node` solver, so the
"everyone serious outsources layout" precedent is weaker than it first looks;
the strong precedents are Bevy/Zed/Slint/Blitz.

The cosmic-text precedent cuts both ways. stern-text adopted cosmic-text
rather than homegrown shaping (`crates/stern-text/Cargo.toml`,
`cosmic-text = "0.19.0"`), and that was correct: text shaping is a
domain where correctness is unownable. But stern-core is currently
**dependency-free** (`crates/stern-core/Cargo.toml` has no `[dependencies]`
section at all), and cosmic-text lives in a subsystem crate, not the core.
Putting taffy in stern-core ends the zero-dep property; putting it in a new
`stern-layout` crate keeps core clean but forces the widget `Ui` to
coordinate two runtimes.

Integration cost is the real issue, not the dependency. Taffy's tree is
retained and keyed by `NodeId` slotmap handles; stern is immediate-mode with
`WidgetId`s derived per frame (`ui.rs:413-424`). To benefit from taffy's
cache you must retain the `TaffyTree` across frames and *diff* it (map
`WidgetId`→`NodeId`, detect added/removed/re-styled nodes each frame);
rebuild-per-frame throws the cache away and pays slotmap allocation per node
per frame. Either way stern writes and owns a reconciliation layer, and
stern's `SizeRule` semantics must round-trip through CSS vocabulary
(`Fit` → auto + max-content behavior, `Fill` → flex-grow, `Percent` →
percentage, `MinMax` → min/max size properties) — a mapping whose edge cases
(what taffy does at zero available space, gutter rounding, flexbox min-size
clamping) become stern's bug surface while remaining outside stern's control.

| Constraint | Assessment |
| --- | --- |
| Determinism | Good: pure f32 arithmetic, no platform calls. But algorithm changes ride in on version bumps; exact-rect golden tests break on upgrade for reasons stern cannot review line-by-line. |
| Immediate-mode fit | Poor-to-fair: retained tree needs per-frame diffing or per-frame rebuild; both are stern-written machinery comparable in size to a small solver. |
| Testability | Good: solver runs headless; taffy has its own conformance suite (generated from Chrome behavior — i.e., web semantics, which stern explicitly does not want to promise). |
| Performance | Good solver; cache only pays off with retained-tree diffing. Per-frame rebuild is `O(nodes)` slotmap churn. |
| Migration cost | High up front: facade + reconciliation + CSS mapping land before the first widget benefits. |
| Spec alignment | Weak: §11's "avoid web-like CSS complexity" (`02-layout-and-interaction.md:94`) is hard to square with shipping a CSS engine, even hidden. |

### 3.b Homegrown measure→arrange two-pass over a frame-local layout tree

Build the engine stern's spec describes: a frame-local tree of layout nodes
(`Row`, `Column`, `Stack`, `Grid`, `Padding`, `Align`, plus leaf nodes that
carry a content measurer), solved in two passes:

1. **Measure (bottom-up).** Each node reports desired size under given
   available space. Leaves answer from intrinsic knowledge: text leaves call
   `TextLayoutStore` (`store.rs:181-188`), icon/control leaves answer from
   theme metrics (`theme.sizes`, `theme.controls.padding_x` — the same values
   `button` already uses at `basic_controls.rs:84-86`). Containers combine
   children per `SizeRule`.
2. **Arrange (top-down).** Parents allocate final rects using the existing
   `linear_layout`/`resolve_tracks` arithmetic (`layout.rs:477-612`), which
   is kept as the per-container distribution kernel — its behavior is already
   pinned by tests (`layout.rs:793-845`).

The tree is *declared* before widget behavior runs and *consumed* during
composition. This is not a new phase invented for layout — it is the
formalization of the prepare/declare/compose pipeline the codebase already
converged on (§2.3): `edit_workspace.rs` computes geometry, declares pointer
targets, then composes; `VirtualTable::prepare` and `DockScene::new` do the
same per-widget. The engine gives that idiom a shared vocabulary and takes
over the by-hand arithmetic.

Concretely (API sketch, non-normative):

```rust
let tree = ui.layout(bounds, |l| {
    l.column(gap(4.0), |l| {
        l.fit(Button::new("Analyze"));           // leaf measurer: text + padding
        l.fill(PanelSlot::new("inspector"));     // handle for later composition
    })
});
// tree.rect(handle) is final, screen-space, this frame
ui.resolve_pointer_targets(|plan| tree.declare_targets(plan, ...));
tree.compose(ui, ...);                            // widgets receive solved rects
```

A retained cache keyed by `WidgetId` lives in `UiMemory` alongside
`scroll_offsets` (the slot the spec reserved, `01-foundations.md:630-632`):
`WidgetId → (constraint_key, measured Size)`. `constraint_key` hashes the
inputs that can change the answer (content hash, style, available-width
bucket). On hit, the measure pass skips the leaf (and, transitively, an
unchanged subtree); text stays additionally cached in `TextLayoutStore`,
which already makes repeat shaping cheap and bounded.

| Constraint | Assessment |
| --- | --- |
| Determinism | Strong: stern owns every f32 operation; algorithm changes are stern PRs with golden-test diffs. |
| Immediate-mode fit | Strong: frame-local tree, no diffing; retained state is a flat cache map using an existing `UiMemory` pattern (`memory.rs:199-200,239-245`). |
| Testability | Strong: solver is a pure function `(tree, bounds) -> rects`; exact-equality tests like today's (`layout.rs:793-828`) extend directly. |
| Performance | Two `O(nodes)` passes per frame plus tree construction; needs an arena (index-based `Vec<Node>`) to avoid per-node boxing. Measure cache bounds the text-shaping cost. |
| Migration cost | Incremental: the distribution kernel already exists; each widget migrates independently (§5). |
| Risk | stern owns the edge cases (Fit-overflow, MinMax interaction with Fill, rounding). Scope control is the mitigation: the node set is §11's list, not CSS. Baseline alignment and wrapping rows are explicitly deferred (§8). |

### 3.c egui-style enhanced single-pass

Keep one pass: `Ui` gains a cursor/allocator; `Button::new("x").show(ui)`
measures its own content (via `TextLayoutStore`), requests that size from the
current region, and receives a rect immediately. Containers size themselves
best-effort: a region's extent is known only *after* its children ran, so
anything that depends on sibling or descendant sizes (right-alignment,
`Fill` sharing with `Fit` siblings, centered groups) either needs the caller
to pre-compute sizes (status quo) or uses the previous frame's remembered
extents and stabilizes over 1-2 frames, as egui does.

This is the cheapest path to the builder ergonomics of KNOWN-GAPS item 8, and
for purely top-down flows (toolbars of fixed-size buttons, stacked panels) it
is fine. It fails on stern-specific grounds:

- **The pointer plan forbids frame-lag geometry.** Audited surfaces must
  install a complete plan before the first behavior call (`ui.rs:428-447`;
  `docs/specs/02-layout-and-interaction.md:357-359`). In single-pass, a
  widget's final rect is discovered *during* emission — after the plan is
  installed. The only ways out are declaring the plan from last frame's rects
  (hit-testing against stale geometry — exactly the class of bug the
  closed-world plan exists to kill) or forbidding planned targets inside
  ui-allocated regions (which makes the new API unusable for precisely the
  overlapping-surface cases that need the plan).
- **Second-frame stabilization breaks the test posture.** stern's tests
  assert exact output for a single composed frame; a model whose geometry is
  only correct on frame N+1 forces every layout test to run warm-up frames
  and every conformance claim to say "eventually".
- **It duplicates state.** The "remembered region extents" are a retained
  layout cache in disguise — the same `UiMemory` map as option (b), but
  holding *stale* data that the design then has to apologize for.

| Constraint | Assessment |
| --- | --- |
| Determinism | Weak: correct only at fixpoint; one-frame jitter on content change is observable in `FrameOutput`. |
| Immediate-mode fit | Strong: it is the classic IMGUI shape. |
| Testability | Weak: warm-up frames required; golden tests capture transients. |
| Performance | Best: single pass, no tree. |
| Migration cost | Lowest per widget, but adds a *third* geometry idiom next to rect-first and prepare/compose without retiring either. |
| Pointer-plan fit | Disqualifying: stale-rect planning or plan-free regions. |

## 4. Recommendation

**Adopt (b): a homegrown measure→arrange two-pass over a frame-local layout
tree, with a retained measure cache keyed by `WidgetId` in `UiMemory`.**

Rationale, in order of weight:

1. **The pointer plan decides it.** Stern's one genuinely unusual runtime
   feature is the same-frame closed-world pointer plan (`ui.rs:437-518`).
   Option (c) is structurally incompatible with it (§3.c). Option (a) and (b)
   both satisfy it, but (b) satisfies it with machinery stern already has:
   the solved tree exports final screen-space rects exactly where
   `edit_workspace.rs:453-538` needs them today. The layout tree can also
   *derive* target declarations (a node flagged interactive knows its id,
   rect, and paint order), directly attacking the by-hand `PointerOrder`
   bookkeeping in `edit_workspace.rs:479-537`.
2. **Spec fidelity.** §11 describes available→desired→final with optional
   pre-placement measurement (`02-layout-and-interaction.md:9-22`) and warns
   away from CSS complexity (`:94`). (b) is that text implemented; (a) is a
   different engine wearing its vocabulary.
3. **Determinism ownership.** The conformance reset (KNOWN-GAPS.md preamble)
   is about stern being able to state exactly what it does. A solver whose
   behavior changes with `cargo update` works against that; the cosmic-text
   precedent is different in kind because text shaping output is treated as
   opaque content, while layout rectangles are stern's own asserted contract
   surface.
4. **The hard 20% is already written.** Distribution arithmetic
   (`layout.rs:477-612`), dock solving (`dock/layout.rs:71`), table geometry
   (`collections/virtual_table.rs:435`), text measurement with bounded
   caching (`store.rs:176-259`) all exist. What is missing is the tree, the
   measure pass, and the seam — which is also exactly the part taffy would
   *not* provide (§3.a's reconciliation layer).

Taffy is not rejected forever. The public API in §5 deliberately never
exposes solver internals (callers see builders, `SizeRule`, and solved
rects), so if requirements later grow to genuine flexbox territory —
multi-line wrapping with baseline alignment across mixed content — taffy can
be swapped in behind the arrange stage as an implementation detail, in a
`stern-layout` crate to preserve stern-core's zero-dependency property.

### Interaction with the pointer plan and spatial/clip scopes

- **Plan derivation.** The solved tree exists before
  `resolve_pointer_targets` runs; a helper walks nodes flagged
  `.interactive(id)` and declares `PointerTarget`s in paint order. Manual
  `plan.target(...)` remains for overlays and special cases. Because the plan
  snapshots the current `SpatialStack` at declaration
  (`ui.rs:469-476`), layout-derived declarations happen at root scope with
  screen-space rects — the same contract consumers implement by hand today.
- **Scope ownership moves to containers, gradually.** Clip and transform
  scopes are currently opened by widget code emitting
  `ClipBegin`/`TransformBegin` primitives (e.g. dock panel clips at
  `crates/stern-widgets/src/ui/dock.rs:590-599`, virtual-table header/body
  clip+translate at `virtual_table.rs:160-167,251-258`). Layout container
  nodes that clip or translate (ScrollArea, Panel-with-clip) will emit those
  same primitives from one place, which both removes per-widget duplication
  and creates the precondition for fixing KNOWN-GAPS item 5 (§6).
- **Coordinate discipline is unchanged.** The engine solves in the current
  scope's logical coordinates, same as every rect parameter today. Nothing
  about RT-01 localization (`spatial.rs:38-145`) changes semantically.

## 5. Widget API migration plan

Target end state per widget: a builder that measures, plus the rect escape
hatch:

```rust
ui.add(Button::new("Analyze"))                    // measured, ui-allocated
ui.add_at(rect, Button::new("Analyze"))           // editor chrome, dock panels
```

`*_at` is not legacy — dock panels, viewport overlays, and anything downstream
of `solve_dock_layout` legitimately have externally-owned rects forever.

Phases (each is a normal issue-sized PR series; the tree compiles throughout):

- **Phase L0 — solver, no consumers.** `stern-core`: layout tree types, arena,
  measure/arrange passes, `UiMemory` measure-cache map (staged like
  `pending_scroll_offsets`, `memory.rs:199-200`), exhaustive deterministic
  tests. Public but undocumented-as-stable. Nothing breaks.
- **Phase L1 — the seam + basic controls.** `stern-widgets`: `Ui::layout`
  entry point; `Widget` trait (`measure(&self, ctx) -> Measurement`,
  `compose(self, ui, rect) -> Response`); builders for `Button`, `Label`,
  `Checkbox`, `Toggle`, `RadioButton`, `Slider`, `IconButton` implemented
  over the existing `*_widget` free functions (which already take
  `(id, rect, …)` and stay untouched). Existing 137 methods delegate
  unchanged. Nothing breaks. The showcase gains one journey using the new
  path — the honesty check that the seam works under the pointer plan.
- **Phase L2 — text fields and containers.** Text-field builders (the
  hardest measurers: caret/selection interplay already isolated behind
  `*_with_text_layouts_and_caret_visibility`, `basic_controls.rs:44-57`);
  `panel`, `scroll_area`, `property_grid` rows accept `Fit` children;
  `Ui::row/column/grid` (`ui/layout.rs:15-97`) gain tree-backed equivalents.
  Nothing breaks yet.
- **Phase L3 — chrome and big surfaces.** `ChromeScene` measures its own item
  widths, replacing the caller-supplied width table
  (`edit_workspace.rs:797-824` currently hard-codes six widths); dock/table
  keep their solvers but their *contents* (tab labels, headers, panel bodies)
  become measured builders. `stern-demo` drops `WorkspaceLayout::new`
  arithmetic where the engine now answers.
- **Phase L4 — breaking cleanup (the point of item 8).** Delete the
  positional variants that builders subsume: the `*_with_label`,
  `*_with_label_target`, `*_value_with_label_target`, and `*_sized` families
  collapse into builder options (`Checkbox::new(...).label(...).value(&mut x)`).
  This is the breaking release: expected to remove roughly half of the
  137-method surface. Pre-alpha rules apply — no deprecation shims. `*_at`
  variants and the behavior layer (`ui/behavior.rs`) survive.

What breaks and when: nothing until L4; L4 breaks every call site of the
deleted variants (workspace-wide, mechanical rewrites in `apps/` and
showcase). Dock/table/viewport public APIs do not break in any phase.

## 6. Performance

Frame cost model, letting N = layout nodes, P = primitives, E = input events:

- Added per frame: tree build `O(N)` (arena push, no boxing), measure `O(N)`
  with cache hits skipping leaf work, arrange `O(N)`. For editor frames N is
  hundreds, not tens of thousands — virtualized surfaces (table, tree,
  asset grid) contribute O(visible) nodes because they stay self-solving
  (§1 non-goals).
- Measure cache: `WidgetId → (constraint_key, Size)` in `UiMemory`; committed
  end-of-frame like scroll offsets (`memory.rs:242-244`); evicted by
  seen-this-frame reconciliation, which `Ui::end_frame` already computes for
  widget owners (`ui.rs:811-817`). Text measurement itself is already
  bounded and LRU-evicted in `TextLayoutStore` (32 MiB / 120 generations,
  `store.rs:13-14`), so the `UiMemory` cache stores only the small
  `(key, Size)` pairs, not shaped payloads.
- Invalidation: `constraint_key` = FNV over (content hash, style token
  revision, available-main-axis bucket). Bucketing available width (e.g.
  quantize to 0.25 logical px) prevents float-drift misses without changing
  results (the arrange pass always re-solves exact rects; the cache only
  short-circuits *measurement*).

Interaction with KNOWN-GAPS item 5 (`O(primitives × events)` input cloning,
`ui.rs:551-555` → `spatial.rs:45`):

- **Where layout makes it worse.** Container nodes that clip/translate emit
  more `ClipBegin`/`TransformBegin`/`End` primitives than hand-rolled layouts
  that share one big clip. Every such primitive is another
  `refresh_scoped_input` → full `UiInput` clone. A naive "every container
  gets a clip node" default would multiply item-5 cost by the container
  depth. Mitigation is design policy: layout nodes emit spatial scopes only
  when semantically required (scrolling, actual clipping), never for plain
  rows/columns — those remain pure rect arithmetic with zero primitive
  overhead, exactly like today's `ui/layout.rs` helpers.
- **Where layout makes it better.** The item-5 fix everyone expects —
  memoize `LocalizedInput` per spatial state instead of per primitive —
  needs a stable identity for "this scope, this frame". Today scopes are
  anonymous push/pop side effects of the primitive stream
  (`spatial.rs:19-36`). Layout nodes give scopes an identity (the node id)
  *before* emission, so the runtime can compute localization once per
  scope-node and reuse it for every primitive pushed within, turning
  `O(P × E)` into `O(scopes × E)`. This RFC does not implement that fix but
  deliberately shapes the tree so the fix has somewhere to live; the item-5
  issue should land against the layout-node scope table, not against the
  anonymous stack.

## 7. Test strategy

1. **Solver unit tests (stern-core).** Extend the exact-equality style of
   `layout.rs:670-855`: for each node kind and each `SizeRule` pairing,
   assert exact solved `Rect`s, including the sanitization edges the current
   kernel pins (NaN, negative, inverted MinMax — `layout.rs:688-711`).
   Measure-pass tests use a fake measurer (no text dependency) so stern-core
   tests stay dependency-free.
2. **Text-backed measurement tests (stern-widgets).** Deterministic because
   fonts are bundled (`engine.rs:236-244`): assert that a `Button` builder's
   measured size equals padding + shaped `ShapedTextLayout.size` for known
   strings, and that cache hits do not change results
   (`store.rs` already proves observation-purity in its own tests,
   `store.rs:964-982`).
3. **Migration conformance guards.** For every widget migrated in L1-L3, a
   guard test composes the same control twice in one test — once via
   `add_at(rect, …)` legacy path, once via a layout tree solved to produce
   the *same* rect — and asserts identical `FrameOutput` primitives and
   semantic nodes. This pins "builders are a seam, not a fork" and runs
   window-free through the existing `Ui::begin_frame` harness. Guards are
   deleted with their legacy variant in L4.
4. **Pointer-plan integration tests.** A solved tree declaring targets must
   produce byte-identical routing to today's hand-declared plan for the same
   geometry (fixture modeled on `edit_workspace.rs:479-537`). Plus the
   negative case: a behavior call before plan installation still fails closed.
5. **Cache-semantics tests.** Measure-cache staleness must be *unobservable*:
   tests mutate content between frames and assert frame N output equals a
   cold-cache compose of the same frame. Eviction follows seen-reconciliation;
   test mirrors `reconcile_widget_owners` behavior (`ui.rs:811-817`).
6. **No warm-up frames anywhere.** Every layout assertion holds on the first
   composed frame; this is the standing regression tripwire against option
   (c) semantics creeping in.

## 8. Open questions for the owner

1. **Crate placement.** Solver in `stern-core` (keeps the seam next to
   `Ui`/`UiMemory`, keeps core dependency-free since the solver has no deps)
   vs a new `stern-layout` crate. **Default: stern-core module**, revisit
   only if the solver grows a dependency.
2. **Builder trait shape.** One `Widget` trait with
   `measure`/`compose` (object-safe, arena-friendly) vs per-widget inherent
   `show(ui)` methods without a trait. **Default: trait**, because the
   layout tree needs to store heterogeneous leaves.
3. **Scope ownership timeline.** Should `scroll_area` move onto layout nodes
   in L2 (bigger churn, unlocks the item-5 fix sooner) or stay widget-owned
   until after L4? **Default: L2**, it is the single highest-traffic clip
   emitter.
4. **Baseline alignment.** §11 lists no baseline contract but item 1 calls
   its absence a gap. Reserve a `baseline: Option<f32>` field on
   `Measurement` now (cheap, forward-compatible) or add it when a consumer
   exists? **Default: reserve the field in L0, implement no alignment.**
5. **L4 timing.** Delete the positional variants immediately after L3, or
   hold L4 until the showcase journeys all run on builders? **Default: gate
   L4 on showcase migration**, since the showcase is the honesty apparatus.
6. **Auto-derived pointer targets.** Opt-in per node (`.interactive(id)`),
   or automatic for every leaf that composes an interactive widget?
   **Default: opt-in**, preserving the plan's explicit-inventory character
   (`02-layout-and-interaction.md:338-359`) until we trust the derivation.
