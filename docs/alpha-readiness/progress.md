# Alpha-Readiness Progress And Evidence

[Back to the alpha-readiness index](../alpha-readiness.md)

Stages 0-1 are Complete. Stage 2 is Current / Authorized. Stages 3-7 are Authorized / Queued for continuous sequential execution without intermediate approval. Every packet still has to pass its deterministic gates, and any Runway stop condition halts the active packet or stage.

Campaign workflow policy: `create-if-available` issues, `create-if-gates-pass` pull requests, and `squash-after-gates` merges. Tagging, package publishing, and an alpha release remain outside this authorization.

## Stage 0: Plan And Baseline

Status: Complete. This closes the documentation task only; Stage 1 is Current / Authorized under the recorded campaign authorization.

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

- Runtime, input, text, presenter, component, quality, and release risks remain unresolved until their authorized packets execute and pass.
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
evidence-backed curation decision, and all later audit packets remained open.

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
Documentation remains disabled until shell URL execution exists.

#### Tests run and results

- Focused About lifecycle: 5/5; showcase: 121 library plus 31 binary tests and
  doc tests.
- Full repository gates, exact seven-path scope, and independent critic passed.

#### Remaining risks and deferred findings

Keyboard focus trapping, global-shortcut suppression, dedicated modal/Close
semantics, and opening documentation remain deferred to overlay, input, and
accessibility packets.

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

Status: In progress. `RT-01` passed its task gate and independent depth-two
remedy review. `RT-02` and `RT-03` remain queued before the integrated Stage 2
gate can close.

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

## Packet Completion Template

Every packet review must use these exact headings and include commands plus concrete results:

```text
Changed files
Reasoning and contract decisions
Tests run and results
Remaining risks and deferred findings
```

Append one record per executed packet. Do not mark a stage complete until its acceptance gate passes. A passing gate advances to the next queued stage without new approval unless a Runway stop condition triggers.
