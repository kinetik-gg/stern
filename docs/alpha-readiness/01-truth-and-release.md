# Stage 1: Truth And Release Baseline

[Back to the alpha-readiness index](../alpha-readiness.md)

## Execution Contract

Campaign status: REND-03 is **Complete / Accepted**; REND-04 is **next**.

| Field | Decision |
| --- | --- |
| Status | Complete / Accepted; Stages 2-4 subsequently passed and Stage 5 is Current / Authorized with `REND-ADR-01` and `REND-03` accepted and `REND-04` next |
| Scope | Capability truth, provisional public boundary, showcase truth, and packageability scaffolding |
| Impact / confidence | High / High overall; `API-01` is Medium-high confidence |
| Campaign prerequisite | Stage 0 documentation gate, complete; campaign authorization recorded |
| Token checkpoint | Small-medium; executed bounded packet checks and advanced to Stage 2 after the accepted gate |

## Packets

| ID | Goal | Dependency | Impact / confidence | Ownership / delegation |
| --- | --- | --- | --- | --- |
| `ALPHA-00` | Replace binary completeness with Model/Paint/Input/Accessibility/Platform/Live Workflow evidence axes | Stage 0 | High / High | Root contract; one bounded worker |
| `API-01` | Define provisional stable/experimental alpha policy; defer final facade curation until `SHOW-02` | `ALPHA-00`; final checkpoint requires `SHOW-02` | High / Medium-high | Root-owned checkpoints |
| `SHOW-01` | Correct navigation, enabled-action truth, modal lifecycle, and bounded contradictory fixtures | `ALPHA-00` | High / High | Disjoint showcase workers only; otherwise serial |
| `REL-01` | Establish packageable manifests, metadata, changelog, publish order, and honest install docs | `ALPHA-00` | High / High | Isolated release worker, sequential with API/README work |

## Ownership And Overlap

During Stage 1, `SHOW-01` owned Z8 and could not overlap live-shell work in Z3;
`API-01` and `REL-01` shared Z7 and ran sequentially. No worker may count
metadata/model-only evidence as a stable capability axis. Package dry-runs do
not authorize a tag or publish.

## Acceptance Gate And Verification Expectations

The gate required stable claims to declare and prove their evidence axes,
experimental surfaces to be excluded from stable counts, bounded showcase
controls to stop implying nonexistent behavior, intended crates to pass package
dry-runs, and the public API policy to remain explicitly provisional. Each
packet task named deterministic checks for its exact files and recorded results
in `progress.md`.

At the Stage 1 gate, any contradictory capability claim, overlapping Z3/Z7/Z8
ownership, or attempt to freeze the final facade before `SHOW-02` would have
stopped the campaign. With no stop condition triggered, the accepted gate
advanced to the already authorized Stage 2 without new approval.

## Deferrals

Final `API-01` curation, prerelease tagging, publishing, and alpha claims remain deferred to Stage 7. Stage 1 packageability is scaffolding only.
