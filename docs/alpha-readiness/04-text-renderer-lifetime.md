# Stage 4: Text, Renderer, And Lifetime

[Back to the alpha-readiness index](../alpha-readiness.md)

## Execution Contract

| Field | Decision |
| --- | --- |
| Status | Current / Authorized after the accepted Stage 3 gate at `1f99111` |
| Scope | Async liveness, desktop/Unicode text, bounded caches, and renderer correctness |
| Impact / confidence | Critical / Medium-high overall |
| Campaign prerequisite | Stage 3 gate; campaign authorization recorded |
| Token checkpoint | Very large; run 4A, checkpoint it, then continue to 4B only when 4A passes |

## Packets

| Lane | ID | Goal | Dependency | Impact / confidence | Ownership |
| --- | --- | --- | --- | --- | --- |
| 4A | `ASYNC-01` | Separate presence, incarnation, cancellation, ID reuse, and tombstone cleanup | `RT-03` | High / High | Isolated only after ownership freezes |
| 4A | `TEXT-01` | Desktop word movement/deletion, drag/double-click selection, caret scroll, multiline retention, true read-only | Stage 3, `RT-01`, `RT-03` | Critical / High | Root-owned text contract |
| 4A | `REND-01` | Balance invalid transform scopes; define premultiplied tint and cross-layer color-space semantics | Stage 3; root color-policy decision | High / High for transform; Medium confidence for color | Root policy; mechanical transform subset may isolate |
| 4B | `TEXT-02` | Grapheme, Unicode word, emoji, ligature, and mixed-bidi editing from authoritative clusters | `TEXT-01` | Critical / Medium-high | Root-owned text contract |
| 4B | `TEXT-03` | Bound/coalesce undo and impose generation/byte budgets on text layouts/resources | `TEXT-01`; ordered input frozen | High / Medium-high | Isolated only after text-store API freezes |
| 4B | `REND-02` | Use one authoritative text layout for paint, hit, caret, and selection at fractional DPI | `TEXT-02`, `REND-01` | Critical / Medium | Root integration |

## Ownership And Overlap

`ASYNC-01` shares Z1 with runtime ownership and cannot precede `RT-03`. `TEXT-01/02/03` and `REND-02` share Z4 and must not edit the same text files concurrently. `REND-01/02` share Z5. Color space and premultiplication are one root-owned cross-layer policy; no Vello leaf task may choose local semantics. Halt if Unicode work requires an unplanned shaping-engine replacement.

## Acceptance Gate And Verification Expectations

The 4A checkpoint requires deterministic desktop editing/read-only behavior, async incarnation cleanup, balanced invalid-transform recovery, and a documented/tested color/tint contract. Record and review that checkpoint before 4B; continue without intermediate approval only when it passes and no stop condition triggers.

The Stage 4 gate requires Unicode/grapheme/bidi fixtures; paint/hit/caret/selection agreement at scales 1.25, 1.5, and 1.75; asserted long-session text/undo/cache budgets; and proof that read-only differs from disabled. Packet tasks define exact deterministic checks. Passing the gate advances to the already Authorized / Queued Stage 5; a failed checkpoint halts the campaign.

## Deferrals

Presenter ownership, external textures, and public editor composition remain Stage 5 or later.
