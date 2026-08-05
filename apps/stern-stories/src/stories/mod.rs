//! Seeded stories for existing widgets.
//!
//! These are rect-based stories: every control paints the caller-supplied
//! rectangle because the layout engine (#931/#942) does not exist yet. The
//! full-width controls they show are the honest baseline recorded by AUDIT
//! #941 — render them anyway; they become before/after evidence as layout
//! lands.

mod basic_controls;
mod dock;
mod fields;
mod inspector;

use crate::story::Story;

/// Returns every seeded story in deterministic registry order.
#[must_use]
pub fn all() -> Vec<Story> {
    vec![
        basic_controls::sheet(),
        fields::sheet(),
        inspector::with_rows(),
        dock::with_seams(),
    ]
}
