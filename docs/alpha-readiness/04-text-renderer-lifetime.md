# Stage 4: Text, Renderer, And Lifetime

[Back to the alpha-readiness index](../alpha-readiness.md)

## Execution Contract

Campaign status: REND-03 is **Complete / Accepted**; REND-04 is **next**.

| Field | Decision |
| --- | --- |
| Status | Complete / Accepted through `REND-02` squash merge `1239dd994619de3765d8cee05c5f8ddd34c2c6de` and passing main-push CI |
| Scope | Async liveness, desktop/Unicode text, bounded caches, and renderer correctness |
| Impact / confidence | Critical / Medium-high overall |
| Campaign prerequisite | Stage 3 gate; campaign authorization recorded |
| Token checkpoint | Stage 4 gate passed; Stage 5 is Current / Authorized with `REND-ADR-01` and `REND-03` accepted and `REND-04` next |

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

`TEXT-03` was serialized into three root-owned packets. `TEXT-03A` bounds and
coalesces local undo while preserving public direct-edit atomicity. `TEXT-03B`
owns retained layout generations, eviction, and byte budgets, including
rejected preview churn. `TEXT-03C` owns incremental renderer-resource export
and resource byte lifetimes. The text-owned portions of audit §§8.4, 10.2, and
11.5 and roadmap `TEXT-03` closed after all three passed. `TEXT-03A` closed
through Issue #562 and PR #563 at squash merge `21be11c`. `TEXT-03B` closed
through Issue #564 and PR
#565 at squash merge `83e2847` after exact-SHA critics, PR CI, three-OS CI, and
main-push CI passed. `TEXT-03C` candidate
`61baa82693957bbb5b71e716c33ab0a133a6eb5f` closed Issue #566 through PR #567
and squash merge `3b5af7b0341520781e1d286605aaf3e3e7dd9bbe` after exact candidate critics, PR CI 29180858792,
three-OS run 29180863795, and main-push CI 29181022198 passed. Roadmap
`TEXT-03` and its retained text-resource lifetime findings are Complete /
Accepted.

`TEXT-01-PRE` is a root-owned shared-foundation prerequisite discovered by the
`TEXT-01` task gate. It adds event-time modifier state to the already accepted
ordered selection seam before either `ASYNC-01` or `TEXT-01` edits the shared
memory/runtime files. It is not a new audit roadmap ID and does not close any
desktop editing finding by itself.

`TEXT-01-PRE2` is the following root-owned prerequisite. It exposes the same
single-pass DomainDrag response as causal root-ordinal actions, including the
exact release that clicked, while keeping action metadata separate from
canonical drop authority. At the `TEXT-01-PRE2` checkpoint, `ASYNC-01` and
`TEXT-01` remained serialized behind its accepted merge at `00b944f` because
all three touched shared memory/runtime and campaign evidence files.
`ASYNC-01` had to squash before `TEXT-01`; both subsequently passed. The
dependency was file serialization, not a semantic text-liveness requirement.

## Accepted 4A Evidence

Checkpoint 4A is Complete / Accepted. Its accepted evidence covers async
incarnation cleanup, canonical desktop editing and true read-only behavior,
balanced invalid-transform recovery, and documented/tested sRGB, alpha, and
tint semantics. At that checkpoint, Stage 4 remained Current / Authorized at
4B; 4B subsequently passed through accepted `REND-02`.

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
At that checkpoint it did not close Stage 4: Unicode cluster authority, bounded
text-store resources, and authoritative fractional-DPI text layout remained
checkpoint 4B. Those responsibilities subsequently passed.

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

This closes audit §6.10 within canonical retained contracts: scalar word
movement/deletion, drag and double-click selection, caret-following horizontal
scroll, retained wrapped-multiline vertical scroll, true focusable/selectable/
copyable ReadOnly behavior, and visible caret-derived IME geometry are
deterministic on canonical retained `Ui` paths. Public free components remain
compatibility paths. At `TEXT-01` acceptance, Unicode cluster authority
remained `TEXT-02`, bounded undo/layout/resource budgets remained `TEXT-03`,
and authoritative fractional-DPI paint/hit/caret/selection agreement remained
`REND-02`; all three responsibilities subsequently passed within their
documented contracts.

