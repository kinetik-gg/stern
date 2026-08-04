//! `Theme::overlay_item`/`Theme::command_palette_item` focus-fill conformance.
//!
//! `00-language.md` §Focus model states the two-layer ring is additive and
//! "never recolors the control body" for most components (see
//! `row_focus_recipe_conformance.rs` and `button_focus_recipe_conformance.rs`,
//! which pin that neutrality) — but it explicitly names menu items as the one
//! family where the ring "combine[s] with hover fill". This file pins the
//! opposite, intentional property for overlay item recipes: `focused` alone
//! (no mouse hover) promotes the row to the same highlight fill `hovered`
//! does, so a keyboard-navigated menu/palette item reads as visibly
//! highlighted, not just ringed. The additive ring itself is a separate paint
//! step outside this recipe's scope (see `crates/stern-widgets/src/ui/overlays.rs`).

#![allow(clippy::float_cmp)]

use stern_core::{Brush, Color, ComponentState, ThemeColors, default_dark_theme};

fn with_focus(mut state: ComponentState, focused: bool) -> ComponentState {
    state.focused = focused;
    state
}

#[test]
fn overlay_item_focus_promotes_to_the_same_highlight_as_hover() {
    let mut colors = ThemeColors::default_dark();
    colors.surface.hover = Color::rgb8(1, 2, 3);
    colors.content.primary = Color::rgb8(4, 5, 6);
    colors.content.secondary = Color::rgb8(7, 8, 9);
    colors.content.disabled = Color::rgb8(10, 11, 12);
    let theme = default_dark_theme().with_colors(colors);

    let rest = ComponentState::default();
    let hovered = ComponentState {
        hovered: true,
        ..ComponentState::default()
    };
    let disabled = ComponentState {
        disabled: true,
        ..ComponentState::default()
    };

    // Focused-without-hover matches the hovered recipe exactly (fill,
    // foreground, border, radius) — the documented exception, not neutrality.
    let focused_only = theme.overlay_item(with_focus(rest, true));
    let hovered_only = theme.overlay_item(hovered);
    assert_eq!(
        focused_only, hovered_only,
        "focused alone must match hovered exactly for overlay_item"
    );
    assert_eq!(focused_only.background, Brush::Solid(colors.surface.hover));
    assert_eq!(focused_only.foreground, colors.content.primary);

    // Rest stays transparent/secondary regardless of the unfocused flag.
    let unfocused_rest = theme.overlay_item(with_focus(rest, false));
    assert_eq!(unfocused_rest.background, Brush::Solid(Color::TRANSPARENT));
    assert_eq!(unfocused_rest.foreground, colors.content.secondary);
    assert_ne!(unfocused_rest, focused_only);

    // Disabled still wins over a focused flag (no highlight leaks through).
    let disabled_focused = theme.overlay_item(with_focus(disabled, true));
    let disabled_unfocused = theme.overlay_item(with_focus(disabled, false));
    assert_eq!(disabled_focused, disabled_unfocused);
    assert_eq!(
        disabled_focused.background,
        Brush::Solid(Color::TRANSPARENT)
    );
    assert_eq!(disabled_focused.foreground, colors.content.disabled);
}

#[test]
fn command_palette_item_focus_promotes_to_highlight_but_never_the_accent_brush() {
    let mut colors = ThemeColors::default_dark();
    colors.surface.hover = Color::rgb8(1, 2, 3);
    colors.content.primary = Color::rgb8(4, 5, 6);
    colors.selection.background = Color::rgb8(7, 8, 9);
    colors.selection.foreground = Color::rgb8(10, 11, 12);
    let theme = default_dark_theme().with_colors(colors);

    // Focused-without-hover-or-selected promotes to the neutral highlight,
    // same as overlay_item, not the accent selection brush.
    let focused_only = theme.command_palette_item(with_focus(ComponentState::default(), true));
    assert_eq!(focused_only.background, Brush::Solid(colors.surface.hover));
    assert_eq!(focused_only.foreground, colors.content.primary);
    assert_ne!(
        focused_only.background,
        Brush::Solid(colors.selection.background)
    );

    // Selected (the active item) is focus-neutral in the sense that adding
    // `focused` on top of an already-selected state changes nothing further
    // — the accent precedence already dominates.
    let selected = ComponentState {
        selected: true,
        ..ComponentState::default()
    };
    let selected_unfocused = theme.command_palette_item(with_focus(selected, false));
    let selected_focused = theme.command_palette_item(with_focus(selected, true));
    assert_eq!(selected_unfocused, selected_focused);
    assert_eq!(
        selected_focused.background,
        Brush::Solid(colors.selection.background)
    );
    assert_eq!(selected_focused.foreground, colors.selection.foreground);
}
