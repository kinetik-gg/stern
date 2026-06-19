//! Theme tokens and component recipes.

use crate::{Brush, Color, CornerRadius, Rect, ShadowPrimitive, Stroke, Vec2};

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
    /// Disabled surface or affordance fill.
    Disabled,
    /// Floating overlay surface.
    Overlay,
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
    /// Disabled surface or affordance fill.
    pub disabled: Color,
    /// Floating overlay surface.
    pub overlay: Color,
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
            SemanticColor::Disabled => self.disabled,
            SemanticColor::Overlay => self.overlay,
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

impl SpacingScale {
    /// Creates a spacing scale.
    #[must_use]
    pub const fn new(xs: f32, sm: f32, md: f32, lg: f32, xl: f32) -> Self {
        Self { xs, sm, md, lg, xl }
    }
}

/// Radius token scale.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RadiusScale {
    /// No rounding.
    pub none: CornerRadius,
    /// Extra-small corner radius.
    pub xs: CornerRadius,
    /// Small corner radius.
    pub sm: CornerRadius,
    /// Medium corner radius.
    pub md: CornerRadius,
    /// Large corner radius.
    pub lg: CornerRadius,
    /// Fully rounded pill radius for height-bound controls.
    pub pill: CornerRadius,
}

impl RadiusScale {
    /// Creates an equal-corner radius scale from scalar values.
    #[must_use]
    pub const fn from_values(xs: f32, sm: f32, md: f32, lg: f32, pill: f32) -> Self {
        Self {
            none: CornerRadius::all(0.0),
            xs: CornerRadius::all(xs),
            sm: CornerRadius::all(sm),
            md: CornerRadius::all(md),
            lg: CornerRadius::all(lg),
            pill: CornerRadius::all(pill),
        }
    }
}

/// Font and line metrics for a text role.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FontToken {
    /// Font family name or logical family.
    pub family: &'static str,
    /// Font size in logical units.
    pub size: f32,
    /// Line height in logical units.
    pub line_height: f32,
}

impl FontToken {
    /// Creates a font token.
    #[must_use]
    pub const fn new(family: &'static str, size: f32, line_height: f32) -> Self {
        Self {
            family,
            size,
            line_height,
        }
    }
}

/// Semantic text style role.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextRole {
    /// Body copy and ordinary labels.
    Body,
    /// Compact labels inside controls.
    Label,
    /// Secondary captions.
    Caption,
    /// Section or panel headings.
    Title,
    /// Monospace values and code-like labels.
    Monospace,
}

/// Typography token scale.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TypographyScale {
    /// Body copy and ordinary labels.
    pub body: FontToken,
    /// Compact labels inside controls.
    pub label: FontToken,
    /// Secondary captions.
    pub caption: FontToken,
    /// Section or panel headings.
    pub title: FontToken,
    /// Monospace values and code-like labels.
    pub monospace: FontToken,
}

impl TypographyScale {
    /// Returns a font token for a text role.
    #[must_use]
    pub const fn get(self, role: TextRole) -> FontToken {
        match role {
            TextRole::Body => self.body,
            TextRole::Label => self.label,
            TextRole::Caption => self.caption,
            TextRole::Title => self.title,
            TextRole::Monospace => self.monospace,
        }
    }
}

/// Opacity tokens for state overlays and disabled affordances.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OpacityScale {
    /// Disabled content opacity.
    pub disabled: f32,
    /// Hover overlay opacity.
    pub hover: f32,
    /// Pressed overlay opacity.
    pub pressed: f32,
    /// Selection fill opacity.
    pub selection: f32,
    /// Modal or menu scrim opacity.
    pub overlay_scrim: f32,
}

/// Elevation tokens for surfaces that escape the base layout.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ElevationScale {
    /// Flat base layer.
    pub flat: f32,
    /// Raised panel or control layer.
    pub raised: f32,
    /// Overlay, menu, or popover layer.
    pub overlay: f32,
}

/// Renderer-neutral shadow style for an elevated surface.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShadowRecipe {
    /// Shadow offset in logical units.
    pub offset: Vec2,
    /// Gaussian blur radius in logical units.
    pub blur_radius: f32,
    /// Amount to expand or shrink the source rectangle before blurring.
    pub spread: f32,
    /// Uniform corner radius for the shadow shape.
    pub radius: f32,
    /// Shadow color.
    pub color: Color,
}

