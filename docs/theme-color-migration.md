# Semantic Theme Color Migration

Stern `1.0.0-rc.2.dev` deliberately replaces the provisional flat color
palette with grouped semantic roles. This is a prerelease breaking change:
legacy fields and broad `SemanticColor` variants were removed rather than kept
as aliases or a second source of truth.

## Customizing a palette

Start from the exact dark palette, mutate only the roles the application owns,
and pass the complete palette through the existing `Theme::with_colors`
boundary:

```rust
use stern::core::{Color, ThemeColors, default_dark_theme};

let mut colors = ThemeColors::default_dark();
colors.surface.application = Color::rgb8(0x16, 0x16, 0x18);
colors.content.primary = Color::rgb8(0xF0, 0xF0, 0xF2);
colors.accent.default = Color::rgb8(0x7C, 0x5C, 0xFC);

let theme = default_dark_theme().with_colors(colors);
```

`ThemeColors` intentionally has no `Default` implementation. Calling
`ThemeColors::default_dark()` makes the selected starting palette explicit.
The group structs are non-exhaustive, so external code should mutate a
starting palette instead of constructing groups with struct literals.

## Field migration

| Removed field | New semantic path | Selection guidance |
| --- | --- | --- |
| `surface` | `surface.application`, `surface.workspace`, or `surface.panel` | Choose the role of the painted region. |
| `surface_raised` | `surface.panel_raised`, `surface.raised`, or `surface.control` | Use `panel_raised` for elevated panels, `control` for controls, and `raised` for other raised surfaces. |
| `surface_hover` | `surface.control_hover` or `surface.hover` | Use the control-specific role for buttons and fields. |
| `surface_active` | `surface.control_pressed` | Preserve existing pressed/active precedence. |
| `surface_sunken` | `surface.sunken` | Use `surface.workspace` instead for viewport/workspace canvases. |
| `text` | `content.primary` | Primary labels and ordinary content. |
| `text_muted` | `content.muted` or `content.secondary` | Choose by information hierarchy. |
| `text_disabled` | `content.disabled` | Disabled content. |
| `accent` | `accent.default` | Default accent paint. |
| `danger` | `status.danger.strong` | Strong danger paint only; this migration does not define redundant status cues. |
| `warning` | `status.warning.strong` | Strong warning paint only. |
| `success` | `status.success.strong` | Strong success paint only. |
| `border` | `border.default` | Ordinary outlines. |
| `border_subtle` | `border.subtle` | Separators and low-emphasis outlines. |
| `focus_ring` | `focus.ring` or `border.focused` | Choose by whether the paint is a ring or a focused border. |
| `selection` | `selection.background` | Use `selection.foreground` for content painted on the selection. |
| `disabled` | `surface.control_disabled` | Disabled control surfaces. |
| `overlay` | `surface.overlay` | Floating surface; use `overlay.scrim` for the blocking scrim behind it. |
| `viewport_background` | `surface.workspace` | Workspace and viewport canvases. |

Ordinary themed foregrounds previously painted with `Color::WHITE` should use
`content.on_accent`, `accent.foreground`, or `selection.foreground` according
to the surface underneath them. Renderer constants and explicitly caller-owned
colors remain valid.

## Resolver migration

`SemanticColor` now names exact token keys. For example:

| Removed variant | Replacement |
| --- | --- |
| `Surface` | `SurfaceApplication`, `SurfaceWorkspace`, or `SurfacePanel` |
| `SurfaceRaised` | `SurfacePanelRaised`, `SurfaceRaised`, or `SurfaceControl` |
| `SurfaceHover` | `SurfaceControlHover` or `SurfaceHover` |
| `SurfaceActive` | `SurfaceControlPressed` |
| `SurfaceSunken` | `SurfaceSunken` |
| `Text` | `ContentPrimary` |
| `TextMuted` | `ContentMuted` or `ContentSecondary` |
| `TextDisabled` | `ContentDisabled` |
| `Accent` | `AccentDefault` |
| `Danger` | `StatusDangerStrong` |
| `Warning` | `StatusWarningStrong` |
| `Success` | `StatusSuccessStrong` |
| `Border` | `BorderDefault` |
| `BorderSubtle` | `BorderSubtle` |
| `FocusRing` | `FocusRing` or `BorderFocused` |
| `Selection` | `SelectionBackground` |
| `Disabled` | `SurfaceControlDisabled` |
| `Overlay` | `SurfaceOverlay` or `OverlayScrim` |
| `ViewportBackground` | `SurfaceWorkspace` |

`SemanticColor::ALL` contains all 53 stored roles in grouped-field order. The
enum is non-exhaustive; external matches must retain a wildcard arm so Stern
can add roles without another forced exhaustive-match cutover:

```rust
use stern::core::SemanticColor;

let group = match SemanticColor::AccentDefault {
    SemanticColor::AccentDefault => "accent",
    _ => "other",
};
```

`ThemeColors::get` and `Theme::color` remain the canonical resolvers.
