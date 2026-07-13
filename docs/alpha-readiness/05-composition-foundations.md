# Stage 5: Presenter And Composition Foundations

[Back to the alpha-readiness index](../alpha-readiness.md)

## Execution Contract

Campaign status: Stage 6 is **Complete / Accepted** through `SHOW-02` squash
merge `f38805e` and passing main Linux CI run `29285719629`. Stage 7 is
**Current / Authorized** with `PERF-01` **next**.

Integrated `DOCK-UI-01`, `DOCK-UI-02`, `VIEW-UI-01`, `VIEW-UI-02`,
`INSP-UI-01`, `INSP-UI-02`, `OUT-UI-01`, `ASSET-UI-01`, `SYS-UI-01`, and
`SHOW-02` are **Complete / Accepted**.

Stage 7 is **Current / Authorized**.

Kinetik UI remains a foundation/developer-preview; this packet does not tag, publish, deploy, release, or claim alpha readiness.

| Field | Decision |
| --- | --- |
| Status | Complete / Accepted through `COLL-UI-02` squash merge `98f4aec` and passing main CI run `29265615424`; Stage 6 subsequently passed and Stage 7 is Current / Authorized with `PERF-01` next |
| Scope | Presenter ownership/external textures and measured public composition foundations |
| Impact / confidence | Critical / Medium overall |
| Campaign prerequisite | Stage 4 gate, Complete / Accepted; campaign authorization recorded |
| Token checkpoint | Gate passed; Stage 6 packets subsequently passed and Stage 7 is Current / Authorized with `PERF-01` next |

## Packets

| Lane | ID | Goal | Dependency | Impact / confidence | Ownership |
| --- | --- | --- | --- | --- | --- |
| Presenter | `REND-ADR-01` | [Decide device/queue/surface/external-texture ownership, sync, lifetime, recovery, offscreen, and multi-window boundary](../adr/0001-gpu-presenter-contract.md) | Accepted Stage 4 policy context | Critical / Medium | Root-only ADR; Complete / Accepted |
| Presenter | `REND-03` | Extract reusable Winit/Vello window, resize, recovery, submit, and present behavior from showcase-private code | `REND-ADR-01`, `IN-02` | Critical / Medium | Root integration; Complete / Accepted |
| Presenter | `REND-04` | Register/update/remove domain-owned native GPU textures without mandatory CPU snapshots | `REND-03` | Critical / Medium | Root integration; Complete / Accepted |
| Composition | `LAYOUT-UI-01` | Measured row/column/grid/padding/stack/scroll allocation through public `Ui` APIs | `RT-01` | Critical / Medium-high | Root-owned shared seam; Complete / Accepted |
| Composition | `OVL-UI-01` | Public painted menus, dropdowns, context/popover/tooltip/palette/modal behavior | `RT-02`, `RT-03`, `LAYOUT-UI-01` | High / Medium | Root arbitration; Complete / Accepted |
| Composition | `CHROME-UI-01` | Public toolbar, tab strip, status bar, and overflow behavior | Layout, overlay/input/action contracts | High / Medium-high | Root-owned shared scene; Complete / Accepted |
| Composition | `COLL-UI-01` | Public virtual list/tree with scroll, keyboard, focus, selection, expansion, semantics | `RT-01`, `LAYOUT-UI-01` | High / High | Complete / Accepted |
| Composition | `COLL-UI-02` | Public table/grid with headers, two-axis scroll, sort, selection, resize | `COLL-UI-01` | High / High | Complete / Accepted |

## Ownership And Overlap

Presenter work owns Z3/Z5: no `REND-03` overlap with `IN-02` or live showcase changes. Accept `REND-ADR-01` before presenter implementation; ambiguity in GPU ownership is a stop. Composition work owns Z6: the accepted `LAYOUT-UI-01` measured-`Ui` seam is frozen for leaf delegation; shared theme/export seams must still be coordinated, and collection packets remain serial. Do not run speculative parallel lanes merely because their semantic dependencies differ.

## Acceptance Gate And Verification Expectations

[ADR 0001](../adr/0001-gpu-presenter-contract.md) accepts `REND-ADR-01` and
freezes the supported presenter/device/external-texture boundary. `REND-03` is
Complete / Accepted through reusable presenter and Showcase adoption evidence;
`REND-04` is Complete / Accepted through native GPU golden, recovery, producer
example, and extracted archive proof. `LAYOUT-UI-01` is Complete / Accepted
through deterministic core allocation, public measured `Ui` containers, and
facade/Showcase dogfood without caller-computed rectangles. `OVL-UI-01` is
Complete / Accepted through pure navigation and typeahead models plus the
public painted scene's pointer arbitration, primitives, action intents, and
ordered semantics. `CHROME-UI-01` is Complete / Accepted through stable-key
overflow projection plus the public painted scene's pointer arbitration,
primitives, typed intents, action dispatch, and ordered semantics.
`COLL-UI-01` is Complete / Accepted through stable-ID cursor navigation and
reconciliation plus public fixed-height virtual list/tree scenes with bounded
10,000-row materialization, scroll, keyboard focus/reveal, selection,
expansion, renderer-neutral paint, pointer arbitration, and ordered semantics.
`COLL-UI-02` is Complete / Accepted through public headers/cells, bounded
100,000-row materialization, two-axis retained scroll, application-owned sort
intents, stable row/cell selection, two-dimensional keyboard focus/reveal,
constrained resize requests, renderer-neutral paint, pointer arbitration, and
ordered table semantics.

The integrated stage gate passed through `COLL-UI-02` squash merge
`98f4aec4c091438d8a86fee05c8c65ed9e96a5f2`, PR CI run `29265455564`,
and main CI run `29265615424`. The accepted presenter exists outside the
Showcase, domain GPU texture interoperability is proven for the supported
boundary, overlays comply with the Stage 2/3 contracts, and collections have
rendered-input plus semantic evidence. Stage 6 subsequently passed, and Stage 7
is Current / Authorized with `PERF-01` next. Each later checkpoint follows the continuous
campaign policy; an ambiguity or failed gate is a stop condition.

## Deferrals

General multi-window behavior, a reusable offscreen presenter, foreign-device
texture import, zero-copy, HDR/wide-gamut/ICC UI conversion, and additional
renderer backends remain deferred. ADR 0001 defines their boundary without
committing alpha implementation.

Collection MVPs remain fixed-height. Variable-height rows, horizontal column
virtualization, arbitrary custom row/cell bodies, multi/range table selection,
editing/clipboard, grouped headers, multi-sort/filter execution, auto-fit,
column reordering, collection drag/drop/context menus, typeahead, inline rename,
and painted scrollbars remain later or explicitly deferred work. These
deferrals do not reopen the accepted large-data, input, focus, selection,
expansion, resize, paint, or semantic contracts.
