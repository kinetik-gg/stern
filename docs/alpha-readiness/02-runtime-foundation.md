# Stage 2: Shared Runtime Foundation

[Back to the alpha-readiness index](../alpha-readiness.md)

## Execution Contract

Campaign status: Stage 5 is **Complete / Accepted** through integrated
`COLL-UI-01` and `COLL-UI-02`; Stage 6 is **Current / Authorized** with
`DOCK-UI-01` **next**.

Integrated `REND-ADR-01`, `REND-03`, `REND-04`, `LAYOUT-UI-01`,
`OVL-UI-01`, `CHROME-UI-01`, `COLL-UI-01`, and `COLL-UI-02` are
**Complete / Accepted**.

Stage 6 is **Current / Authorized**; Stage 7 remains **Authorized / Queued**.

Kinetik UI remains a foundation/developer-preview; this packet does not tag, publish, deploy, release, or claim alpha readiness.

| Field | Decision |
| --- | --- |
| Status | Complete / Accepted at `5cf07b8`; Stages 3-5 subsequently passed and Stage 6 is Current / Authorized with `DOCK-UI-01` next |
| Scope | Shared coordinate, arbitration, and interaction-ownership invariants |
| Impact / confidence | Critical / High |
| Campaign prerequisite | Stage 1 gate; campaign authorization recorded |
| Token checkpoint | Large; executed serial/root-owned and checkpointed each invariant |

## Packets

| ID | Goal | Dependency | Impact / confidence | Ownership |
| --- | --- | --- | --- | --- |
| `RT-01` | One scoped local-to-screen transform and effective clip for paint, hit, semantics, focus, drag, debug, and IME | Stage 1 | Critical / High | Root-owned, serial |
| `RT-02` | Topmost, modal, z-order, and effective-clip-aware pointer arbitration | `RT-01` | Critical / High | Root-owned, serial |
| `RT-03` | Reconcile capture, focus, active, text/IME, and drag owners against widgets seen this frame | `RT-01`, `RT-02` | Critical / High | Root-owned, serial |

## Ownership And Overlap

During Stage 2, all three packets owned Z1 and could not run concurrently.
`RT-01` inventoried manual offset consumers so collection/inspector callers did
not double-apply offsets. `RT-02` began with a recorded contract decision
reconciling topmost arbitration with the specification's immediate widget-call
model; an unrelated retained-tree expansion would have stopped the packet.
`ASYNC-01` waited for accepted `RT-03` and subsequently passed in Stage 4.

## Acceptance Gate And Verification Expectations

The gate required deterministic nested-transform/scroll tests to show identical
geometry for paint, hit, semantics, focus, debug, and IME; clipped children to
be inert; topmost overlays to block underlying interaction; removed widgets to
retain no interaction/platform ownership; and collection/inspector consumers to
stop double-applying offsets.

Each packet received its own bounded task and checks. An unresolved invariant or
public-contract gap would have stopped the campaign; with none remaining, the
accepted Stage 2 gate advanced to the already authorized Stage 3 without new
approval.

`RT-03` uses frame-local widget presence, kept distinct from eligibility and
duplicate registration, to cancel removed interaction owners at `end_frame`.
Its independent critic and the integrated Stage 2 gate passed after one
fixture-only depth-one remedy. Stages 3-5 subsequently passed, and Stage 6 is
Current / Authorized with `DOCK-UI-01` next under the continuous campaign
authorization.

## Deferrals

At Stage 2 acceptance, ordered input, shell execution, async incarnation policy,
and component work remained out of scope. Stages 3-4 subsequently accepted the
ordered-input, shell, async, and Stage 5 composition portions; public editor
component work remains Stage 6 or later.