The `TEXT-01` semantic prerequisites of `TEXT-02`, `TEXT-03`, and dependent
editor packets are satisfied, and accepted `REND-01` unblocked the renderer
side of 4B. `TEXT-02`, `TEXT-03`, and `REND-02` are Complete / Accepted.
Inspector/outliner still wait for their Stage 5 composition and collection
prerequisites. Checkpoints 4A and 4B are complete, and the integrated Stage 4
gate is accepted.

## `REND-02`: authoritative fractional-DPI text projection

Status: Complete / Accepted. Issue #568 closed through PR #569. Candidate
`156ceaec62312669b30479f2f5e359346408dc1e` passed three exact-SHA critics at
P0/P1/P2=`0/0/0`, PR CI run 29186376228, and exact-SHA Ubuntu/Windows/macOS run
29186433862 before authorized squash merge
`1239dd994619de3765d8cee05c5f8ddd34c2c6de`; main-push CI run 29186580620
passed the merge SHA.

Resolved registered layouts are now the sole Vello shaping and glyph-topology
authority. Exact positive axis-aligned transforms project each absolute point
through the full f64 affine, round in f64 exactly once, and narrow to f32 only
for Vello storage. Exact scaled font size and non-uniform outline ratio remain
unchanged, while every general affine remains on the raw unhinted path. The
private fallback store retains only logical layoutless or missing-resource
compatibility keys under the accepted 32 MiB and 120-idle-generation policy;
registered layouts never enter it.

Three human-authorized implementation/process correction packets preserve that
contract.
`REND-02-PC1` replaced the stale Showcase assertion that required
integer-rounded registered font sizes. `REND-02-PC2` added the `2.8_f32` at
1.25 f64-rounding witness and made identity-transform selection, caret, and
glyph-anchor parity strict at 1.25, 1.5, and 1.75. `REND-02-PC3` resolves a
registered layout before validating its ignored compatibility metrics, uses a
private deterministic finite-positive placeholder only for invalid command
fields on that registered path, preserves strict layoutless/missing-resource
validation and deterministic diagnostics, and leaves the fallback store empty
for registered layouts. `REND-02-PC4` was the documentation-only exact-SHA
evidence correction that synchronized the tracked readiness sections with PC3;
it changed no production behavior.

Focused evidence passes: authority 6/6, cache 6/6, layouts 4/4, paths 4/4,
and snapping 7/7; Vello 95 unit plus 24 integration tests; render 9 unit plus
23 integration tests and 1 compile-fail doc test; text 102 unit plus 80
integration tests; widget text-field conformance 125/125; facade 14 unit plus
11 public-API tests; Showcase 132 library plus 25 binary tests; and
warning-denied touched Vello/Showcase Clippy. All six workspace gates passed on
the final tree, and `RUSTDOCFLAGS` was restored to its prior unset state.

The authority guarantee is limited to canonical registered layouts.
Layoutless and unresolved-resource paint remains non-authoritative
compatibility behavior. Fractional command translations retain the existing
generic-rectangle band of at most 1.0001 physical pixels. CPU scene encoding
does not prove GPU raster or pixel identity. Duplicate `TextLayoutCache`
curation remains Stage 7 `API-01`; presenter ownership, external textures, and
public composition remain Stage 5.

## Integrated Stage 4 Acceptance

