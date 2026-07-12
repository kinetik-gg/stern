# Stage 4: Text, Renderer, And Lifetime

[Back to the alpha-readiness index](../alpha-readiness.md)

## Execution Contract

| Field | Decision |
| --- | --- |
| Status | Current / Authorized; checkpoint 4A, `TEXT-02`, and `TEXT-03A/B` are Complete / Accepted; `TEXT-03C` is the current implementation candidate |
| Scope | Async liveness, desktop/Unicode text, bounded caches, and renderer correctness |
| Impact / confidence | Critical / Medium-high overall |
| Campaign prerequisite | Stage 3 gate; campaign authorization recorded |
| Token checkpoint | Very large; 4A passed its checkpoint, and execution continues through the remaining 4B packets and Stage 4 gate |

## Packets

| Lane | ID | Goal | Dependency | Impact / confidence | Ownership |
| --- | --- | --- | --- | --- | --- |
| 4A | `ASYNC-01` | Separate presence, incarnation, cancellation, ID reuse, and tombstone cleanup | `RT-03`, accepted `TEXT-01-PRE2` merge | High / High | Root-owned shared foundation; read-only critics |
| 4A | `TEXT-01` | Desktop word movement/deletion, drag/double-click selection, caret scroll, multiline retention, true read-only | Stage 3, `RT-01`, `RT-03` | Critical / High | Root-owned text contract |
| 4A | `REND-01` | Balance invalid transform scopes; define premultiplied tint and cross-layer color-space semantics | Stage 3; root color-policy decision | High / High for transform; Medium confidence for color | Root policy; mechanical transform subset may isolate |
| 4B | `TEXT-02` | Grapheme, Unicode word, emoji, ligature, and mixed-bidi editing from authoritative clusters | `TEXT-01` | Critical / Medium-high | Root-owned text contract |
| 4B | `TEXT-03` | Bound/coalesce undo and impose generation/byte budgets on text layouts/resources | `TEXT-01`; ordered input frozen | High / Medium-high | Isolated only after text-store API freezes |
| 4B | `REND-02` | Use one authoritative text layout for paint, hit, caret, and selection at fractional DPI | `TEXT-02`, `REND-01` | Critical / Medium | Root integration |

`TEXT-02` executes as one serialized root-owned Z4 chain: `TEXT-02A` establishes
UAX #29 grapheme/word editing plus explicit caret affinity; `TEXT-02B` derives
authoritative shaped visual stops for ligatures and bidi hit/caret/selection;
`TEXT-02C` integrates that authority through canonical retained widgets,
ReadOnly, ordered mutation/re-resolution, pointer selection, and IME before the
roadmap ID closes. The subpackets never run concurrently. `TEXT-02A` closed
Issue #554 through PR #555 and squash merge `ac9a1e2`. `TEXT-02B` closed Issue
#556 through PR #557 and squash merge `676cb4e`; the follow-up wrap-delimiter
cell prerequisite closed Issue #559 through PR #560 and squash merge `2814a3c`.
`TEXT-02C` closed Issue #558 through PR #561 and squash merge `691c6ab` after
its exact-SHA critics, three-OS run 29174582250, and PR run 29174571824 passed.
Audit §6.9 and roadmap `TEXT-02` are Complete / Accepted.

`TEXT-02B` adds one owned, exact-source-bound `ShapedTextNavigation` derived
from the positioned cluster ranges already stored by cosmic-text. Construction
is all-or-nothing for line/run/glyph geometry, cluster overlap/direction, and
EGC coverage. Coordinate nodes retain same-position affinity aliases, while
physical bidi/wrap seams remain distinct. The same map owns visual character
and word motion, hit testing, caret rectangles, and selection spans. Existing
public shaped struct literals and byte-only geometry methods remain compatible.
Accepted `TEXT-02B` remains a qualified text-layer API. `TEXT-02C` supplies the
canonical retained widget, ReadOnly, pointer, ordered re-resolution, and IME
integration. The Unicode-authoritative alpha path requires a canonical retained
field configured with `TextLayoutStore`; free components and no-store
construction remain compatibility paths.

`TEXT-03` is serialized into three root-owned packets. `TEXT-03A` bounds and
coalesces local undo while preserving public direct-edit atomicity. `TEXT-03B`
owns retained layout generations, eviction, and byte budgets, including
rejected preview churn. `TEXT-03C` owns incremental renderer-resource export
and resource byte lifetimes. Audit §§8.4, 10.2, and 11.5 and roadmap `TEXT-03`
close only after all three pass. `TEXT-03A` closed through Issue #562 and PR
#563 at squash merge `21be11c`. `TEXT-03B` closed through Issue #564 and PR
#565 at squash merge `83e2847` after exact-SHA critics, PR CI, three-OS CI, and
main-push CI passed. Issue #566 tracks the current `TEXT-03C` implementation
candidate.

`TEXT-01-PRE` is a root-owned shared-foundation prerequisite discovered by the
`TEXT-01` task gate. It adds event-time modifier state to the already accepted
ordered selection seam before either `ASYNC-01` or `TEXT-01` edits the shared
memory/runtime files. It is not a new audit roadmap ID and does not close any
desktop editing finding by itself.

