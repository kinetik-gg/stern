# Stage 6: Public Editor Vertical Slice

[Back to the alpha-readiness index](../alpha-readiness.md)

## Execution Contract

| Field | Decision |
| --- | --- |
| Status | Current / Authorized; Stage 5 is Complete / Accepted and `DOCK-UI-01` is next |
| Scope | Reusable editor components and one coherent public-facade workflow |
| Impact / confidence | High-critical / Medium overall |
| Campaign prerequisite | Stage 5 gate, Complete / Accepted; campaign authorization covers every non-deferred packet |
| Token checkpoint | Very large; execute and review non-deferred packets individually, beginning with `DOCK-UI-01`, before `SHOW-02` |

## Packets

| ID | Goal | Dependency | Impact / confidence | Ownership |
| --- | --- | --- | --- | --- |
| `DOCK-UI-01` | Theme-driven public Dock/Frame/Panel/tab/splitter/drop-preview painter | `LAYOUT-UI-01`, `CHROME-UI-01` | High / High | Root shared component |
| `DOCK-UI-02` | Public Dock controller/persistence for select, close, drag, split, merge, resize, join, swap, focus, round-trip | `DOCK-UI-01`, `RT-02`, `RT-03` | Critical / High | Root integration |
| `INSP-UI-01` | Live property grid with sections, validation/help/status, reset, keyframe affordances | `LAYOUT-UI-01`, `COLL-UI-01`, `TEXT-01` | High / Medium-high | Isolated module after seams freeze |
| `INSP-UI-02` | Real select/color/asset/path overlay or shell picker flows with commit/cancel/focus | `OVL-UI-01`, `INSP-UI-01`, `IN-02` | High / Medium | Isolated module; root shell edge |
| `OUT-UI-01` | Public outliner with expansion, selection, keyboard, rename, state toggles, drag/drop, context actions | `COLL-UI-01`, `OVL-UI-01`, `TEXT-01` | High / Medium | Isolated module |
| `ASSET-UI-01` | Public asset browser with live filter/sort, grid/list, rename, selection, preview, drag/drop, actions | `COLL-UI-01/02`, `OVL-UI-01` | High / Medium | Isolated; not concurrent with shared collections |
| `VIEW-UI-01` | Consolidated viewport texture placement, clip, focus, cursor, navigation, conversion, actions, semantics | `RT-01`, `REND-04`, `LAYOUT-UI-01` | Critical / Medium | Root boundary |
| `VIEW-UI-02` | Painted theme-driven viewport tools/handles emitting application-owned requests | `VIEW-UI-01`, `RT-02` | High / Medium | Root integration; bounded leaf work possible |
| `SYS-UI-01` | Persistent job, diagnostics, and feedback components with real time/repaint/action flow | Layout/chrome/action contracts; `ASYNC-01` when applicable | High / Medium-high | Isolated module |
| `TL-UI-01` | Optional timeline painter with render/hit truth | Separate scope authorization; Stage 5 foundations | Deferred / Medium | Deferred |
| `TL-UI-02` | Optional timeline scrub/select/zoom/edit controller | `TL-UI-01` | Deferred / Medium | Deferred |
| `NG-UI-01` | Optional node-graph render truth for every hit target | Separate scope authorization; Stage 5 foundations | Deferred / Medium | Deferred |
| `NG-UI-02` | Optional node-graph pan/select/move/box-select/connect controller | `NG-UI-01` | Deferred / Medium | Deferred |
| `SHOW-02` | Facade-only workflow: select/rename, inspect/edit, filter/drag asset, manipulate viewport, save in-memory state | Selected Stage 6 packets, `REND-03/04`, Stages 2-5 gates | Critical / Medium | Root-only integration |

## Ownership And Overlap

Shared Dock, viewport, and `SHOW-02` integration stay root-owned. Inspector, outliner, asset browser, and system feedback may delegate only after dependencies freeze and exact module paths are disjoint. No worker may concurrently edit shared collections, theme/exports/`Ui`, or Z8 showcase files. Public facade/prelude work waits for component implementation.

## Acceptance Gate And Verification Expectations

Each non-deferred packet gets its own gate review. A passing review advances to the next dependency-ready packet without intermediate approval; any Runway stop condition halts execution. The stage gate requires the selected coherent workflow to use only the facade and reusable presenter path; shared components are not hand-painted in the showcase; enabled actions produce distinct application-state outcomes; and rendered-input, semantic, persistence, and workflow evidence is present rather than metadata/model-only proof.

## Deferrals

`TL-UI-01`, `TL-UI-02`, `NG-UI-01`, and `NG-UI-02` remain visible but excluded from stable alpha unless separately authorized and completed. Production persistence beyond required Dock/showcase state is also deferred.
