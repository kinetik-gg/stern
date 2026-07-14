use super::{
    ButtonRecipe, ButtonVariant, CheckRecipe, ComponentState, ControlMetrics, DurationScale,
    ElevationScale, FontToken, OpacityScale, PanelRecipe, RadiusScale, RowRecipe, SemanticColor,
    SeparatorRecipe, ShadowRecipe, SliderRecipe, SpacingScale, TabRecipe, TextFieldRecipe,
    TextRecipe, TextRole, ThemeColors, ToggleRecipe, TypographyScale,
};
use crate::{Brush, Color, CornerRadius, Stroke, Vec2};

/// Complete theme.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Theme {
    /// Color tokens.
    pub colors: ThemeColors,
    /// Spacing tokens.
    pub spacing: SpacingScale,
    /// Radius tokens.
    pub radii: RadiusScale,
    /// Typography tokens.
    pub typography: TypographyScale,
    /// Opacity tokens.
    pub opacity: OpacityScale,
    /// Elevation tokens.
    pub elevation: ElevationScale,
    /// Motion duration tokens.
    pub duration: DurationScale,
    /// Control metrics.
    pub controls: ControlMetrics,
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

    /// Resolves a text style role.
    #[must_use]
    pub const fn font(self, role: TextRole) -> FontToken {
        self.typography.get(role)
    }

    /// Returns this theme with a replaced color scale.
    #[must_use]
    pub const fn with_colors(mut self, colors: ThemeColors) -> Self {
        self.colors = colors;
        self
    }

    /// Returns this theme with a replaced spacing scale.
    #[must_use]
    pub const fn with_spacing(mut self, spacing: SpacingScale) -> Self {
        self.spacing = spacing;
        self
    }

    /// Returns this theme with a replaced radius scale.
    #[must_use]
    pub const fn with_radii(mut self, radii: RadiusScale) -> Self {
        self.radius = radii.sm;
        self.radii = radii;
        self
    }

    /// Returns this theme with a replaced typography scale.
    #[must_use]
    pub const fn with_typography(mut self, typography: TypographyScale) -> Self {
        self.text_size = typography.body.size;
        self.typography = typography;
        self
    }

    /// Returns this theme with a replaced opacity scale.
    #[must_use]
    pub const fn with_opacity(mut self, opacity: OpacityScale) -> Self {
        self.opacity = opacity;
        self
    }

    /// Returns this theme with a replaced elevation scale.
    #[must_use]
    pub const fn with_elevation(mut self, elevation: ElevationScale) -> Self {
        self.elevation = elevation;
        self
    }

    /// Returns this theme with a replaced duration scale.
    #[must_use]
    pub const fn with_duration(mut self, duration: DurationScale) -> Self {
        self.duration = duration;
        self
    }

    /// Returns this theme with replaced control metrics.
    #[must_use]
    pub const fn with_controls(mut self, controls: ControlMetrics) -> Self {
        self.border_width = controls.border_width;
        self.controls = controls;
        self
    }

    /// Returns the standard label recipe.
    #[must_use]
    pub const fn label(&self, role: TextRole, disabled: bool) -> TextRecipe {
        TextRecipe {
            foreground: if disabled {
                self.colors.content.disabled
            } else {
                self.colors.content.primary
            },
            font: self.typography.get(role),
        }
    }

    /// Resolves the standard button recipe for a state.
    #[must_use]
    pub fn button(&self, state: ComponentState) -> ButtonRecipe {
        self.button_variant(ButtonVariant::Standard, state)
    }

    /// Resolves a button recipe for a visual variant and state.
    #[must_use]
    pub fn button_variant(&self, variant: ButtonVariant, state: ComponentState) -> ButtonRecipe {
        let background = if state.disabled {
            self.colors.surface.control_disabled
        } else {
            match variant {
                ButtonVariant::Standard => {
                    if state.selected || state.pressed {
                        self.colors.surface.control_pressed
                    } else if state.hovered {
                        self.colors.surface.control_hover
                    } else {
                        self.colors.surface.control
                    }
                }
                ButtonVariant::Primary => {
                    if state.pressed {
                        self.colors.accent.pressed
                    } else if state.selected {
                        self.colors.accent.default
                    } else if state.hovered {
                        self.colors.accent.hover
                    } else {
                        self.colors.accent.default
                    }
                }
                ButtonVariant::Ghost => {
                    if state.selected || state.pressed {
                        self.colors.surface.control_pressed
                    } else if state.hovered {
                        self.colors.surface.control_hover
                    } else {
                        Color::TRANSPARENT
                    }
                }
                ButtonVariant::Danger => {
                    if state.selected || state.pressed {
                        self.colors.status.danger.strong.with_alpha(0.86)
                    } else if state.hovered {
                        self.colors.status.danger.strong.with_alpha(0.92)
                    } else {
                        self.colors.status.danger.strong
                    }
                }
            }
        };

        let foreground = if state.disabled {
            self.colors.content.disabled
        } else if matches!(variant, ButtonVariant::Primary | ButtonVariant::Danger) {
            self.colors.content.on_accent
        } else {
            self.colors.content.primary
        };
        let border_color = if state.focused {
            self.colors.focus.ring
        } else if matches!(variant, ButtonVariant::Ghost) {
            self.colors.border.subtle
        } else {
            self.colors.border.default
        };

        ButtonRecipe {
            background: Brush::Solid(background),
            foreground,
            border: Stroke::new(self.controls.border_width, Brush::Solid(border_color)),
            radius: self.radii.sm,
        }
    }

    /// Resolves a tab recipe for a state.
    #[must_use]
    pub fn tab(&self, state: ComponentState) -> TabRecipe {
        let background = if state.disabled {
            self.colors.surface.control_disabled
        } else if state.selected || state.pressed {
            self.colors.surface.control_pressed
        } else if state.hovered {
            self.colors.surface.hover
        } else {
            self.colors.surface.panel
        };
        let foreground = if state.disabled {
            self.colors.content.disabled
        } else {
            self.colors.content.primary
        };
        let border_color = if state.focused {
            self.colors.focus.ring
        } else {
            self.colors.border.default
        };

        TabRecipe {
            background: Brush::Solid(background),
            foreground,
            border: Stroke::new(self.controls.border_width, Brush::Solid(border_color)),
            radius: self.radii.none,
            indicator: state
                .selected
                .then_some(Brush::Solid(self.colors.accent.default)),
            indicator_thickness: 2.0,
        }
    }

    /// Resolves a list or table row recipe for a state.
    #[must_use]
    pub fn row(&self, state: ComponentState) -> RowRecipe {
        let background = if state.disabled {
            self.colors.surface.control_disabled
        } else if state.selected {
            self.colors.selection.background
        } else if state.hovered {
            self.colors.surface.hover
        } else {
            self.colors.surface.sunken
        };
        let foreground = if state.disabled {
            self.colors.content.disabled
        } else if state.selected {
            self.colors.selection.foreground
        } else {
            self.colors.content.primary
        };
        RowRecipe {
            background: Brush::Solid(background),
            foreground,
            border: Stroke::new(
                self.controls.border_width,
                Brush::Solid(self.colors.border.subtle),
            ),
            radius: self.radii.none,
        }
    }

    /// Resolves a checkbox recipe for a state.
    #[must_use]
    pub fn checkbox(&self, state: ComponentState) -> CheckRecipe {
        let fill = if state.disabled {
            self.colors.surface.control_disabled
        } else if state.selected {
            self.colors.accent.default
        } else if state.hovered {
            self.colors.surface.control_hover
        } else {
            self.colors.surface.sunken
        };
        let border_color = if state.focused {
            self.colors.focus.ring
        } else {
            self.colors.border.default
        };
        CheckRecipe {
            fill: Brush::Solid(fill),
            mark: if state.disabled {
                self.colors.content.disabled
            } else {
                self.colors.content.on_accent
            },
            border: Stroke::new(self.controls.border_width, Brush::Solid(border_color)),
            radius: self.radii.xs,
            size: self.controls.check_size,
        }
    }

    /// Resolves a radio button recipe for a state.
    #[must_use]
    pub fn radio_button(&self, state: ComponentState) -> CheckRecipe {
        CheckRecipe {
            radius: self.radii.pill,
            ..self.checkbox(state)
        }
    }

    /// Resolves a toggle recipe for a state.
    #[must_use]
    pub fn toggle(&self, state: ComponentState) -> ToggleRecipe {
        let track = if state.disabled {
            self.colors.surface.control_disabled
        } else if state.selected {
            self.colors.accent.default
        } else if state.hovered {
            self.colors.surface.control_hover
        } else {
            self.colors.surface.control_pressed
        };
        let thumb = if state.disabled {
            self.colors.content.disabled
        } else {
            self.colors.content.primary
        };
        let border_color = if state.focused {
            self.colors.focus.ring
        } else {
            self.colors.border.default
        };
        ToggleRecipe {
            track: Brush::Solid(track),
            thumb: Brush::Solid(thumb),
            border: Stroke::new(self.controls.border_width, Brush::Solid(border_color)),
            padding: 2.0,
        }
    }

    /// Resolves a slider recipe for a state.
    #[must_use]
    pub fn slider(&self, state: ComponentState) -> SliderRecipe {
        let fill = if state.disabled {
            self.colors.content.disabled
        } else {
            self.colors.accent.default
        };
        let border_color = if state.focused {
            self.colors.focus.ring
        } else {
            self.colors.border.default
        };
        SliderRecipe {
            track: Brush::Solid(self.colors.surface.sunken),
            fill: Brush::Solid(fill),
            border: Stroke::new(self.controls.border_width, Brush::Solid(border_color)),
            radius: self.radii.pill,
        }
    }

    /// Resolves a text field recipe for a state.
    #[must_use]
    pub fn text_field(&self, state: ComponentState) -> TextFieldRecipe {
        let border_color = if state.focused {
            self.colors.border.focused
        } else if state.hovered {
            self.colors.border.hover
        } else {
            self.colors.border.subtle
        };
        TextFieldRecipe {
            background: Brush::Solid(if state.disabled {
                self.colors.surface.control_disabled
            } else {
                self.colors.surface.sunken
            }),
            foreground: if state.disabled {
                self.colors.content.disabled
            } else {
                self.colors.content.primary
            },
            border: Stroke::new(self.controls.border_width, Brush::Solid(border_color)),
            radius: self.radii.sm,
            selection: Brush::Solid(
                self.colors
                    .selection
                    .background
                    .with_alpha(self.opacity.selection),
            ),
            caret: if state.disabled {
                self.colors.content.disabled
            } else {
                self.colors.content.primary
            },
            padding_x: self.controls.padding_x * 0.5,
            padding_y: self.controls.padding_y,
            font: self.typography.body,
        }
    }

    /// Resolves a shadow recipe from an elevation token and radius.
    #[must_use]
    pub fn elevation_shadow(&self, elevation: f32, radius: f32) -> Option<ShadowRecipe> {
        if !elevation.is_finite() || elevation <= 0.0 {
            return None;
        }
        Some(ShadowRecipe {
            offset: Vec2::new(0.0, elevation * 0.75),
            blur_radius: (elevation * 4.0).max(1.0),
            spread: 0.0,
            radius: radius.max(0.0),
            color: Color::rgba(0.0, 0.0, 0.0, (0.18 + elevation * 0.018).min(0.34)),
        })
    }

    /// Resolves a passive panel recipe.
    #[must_use]
    pub fn panel(&self) -> PanelRecipe {
        PanelRecipe {
            background: Brush::Solid(self.colors.surface.panel_raised),
            border: Stroke::new(
                self.controls.border_width,
                Brush::Solid(self.colors.border.default),
            ),
            radius: self.radii.sm,
            shadow: None,
        }
    }

    /// Resolves a separator recipe.
    #[must_use]
    pub fn separator(&self) -> SeparatorRecipe {
        SeparatorRecipe {
            stroke: Stroke::new(
                self.controls.separator_width,
                Brush::Solid(self.colors.border.subtle),
            ),
        }
    }
}