`TEXT-01-PRE2` is the following root-owned prerequisite. It exposes the same
single-pass DomainDrag response as causal root-ordinal actions, including the
exact release that clicked, while keeping action metadata separate from
canonical drop authority. `ASYNC-01` and `TEXT-01` remain serialized behind its
accepted merge at `00b944f` because all three touch shared memory/runtime and
campaign evidence files. `ASYNC-01` follows that merge and must squash before
`TEXT-01`; the dependency is file serialization, not a semantic text-liveness
requirement.

## Accepted 4A Evidence

Checkpoint 4A is Complete / Accepted. Its accepted evidence covers async
incarnation cleanup, canonical desktop editing and true read-only behavior,
balanced invalid-transform recovery, and documented/tested sRGB, alpha, and
tint semantics. Stage 4 remains Current / Authorized at 4B.

`ASYNC-01` is Complete / Accepted. Issue #526 closed through PR #527 and
squash-merged as `9d026c5`. Durable presence/incarnation, cancellation,
same-ID replacement, observer validation, tombstone cleanup, non-cloneable
authority, focused tests, all six workspace gates, three exact-SHA critics,
three-OS run 29146185811, and PR run 29146379516 passed.

`REND-01A` is Complete / Accepted. Issue #518 closed through PR #520 and
squash-merged as `1aee4f4`. Rejected non-finite and overflowing transform
begins now retain balanced recovery frames. Its local gates, exact-SHA critic,
three-OS run 29141679730, and PR checks passed. The `REND-01B` implementation
record below owns the cross-layer sRGB, alpha, tint, gradient, and image policy.

### `REND-01B`: sRGB, alpha, and tint contract

Status: Complete / Accepted. Issue #550 closed through PR #551. Candidate
`609ae127` passed all local gates and three exact-SHA critics, three-OS run
29165037981, and PR run 29165219725 before squash merge `9c1c044`.

#### Changed files

- `CHANGELOG.md`
- `crates/kinetik-ui-core/src/render.rs`
- `crates/kinetik-ui-core/tests/render_color_conformance.rs`
- `crates/kinetik-ui-render/src/lib.rs`
- `crates/kinetik-ui-render/tests/color_alpha_conformance.rs`
- `crates/kinetik-ui-vello/src/{geometry,image,sanitize,tests}.rs`
- `crates/kinetik-ui-vello/src/tests/color_alpha.rs`
- `crates/kinetik-ui-vello/tests/{render_color_conformance,render_translation_conformance}.rs`
- `docs/specs/{03-rendering-text-components,04-runtime-platform}.md`
- `docs/render-snapshots.md`
- `docs/alpha-readiness/{04-text-renderer-lifetime,progress}.md`

#### Reasoning and contract decisions

Core color is straight sRGB plus straight alpha while its const constructors
remain unchecked and source-compatible. Vello translation is the one
sanitization authority and creates deterministic command values before Peniko
mapping. Gradients explicitly select sRGB and premultiplied-alpha interpolation.
Image bytes stay in sRGB byte space; straight tint uses one two-factor rounding,
while premultiplied RGB uses one three-factor rounding that includes tint alpha.
The existing image/tint cache key remains valid because its signature includes
format, alpha, dimensions, and shared payload identity. Public resource
snapshot structures and text grammar remain unchanged.

#### Tests run and results

- Core color conformance passed 2/2 and render image/resource conformance passed
  3/3.
- Private Vello color/alpha conformance passed 7/7, including exact RGBA/BGRA
  byte goldens, the `64 * 64 * 135 -> 9` one-round witness, alpha-only cache
  invalidation, and texture upload metadata.
- Public submission conformance passed 1/1, including the premultiplied solid
  draw word, raw sRGB gradient stops, fallback text, shadow, tinted image, and
  texture resources. The exact all-occurrence sanitization/diagnostic-order
  test passed 1/1.
- Complete core, render, and Vello crate suites passed (including 180 core unit
  tests, 12 unchanged resource-snapshot conformance tests, 94 Vello unit tests,
  18 translation tests, and 5 retained transform-recovery tests). Formatting,
  warning-denied workspace Clippy, workspace tests, workspace build, all-feature
  example checks, and warning-denied workspace docs passed with the isolated
  `.target-rend01b` cache.
- Three exact-SHA critics reported P0/P1/P2=`0/0/0` on candidate `609ae127`.
  Ubuntu, Windows, and macOS passed run 29165037981; PR-context run 29165219725
  passed before PR #551 squash-merged as `9c1c044`.

#### Remaining risks and deferred findings

Vello 0.9 does not publicly expose its resolved 512-sample gradient ramp, so
the executable fence covers explicit Peniko fields/interpolation and public raw
encoded stops; resolved-ramp behavior remains source-verified dependency risk.
Premultiplied payload correctness is caller-owned and deliberately not scanned.
`InvalidGeometry` remains the alpha-cycle diagnostic name for invalid colors.
HDR, wide gamut, ICC conversion, external GPU resources, presenter ownership,
and CPU/GPU pixel goldens remain later work or explicit deferrals.

