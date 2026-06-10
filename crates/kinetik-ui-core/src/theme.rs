//! Theme tokens and component recipes.

use crate::{Brush, Color, CornerRadius, Stroke};

/// Semantic color role.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SemanticColor {
    /// Main application surface.
    Surface,
    /// Elevated or grouped surface.
    SurfaceRaised,
    /// Hovered surface.
    SurfaceHover,
    /// Active/pressed surface.
    SurfaceActive,
    /// Sunken input surface.
    SurfaceSunken,
    /// Primary text.
    Text,
    /// Muted secondary text.
    TextMuted,
    /// Disabled text.
    TextDisabled,
    /// Accent color.
    Accent,
    /// Danger/destructive color.
    Danger,
    /// Warning color.
    Warning,
    /// Success color.
    Success,
    /// Normal border.
    Border,
    /// Subtle border.
    BorderSubtle,
    /// Focus ring.
    FocusRing,
    /// Selection fill.
    Selection,
    /// Viewport background.
    ViewportBackground,
}

/// Theme color tokens.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThemeColors {
    /// Main application surface.
    pub surface: Color,
    /// Elevated or grouped surface.
    pub surface_raised: Color,
    /// Hovered surface.
    pub surface_hover: Color,
    /// Active/pressed surface.
    pub surface_active: Color,
    /// Sunken input surface.
    pub surface_sunken: Color,
    /// Primary text.
    pub text: Color,
    /// Muted secondary text.
    pub text_muted: Color,
    /// Disabled text.
    pub text_disabled: Color,
    /// Accent color.
    pub accent: Color,
    /// Danger/destructive color.
    pub danger: Color,
    /// Warning color.
    pub warning: Color,
    /// Success color.
    pub success: Color,
    /// Normal border.
    pub border: Color,
    /// Subtle border.
    pub border_subtle: Color,
    /// Focus ring.
    pub focus_ring: Color,
    /// Selection fill.
    pub selection: Color,
    /// Viewport background.
    pub viewport_background: Color,
}

impl ThemeColors {
    /// Returns a semantic color.
    #[must_use]
    pub const fn get(self, role: SemanticColor) -> Color {
        match role {
            SemanticColor::Surface => self.surface,
            SemanticColor::SurfaceRaised => self.surface_raised,
            SemanticColor::SurfaceHover => self.surface_hover,
            SemanticColor::SurfaceActive => self.surface_active,
            SemanticColor::SurfaceSunken => self.surface_sunken,
            SemanticColor::Text => self.text,
            SemanticColor::TextMuted => self.text_muted,
            SemanticColor::TextDisabled => self.text_disabled,
            SemanticColor::Accent => self.accent,
            SemanticColor::Danger => self.danger,
            SemanticColor::Warning => self.warning,
            SemanticColor::Success => self.success,
            SemanticColor::Border => self.border,
            SemanticColor::BorderSubtle => self.border_subtle,
            SemanticColor::FocusRing => self.focus_ring,
            SemanticColor::Selection => self.selection,
            SemanticColor::ViewportBackground => self.viewport_background,
        }
    }
}

/// Spacing token scale.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpacingScale {
    /// Extra-small spacing.
    pub xs: f32,
    /// Small spacing.
    pub sm: f32,
    /// Medium spacing.
    pub md: f32,
    /// Large spacing.
    pub lg: f32,
    /// Extra-large spacing.
    pub xl: f32,
}

/// Component state used by style recipes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)]
pub struct ComponentState {
    /// Hovered state.
    pub hovered: bool,
    /// Pressed state.
    pub pressed: bool,
    /// Focused state.
    pub focused: bool,
    /// Disabled state.
    pub disabled: bool,
    /// Selected state.
    pub selected: bool,
}

/// Button visual recipe output.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ButtonRecipe {
    /// Background brush.
    pub background: Brush,
    /// Text/icon color.
    pub foreground: Color,
    /// Border stroke.
    pub border: Stroke,
    /// Corner radius.
    pub radius: CornerRadius,
}

/// Complete theme.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Theme {
    /// Color tokens.
    pub colors: ThemeColors,
    /// Spacing tokens.
    pub spacing: SpacingScale,
    /// Default corner radius.
    pub radius: CornerRadius,
    /// Default border width.
    pub border_width: f32,
    /// Default text size.
    pub text_size: f32,
}

