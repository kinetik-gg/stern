//! Drift pins for the vendored Phosphor glyph copies.
//!
//! `components/vendored_icons.rs` duplicates three generated definitions from
//! `stern-icons-phosphor` (a dev-dependency here) so the widget crate does not
//! link the complete icon set. These tests fail if either side changes.

use crate::components::vendored_icons::{CARET_DOWN_ICON, CARET_UP_ICON, CHECK_ICON};

#[test]
fn vendored_caret_down_matches_phosphor_bold_definition() {
    let upstream = stern_icons_phosphor::bold::CARET_DOWN.icon();
    assert_eq!(CARET_DOWN_ICON.id(), upstream.id());
    assert_eq!(*CARET_DOWN_ICON.graphic(), *upstream.graphic());
}

#[test]
fn vendored_caret_up_matches_phosphor_bold_definition() {
    let upstream = stern_icons_phosphor::bold::CARET_UP.icon();
    assert_eq!(CARET_UP_ICON.id(), upstream.id());
    assert_eq!(*CARET_UP_ICON.graphic(), *upstream.graphic());
}

#[test]
fn vendored_check_matches_phosphor_bold_definition() {
    let upstream = stern_icons_phosphor::bold::CHECK.icon();
    assert_eq!(CHECK_ICON.id(), upstream.id());
    assert_eq!(*CHECK_ICON.graphic(), *upstream.graphic());
}
