# Stage 2: Shared Runtime Foundation

[Back to the alpha-readiness index](../alpha-readiness.md)

## Execution Contract

| Field | Decision |
| --- | --- |
| Status | Complete / Accepted at `5cf07b8`; Stage 3 subsequently passed and Stage 4 is Current |
| Scope | Shared coordinate, arbitration, and interaction-ownership invariants |
| Impact / confidence | Critical / High |
| Campaign prerequisite | Stage 1 gate; campaign authorization recorded |
| Token checkpoint | Large; run serial/root-owned and checkpoint each invariant |

## Packets

| ID | Goal | Dependency | Impact / confidence | Ownership |
| --- | --- | --- | --- | --- |
| `RT-01` | One scoped local-to-screen transform and effective clip for paint, hit, semantics, focus, drag, debug, and IME | Stage 1 | Critical / High | Root-owned, serial |
| `RT-02` | Topmost, modal, z-order, and effective-clip-aware pointer arbitration | `RT-01` | Critical / High | Root-owned, serial |
| `RT-03` | Reconcile capture, focus, active, text/IME, and drag owners against widgets seen this frame | `RT-01`, `RT-02` | Critical / High | Root-owned, serial |

## Ownership And Overlap

All three packets own Z1 and may not run concurrently. `RT-01` must inventory manual offset consumers so collection/inspector callers do not double-apply offsets. `RT-02` starts with a recorded contract decision reconciling topmost arbitration with the specification's immediate widget-call model; stop if it expands into an unrelated retained-tree rewrite. `ASYNC-01` waits for `RT-03`.

## Acceptance Gate And Verification Expectations

Go only when deterministic nested-transform/scroll tests show identical geometry for paint, hit, semantics, focus, debug, and IME; clipped children are inert; topmost overlays block underlying interaction; removed widgets retain no interaction/platform ownership; and collection/inspector consumers no longer double-apply offsets.

Each packet receives its own bounded task and checks. An unresolved invariant or public-contract gap is a stop condition; otherwise, record the gate and advance to the already Authorized / Queued Stage 3 without new approval.

`RT-03` uses frame-local widget presence, kept distinct from eligibility and
duplicate registration, to cancel removed interaction owners at `end_frame`.
Its independent critic and the integrated Stage 2 gate passed after one
fixture-only depth-one remedy. Stage 3 subsequently passed, and Stage 4 is
Current / Authorized under the continuous campaign authorization.

## Deferrals

Ordered input, shell execution, async incarnation policy, and component work remain out of scope for this stage.
