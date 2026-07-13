# Stage 0: Plan And Baseline

[Back to the alpha-readiness index](../alpha-readiness.md)

## Execution Contract

Campaign status: integrated `OVL-UI-01` is **Complete / Accepted**;
`CHROME-UI-01` is **next**, and `COLL-UI-01` remains queued behind the frozen
measured-`Ui` and overlay seams.

Integrated `REND-04`, `LAYOUT-UI-01`, and `OVL-UI-01` are **Complete /
Accepted**.

Stage 5 remains **Current / Authorized**; Stages 6-7 remain **Authorized / Queued**.

Kinetik UI remains a foundation/developer-preview; this packet does not tag, publish, deploy, release, or claim alpha readiness.

| Field | Decision |
| --- | --- |
| Status | Complete; documentation only; no audit roadmap ID closes here |
| Impact / confidence | High / High |
| Dependency | Accepted architecture gate at audited revision `32b45f2` |
| Token checkpoint | Documentation verification passed; Stages 1-4 subsequently completed and Stage 5 is Current / Authorized with presenter, external-texture, measured-layout, and overlay work accepted; `CHROME-UI-01` is next |

## Scope And Work Items

| Work item | Output | Ownership |
| --- | --- | --- |
| Validate the plan | One acyclic Stage 0-7 campaign with 43 unique audit IDs | Root documentation task |
| Normalize the ledger | Current packet names, dependencies, impact/confidence, ownership, and deferrals | Root documentation task |
| Publish the mirror | Canonical index, split stage files, and progress/evidence log | Root documentation task |
| Prepare later work | Stage 1 scope and gates for the authorized sequential campaign | Root planning only |

This stage may not edit Rust source, tests, manifests, workflows, README, release policy, or existing specifications. It may not claim source/test verification.

## Dependency Model

The conservative campaign sequence is:

```text
Stage 0 -> Stage 1 -> Stage 2 -> Stage 3 -> Stage 4 -> Stage 5 -> Stage 6 -> Stage 7
```

This is a token-spend and integration rule. Packet tables in later files record semantic prerequisites; a packet that is semantically independent still waits for the preceding stage gate. All non-deferred Stage 1-7 work shares the recorded campaign authorization.

## Ownership And Overlap Map

Only one active task may own a zone unless accepted tasks prove exact disjoint files.

| Zone | Normalized live paths | Conflicting packets |
| --- | --- | --- |
| Z1 runtime ownership | `kinetik-ui-core/src/{memory,runtime,interaction,accessibility}/**`; widget `ui/{frame,behavior,passive,output}.rs` | `RT-01/02/03`, `ASYNC-01`, parts of `LAYOUT-UI-01` |
| Z2 ordered input | core `input.rs`; text `edit.rs`; widget `text_controls.rs`; Winit `input.rs` | `IN-01`, `IN-03`, `TEXT-01/02` |
| Z3 live shell/presenter | Winit `requests.rs`; showcase `live.rs`; presenter integration | `IN-02`, `REND-03`, live `SHOW-01/02` work |
| Z4 text store/layout | `kinetik-ui-text/src/{boundary,selection,edit,layout,store,cache,undo}.rs`; Vello `text.rs` | `TEXT-01/02/03`, `REND-02` |
| Z5 render/color/resources | core `render.rs`; render `lib.rs`; Vello `translation,image,encoding,text,renderer.rs` | `REND-01/02/03/04`, `VIS-01` |
| Z6 public UI/theme | widget `ui/**`, components, overlays, chrome, collections, core theme, facade/preludes | `LAYOUT-UI-01`, Stage 5 UI, Stage 6 UI, `API-01` |
| Z7 release/public docs | manifests, `README.md`, release/changelog/migration docs | `REL-01`, `API-01`, `CI-01`, `ALPHA-GATE` |
| Z8 showcase editor | showcase `app.rs`, `main.rs`, `editor/**`, `app/runtime/**` | `SHOW-01/02`, component adoption, `VIS-01` |

## Acceptance Gate And Verification Expectations

Stage 0 completed only after all ten target documents existed, all 43 unique IDs were represented, index-local links resolved, dependencies and overlap zones matched the accepted plan, and documentation-only checks passed. Stages 1-4 subsequently completed, and Stage 5 is Current / Authorized with `REND-ADR-01`, `REND-03`, integrated `REND-04`, `LAYOUT-UI-01`, and `OVL-UI-01` accepted; `CHROME-UI-01` is next, while `COLL-UI-01` remains queued behind the frozen measured-`Ui` and overlay seams. A stale path, invented finding, changed alpha scope, or write outside the allowed paths remains a stop condition.

Verification is limited to Markdown/file/link/ID review, `git diff --check`, targeted search, and targeted status. No source or test gate is claimed.

## Deferrals

Stage 0 closes documentation only and changes neither the product label from foundation/developer preview nor any implementation finding. Implementation now proceeds at Stage 5 under the separate Stage 1-7 campaign authorization.
