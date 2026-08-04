# Visual Conformance Methodology

This is the method doc for bringing Stern's widget style recipes into
conformance with the design system's visual language. Family issues
(buttons #910, and the fields/choice/overlay/chrome/collections/status/
deferred-surfaces families after it) follow this document verbatim.

## Authority

`docs/visual-spec/00-language.md` and the numbered family files
(`01-buttons.md`, `02-fields.md`, ...) are **the resolved, normative visual
specification**. They were synthesized once from `../stern-design-system`
(`theme/labs.css`, `generated/rust/stern_tokens.rs`, `src/components/*.md`)
with a fixed precedence documented at the top of `00-language.md`, and their
divergence table (`00-language.md` §Divergence) already records every
labs-vs-token conflict found (D1-D7) with an owner-accepted resolution.

**A family issue implements `docs/visual-spec/*.md` directly.** There is no
DS lookup step: read the family file (plus `00-language.md` for tiers, focus,
motion, and typography, which the family files assume as background), and
implement exactly the values in its per-state tables. Do not re-derive values
from `../stern-design-system` — that repository is background only for
family issues (see §Spec-file maintenance below for the one case where DS
sources matter again).

## Conflict handling

If a family file is silent or ambiguous for a case an issue needs (a state
combination it doesn't table, a component it doesn't mention):

1. Check `docs/design-system-tokens.md` §Exceptions and divergences first —
   the value may already be a recorded, decided gap.
2. If not, do not guess. Either extend the existing pattern in the most
   conservative, lowest-invention way and say so explicitly in the PR body
   (e.g. "table X doesn't cover state Y; treated it as Z because ..."), or
   stop and leave an issue comment describing the gap.
3. Log anything discovered that is out of scope for the current issue to
   `KNOWN-GAPS.md` (see the `## Visual conformance (Issue #910)` section for
   the shape: what's missing, why it's out of scope, file/line pointers).
   Never drop a gap silently.

## Test pattern

For each component, for each state applicable to it
(idle/hover/pressed/focused/disabled/selected — not every component uses
every state), assert the **resolved** recipe output against the spec's
per-state table:

- fill (background brush)
- text/icon color (foreground)
- border color and width
- corner radius
- padding and min-height per the density ladder, where the recipe or
  adjoining control-metrics tokens carry them

Tests live next to the family's existing conformance tests, not in a new
file, and are named for the family/component so concurrent work on other
recipe families (or on unrelated metrics tests in the same theme test files)
stays easy to merge around. The button family's reference shape:

- `crates/stern-core/src/theme/model.rs` — `Theme::button_variant`, the
  recipe function itself. Doc comment on the function cites the spec file
  and states any non-obvious precedence rule (e.g. "chosen" vs "pressed").
- `crates/stern-core/src/theme/tests.rs` —
  `button_variants_match_visual_spec_state_colors` (Standard/Ghost/Danger,
  full 7-state matrix: normal, hovered, chosen, pressed, chosen+hovered,
  pressed+hovered, disabled) and
  `primary_button_uses_exact_accent_roles_and_bounded_state_precedence`
  (Primary, kept separate since its accent-role precedence predates this
  issue and 01-buttons.md's Primary table has no "selected" row to pin
  against).
- `crates/stern-core/tests/button_focus_recipe_conformance.rs` — asserts
  `focused` never changes fill/border/text on its own (the two-layer focus
  ring is a separate, additive paint step per `00-language.md` §Focus
  model), across the same state matrix.

A state combination not explicitly tabled in the family file (e.g. a
"selected and hovered" icon button, when the file only tables "selected")
gets resolved by the same precedence the recipe function itself uses — do
not invent a second, untested precedence rule in the test.

## Rule: tokens only (STERN-COL-001)

Recipes resolve exclusively through mapped theme tokens
(`self.colors.*`, `self.radii.*`, `self.strokes.*`, `self.spacing.*` /
`self.controls.*`) — never a raw hex literal or an inline numeric constant
standing in for a token. The only allowed exception is a value the spec
itself marks as a **derived constant** (`00-language.md` divergences D4-D6:
node-selection-ring grays, window-close-hover red, palette-shadow spread) —
those are hand-rolled precisely because no token exists yet, and each is
commented `// derived: not a DS token, see visual-spec 00 §Divergence` at
its definition site.

## Spec-file maintenance (not a family-issue concern)

The extraction method below is what produced `docs/visual-spec/*.md` in the
first place and is what a future refresh of those files would repeat. It is
**out of scope for ordinary family issues** — implementers read the already-
resolved spec files per §Authority above. Keep this section only for whoever
re-synthesizes `docs/visual-spec/*` after a design-system update:

1. Start from `../stern-design-system/theme/labs.css` — the token-faithful
   reference rendering. It is authoritative for anatomy, state behavior, and
   composition.
2. Cross-check every color/metric against
   `../stern-design-system/generated/rust/stern_tokens.rs` (vendored at
   `crates/stern-core/src/theme/generated_tokens.rs`) — authoritative for the
   exact value of anything labs.css only implies visually.
3. Read `../stern-design-system/src/components/*.md` for normative
   requirement prose and keyboard/interaction tables labs.css doesn't
   capture (e.g. "every icon button requires an accessible label").
4. Treat `../stern-design-system/theme/stern.css` (the doc-site sketch) as
   anatomy hints only — never a value source; its numbers are not normative.
5. `../stern-design-system/generated/specimen-data.json` (machine-resolved
   colors/metrics used by the DS's own labs) is a cross-check when labs.css
   and the vendored tokens disagree in a way `00-language.md`'s existing
   divergence table doesn't already cover.
6. Where labs and tokens disagree, the token wins and the disagreement is
   recorded as a new divergence-table row (`00-language.md` §Divergence),
   not silently resolved — divergences need an explicit owner decision
   before a family issue can treat them as normative, exactly like D1-D7
   were resolved on 2026-08-03.
7. Record any exception found in this pass (a stern value with no DS token,
   or a token group not yet wired into `ThemeColors`/`SizeScale`/etc.) in
   `docs/design-system-tokens.md` §Exceptions and divergences, not inline in
   the visual-spec file itself — that file stays pure "these are the
   values," not "here's how we know."
