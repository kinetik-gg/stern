# Visual spec — Editor chrome & dock

Authority: `00-language.md`. Family issue: #914.
Source: labs `.editor-*`, `.dock-*`, `.mac-*`; DS `src/components/{toolbars,
editor-chrome-components,navigation-surfaces}.md`, `src/patterns/{workspace-chrome,
docking-and-panels,window-frame}.md`, `src/principles/density.md`.

## Vertical bar ladder (top to bottom)

| Bar | Height | Fill | Border | Content type |
|---|---|---|---|---|
| titlebar | 32 | S1 `#111111` | b-b `border.subtle` | identity: brand 600 11 (Space Grotesk), primary |
| application/workspace bar | `size.workspaceBar` 40 | S1 | b-b `border.subtle` | workspace tabs + app menus |
| toolbar | 28 | S2 `#141414` | b-b `border.subtle` | quiet buttons 24, gap 3, padding-inline 6 |
| panel header | `size.panelHeader` 30 | S2 | b-b `border.default` | title control-strong 11/600 primary |
| status bar | 22 | S1 | b-t `border.subtle` | meta mono 9 muted, item gap 14, padding-inline 8 |

(labs titlebar=32/toolbar=28/panelbar=30/statusbar=22 map onto the ladder; the
40 workspace bar comes from `size.workspaceBar` and the DS chrome pattern.)

## Titlebar & window controls

Drag region = all space not occupied by controls. Window control buttons: 38 wide
× bar height, transparent, icon 12 muted; hover: icon primary + fill S4; close
hover: white icon on `#C42B1C` (D6 derived constant). macOS variant: 12px traffic
lights, gap 8, centered title control type secondary.

## Workspace bar

Workspace tabs: height 24 within the 40 bar, quiet-button visuals; active
workspace = mode-choice selected (S3 fill + border.strong ring + text primary).
Brand identity leading, meta mono muted for project context.

## Frame (docked) & tab strip

Frame: border 1px `border.default`; ACTIVE frame: `border.strong` (this is the
focus hierarchy signal — no accent). Frame header: height 30, fill S2, border-b
`border.default`, padding 0 6 0 8, title 11/600 primary, quiet icon buttons 22
trailing, gap 6.

Frame tab strip (multiple panels in one frame): strip height 30 aligned in
header; tabs height `size.tab` 28... labs show inline tabs at 24 — frame tabs
use 28 (`size.tab`) per token. Tab: padding-inline 10, radius.sm top corners
only, text muted; hover text primary + S4; SELECTED tab: fill of the panel body
(S2) + text primary + b-t 1px `border.strong`, no bottom border (merges with
body). Close glyph on hover: quiet 16 icon button.

## Panel body

Fill S2 `#141414` (`surface.panel`); raised sub-cards S3. Content padding
`padding.panel` 8. Empty-state canvas: S0 with 16px dot grid (`#303030` dots
1px), placeholder meta mono muted.

## Splitter

Hit target 7 (`size.handle.hit`), visual 1px line (`size.handle.visual`)
`border.subtle`, centered in the hit area; hover/drag: line unchanged (labs),
cursor col/row-resize. Transactional drag per behavior spec.

## Dock drop zones

Overlay on target region: fill `accent.default` at 16% opacity, border 1px
`border.strong`, radius.sm, label meta mono 700 in `focus.indicator` `#4DB2FF`.
Center zone insets 28%/24%.

## Sidepanel

Width 220–230 default, fill S2, border-l `border.subtle`.
