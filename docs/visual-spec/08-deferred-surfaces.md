# Visual spec — Deferred surfaces (node graph, timeline, viewport)

**NOT in v0.0.1-alpha.1 scope.** Recorded now because the labs values were
extracted during the same synthesis pass; future family issues execute against
this file without re-deriving. Authority: `00-language.md`.

## Node graph

Canvas: S0 + 16px dot grid (1px `#353535` dots). Card: width 180, fill S2,
border `border.strong`, radius.md, elevation 2 (`shadow.medium`).
Header: height 28, padding-inline 9, border-b `border.default`, title
control-strong primary; selected-indicator 7px `accent.default` dot trailing.
Body: padding 8, row gap 6; rows grid label-flex/value-74, label detail muted.
Value chip: height 22, fill S0, border `border.default`, radius.sm, mono 10
secondary right-aligned.
SELECTED card: border `#686868` + ring (1px S0 separator + 2px `#505050`) —
derived constants D4 (neutral selection preserves canvas color-coding).
Port: 12×12 circle, domain color fill, 2px S2 separator border; in at -6 left,
out at -6 right. Wire: 2px round-cap, domain color (default `#F0C66D`).

## Timeline

Ruler: height 28, fill S1, 1px `border.default` ticks each 50, labels mono 9
muted baseline 18. Label rail: width 112, fill S2, border-r `border.default`,
rows border-b `border.subtle`. Row height 28. Track: minor gridline white 2.4%
each 50. Clip: height 20, top 4, radius.sm, fill domain color (default
`accent.default`), border white 20%, label mono 9 white clipped.
Playhead: 1px `focus.indicator` full-height + 9×8 triangle cap top.
Transport buttons: icon buttons per 01, play/pause icons filled weight.

## Viewport

Stage: S0 variant `#101010`, border `border.default`. Content frame shadow:
ring 1px `#8A8A8A` + `shadow.high`-class drop. Checkerboard: S3/`#202020` 16px
at 24% where transparency matters. Safe area: 1px white 48% inset 9%.
Selection bounds: 1px `focus.indicator` + center crosshairs extending 16 beyond
bounds. Overlay clusters (tools top-left inset 8, status bottom-right inset 8):
fill `#111111` at 94%, border `border.default`, radius.md, elevation 2,
padding 4, gap 3, quiet icon buttons 24; status text mono 9 muted line-height 24.