impl ShadowRecipe {
    /// Creates a shadow primitive for a rectangle.
    #[must_use]
    pub const fn primitive(self, rect: Rect) -> ShadowPrimitive {
        ShadowPrimitive::new(
            rect,
            self.offset,
            self.blur_radius,
            self.spread,
            self.radius,
            self.color,
        )
    }
}

/// Motion duration tokens in milliseconds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DurationScale {
    /// Immediate transition.
    pub instant: f32,
    /// Fast affordance transition.
    pub fast: f32,
    /// Ordinary transition.
    pub normal: f32,
    /// Deliberate transition.
    pub slow: f32,
}

/// Control sizing and stroke metrics.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ControlMetrics {
    /// Default one-line control height.
    pub control_height: f32,
    /// Compact control height.
    pub compact_control_height: f32,
    /// Icon glyph side length.
    pub icon_size: f32,
    /// Checkbox and radio side length.
    pub check_size: f32,
    /// Horizontal text/control padding.
    pub padding_x: f32,
    /// Vertical text/control padding.
    pub padding_y: f32,
    /// Default border width.
    pub border_width: f32,
    /// Focus ring stroke width.
    pub focus_width: f32,
    /// Separator stroke width.
    pub separator_width: f32,
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

/// Button visual variant.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ButtonVariant {
    /// Neutral raised button.
    #[default]
    Standard,
    /// Primary call-to-action button.
    Primary,
    /// Low-emphasis button with transparent fill.
    Ghost,
    /// Destructive button.
    Danger,
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

/// Text visual recipe output.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextRecipe {
    /// Foreground text color.
    pub foreground: Color,
    /// Text font token.
    pub font: FontToken,
}

/// Panel visual recipe output.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PanelRecipe {
    /// Background brush.
    pub background: Brush,
    /// Border stroke.
    pub border: Stroke,
    /// Corner radius.
    pub radius: CornerRadius,
    /// Optional panel shadow.
    pub shadow: Option<ShadowRecipe>,
}

/// Separator visual recipe output.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SeparatorRecipe {
    /// Separator stroke.
    pub stroke: Stroke,
}

/// Tab visual recipe output.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TabRecipe {
    /// Background brush.
    pub background: Brush,
    /// Text/icon color.
    pub foreground: Color,
    /// Border stroke.
    pub border: Stroke,
    /// Corner radius.
    pub radius: CornerRadius,
    /// Optional active indicator brush.
    pub indicator: Option<Brush>,
    /// Active indicator thickness.
    pub indicator_thickness: f32,
}

/// List or table row recipe output.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RowRecipe {
    /// Background brush.
    pub background: Brush,
    /// Text/icon color.
    pub foreground: Color,
    /// Border stroke.
    pub border: Stroke,
    /// Corner radius.
    pub radius: CornerRadius,
}

/// Checkbox and radio visual recipe output.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CheckRecipe {
    /// Box or circle fill.
    pub fill: Brush,
    /// Mark color.
    pub mark: Color,
    /// Border stroke.
    pub border: Stroke,
    /// Corner radius.
    pub radius: CornerRadius,
    /// Box or circle side length.
    pub size: f32,
}

/// Toggle visual recipe output.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ToggleRecipe {
    /// Track fill.
    pub track: Brush,
    /// Thumb fill.
    pub thumb: Brush,
    /// Track border.
    pub border: Stroke,
    /// Inner track padding.
    pub padding: f32,
}

/// Slider visual recipe output.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SliderRecipe {
    /// Track fill.
    pub track: Brush,
    /// Filled range brush.
    pub fill: Brush,
    /// Track border.
    pub border: Stroke,
    /// Track radius.
    pub radius: CornerRadius,
}

