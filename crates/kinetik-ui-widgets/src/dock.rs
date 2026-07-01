//! `Dock`, `Frame`, and `Panel` models for editor layouts.

use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet},
};

use kinetik_ui_core::{ActionId, Axis, IconId, Point, Rect, Size, Vec2};

const DEFAULT_SPLIT_RATIO: f32 = 0.5;
const DEFAULT_SPLIT_MINIMUM: f32 = 100.0;
const DEFAULT_SPLITTER_THICKNESS: f32 = 6.0;
const DROP_EDGE_FRACTION: f32 = 0.25;

mod actions;
mod ids;
mod layout;
mod model;
mod policy;
mod registry;
mod snapshot;
mod tabs;

pub use actions::*;
pub use ids::*;
pub use layout::*;
pub use model::*;
pub use policy::*;
pub use registry::*;
pub use snapshot::*;
pub use tabs::*;

#[cfg(test)]
mod tests {
    include!("dock/tests/core.rs");
    include!("dock/tests/snapshot.rs");
}
