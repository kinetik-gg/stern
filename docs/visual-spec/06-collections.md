# Visual spec — Collections: lists, tables, trees, virtual rows, assets

Authority: `00-language.md`. Family issue: #915.
Source: labs `.ui-collection`, `.tree-row`, `.virtual-*`, `.ui-table`, `.asset-*`;
DS `src/collections/*.md`, `src/components/collection-components.md`.

## Collection container

Fill S2 `#141414`, border 1px `border.default`, radius.sm 3, clipped.

## Header row (tables / column headers)

Height 24 (`size.row.compact`), fill S1 `#111111`, text meta mono (9/700)
UPPERCASE +0.06em muted, padding-inline 8, border-b `border.subtle`.
Sort indicator: caret icon 12 muted; active sort column: text secondary + caret
`focus.indicator`. Column resize handle: 7 hit / 1 visual `border.subtle`
(same splitter recipe as dock), min column width 1px floor preserved.

## Rows (list & table)

Height: compact contexts 24 (`size.row.compact`), standard/virtualized 28
(`size.row.standard`). Padding-inline 8, grid leading 18 (icon col) / flex /
trailing meta, gap 6, border-b `border.subtle` (last row none), text control
(11) secondary; meta trailing meta mono (9) muted.

| State | Fill | Text |
|---|---|---|
| rest | transparent | secondary |
| hover | S4 `#1C1C1C` | secondary |
| selected | `selection.background` `#0C8CE9` | white; meta white at 78% |
| focused (kbd, not selected) | S4 + inset focus ring (1px surface separator + 1px `#4DB2FF`) | secondary |
| disabled row | transparent | disabled |

Multi-select ranges paint each row fully; no gap treatment. Zebra striping: none.

## Tree rows

Height 24, indent 16/level added to left padding (base 6). Columns:
disclosure 16 / icon 16 / label flex / meta / actions. Disclosure: caret-right
icon 12 muted, rotates 90° when expanded (80ms standard). Same state table as
rows. Inline visibility/lock toggles: quiet icon buttons 16, muted → primary on
hover; remain visible on selected rows in white at 78%.

## Virtualized viewport

Container: collection container recipe. Scrollbar: 6 wide thumb,
`border.strong` `#3D3D3D`, radius.full, track transparent, inset 2.
(Overscan dimming is a lab affordance — NOT shipped.)

## Inline edit (rename)

Swap label for a single-line field (02 spec) at row height minus 4 block; field
inherits row width to the meta column. Commit/cancel per behavior spec.

## Asset grid

Grid 3-up (responsive by width), gap 6. Card: fill S2, border `border.default`,
radius.sm, clipped. Preview: 16:10, checkerboard S3/`#202020` 16px for
transparency, centered icon 20 muted. Copy block: padding 6, name detail (10)
primary-ellipsized, meta mono (9) muted.
Selected card: fill S3, border `border.strong`, plus 7px `accent.default` dot
badge top-right inset 6 with 2px S0 separator ring. Hover: border strong.
(Cards use border+badge selection, not full accent fill — selection doctrine
exception sanctioned here for large surfaces; recorded as normative.)
