# Kinetik UI Alpha-Readiness Plan

This is the canonical human-facing index for the alpha-readiness campaign audited at revision `32b45f2`. The current product label is **foundation / developer preview**: the repository is not yet alpha-ready, and packageability alone must not be presented as release readiness.

Runway state controls execution details such as executor, depth, and gate status. These tracked documents preserve the accepted scope and review contract. If this roadmap and an active Runway task disagree, stop and reconcile them before implementation.

## Authorization And Status

Stages 0-4 are **Complete**; Stage 4 is **Complete / Accepted** through the accepted `REND-02` merge `1239dd9`. Stage 5 is **Current / Authorized** with `REND-ADR-01` next. Stages 6-7 remain **Authorized / Queued** and execute in order as their prerequisite gates pass. The Stage 1-7 campaign is authorized for continuous sequential execution without intermediate approval, but any Runway stop condition halts the active packet or stage.

The campaign workflow policy is `create-if-available` for issues, `create-if-gates-pass` for pull requests, and `squash-after-gates` for merges. Those permissions do not authorize a tag, package publish, alpha release, or a claim that unresolved findings are fixed.

| Stage | Status | Scope | Spend checkpoint |
| --- | --- | --- | --- |
| [0. Plan And Baseline](alpha-readiness/00-plan-and-baseline.md) | Complete; documentation only | Publish the packet ledger, dependencies, overlap rules, gates, and deferrals | Documentation gate passed |
| [1. Truth And Release](alpha-readiness/01-truth-and-release.md) | Complete / Accepted | Capability truth, provisional API boundary, showcase truth, packageability baseline | Gate passed at `c8fbf53` |
| [2. Runtime Foundation](alpha-readiness/02-runtime-foundation.md) | Complete / Accepted | Coordinates, arbitration, and interaction ownership | Gate passed at `5cf07b8` |
| [3. Input And Shell](alpha-readiness/03-input-and-shell.md) | Complete / Accepted | Ordered input, platform requests, and pointer normalization | Gate passed at `1f99111` |
| [4. Text, Renderer, And Lifetime](alpha-readiness/04-text-renderer-lifetime.md) | Complete / Accepted | Async liveness, desktop/Unicode text, bounded caches, renderer correctness | Gate passed through accepted `REND-02` merge `1239dd9` |
| [5. Composition Foundations](alpha-readiness/05-composition-foundations.md) | Current / Authorized | Presenter ADR/path, external textures, measured layout, overlays, chrome, collections | Very large; `REND-ADR-01` next, then checkpoint the shared-`Ui` seams |
| [6. Editor Vertical Slice](alpha-readiness/06-editor-vertical-slice.md) | Authorized / Queued | Dock, inspector, outliner, assets, viewport, feedback, and public workflow | Very large; gate non-deferred packets individually |
| [7. Quality And Alpha Gate](alpha-readiness/07-quality-and-alpha-gate.md) | Authorized / Queued | Performance, visuals, accessibility boundary, CI, final API and release decision | Large; no tag or publish without explicit authority |

See [Progress And Evidence](alpha-readiness/progress.md) for the current authorization record and the required packet-completion format.

## Roadmap Ledger

The campaign contains 43 unique audit roadmap IDs. `API-01` has a provisional checkpoint in Stage 1 and a final checkpoint in Stage 7, but remains one audit ID.

| Stage | Audit roadmap IDs |
| --- | --- |
| 1 | `ALPHA-00`, `API-01`, `SHOW-01`, `REL-01` |
| 2 | `RT-01`, `RT-02`, `RT-03` |
| 3 | `IN-01`, `IN-02`, `IN-03` |
| 4 | `ASYNC-01`, `TEXT-01`, `TEXT-02`, `TEXT-03`, `REND-01`, `REND-02` |
| 5 | `REND-ADR-01`, `REND-03`, `REND-04`, `LAYOUT-UI-01`, `OVL-UI-01`, `CHROME-UI-01`, `COLL-UI-01`, `COLL-UI-02` |
| 6 | `DOCK-UI-01`, `DOCK-UI-02`, `INSP-UI-01`, `INSP-UI-02`, `OUT-UI-01`, `ASSET-UI-01`, `VIEW-UI-01`, `VIEW-UI-02`, `SYS-UI-01`, `TL-UI-01`, `TL-UI-02`, `NG-UI-01`, `NG-UI-02`, `SHOW-02` |
| 7 | `PERF-01`, `VIS-01`, `A11Y-01`, `CI-01`, final `API-01`, `ALPHA-GATE` |

Stage 0 closes no audit roadmap ID; it publishes the contract under which those IDs may later be executed.

## Dependency And Stop/Go Policy

Campaign order is conservative: Stage 0 -> 1 -> 2 -> 3 -> 4 -> 5 -> 6 -> 7. It controls token spend and shared-file integration. A packet's semantic dependencies are narrower and appear in its stage file; semantic independence never bypasses campaign sequencing.

- One roadmap packet becomes one fresh bounded Runway task unless an accepted package explicitly splits it.
- Root tasks start at depth 0 and remedies stop at depth 2.
- Only one active implementation packet may own an overlap zone unless both tasks prove exact disjoint files.
- Metadata/model-only evidence cannot satisfy stable Paint, Input, Accessibility, Platform, or Live Workflow axes.
- Passing a stage gate advances to the next queued stage without new approval; every task still obeys the recorded Runway stop conditions.
- A failed gate, unresolved public-contract decision, undeclared overlap, or required out-of-scope change is a stop.
- Issue creation, pull-request creation, and squash merge follow the campaign policy above; tagging, publishing, and release claims require separate authority.

Timeline, node graph, native accessibility adapters, floating native windows, broad multi-window behavior, additional renderer backends, and production persistence beyond required Dock/showcase state are deliberately deferred unless separately authorized.
