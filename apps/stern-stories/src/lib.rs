//! Story harness for Stern.
//!
//! A story is a deterministic composed frame: `Story { id, title, compose }`
//! plus a sizes × scales matrix. The harness renders every story variant
//! headlessly through the CPU raster path (no GPU, no window, bundled fonts),
//! writes PNGs plus a contact sheet and manifest, compares renders against
//! human-blessed goldens, and hosts an interactive browser built on
//! `stern-app`.
//!
//! This is the structural prevention for the AUDIT #941 failure class: model
//! tests validate values, stories validate pixels. Blessing goldens is a
//! human act — nothing in this crate ever blesses automatically.

pub mod contact_sheet;
pub mod diff;
pub mod frame;
pub mod manifest;
pub mod raster;
pub mod stories;
pub mod story;

pub use story::{Story, StoryKind, StoryVariant, registry, story_matches_filter};
