# Stage 5: Presenter And Composition Foundations

[Back to the alpha-readiness index](../alpha-readiness.md)

## Execution Contract

Campaign status: REND-03 is **Complete / Accepted**; REND-04 is **next**.

| Field | Decision |
| --- | --- |
| Status | Current / Authorized; REND-03 is **Complete / Accepted**; REND-04 is **next**. |
| Scope | Presenter ownership/external textures and measured public composition foundations |
| Impact / confidence | Critical / Medium overall |
| Campaign prerequisite | Stage 4 gate, Complete / Accepted; campaign authorization recorded |
| Token checkpoint | Very large; presenter ownership and Showcase adoption are accepted through `REND-03`, so execute `REND-04` next before checkpointing the measured-`Ui` seam |

## Packets

| Lane | ID | Goal | Dependency | Impact / confidence | Ownership |
| --- | --- | --- | --- | --- | --- |
| Presenter | `REND-ADR-01` | [Decide device/queue/surface/external-texture ownership, sync, lifetime, recovery, offscreen, and multi-window boundary](../adr/0001-gpu-presenter-contract.md) | Accepted Stage 4 policy context | Critical / Medium | Root-only ADR; Complete / Accepted |
| Presenter | `REND-03` | Extract reusable Winit/Vello window, resize, recovery, submit, and present behavior from showcase-private code | `REND-ADR-01`, `IN-02` | Critical / Medium | Root integration; Complete / Accepted |
| Presenter | `REND-04` | Register/update/remove domain-owned native GPU textures without mandatory CPU snapshots | `REND-03` | Critical / Medium | Root integration |
| Composition | `LAYOUT-UI-01` | Measured row/column/grid/padding/stack/scroll allocation through public `Ui` APIs | `RT-01` | Critical / Medium-high | Root-owned shared seam |
| Composition | `OVL-UI-01` | Public painted menus, dropdowns, context/popover/tooltip/palette/modal behavior | `RT-02`, `RT-03`, `LAYOUT-UI-01` | High / Medium | Root arbitration; leaf work after seam freeze |
| Composition | `CHROME-UI-01` | Public toolbar, tab strip, status bar, and overflow behavior | Layout, overlay/input/action contracts | High / Medium-high | Isolated leaf after seams freeze |
| Composition | `COLL-UI-01` | Public virtual list/tree with scroll, keyboard, focus, selection, expansion, semantics | `RT-01`, `LAYOUT-UI-01` | High / High | Isolated after seams freeze |
| Composition | `COLL-UI-02` | Public table/grid with headers, two-axis scroll, sort, selection, resize | `COLL-UI-01` | High / High | Isolated; serial with `COLL-UI-01` |

## Ownership And Overlap

Presenter work owns Z3/Z5: no `REND-03` overlap with `IN-02` or live showcase changes. Accept `REND-ADR-01` before presenter implementation; ambiguity in GPU ownership is a stop. Composition work owns Z6: freeze `LAYOUT-UI-01` and shared theme/export seams before leaf delegation, and keep collection packets serial. Do not run speculative parallel lanes merely because their semantic dependencies differ.

## Acceptance Gate And Verification Expectations

[ADR 0001](../adr/0001-gpu-presenter-contract.md) accepts `REND-ADR-01` and
freezes the supported presenter/device/external-texture boundary. `REND-03` is
Complete / Accepted through reusable presenter and Showcase adoption evidence;
`REND-04` is next for native GPU texture registration. Then record and verify
that measured `Ui` APIs prove common composition without manual rectangles. After each checkpoint
passes, the campaign continues without intermediate approval; an ambiguity or
failed gate is a stop condition. The stage gate then requires a supported
presenter outside the showcase; proven domain GPU texture interoperability or
removal from the alpha promise; overlay compliance with Stage 2/3 contracts;
and rendered-input plus semantic tests for chrome and collections.

## Deferrals

General multi-window behavior, a reusable offscreen presenter, foreign-device
texture import, zero-copy, HDR/wide-gamut/ICC UI conversion, and additional
renderer backends remain deferred. ADR 0001 defines their boundary without
committing alpha implementation.
