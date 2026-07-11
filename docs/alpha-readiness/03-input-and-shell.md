# Stage 3: Ordered Input And Shell

[Back to the alpha-readiness index](../alpha-readiness.md)

## Execution Contract

| Field | Decision |
| --- | --- |
| Status | In progress; `IN-01` merged, `IN-02` local audit and full gate passed pending exact-SHA three-OS CI |
| Scope | Sequence-preserving input, platform request execution, and pointer normalization |
| Impact / confidence | Critical / High (`IN-03` is High / High) |
| Campaign prerequisite | Stage 2 gate; campaign authorization recorded |
| Token checkpoint | Medium-large; remain serial through input-contract freeze |

## Packets

| ID | Goal | Dependency | Impact / confidence | Ownership |
| --- | --- | --- | --- | --- |
| `IN-01` | Preserve one ordered key/text/IME/pointer/focus/wheel stream and wire ordinary `KeyEvent.text` typing | Stage 2 gate | Critical / High | Root-owned contract |
| `IN-02` | Execute clipboard, URL, cursor, IME, repaint, and async shell results with one-frame request ownership | `IN-01` | Critical / High | Root integration |
| `IN-03` | Normalize line/pixel wheel, click counts, drag threshold, and drag-release click suppression | `IN-01`, `RT-02` | High / High | Root-owned while input contract is active |

`IN-01` now defines one canonical ordered stream with compatibility projections,
source-aware hardware text and preedit-driven IME behavior, one frame-local text
claim, event-time pointer localization, and deterministic mixed-mode conflict
diagnostics. Its depth-two remedy and independent re-review passed, as did the
complete local CI-equivalent gate. Existing pointer primitives intentionally remain snapshot-driven;
event-aware click, drag, and wheel policy stays in `IN-03`.

`IN-02` now uses one consumed Winit batch, ordered injectable shell services,
payload-free failures and debug output, targeted one-shot clipboard responses,
same-owner IME rectangle updates, stateful repaint replacement, strict
parseable-host HTTP(S) validation, and a live-loop rollover path that cannot
replay shell work after a recoverable surface failure. Help, the interactive
About control, and F1 route one fixed HTTPS Documentation action. Real Showcase
output crosses fake Winit cursor, IME, clipboard, URL, and repaint boundaries in
deterministic tests. Three independent depth-one re-reviewers closed the
depth-zero audit's four findings with no P0/P1/P2 findings. The complete local
CI-equivalent gate passes; exact-SHA three-OS CI, PR checks, and squash merge
remain before acceptance.

## Ownership And Overlap

`IN-01` and `IN-03` own Z2 and remain serial with text-input consumption. `IN-01` must replace the separate key/text collections with an ordered stream or equivalent sequence-preserving contract. `IN-02` owns Z3 and may not overlap `REND-03` or live `SHOW-01/02` changes.

## Acceptance Gate And Verification Expectations

Go only when hardware-style typing and the IME lifecycle work in the supported live shell; mixed key/text order is preserved; copy/cut/paste, URLs, cursor, IME rectangles, repaint, and async requests execute with one-frame ownership; and mouse/touchpad scroll, double-click, drag threshold, and click suppression are deterministic.

Packet tasks must include contract, core, adapter, and supported-shell checks appropriate to their owned paths. Event reordering, stale requests, Z2/Z3 overlap, and shell behavior with no recorded owner are stop conditions; otherwise, record the gate and advance to the already Authorized / Queued Stage 4 without new approval.

## Deferrals

Desktop/Unicode editing, presenter extraction, and showcase workflow integration remain later-stage work.
