//! Complete six-weight asset discovery.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    str::FromStr,
};

use crate::{Catalog, Error, ErrorKind, Result, Snapshot, StableId, assign_stable_ids};

/// Official Phosphor icon weight.
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
    /// Two ordered opacity layers.
    Duotone,
}

impl Weight {
    /// All source weights in stable public order.
    pub const ALL: [Self; 6] = [
        Self::Thin,
        Self::Light,
        Self::Regular,
        Self::Bold,
        Self::Fill,
        Self::Duotone,
    ];

    /// Exact upstream directory spelling.
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

    fn filename(self, name: &str) -> String {
        if self == Self::Regular {
            format!("{name}.svg")
        } else {
            format!("{name}-{}.svg", self.as_str())
        }
    }
}

impl fmt::Display for Weight {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for Weight {
    type Err = ();
    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|weight| weight.as_str() == value)
            .ok_or(())
    }
}

/// One verified weight asset.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Asset {
    /// Canonical icon name.
    pub name: String,
    /// Source weight.
    pub weight: Weight,
    /// Exact path inside the pinned archive.
    pub archive_path: String,
    /// Stable identity including canonical name and weight.
    pub stable_id: StableId,
}

/// One canonical icon and all its weight assets.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiscoveredIcon {
    /// Canonical catalog name.
    pub name: String,
    /// Exactly six assets in [`Weight::ALL`] order.
    pub assets: Vec<Asset>,
}

/// Complete deterministic source inventory.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Discovery {
    /// Icons in canonical catalog order.
    pub icons: Vec<DiscoveredIcon>,
}

impl Discovery {
    /// Discovers and cross-validates every SVG against the official catalog.
    ///
    /// # Errors
    ///
    /// Returns a discovery error for missing, extra, duplicate, or misnamed assets.
    pub fn from_snapshot(snapshot: &Snapshot, catalog: &Catalog) -> Result<Self> {
        Self::from_paths(catalog, snapshot.paths())
    }

    /// Cross-validates an explicit archive path inventory.
    ///
    /// This seam lets tooling diagnose incomplete inventories before reading
    /// asset payloads and makes missing/extra-weight behavior independently
    /// testable.
    ///
    /// # Errors
    ///
    /// Returns a discovery, name, or stable-ID collision error.
    #[allow(clippy::too_many_lines)]
    pub fn from_paths<'a>(
        catalog: &Catalog,
        paths: impl IntoIterator<Item = &'a str>,
    ) -> Result<Self> {
        let canonical: BTreeSet<&str> = catalog
            .records
            .iter()
            .map(|record| record.name.as_str())
            .collect();
        let mut found = BTreeMap::<Weight, BTreeSet<String>>::new();
        for path in paths {
            let Some(rest) = path.strip_prefix("package/assets/") else {
                continue;
            };
            let is_svg = std::path::Path::new(rest)
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("svg"));
            if !is_svg {
                continue;
            }
            let parts: Vec<_> = rest.split('/').collect();
            if parts.len() != 2 || parts.iter().any(|part| part.is_empty()) {
                return Err(Error::new(
                    ErrorKind::Discovery,
                    path,
                    "SVG asset path must be exactly `package/assets/<weight>/<filename>.svg`",
                ));
            }
            let directory = parts[0];
            let filename = parts[1];
            let weight = directory.parse::<Weight>().map_err(|()| {
                Error::new(
                    ErrorKind::Discovery,
                    path,
                    format!("unknown SVG weight directory `{directory}`"),
                )
            })?;
            let suffix = if weight == Weight::Regular {
                ".svg".to_owned()
            } else {
                format!("-{}.svg", weight.as_str())
            };
            let name = filename.strip_suffix(&suffix).ok_or_else(|| {
                Error::new(
                    ErrorKind::Discovery,
                    path,
                    format!(
                        "filename does not use the `{}` weight convention",
                        weight.as_str()
                    ),
                )
            })?;
            if name.is_empty() {
                return Err(Error::new(
                    ErrorKind::Discovery,
                    path,
                    "asset name is empty",
                ));
            }
            if !canonical.contains(name) {
                return Err(Error::new(
                    ErrorKind::Discovery,
                    path,
                    format!("asset `{name}` has no canonical catalog record"),
                ));
            }
            if !found.entry(weight).or_default().insert(name.to_owned()) {
                return Err(Error::new(
                    ErrorKind::Discovery,
                    path,
                    "duplicate weight asset",
                ));
            }
        }
        for weight in Weight::ALL {
            let names = found.get(&weight).ok_or_else(|| {
                Error::new(
                    ErrorKind::Discovery,
                    weight.as_str(),
                    "weight directory has no SVG assets",
                )
            })?;
            if names.len() != canonical.len() {
                let missing = canonical
                    .iter()
                    .find(|name| !names.contains(**name))
                    .copied()
                    .unwrap_or("none");
                return Err(Error::new(
                    ErrorKind::Discovery,
                    weight.as_str(),
                    format!(
                        "expected {} assets, found {}; first missing `{missing}`",
                        canonical.len(),
                        names.len()
                    ),
                ));
            }
        }

        let definitions = catalog
            .records
            .iter()
            .flat_map(|record| Weight::ALL.map(|weight| (record.name.as_str(), weight)));
        let ids = assign_stable_ids(definitions)?;
        let icons = catalog
            .records
            .iter()
            .map(|record| {
                let assets = Weight::ALL
                    .into_iter()
                    .map(|weight| {
                        let archive_path = format!(
                            "package/assets/{}/{}",
                            weight.as_str(),
                            weight.filename(&record.name)
                        );
                        let stable_id = ids[&(record.name.clone(), weight)];
                        Asset {
                            name: record.name.clone(),
                            weight,
                            archive_path,
                            stable_id,
                        }
                    })
                    .collect();
                DiscoveredIcon {
                    name: record.name.clone(),
                    assets,
                }
            })
            .collect();
        Ok(Self { icons })
    }

    /// Total number of weight assets.
    #[must_use]
    pub fn asset_count(&self) -> usize {
        self.icons.iter().map(|icon| icon.assets.len()).sum()
    }

    /// Iterates over all weight assets in catalog/weight order.
    pub fn assets(&self) -> impl Iterator<Item = &Asset> {
        self.icons.iter().flat_map(|icon| &icon.assets)
    }
}
