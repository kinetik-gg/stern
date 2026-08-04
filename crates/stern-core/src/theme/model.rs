use super::{
    ButtonRecipe, ButtonVariant, CheckRecipe, ComponentState, ControlMetrics, DurationScale,
    ElevationLevel, ElevationScale, FocusRingRecipe, FontFamilyRole, FontToken, OpacityScale,
    PanelRecipe, RadiusScale, RowRecipe, SemanticColor, SeparatorRecipe, ShadowRecipe, SizeScale,
    SliderRecipe, SpacingScale, StrokeScale, TabRecipe, TextFieldRecipe, TextRecipe, TextRole,
    ThemeColors, ToggleRecipe, TypographyScale,
};
use crate::{Brush, Color, CornerRadius, Stroke, Vec2};

const SELECTION_INDICATOR_SIZE: f32 = 14.0;

/// Complete theme.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Theme {
    /// Color tokens.
    pub colors: ThemeColors,
    /// Spacing tokens.
    pub spacing: SpacingScale,
    /// Size tokens.
    pub sizes: SizeScale,
    /// Radius tokens.
    pub radii: RadiusScale,
    /// Stroke-width tokens.
    pub strokes: StrokeScale,
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
    /// Legacy one-way mirror of [`Self::strokes`]'s `default` role.
    ///
    /// Recipes and widgets read [`Self::strokes`] directly. Prefer
    /// [`Self::with_strokes`] so this compatibility value stays synchronized.
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

    /// Resolves a semantic font-family role.
    #[must_use]
    pub const fn font_family(self, role: FontFamilyRole) -> &'static str {
        self.typography.family(role)
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

    /// Returns this theme with a replaced size scale.
    #[must_use]
    pub const fn with_sizes(mut self, sizes: SizeScale) -> Self {
        self.sizes = sizes;
        self
    }

    /// Returns this theme with a replaced radius scale.
    #[must_use]
    pub const fn with_radii(mut self, radii: RadiusScale) -> Self {
        self.radius = radii.sm;
        self.radii = radii;
        self
    }

    /// Returns this theme with a replaced stroke scale.
    #[must_use]
    pub const fn with_strokes(mut self, strokes: StrokeScale) -> Self {
        self.border_width = strokes.default;
        self.strokes = strokes;
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

    /// Returns this theme with replaced control sizing and padding metrics.
    #[must_use]
    pub const fn with_controls(mut self, controls: ControlMetrics) -> Self {
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

    /// Resolves the independent two-tone focus ring when it is visible.
    #[must_use]
    pub const fn focus_ring(&self, visible: bool) -> Option<FocusRingRecipe> {
        if !visible {
            return None;
        }
        Some(FocusRingRecipe {
            primary: Stroke::new(
                self.strokes.focus.primary,
                Brush::Solid(self.colors.focus.indicator),
            ),
            separator: Stroke::new(
                self.strokes.focus.separator,
                Brush::Solid(self.colors.focus.separator),
            ),
        })
    }

    /// Resolves the standard button recipe for a state.
    #[must_use]
    pub fn button(&self, state: ComponentState) -> ButtonRecipe {
        self.button_variant(ButtonVariant::Standard, state)
    }

    /// Resolves a button recipe for a visual variant and state.
    ///
    /// Values match `docs/visual-spec/01-buttons.md` (family issue #910),
    /// authoritative over `../stern-design-system`. `state.selected` without
    /// `state.pressed` is the "chosen" mode-choice state (selectable icon
    /// button, 01-buttons.md §Icon button): NEUTRAL `surface.control` fill +
    /// `border.strong` ring, not accent (00-language.md §Selection-vs-hover
    /// doctrine). A transient `state.pressed` takes precedence over a
    /// persistent chosen state for fill/text; both promote the border to
    /// `border.strong` equally.
    #[must_use]
    pub fn button_variant(&self, variant: ButtonVariant, state: ComponentState) -> ButtonRecipe {
        // Chosen (persistent "toggled on") vs. pressed (transient mouse-down):
        // both ComponentState flags can be true at once, so pressed wins.
        let chosen = state.selected && !state.pressed;

        let background = if state.disabled {
            // 01-buttons.md: every variant's disabled fill is S1
            // `surface.application`, not the `surface.control_disabled`
            // tier other families use.
            self.colors.surface.application
        } else {
            match variant {
                ButtonVariant::Standard => {
                    if state.pressed {
                        self.colors.surface.control_pressed
                    } else if chosen {
                        self.colors.surface.control
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
                    if state.pressed {
                        self.colors.surface.control_pressed
                    } else if chosen {
                        self.colors.surface.control
                    } else if state.hovered {
                        self.colors.surface.control_hover
                    } else {
                        Color::TRANSPARENT
                    }
                }
                // Danger's fill is constant across idle/hover/pressed
                // (`status.danger.surface`); only the border and pressed
                // text promote.
                ButtonVariant::Danger => self.colors.status.danger.surface,
            }
        };

        let border_color = if state.disabled {
            self.colors.border.disabled
        } else {
            match variant {
                ButtonVariant::Standard => {
                    if state.pressed || chosen || state.hovered {
                        self.colors.border.strong
                    } else {
                        self.colors.border.default
                    }
                }
                ButtonVariant::Ghost => {
                    if state.pressed || chosen || state.hovered {
                        self.colors.border.strong
                    } else {
                        Color::TRANSPARENT
                    }
                }
                ButtonVariant::Primary => Color::TRANSPARENT,
                ButtonVariant::Danger => {
                    if state.pressed || state.hovered {
                        self.colors.status.danger.strong
                    } else {
                        self.colors.status.danger.border
                    }
                }
            }
        };

        let foreground = if state.disabled {
            self.colors.content.disabled
        } else {
            match variant {
                ButtonVariant::Primary => self.colors.content.on_accent,
                ButtonVariant::Danger => {
                    if state.pressed {
                        self.colors.content.on_accent
                    } else {
                        self.colors.status.danger.foreground
                    }
                }
                ButtonVariant::Standard | ButtonVariant::Ghost => {
                    if state.pressed {
                        self.colors.content.primary
                    } else if chosen {
                        self.colors.content.secondary
                    } else if state.hovered {
                        self.colors.content.primary
                    } else {
                        self.colors.content.secondary
                    }
                }
            }
        };

        ButtonRecipe {
            background: Brush::Solid(background),
            foreground,
            border: Stroke::new(self.strokes.default, Brush::Solid(border_color)),
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
        TabRecipe {
            background: Brush::Solid(background),
            foreground,
            border: Stroke::new(
                self.strokes.default,
                Brush::Solid(self.colors.border.default),
            ),
            radius: self.radii.none,
            indicator: None,
            indicator_thickness: self.strokes.emphasis,
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
                self.strokes.hairline,
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
        CheckRecipe {
            fill: Brush::Solid(fill),
            mark: if state.disabled {
                self.colors.content.disabled
            } else {
                self.colors.content.on_accent
            },
            border: Stroke::new(
                self.strokes.default,
                Brush::Solid(self.colors.border.default),
            ),
            radius: self.radii.sm,
            size: SELECTION_INDICATOR_SIZE,
        }
    }

    /// Resolves a radio button recipe for a state.
    #[must_use]
    pub fn radio_button(&self, state: ComponentState) -> CheckRecipe {
        CheckRecipe {
            radius: self.radii.full,
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
        ToggleRecipe {
            track: Brush::Solid(track),
            thumb: Brush::Solid(thumb),
            border: Stroke::new(
                self.strokes.default,
                Brush::Solid(self.colors.border.default),
            ),
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
        SliderRecipe {
            track: Brush::Solid(self.colors.surface.sunken),
            fill: Brush::Solid(fill),
            border: Stroke::new(
                self.strokes.default,
                Brush::Solid(self.colors.border.default),
            ),
            radius: self.radii.full,
        }
    }

    /// Resolves a text field recipe for a state.
    ///
    /// Values match `docs/visual-spec/02-fields.md` (family issue #911),
    /// authoritative over `../stern-design-system`. Per that file's single-
    /// line field table, fill never changes across idle/hover/focused (only
    /// `border.default`/`border.strong` and disabled diverge) — fields read
    /// as wells, buttons as raised, per `00-language.md` §Selection-vs-hover
    /// doctrine. `focused` resolves its border to `border.strong`, the same
    /// tier as `hovered`, not `border.focused`: `00-language.md`'s universal
    /// focus model draws the accent ring as a separate two-layer paint step
    /// outside the control bounds ("focus never recolors the control body"),
    /// so the border color itself never becomes the ring color. Caret and
    /// IME composition both key off `focus.ring` per the same file's Caret
    /// note; selection highlight is `selection.background` at full opacity
    /// (no alpha — the previous `opacity.selection` blend had no basis in
    /// the spec or a DS token). `read-only` and `invalid` are in the spec's
    /// state table but are not resolved here: `ComponentState` has no field
    /// for either (see `KNOWN-GAPS.md`).
    #[must_use]
    pub fn text_field(&self, state: ComponentState) -> TextFieldRecipe {
        let border_color = if state.disabled {
            self.colors.border.disabled
        } else if state.focused || state.hovered {
            self.colors.border.strong
        } else {
            self.colors.border.default
        };
        TextFieldRecipe {
            background: Brush::Solid(if state.disabled {
                self.colors.surface.control_disabled
            } else {
                self.colors.surface.control
            }),
            foreground: if state.disabled {
                self.colors.content.disabled
            } else {
                self.colors.content.primary
            },
            border: Stroke::new(self.strokes.default, Brush::Solid(border_color)),
            radius: self.radii.sm,
            selection: Brush::Solid(self.colors.selection.background),
            caret: self.colors.focus.ring,
            padding_x: self.controls.padding_x,
            padding_y: self.controls.padding_y,
            font: self.typography.get(TextRole::Body),
        }
    }

    /// Resolves the exact shadow recipe for a typed elevation level and radius.
    #[must_use]
    pub fn elevation_shadow(&self, level: ElevationLevel, radius: f32) -> Option<ShadowRecipe> {
        let (offset_y, blur_radius, alpha) = match level {
            ElevationLevel::None => return None,
            ElevationLevel::Low => (2.0, 6.0, 0.32),
            ElevationLevel::Medium => (6.0, 18.0, 0.42),
            ElevationLevel::High => (12.0, 36.0, 0.52),
        };
        Some(ShadowRecipe {
            offset: Vec2::new(0.0, offset_y),
            blur_radius,
            spread: 0.0,
            radius: radius.max(0.0),
            color: Color::rgba(0.0, 0.0, 0.0, alpha),
        })
    }

    /// Resolves a passive panel recipe.
    #[must_use]
    pub fn panel(&self) -> PanelRecipe {
        PanelRecipe {
            background: Brush::Solid(self.colors.surface.panel_raised),
            border: Stroke::new(
                self.strokes.default,
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
                self.strokes.hairline,
                Brush::Solid(self.colors.border.subtle),
            ),
        }
    }
}
