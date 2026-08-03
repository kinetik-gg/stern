# Visual spec — Status banners, feedback, progress contexts, inspector rows

Authority: `00-language.md`. Owned by family issues: banners/toasts → #913 (overlays),
inspector rows → #914 (chrome), status dots/jobs → #914 (status bar scope).
Source: labs `.ui-feedback`, `.ui-status-dot`, stern.css `.stern-property`;
DS `src/components/{feedback-status,progress-and-feedback,inspector-components}.md`.

## Status banner (inline feedback)

Min-height 38, padding 7/8, radius.sm, grid: icon 18 / copy flex / action auto,
gap 8. Neutral: fill S2, border `border.subtle`, icon+copy secondary.
Status variants use the status 4-tuple: fill `status.X.surface`, border
`status.X.border`, icon + emphasis `status.X.foreground`; copy stays
`content.secondary`, detail line detail type (10) muted, margin-top 2.
Inline action: quiet button 20.

## Status dot

6×6 circle in `status.X.strong`, margin-right 5. Warning variant: 6×6 square
rotated 45° (diamond) — shape encodes severity class beyond color (a11y).

## Job list rows

Row 28 (`size.row.standard`): name secondary / progress bar (03 spec, width
flex max 160) / percent meta mono muted / cancel quiet icon button 16.
Completed: progress replaced by success dot + meta "done". Failed: danger dot +
retry quiet button.

## Inspector property rows

Row min-height 28, padding 4/10, grid: label 110 / value flex, gap 8.
Label: control type (11) muted, right-aligned, ellipsized.
Value cell: any 02/03-spec control at height 24; vector pairs use axis prefixes
(02 §unit affix). Section header: height 30, fill S1, border-b `border.default`,
title control-strong primary, disclosure caret leading, keyframe diamond
trailing (8×8 rotated 45°, `status.warning.foreground` fill filled-state /
`border.strong` outline empty-state, 1px black separator border).
Row hover: fill S4 on the row (value control keeps own states).
Validation: invalid value → field invalid state (02); row label unchanged.
Mixed values (multi-selection): value text "—" muted + italic off (no italics in
system); tooltip explains.

## Empty states

Centered stack, gap 8: icon 20 muted, headline control secondary, hint detail
muted, optional default-variant button. Never on accent.
