//! Complete immutable Phosphor 2.1.1 icon definitions for Stern.
//!
//! Definitions are generated as independent statics. Public re-exports do not
//! form a catalog table, allowing the linker to discard every unused icon.

use stern_core::StaticIcon;

#[rustfmt::skip]
mod generated;

pub use generated::{bold, duotone, fill, light, regular, thin};

/// Exact upstream package version used to generate these definitions.
pub const UPSTREAM_VERSION: &str = "2.1.1";
/// Number of canonical icon names in each weight namespace.
pub const ICONS_PER_WEIGHT: usize = 1_512;
/// Number of generated canonical definitions across all weights.
pub const ICON_DEFINITION_COUNT: usize = 9_072;
/// Number of deprecated alias constants in each weight namespace.
pub const ALIASES_PER_WEIGHT: usize = 18;
/// Upstream 2.1.1 exposes no RTL metadata field; no RTL value is inferred.
pub const HAS_RTL_METADATA: bool = false;

/// Official Phosphor visual weight.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Weight {
    /// One-pixel visual weight.
    Thin,
    /// Light visual weight.
    Light,
    /// Default regular visual weight.
    Regular,
    /// Bold visual weight.
    Bold,
    /// Solid filled shapes.
    Fill,
    /// Two-tone ordered opacity layers.
    Duotone,
}

impl Weight {
    /// Returns the exact upstream spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Thin => "thin",
            Self::Light => "light",
            Self::Regular => "regular",
            Self::Bold => "bold",
            Self::Fill => "fill",
            Self::Duotone => "duotone",
        }
    }
}

/// Copyable typed handle for one Phosphor definition.
#[derive(Clone, Copy, Debug)]
pub struct PhosphorIcon {
    name: &'static str,
    weight: Weight,
    identity: &'static str,
    icon: StaticIcon,
}

impl PhosphorIcon {
    /// Constructs one generated definition.
    #[doc(hidden)]
    #[must_use]
    pub const fn new(
        name: &'static str,
        weight: Weight,
        identity: &'static str,
        icon: StaticIcon,
    ) -> Self {
        Self {
            name,
            weight,
            identity,
            icon,
        }
    }

    /// Returns the canonical kebab-case upstream name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        self.name
    }

    /// Returns this definition's visual weight.
    #[must_use]
    pub const fn weight(self) -> Weight {
        self.weight
    }

    /// Returns a stable per-definition identity marker.
    #[must_use]
    pub const fn identity(self) -> &'static str {
        self.identity
    }

    /// Returns the backend-independent immutable Stern icon handle.
    #[must_use]
    pub const fn icon(self) -> StaticIcon {
        self.icon
    }
}

impl From<PhosphorIcon> for StaticIcon {
    fn from(value: PhosphorIcon) -> Self {
        value.icon
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ALIASES_PER_WEIGHT, HAS_RTL_METADATA, ICON_DEFINITION_COUNT, ICONS_PER_WEIGHT, Weight,
        bold, duotone, fill, light, regular, thin,
    };

    #[test]
    fn exposes_complete_six_weight_shape() {
        assert_eq!(ICONS_PER_WEIGHT, 1_512);
        assert_eq!(ICON_DEFINITION_COUNT, 9_072);
        assert_eq!(ALIASES_PER_WEIGHT, 18);
        assert!(!std::hint::black_box(HAS_RTL_METADATA));
        assert_eq!(thin::AIRPLANE.weight(), Weight::Thin);
        assert_eq!(light::AIRPLANE.weight(), Weight::Light);
        assert_eq!(regular::FLOPPY_DISK.weight(), Weight::Regular);
        assert_eq!(bold::AIRPLANE.weight(), Weight::Bold);
        assert_eq!(fill::AIRPLANE.weight(), Weight::Fill);
        assert_eq!(duotone::AIRPLANE.weight(), Weight::Duotone);
    }

    #[test]
    fn definitions_are_borrowed_and_retain_duotone_opacity() {
        let regular = regular::FLOPPY_DISK.icon().graphic();
        assert!(!regular.layers.is_empty());
        assert!(!regular.layers[0].paths[0].elements.is_empty());
        let duotone = duotone::AIRPLANE.icon().graphic();
        assert!(duotone.layers.iter().any(|layer| layer.opacity < 1.0));
        assert_eq!(regular::FLOPPY_DISK.name(), "floppy-disk");
        assert!(
            regular::FLOPPY_DISK
                .identity()
                .contains("regular:floppy-disk")
        );
    }

    #[test]
    #[allow(deprecated)]
    fn deprecated_alias_is_the_canonical_handle() {
        assert_eq!(regular::CADUCEUS.icon(), regular::ASCLEPIUS.icon());
        assert_eq!(regular::CADUCEUS.name(), "asclepius");
    }

    #[test]
    fn linkage_sentinel_is_a_real_unrelated_definition() {
        assert_eq!(regular::ANCHOR.name(), "anchor");
        assert_eq!(regular::ANCHOR.weight(), Weight::Regular);
        assert_eq!(
            regular::ANCHOR.identity(),
            "STERN_PHOSPHOR_2_1_1:regular:anchor"
        );
        assert!(!regular::ANCHOR.icon().graphic().layers.is_empty());
    }
}
