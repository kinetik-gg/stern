//! Offline, deterministic icon-source ingestion for Stern's Rust generators.
//!
//! This development-only crate performs no work unless its APIs are called.

mod archive;
mod catalog;
mod discovery;
mod error;
mod ids;
mod model;
mod naming;
mod svg;

pub use archive::{EXPECTED_NPM_INTEGRITY, EXPECTED_SHA256, Snapshot, SnapshotProvenance};
pub use catalog::{Catalog, CatalogAlias, CatalogRecord, RtlMetadata};
pub use discovery::{Asset, DiscoveredIcon, Discovery, Weight};
pub use error::{Error, ErrorKind, Result};
pub use ids::{StableId, assign_stable_ids, assign_stable_ids_with};
pub use model::{
    FillRule, NormalizedIcon, NormalizedPath, PathCommand, Point, StrokeCap, StrokeJoin,
    StrokeStyle,
};
pub use naming::{ConstantName, assign_constant_names, constant_name};
pub use svg::normalize_svg;

/// Path to the checked-in source archive relative to the workspace root.
pub const VENDORED_ARCHIVE: &str = "third-party/phosphor/phosphor-core-2.1.1.tgz";
