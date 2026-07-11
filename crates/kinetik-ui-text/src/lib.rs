//! Text layout, editing state, and engine adapters for Kinetik UI.

mod boundary;
mod cache;
mod edit;
mod engine;
/// Bundled font assets used by the default text engine.
pub mod fonts;
mod layout;
mod navigation;
mod selection;
mod store;
mod style;
mod undo;
mod viewport;

pub use cache::TextLayoutCache;
pub use edit::{OrderedTextInputResult, TextEditMode, TextEditState};
pub use engine::CosmicTextEngine;
pub use layout::{ShapedGlyph, ShapedGlyphRun, ShapedTextLayout, ShapedTextLine, TextLayout};
pub use navigation::{
    SHAPED_TEXT_GEOMETRY_EPSILON, ShapedCaretStop, ShapedTextNavigation, TextNavigationError,
    TextNavigationOutcome,
};
pub use selection::{TextAffinity, TextCaret, TextComposition, TextSelection};
pub use store::{StoredTextLayout, TextLayoutStore};
pub use style::{TextLayoutKey, TextStyle};
pub use undo::TextUndoStack;
pub use viewport::{TextViewport, TextViewportMode};

pub(crate) use undo::EditSnapshot;

/// Bundled default UI font family.
pub const DEFAULT_FONT_FAMILY: &str = "Inter";
/// Bundled default monospace font family.
pub const DEFAULT_MONOSPACE_FONT_FAMILY: &str = "Geist Mono";

#[cfg(test)]
mod tests;
