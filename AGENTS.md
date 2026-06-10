# AGENTS.md

This file gives coding agents project-specific guidance for working on Kinetik UI.

Follow [docs/specs.md](docs/specs.md) as the source of truth for architecture, terminology, and implementation sequencing.

## Operating Rules

- Preserve the separation between core runtime, widgets, rendering, platform integration, and application/domain logic.
- Keep `kinetik-ui-core` free of `winit`, `wgpu`, `vello`, OS APIs, and renderer-specific types.
- Do not make components the lowest-level abstraction. Extract reusable behavior primitives first.
- Do not hardcode visual behavior into interaction primitives.
- Do not put application/domain behavior inside UI widgets.
- Do not run heavy processing inside UI widget calls.
- Prefer deterministic, testable functions for layout, input state transitions, hit testing, focus, and action routing.
- Add focused tests for any core behavior that can be tested without a window or GPU.
- Use the spec terminology consistently: `WidgetId`, `UiInput`, `UiMemory`, `Response`, `Primitive`, `SemanticNode`, `Action`, `DockArea`, `Frame`, `Panel`, `ViewportSurface`.

## Architecture Constraints

The editor hierarchy is:

```text
DockArea
  -> Frame
      -> Panel
          -> Components
              -> Primitives
```

Responsibilities:

- `DockArea` arranges Frames.
- `Frame` owns docked/sub-window behavior such as active state, close/dismiss, resize, tab/merge, and focus.
- `Panel` is a passive content surface and must not decide docking, dragging, dismissal, merge behavior, or outer size.

Behavior primitives should be visually neutral:

```text
pressable
selectable
draggable
focusable
scrollable
text_editable
drop_target
tooltip_trigger
context_menu_trigger
```

Components should compose these primitives with layout and theme recipes.

## Action System

Actions are application-owned commands presented and dispatched by the UI.

The UI may display actions as:

- menu items
- toolbar buttons
- context menu entries
- command palette rows
- shortcuts
- inspector buttons
- timeline controls

The UI emits action invocations. The application executes them.

Do not duplicate command logic inside individual buttons, menus, and shortcuts.

## Rendering

Widgets emit backend-independent render primitives.

Renderers must consume primitives and resource handles, not component types.

The renderer boundary should allow Vello as the first 2D backend while keeping the core renderer contract replaceable.

Viewport, video, image-processing, and 3D content should flow through texture surfaces. The UI owns the viewport rectangle, interaction routing, coordinate conversion, and overlays. Domain renderers own texture production.

## Text

Treat text as a first-class subsystem.

Do not implement text fields as simple string drawing. Text fields need shaping, caret movement, selection, editing state, copy/paste integration, and local undo. Use the text subsystem boundary described in the spec.

## Testing Expectations

Prefer unit tests for:

- geometry and DPI conversion
- measurement and layout
- widget ID stability
- hit testing
- press/drag/focus state transitions
- shortcut routing
- action dispatch
- scroll clamping
- slider value mapping
- table/list virtualization
- theme token resolution
- primitive emission
- text editing state
- viewport coordinate conversion
- redraw scheduling

Core tests must not require a window, GPU, or platform services.

## Pull Request Shape

Work should be shaped as issue-based PRs.

Each issue should map to one spec-defined area or one explicitly bounded slice of a spec-defined area.

Prefer PRs that are small enough to review carefully, but complete enough to leave the touched subsystem coherent.

Do not create a PR that mixes unrelated architecture layers. For example:

- Do not combine `WidgetId` work with renderer backend work.
- Do not combine theme token work with winit event normalization.
- Do not combine table virtualization with text editing.
- Do not combine DockArea behavior with Vello rendering.

Good PR boundaries:

- geometry types and tests
- DPI conversion helpers
- `WidgetId` and ID stack
- row/column layout
- `pressable` behavior
- `draggable` behavior
- primitive enum and snapshots
- theme token resolution
- action descriptors and invocation queue
- virtual list visible range calculation
- viewport coordinate conversion

Each PR should include:

- the relevant spec section
- the issue goal
- the implemented API or behavior
- acceptance criteria
- tests for deterministic behavior
- examples when a shared usage pattern is introduced
- notes for any deliberate deviation from the spec

Do not bundle unrelated refactors with feature work.

If the issue reveals a spec gap, update the spec in the same PR only when the gap is directly required for the implementation. Otherwise, leave a note for a separate spec PR.

## Issue Workflow

Issue descriptions should include:

```text
Goal
Relevant spec sections
Expected API or behavior
Non-goals
Tests required
Examples required, if any
Acceptance criteria
```

Agents should follow the issue boundary. If implementation pressure suggests a larger change, stop and document the needed follow-up instead of silently expanding scope.

When implementing a spec phase, prefer this order:

```text
1. Add or update types.
2. Add deterministic tests.
3. Implement behavior.
4. Add examples or snapshots if the API is user-facing.
5. Run the relevant local checks.
6. Summarize the spec sections touched.
```

## Commit Message Format

Use Conventional Commits. Commit messages that do not follow this format must be corrected before review.

Format:

```text
<type>(<scope>): <summary>
```

Allowed types:

```text
feat
fix
docs
test
refactor
perf
build
ci
chore
style
revert
```

Recommended scopes:

```text
core
geometry
dpi
input
layout
ids
memory
interaction
actions
render
vello
winit
theme
widgets
text
overlay
dock
collections
viewport
accessibility
showcase
ci
docs
```

Examples:

```text
feat(ids): add widget id stack
test(layout): cover fill and fixed row sizing
docs(spec): clarify frame and panel responsibilities
ci(workflows): add workspace checks
fix(input): preserve pointer capture during drag
```

Use `!` for breaking API changes:

```text
feat(layout)!: rename SizeRule::FitContent to SizeRule::Fit
```

Commit summaries should be imperative, concise, and specific.

Do not use vague summaries such as:

```text
update stuff
fix things
misc changes
work
```

PRs with non-conforming commit messages are not review-ready.

When a PR contains multiple commits, each commit must follow Conventional Commits. Squash commits must also follow Conventional Commits.

## Release Policy

Follow [docs/release.md](docs/release.md).

Release rules:

- use SemVer
- use `vX.Y.Z` tags
- keep `CHANGELOG.md` updated
- use Conventional Commits as release input
- mark breaking changes with `!` or a `BREAKING CHANGE:` footer
- document migration notes for breaking shared API changes

Do not introduce release automation that conflicts with the release policy.

## Review Readiness

A PR is review-ready when:

- it compiles
- formatting passes
- clippy passes for touched crates
- all commits follow Conventional Commits
- deterministic tests are included
- shared APIs have example usage when appropriate
- architecture boundaries from the spec are preserved
- no unrelated files were reformatted or refactored
- no heavy work is introduced into UI widget calls
- no renderer/platform dependencies are introduced into `kinetik-ui-core`

Do not mark a PR review-ready if it knowingly violates these conditions.

## CI Expectations

Keep code compatible with the repository CI expectations:

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo build --workspace --all-features
cargo check --workspace --examples --all-features
cargo doc --workspace --all-features --no-deps
```

CI should run on pull requests and pushes to `main`.