### `REND-01`: integrated renderer closure

Status: Complete / Accepted. Audit §§6.12-6.13 are closed by two serialized,
accepted packets. `REND-01A` closed Issue #518 through PR #520 and squash merge
`1aee4f4`; three-OS run 29141679730 and its PR checks passed, accepting balanced
recovery frames for rejected non-finite and overflowing transform begins.
`REND-01B` closed Issue #550 through PR #551 and squash merge `9c1c044` after
candidate `609ae127`, all local gates, three exact-SHA critics, three-OS run
29165037981, and PR run 29165219725 passed, accepting the cross-layer straight
sRGB/alpha, explicit gradient interpolation, and exact straight/premultiplied
tint contract.

Together with accepted `ASYNC-01` and `TEXT-01`, this closes checkpoint 4A.
It does not close Stage 4: Unicode cluster authority, bounded text-store
resources, and authoritative fractional-DPI text layout remain checkpoint 4B.

`TEXT-01` is Complete / Accepted at `93d6a5f` after this integrated evidence
closure. Its implementation was deliberately serialized into the following
root-owned packets:

| Packet | Issue / PR | Squash merge | Accepted responsibility |
| --- | --- | --- | --- |
| `TEXT-01-PRE` | #522 / #523 | `f2fd2d0` | Event-time selection modifiers |
| `TEXT-01-PRE2` | #524 / #525 | `00b944f` | Causal DomainDrag actions |
| `TEXT-01A` | #528 / #529 | `f448c40` | Scalar desktop word editing |
| `TEXT-01B1` | #530 / #532 | `4d25a2b` | Pure retained text viewport math |
| `TEXT-01B2` | #531 / #533 | `c191516` | Logical text-owner mode separate from IME |
| `TEXT-01B3-PRE` | #534 / #535 | `288657a` | Read-only ordered-input policy |
| `TEXT-01B3-PRE2` | #536 / #537 | `6df12e8` | Final root primary-press ordinal |
| `TEXT-01B3-PRE3` | #539 / #540 | `1b29284` | Completed same-frame pointer routing |
| `TEXT-01B3-PRE4` | #541 / #542 | `ec24e96` | Retained selection gesture anchor |
| `TEXT-01B3` | #538 / #543 | `9102293` | Canonical text-field kernel |
| `TEXT-01B4-PRE5` | #545 / #546 | `9d09d3c` | Ordered-input preview/claim provenance |
| `TEXT-01B4` | #544 / #547 | `93d6a5f` | Numeric/search/path/vector wrapper integration |

This closes audit §6.10: scalar word movement/deletion, drag and double-click
selection, caret-following horizontal scroll, retained wrapped-multiline
vertical scroll, true focusable/selectable/copyable ReadOnly behavior, and
visible caret-derived IME geometry are deterministic on canonical retained
`Ui` paths. Public free components remain compatibility paths. Unicode cluster
authority remains `TEXT-02`; bounded undo/layout/resource budgets remain
`TEXT-03`; authoritative fractional-DPI paint/hit/caret/selection agreement
remains `REND-02`.

The `TEXT-01` semantic prerequisites of `TEXT-02`, `TEXT-03`, and dependent
editor packets are satisfied, and accepted `REND-01` unblocks the renderer side
of 4B. `TEXT-02` and `TEXT-03A/B` are Complete / Accepted and `TEXT-03C` is the
current serialized implementation candidate. `REND-02` remains behind both
accepted `TEXT-02` and `REND-01` and follows `TEXT-03C` for shared evidence;
inspector/outliner still wait for their Stage 5 composition and collection
prerequisites. Checkpoint 4A is complete, while Stage 4 remains Current at 4B.

## Ownership And Overlap

`ASYNC-01` shares Z1 with runtime ownership and cannot precede `RT-03` or the
accepted DomainDrag prerequisite. It also removes authority-bearing memory
Clone, so its runtime/harness/facade integration must finish before `TEXT-01`.
`TEXT-01/02/03` and `REND-02` share Z4 and must not edit the same text files
concurrently. `REND-01/02` share Z5. Color space and premultiplication are one
root-owned cross-layer policy; no Vello leaf task may choose local semantics.
Halt if Unicode work requires an unplanned shaping-engine replacement.

## Acceptance Gate And Verification Expectations

The 4A checkpoint, `TEXT-02`, and `TEXT-03A/B` are Complete / Accepted with deterministic desktop and Unicode editing, bounded/coalesced local undo and retained layouts, async incarnation cleanup, balanced transform recovery, and documented/tested color/tint behavior. `TEXT-03C` continues 4B; continue without intermediate approval only while packet gates pass and no stop condition triggers.

The Stage 4 gate requires Unicode/grapheme/bidi fixtures; paint/hit/caret/selection agreement at scales 1.25, 1.5, and 1.75; asserted long-session text/undo/cache budgets; and proof that read-only differs from disabled. Packet tasks define exact deterministic checks. Passing the gate advances to the already Authorized / Queued Stage 5; a failed checkpoint halts the campaign.

## Deferrals

Presenter ownership, external textures, and public editor composition remain Stage 5 or later.
