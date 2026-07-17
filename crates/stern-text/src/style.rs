use stern_core::{FontFeatureScale, FontFeatureToken};

/// Fixed set of supported low-level text-shaping features.
///
/// Stern intentionally exposes only named feature combinations instead of
/// arbitrary OpenType tags. Semantic token lookup remains owned by the theme
/// foundation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextFeatureSet(u8);

impl TextFeatureSet {
    /// No optional shaping features.
    pub const NONE: Self = Self(0);
    /// Tabular numeric figures (`tnum=1`).
    pub const TABULAR_NUMBERS: Self = Self(1 << 0);

    /// Resolves one semantic foundation token into Stern's bounded feature set.
    ///
    /// Customized values that Stern does not support fail soft instead of
    /// exposing arbitrary OpenType tags through the text API.
    #[must_use]
    pub fn resolve_semantic(scale: FontFeatureScale, token: FontFeatureToken) -> Option<Self> {
        match token {
            FontFeatureToken::Numeric if scale.get(token) == "tabular-nums" => {
                Some(Self::TABULAR_NUMBERS)
            }
            FontFeatureToken::Numeric => None,
        }
    }

    pub(crate) const fn has_tabular_numbers(self) -> bool {
        self.0 & Self::TABULAR_NUMBERS.0 != 0
    }

    pub(crate) const fn ordering_key(self) -> u8 {
        self.0
    }
}

impl Default for TextFeatureSet {
    fn default() -> Self {
        Self::NONE
    }
}

/// Presentation policy applied when laid-out text exceeds its width.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TextOverflow {
    /// Preserve the complete shaped glyph presentation even when it exceeds the width.
    #[default]
    Visible,
    /// Replace the hidden end of one eligible display line with an ellipsis glyph.
    EndEllipsis,
}

/// Font properties used by text measurement and layout.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TextStyle {
    /// Font family name.
    pub family: String,
    /// Font size in logical units.
    pub size_bits: u32,
    /// Line height in logical units.
    pub line_height_bits: u32,
    /// Requested font weight as an exact OpenType-compatible numeric value.
    pub weight: u16,
    /// Optional low-level shaping features.
    pub features: TextFeatureSet,
}

impl TextStyle {
    /// Creates a text style from logical sizes.
    #[must_use]
    pub fn new(family: impl Into<String>, size: f32, line_height: f32) -> Self {
        Self {
            family: family.into(),
            size_bits: size.to_bits(),
            line_height_bits: line_height.to_bits(),
            weight: 400,
            features: TextFeatureSet::NONE,
        }
    }

    /// Sets the exact requested font weight.
    #[must_use]
    pub const fn with_weight(mut self, weight: u16) -> Self {
        self.weight = weight;
        self
    }

    /// Sets the low-level shaping features for this style.
    #[must_use]
    pub const fn with_features(mut self, features: TextFeatureSet) -> Self {
        self.features = features;
        self
    }

    /// Returns the font size.
    #[must_use]
    pub const fn size(&self) -> f32 {
        f32::from_bits(self.size_bits)
    }

    /// Returns the line height.
    #[must_use]
    pub const fn line_height(&self) -> f32 {
        f32::from_bits(self.line_height_bits)
    }
}

/// Request for measuring or laying out text.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TextLayoutKey {
    /// Text content.
    pub text: String,
    /// Style.
    pub style: TextStyle,
    /// Maximum width in logical units.
    pub width_bits: u32,
    /// Whether text may wrap.
    pub wrap: bool,
    /// Presentation policy for text that exceeds the requested width.
    pub overflow: TextOverflow,
}

impl TextLayoutKey {
    /// Creates a text layout key.
    #[must_use]
    pub fn new(text: impl Into<String>, style: TextStyle, width: f32, wrap: bool) -> Self {
        Self {
            text: text.into(),
            style,
            width_bits: width.to_bits(),
            wrap,
            overflow: TextOverflow::Visible,
        }
    }

    /// Sets the presentation policy for text that exceeds the requested width.
    #[must_use]
    pub const fn with_overflow(mut self, overflow: TextOverflow) -> Self {
        self.overflow = overflow;
        self
    }

    /// Returns the maximum width.
    #[must_use]
    pub const fn width(&self) -> f32 {
        f32::from_bits(self.width_bits)
    }
}
