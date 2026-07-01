use std::collections::HashMap;

use kinetik_ui_core::Size;

use crate::{TextLayout, TextLayoutKey};

/// Text layout cache.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TextLayoutCache {
    layouts: HashMap<TextLayoutKey, TextLayout>,
}

impl TextLayoutCache {
    /// Creates an empty text layout cache.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns a cached layout.
    #[must_use]
    pub fn get(&self, key: &TextLayoutKey) -> Option<TextLayout> {
        self.layouts.get(key).copied()
    }

    /// Inserts a cached layout.
    pub fn insert(&mut self, key: TextLayoutKey, layout: TextLayout) {
        self.layouts.insert(key, layout);
    }

    /// Returns an existing layout or inserts a newly measured layout.
    pub fn get_or_measure(&mut self, key: TextLayoutKey) -> TextLayout {
        if let Some(layout) = self.get(&key) {
            layout
        } else {
            let layout = fallback_measure(&key);
            self.insert(key, layout);
            layout
        }
    }

    /// Clears all cached layouts.
    pub fn clear(&mut self) {
        self.layouts.clear();
    }

    /// Returns the number of cached entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.layouts.len()
    }

    /// Returns true when the cache is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.layouts.is_empty()
    }
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]
fn fallback_measure(key: &TextLayoutKey) -> TextLayout {
    let line_height = key.style.line_height();
    let char_width = key.style.size() * 0.55;
    let wrap_width = key.width().max(0.0);
    let mut line_count = 0;
    let mut measured_width = 0.0_f32;

    for line in key.text.split('\n') {
        let raw_width = line.chars().count() as f32 * char_width;
        if key.wrap && wrap_width > 0.0 && raw_width > wrap_width {
            let wrapped_lines = (raw_width / wrap_width).ceil() as usize;
            line_count += wrapped_lines;
            measured_width = measured_width.max(wrap_width);
        } else {
            line_count += 1;
            measured_width = measured_width.max(raw_width);
        }
    }

    let line_count = line_count.max(1);
    let width = if key.wrap {
        measured_width.min(wrap_width).max(0.0)
    } else {
        measured_width
    };

    TextLayout {
        size: Size::new(width, line_height * line_count as f32),
        line_count,
    }
}
