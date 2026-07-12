# Alpha-Readiness Progress And Evidence

[Back to the alpha-readiness index](../alpha-readiness.md)

Stages 0-4 are Complete; Stage 4 is Complete / Accepted through `REND-02` squash merge `1239dd994619de3765d8cee05c5f8ddd34c2c6de`. Stage 5 is Current / Authorized with `REND-ADR-01` next. Stages 6-7 remain Authorized / Queued for continuous sequential execution without intermediate approval. Every remaining packet still has to pass its deterministic gates, and any Runway stop condition halts the active packet or stage.

Campaign workflow policy: `create-if-available` issues, `create-if-gates-pass` pull requests, and `squash-after-gates` merges. Tagging, package publishing, and an alpha release remain outside this authorization.

## Stage 0: Plan And Baseline

Status: Complete. This closed the documentation task only; Stages 1-4 subsequently completed and Stage 5 is Current / Authorized with `REND-ADR-01` next under the recorded campaign authorization.

### Changed files

- `docs/alpha-readiness.md`
- `docs/alpha-readiness/00-plan-and-baseline.md`
- `docs/alpha-readiness/01-truth-and-release.md`
- `docs/alpha-readiness/02-runtime-foundation.md`
- `docs/alpha-readiness/03-input-and-shell.md`
- `docs/alpha-readiness/04-text-renderer-lifetime.md`
- `docs/alpha-readiness/05-composition-foundations.md`
- `docs/alpha-readiness/06-editor-vertical-slice.md`
- `docs/alpha-readiness/07-quality-and-alpha-gate.md`
- `docs/alpha-readiness/progress.md`

### Reasoning and contract decisions

- Published a tracked canonical index plus split stages because local Runway state is not the durable human review surface.
- Preserved all 43 unique audit roadmap IDs; `API-01` remains one ID with provisional and final checkpoints.
- Kept semantic packet dependencies distinct from conservative Stage 0-7 campaign sequencing.
- Recorded root-owned contract zones, conditional leaf delegation, overlap exclusions, per-stage gates, and token checkpoints.
- Kept the current label at foundation/developer preview, closed Stage 0 as documentation-only, and recorded the current campaign status above.

### Tests run and results

- `git diff --check -- docs/alpha-readiness.md docs/alpha-readiness` — passed.
- Required-roadmap-anchor search across the index and split directory — passed.
- `git status --short -- docs/alpha-readiness.md docs/alpha-readiness` — passed and showed only the intended untracked index and stage directory.
- Supplemental ledger audit — passed with 43 unique roadmap IDs.
- Supplemental index-link audit — passed with nine local links and zero missing targets.
- No Rust source/test verification was in scope or claimed.

### Remaining risks and deferred findings

- At the Stage 0 checkpoint, runtime, input, text, presenter, component,
  quality, and release risks were unresolved. The authorized Stage 1-4 runtime,
  input, and text portions subsequently passed; presenter, component, quality,
  and release risks remain Stage 5-7 work.
- Timeline and node-graph packets remain deferred unless explicitly added to alpha scope.
- Native accessibility may remain a documented semantic-output-only boundary; floating native windows, broad multi-window behavior, additional renderers, and broader production persistence remain deferred.
- Packageability must not be interpreted as permission to tag, publish, or claim alpha readiness; pull-request merges follow the separate `squash-after-gates` campaign policy.

## Stage 1: Truth And Release Credibility

