# Stage 3: Ordered Input And Shell

[Back to the alpha-readiness index](../alpha-readiness.md)

## Execution Contract

| Field | Decision |
| --- | --- |
| Status | Complete / Accepted at `1f99111`; all four serial packets merged and the final exact-SHA gates passed |
| Scope | Sequence-preserving input, platform request execution, and pointer normalization |
| Impact / confidence | Critical / High (`IN-03` is High / High) |
| Campaign prerequisite | Stage 2 gate; campaign authorization recorded |
| Token checkpoint | Medium-large; executed serially through the input-contract freeze |

## Packets

| ID | Goal | Dependency | Impact / confidence | Ownership |
| --- | --- | --- | --- | --- |
| `IN-01` | Preserve one ordered key/text/IME/pointer/focus/wheel stream and wire ordinary `KeyEvent.text` typing | Stage 2 gate | Critical / High | Root-owned contract |
| `IN-02` | Execute clipboard, URL, cursor, IME, repaint, and async shell results with one-frame request ownership | `IN-01` | Critical / High | Root integration |
| `IN-03A` | Normalize line/pixel wheel units and calculate live click counts | `IN-02`, `RT-02` | High / High | Root-owned input adapter contract |
| `IN-03B` | Add drag threshold, drag-release click suppression, and ordered selection gestures | `IN-03A` | High / High | Root-owned pointer transition contract |

`IN-01` now defines one canonical ordered stream with compatibility projections,
source-aware hardware text and preedit-driven IME behavior, one frame-local text
claim, event-time pointer localization, and deterministic mixed-mode conflict
diagnostics. Its depth-two remedy and independent re-review passed, as did the
complete local CI-equivalent gate. Existing pointer primitives intentionally
remain snapshot-driven; accepted `IN-03` owns event-aware click, drag, and wheel
policy.

`IN-02` now uses one consumed Winit batch, ordered injectable shell services,
payload-free failures and debug output, targeted one-shot clipboard responses,
same-owner IME rectangle updates, stateful repaint replacement, strict
parseable-host HTTP(S) validation, and a live-loop rollover path that cannot
replay shell work after a recoverable surface failure. Help, the interactive
About control, and F1 route one fixed HTTPS Documentation action. Real Showcase
output crosses fake Winit cursor, IME, clipboard, URL, and repaint boundaries in
deterministic tests. Three independent depth-one re-reviewers closed the
depth-zero audit's four findings with no P0/P1/P2 findings. The complete local
CI-equivalent gate, exact-SHA three-OS CI, PR checks, and squash merge passed;
issue `#512` is closed and squash `e151b111` is accepted.

`IN-03A` consumes canonical wheel events with typed line/pixel provenance, a
private 40-unit line step, exact logical pixels, per-component sanitization, and
one sign inversion. Empty canonical streams keep the legacy logical magnitude.
The live Winit path now calculates click counts from inclusive 500 ms/four-unit
press boundaries, carries counts through matching releases, resets invalid
history deterministically, and retains the explicit-count compatibility API.
Focused core, routing, spatial, Winit, showcase, and warning-denied Clippy gates
passed. Three depth-one re-reviewers closed the DPI-evidence and rustdoc findings
with no P0/P1/P2 findings, and the complete local CI-equivalent gate passed.
Exact-SHA three-OS CI run `29135844832`, PR checks, and squash merge
`889c3762` passed; issue `#514` is closed.

`IN-03B` folds nonempty canonical primary/secondary transitions once, retains
a private four-current-scope-unit inclusive threshold latch, and suppresses
release clicks after crossing. Only domain draggables publish drag sources.
The runtime also retains original root event ordinals in a private spatial
sidecar and exposes neutral captured-selection actions without changing public
input or common response layouts. A matching ordered text claim exposes editing
events with the same ordinals, enabling later accepted `TEXT-01` to merge
without pointer reparsing. Empty canonical streams remain compatible.
Its final-depth remedy resolves composite numeric scrub interaction once as a
domain drag, preserves sequential cleanup provenance and causal cancellation
metadata, and retains ReleaseAll as a global spatial fence. Unrelated behavior
cannot erase an owner's earlier move or release, and wheel mutation stops at the
same fence. Closed plans declare domain-drag source intent, select same-frame
ordinary ownership from the first causal press, and validate threshold/release
evidence in the source transform and clip before routing the first causal
release. Canonical unplanned commits fail closed while empty-stream legacy drop
behavior remains compatible. Planned active drags and release commits are
source/target evaluation-order invariant, split button owners preserve
pre-fence output without stale hover/cursor publication, and selection
cancellation cannot replay on a second same-frame claim. Forty-six focused
core adversarial tests and forty-four widget taxonomy tests passed; the complete
six-command workspace gate passed on the final evidence candidate.

## Ownership And Overlap

During Stage 3, `IN-03A` and `IN-03B` owned Z2 and executed serially with
text-input consumption. `IN-03B` could not start until A's click metadata was
squash-merged and then had to freeze the ordered selection-gesture seam before
`TEXT-01`. `ASYNC-01` and `TEXT-01` could not overlap B's memory/runtime
ownership; both successors subsequently passed in Stage 4.

## Acceptance Gate And Verification Expectations

The gate required hardware-style typing and the IME lifecycle to work in the
supported live shell; mixed key/text order to remain preserved; copy/cut/paste,
URLs, cursor, IME rectangles, repaint, and async requests to execute with
one-frame ownership; and mouse/touchpad scroll, double-click, drag threshold,
and click suppression to be deterministic.

Packet tasks included contract, core, adapter, and supported-shell checks
appropriate to their owned paths. Event reordering, stale requests, Z2/Z3
overlap, and shell behavior with no recorded owner would have stopped the
campaign. Those checks passed; at the Stage 3 gate, the campaign advanced Stage
4 to Current / Authorized without requiring new approval. Stage 4 subsequently
passed, and Stage 5 is now Current / Authorized with `REND-ADR-01` accepted and
`REND-03` next.

## Deferrals

At Stage 3 acceptance, desktop/Unicode editing, presenter extraction, and showcase workflow integration remained later-stage work. Stage 4 subsequently accepted the canonical desktop/Unicode scope; presenter extraction and broader showcase workflow integration remain Stage 5 or later.
