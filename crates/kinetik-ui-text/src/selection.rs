use kinetik_ui_core::TextRange;

use crate::boundary::{clamp_boundary, clamp_text_range};

/// Logical association used when one byte boundary has multiple visual carets.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TextAffinity {
    /// Associate the boundary with the cluster or run immediately before it.
    Before,
    /// Associate the boundary with the cluster or run beginning at it.
    #[default]
    After,
}

/// A grapheme-safe caret byte offset and its visual-run association.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TextCaret {
    /// UTF-8 byte offset in the logical text buffer.
    pub offset: usize,
    /// Association used at shaped run and line seams.
    pub affinity: TextAffinity,
}

impl TextCaret {
    /// Creates a caret position without applying text-specific clamping.
    #[must_use]
    pub const fn new(offset: usize, affinity: TextAffinity) -> Self {
        Self { offset, affinity }
    }

    /// Creates a byte-only compatibility caret with [`TextAffinity::After`].
    #[must_use]
    pub const fn at(offset: usize) -> Self {
        Self::new(offset, TextAffinity::After)
    }
}

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

    /// Returns this selection clamped backward to extended-grapheme boundaries.
    #[must_use]
    pub fn clamp_to_text(self, text: &str) -> Self {
        Self {
            anchor: clamp_boundary(text, self.anchor),
            active: clamp_boundary(text, self.active),
        }
    }

    /// Returns the sorted range clamped to extended-grapheme boundaries.
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