/// Text field visual recipe output.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextFieldRecipe {
    /// Background brush.
    pub background: Brush,
    /// Text color.
    pub foreground: Color,
    /// Border stroke.
    pub border: Stroke,
    /// Corner radius.
    pub radius: CornerRadius,
    /// Selection fill brush.
    pub selection: Brush,
    /// Caret color.
    pub caret: Color,
    /// Horizontal padding.
    pub padding_x: f32,
    /// Vertical padding.
    pub padding_y: f32,
    /// Font token.
    pub font: FontToken,
}

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
                self.colors.text_disabled
            } else {
                self.colors.text
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
        let mut background = match variant {
            ButtonVariant::Standard => self.colors.surface_raised,
            ButtonVariant::Primary => self.colors.accent,
            ButtonVariant::Ghost => Color::TRANSPARENT,
            ButtonVariant::Danger => self.colors.danger,
        };
        if state.disabled {
            background = self.colors.disabled;
        } else if state.selected || state.pressed {
            background = match variant {
                ButtonVariant::Standard | ButtonVariant::Ghost => self.colors.surface_active,
                ButtonVariant::Primary => self.colors.accent.with_alpha(0.86),
                ButtonVariant::Danger => self.colors.danger.with_alpha(0.86),
            };
        } else if state.hovered {
            background = match variant {
                ButtonVariant::Standard | ButtonVariant::Ghost => self.colors.surface_hover,
                ButtonVariant::Primary => self.colors.accent.with_alpha(0.92),
                ButtonVariant::Danger => self.colors.danger.with_alpha(0.92),
            };
        }

        let foreground = if state.disabled {
            self.colors.text_disabled
        } else if matches!(variant, ButtonVariant::Primary | ButtonVariant::Danger) {
            Color::WHITE
        } else {
            self.colors.text
        };
        let border_color = if state.focused {
            self.colors.focus_ring
        } else if matches!(variant, ButtonVariant::Ghost) {
            self.colors.border_subtle
        } else {
            self.colors.border
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
            self.colors.disabled
        } else if state.selected || state.pressed {
            self.colors.surface_active
        } else if state.hovered {
            self.colors.surface_hover
        } else {
            self.colors.surface
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

        TabRecipe {
            background: Brush::Solid(background),
            foreground,
            border: Stroke::new(self.controls.border_width, Brush::Solid(border_color)),
            radius: self.radii.none,
            indicator: state.selected.then_some(Brush::Solid(self.colors.accent)),
            indicator_thickness: 2.0,
        }
    }

    /// Resolves a list or table row recipe for a state.
    #[must_use]
    pub fn row(&self, state: ComponentState) -> RowRecipe {
        let background = if state.disabled {
            self.colors.disabled
        } else if state.selected {
            self.colors.selection
        } else if state.hovered {
            self.colors.surface_hover
        } else {
            self.colors.surface_sunken
        };
        let foreground = if state.disabled {
            self.colors.text_disabled
        } else {
            self.colors.text
        };
        RowRecipe {
            background: Brush::Solid(background),
            foreground,
            border: Stroke::new(
                self.controls.border_width,
                Brush::Solid(self.colors.border_subtle),
            ),
            radius: self.radii.none,
        }
    }

    /// Resolves a checkbox recipe for a state.
    #[must_use]
    pub fn checkbox(&self, state: ComponentState) -> CheckRecipe {
        let fill = if state.disabled {
            self.colors.disabled
        } else if state.selected {
            self.colors.accent
        } else if state.hovered {
            self.colors.surface_hover
        } else {
            self.colors.surface_sunken
        };
        let border_color = if state.focused {
            self.colors.focus_ring
        } else {
            self.colors.border
        };
        CheckRecipe {
            fill: Brush::Solid(fill),
            mark: if state.disabled {
                self.colors.text_disabled
            } else {
                Color::WHITE
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
            self.colors.disabled
        } else if state.selected {
            self.colors.accent
        } else if state.hovered {
            self.colors.surface_hover
        } else {
            self.colors.surface_active
        };
        let thumb = if state.disabled {
            self.colors.text_disabled
        } else {
            self.colors.text
        };
        let border_color = if state.focused {
            self.colors.focus_ring
        } else {
            self.colors.border
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
            self.colors.text_disabled
        } else {
            self.colors.accent
        };
        let border_color = if state.focused {
            self.colors.focus_ring
        } else {
            self.colors.border
        };
        SliderRecipe {
            track: Brush::Solid(self.colors.surface_sunken),
            fill: Brush::Solid(fill),
            border: Stroke::new(self.controls.border_width, Brush::Solid(border_color)),
            radius: self.radii.pill,
        }
    }

    /// Resolves a text field recipe for a state.
    #[must_use]
    pub fn text_field(&self, state: ComponentState) -> TextFieldRecipe {
        let border_color = if state.focused {
            self.colors.accent
        } else if state.hovered {
            self.colors.border
        } else {
            self.colors.border_subtle
        };
        TextFieldRecipe {
            background: Brush::Solid(if state.disabled {
                self.colors.disabled
            } else {
                self.colors.surface_sunken
            }),
            foreground: if state.disabled {
                self.colors.text_disabled
            } else {
                self.colors.text
            },
            border: Stroke::new(self.controls.border_width, Brush::Solid(border_color)),
            radius: self.radii.sm,
            selection: Brush::Solid(self.colors.selection.with_alpha(self.opacity.selection)),
            caret: if state.disabled {
                self.colors.text_disabled
            } else {
                self.colors.text
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
            background: Brush::Solid(self.colors.surface_raised),
            border: Stroke::new(self.controls.border_width, Brush::Solid(self.colors.border)),
            radius: self.radii.sm,
            shadow: self.elevation_shadow(self.elevation.raised, self.radii.sm.top_left),
        }
    }

    /// Resolves a separator recipe.
    #[must_use]
    pub fn separator(&self) -> SeparatorRecipe {
        SeparatorRecipe {
            stroke: Stroke::new(
                self.controls.separator_width,
                Brush::Solid(self.colors.border_subtle),
            ),
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
            disabled: Color::rgb(0.075, 0.075, 0.075),
            overlay: Color::rgb(0.105, 0.105, 0.105),
            viewport_background: Color::rgb(0.02, 0.02, 0.02),
        },
        spacing: SpacingScale::new(2.0, 4.0, 8.0, 12.0, 16.0),
        radii: RadiusScale::from_values(2.0, 3.0, 5.0, 8.0, 999.0),
        typography: TypographyScale {
            body: FontToken::new("sans-serif", 12.0, 17.0),
            label: FontToken::new("sans-serif", 12.0, 16.0),
            caption: FontToken::new("sans-serif", 11.0, 15.0),
            title: FontToken::new("sans-serif", 14.0, 19.0),
            monospace: FontToken::new("monospace", 12.0, 17.0),
        },
        opacity: OpacityScale {
            disabled: 0.45,
            hover: 0.08,
            pressed: 0.14,
            selection: 0.35,
            overlay_scrim: 0.55,
        },
        elevation: ElevationScale {
            flat: 0.0,
            raised: 1.0,
            overlay: 8.0,
        },
        duration: DurationScale {
            instant: 0.0,
            fast: 80.0,
            normal: 140.0,
            slow: 220.0,
        },
        controls: ControlMetrics {
            control_height: 28.0,
            compact_control_height: 22.0,
            icon_size: 16.0,
            check_size: 14.0,
            padding_x: 8.0,
            padding_y: 4.0,
            border_width: 1.0,
            focus_width: 1.0,
            separator_width: 1.0,
        },
        radius: CornerRadius::all(3.0),
        border_width: 1.0,
        text_size: 12.0,
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::{
        ButtonVariant, ComponentState, ControlMetrics, DurationScale, ElevationScale, OpacityScale,
        RadiusScale, SemanticColor, SpacingScale, TextRole, TypographyScale, default_dark_theme,
    };
    use crate::{Brush, Color, CornerRadius};

    #[test]
    fn resolves_semantic_colors() {
        let theme = default_dark_theme();

        assert_eq!(theme.color(SemanticColor::Accent), theme.colors.accent);
        assert_eq!(
            theme.color(SemanticColor::TextMuted),
            theme.colors.text_muted
        );
        assert_eq!(theme.color(SemanticColor::Overlay), theme.colors.overlay);
    }

    #[test]
    fn default_theme_has_dense_editor_spacing() {
        let theme = default_dark_theme();

        assert_eq!(theme.spacing.md, 8.0);
        assert_eq!(theme.text_size, 12.0);
        assert_eq!(theme.border_width, 1.0);
        assert_eq!(theme.controls.control_height, 28.0);
        assert_eq!(theme.controls.icon_size, 16.0);
        assert_eq!(theme.font(TextRole::Body).line_height, 17.0);
    }

    #[test]
    fn token_overrides_are_structural_and_predictable() {
        let typography = TypographyScale {
            body: super::FontToken::new("sans-serif", 13.0, 18.0),
            ..default_dark_theme().typography
        };
        let controls = ControlMetrics {
            border_width: 2.0,
            ..default_dark_theme().controls
        };
        let theme = default_dark_theme()
            .with_spacing(SpacingScale::new(1.0, 3.0, 6.0, 9.0, 12.0))
            .with_radii(RadiusScale::from_values(1.0, 2.0, 3.0, 4.0, 999.0))
            .with_typography(typography)
            .with_opacity(OpacityScale {
                hover: 0.2,
                ..default_dark_theme().opacity
            })
            .with_elevation(ElevationScale {
                raised: 3.0,
                ..default_dark_theme().elevation
            })
            .with_duration(DurationScale {
                normal: 180.0,
                ..default_dark_theme().duration
            })
            .with_controls(controls);

        assert_eq!(theme.spacing.xs, 1.0);
        assert_eq!(theme.spacing.md, 6.0);
        assert_eq!(theme.radii.sm, CornerRadius::all(2.0));
        assert_eq!(theme.radius, CornerRadius::all(2.0));
        assert_eq!(theme.text_size, 13.0);
        assert_eq!(theme.opacity.hover, 0.2);
        assert_eq!(theme.elevation.raised, 3.0);
        assert_eq!(theme.duration.normal, 180.0);
        assert_eq!(theme.controls.border_width, 2.0);
        assert_eq!(theme.border_width, 2.0);
        assert_eq!(theme.colors, default_dark_theme().colors);
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
        let primary = theme.button_variant(ButtonVariant::Primary, ComponentState::default());

        assert_eq!(normal.background, Brush::Solid(theme.colors.surface_raised));
        assert_eq!(hovered.background, Brush::Solid(theme.colors.surface_hover));
        assert_eq!(focused.border.brush, Brush::Solid(theme.colors.focus_ring));
        assert_eq!(disabled.foreground, theme.colors.text_disabled);
        assert_eq!(primary.background, Brush::Solid(theme.colors.accent));
        assert_eq!(primary.foreground, Color::WHITE);
    }

    #[test]
    fn component_recipes_cover_common_states() {
        let theme = default_dark_theme();
        let selected = ComponentState {
            selected: true,
            ..ComponentState::default()
        };
        let focused = ComponentState {
            focused: true,
            ..ComponentState::default()
        };

        assert_eq!(
            theme.tab(selected).indicator,
            Some(Brush::Solid(theme.colors.accent))
        );
        assert_eq!(
            theme.row(selected).background,
            Brush::Solid(theme.colors.selection)
        );
        assert_eq!(
            theme.checkbox(selected).fill,
            Brush::Solid(theme.colors.accent)
        );
        assert_eq!(
            theme.toggle(selected).track,
            Brush::Solid(theme.colors.accent)
        );
        assert_eq!(
            theme.slider(focused).border.brush,
            Brush::Solid(theme.colors.focus_ring)
        );
        assert_eq!(
            theme.text_field(focused).border.brush,
            Brush::Solid(theme.colors.accent)
        );
        assert!(theme.panel().shadow.is_some());
    }

    #[test]
    fn elevation_shadow_materializes_shadow_primitives() {
        let theme = default_dark_theme();
        let shadow = theme
            .elevation_shadow(theme.elevation.overlay, theme.radii.md.top_left)
            .expect("overlay elevation casts a shadow");
        let primitive = shadow.primitive(crate::Rect::new(0.0, 0.0, 20.0, 10.0));

        assert_eq!(primitive.rect, crate::Rect::new(0.0, 0.0, 20.0, 10.0));
        assert!(primitive.blur_radius > theme.elevation.overlay);
        assert_eq!(primitive.radius, theme.radii.md.top_left);
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
