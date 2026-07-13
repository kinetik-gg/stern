# Stage 7: Quality And Alpha Gate

[Back to the alpha-readiness index](../alpha-readiness.md)

## Execution Contract

| Field | Decision |
| --- | --- |
| Status | Current / Authorized; Stage 6 and `SHOW-02` are Complete / Accepted, with `PERF-01` next |
| Scope | Performance, Vello visuals, accessibility boundary, CI, final API, and release decision |
| Impact / confidence | High / High after upstream gates; `ALPHA-GATE` is Critical / High |
| Campaign prerequisite | Satisfied through `SHOW-02` squash merge `f38805e` and passing main Linux CI run `29285719629` |
| Token checkpoint | Large; review evidence packet-by-packet before the final release decision |

## Packets

| ID | Goal | Dependency | Impact / confidence | Ownership |
| --- | --- | --- | --- | --- |
| `PERF-01` | Collect frame/repaint/cache/semantic/collection/texture metrics and enforce workload budgets | `SHOW-02`, `TEXT-03`, stable collections/presenter | High / Medium-high | Isolated harness; root sets budgets |
| `VIS-01` | Vello-backed visual regression at full/compact layouts and scales 1, 1.25, 1.5, 2 | `SHOW-02`, stable presenter, `REND-01/02` | High / Medium-high | Isolated visual worker |
| `A11Y-01` | Implement a native adapter or precisely document a semantic-output-only alpha boundary | `SHOW-02`, stable semantic tree | High / High | Root decision; isolated follow-up |
| `CI-01` | Pre-tag Linux/Windows/macOS, MSRV, packages, docs, advisories, licenses, controlled rendering | `REL-01`, `SHOW-02`, `VIS-01` contract | High / High | Isolated CI worker |
| final `API-01` | Curate facade/prelude around the proven public vertical path and migration notes | `SHOW-02` | High / Medium-high | Root checkpoint; same audit ID as Stage 1 |
| `ALPHA-GATE` | Run acceptance suite, notes, package order, and prerelease tag/release decision | All packets above | Critical / High | Root gate; explicit release authority required |

## Ownership And Overlap

Freeze `SHOW-02` fixtures before delegating performance, visual, or CI work. `VIS-01` owns Z5/Z8 evidence and must use Vello-backed output; CPU raster evidence is not a release oracle. Final `API-01`, `A11Y-01`, and `ALPHA-GATE` decisions are root-owned. Z7 release/API/CI edits are serialized.

## Acceptance Gate And Verification Expectations

Each packet must prove its acceptance contract with deterministic commands. The release-candidate gate includes the repository's formatting, clippy, workspace test, build, example-check, and documentation commands, plus package/platform, performance, Vello visual, accessibility-boundary, advisory, and license evidence required by its packet.

Go to an alpha release decision only when every applicable audit exit criterion is proven or explicitly removed from the promise, all deferred findings remain visible, and full CI-equivalent checks pass. A packageability baseline is not enough. Campaign tasks may create issues, open pull requests, and squash merge after their gates under the recorded policy; stop before any tag, package publish, or alpha release unless the user grants that exact authority.

## Deferrals

Native accessibility may remain deferred only if `A11Y-01` explicitly establishes a semantic-output-only boundary. Timeline, node graph, floating native windows, broad multi-window behavior, additional backends, and broader production persistence remain excluded unless separately authorized.