impl Theme {
    /// Resolves a semantic color.
    #[must_use]
    pub const fn color(self, role: SemanticColor) -> Color {
        self.colors.get(role)
    }

    /// Resolves the standard button recipe for a state.
    #[must_use]
    pub fn button(&self, state: ComponentState) -> ButtonRecipe {
        let background = if state.disabled {
            self.colors.surface
        } else if state.selected || state.pressed {
            self.colors.surface_active
        } else if state.hovered {
            self.colors.surface_hover
        } else {
            self.colors.surface_raised
        };
        let foreground = if state.disabled {
            self.colors.text_disabled
        } else {
            self.colors.text
        };
        let border_color = if state.focused {
            self.colors.focus_ring
        } else {
            self.colors.border
        };

        ButtonRecipe {
            background: Brush::Solid(background),
            foreground,
            border: Stroke::new(self.border_width, Brush::Solid(border_color)),
            radius: self.radius,
        }
    }
}

/// Returns the default dark editor theme.
#[must_use]
pub const fn default_dark_theme() -> Theme {
    Theme {
        colors: ThemeColors {
            surface: Color::rgb(0.055, 0.055, 0.055),
            surface_raised: Color::rgb(0.085, 0.085, 0.085),
            surface_hover: Color::rgb(0.13, 0.13, 0.13),
            surface_active: Color::rgb(0.16, 0.16, 0.16),
            surface_sunken: Color::rgb(0.035, 0.035, 0.035),
            text: Color::rgb(0.86, 0.86, 0.86),
            text_muted: Color::rgb(0.52, 0.52, 0.52),
            text_disabled: Color::rgb(0.30, 0.30, 0.30),
            accent: Color::rgb(0.13, 0.40, 0.96),
            danger: Color::rgb(0.86, 0.22, 0.22),
            warning: Color::rgb(0.90, 0.62, 0.18),
            success: Color::rgb(0.26, 0.70, 0.38),
            border: Color::rgb(0.21, 0.21, 0.21),
            border_subtle: Color::rgb(0.14, 0.14, 0.14),
            focus_ring: Color::rgb(0.25, 0.55, 1.0),
            selection: Color::rgb(0.13, 0.40, 0.96),
            viewport_background: Color::rgb(0.02, 0.02, 0.02),
        },
        spacing: SpacingScale {
            xs: 2.0,
            sm: 4.0,
            md: 8.0,
            lg: 12.0,
            xl: 16.0,
        },
        radius: CornerRadius::all(2.0),
        border_width: 1.0,
        text_size: 12.0,
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::{ComponentState, SemanticColor, default_dark_theme};
    use crate::{Brush, Color};

    #[test]
    fn resolves_semantic_colors() {
        let theme = default_dark_theme();

        assert_eq!(theme.color(SemanticColor::Accent), theme.colors.accent);
        assert_eq!(
            theme.color(SemanticColor::TextMuted),
            theme.colors.text_muted
        );
    }

    #[test]
    fn default_theme_has_dense_editor_spacing() {
        let theme = default_dark_theme();

        assert_eq!(theme.spacing.md, 8.0);
        assert_eq!(theme.text_size, 12.0);
        assert_eq!(theme.border_width, 1.0);
    }

    #[test]
    fn button_recipe_uses_state_colors() {
        let theme = default_dark_theme();

        let normal = theme.button(ComponentState::default());
        let hovered = theme.button(ComponentState {
            hovered: true,
            ..ComponentState::default()
        });
        let focused = theme.button(ComponentState {
            focused: true,
            ..ComponentState::default()
        });
        let disabled = theme.button(ComponentState {
            disabled: true,
            ..ComponentState::default()
        });

        assert_eq!(normal.background, Brush::Solid(theme.colors.surface_raised));
        assert_eq!(hovered.background, Brush::Solid(theme.colors.surface_hover));
        assert_eq!(focused.border.brush, Brush::Solid(theme.colors.focus_ring));
        assert_eq!(disabled.foreground, theme.colors.text_disabled);
    }

    #[test]
    fn active_selection_uses_blue_accent_family() {
        let theme = default_dark_theme();

        assert_eq!(theme.colors.accent, theme.colors.selection);
        assert!(theme.colors.accent.b > theme.colors.accent.r);
        assert!(theme.colors.accent.b > theme.colors.accent.g);
    }

    #[test]
    fn transparent_color_remains_available() {
        assert_eq!(Color::TRANSPARENT.a, 0.0);
    }
}
