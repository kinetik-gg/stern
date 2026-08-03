# Visual spec — Choice controls, sliders, tabs, progress

Authority: `00-language.md`. Family issue: #912.
Source: labs `.ui-check`, `.ui-radio`, `.ui-switch`, `.ui-segmented`, `.ui-tabs`,
`.ui-slider*`, `.ui-progress`; DS `src/components/selection-controls.md`, `sliders-and-tabs.md`.

## Checkbox

Row: min-height 24, gap 6 (label gap; labs 7 → normalize 6), label control type
(11) secondary. Box: 14×14, radius.sm 3, fill S1 `#111111`, border 1px
`border.strong` `#3D3D3D`.

| State | Box fill | Box border | Glyph |
|---|---|---|---|
| unchecked | S1 | strong | — |
| checked | `accent.default` `#0C8CE9` | none | white check, 2px stroke (`stroke.emphasis`), inset ~(4,1) 4×8 rotated 45° |
| mixed | `accent.default` | none | white 6×2 bar centered |
| disabled | S1 | `border.disabled` | glyph at 100%; label + box read disabled via `content.disabled` label and border |
| focused | + focus ring on the BOX only | | |

Hover: label promotes to primary; box border unchanged.

## Radio

Same row/pattern as checkbox. Mark: 14×14 circle, fill S1, border strong.
Checked: inner dot inset 3 (8px dot) in `content.primary` `#E8E8E8` — radios are
NEUTRAL when checked (labs), unlike checkboxes. Group: vertical stack, gap 2.

## Switch (toggle)

Row min-height 24, gap 6, label control type secondary.
Track: 26×14, `radius.full`, knob 8×8 circle at inset 2.

| State | Track fill | Track border | Knob |
|---|---|---|---|
| off | `border.default` `#2A2A2A` | `border.strong` | `content.muted` `#999999`, x=2 |
| on | `accent.subtle` `#0B2A3F` | `border.strong` | `focus.indicator` `#4DB2FF`, x=14 (translate 12) |
| disabled | off-style with `border.disabled`, knob `content.disabled` | | |

Knob motion: 80ms standard (the only positional animation in the system).

## Segmented control & tab strip

Container: padding 2, fill S0 `#0B0B0B` (sunken well), border `border.subtle`,
`radius.md` 6. Items: height 24, padding-inline 11 → normalize 8
(`inline.default`)... labs use 11; **normative 10** (midpoint sanctioned once
here — record in family PR), radius.sm, transparent at rest, text muted → hover:
text primary + fill S4.

Selected item (mode-choice doctrine): text primary, fill S3 `#181818`, ring 1px
`border.strong`, shadow 0 1 2 / 24% (sub-`shadow.low`; use shadow.low). NOT accent.

Document-level tab strip (frame tabs) is specified in 05-chrome-dock.md; this
file covers the inline segmented/tabs control only.

## Slider

Row grid: label 72 / track flex / readout 48, min-height 28, gap 8.
Label control-muted; readout meta (10 mono, secondary, right, tabular).
Track: height 3, `radius.full`; filled span `accent.default`, remainder
`border.default` `#2A2A2A`. Thumb: 12×12 circle, fill `content.primary` `#E8E8E8`,
ring 2px in surrounding surface color (separator effect), `shadow.low`.
Hover: thumb fill white. Dragging: thumb fill white, track fill `accent.hover`.
Disabled: track remainder `border.subtle`, fill `content.disabled`, thumb
`content.disabled`. Focus: ring around the whole track row hit area.

## Progress bar

Height 4, `radius.full`, track `border.subtle` `#222222`, fill `accent.default`.
Indeterminate: 30% width fill sweeping, 1.2s standard easing loop.
Status-colored progress uses `status.*.strong`.
