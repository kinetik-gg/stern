//! Theme tokens and component recipes.

mod defaults;
#[rustfmt::skip] // Vendored generated file; formatting must stay byte-stable.
pub mod generated_tokens;
mod model;
mod recipes;
#[cfg(test)]
mod tests;
mod tokens;

pub use defaults::default_dark_theme;
pub use model::Theme;
pub use recipes::{
    ButtonRecipe, ButtonVariant, CheckRecipe, ComponentState, FocusRingRecipe, PanelRecipe,
    RowRecipe, SeparatorRecipe, SliderRecipe, TabRecipe, TextFieldRecipe, TextRecipe, ToggleRecipe,
};
pub use tokens::{
    AccentColors, BorderColors, ContentColors, ControlMetrics, ControlSizeScale, DurationScale,
    ElevationLevel, ElevationScale, FocusColors, FocusStrokeScale, FontFamilyRole, FontFamilyScale,
    FontFeatureScale, FontFeatureToken, FontLineHeightScale, FontLineHeightToken, FontSizeScale,
    FontSizeToken, FontToken, FontWeightScale, FontWeightToken, HandleSizeScale, IconSizeScale,
    OpacityScale, OverlayColors, RadiusScale, RowSizeScale, SelectionColors, SemanticColor,
    ShadowRecipe, SizeScale, SizeToken, SpacingRole, SpacingScale, SpacingStep,
    StatusColorFamilyColors, StatusColors, StrokeScale, SurfaceColors, TextRole, TextRoleMetrics,
    ThemeColors, TypographyScale,
};
