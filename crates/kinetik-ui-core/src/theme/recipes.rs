use super::{FontToken, ShadowRecipe};
use crate::{Brush, Color, CornerRadius, Stroke};

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
