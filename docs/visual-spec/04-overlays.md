# Visual spec — Overlays: menus, dropdowns, tooltip, popover, modal, command palette

Authority: `00-language.md`. Family issue: #913.
Source: labs `.ui-menu`, `.ui-combo`, `.overlay-stage`, `.command-palette`;
DS `src/components/{menus,dropdowns,overlay-components,command-palette}.md`,
`src/behaviors/overlays.md`.

## Menu / context menu / dropdown list

Surface: fill `surface.overlay` `#181818` (D2 — labs show `#141414`; token rules),
border 1px `border.default`, `radius.md` 6, elevation 2 (`shadow.medium`),
padding 4 all sides, item gap 1 (labs) → normalize 0 with 1px optical gap
acceptable; **normative: gap 1** as labs. Opens at anchor + 4 offset.

Item: height 24 (`size.row.compact`), padding-inline 6 (`inline.compact` — labs 7),
radius.sm, grid: leading 16 (icon/check column) / label flex / trailing shortcut,
gaps 6. Label control type (11) secondary.

| Item state | Fill | Text |
|---|---|---|
| rest | transparent | secondary `#B8B8B8` |
| hover / active-path | S4 `#1C1C1C` | primary `#E8E8E8` |
| focused (kbd) | S4 + focus ring per 00 | primary |
| disabled | transparent | disabled `#666666` |
| selected option | (rest/hover fill) + check glyph 12 in `focus.indicator` `#4DB2FF` in leading column | per rest/hover |

Hover is NEUTRAL, never accent (00 doctrine). Shortcut column: meta type (9 mono)
muted; on hover stays muted. Group heading: padding 7/8/4, meta type UPPERCASE
+0.06em muted. Separator: 1px `border.subtle` full-width inset 4, 4 block margin.
Submenu indicator: caret-right icon 12 muted in trailing column; submenu opens
overlapping -4, same surface spec.

## Dropdown (select) trigger

It is a field-family control: height 24, S3 fill, `border.default`, radius.sm,
padding-inline 8, label 11 secondary + trailing caret-down 12 muted.
Hover/focus/disabled exactly per 02-fields single-line states (fill constant,
border promotes). Open state: border.strong (as focused, no ring unless kbd).

## Tooltip

Elevation 1: fill S3 `#181818`, border `border.default`, radius.sm 3,
`shadow.low`, padding 4/8, text detail type (10) secondary, max-width 280.
Optional shortcut suffix meta mono muted. Offset 6 from anchor. No arrow.

## Popover

Elevation 2, same surface recipe as menu (overlay fill, border.default,
radius.md, shadow.medium) with content padding 8 (`padding.panel`). No arrow.

## Modal

Scrim: `overlay.scrim` `#0B0B0B` at 38% opacity covering the window.
Panel: fill S2 `#141414`, border `border.strong`, radius.md, elevation 3
(`shadow.high`), min-width 320, max-width 560.
Header: height 34, padding-inline 12, title control-strong (11/600), border-b
`border.subtle`. Body: padding 12, body type. Footer: height 44, padding-inline
12, actions right-aligned, gap 8 (`gap.group`), border-t `border.subtle`.
Danger modals: title row leading icon 16 `status.danger.foreground`.

## Command palette

Anchored top-center, offset-top 32, width min(430, viewport−32).
Surface: fill S2 `#141414`, border `border.strong`, radius.md, elevation 3
(`shadow.high` — D7). Search row: height 38, grid 18/flex/auto, padding-inline 10,
gap 7, border-b `border.default`; input body type (12) primary on transparent;
leading search icon 12 muted; trailing esc-hint meta mono muted.
Results: padding 4; group heading = menu group heading; item height 28
(`size.row.standard`), grid 20/flex/auto, padding-inline 7, radius.sm,
text secondary.
Active item: `selection.background` `#0C8CE9` + white (this IS data selection),
kbd inherits at 72% opacity. Footer: height 26, fill S1, border-t `border.subtle`,
meta mono muted, hint items gap 12.

## Feedback toast stack

Anchor bottom-right inset 12, stack gap 8, width 320. Each toast = status banner
per 07 spec at elevation 2. Auto-dismiss per behavior spec; visuals only here.
