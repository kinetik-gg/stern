# Rust UI Conformance Methodology

This policy defines how Kinetik UI may use external Rust UI toolkits as references for future tests, harness APIs, and renderer contracts.

## Authority

Local Kinetik documents are authoritative, in this order:

1. `AGENTS.md`
2. `docs/specs.md`
3. `docs/render-snapshots.md`
4. Runway task files for the active run

External references are subordinate. If an external observation conflicts with Kinetik terminology, crate boundaries, renderer strategy, semantic model, testing expectations, or component philosophy, Kinetik wins and the observation is rejected or rewritten as a Kinetik-owned invariant.

## Scope

First-wave references are Rust UI libraries and Rust UI test tooling:

- [egui_kittest](https://docs.rs/egui_kittest) and egui testing practice
- [kittest](https://github.com/rerun-io/kittest)
- [Masonry / Xilem](https://github.com/linebender/xilem) and [masonry_testing](https://docs.rs/masonry_testing)
- [Iced](https://book.iced.rs/architecture.html)
- [Slint](https://github.com/slint-ui/slint/blob/master/LICENSE.md), with license caution

Blender, Godot, Roblox Studio, Adobe apps, Houdini, and other product applications are out of scope for this first wave. They may be studied later for workflow and interaction patterns, but not for this Rust UI conformance baseline.

## Hard Bans

- Do not copy external source code, tests, fixtures, snapshots, generated artifacts, or API shapes.
- Do not visually clone another toolkit's component look, theme, spacing, icons, or examples.
- Do not add dependencies because a reference toolkit uses them.
- Do not use RUN-001 or first-wave policy work to implement harnesses, widgets, renderer code, source changes, or other behavior changes; implementation starts in later accepted Runway tasks.
- Do not vendor external repositories or generated test data.
- Do not make external references authoritative over Kinetik's spec.
- Do not use a license-ambiguous reference as implementation material.
- Do not turn first-wave conformance work into Blender or product-app UI research.

## Conversion Method

Every reference-derived task must use this chain:

```text
reference observation
-> Kinetik invariant
-> target subsystem
-> likely test/harness form
-> acceptance signal
-> non-adoption/license caution
```

### 1. Reference Observation

Record the source, observed date, and version when practical. Paraphrase behavior, harness shape, renderer practice, or test taxonomy. Keep the note small enough that it can be reviewed without consulting the external project.

Acceptable observations:

- A harness simulates pointer, keyboard, text, time, or accessibility events.
- A query API finds controls by role, label, state, or semantic affordance.
- A framework separates application state/update logic from view construction.
- A renderer test snapshots command streams or resource inventories before pixels.

Rejected observations:

- Source snippets.
- Exact API signatures to reproduce.
- Visual recipes or component styling.
- Behavior that exists only because of another toolkit's architecture and has no Kinetik invariant.

### 2. Kinetik Invariant

Rewrite the observation as a Kinetik-owned rule anchored in `docs/specs.md`.

Examples:

- A user-like click observation becomes: `pressable` emits one click after primary pointer down/up on the same active widget.
- A semantic-query observation becomes: `SemanticTree -> AccessibilitySnapshot` preserves role, label, bounds, enabled state, focus state, parent/child relationships, and action affordances.
- An action-architecture observation becomes: widgets emit `ActionInvocation`; the application executes actions.
- A renderer-snapshot observation becomes: backend-neutral resource snapshots and Vello command snapshots prove primitive translation before any pixel test is considered.

### 3. Target Subsystem

Choose one subsystem. Do not mix unrelated layers in one task.

Common targets:

- `kinetik-ui-core`: geometry, DPI, input, IDs, memory, layout, interaction, actions, semantics, redraw scheduling.
- `kinetik-ui-widgets`: components composed from Kinetik behavior primitives, theme recipes, render primitives, and semantic nodes.
- `kinetik-ui-render`: backend-neutral primitives, resources, renderer contracts, deterministic resource snapshots.
- `kinetik-ui-vello`: Vello translation snapshots and backend diagnostics.
- `kinetik-ui-winit`: event normalization, platform requests, accessibility handoff, redraw scheduling.
- `apps/kinetik-ui-showcase`: later visual smoke coverage only, using ordinary public APIs.

### 4. Test Or Harness Form

Prefer windowless, GPU-free tests first.

Use these forms before pixel tests:

- Unit tests for finite geometry, DPI, layout, IDs, hit testing, interaction transitions, action dispatch, shortcut routing, virtualization, text editing state, viewport conversion, and redraw scheduling.
- Harness event simulation for pointer, keyboard, text, time, focus, and accessibility-like inputs.
- Semantic queries over Kinetik `SemanticTree` or `AccessibilitySnapshot`.
- Primitive snapshots for generated backend-independent draw lists.
- `RenderResources::snapshot()` for deterministic resource inventories.
- `kinetik-ui-vello::render_translation_snapshot` for Vello command streams.
- Structured renderer assertions when a command snapshot cannot prove the contract.

Pixel snapshots are last-resort showcase smoke coverage. They must not become the default conformance signal.

### 5. Acceptance Signal

Each conformance task must name a deterministic pass condition:

- exact state transition
- exact `Response` flags
- exact emitted `ActionInvocation`
- exact semantic node fields and traversal order
- exact normalized layout rects or visible ranges
- exact primitive order after normalization
- exact sorted resource inventory
- exact sanitized renderer command stream
- explicit structured warning or debug assertion for invalid input

The signal must be stable without a real window, GPU, OS accessibility service, wall-clock timing, unordered map iteration, raw font bytes, or raw pixel bytes unless the task explicitly justifies a later-stage visual smoke test.

### 6. Non-Adoption And License Caution

Every reference-derived task must say what is not being adopted.

Required cautions:

- egui / egui_kittest: use harness ergonomics as inspiration only; do not mirror egui immediate-mode API shape, visuals, or snapshot storage layout.
- kittest: use semantic-query goals as inspiration only; Kinetik queries must target Kinetik semantic snapshots and must not require adopting AccessKit in core.
- Masonry / Xilem: use headless harness and controlled-event ideas only; do not adopt retained-tree architecture, pass scheduling, or renderer internals.
- Iced: use state/message/update separation as an action-boundary reference only; do not force Kinetik widgets into Iced's application architecture.
- Slint: use only public test taxonomy or scenario ideas after license review. Slint's licensing includes GPLv3, royalty-free, and commercial options with platform-specific constraints; do not copy code, examples, generated artifacts, visual design, or license-dependent assets.

## Conflict Handling

- If a reference conflicts with `AGENTS.md`, `docs/specs.md`, or `docs/render-snapshots.md`, reject the reference or rewrite it into a compatible Kinetik invariant.
- If a behavior requires a window, GPU, platform service, or OS accessibility daemon, reduce it to a core invariant first. If reduction is impossible, mark it as a later platform or showcase smoke test.
- If a reference is visual style, reject it for first-wave conformance.
- If a proposed test would encourage copying another toolkit's API, rewrite it around Kinetik terminology: `WidgetId`, `UiInput`, `UiMemory`, `Response`, `Primitive`, `SemanticNode`, `Action`, `DockArea`, `Frame`, `Panel`, and `ViewportSurface`.
- If license status is unclear, stop and request legal/project-owner review before using the material.

## Runway Task Shape

Future Runway tasks using this methodology should include:

- source observation, version/date, and short paraphrase
- Kinetik invariant and authoritative spec section
- one target subsystem
- non-goals and explicit non-adoption
- exact test or harness form
- deterministic acceptance signal
- verification commands
- license note when the source is not clearly permissive for the intended use

Documentation and test tasks may cite external URLs, but implementation tasks must remain Kinetik-native and must not require network access to pass.
