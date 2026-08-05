//! Story registry types and the sizes × scales matrix.

use stern::core::{Rect, Size};
use stern::widgets::Ui;

/// Story scope tier, which selects the sizes × scales matrix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoryKind {
    /// Single component or a component state sheet.
    Component,
    /// Assembly of components (the rung where seam/clipping defects live).
    Composition,
    /// Full-window workspace scene.
    Workspace,
}

/// One renderable variant of a story: a logical size at a scale factor.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StoryVariant {
    /// Logical (pre-scale) size of the story window.
    pub logical: Size,
    /// Device scale factor applied when rasterizing.
    pub scale: f32,
}

impl StoryVariant {
    /// Creates a variant.
    #[must_use]
    pub const fn new(width: f32, height: f32, scale: f32) -> Self {
        Self {
            logical: Size::new(width, height),
            scale,
        }
    }

    /// Deterministic file-name stem for this variant of `story_id`.
    #[must_use]
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    pub fn file_stem(&self, story_id: &str) -> String {
        format!(
            "{}_{}x{}@{:.2}",
            story_id.replace('/', "-"),
            self.logical.width as u32,
            self.logical.height as u32,
            self.scale
        )
    }
}

/// A declared story: a deterministic composed frame next to the widgets it
/// exercises.
pub struct Story {
    /// Stable kebab-case identity, `family/name`.
    pub id: &'static str,
    /// Human-readable title shown in the browser and contact sheet.
    pub title: &'static str,
    /// Scope tier selecting the size/scale matrix.
    pub kind: StoryKind,
    /// Composes the story into `ui` within `rect` (logical units).
    pub compose: fn(&mut Ui<'_>, Rect),
}

impl Story {
    /// Returns the sizes × scales matrix for this story.
    ///
    /// Component and composition stories render at a component-sheet size at
    /// 1.0 and 2.0 scale. Workspace stories render full-window at three
    /// sizes including a 4K-logical window, and at 1.0 and 2.0 scale for the
    /// base size.
    #[must_use]
    pub fn variants(&self) -> Vec<StoryVariant> {
        match self.kind {
            StoryKind::Component | StoryKind::Composition => vec![
                StoryVariant::new(640.0, 480.0, 1.0),
                StoryVariant::new(640.0, 480.0, 2.0),
            ],
            StoryKind::Workspace => vec![
                StoryVariant::new(960.0, 640.0, 1.0),
                StoryVariant::new(960.0, 640.0, 2.0),
                StoryVariant::new(1280.0, 800.0, 1.0),
                StoryVariant::new(3840.0, 2160.0, 1.0),
            ],
        }
    }
}

impl std::fmt::Debug for Story {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Story")
            .field("id", &self.id)
            .field("title", &self.title)
            .field("kind", &self.kind)
            .finish_non_exhaustive()
    }
}

/// Returns every registered story in deterministic order.
#[must_use]
pub fn registry() -> Vec<Story> {
    crate::stories::all()
}

/// Reports whether a story id matches a `--filter` argument.
///
/// Matching is case-insensitive substring containment. An empty filter
/// matches everything.
#[must_use]
pub fn story_matches_filter(story_id: &str, filter: &str) -> bool {
    if filter.is_empty() {
        return true;
    }
    story_id
        .to_ascii_lowercase()
        .contains(&filter.to_ascii_lowercase())
}
