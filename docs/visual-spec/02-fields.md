# Visual spec — Fields

Authority: `00-language.md`. Family issue: #911.
Source: labs `.ui-field*`, `.ui-unit`, `.text-edit-*`; DS `src/components/fields.md`.

## Single-line field (text / search / path / numeric)

Height `size.control.sm` 24, padding-inline 8, radius.sm 3, border 1px,
input text = body (12/1, ui) — labs show 11 for the box but 12 for edit runs;
**normative: 12** for input text, 11 for affixes. Caret: 1px `focus.ring` `#4DB2FF`,
height = line box. Selection highlight: `selection.background` `#0C8CE9` fill,
`selection.foreground` white text. IME composition: 1px underline `#4DB2FF`.

| State | Fill | Border | Text |
|---|---|---|---|
| idle | S3 `#181818` | `border.default` `#2A2A2A` | primary `#E8E8E8`; placeholder muted `#999999` |
| hover | S3 (unchanged) | `border.strong` `#3D3D3D` | unchanged |
| focused | S3 | `border.strong` + focus ring per 00 | primary |
| read-only | S2 `#141414` | `border.default` | muted `#999999` |
| disabled | `control_disabled` `#141414` | `border.disabled` `#222222` | disabled `#666666` |
| invalid | S3 | `border.invalid` `#F18A90` | primary; trailing status icon 12 in `status.danger.foreground`, right inset 7; reserve 28 right padding |

Note the deliberate difference from buttons: field FILL never changes on
hover/focus — only the border and ring. Fields read as wells, buttons as raised.

## Field row (label + control)

Grid: label column 92 (inspector contexts: 110), gap 8
(`gap.inspector_label_value`), row min-height 32 (24 control + 2×`padding.block` 4).
Label: control type (11/1), muted, right-aligned, ellipsized.

## Unit affix / field group

Affix cell: min-width 28, height 24, padding-inline 6, fill S2 `#141414`,
border `border.default`, text meta (9 mono, muted), centered. Groups fuse borders:
outer radius.sm at group ends only, adjacent borders collapse to 1px.
Axis prefix (X/Y/Z...): 18 wide, meta type (9 mono 700), muted, no box.

## Numeric scrub

Visuals identical to numeric field. During scrub: treat as focused (border.strong,
no ring), cursor = ew-resize. Value text mono with tabular figures.

## Multi-line text area

Same chrome as single-line; padding 8 all sides; min-height 3 rows
(3×16 line + 2×8), body type 12/1.35.

## Search field

Leading search icon 12, muted; clear affordance = quiet icon button 20 when
non-empty. Otherwise identical to single-line.