| Roadmap ID | Accepted responsibility | Integrated evidence |
| --- | --- | --- |
| `ASYNC-01` | Deterministic presence/incarnation separation, cancellation, ID reuse, observer validation, and tombstone cleanup | Issue #526, PR #527, squash `9d026c5f5a2108e79253e977868f60ec6522e9b8` |
| `TEXT-01` | Canonical desktop editing, pointer selection, caret-following viewports, IME geometry, and ReadOnly behavior distinct from Disabled | Implementation `93d6a5f775fea1bc416ec7bf360cd95b2ac60061`; integrated close `eaf214f77a7cf62877571ddd2ef78b0e94b0497b` |
| `TEXT-02` | Canonical grapheme, word, emoji, ligature, bidi, wrap, hit, caret, selection, and retained-widget authority | Final squash `691c6ab56a6603b5f4857552fa70148b11715f1c` |
| `TEXT-03` | Bounded/coalesced undo, retained-layout generation/count/byte policy, and incremental renderer-resource lifetime | Final squash `3b5af7b0341520781e1d286605aaf3e3e7dd9bbe` |
| `REND-01` | Balanced transform recovery and cross-layer straight-sRGB/alpha plus exact straight/premultiplied tint behavior | `REND-01A` `1aee4f41248251e1a365967ba1d655d49b04abbf`, `REND-01B` `9c1c0440385068ef58db5c6a34833f552c704c61`, close `365cfb0527a22965d51521e5e14feede733c5477` |
| `REND-02` | One registered-layout authority for fractional-DPI paint, hit, caret, and selection geometry | Issue #568, PR #569, squash `1239dd994619de3765d8cee05c5f8ddd34c2c6de` |

The integrated gate closes audit §§6.8-6.10 and §§6.12-6.14 only within the
documented canonical contracts above. It accepts only the text-owned portions
of §§8.4, 10.2, and 11.5; broader image, resource, performance, and lifecycle
findings remain open unless a separately accepted packet closed them. Audit
§11.7 is not globally closed because duplicate `TextLayoutCache` compatibility
curation remains final Stage 7 `API-01` work.

## Ownership And Overlap

During Stage 4 execution, `ASYNC-01` shared Z1 with runtime ownership and could
not precede `RT-03` or the accepted DomainDrag prerequisite. It also removed
authority-bearing memory Clone, so its runtime/harness/facade integration had
to finish before `TEXT-01`. `TEXT-01/02/03` and `REND-02` shared Z4 and could
not edit the same text files concurrently; `REND-01/02` shared Z5. Color space
and premultiplication remain one root-owned cross-layer policy; no Vello leaf
task may choose local semantics. The stage would have halted if Unicode work
required an unplanned shaping-engine replacement.

## Acceptance Gate And Verification Expectations

All six Stage 4 roadmap packets and the integrated gate are Complete / Accepted.
The accepted evidence proves deterministic async presence/incarnation/
cancellation cleanup; canonical desktop and ordered ReadOnly behavior distinct
from Disabled; Unicode grapheme, word, emoji, ligature, bidi, wrap, hit, caret,
and selection authority; tested generation/count/byte boundaries for undo,
retained layouts, renderer resources, and fallback caches; balanced transform
recovery and cross-layer sRGB/alpha/tint behavior; and registered paint/hit/
caret/selection geometry agreement at scale factors 1.25, 1.5, and 1.75.

At this gate, Stage 5 advanced to Current / Authorized with `REND-ADR-01`
next. ADR 0001 subsequently accepted that decision, `REND-03` is now Complete /
Accepted, and `REND-04` is next.
Every Stage 5 packet still requires its own deterministic task gate; a failed
checkpoint or unresolved ownership decision halts the campaign.

## Deferrals

Canonical retained text paths alone receive the authoritative guarantee;
compatibility paths remain qualified. Undo run barriers, terminal text-resource
rebuild behavior, and payload metrics that are not process RSS remain explicit.
Vello's resolved gradient ramp remains a source-verified dependency risk, and
premultiplied payload validity remains caller-owned. HDR/wide-gamut/ICC, final
GPU pixels, presenter/swapchain ownership, external textures, and public editor
composition remain Stage 5 or later. The repository remains foundation /
developer preview, not alpha-ready.
