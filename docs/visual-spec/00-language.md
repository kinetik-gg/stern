# Stern Visual Language — normalized from the design system

This directory is the **resolved, normative visual specification** for stern's
components. It was synthesized once from the design system
(`../stern-design-system`) with this precedence:

1. `theme/labs.css` — the token-faithful reference renderings ("intentionally map
   one-to-one to Candidate tokens"). Authority for anatomy, state behavior, and
   composition.
2. `generated/rust/stern_tokens.rs` (vendored at
   `crates/stern-core/src/theme/generated_tokens.rs`) — authority for every value.
3. `src/components/*.md` — normative requirement statements and keyboard tables.
4. `theme/stern.css` — doc-site sketch; anatomy hints only, its values are NOT normative.

Where labs and tokens disagreed, tokens win and the divergence is recorded in
§Divergence table below. Implementation agents follow these files verbatim —
**no visual judgment calls in family issues**; anything unclear becomes an issue
comment, not a guess.

All units are logical pixels. All colors are the dark theme (the only normative
theme). `token:` names refer to `generated_tokens::COLORS/SPACING/RADII/SIZES/...`.

## Surface tiers

| Tier | Role | Token | Value |
|---|---|---|---|
| S0 | workspace / canvas / sunken wells | `color.semantic.surface.workspace` / `.sunken` | `#0B0B0B` |
| S1 | application background, section headers in panels | `.surface.application` | `#111111` |
| S2 | panels, menus, cards, bars | `.surface.panel` | `#141414` |
| S3 | controls at rest, raised panels, overlays | `.surface.control` / `.raised` / `.overlay` | `#181818` |
| S4 | hover | `.surface.hover` / `.control_hover` | `#1C1C1C` |
| — | pressed | `.surface.control_pressed` | `#2A2A2A` (see divergence D1) |
| — | control disabled | `.surface.control_disabled` | `#141414` |

Rule: a component's resting fill is exactly one tier above its container. Never
skip two tiers. Canvas-like regions (dock bodies, node canvas, viewport) sit on S0
with an optional 16px grid/dot pattern at ~2.8% white.

## Border tiers

| Tier | Token | Value | Use |
|---|---|---|---|
| subtle | `border.subtle` | `#222222` | hairline separators, bar borders, row dividers |
| default | `border.default` | `#2A2A2A` | control borders at rest, panel/menu outlines |
| strong | `border.strong` | `#3D3D3D` | hover borders, selected-tab ring, active frame |
| focused | `border.focused` | `#4DB2FF` | (focus uses the ring, below — not the border) |
| invalid | `border.invalid` | `#F18A90` | invalid fields (see fields spec) |
| disabled | `border.disabled` | `#222222` | disabled controls |

Stroke width is always `stroke.default` = 1. `stroke.emphasis` = 2 only where a
spec file says so (check glyph, slider-thumb ring, wires).

## Text tiers

| Tier | Token | Value | Use |
|---|---|---|---|
| primary | `content.primary` | `#E8E8E8` | hovered/active control text, titles, field input text |
| secondary | `content.secondary` | `#B8B8B8` | resting control text, labels, list rows |
| muted | `content.muted` | `#999999` | field labels, meta, group headings, placeholders |
| disabled | `content.disabled` | `#666666` | disabled anything |
| on-accent | `content.on_accent` | `#FFFFFF` | text on accent fills |
| link | `content.link` | `#259CF0` | links |

## Typography scale (derived from labs — see divergence D5)

Families from `FONT_FAMILIES`: `ui` = Inter, `brand` = Space Grotesk, `mono` = Space Mono.

| Step | Size/LH | Weight | Family | Use |
|---|---|---|---|---|
| body | 12 / 1.35 | 400–500 | ui | field input text, palette search, body copy |
| control | 11 / 1 | 500 | ui | buttons, menu items, labels, rows, tabs |
| control-strong | 11 / 1 | 600 | ui or brand | frame/panel titles, section headers |
| detail | 10 / 1.3 | 400–500 | ui | secondary detail lines, feedback detail |
| meta | 9 / 1 | 500–700 | mono | shortcuts (kbd), group headings (UPPERCASE +0.06em), status bars, numeric readouts, badges |

Numeric readouts and kbd are always `mono` with tabular figures.
Brand (`Space Grotesk` 600) appears ONLY in application titlebar identity and
frame/lab titles.

## Focus model (universal)

Two-layer ring drawn OUTSIDE the control bounds, never changing layout
(`FOCUS_CHANGES_LAYOUT_BOUNDS = false`):
1. separator: 1px (`stroke.focus.separator`) in `focus.separator` `#0B0B0B`
2. ring: 1px (`stroke.focus.primary`) in `focus.ring` `#4DB2FF`

CSS-equivalent: `0 0 0 1px #0B0B0B, 0 0 0 2px #4DB2FF`. Applies identically to
buttons, fields, checks, radios, switch tracks, sliders, tabs, menu items
(combined with hover fill), color selectors. Focus never recolors the control body.

## Selection vs hover doctrine

- **Hover is always neutral**: fill S4 `#1C1C1C`, text promotes one tier
  (secondary→primary). Menus and rows NEVER hover in accent.
- **Selection is always accent**: `selection.background` `#0C8CE9` fill +
  `selection.foreground` `#FFFFFF` text. Used by: selected collection rows, tree
  rows, palette active item, text selection. Selected-row meta text = white at 78%.
- **Chosen-but-not-selection state** (selected tab, segmented choice, pressed-in
  toggle button): NEUTRAL — S3 fill + `border.strong` ring, NOT accent.
  Accent marks *data selection*, not *mode choice*. A selected menu option shows a
  `#4DB2FF` check glyph, not an accent row.

## Geometry ladder

| Metric | Token | Value |
|---|---|---|
| control heights | `size.control.{xs,sm,md,lg}` | 20 / 24 / 28 / 32 |
| standard control | — | **24** (`sm`) is the default control height everywhere |
| rows | `size.row.{compact,standard}` | 24 / 28 |
| panel header | `size.panelHeader` | 30 |
| workspace bar | `size.workspaceBar` | 40 |
| tab | `size.tab` | 28 (within a 24-item strip container; see 03) |
| icons | `size.icon.{sm,md,lg}` | 12 / 16 / 20 (see divergence D3) |
| splitter | `size.handle.visual` / `.hit` | 1 visual / 7 hit |
| radius | `radius.{sm,md,lg,full}` | 3 / 6 / 12 / 9999 |
| control padding | `spacing.padding.control.inline.default` / `.compact` / `.block` | 8 / 6 / 4 |
| gaps | `spacing.gap.{icon_label,control_tight,group,section}` | 4 / 4 / 8 / 16 |
| panel padding | `spacing.padding.panel` | 8 |

Radius rules: controls/items/chips = `radius.sm` 3; menus, cards, palettes,
segmented containers, editor frames = `radius.md` 6; never `lg` on controls.

## Elevation & shadows

| Level | Token | Shadow | Use |
|---|---|---|---|
| 0 | `elevation.none` | none | in-flow UI |
| 1 | `elevation.low` | `shadow.low` 0 2 6 / 32% | tooltips, small cards |
| 2 | `elevation.medium` | `shadow.medium` 0 6 18 / 42% | menus, dropdowns, popovers, viewport overlays |
| 3 | `elevation.high` | `shadow.high` 0 12 36 / 52% | modals, command palette |

Overlay scrim: `overlay.scrim` `#0B0B0B` at 38% opacity (modal only).

## Motion

`durationMs.fast` 80ms + `easing.standard` cubic-bezier(0.2,0,0,1) for ALL
hover/press/check/switch transitions. `durationMs.normal` 120ms for overlay
open/close opacity. Nothing animates position except the switch knob. Honor
reduced-motion by dropping to `durationMs.instant`.

## Status colors

Each status has a 4-tuple `{surface, border, foreground, strong}` from
`color.status.*`: danger `#1B1314/#3D292B/#F18A90/#D9535B`, warning
`#1A1711/#3A3326/#F0C66D/#D9A441`, success `#121A15/#29372E/#72D998/#39B868`,
info `#101820/#25343F/#6CBFFF/#0C8CE9`. Banners use surface+border+foreground;
solid indicators use strong.

## Divergence table (labs vs tokens — **resolutions accepted by the owner, 2026-08-03**)

The "Normative here" column is final. Implementation follows it without further
review; the labs/DS sources should eventually be updated to match (tracked as a
design-system follow-up, not an alpha.1 task).

| # | Site | labs.css | Token | Normative here |
|---|---|---|---|---|
| D1 | button pressed fill | `#0B0B0B` (sink-to-bg) | `surface.control_pressed` `#2A2A2A` | **token** `#2A2A2A` |
| D2 | menu surface | panel `#141414` | `surface.overlay` `#181818` | **token** `#181818` |
| D3 | control icon size | 14px | `size.icon.sm/md` 12/16 | **12** in ≤24 controls, **16** in ≥28 |
| D4 | node selection ring | grays `#686868`/`#505050` | (no token) | labs grays, recorded as derived constants |
| D5 | type scale 9/10/11/12 | (labs only) | (no size tokens exist) | labs scale, propose upstreaming as tokens |
| D6 | window close hover | `#C42B1C` | (no token) | keep (platform convention), derived constant |
| D7 | palette shadow | 0 18 44 / 56% | `shadow.high` 0 12 36 / 52% | **token** shadow.high |

Derived constants (D4–D6) live in one place in stern-core theme, documented as
`// derived: not a DS token, see visual-spec 00 §Divergence`.
