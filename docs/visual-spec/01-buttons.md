# Visual spec — Buttons

Authority: `00-language.md` (tiers, focus, motion, typography). Family issue: #910.
Source: labs `.ui-button`, `.ui-icon-button`; DS `src/components/buttons.md`.

## Button (default variant)

Anatomy: single box, optional leading icon, label. Height `size.control.sm` 24,
padding-inline `spacing.padding.control.inline.default` 8, icon-label gap 6
(between `gap.icon_label` 4 and inline.compact 6 — use 6, matches labs),
radius `radius.sm` 3, border 1px, font control (11/1, 500, ui). Min-width: none;
content-sized. Icon `size.icon.sm` 12 (D3), color = currentColor.

| State | Fill | Border | Text |
|---|---|---|---|
| idle | S3 `#181818` | `border.default` `#2A2A2A` | secondary `#B8B8B8` |
| hover | S4 `#1C1C1C` | `border.strong` `#3D3D3D` | primary `#E8E8E8` |
| pressed | `control_pressed` `#2A2A2A` (D1) | `border.strong` | primary |
| focused | (idle/hover fill) | unchanged | unchanged + focus ring per 00 |
| disabled | S1 `#111111` | `border.disabled` `#222222` | disabled `#666666` |
| busy | as idle | as idle | muted `#999999`; spinner icon rotates 1s linear |

Transitions: fill/border/text 80ms standard.

## Primary variant

| State | Fill | Border | Text |
|---|---|---|---|
| idle | `accent.default` `#0C8CE9` | none (transparent) | on_accent `#FFFFFF` |
| hover | `accent.hover` `#259CF0` | none | on_accent |
| pressed | `accent.pressed` `#0876C5` | none | on_accent |
| disabled | S1 | `border.disabled` | disabled (same as default disabled) |

## Quiet (ghost) variant

Idle: transparent fill, transparent border, text secondary. Hover/pressed/disabled:
identical to default variant's hover/pressed/disabled. Used for toolbar and
panel-header actions.

## Danger variant

| State | Fill | Border | Text |
|---|---|---|---|
| idle | `status.danger.surface` `#1B1314` | `status.danger.border` `#3D292B` | `status.danger.foreground` `#F18A90` |
| hover | `#1B1314` (unchanged) | `status.danger.strong` `#D9535B` | unchanged |
| pressed | `#1B1314` | `#D9535B` | on_accent `#FFFFFF` |
| disabled | as default disabled | | |

(Danger hover/pressed are not shown in labs; the rule above extends the danger
4-tuple by promoting the border to `strong` — the same promote-one-tier move the
default variant makes. Recorded here as the normative resolution.)

## Icon button

24×24 (`size.control.sm` square), icon 12 centered, radius.sm, same state table
as default variant. Quiet icon button = quiet variant states. Selectable icon
button ("chosen" mode): selected = S3 fill + `border.strong` ring (mode-choice
doctrine, 00 §Selection-vs-hover), NOT accent. Every icon button REQUIRES an
accessible label (existing API contract).

## Button group / split button

Fused row: shared 1px borders (adjacent edges collapse), outer radius.sm only at
group ends, inner radii 0. Divider between segments = `border.default`.

## Sizes

Default 24. Dense contexts (color-picker cards, gradient rows) may use
`size.control.xs` 20 with padding-inline 6 (`inline.compact`) and icon 12.
Prominent contexts may use `md` 28 / `lg` 32 with icon 16 — no other changes.
