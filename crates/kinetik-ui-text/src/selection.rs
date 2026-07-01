use kinetik_ui_core::TextRange;

use crate::boundary::{clamp_boundary, clamp_text_range};

/// Selection range in byte offsets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextSelection {
    /// Anchor byte offset.
    pub anchor: usize,
    /// Active byte offset.
    pub active: usize,
}

impl TextSelection {
    /// Creates a selection.
    #[must_use]
    pub const fn new(anchor: usize, active: usize) -> Self {
        Self { anchor, active }
    }

    /// Returns the sorted selection range.
    #[must_use]
    pub fn range(self) -> core::ops::Range<usize> {
        self.anchor.min(self.active)..self.anchor.max(self.active)
    }

    /// Returns this selection clamped to UTF-8 boundaries in text.
    #[must_use]
    pub fn clamp_to_text(self, text: &str) -> Self {
        Self {
            anchor: clamp_boundary(text, self.anchor),
            active: clamp_boundary(text, self.active),
        }
    }

    /// Returns the sorted selection range clamped to UTF-8 boundaries in text.
    #[must_use]
    pub fn range_in(self, text: &str) -> core::ops::Range<usize> {
        self.clamp_to_text(text).range()
    }

    /// Returns true when the selection is collapsed.
    #[must_use]
    pub const fn is_caret(self) -> bool {
        self.anchor == self.active
    }
}

/// Active IME/preedit composition state for a text field.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TextComposition {
    /// Current preedit text.
    pub text: String,
    /// Optional selected byte range inside the preedit text.
    pub selection: Option<TextRange>,
}

impl TextComposition {
    /// Creates a composition snapshot.
    #[must_use]
    pub fn new(text: impl Into<String>, selection: Option<TextRange>) -> Self {
        let text = text.into();
        Self {
            selection: selection.map(|selection| clamp_text_range(&text, selection)),
            text,
        }
    }
}
