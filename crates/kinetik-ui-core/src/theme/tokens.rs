use crate::{Color, CornerRadius, Rect, ShadowPrimitive, Vec2};

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