Status: Complete at `c8fbf536023fcd089c9afda1b9af789dd4dbbc20`.
The merged stage passed formatting, warning-denied Clippy, the full workspace
test suite, all-feature build, and example checks locally. The exact commit then
passed the warning-denied Windows, Linux, and macOS release matrix in
[CI run 29115608034](https://github.com/kinetik-gg/kinetik-ui/actions/runs/29115608034),
including documentation. The local Windows GNU rustdoc attempt was terminated
after reproducing its pathological allocation behavior; no warning policy was
weakened.

### `ALPHA-00`: capability truth contract

#### Changed files

- `apps/kinetik-ui-showcase/src/app/tests/navigation.rs`
- `crates/kinetik-ui-widgets/src/lib.rs`
- `crates/kinetik-ui-widgets/src/taxonomy.rs`
- `crates/kinetik-ui-widgets/src/taxonomy/{evidence,matrix,metadata,types,validation}.rs`
- `crates/kinetik-ui-widgets/tests/asset_browser_conformance.rs`
- `crates/kinetik-ui-widgets/tests/component_taxonomy_conformance.rs`
- `crates/kinetik-ui-widgets/tests/component_taxonomy_conformance/{capabilities,controls,inspector_contracts,matrix,stages}.rs`
- `docs/catalogue-conformance-matrix.md`
- `docs/showcase-plan.md`

#### Reasoning and contract decisions

Replaced binary completeness claims with Stable, Experimental, and Planned
status plus six independently required capability axes. Metadata-only
descriptors prove no capability axis; every current catalogue row remains
Experimental pending accepted behavioral evidence.

#### Tests run and results

- Taxonomy conformance: 42/42 passed; asset-browser conformance: 11/11 passed.
- Full repository source gates and warning-denied docs passed after one bounded
  depth-1 stale-assertion remedy.
- Independent audit critic: passed with no material finding.

#### Remaining risks and deferred findings

No production catalogue entry is Stable by design. Promotion remains an
evidence-backed curation decision. At the `ALPHA-00` checkpoint, all later audit
packets remained open; the remaining Stage 1 packets and Stages 2-4
subsequently passed, while Stages 5-7 and final `API-01` curation remain open as
documented.

### Provisional `API-01`: alpha surface policy

#### Changed files

- `crates/kinetik-ui/src/lib.rs`
- `crates/kinetik-ui/tests/public_api_surface.rs`
- `docs/public-api-policy.md`

#### Reasoning and contract decisions

Classified the facade and prelude as provisional Experimental. Advanced widgets
stay module-qualified and compatibility paths remain available until the final
post-`SHOW-02` API checkpoint.

#### Tests run and results

- Consumer surface tests: 5/5 with all features and 5/5 without defaults.
- Facade all-target/all-feature and no-default checks passed.
- Local non-doc gates passed; the exact-commit three-OS matrix supplied the
  authoritative warning-denied docs result.
- Independent audit critic: passed.

#### Remaining risks and deferred findings

Final facade curation, deprecations, migration notes, and the checked API
snapshot remain deferred to final `API-01`. Duplicate and legacy contracts are
still provisional.

### `REL-01`: packageable prerelease baseline

#### Changed files

- `Cargo.toml`, `Cargo.lock`, and `apps/kinetik-ui-showcase/Cargo.toml`
- `crates/kinetik-ui-{core,render,text,vello,widgets,winit}/Cargo.toml`
- `crates/kinetik-ui/Cargo.toml`
- `README.md`, `CHANGELOG.md`, `docs/release.md`, and `docs/crate-migration.md`

#### Reasoning and contract decisions

Established an unpublished `0.1.0-alpha.1` package baseline at Rust 1.89.
Seven libraries are packageable; internal dependencies use local paths plus
exact registry versions; the showcase is explicitly non-publishable. Source
use, packageability, publication, tagging, and alpha acceptance are distinct.

#### Tests run and results

- Metadata audit: 8 packages, 7 publishable libraries, 16 internal edges.
- Seven package archives, normalized inspections, and extracted builds passed
  in dependency order with ephemeral bootstrap patches.
- Full non-doc repository gates and independent audit critic passed; the
  exact-commit three-OS matrix closed warning-denied docs.

#### Remaining risks and deferred findings

The root lock retains yanked `swash 0.2.8`, while fresh archives resolve 0.2.9;
Stage 7 must update and re-audit the lock before publication. Rust 1.89 execution
also remains a pre-tag CI gate. No tag, publication, or release occurred.

### `SHOW-01A`: truthful navigation and CLI catalogue

#### Changed files

- `apps/kinetik-ui-showcase/src/app.rs`
- `apps/kinetik-ui-showcase/src/app/runtime/{chrome,lifecycle}.rs`
- `apps/kinetik-ui-showcase/src/app/tests/navigation.rs`
- `apps/kinetik-ui-showcase/src/artifacts.rs`
- `apps/kinetik-ui-showcase/src/main.rs`
- `docs/showcase-plan.md`

#### Reasoning and contract decisions

Established one public five-page catalogue for navigation and CLI tooling.
Editor navigation lives in a passive status rail so the editor's own menu and
toolbar remain authoritative. Strict parsing distinguishes absent, missing, and
unknown `--page` values before mode dispatch.

#### Tests run and results

- Navigation: 6/6; CLI: 4/4; artifact order: 1/1; editor menu/toolbar remedies:
  1/1 each.
- Showcase: 101 library plus 31 binary tests; full repository gates passed.
- Initial critic failed the editor placement; the depth-1 remedy passed final
  re-review.

#### Remaining risks and deferred findings

Compact click simulation and live-window visual QA remain absent. The rail can
obscure passive status text at very narrow widths.

### `SHOW-01B`: enabled-action truth

#### Changed files

- `apps/kinetik-ui-showcase/src/app.rs`
- `apps/kinetik-ui-showcase/src/app/runtime/{actions,components,systems}.rs`
- `apps/kinetik-ui-showcase/src/app/tests/{actions,components,editor,helpers,systems}.rs`
- `apps/kinetik-ui-showcase/src/editor/{models,root_state,showcase}.rs`
- `apps/kinetik-ui-showcase/src/editor/showcase/{core_chrome,menus}.rs`
- `apps/kinetik-ui-showcase/src/editor/tests.rs`
- `apps/kinetik-ui-showcase/src/editor/tests/{chrome_fixtures,toolbar_helpers}.rs`
- `docs/showcase-plan.md`

#### Reasoning and contract decisions

Unified labels, enabled state, semantic IDs, shortcuts, routing, and outcomes.
Every enabled touched action now has a distinct application-state effect;
unfinished entries are disabled Experimental surfaces. Play, Stop, Build, and
Export are distinct; Systems Save is explicitly an in-memory snapshot.

#### Tests run and results

- Focused action truth: 12/12; existing action filter: 19/19.
- Showcase: 117 library plus 31 binary tests and doc tests; full repository
  gates and visual render-once inspection passed.
- Initial critic passed with a low F5 risk; bounded tightening and final
  independent re-review passed with no finding.

#### Remaining risks and deferred findings

Direct internal calls can invoke idempotent Play while running, though all
current surfaces block it. Panel `OpenNew` needs re-audit if exposed. Save is
memory-only; several application operations remain disabled Experimental.

### `SHOW-01C`: retained About modal

#### Changed files

- `apps/kinetik-ui-showcase/src/editor/root_state.rs`
- `apps/kinetik-ui-showcase/src/editor/showcase.rs`
- `apps/kinetik-ui-showcase/src/editor/showcase/{core_chrome,menus}.rs`
- `apps/kinetik-ui-showcase/src/editor/tests.rs`
- `apps/kinetik-ui-showcase/src/editor/tests/{chrome_fixtures,interactions}.rs`

#### Reasoning and contract decisions

About owns retained application state and resolves modal input before editor
controls. A full-viewport guard prevents pointer click-through, app-specific
paint is emitted last, and open/close produce distinct observable outcomes.
At this checkpoint, Documentation remained disabled pending shell URL
execution; `IN-02` subsequently supplied that supported-shell path.

#### Tests run and results

- Focused About lifecycle: 5/5; showcase: 121 library plus 31 binary tests and
  doc tests.
- Full repository gates, exact seven-path scope, and independent critic passed.

#### Remaining risks and deferred findings

At this checkpoint, opening Documentation remained deferred to the input shell
packet and subsequently passed with `IN-02`. Keyboard focus trapping,
global-shortcut suppression, and dedicated modal/Close semantics remain
deferred to overlay and accessibility packets.

### `SHOW-01D`: per-frame Dock preview tabs

#### Changed files

- `apps/kinetik-ui-showcase/src/app/runtime/layout.rs`
- `apps/kinetik-ui-showcase/src/app/tests/layout.rs`

#### Reasoning and contract decisions

Replaced one shared panel-bottom tab origin with geometry derived per solved
frame from frame bounds, panel order, and panel count. IDs remain identity only.

#### Tests run and results

- Focused Dock preview: 3/3; showcase all-features: 127/127.
- Full repository gates, scope checks, and independent critic passed.

#### Remaining risks and deferred findings

Extremely narrow frames can produce zero-width tabs, and existing label drawing
is not clipped to the frame.

### `SHOW-01E`: state-derived Mass validation

#### Changed files

- `apps/kinetik-ui-showcase/src/editor/models.rs`
- `apps/kinetik-ui-showcase/src/editor/showcase/inspector_fixtures.rs`
- `apps/kinetik-ui-showcase/src/editor/tests/chrome_fixtures.rs`

#### Reasoning and contract decisions

Reused the numeric-draft classifier. Positive finite mass values are neutral;
zero, negative, non-finite, empty, and unparsable drafts are errors.

#### Tests run and results

- Focused Mass fixtures: 3/3; showcase all-features: 127/127.
- Full repository gates, scope checks, and independent critic passed.

#### Remaining risks and deferred findings

Validation can visually update on the next frame after an in-frame edit. The
exact-message assertion shares the production constant, so coordinated wording
drift still requires review.

### `SHOW-01F`: current-frame diagnostics

#### Changed files

- `apps/kinetik-ui-showcase/src/app/runtime/{chrome,systems}.rs`
- `apps/kinetik-ui-showcase/src/app/tests/systems.rs`

#### Reasoning and contract decisions

Diagnostics snapshot the current `Ui` output prefix before drawing themselves,
removing previous-frame retention while preserving frame-output ownership.

#### Tests run and results

- Focused current-frame diagnostics: 2/2; showcase: 119 library plus 31 binary
  tests.
- Full repository gates, lifecycle/source checks, and independent critic passed.

#### Remaining risks and deferred findings

Counts intentionally represent the prefix at the diagnostic call site rather
than final output including diagnostics emitted afterward.

## Stage 2: Runtime Foundation

Status: Complete / Accepted. `RT-01`, `RT-02`, and `RT-03` passed their bounded
critics and complete gates. The integrated Stage 2 gate passed at `5cf07b8`;
Stages 3-4 subsequently passed and Stage 5 is Current / Authorized with
`REND-ADR-01` next.

### `RT-01`: scoped coordinates and clipping

#### Changed files

- `crates/kinetik-ui-core/src/debug.rs`
- `crates/kinetik-ui-core/src/runtime.rs`
- `crates/kinetik-ui-core/src/runtime/{spatial,ui}.rs`
- `crates/kinetik-ui-core/tests/runtime_spatial_conformance.rs`
- `crates/kinetik-ui-widgets/src/collections/tree_layout.rs`
- `crates/kinetik-ui-widgets/src/inspector/layout.rs`
- `crates/kinetik-ui-widgets/src/ui/passive.rs`
- `crates/kinetik-ui-widgets/tests/runtime_spatial_conformance.rs`
- `apps/kinetik-ui-showcase/src/editor/showcase/{inspector_fixtures,panels}.rs`
- `docs/specs/01-foundations.md`
- `docs/specs/02-layout-and-interaction.md`
- `docs/alpha-readiness/progress.md`

#### Reasoning and contract decisions

Added one private runtime spatial stack that composes affine transforms and
exact convex clips for input, semantics, IME rectangles, debug bounds, and
primitive consumers. Every `Ui` input accessor now exposes current-scope
coordinates. Invisible or non-invertible scopes suppress pointer state except
the exact release edge needed to clean an existing primary-capture or
secondary-press owner; parent input is restored when the scope ends. Tree and
inspector virtualization now uses scroll only to select a materialized range,
while the runtime owns the sole content-to-viewport translation.

#### Tests run and results

- Core spatial conformance: 9/9 passed; widget spatial conformance: 5/5
  passed; showcase library: 123/123 passed.
- `cargo fmt --all -- --check` and `git diff --check` passed.
- Warning-denied workspace Clippy, all-feature workspace tests, build, and
  example checks passed.
- Warning-denied all-feature workspace documentation passed locally and
  generated all eight crate documentation sets.
- The implementation critic required two bounded remedies: emitted-geometry
  and nested focus/IME evidence at depth one, then inert pointer-edge and
  captured-cursor enforcement at depth two. Both independent re-reviews
  passed; the final core remedy suite was 9/9.

#### Remaining risks and deferred findings

Text primitives still do not expose dimensions for debug bounds; no incorrect
coordinate is reported, but any future text-geometry API must use the same
spatial resolver. Fully clipped semantic nodes retain zero bounds to preserve
valid parent-child trees while focusability and focus actions are removed.
Topmost/modal pointer arbitration and removed-owner reconciliation are
deliberately assigned to `RT-02` and `RT-03` rather than hidden in this packet.

### `RT-02`: topmost pointer-target arbitration

#### Changed files

- `crates/kinetik-ui-core/src/interaction/{hit,overlay,scroll}.rs`
- `crates/kinetik-ui-core/src/{lib,memory,runtime}.rs`
- `crates/kinetik-ui-core/src/runtime/{pointer,spatial,ui}.rs`
- `crates/kinetik-ui-core/tests/pointer_arbitration_conformance.rs`
- `crates/kinetik-ui-widgets/src/ui/{frame,passive}.rs`
- `crates/kinetik-ui-widgets/tests/pointer_arbitration_conformance.rs`
- `apps/kinetik-ui-showcase/src/editor/showcase/{core_chrome,menus}.rs`
- `apps/kinetik-ui-showcase/src/editor/tests/interactions.rs`
- `docs/specs/{01-foundations,02-layout-and-interaction,03-rendering-text-components}.md`
- `docs/alpha-readiness/progress.md`

#### Reasoning and contract decisions

Added one render-start, frame-local pointer plan with explicit paint order and
independent ordinary, drop, and wheel routes. Target resolution reuses the
RT-01 transform and exact-clip contract, fails closed for ambiguous plans, and
cancels stale or ineligible owners before routed behavior can observe release
edges. Planned scrolling freezes the current frame's offset and commits the
next offset at frame end so behavior evaluation order cannot move later hit,
paint, semantic, debug, or clip geometry. The low-level unplanned path retains
legacy immediate behavior for compatibility. Source-present drop responses now
authorize source identity, hover, and drop through the resolved drop route.
The showcase menu and modal install closed plans only while their overlays are
open, including background blockers and modal barriers.

#### Tests run and results

- Core pointer-arbitration conformance: 8/8 passed, including fully clipped
  top targets, removed owners, primary/secondary release-edge cancellation,
  and losing drop destinations exposing no source identity.
- Widget pointer-arbitration conformance: 1/1 passed for two-frame staged
  scrolling with frozen current-frame geometry.
- Showcase editor tests: 55/55 passed (69 filtered), including menu and modal
  click-through guards.
- `cargo fmt --all -- --check` and `git diff --check` passed.
- Warning-denied workspace Clippy, all-feature workspace tests, build, and
  example checks passed.
- Warning-denied all-feature workspace documentation passed locally and
  generated all eight crate documentation sets.
- The independent critic passed the depth-one remedy after verifying drop-route
  authorization and the clipped, removed-owner, and release-edge evidence.

#### Remaining risks and deferred findings

Unplanned low-level interactions intentionally preserve evaluation-order
compatibility; audited layered components must install a complete pointer plan.
Unique paint ordinals are a caller contract, although duplicate ordinals or
conflicting target IDs fail closed. Keyboard modal focus trapping remains an
overlay/accessibility packet. Owner disappearance in a frame with no installed
plan was deliberately assigned to and completed by `RT-03`. Planned scrolling
exposes its new offset on the next frame and therefore depends on the existing
repaint contract.

### `RT-03`: removed-widget ownership reconciliation

#### Changed files

- `crates/kinetik-ui-core/src/{identity,memory}.rs`
- `crates/kinetik-ui-core/src/runtime/ui.rs`
- `crates/kinetik-ui-core/tests/harness.rs`
- `crates/kinetik-ui-core/tests/ownership_reconciliation_conformance.rs`
- `crates/kinetik-ui-core/tests/focus_keyboard_conformance/support.rs`
- `crates/kinetik-ui-core/tests/pointer_arbitration_conformance.rs`
- `crates/kinetik-ui-core/tests/pointer_conformance/{drop_target,drag_capture}.rs`
- `crates/kinetik-ui-core/tests/runtime_spatial_conformance.rs`
- `crates/kinetik-ui-widgets/src/ui/frame.rs`
- `crates/kinetik-ui-widgets/tests/ownership_reconciliation_conformance.rs`
- `docs/specs/01-foundations.md`
- `docs/alpha-readiness/{02-runtime-foundation,progress}.md`

#### Reasoning and contract decisions

Separated frame-local widget presence from duplicate-registration accounting.
Normal IDs and scopes prove both presence and uniqueness, while semantic nodes
and evaluated text-input helpers can prove presence without manufacturing a
duplicate warning. At the `RT-03` checkpoint, derived IDs and the already
accepted `RT-02` pointer plans were treated as planning-only for presence
accounting.
At frame finalization, one missing capture, active, pressed, secondary-pressed,
or drag owner cancels the complete pointer transaction. Missing focus and
text/IME owners clear through the existing pending-stop path, which emits one
stop for removal and no repeated stop next frame. Cleanup requests repaint and
does not rewrite immediate responses or prune unrelated retained/application
state. Disabled, clipped, collapsed, and explicitly registered hidden widgets
remain present; their eligibility stays governed by RT-01/RT-02 behavior.

#### Tests run and results

- Core ownership reconciliation: 7/7 passed; widget-facade ownership
  reconciliation: 1/1 passed.
- Core pointer arbitration: 8/8 passed; core runtime spatial: 9/9 passed; core
  focus/keyboard: 36/36 passed.
- Widget pointer arbitration: 1/1 passed; widget runtime spatial: 5/5 passed;
  widget text-field conformance: 46/46 passed.
- Core library tests: 154/154 passed. `cargo fmt --all -- --check` and
  `git diff --check` passed; warning-denied all-target/all-feature Clippy passed
  for the touched core and widget crates.
- The depth-one fixture-only remedy made intended cross-frame presence explicit;
  harness tests passed 13/13 and pointer conformance passed 28/28 without
  production or expectation changes.
- The independent critic passed after that depth-one remedy. Full formatting
  and diff checks, warning-denied workspace Clippy, all-feature workspace tests,
  build, example checks, and warning-denied documentation for all eight crates
  passed.

#### Remaining risks and deferred findings

Standalone behavior functions used outside a `Ui` frame cannot participate in
end-frame reconciliation; framed custom widgets must register their identity.
At the `RT-03` checkpoint, presence deliberately did not define async
incarnation, cancellation-token, or tombstone policy; those responsibilities
remained `ASYNC-01` and subsequently passed. Ordered platform input and shell
execution were subsequently completed as Stage 3 work under the continuous
campaign authorization.

## Stage 3: Ordered Input And Shell

Status: Complete / Accepted at `1f991113816f3c6b8ce9063a9d37ebe367109f2c`.
`IN-01`, `IN-02`, `IN-03A`, and `IN-03B` all passed independent exact-SHA
audit, complete local workspace gates, remote CI, PR checks, and squash merge.
The final Stage 3 matrix was
[CI run 29140855335](https://github.com/kinetik-gg/kinetik-ui/actions/runs/29140855335);
PR [#517](https://github.com/kinetik-gg/kinetik-ui/pull/517) passed checks in
run 29141040177 and produced the accepted merge.
Stage 4 subsequently passed, and Stage 5 is Current / Authorized with
`REND-ADR-01` next.

### `IN-01`: ordered platform input

#### Changed files

- `apps/kinetik-ui-showcase/src/{live,app/runtime/lifecycle}.rs`
- `crates/kinetik-ui-core/src/{debug,input,lib,memory,test_harness}.rs`
- `crates/kinetik-ui-core/src/runtime/{spatial,types,ui}.rs`
- `crates/kinetik-ui-core/tests/{diagnostic_conformance,harness,input_ordering_conformance,runtime_spatial_conformance}.rs`
- `crates/kinetik-ui-text/src/{edit,lib,tests}.rs`
- `crates/kinetik-ui-winit/src/{input,tests}.rs`
- `crates/kinetik-ui-widgets/src/components.rs`
- `crates/kinetik-ui-widgets/src/components/numeric_inputs.rs`
- `crates/kinetik-ui-widgets/src/components/{text_fields,text_support}.rs`
- `crates/kinetik-ui-widgets/tests/text_field_conformance.rs`
- `crates/kinetik-ui-widgets/tests/text_field_conformance/{focus_and_clipboard,multiline_navigation,numeric_and_scrub,ordered_input,wrappers_and_path}.rs`
- `docs/specs/{01-foundations,03-rendering-text-components}.md`
- `docs/alpha-readiness/{03-input-and-shell,progress}.md`

#### Reasoning and contract decisions

Added one public ordered `UiInputEvent` stream whose official producers update
legacy projections atomically. Mixed canonical/direct mutation diagnoses once
and fails text editing closed, while an empty stream retains deterministic
legacy text-domain ordering. Winit now retains event-time pointer positions,
typed wheel provenance, ordinary hardware key text, and preedit-driven IME
composition separately from IME availability.

Text editing applies ordered commands, hardware text, IME, clipboard results,
and focus loss once through a frame-local owner claim. Spatial scopes transform
each pointer event independently, retain line units, transform pixel vectors,
and preserve only required release cleanup. Existing pointer primitives remain
snapshot-driven by design; `IN-03` owns event-aware pointer policy.

The initial critic found mixed-mode projection validation, scoped pointer
localization, numeric-wrapper rescanning, event-position evidence, and active
preedit insertion gaps. A depth-one remedy closed those findings. Re-review
then found one combined text-plus-pointer conflict that could still heal a
pointer snapshot; the depth-two remedy preserved canonical pointer evidence for
every root conflict, and the final independent re-review passed with no finding.

#### Tests run and results

- Initial focused packet commands: 231/231 passed across core input,
  diagnostics, spatial, focus/keyboard, harness, text, Winit, widget text
  fields, and the accepted RT-02/RT-03 regressions.
- Depth-one remedy commands passed 160/160 focused tests; the depth-two remedy
  passed 41/41 focused tests.
- `cargo fmt --all -- --check` and `git diff --check` passed.
- Warning-denied all-target/all-feature workspace Clippy passed.
- All-feature workspace tests passed 1,462/1,462; the workspace build and
  example checks passed.
- Warning-denied all-feature workspace documentation passed for all eight
  packages.
- Showcase all-feature compilation passed with live `KeyEvent.text` forwarding.
- Final independent critic passed at depth two with zero P0, P1, or P2 findings.

#### Remaining risks and deferred findings

Adding `UiInput.events` is source-breaking for unknown external callers that use
exhaustive public struct literals; in-repository literals were inventoried and
migrated, and the facade remains provisional Experimental. Empty-stream legacy
input cannot recover pointer order. At the `IN-01` checkpoint, the compatibility
`wheel_delta` still mixed line and pixel units, press/drag/click primitives
still consumed final snapshots pending `IN-03`, and shell request execution
remained `IN-02`; both successor responsibilities subsequently passed. At that
implementation-record checkpoint, the independent critic and complete local
gate were accepted while PR CI and squash merge were not yet claimed; they also
subsequently passed.

### `IN-02`: one-frame shell services

#### Changed files

- `Cargo.lock` and `CHANGELOG.md`
- `crates/kinetik-ui-core/src/runtime/{types,ui,tests}.rs`
- `crates/kinetik-ui-core/tests/{ownership_reconciliation_conformance,runtime_spatial_conformance}.rs`
- `crates/kinetik-ui-core/tests/focus_keyboard_conformance/text_lifecycle.rs`
- `crates/kinetik-ui-widgets/src/components/text_support.rs`
- `crates/kinetik-ui-widgets/src/ui/tests/text.rs`
- `crates/kinetik-ui-winit/Cargo.toml`
- `crates/kinetik-ui-winit/src/{input,lib,repaint,requests,shell,tests}.rs`
- `crates/kinetik-ui-winit/tests/shell_services.rs`
- `crates/kinetik-ui/tests/public_api_surface.rs`
- `apps/kinetik-ui-showcase/src/{app,live}.rs`
- `apps/kinetik-ui-showcase/src/app/runtime/{actions,lifecycle}.rs`
- `apps/kinetik-ui-showcase/src/app/tests/actions.rs`
- `apps/kinetik-ui-showcase/src/editor/root_state.rs`
- `apps/kinetik-ui-showcase/src/editor/showcase/{core_chrome,menus}.rs`
- `apps/kinetik-ui-showcase/src/editor/tests/chrome_fixtures.rs`
- `docs/specs/{01-foundations,04-runtime-platform}.md`
- `docs/{showcase-plan,alpha-readiness/03-input-and-shell,alpha-readiness/progress}.md`

#### Reasoning and contract decisions

Made `WinitPlatformRequests` a private-field, non-cloneable one-frame batch;
translation replaces prior state and applying to a window consumes it. Cursor
defaults actively, title is final/optional, IME Start/Update/Stop stays ordered,
and window application returns ordered shell work plus the sole repaint intent.
Clipboard and browser work uses injectable services with a retained native
clipboard, hardened HTTP/HTTPS-only opening, continued failure processing, and
payload-free diagnostics. Core `PlatformRequest` and `FrameOutput` debug output
also redacts external payloads before Winit translation. URL validation requires
a parseable HTTP(S) host and rejects malformed raw authorities. Targeted paste
responses enter the IN-01 stream once.

Repaint policy moved into a stateful Winit scheduler with replacement semantics,
response promotion, overflow safety, one-shot deadlines, and bounded Continuous
state. The live loop consumes external work before render, then always rolls
input and responses before scheduling, including recoverable surface errors.
Documentation is an application-owned fixed HTTPS action shared by Help, About,
and F1; widgets do not open browsers. The About Documentation control owns an
explicit pointer target and pressable route. Real Showcase frames traverse
injectable Winit cursor, IME, clipboard, URL, and repaint boundaries in tests.

#### Tests run and results

- Core all-feature suite: 346 passed, 0 failed before audit; the depth-one
  redaction-focused core run also passed.
- Winit all-feature suite: 42 passed, 0 failed.
- Widget all-feature suite: passed, including current-owner geometry updates.
- Showcase: 126 library plus 25 binary tests passed; Documentation source/F1
  tests and recoverable rollover passed.
- Qualified facade public surface: 5/5 passed without prelude promotion.
- Warning-denied focused Clippy across core, Winit, widgets, facade, and showcase:
  passed.
- Depth-zero independent audit: failed with 2 P1 and 2 P2 findings covering
  pre-translation debug redaction, About-control interaction, malformed URL
  authorities, and missing real Showcase-to-fake-Winit integration. All four
  have deterministic depth-one remedies; focused core, Winit (42/42), Showcase
  (128 library plus 25 binary), and warning-denied Clippy checks pass. Three
  independent exact-SHA re-reviewers passed with no P0/P1/P2 findings.
- `cargo package -p kinetik-ui-winit --allow-dirty --list`: passed and included
  both new modules plus integration tests. The direct archive attempt reproduced
  the accepted unpublished `kinetik-ui-core` registry bootstrap limitation.
- Complete workspace gate: formatting, warning-denied Clippy, all-feature tests,
  all-feature build, all-feature examples, and warning-denied documentation all
  passed on the audited code candidate.

#### Remaining risks and deferred findings

Native clipboard ownership, real browser launch, candidate placement, and OS
event-loop timing need desktop/three-OS smoke beyond deterministic fakes. Delayed
clipboard target reuse remains governed by the accepted `ASYNC-01` incarnation
policy. This is a provisional breaking Winit API change; migration is recorded
in the changelog. Direct archive creation still requires the Stage 1 ephemeral local
registry until internal crates are published. Independent audit and the local
full gate are accepted. Exact-SHA three-OS CI run `29134362277` and PR checks
passed; PR `#513` squash-merged as `e151b111` and issue `#512` is closed.

### `IN-03A`: wheel and click normalization

#### Changed files

- `CHANGELOG.md`
- `crates/kinetik-ui-core/src/{input,test_harness}.rs`
- `crates/kinetik-ui-core/src/interaction/scroll.rs`
- `crates/kinetik-ui-core/tests/wheel_click_normalization_conformance.rs`
- `crates/kinetik-ui-winit/src/{input,tests}.rs`
- `apps/kinetik-ui-showcase/src/live.rs`
- `docs/specs/{01-foundations,04-runtime-platform}.md`
- `docs/alpha-readiness/{03-input-and-shell,progress}.md`

#### Reasoning and contract decisions

Canonical scroll consumption now folds ordered wheel events without reusing the
mixed compatibility snapshot. Lines use a private 40-unit current-scope step;
logical pixels remain exact after the existing Winit DPI conversion and RT-01
spatial projection. Components, products, and accumulation sanitize to finite
values before direction inverts once. Empty streams retain legacy behavior, and
the ambiguous harness aliases now emit Pixels to preserve prior magnitude.

Winit retains a private click sequence across frames. Inclusive 500 ms and
four-logical-unit press boundaries increment with saturation; matching releases
carry, unmatched releases emit zero, and mismatch, missing evidence, backwards
time, pointer leave, focus loss, real sanitized scale change, or explicit input
clears continuation. A scale change also records pointer leave to invalidate
stale logical pointer evidence until the next move. The existing explicit-count
method remains exact and documents its automatic-history reset. The live showcase uses
`mouse_button_at(..., Instant::now())` instead of hardcoded one.

#### Tests run and results

- New wheel normalization conformance: 6/6 passed.
- Input ordering: 9/9 passed.
- Pointer arbitration: 8/8 passed.
- Scrollable pointer conformance filter: 2/2 passed.
- Ordered spatial localization filter: 1/1 passed.
- Core all-feature suite: passed, including all routing and spatial regressions.
- Winit all features: 45 unit plus 4 shell integration tests passed after the
  depth-one DPI-evidence remedy.
- Showcase all features: 128 library plus 25 binary tests passed.
- Warning-denied focused Clippy across core, Winit, and showcase: passed.
- Depth-zero audit found stale logical pointer evidence after a DPI change and
  incomplete public mouse-button lifecycle docs. Depth one records PointerLeft
  on a real sanitized scale change, clears private/projected evidence, documents
  exact reset/output semantics, and passes its focused regression and Clippy.
- Three independent depth-one re-reviewers passed with no P0/P1/P2 findings.
- Complete workspace gate: formatting, warning-denied Clippy, all-feature tests,
  all-feature build, all-feature examples, and warning-denied documentation all
  passed on the audited code candidate.

#### Remaining risks and deferred findings

The 40-unit line step is a private cross-platform default without momentum,
acceleration, overscroll, or gesture phases. Winit click sequencing has no
portable OS setting or widget identity. Drag threshold, release-click
suppression, canonical pointer transitions, and ordered selection ordinals stay
in serial packet `IN-03B`; no B-owned behavior is claimed here.

### `IN-03B`: drag threshold and ordered selection gestures

#### Changed files

- `CHANGELOG.md`
- `crates/kinetik-ui-core/src/{interaction,lib,memory}.rs`
- `crates/kinetik-ui-core/src/interaction/{drag_select,hit,overlay,press,scroll,tests}.rs`
- `crates/kinetik-ui-core/src/runtime/{pointer,spatial,tests,ui}.rs`
- `crates/kinetik-ui-core/tests/{drag_threshold_conformance,ownership_reconciliation_conformance,pointer_arbitration_conformance,runtime_spatial_conformance}.rs`
- `crates/kinetik-ui-core/tests/pointer_conformance/{drag_capture,drop_target}.rs`
- `crates/kinetik-ui-widgets/src/components/{numeric_inputs,text_fields}.rs`
- `crates/kinetik-ui-widgets/tests/component_taxonomy_conformance/controls.rs`
- `crates/kinetik-ui-widgets/tests/text_field_conformance/numeric_and_scrub.rs`
- `crates/kinetik-ui/tests/public_api_surface.rs`
- `docs/specs/{01-foundations,02-layout-and-interaction}.md`
- `docs/alpha-readiness/{03-input-and-shell,progress}.md`

#### Reasoning and contract decisions

Nonempty canonical pointer transitions now fold once in order; the empty stream
keeps legacy snapshot behavior. A private retained press origin and inclusive
four-current-scope-unit latch suppress clicks after crossing. The first domain
drag update reports full origin displacement and later frames report only new
movement. Pressable and selection behavior share suppression without becoming
drop sources; only `draggable` publishes active/released drag identity.

Spatial localization carries original root event indices in a private sidecar
owned by `Ui`, so public `UiInput`, `UiInputEvent`, and `Response` layouts remain
unchanged. `Ui::captured_selection_gesture` emits ordinal-bearing Press, Move,
Release, and Cancel actions, reports below-threshold selection movement, and
cannot replay actions for the same owner in one frame. Root conflicts block new
pointer/drop actions while ordered release/cancel evidence can clean an existing
owner. `Ui::claim_ordered_text_input_events` exposes the corresponding claimed
editing-domain events with the same original ordinals, so `TEXT-01` does not
need to parse the pointer stream.

The depth-one remedy retains cleanup-only release provenance, defers ordered
release-all/focus cancellation until preceding transitions are observable,
uses event-time release geometry for drops, rejects missing canonical button
positions, clears disabled secondary owners, blocks conflicted tooltip/scroll
hover, and prevents selection from publishing a retained domain drag.

The depth-one audit and complete workspace test exposed legacy pre-press delta
replay, same-frame clipped cleanup loss, mode replay, non-causal cancellation
metadata, and final-snapshot/multiple-release drop routing. The depth-two
candidate now resolves numeric scrub as one DomainDrag response, retains global
ReleaseAll fences through every scope, defers cancellation when an unrelated
behavior encounters another owner, and preserves pre-fence wheel/move/release
output. Closed plans require declared domain-drag source intent, derive
same-frame ownership from the first causal press, latch threshold evidence in
the source transform, validate source clipping, and route the matching causal
release. Canonical unplanned commits fail closed; empty-stream legacy drops
remain compatible. Final audit remedies keep first-release evidence immutable
across later same-frame transactions, cancel split primary/secondary ownership
per channel before raising a global fence, block planned releases after owner
mismatch, share one planner/primitive threshold predicate, and suppress passive
hover/cursor output after canonical focus loss without discarding pre-fence
wheel or drag input. Closed plans also expose threshold-crossed active sources
before release, ignore non-causal earlier releases, and prevent repeated
selection calls from replaying a direct cancellation. Active previews enforce
the same captured-source effective clip as release commits.

#### Tests run and results

- New drag-threshold conformance: 46/46 passed, covering boundaries, accumulated
  and subsequent deltas, move-back latch, same-frame release, pressable
  suppression, double-click, conflict cleanup, drop order, selection ordinals,
  spatial gaps and cleanup provenance, release-all cancellation, canonical drop
  geometry, ordered text merging, missing event positions, and plain-capture
  cleanup, plus legacy relocation, exact gesture modes, same-frame clipped
  ownership, unrelated-first cancellation, global fences, pre-fence wheel
  input, unplanned fail-closed drops, transformed target-first probes,
  same-frame press/release planning, immutable first-release authority,
  below-threshold first transactions, split button owners, owner-mismatch
  fail-closed routing, no-owner focus loss, active target-first drag hover,
  non-causal earlier releases, captured-source active clipping, no-replay
  cancellation, and release-time plans.
- Widget component-taxonomy conformance: 44/44 passed, including canonical
  accumulated scrub crossing, release publication, pre-press movement rejection,
  and below-threshold focus preservation without a second pointer pass.
- Core all-feature suite: passed, including 157 unit tests, 46 drag-threshold
  cases, 28 pointer-conformance cases, and the remaining integration/doc tests.
- Widget all-feature suite: passed after updating superseded legacy scrub
  fixtures to use origin-to-position crossing geometry.
- Showcase all-feature suite: 128 library plus 25 binary tests passed, including
  the three legacy click/navigation regressions found by the workspace gate.
- Facade public API surface with all features: 5/5 passed.
- Warning-denied all-target/all-feature Clippy across core, widgets, facade, and
  Showcase passed; formatting and diff checks passed.
- The complete six-command workspace gate and three independent exact-SHA
  critics passed with no P0/P1/P2 findings. Ubuntu, Windows, and macOS passed in
  run 29140855335; PR checks passed in run 29141040177; PR #517 squash-merged as
  `1f991113` and issue #516 is closed.

#### Remaining risks and deferred findings

The threshold is a fixed private current-scope logical default rather than an OS
or application setting. Scope changes during a retained gesture, touch/stylus/
multipointer input, momentum, gesture phases, per-widget adapter click identity,
and drag payload semantics remain deferred. At the `IN-03B` checkpoint,
`TEXT-01` owned actual caret/word/selection editing and had to consume this seam
without reparsing pointer events; it subsequently passed.

### `TEXT-01-PRE`: event-time selection modifiers

Status: Complete. Issue #522 closed through PR #523, squash-merged as
`f2fd2d0`. This is a shared-foundation prerequisite, not a new audit roadmap
ID. At the `TEXT-01-PRE` checkpoint, `ASYNC-01` and `TEXT-01` remained gated on
the separate `TEXT-01-PRE2` seam; the prerequisite and both successors
subsequently passed.

#### Changed files

- `CHANGELOG.md`
- `crates/kinetik-ui-core/src/interaction/drag_select.rs`
- `crates/kinetik-ui-core/src/interaction/press.rs`
- `crates/kinetik-ui-core/src/memory.rs`
- `crates/kinetik-ui-core/src/runtime/ui.rs`
- `crates/kinetik-ui-core/tests/selection_modifier_conformance.rs`
- `crates/kinetik-ui/tests/public_api_surface.rs`
- `docs/specs/01-foundations.md`
- `docs/specs/02-layout-and-interaction.md`
- `docs/alpha-readiness/04-text-renderer-lifetime.md`
- `docs/alpha-readiness/progress.md`

#### Reasoning and contract decisions

Captured selection actions retain the modifier state from their original root
event ordinal without adding metadata to `UiInput` or replaying pointer events.
A private cross-frame baseline handles pointer events before same-frame modifier
changes, while spatially filtered actions still resolve through root ordinals.
Legacy empty streams use and retain their snapshot. Conflicted streams ignore
modifier/key mutations, and focus loss clears and suspends the baseline until a
valid focus gain. Same-owner claims remain no-replay. Adding the public field is
an accepted provisional alpha source break with a changelog migration note.

#### Tests run and results

- New selection-modifier conformance: 11/11 passed after a depth-one
  evidence-only remedy pinned different-owner same-frame claim compatibility.
- Existing drag-threshold conformance: 46/46 passed.
- Facade public API surface: 5/5 passed.
- The complete six-command workspace gate and independent exact-SHA audits
  passed. Ubuntu, Windows, and macOS passed in run 29142938717; PR checks passed
  in run 29143144569 before PR #523 squash-merged as `f2fd2d0`.

#### Remaining risks and deferred findings

At the `TEXT-01-PRE` checkpoint, `TEXT-01` still needed the separately gated
ordinal-bearing DomainDrag seam for canonical editable numeric scrub;
expanding this modifiers-only packet after its accepted task gate would have
mixed contracts. `TEXT-01-PRE2` owned that shared prerequisite before
`ASYNC-01`. Actual word movement/deletion, selection, read-only, multiline,
caret-scroll, IME-owner, and text rendering behavior remained `TEXT-01` or later
Stage 4 work at that checkpoint. The prerequisite and both successors
subsequently passed within their documented contracts.

### `TEXT-01-PRE2`: causal DomainDrag actions

Status: Complete. Issue #524 closed through PR #525, squash-merged as
`00b944f`. This is the second shared-foundation prerequisite discovered by the
`TEXT-01` task gate, not a new audit roadmap ID. At the `TEXT-01-PRE2`
checkpoint, `ASYNC-01` was serialized next and `TEXT-01` remained gated on its
merge because the packets shared runtime, memory, facade, and evidence files;
both successors subsequently passed.

#### Changed files

- `CHANGELOG.md`
- `crates/kinetik-ui-core/src/interaction.rs`
- `crates/kinetik-ui-core/src/interaction/drag_select.rs`
- `crates/kinetik-ui-core/src/interaction/press.rs`
- `crates/kinetik-ui-core/src/lib.rs`
- `crates/kinetik-ui-core/src/memory.rs`
- `crates/kinetik-ui-core/src/runtime/ui.rs`
- `crates/kinetik-ui-core/tests/domain_drag_action_conformance.rs`
- `crates/kinetik-ui/tests/public_api_surface.rs`
- `docs/specs/01-foundations.md`
- `docs/specs/02-layout-and-interaction.md`
- `docs/alpha-readiness/04-text-renderer-lifetime.md`
- `docs/alpha-readiness/progress.md`

#### Reasoning and contract decisions

`Ui::captured_domain_drag_gesture` exposes DomainDrag-specific Press, Move,
Release, and Cancel actions with original root ordinals and event-time
modifiers. Every Release carries its own `release_clicked` outcome, so a field
can distinguish below-threshold caret placement when one canonical frame
contains multiple transactions. The public action ordinal and private release/
drop authority are separate channels; action metadata cannot authorize a drop.

Ordinary, transformed, and captured DomainDrag calls share one first response
per widget in an explicitly begun memory frame. Later observations return that
exact per-frame response, deliver no actions, and do not mutate memory. Runtime
frame finalization closes the cache, while unframed standalone `draggable`
calls retain their previous uncached behavior. This is a provisional breaking
behavioral change for duplicate same-ID calls; callers migrate to one
authoritative call or distinct IDs. No public free captured adapter or local
ordinal namespace was added.

#### Tests run and results

- New DomainDrag action conformance: 16/16 passed, covering threshold outcomes,
  multiple releases, outside/missing positions, spatial ordinal gaps and
  modifiers, full action metadata, legacy/disabled/focus/release-all/conflict/
  clipped cancellation, exact response caching, ordinary/captured/transformed
  orders, disabled-first reset, standalone compatibility, claim independence,
  and transformed/clipped planned/unplanned drops.
- Existing selection modifier, drag threshold, pointer, and runtime spatial
  conformance passed at 11/11, 46/46, 28/28, and 12/12 respectively.
- Facade public API surface passed 5/5; warning-denied core Clippy passed.
- The complete six-command workspace gate and three independent exact-SHA
  implementation/API/evidence audits passed with P0=0, P1=0, P2=0. Ubuntu,
  Windows, and macOS passed in run 29144941082; PR-context checks passed in run
  29145087602 before PR #525 squash-merged as `00b944f`.

#### Remaining risks and deferred findings

The action seam deliberately remains runtime-only; low-level standalone
`draggable` callers receive the existing aggregate response without ordered
actions. At the `TEXT-01-PRE2` checkpoint, `TEXT-01` still had to consume the
captured response once and owned actual numeric caret arbitration, desktop
selection, word behavior, read-only modes, viewports, and IME geometry;
`ASYNC-01` was the next root-owned shared foundation because it edited the same
memory/runtime and evidence files. Both successors subsequently passed.

### `ASYNC-01`: durable presence and incarnation

Status: Complete. Issue #526 closed through PR #527 and squash-merged as
`9d026c5`. The exact task and dependency gates passed after correcting
presence/active semantics, cancellation replacement precedence, tombstone
epochs, observational equality, observer migration, and the direct
`UiTestHarness` Clone dependency.

#### Changed files

- `CHANGELOG.md`
- `crates/kinetik-ui-core/src/liveness.rs`
- `crates/kinetik-ui-core/src/memory.rs`
- `crates/kinetik-ui-core/src/observers.rs`
- `crates/kinetik-ui-core/src/runtime/ui.rs`
- `crates/kinetik-ui-core/src/lib.rs`
- `crates/kinetik-ui-core/src/test_harness.rs`
- `crates/kinetik-ui-core/tests/async_liveness_conformance.rs`
- `crates/kinetik-ui-core/tests/observer_conformance.rs`
- `crates/kinetik-ui-core/tests/domain_drag_action_conformance.rs`
- `crates/kinetik-ui/tests/public_api_surface.rs`
- `docs/specs/01-foundations.md`
- `docs/specs/04-runtime-platform.md`
- `docs/alpha-readiness/04-text-renderer-lifetime.md`
- `docs/alpha-readiness/progress.md`

#### Reasoning and contract decisions

Frame-local presence and durable active incarnation are separate. Repeated
marks across continuously present frames return one opaque registry-scoped
token. First activation, reentry, and explicit restart allocate checked
registry-wide monotonic incarnations. Beginning a frame clears only presence;
the token remains applicable until omission finalizes.

Validation rejects foreign scopes, reports a different latest incarnation
before interpreting its active/tombstone reason, accepts the exact active
incarnation, preserves exact cancellation evidence for the latest cancelled
tombstone, and treats removed/omitted tombstones as stale targets. Thus an old
cancelled token becomes `StaleIncarnation` after same-ID replacement and cannot
cancel the replacement. Tombstones survive one full following frame without
repeated cancellation extending their epoch, then prune without resetting the
scope or allocator.

Authority-bearing `UiMemory`, `LivenessRegistry`, `ObserverRegistry`, and the
`UiTestHarness` wrapper are non-cloneable. Observational equality ignores only
private registry scope and compares all behavior-bearing state; token equality
still includes scope. Observer subscriptions retain one token for one
incarnation, validate during FIFO drain, expose cancelled/stale-incarnation
skips, and require a new subscription after restart/reentry.

#### Tests run and results

- Durable liveness conformance: 15/15 passed, including repeated marks and
  1,000 frames, pre-finalization applicability, both result/remove and
  result/cancel orders, replacement/reentry, foreign registries, equality,
  tombstone grace/pruning, ABA, widget-presence independence, compatibility,
  Send+Sync, and zero unrelated output.
- Observer conformance: 10/10 passed, including 1,000-frame stable
  subscriptions without refresh, FIFO, inactive precedence, cancellation/drain
  order, reincarnation, pruned reason degradation, scope-neutral equality, and
  reentrant deferral.
- PRE2 DomainDrag regression: 16/16 passed after replacing the three memory
  Clone assertions with deterministic Debug snapshots.
- Facade public API surface: 7/7 passed for canonical incarnation/removal/status
  APIs and deprecated generation aliases.
- Complete core all-feature tests passed: 160 unit tests, every integration
  suite, and four compile-fail doctests for token opacity and non-cloneable
  authority.
- Warning-denied workspace Clippy and the complete six-command workspace gate
  passed at exact candidate `0299c15`. Three independent exact-SHA critics
  passed with P0=0, P1=0, P2=0. Ubuntu, Windows, and macOS passed workflow run
  29146185811; PR-context run 29146379516 passed before PR #527 squash-merged.

#### Remaining risks and deferred findings

Cancellation prevents UI delivery but does not reclaim arbitrary worker
resources. `apply_update` intentionally does not deduplicate caller-owned
result identities. Tombstones are bounded by time, not a hard count, and tokens
are process-local. `TEXT-01` depends only on serialized file ownership and the
non-cloneable memory migration; it does not semantically depend on liveness.

### `TEXT-01`: integrated desktop text behavior

Status: Complete / Accepted at implementation merge `93d6a5f` after the
documentation-only Issue #548 closure. Audit §6.10 is closed on canonical
retained `Ui` paths. At this packet's acceptance, checkpoint 4A was accepted
and Stage 4B became Current, with `TEXT-02` next; Stage 4 subsequently passed.

Implementation ledger:

| Packet | Issue / PR | Squash merge | Result |
| --- | --- | --- | --- |
| `TEXT-01-PRE` | #522 / #523 | `f2fd2d0` | Event-time selection modifiers |
| `TEXT-01-PRE2` | #524 / #525 | `00b944f` | Causal DomainDrag actions |
| `TEXT-01A` | #528 / #529 | `f448c40` | Scalar word move/extend/delete/select |
| `TEXT-01B1` | #530 / #532 | `4d25a2b` | Pure single-line/wrapped viewport math |
| `TEXT-01B2` | #531 / #533 | `c191516` | Logical owner mode separate from native IME |
| `TEXT-01B3-PRE` | #534 / #535 | `288657a` | Read-only ordered-input policy |
| `TEXT-01B3-PRE2` | #536 / #537 | `6df12e8` | Final root primary-press ordinal |
| `TEXT-01B3-PRE3` | #539 / #540 | `1b29284` | Completed same-frame pointer routing |
| `TEXT-01B3-PRE4` | #541 / #542 | `ec24e96` | Retained selection gesture anchor |
| `TEXT-01B3` | #538 / #543 | `9102293` | Canonical text-field kernel |
| `TEXT-01B4-PRE5` | #545 / #546 | `9d09d3c` | Exact ordered preview/claim provenance |
| `TEXT-01B4` | #544 / #547 | `93d6a5f` | Canonical numeric/search/path/vector `Ui` wrappers |

#### Changed files

- `CHANGELOG.md`
- `docs/specs/01-foundations.md`
- `docs/specs/03-rendering-text-components.md`
- `docs/specs/04-runtime-platform.md`
- `docs/alpha-readiness/04-text-renderer-lifetime.md`
- `docs/alpha-readiness/progress.md`

The implementation subpackets changed bounded core input/memory/runtime, text
editing/viewport, widget text/wrapper, facade, and conformance-test paths. Their
exact changed-file inventories remain in their issue-linked PRs and Runway
records; this integrated closure changes documentation only.

#### Reasoning and contract decisions

Desktop word behavior is UTF-8 scalar safe and deliberately uses whitespace,
ASCII alphanumeric-plus-underscore, and other-scalar runs until `TEXT-02`.
Pointer selection and editable numeric scrub consume the canonical Selection or
DomainDrag response with original root ordinals; they do not reparse pointer
events. Editable scrub resolves DomainDrag once, previews cloned editing state
only for an authoritative accepted transaction, consumes its exact cached claim,
and commits once. A below-threshold exact clicked release places the caret.

`TextFieldAccess` separates Editable, ReadOnly, and Disabled capabilities.
ReadOnly remains focusable, navigable, selectable, scrollable, and copyable
without mutation or native IME. Logical owner mode is separate from native IME
state. Each field freezes one retained viewport offset, uses entry geometry for
event-time pointer hits and post-edit geometry for paint/caret/selection/preedit/
IME, and stages wheel/caret reveal for the following frame. IME uses only the
visible clipped caret rectangle. Canonical retained `Ui` methods share the
crate-private runtime kernel; public free components remain compatible legacy
paths. Bool APIs remain compatible (`false = Editable`, `true = Disabled`).

#### Tests run and results

- Every implementation subpacket passed its focused deterministic suites, the
  complete six-command workspace gate, independent exact-SHA criticism,
  exact-SHA Ubuntu/Windows/macOS, and PR-context CI before squash merge.
- The final wrapper candidate passed text-field conformance 116/116, runtime
  spatial conformance 7/7, public API surface 7/7, widget unit tests 215/215,
  all six workspace gates, three exact-SHA critics with P0=0/P1=0/P2=0,
  three-OS run 29161566898, and PR run 29161777837 before PR #547 merged.
- Integrated closure verification passed on the documentation candidate: text
  crate 65 unit + 9 read-only + 14 viewport tests; core owner mode 18/18,
  staged scroll 4/4, and the complete core suite; widget text-field 116/116,
  runtime spatial 7/7, public API 7/7, and the complete widget suite including
  215 unit tests; facade public API 7/7. Formatting, warning-denied workspace
  Clippy, workspace tests, workspace build, all-feature example checks, and
  warning-denied workspace docs all passed with the isolated
  `.target-text01-close` cache.

#### Remaining risks and deferred findings

At the `TEXT-01` checkpoint, grapheme clusters, Unicode words, emoji, ligatures,
and mixed bidi remained `TEXT-02`; undo coalescing and text-layout/resource
generation and byte budgets remained `TEXT-03`; and one authoritative
fractional-DPI layout for paint, hit, caret, and selection remained `REND-02`.
All three responsibilities subsequently passed. Viewport motion is
intentionally staged to the next frame. There is no dedicated read-only
semantic bit. Public free components remain compatibility paths, and future
retained `Ui` wrappers must use the canonical transaction kernel rather than
reintroduce split ownership or aggregate-pointer authority.

### `TEXT-02A`: Unicode editing and caret-affinity foundation

Status: Complete / Accepted. Issue #554 closed through PR #555 and squash merge
`ac9a1e2` after candidate `44c16d1`, exact-SHA review, three-OS CI, and PR CI
passed. This is the first serialized foundation of `TEXT-02`; it does not close
audit §6.9 or roadmap `TEXT-02`.

#### Changed files

- `Cargo.lock`
- `CHANGELOG.md`
- `crates/kinetik-ui-text/Cargo.toml`
- `crates/kinetik-ui-text/src/{boundary,edit,lib,selection,tests,undo}.rs`
- `crates/kinetik-ui-text/tests/unicode_editing_conformance.rs`
- `crates/kinetik-ui/tests/public_api_surface.rs`
- `docs/specs/03-rendering-text-components.md`
- `docs/alpha-readiness/{04-text-renderer-lifetime,progress}.md`

#### Reasoning and contract decisions

Logical editing now uses UAX #29 extended grapheme clusters and one full-buffer
word-bound segmentation pass. Unicode whitespace is the only traversal
separator; punctuation, symbols, and emoji remain distinct UAX segments.
Offsets inside a segment, exact boundaries, and buffer end have explicit
forward/backward/select tie rules. Combining sequences, emoji modifiers,
regional-indicator flags, ZWJ emoji, and CRLF are indivisible; explicit-line
columns count graphemes without normalizing bytes.

`TextCaret` adds an offset plus `TextAffinity::{Before, After}` without removing
byte-only APIs. Start/internal byte-only positions default to `After`, a
non-empty end defaults to `Before`, movement has fixed directional affinity,
and undo/redo records the effective affinity. A private offset stamp prevents
direct mutation of public `TextSelection::active` from exposing stale affinity.
Every operation canonicalizes public endpoints before selection-vs-caret
branching, so two raw endpoints inside one grapheme take the canonical caret
path and true no-ops preserve redo.

#### Tests run and results

- Dedicated Unicode editing conformance passed 12/12, covering combining,
  emoji modifier/flag/ZWJ, CRLF and grapheme columns, contextual words,
  punctuation/whitespace ties, affinity/equality/undo, malformed public
  selections, ordered insertion/navigation, composition, and ReadOnly copy.
- Complete text verification passed: 65 unit, 9 ReadOnly ordered-input, 14 text
  viewport, and 12 Unicode editing tests; doc tests also passed.
- Facade public API conformance passed 8/8, including qualified additive caret
  APIs and unchanged legacy byte-offset calls.
- All six workspace gates passed in ignored `target/runway/text02a`: formatting,
  warning-denied workspace Clippy, all-feature workspace tests, all-feature
  workspace build, all-feature examples, and warning-denied workspace docs.
- Three exact-SHA candidate critics reported P0/P1/P2=`0/0/0` on candidate
  `44c16d1`. Ubuntu, Windows, and macOS passed run 29168078196; PR-context run
  29168249096 passed before PR #555 squash-merged as `ac9a1e2`.

#### Remaining risks and deferred findings

This packet deliberately supplied logical segmentation and affinity only. At
the `TEXT-02A` checkpoint, authoritative shaped visual stops, mixed-bidi
Left/Right, ligature subdivision, and hit/caret/selection rectangles remained
`TEXT-02B`; canonical widget, ReadOnly, pointer, ordered re-resolution, and IME
integration remained `TEXT-02C`; fractional-DPI paint/hit/caret/selection
parity remained `REND-02`; and undo coalescing plus layout/resource generation
and byte budgets remained `TEXT-03`. Those slices subsequently passed.
Locale-tailored segmentation, normalization, color emoji, and an engine
replacement remain out of scope.

### `TEXT-02B`: source-bound shaped navigation authority

Status: Complete / Accepted. Issue #556 closed through PR #557 after candidate
`6735879`, exact-SHA review, three-OS CI, and PR CI passed, then squash-merged as
`676cb4e`. The retained-widget integration exposed omitted ASCII wrap
delimiters as a prerequisite; Issue #559 closed through PR #560 after remediated
candidate `0b63eb2` and the same gates passed, then squash-merged as `2814a3c`.
This accepted text-layer foundation does not close audit §6.9 or roadmap
`TEXT-02` without `TEXT-02C`.

#### Changed files

- `CHANGELOG.md`
- `crates/kinetik-ui-text/src/{edit,lib,navigation}.rs`
- `crates/kinetik-ui-text/tests/unicode_layout_conformance.rs`
- `crates/kinetik-ui/tests/public_api_surface.rs`
- `docs/specs/03-rendering-text-components.md`
- `docs/alpha-readiness/{04-text-renderer-lifetime,progress}.md`

#### Reasoning and contract decisions

`ShapedTextLayout::navigation` now derives one owned, exact-source-bound map
from existing cosmic-text positioned cluster ranges. Duplicate glyphs for one
cluster form a union, multi-EGC clusters divide by grapheme count, and private
coordinate nodes retain same-position Before/After aliases without collapsing
bidi or wrap seams at different coordinates. One map owns visual character and
full-buffer word motion, hit testing, caret rectangles, and logical-selection
visual spans. Construction rejects malformed public line/run/glyph geometry,
  derived overflow (including cross-cluster visual unions), out-of-sequence
  visual lines, overlap, direction disagreement, and incomplete EGC coverage
  all-or-nothing. Finite extreme hit distances are compared in f64 before the
  selected public coordinate is returned as f32 geometry.

The public shaped structs carry no historical source provenance, so callers
must still pair a layout with the exact source originally shaped. The map owns
the supplied snapshot and later `TextEditState` calls reject unequal text
before any canonicalization or mutation. Matching calls canonicalize both
public endpoints before branching, preserve composition and undo/redo, and
report `Moved`, `Unchanged`, or `SourceMismatch`. Existing public struct
literals and byte-only geometry methods remain source-compatible.

#### Tests run and results

- Dedicated shaped Unicode conformance passed 15/15, covering combining and
  emoji clusters, real and synthetic multi-EGC clusters, pure RTL and mixed
  bidi, wrapped seams, empty-line aliases, full-buffer visual words,
  transactional stale-map rejection, physical selection collapse, all error
  variants, public epsilon thresholds, finite extreme hit distances, and
  derived geometry overflow.
- Complete text verification passed 66 unit, 9 ReadOnly ordered-input, 14 text
  viewport, 12 Unicode editing, and 15 Unicode layout tests; doc tests passed.
- Facade public API conformance passed 9/9 and widget text-field conformance
  passed 116/116. Formatting, warning-denied workspace Clippy, all-feature
  workspace tests, all-feature workspace build, all-feature example checks,
  and warning-denied workspace docs passed with isolated
  `target/runway/text02b`.
- Three exact-SHA critics reported P0/P1/P2=`0/0/0` for both accepted
  candidates. The original packet and wrap-cell prerequisite each passed local
  focused/workspace verification, exact-SHA Ubuntu/Windows/macOS dispatch, and
  PR-context CI before squash merge.

#### Remaining risks and deferred findings

The constructor can prove structural consistency with its caller-supplied
source but not historical shaping provenance. Pinned bundled Inter supplies a
real `->` multi-EGC cluster; future font/shaper upgrades must deliberately
revalidate that witness. At the `TEXT-02B` checkpoint, canonical widget/
ReadOnly/pointer/ordered re-resolution/IME integration remained `TEXT-02C`,
fractional-DPI paint parity remained `REND-02`, and undo/layout/resource budgets
remained `TEXT-03`; those slices subsequently passed.

### `TEXT-02C`: retained shaped text authority

Status: Complete / Accepted. Issue #558 closed through PR #561 after candidate
`12443ec`, three exact-SHA critics, three-OS run 29174582250, and PR run
29174571824 passed, then squash-merged as `691c6ab`. Main push run 29174764195
also passed. This closes audit §6.9 and roadmap `TEXT-02`.

#### Changed files

- `CHANGELOG.md`
- `crates/kinetik-ui-text/src/edit.rs`
- `crates/kinetik-ui-text/tests/shaped_key_conformance.rs`
- `crates/kinetik-ui-widgets/src/components/{text_fields,text_geometry,text_interaction}.rs`
- `crates/kinetik-ui-widgets/src/components/tests/text_fields.rs`
- `crates/kinetik-ui-widgets/tests/text_field_conformance.rs`
- `crates/kinetik-ui-widgets/tests/text_field_conformance/unicode_authority.rs`
- `crates/kinetik-ui/tests/public_api_surface.rs`
- `docs/specs/03-rendering-text-components.md`
- `docs/alpha-readiness/{04-text-renderer-lifetime,progress}.md`

#### Reasoning and contract decisions

`TextEditState::apply_visual_navigation_key` gives ordered replay one qualified
pressed/repeated Left/Right entry point. Every consumed horizontal key resolves
the exact current model source after preceding mutations; invalid or stale
retained maps fail closed without scalar fallback. Active preedit consumes the
key unchanged before invoking the resolver because native IME owns movement
inside display-only composition text.

Entry pointer hits remain frozen to their causal display snapshot. Post-replay
geometry owns one exact display-source `ShapedTextNavigation` and registered
layout for paint, hit, caret affinity, disjoint selection, preedit underline and
caret, viewport reveal, and visible native IME placement. Composition mapping
preserves shaped affinity while collapsing interior and end hits to the model
insertion seam. Invalid retained navigation discards the entire shaped snapshot
and uses the layoutless compatibility path. The Unicode-authoritative alpha
promise therefore applies only to canonical retained fields configured with
`TextLayoutStore`.

#### Tests run and results

- Shaped-key conformance passed 4/4; the complete text crate passed 66 unit, 9
  ReadOnly, 4 shaped-key, 14 viewport, 12 Unicode-editing, and 23 Unicode-layout
  tests plus doc tests.
- Retained text-field conformance passed 125/125, including the full mutation
  and pointer causal matrices, grapheme/bidi/wrap/offset fixtures, preedit
  geometry and hit mapping, validation fallback, access modes, and clipboard.
- The complete widgets crate and all integrations passed; facade public API
  conformance passed 9/9.
- Formatting, warning-denied workspace Clippy, all-feature workspace tests,
  all-feature workspace build, all-feature examples, and warning-denied
  workspace docs passed in `target/runway/text02c`. `RUSTDOCFLAGS` was restored.

#### Remaining risks and deferred findings

At the `TEXT-02C` checkpoint, fractional device projection remained `REND-02`
and retained layout/resource generation and byte budgets remained
`TEXT-03B/C`; both subsequently passed. Rejected numeric scrub previews could
register unused layouts before those budgets landed. Invalid internal
navigation falls back atomically but has no public diagnostic channel.
Free/no-store compatibility paths do not carry the Unicode-authoritative alpha
promise. Locale tailoring, normalization, color emoji policy, and vertical
visual navigation remain outside alpha scope.

### `TEXT-03A`: bounded and coalesced local undo

Status: Complete / Accepted for the `TEXT-03A` slice. Issue #562 closed through
PR #563. Candidate `7f57c77b47ea394d848451548782aad053cdd26a` passed
three exact-SHA critics at P0/P1/P2=`0/0/0`, PR CI run 29176731298, and
Ubuntu/Windows/macOS run 29176736532 before squash merge
`21be11cb8a16cde9666932f99ef62b01793c7845`; main-push CI run 29176933068
passed the merge SHA. This remained only the A slice of `TEXT-03` evidence for
audit §§8.4, 10.2, and 11.5 and, at that checkpoint, did not close those
findings, roadmap `TEXT-03`, or Stage 4 without `TEXT-03B/C` and `REND-02`.

#### Changed files

- `CHANGELOG.md`
- `crates/kinetik-ui-text/src/{edit,undo}.rs`
- `crates/kinetik-ui-text/tests/undo_budget_conformance.rs`
- `docs/specs/03-rendering-text-components.md`
- `docs/alpha-readiness/{04-text-renderer-lifetime,progress}.md`

#### Reasoning and contract decisions

Local undo and redo retain at most 128 combined snapshots and 4 MiB of exact
UTF-8 snapshot text. Fixed selection, affinity, and container metadata is
separately bounded by the entry cap. Count and payload accumulation use checked
arithmetic. New history and traversal evict the deepest/farthest states first
while preserving the nearest retainable reverse target. Oversized pre-edit
states clear both directions; an oversized traversal state makes that traversal
one-way instead of admitting a discontinuous jump. Retainability and target
existence are checked before a full snapshot is allocated.

Only canonical ordered hardware insertion plus unmodified Backspace and Delete
without active composition coalesce. Kind, direction, text/range continuity,
exact caret/affinity, and composition state must match; runs end at an inclusive
4096 changed UTF-8 bytes, and a
crossing fragment starts a new unit whole. Public direct edits, legacy slices,
modified or active-preedit deletion, paste/cut, IME commit, selection
replacement, word deletion, and multiline Enter remain atomic. Navigation,
selection, pointer placement, composition, focus loss, shortcuts,
target-matching paste results including filtered-empty input, and traversal
fence a run; wrong-target paste does not. Stale shaped navigation and
active-preedit suppressed arrows retain whole-state transactionality. No public
API or dependency changed.

#### Tests run and results

- Dedicated undo-budget conformance passed 12/12, covering 128-entry and 4 MiB
  boundaries, barriers, 4096-byte chunking, 10,000-byte literal traversal,
  100,000 fragments, 10,000 alternating atomic replacements, multibyte
  non-splitting, repeated Unicode deletion, modifier/preedit eligibility,
  direction switches, the semantic fence matrix including cut/word
  deletion/selection replacement and filtered/foreign paste, IME/paste/Enter
  units, mixed undo/redo event ordering, stale and preedit transactionality,
  forward/reversed selection and affinity restoration, direct atomicity, clone
  continuity, and wrong-target/no-op redo behavior.
- Private history tests passed 9/9 for exact UTF-8 accounting, inclusive byte
  eviction, full-stack bidirectional transfer, combined count and long
  alternating byte eviction, checked run retention without extra snapshots,
  snapshot-allocation eligibility, nearest-target transfer eviction, and
  one-way oversized transfers.
- Complete text verification passed 75 unit, 9 ReadOnly, 4 shaped-key, 14
  viewport, 12 undo-budget, 12 Unicode-editing, and 23 Unicode-layout tests.
- Retained text-field conformance passed 125/125; the complete widgets crate
  and integrations passed; facade public API conformance passed 9/9.
- All six workspace gates passed in `target/runway/text03a`: formatting,
  warning-denied Clippy, all-feature workspace tests, all-feature build,
  all-feature examples, and warning-denied docs. `RUSTDOCFLAGS` was restored.

#### Remaining risks and deferred findings

Snapshots remain O(buffer size) at retained unit boundaries. Buffers over 4 MiB
intentionally lose reverse traversal at explicit barriers. Operation-based
4096-byte grouping is coarser than elapsed-time editor grouping. Equal-length
direct mutation through the public `text`/`selection` fields cannot be detected
without a breaking encapsulation change; method-driven canonical fields carry
the alpha guarantee. At the `TEXT-03A` checkpoint, layout generations/bytes,
rejected-preview churn, and incremental renderer-resource export remained
`TEXT-03B/C`, while fractional projection remained `REND-02`; those slices
subsequently passed.

### `TEXT-03B`: retained layout generations and payload budgets

Status: Complete / Accepted. Issue #564 closed through PR #565 and squash merge
`83e2847`. The accepted candidate `2c79c10` passed three exact-SHA critics at
P0/P1/P2=`0/0/0`, PR CI run 29179370375, Ubuntu/Windows/macOS run 29179370450,
and main-push CI run 29179571377. This is partial
`TEXT-03` evidence for audit §§8.4, 10.2, 11.5, and the bounded-cache portion of
§11.7; it does not close `TEXT-03`, Stage 4, or the duplicate-cache API finding
without `TEXT-03C`, `REND-02`, and final Stage 7 API curation.

#### Changed files

- `CHANGELOG.md`
- `crates/kinetik-ui-text/src/{cache,lib,store,tests}.rs`
- `crates/kinetik-ui-text/tests/layout_budget_conformance.rs`
- `crates/kinetik-ui-widgets/src/components/{text_fields,text_geometry,text_support}.rs`
- `crates/kinetik-ui-widgets/src/ui/{frame,output}.rs`
- `crates/kinetik-ui-widgets/tests/text_layout_lifetime_conformance.rs`
- `crates/kinetik-ui/tests/public_api_surface.rs`
- `docs/specs/03-rendering-text-components.md`
- `docs/alpha-readiness/{04-text-renderer-lifetime,progress}.md`

#### Reasoning and contract decisions

`TextLayoutStore` now strictly retains at most 32 MiB of checked owned key and
shaped-layout payload. The metric counts each owned struct and String/Vec
capacity once while deliberately excluding map buckets, allocator/Arc headers,
shared fonts, external Arcs, and shaping-engine internals. Current-generation
entries are pinned; older pressure evicts by generation, touch ordinal, then ID.
Untouched layouts survive 120 completed idle generations and expire entering
generation 121. Admission rejection is transactional and canonical callers use
the additive fallible path. The existing infallible method remains compatible
with a store-local zero sentinel, while arbitrary preassigned layout IDs remain
caller-owned.

IDs remain stable while resident. Evicted IDs are retired within one change
epoch to prevent ABA, and shared key ownership gives both key and ID indices
without duplicating payload. A lazy fixed journal retains at most 256 KiB of
dirty ID records and uses process-unique store incarnation, epoch, and revision
to reject foreign/stale cursors. Ordinary rollover requires a full snapshot;
checked epoch exhaustion preserves residents, drops tombstones, and enters
permanent resync-only mode. `stored_layout` resolves final presence in expected
O(1), and full iteration is ID ordered. Actual renderer add/remove/full-reset
integration remained `TEXT-03C` at the `TEXT-03B` checkpoint and subsequently
passed.

Frame attachment advances exactly once and assumes generation G resources are
reconciled before G+1 begins. Entry pointer geometry and every event-navigation
shape are transient; final field geometry alone retains. Rejected numeric scrub
previews therefore produce no extra additions, touches, bytes, IDs, or journal
records compared with an otherwise identical control. The compatibility
`TextLayoutCache` receives the same byte/age/pinning policy while preserving
its method signatures and visible Clone/PartialEq/Debug behavior.

#### Tests run and results

- Text unit verification passed 102/102, including 27 new private store/cache
  tests for exact UTF-8 bytes, G+120/G+121, current pins, deterministic LRU
  tie-breaks, collision probing through `u64::MAX`, checked generation/touch/
  revision counters, exact production journal capacity, source exhaustion and
  terminal mode, same-store/foreign/future cursors, observational export,
  external Arc lifetime, and cache compatibility.
- Public layout-budget conformance passed 6/6: 100,000 stable hits, 1,000
  Unicode/wrapped dynamic generations, literal over-budget rejection,
  source-bound cursors/final presence, transient shaping, and compatibility
  cache plateau.
- Widget layout-lifetime conformance passed 5/5, including exactly-once frame
  attachment, arbitrary external-ID preservation (including raw zero),
  accepted final-only retention, same-frame 32 MiB saturation with resolvable
  accepted IDs and layoutless rejections, and 1,000 unique rejected preview/
  control frames followed by actual deterministic pressure. Retained
  text-field conformance passed 125/125.
- Facade public API conformance passed 10/10; renderer resource snapshots
  remained compatible at 12/12; warning-denied Clippy passed for the touched
  text, widget, facade, render, and Vello dependency surface.
- All six workspace gates passed in `target/runway/text03b`: formatting,
  warning-denied Clippy, all-feature workspace tests, all-feature build,
  all-feature examples, and warning-denied docs. `RUSTDOCFLAGS` was restored.

#### Remaining risks and deferred findings

Transient shaping can temporarily allocate and consume CPU outside the retained
metric. Allocator metadata, shared font data, engine internals, and external Arc
owners are excluded by contract. Current-generation saturation intentionally
degrades new canonical generic text to layoutless fallback. Direct callers own
raw handle validity across clear/eviction. At the `TEXT-03B` checkpoint,
renderer reconciliation and external Arc byte lifetime remained `TEXT-03C`,
while Vello cache lifetime and fractional projection remained later packets;
those slices subsequently passed. The public approximate cache remains a
duplicate contract until final `API-01` curation in Stage 7.

### `TEXT-03C`: incremental text resource reconciliation

Status: Complete / Accepted through Issue #566 and PR #567 at squash merge
`3b5af7b0341520781e1d286605aaf3e3e7dd9bbe`. Candidate
`61baa82693957bbb5b71e716c33ab0a133a6eb5f` and the
amended task SHA-256
`41e7ebec3c3cfc638f62361d39be0016b8293cb36346e169dee24f136601f5d0`
passed three independent read-only task critics at P0/P1/P2=`0/0/0` after one
bounded task remedy. Focused renderer, facade, and persistent-showcase tests and
warning-denied touched-surface Clippy pass. All six workspace gates pass in the
isolated `target/runway/text03c` cache with `RUSTDOCFLAGS` restored. Exact-
candidate critics passed at P0/P1/P2=`0/0/0`; PR CI 29180858792, three-OS run
29180863795, and main-push CI 29181022198 passed. This closes roadmap `TEXT-03`
and the text-resource portions of audit §§8.4, 10.2, and 11.5, but not Stage 4
or duplicate-cache §11.7.

#### Changed files

- `CHANGELOG.md`
- `crates/kinetik-ui-render/src/lib.rs`
- `crates/kinetik-ui-render/tests/text_layout_reconciliation_conformance.rs`
- `crates/kinetik-ui/src/lib.rs`
- `crates/kinetik-ui/tests/public_api_surface.rs`
- `apps/kinetik-ui-showcase/src/{app,live,main}.rs`
- `apps/kinetik-ui-showcase/src/app/runtime/lifecycle.rs`
- `apps/kinetik-ui-showcase/src/app/tests/{resources,vello}.rs`
- `docs/specs/03-rendering-text-components.md`
- `docs/alpha-readiness/{04-text-renderer-lifetime,progress}.md`

#### Reasoning and contract decisions

Each renderer registry now pairs with one intentionally non-clonable,
caller-owned `TextLayoutResourceSync`. Initial, explicit reset, foreign/stale,
rollover, and terminal cursors clear and rebuild only text resources. Ordinary
batches consume bounded dirty IDs and resolve final store presence, so present
IDs upsert only when key or Arc identity differs and absent IDs remove. A
zero-change incremental pass clones no key, replaces no Arc, and mutates no map.
Full and incremental report counts plus the no-op predicate are literal, and
multiple delayed consumers remain independent.

Renderer payload accounting mirrors the accepted store boundary with checked
owned-key and reachable shaped-layout capacities. Store and registry metrics
are reachability measures, not additive RSS, because layouts are Arc-shared.
Resource Arcs may intentionally bridge store eviction until the next
reconciliation; stale entries then release unless an external owner remains.
Manual full-snapshot registration remains compatible but may not be mixed into
a managed text namespace between sync calls.

`UiState` exposes an additive caller-owned reconciliation helper. The showcase
owns one persistent registry and sync state, registers static media once,
reconciles after each completed frame, and returns a borrowed registry instead
of cloning static payloads and every retained text key on each access.

#### Tests run and results

- Renderer reconciliation conformance passed 8/8: initial/no-op, add/remove,
  duplicate final absence, clear/manual/foreign reset, delayed consumers, Arc
  lifetime, 1,000 dynamic generations, and same-frame 32 MiB saturation.
- Renderer unit tests passed 9/9, including dirty update/identity classification
  and checked overflow rejection.
- Facade public API passed 11/11 and facade unit tests passed 14/14, including
  two independent caller-owned consumers.
- Showcase resource-focused tests passed 8/8, including persistent registry/key/
  Arc identity, genuine page-exclusive stale-layout removal, and 1,000 page
  frames with exact store/resource presence and equal bounded metrics.
- Existing renderer resource snapshots passed 12/12, showcase quality passed
  2/2, and Vello text-focused compatibility passed 37 unit plus 4 translation
  tests.
- Complete affected crates passed: renderer 9 unit, 3 color, 12 snapshot, 8
  reconciliation, and 1 compile-fail doc test; facade 14 unit plus 11 public API;
  showcase 131 library plus 25 binary; Vello 94 unit, 1 color, 18 translation,
  and 5 transform-recovery tests.
- Warning-denied Clippy passed for render, facade, and showcase all targets and
  all features.
- All six workspace gates passed in `target/runway/text03c`: formatting,
  warning-denied workspace Clippy, all-feature workspace tests, all-feature
  workspace build, all-feature examples, and warning-denied workspace docs.
  `RUSTDOCFLAGS` was restored.

#### Remaining risks and deferred findings

Terminal journal exhaustion deliberately causes a full text rebuild every
reconciliation. Payload metrics exclude shared fonts, backend caches, allocator
metadata, Arc headers, and external owners and are not RSS. Managed registries
require the documented one-in-flight generation boundary and forbid interleaved
manual text mutation. At the `TEXT-03C` checkpoint, Vello physical text cache
policy and authoritative fractional projection remained `REND-02`; they
subsequently passed. Image/texture budgets, presenter ownership, and external
GPU resources remain later packets. Approximate `TextLayoutCache` duplication
remains final Stage 7 `API-01`.

### `REND-01B`: sRGB, alpha, and tint contract

Status: Complete / Accepted. Issue #550 closed through PR #551. Candidate
`609ae127` passed all local gates and three exact-SHA critics, three-OS run
29165037981, and PR run 29165219725 before squash merge `9c1c044`.

#### Changed files

- `CHANGELOG.md`
- `crates/kinetik-ui-core/src/render.rs`
- `crates/kinetik-ui-core/tests/render_color_conformance.rs`
- `crates/kinetik-ui-render/src/lib.rs`
- `crates/kinetik-ui-render/tests/color_alpha_conformance.rs`
- `crates/kinetik-ui-vello/src/{geometry,image,sanitize,tests}.rs`
- `crates/kinetik-ui-vello/src/tests/color_alpha.rs`
- `crates/kinetik-ui-vello/tests/{render_color_conformance,render_translation_conformance}.rs`
- `docs/specs/{03-rendering-text-components,04-runtime-platform}.md`
- `docs/render-snapshots.md`
- `docs/alpha-readiness/{04-text-renderer-lifetime,progress}.md`

#### Reasoning and contract decisions

The public `Color` contract is straight sRGB with straight alpha and an
unchecked, source-compatible constructor boundary. Translation diagnoses each
invalid occurrence once, canonicalizes/clamps its channels before command
snapshots, and passes the same values directly into `AlphaColor<Srgb>`.
Gradients explicitly request sRGB/premultiplied interpolation. Image format and
alpha metadata remain caller-owned; premultiplied tint applies tint alpha to RGB
with the exact single-round integer formula. Existing public resource snapshots
remain sorted payload-presence inventories without new format/alpha fields.

#### Tests run and results

- `cargo test -p kinetik-ui-core --test render_color_conformance --all-features`
  passed 2/2.
- `cargo test -p kinetik-ui-render --test color_alpha_conformance --all-features`
  passed 3/3.
- `cargo test -p kinetik-ui-vello --lib color_alpha --all-features` passed 7/7.
- `cargo test -p kinetik-ui-vello --test render_color_conformance --all-features`
  passed 1/1.
- The focused all-occurrence translation sanitization test passed 1/1.
- Complete core/render/Vello suites and all six workspace gates passed with the
  isolated `.target-rend01b` cache: format check, warning-denied workspace
  Clippy, workspace tests, workspace build, all-feature example checks, and
  warning-denied workspace docs.
- Three exact-SHA critics reported P0/P1/P2=`0/0/0` on candidate `609ae127`.
  Ubuntu, Windows, and macOS passed run 29165037981; PR-context run 29165219725
  passed before PR #551 squash-merged as `9c1c044`.

#### Remaining risks and deferred findings

Vello 0.9's resolved ramp is private, so direct Peniko field/interpolation and
public raw-stop assertions are the executable upgrade fence; the resolved ramp
is source-verified residual evidence. Premultiplied payload bytes are trusted.
The public diagnostic retains the `InvalidGeometry` name for invalid colors.
HDR/wide-gamut/ICC conversion, external textures, presentation, and pixel
goldens are not introduced by this packet.

### `REND-01-CLOSE`: integrated renderer evidence and checkpoint 4A

Status: Complete / Accepted. The merged evidence for `REND-01A` and
`REND-01B` closes audit §§6.12-6.13 and, together with accepted `ASYNC-01` and
`TEXT-01`, accepts checkpoint 4A. At that checkpoint, Stage 4 remained Current /
Authorized at 4B; 4B subsequently passed.

#### Changed files

- `docs/alpha-readiness/04-text-renderer-lifetime.md`
- `docs/alpha-readiness/progress.md`

#### Reasoning and contract decisions

`REND-01A` accepts balanced recovery frames for rejected non-finite and
overflowing transform begins. `REND-01B` accepts straight sRGB plus straight
alpha as the public color contract, Vello translation as the sanitization
authority, explicit sRGB/premultiplied gradient interpolation, and exact
straight/premultiplied tint rounding without expanding public resource
snapshots. Their merged evidence is sufficient to accept `REND-01` and the 4A
checkpoint; no source, test, spec, changelog, workflow, or audit-output behavior
changes in this documentation-only closure.

At the `REND-01-CLOSE` checkpoint, `TEXT-02` was explicitly next, `TEXT-03`
remained behind the text-store API freeze, and `REND-02` remained behind both
`TEXT-02` and accepted `REND-01`. Unicode editing, text-store budgets, and
authoritative fractional-DPI text layout were checkpoint 4B requirements at
that time; all subsequently passed.

#### Tests run and results

- `REND-01A` closed Issue #518 through PR #520 and squash merge `1aee4f4`.
  Its focused transform-recovery evidence, local gates, exact-SHA review,
  three-OS run 29141679730, and PR checks passed.
- `REND-01B` closed Issue #550 through PR #551 and squash merge `9c1c044`.
  Candidate `609ae127` passed core color conformance 2/2, render color/alpha
  conformance 3/3, private Vello color/alpha conformance 7/7, public Vello
  submission conformance 1/1, exact sanitization ordering 1/1, the complete
  core/render/Vello suites, and all six workspace gates.
- Three exact-SHA `REND-01B` critics reported P0/P1/P2=`0/0/0`. Ubuntu,
  Windows, and macOS passed run 29165037981, and PR-context run 29165219725
  passed before merge.
- This documentation-only closure candidate passed the exact path/residue
  guard, every prescribed focused test, the complete core/render/Vello suites,
  and all six workspace gates using ignored `target/runway/rend01-close`.

#### Remaining risks and deferred findings

Vello's resolved 512-sample gradient ramp remains private and source-verified
rather than directly executable. Premultiplied payload correctness remains
caller-owned, and `InvalidGeometry` remains the diagnostic name for invalid
colors. HDR/wide-gamut/ICC conversion, external GPU resources,
presenter/swapchain ownership, and CPU/GPU pixel goldens remain deferred.
At that checkpoint, fractional-DPI authoritative text layout remained
`REND-02`, Unicode clusters remained `TEXT-02`, and bounded/coalesced undo plus
text layout/resource budgets remained `TEXT-03`. Stage 4 and the campaign were
not complete at that time; these historical 4B dependencies subsequently
passed.

### `REND-02`: authoritative fractional-DPI text projection

Status: Complete / Accepted. Issue #568 closed through PR #569. Candidate
`156ceaec62312669b30479f2f5e359346408dc1e` passed three exact-SHA critics at
P0/P1/P2=`0/0/0`, PR CI run 29186376228, and exact-SHA Ubuntu/Windows/macOS run
29186433862 before authorized squash merge
`1239dd994619de3765d8cee05c5f8ddd34c2c6de`; main-push CI run 29186580620
passed the merge SHA.

#### Changed files

- `CHANGELOG.md`
- `apps/kinetik-ui-showcase/src/app/tests/vello.rs`
- `apps/kinetik-ui-showcase/src/main.rs`
- `crates/kinetik-ui-vello/src/encoding.rs`
- `crates/kinetik-ui-vello/src/lib.rs`
- `crates/kinetik-ui-vello/src/renderer.rs`
- `crates/kinetik-ui-vello/src/tests.rs`
- `crates/kinetik-ui-vello/src/tests/common.rs`
- `crates/kinetik-ui-vello/src/tests/text_authority.rs`
- `crates/kinetik-ui-vello/src/tests/text_cache.rs`
- `crates/kinetik-ui-vello/src/tests/text_layouts.rs`
- `crates/kinetik-ui-vello/src/tests/text_paths.rs`
- `crates/kinetik-ui-vello/src/tests/text_snapping.rs`
- `crates/kinetik-ui-vello/src/text.rs`
- `crates/kinetik-ui-vello/src/translation.rs`
- `docs/alpha-readiness/04-text-renderer-lifetime.md`
- `docs/alpha-readiness/progress.md`
- `docs/specs/03-rendering-text-components.md`

#### Reasoning and contract decisions

Resolved registered `TextLayoutResource` layouts are the sole Vello shaping
and glyph-topology authority. Primitive text/family/size/line-height metadata
remains compatibility input and cannot reshape or override a resolved layout.
Exactly positive axis-aligned transforms use the shared snapped translation,
exact scaled font size, exact non-uniform outline ratio, and one full-f64
projection and rounding of each absolute glyph point before f32 storage. Every
nonzero-skew, rotated, reflected, negative, singular, or otherwise general
affine remains on the raw unhinted path.

Layoutless and unresolved-resource compatibility paint shapes logical keys
through a renderer-private `TextLayoutStore`. It advances once per submitted
frame, retains at most 32 MiB, expires after 120 idle generations, shapes
transiently on rejection, and is independent of framebuffer scale. Registered
resources never enter that fallback store.

Three authorized implementation/process correction packets align the evidence
with this frozen contract. `REND-02-PC1` replaced the stale Showcase integer-font assertion with
registered-resource and exact scaled-size evidence. `REND-02-PC2` keeps affine
projection and rounding in f64 until final f32 storage, adds the literal
`2.8_f32` at 1.25 witness that encodes physical x=3, and requires strict
identity-transform selection/caret/glyph-anchor parity at 1.25, 1.5, and 1.75.
`REND-02-PC3` resolves registered resources before validating compatibility
metrics, substitutes a private deterministic finite-positive placeholder only
for ignored invalid registered command fields, preserves strict validation and
deterministic diagnostics for layoutless/missing-resource fallback, and proves
registered layouts leave the fallback store empty. `REND-02-PC4` was the
documentation-only exact-SHA evidence correction that synchronized these
tracked readiness sections with PC3; it changed no production behavior.

#### Tests run and results

- Vello focused results passed: authority 6/6; fallback cache 6/6; registered layouts
  4/4; transform paths 4/4; text snapping 7/7.
- Complete Vello verification passed 95 unit and 24 integration tests.
- Render passed 9 unit, 23 integration, and 1 compile-fail documentation test.
- Text passed 102 unit and 80 integration tests.
- Widget text-field conformance passed 125/125.
- Facade passed 14 unit and 11 public-API tests.
- Showcase passed 132 library and 25 binary tests.
- Warning-denied all-target/all-feature Clippy for Vello and Showcase passed.
- All six workspace gates passed on the final tree, and `RUSTDOCFLAGS` was
  restored to its prior unset state.

#### Remaining risks and deferred findings

The authoritative guarantee applies only to canonical registered layouts.
Layoutless and missing-resource fail-soft paint remains non-authoritative and
cannot reconstruct retained wrap or Unicode navigation state. Fractional
command translations retain the generic rectangle quantization band of at most
1.0001 physical pixels. CPU scene encoding proves submitted topology and
coordinates, not final GPU raster coverage or cross-GPU pixel identity. The
duplicate public `TextLayoutCache` remains Stage 7 `API-01`; presenter
ownership, external textures, and public composition remain Stage 5.

### `STAGE-4-CLOSE`: integrated text, renderer, and lifetime acceptance gate

Status: Stage 4 is Complete / Accepted through the accepted `REND-02` squash
merge `1239dd994619de3765d8cee05c5f8ddd34c2c6de`. Stage 5 is Current /
Authorized with `REND-ADR-01` next. Issue #570 owns this documentation-only
integration; it implements or pre-accepts no Stage 5 packet.

#### Changed files

- `docs/alpha-readiness.md`
- `docs/alpha-readiness/00-plan-and-baseline.md`
- `docs/alpha-readiness/01-truth-and-release.md`
- `docs/alpha-readiness/02-runtime-foundation.md`
- `docs/alpha-readiness/03-input-and-shell.md`
- `docs/alpha-readiness/04-text-renderer-lifetime.md`
- `docs/alpha-readiness/05-composition-foundations.md`
- `docs/alpha-readiness/progress.md`

#### Reasoning and contract decisions

The close packet integrates six accepted roadmap responsibilities without
changing source, tests, specifications, changelog, workflows, release outputs,
or audit artifacts:

- `ASYNC-01` accepts deterministic presence/incarnation/cancellation cleanup at
  `9d026c5f5a2108e79253e977868f60ec6522e9b8`.
- `TEXT-01` accepts canonical desktop/ReadOnly behavior at implementation
  `93d6a5f775fea1bc416ec7bf360cd95b2ac60061` and integrated close
  `eaf214f77a7cf62877571ddd2ef78b0e94b0497b`.
- `TEXT-02` accepts canonical Unicode/shaped authority at
  `691c6ab56a6603b5f4857552fa70148b11715f1c`.
- `TEXT-03` accepts bounded undo/layout/resource lifetime at
  `3b5af7b0341520781e1d286605aaf3e3e7dd9bbe`.
- `REND-01` accepts balanced transform recovery and cross-layer color/tint
  behavior through `REND-01A` `1aee4f41248251e1a365967ba1d655d49b04abbf`,
  `REND-01B` `9c1c0440385068ef58db5c6a34833f552c704c61`, and integrated
  close `365cfb0527a22965d51521e5e14feede733c5477`.
- `REND-02` accepts registered fractional-DPI paint/hit/caret/selection
  authority at `1239dd994619de3765d8cee05c5f8ddd34c2c6de`.

This closes audit §§6.8-6.10 and §§6.12-6.14 only inside the documented
canonical contracts and accepts only the text-owned portions of §§8.4, 10.2,
and 11.5. Earlier packet status remains in the chronology only where explicitly
time-qualified. Stage 5 begins with the root-owned `REND-ADR-01` decision; no
presenter or composition contract is inferred by this transition.

#### Tests run and results

- Accepted Stage 1 `c8fbf536023fcd089c9afda1b9af789dd4dbbc20`, Stage 2
  `5cf07b8b9a64083d31da687f348f4eeb001f1754`, and Stage 3
  `1f991113816f3c6b8ce9063a9d37ebe367109f2c` anchors, plus every full Stage 4
  anchor listed above, resolve as ancestors of the accepted base.
- All 25 close-specific focused commands passed:
  - core async liveness 15/15, observer 10/10, and color 2/2;
  - text ReadOnly 9/9, viewport 14/14, Unicode editing 12/12, Unicode layout
    23/23, shaped key 4/4, undo budget 12/12, and layout budget 6/6;
  - widget text field 125/125 and layout lifetime 5/5;
  - render reconciliation 8/8, resource snapshots 12/12, and color/alpha 3/3;
  - Vello public color 1/1, public translation 18/18, transform recovery 5/5,
    authority 6/6, cache 6/6, layouts 4/4, paths 4/4, and snapping 7/7; and
  - facade public API 11/11 plus Showcase library 132/132 and binary 25/25.
  All six final close workspace gates also passed: formatting, warning-denied
  all-target/all-feature workspace Clippy, all-feature workspace tests,
  all-feature workspace build, all-feature example checks, and warning-denied
  all-feature/no-deps workspace docs. `RUSTDOCFLAGS` was restored to unset.
- `REND-02` Issue #568 / PR #569 candidate
  `156ceaec62312669b30479f2f5e359346408dc1e` passed exact-SHA critics at
  P0/P1/P2=`0/0/0`, PR CI 29186376228, and exact-SHA three-OS CI 29186433862
  before squash merge `1239dd994619de3765d8cee05c5f8ddd34c2c6de`; main-push
  CI 29186580620 passed the merge SHA.
- The final `REND-02` tree passed authority 6/6, cache 6/6, layouts 4/4, paths
  4/4, snapping 7/7, the recorded affected-crate suites, warning-denied touched
  Clippy, and all six workspace gates; `RUSTDOCFLAGS` was restored to unset.
- This documentation integration passed `git diff --check`, exact eight-path
  equality, cross-document status/candidate/residual searches, and the frozen
  two-file `output/` path/size/SHA-256 comparison.

#### Remaining risks and deferred findings

Canonical retained text paths alone receive the authoritative guarantee;
compatibility paths remain qualified, and fallback paint cannot reconstruct
retained wrap or Unicode navigation state. Undo run barriers, terminal
text-resource rebuild behavior, and payload metrics that are not process RSS
remain explicit. Fractional translated rectangles retain the generic band of
at most 1.0001 physical pixels, and CPU scene evidence is not a final GPU pixel
golden.

Duplicate `TextLayoutCache` compatibility curation remains Stage 7 `API-01`, so
audit §11.7 is not globally closed. Vello's resolved gradient ramp remains a
source-verified dependency risk, and premultiplied payload validity remains
caller-owned. HDR/wide-gamut/ICC, final GPU pixels, presenter/swapchain
ownership, external textures, and public editor composition remain Stage 5 or
later. Broader image/resource/performance/lifecycle findings remain open unless
a separately accepted packet closed them. The repository remains foundation /
developer preview, not alpha-ready; no tag, publish, deployment, or release is
authorized.

## Packet Completion Template

Every packet review must use these exact headings and include commands plus concrete results:

```text
Changed files
Reasoning and contract decisions
Tests run and results
Remaining risks and deferred findings
```

Append one record per executed packet. Do not mark a stage complete until its acceptance gate passes. A passing gate advances to the next queued stage without new approval unless a Runway stop condition triggers.
