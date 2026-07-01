#![allow(clippy::float_cmp)]

use crate::boundary::{clamp_boundary, next_boundary, previous_boundary};
use crate::fonts::INTER_FONTDB_FAMILY;
use crate::{
    CosmicTextEngine, DEFAULT_FONT_FAMILY, DEFAULT_MONOSPACE_FONT_FAMILY, ShapedTextLayout,
    TextComposition, TextEditState, TextLayoutCache, TextLayoutKey, TextLayoutStore, TextSelection,
    TextStyle, fonts,
};
use cosmic_text::fontdb;
use kinetik_ui_core::{
    Key, KeyEvent, KeyState, Modifiers, TextInputEvent, TextLayoutId, TextRange,
};

#[test]
fn creates_cosmic_text_engine() {
    let mut engine = CosmicTextEngine::new();

    let _ = engine.font_system();
}

#[test]
fn bundled_font_database_sets_default_family_aliases() {
    let mut engine = CosmicTextEngine::new();

    assert_eq!(
        engine
            .font_system
            .db()
            .family_name(&fontdb::Family::SansSerif),
        INTER_FONTDB_FAMILY
    );
    assert_eq!(
        engine
            .font_system
            .db()
            .family_name(&fontdb::Family::Monospace),
        DEFAULT_MONOSPACE_FONT_FAMILY
    );
    assert_eq!(
        query_font_bytes(&mut engine, &[fontdb::Family::SansSerif]),
        fonts::INTER_VARIABLE
    );
    assert_eq!(
        query_font_bytes(&mut engine, &[fontdb::Family::Monospace]),
        fonts::GEIST_MONO_VARIABLE
    );
}

#[test]
fn generic_families_shape_with_bundled_fonts() {
    let mut engine = CosmicTextEngine::new();
    let sans = engine.shape_text(&TextLayoutKey::new(
        "Kinetik",
        TextStyle::new("sans-serif", 13.0, 18.0),
        200.0,
        false,
    ));
    let mono = engine.shape_text(&TextLayoutKey::new(
        "fn main()",
        TextStyle::new("monospace", 13.0, 18.0),
        200.0,
        false,
    ));

    assert!(!sans.runs.is_empty());
    assert!(
        sans.runs
            .iter()
            .all(|run| run.font.data.data() == fonts::INTER_VARIABLE)
    );
    assert!(!mono.runs.is_empty());
    assert!(
        mono.runs
            .iter()
            .all(|run| run.font.data.data() == fonts::GEIST_MONO_VARIABLE)
    );
}

#[test]
fn named_default_families_shape_with_bundled_fonts() {
    let mut engine = CosmicTextEngine::new();
    let sans = engine.shape_text(&TextLayoutKey::new(
        "Default",
        TextStyle::new(DEFAULT_FONT_FAMILY, 12.0, 16.0),
        200.0,
        false,
    ));
    let mono = engine.shape_text(&TextLayoutKey::new(
        "012345",
        TextStyle::new(DEFAULT_MONOSPACE_FONT_FAMILY, 12.0, 16.0),
        200.0,
        false,
    ));

    assert!(!sans.runs.is_empty());
    assert!(
        sans.runs
            .iter()
            .all(|run| run.font.data.data() == fonts::INTER_VARIABLE)
    );
    assert!(!mono.runs.is_empty());
    assert!(
        mono.runs
            .iter()
            .all(|run| run.font.data.data() == fonts::GEIST_MONO_VARIABLE)
    );
}

#[test]
fn cosmic_text_engine_shapes_owned_glyph_runs() {
    let mut engine = CosmicTextEngine::new();
    let key = TextLayoutKey::new(
        "Hello",
        TextStyle::new("sans-serif", 16.0, 22.0),
        200.0,
        false,
    );

    let layout = engine.shape_text(&key);

    assert_eq!(layout.line_count, 1);
    assert!(!layout.is_empty());
    assert!(layout.size.width > 0.0);
    assert!(layout.size.height >= 22.0);
    assert!(layout.runs.iter().all(|run| !run.font.data.is_empty()));
}

#[test]
fn shaped_text_layout_counts_explicit_lines() {
    let mut engine = CosmicTextEngine::new();
    let key = TextLayoutKey::new(
        "one\ntwo",
        TextStyle::new("sans-serif", 14.0, 20.0),
        200.0,
        true,
    );

    let layout = engine.shape_text(&key);

    assert_eq!(layout.line_count, 2);
    assert_eq!(layout.lines.len(), 2);
    assert_eq!(layout.lines[0].text_start, 0);
    assert_eq!(layout.lines[0].text_end, 3);
    assert_eq!(layout.lines[1].text_start, 4);
    assert_eq!(layout.lines[1].text_end, 7);
    assert_eq!(
        layout.glyph_count(),
        layout.runs.iter().map(|run| run.glyphs.len()).sum()
    );
}

#[test]
fn shaped_text_layout_returns_caret_rects_for_byte_offsets() {
    let mut engine = CosmicTextEngine::new();
    let key = TextLayoutKey::new(
        "one\ntwo",
        TextStyle::new("sans-serif", 14.0, 20.0),
        200.0,
        false,
    );
    let layout = engine.shape_text(&key);

    let start = layout.caret_rect(0);
    let after_first = layout.caret_rect(3);
    let second_line = layout.caret_rect(4);

    assert!(after_first.x > start.x);
    assert!(second_line.y > start.y);
    assert_eq!(second_line.x, 0.0);
    assert!(second_line.height >= 20.0);
}

#[test]
fn shaped_text_layout_returns_selection_rects_from_glyph_positions() {
    let mut engine = CosmicTextEngine::new();
    let key = TextLayoutKey::new(
        "one\ntwo",
        TextStyle::new("sans-serif", 14.0, 20.0),
        200.0,
        false,
    );
    let layout = engine.shape_text(&key);

    let rects = layout.selection_rects(1..6);

    assert_eq!(rects.len(), 2);
    assert!(rects[0].width > 0.0);
    assert!(rects[1].width > 0.0);
    assert!(rects[1].y > rects[0].y);
}

#[test]
fn shaped_text_layout_hit_tests_points_to_byte_offsets() {
    let mut engine = CosmicTextEngine::new();
    let key = TextLayoutKey::new(
        "one\ntwo",
        TextStyle::new("sans-serif", 14.0, 20.0),
        200.0,
        false,
    );
    let layout = engine.shape_text(&key);
    let first_end = layout.caret_rect(3);
    let second_line = layout.caret_rect(4);

    assert_eq!(layout.hit_test_point(-10.0, 0.0), 0);
    assert_eq!(layout.hit_test_point(first_end.x + 40.0, 0.0), 3);
    assert_eq!(layout.hit_test_point(0.0, second_line.y), 4);
    assert_eq!(layout.hit_test_point(first_end.x + 40.0, second_line.y), 7);
}

#[test]
fn shaped_text_layout_clamps_geometry_offsets_to_utf8_boundaries() {
    let mut engine = CosmicTextEngine::new();
    let text = "éa";
    let key = TextLayoutKey::new(text, TextStyle::new("sans-serif", 14.0, 20.0), 200.0, false);
    let layout = engine.shape_text(&key);
    let after_first_character = "é".len();

    assert_eq!(layout.caret_rect(1), layout.caret_rect(0));
    assert_eq!(
        layout.selection_rects(1..after_first_character),
        layout.selection_rects(0..after_first_character)
    );

    let first_caret = layout.caret_rect(0);
    let second_caret = layout.caret_rect(after_first_character);
    let hit = layout.hit_test_point(
        (first_caret.x + second_caret.x) * 0.5,
        first_caret.y + first_caret.height * 0.5,
    );

    assert!(text.is_char_boundary(hit));
    assert!(hit == 0 || hit == after_first_character);
}

#[test]
fn shaped_text_layout_reports_empty_layout() {
    let layout = ShapedTextLayout {
        size: kinetik_ui_core::Size::new(0.0, 20.0),
        line_count: 1,
        lines: Vec::new(),
        runs: Vec::new(),
    };

    assert!(layout.is_empty());
    assert_eq!(layout.glyph_count(), 0);
}

#[test]
fn text_layout_store_assigns_stable_cached_ids() {
    let mut store = TextLayoutStore::new();
    let key = TextLayoutKey::new(
        "Label",
        TextStyle::new("sans-serif", 12.0, 16.0),
        100.0,
        false,
    );

    let first = store.layout_id(key.clone());
    let second = store.layout_id(key);

    assert_eq!(first, second);
    assert_eq!(store.len(), 1);
    assert!(!store.layout(first).expect("layout is cached").is_empty());
}

#[test]
fn text_layout_store_assigns_distinct_ids_for_preferred_id_collision() {
    let mut store = TextLayoutStore::new();
    let style = TextStyle::new("sans-serif", 12.0, 16.0);
    let first_key = TextLayoutKey::new("First", style.clone(), 100.0, false);
    let second_key = TextLayoutKey::new("Second", style, 100.0, false);
    let colliding_id = TextLayoutId::from_raw(42);

    let first = store.layout_id_with_preferred_id(first_key.clone(), colliding_id);
    let second = store.layout_id_with_preferred_id(second_key, colliding_id);
    let repeated_first = store.layout_id_with_preferred_id(first_key, colliding_id);

    assert_eq!(first, colliding_id);
    assert_eq!(repeated_first, first);
    assert_ne!(first, second);
    assert_eq!(store.len(), 2);
    assert!(store.layout(first).is_some());
    assert!(store.layout(second).is_some());
}

#[test]
fn text_layout_store_exports_all_entries_after_preferred_id_collision() {
    let mut store = TextLayoutStore::new();
    let style = TextStyle::new("sans-serif", 12.0, 16.0);
    let first_key = TextLayoutKey::new("First", style.clone(), 100.0, false);
    let second_key = TextLayoutKey::new("Second", style, 80.0, false);
    let colliding_id = TextLayoutId::from_raw(7);

    let first = store.layout_id_with_preferred_id(first_key.clone(), colliding_id);
    let second = store.layout_id_with_preferred_id(second_key.clone(), colliding_id);
    let mut entries = store.layouts().collect::<Vec<_>>();
    entries.sort_by_key(|entry| entry.id);

    assert_eq!(entries.len(), 2);
    assert_eq!(
        entries.iter().map(|entry| entry.id).collect::<Vec<_>>(),
        vec![first, second]
    );
    assert!(entries.iter().any(|entry| entry.key == &first_key));
    assert!(entries.iter().any(|entry| entry.key == &second_key));
    assert!(entries.iter().all(|entry| store.layout(entry.id).is_some()));
}

#[test]
fn text_layout_store_reuses_deterministic_ids_after_clear() {
    let key = TextLayoutKey::new(
        "Stable",
        TextStyle::new("sans-serif", 12.0, 16.0),
        100.0,
        false,
    );
    let mut store = TextLayoutStore::new();

    let first = store.layout_id(key.clone());
    store.clear();
    let second = store.layout_id(key);

    assert_eq!(first, second);
    assert_eq!(store.len(), 1);
}

#[test]
fn text_layout_store_exports_cached_layout_entries() {
    let mut store = TextLayoutStore::new();
    let key = TextLayoutKey::new(
        "Label",
        TextStyle::new("sans-serif", 12.0, 16.0),
        100.0,
        false,
    );
    let id = store.layout_id(key.clone());

    let entries = store.layouts().collect::<Vec<_>>();

    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].id, id);
    assert_eq!(entries[0].key, &key);
    assert_eq!(
        entries[0].layout.glyph_count(),
        store.layout(id).unwrap().glyph_count()
    );
    assert!(std::sync::Arc::ptr_eq(
        &entries[0].layout,
        store.layouts.get(&id).expect("cached layout")
    ));
}

#[test]
fn cache_returns_hits_and_can_invalidate() {
    let style = TextStyle::new("Inter", 12.0, 16.0);
    let key = TextLayoutKey::new("hello", style, 100.0, false);
    let mut cache = TextLayoutCache::new();

    let first = cache.get_or_measure(key.clone());
    let second = cache.get_or_measure(key);

    assert_eq!(cache.len(), 1);
    assert_eq!(first, second);
    cache.clear();
    assert!(cache.is_empty());
}

#[test]
fn wrapped_measurement_increases_line_count() {
    let style = TextStyle::new("Inter", 10.0, 14.0);
    let key = TextLayoutKey::new("long text string", style, 10.0, true);
    let mut cache = TextLayoutCache::new();

    let layout = cache.get_or_measure(key);

    assert!(layout.line_count > 1);
}

#[test]
fn measurement_counts_explicit_lines() {
    let style = TextStyle::new("Inter", 10.0, 14.0);
    let key = TextLayoutKey::new("one\ntwo\nthree", style, 200.0, true);
    let mut cache = TextLayoutCache::new();

    let layout = cache.get_or_measure(key);

    assert_eq!(layout.line_count, 3);
}

#[test]
fn inserts_text_at_caret() {
    let mut state = TextEditState::new("ab");
    state.set_caret(1);

    state.insert_text("X");

    assert_eq!(state.text, "aXb");
    assert_eq!(state.caret(), 2);
}

#[test]
fn replaces_selection() {
    let mut state = TextEditState::new("abcd");
    state.selection = TextSelection::new(1, 3);

    state.insert_text("X");

    assert_eq!(state.text, "aXd");
    assert_eq!(state.caret(), 2);
}

#[test]
fn selected_text_and_cut_use_current_selection() {
    let mut state = TextEditState::new("abcd");
    state.set_selection(TextSelection::new(1, 3));

    assert_eq!(state.selected_text(), Some("bc"));
    assert_eq!(state.cut_selection(), Some("bc".to_owned()));

    assert_eq!(state.text, "ad");
    assert_eq!(state.caret(), 1);
    assert!(state.undo());
    assert_eq!(state.text, "abcd");
}

#[test]
fn paste_text_records_local_undo() {
    let mut state = TextEditState::new("ad");
    state.set_caret(1);

    state.paste_text("bc");

    assert_eq!(state.text, "abcd");
    assert!(state.undo());
    assert_eq!(state.text, "ad");
}

#[test]
fn backspace_and_delete_handle_ascii_boundaries_and_edges() {
    let mut backspace = TextEditState::new("abc");
    backspace.set_caret(2);

    backspace.backspace();

    assert_eq!(backspace.text, "ac");
    assert_eq!(backspace.caret(), 1);
    assert!(backspace.undo());
    assert_eq!(backspace.text, "abc");

    let mut at_start = TextEditState::new("abc");
    at_start.set_caret(0);
    at_start.backspace();
    assert_eq!(at_start.text, "abc");
    assert_eq!(at_start.caret(), 0);
    assert!(!at_start.undo());

    let mut delete = TextEditState::new("abc");
    delete.set_caret(1);

    delete.delete_forward();

    assert_eq!(delete.text, "ac");
    assert_eq!(delete.caret(), 1);
    assert!(delete.undo());
    assert_eq!(delete.text, "abc");

    let mut at_end = TextEditState::new("abc");
    at_end.set_caret(3);
    at_end.delete_forward();
    assert_eq!(at_end.text, "abc");
    assert_eq!(at_end.caret(), 3);
    assert!(!at_end.undo());
}

#[test]
fn backspace_and_delete_use_utf8_character_boundaries() {
    let mut backspace = TextEditState::new("aéz");
    backspace.set_caret("aé".len());

    backspace.backspace();

    assert_eq!(backspace.text, "az");
    assert_eq!(backspace.caret(), 1);
    assert!(backspace.text.is_char_boundary(backspace.caret()));

    let mut delete = TextEditState::new("aéz");
    delete.set_caret(1);

    delete.delete_forward();

    assert_eq!(delete.text, "az");
    assert_eq!(delete.caret(), 1);
    assert!(delete.text.is_char_boundary(delete.caret()));
}

#[test]
fn backspace_and_delete_remove_selection_in_either_direction() {
    let mut backspace = TextEditState::new("abcd");
    backspace.set_selection(TextSelection::new(3, 1));

    backspace.backspace();

    assert_eq!(backspace.text, "ad");
    assert_eq!(backspace.caret(), 1);
    assert!(backspace.undo());
    assert_eq!(backspace.text, "abcd");

    let mut delete = TextEditState::new("abcd");
    delete.set_selection(TextSelection::new(1, 3));

    delete.delete_forward();

    assert_eq!(delete.text, "ad");
    assert_eq!(delete.caret(), 1);
    assert!(delete.undo());
    assert_eq!(delete.text, "abcd");
}

#[test]
fn committed_and_pasted_text_replace_selection_with_local_undo() {
    let mut committed = TextEditState::new("abcd");
    committed.set_selection(TextSelection::new(1, 3));

    committed.insert_text("XY");

    assert_eq!(committed.text, "aXYd");
    assert_eq!(committed.caret(), 3);
    assert!(committed.undo());
    assert_eq!(committed.text, "abcd");
    assert_eq!(committed.selection, TextSelection::new(1, 3));

    let mut pasted = TextEditState::new("abcd");
    pasted.set_selection(TextSelection::new(3, 1));

    pasted.paste_text("é");

    assert_eq!(pasted.text, "aéd");
    assert_eq!(pasted.caret(), "aé".len());
    assert!(pasted.undo());
    assert_eq!(pasted.text, "abcd");
    assert_eq!(pasted.selection, TextSelection::new(3, 1));
}

#[test]
fn clamps_public_selection_before_replacing_text() {
    let mut state = TextEditState::new("éa");
    state.selection = TextSelection::new(1, 99);

    state.insert_text("X");

    assert_eq!(state.text, "X");
    assert_eq!(state.caret(), 1);
}

#[test]
fn applies_text_and_key_events() {
    let mut state = TextEditState::new("");

    state.apply_input(&[TextInputEvent::Commit("a".to_owned())], &[]);
    state.apply_input(
        &[],
        &[KeyEvent::new(
            Key::Backspace,
            KeyState::Pressed,
            Modifiers::default(),
            false,
        )],
    );

    assert_eq!(state.text, "");
}

#[test]
fn moves_caret_by_character_boundaries() {
    let mut state = TextEditState::new("aé");

    state.move_left();
    assert_eq!(state.caret(), 1);
    state.move_right();
    assert_eq!(state.caret(), 3);
}

#[test]
fn boundary_helpers_clamp_inside_multibyte_characters() {
    let text = "aé中z";

    assert_eq!(clamp_boundary(text, 2), 1);
    assert_eq!(clamp_boundary(text, 5), 3);
    assert_eq!(previous_boundary(text, 2), Some(1));
    assert_eq!(previous_boundary(text, 5), Some(3));
    assert_eq!(next_boundary(text, 2), Some(3));
    assert_eq!(next_boundary(text, 5), Some("aé中".len()));
    assert_eq!(previous_boundary(text, text.len() + 8), Some("aé中".len()));
    assert_eq!(next_boundary(text, text.len() + 8), None);
}

#[test]
fn movement_collapses_selection_and_supports_home_end() {
    let mut state = TextEditState::new("abcd");
    state.set_selection(TextSelection::new(1, 3));

    state.move_left();
    assert_eq!(state.caret(), 1);

    state.set_selection(TextSelection::new(1, 3));
    state.move_right();
    assert_eq!(state.caret(), 3);

    state.move_home();
    assert_eq!(state.caret(), 0);
    state.move_end();
    assert_eq!(state.caret(), 4);
}

#[test]
fn multiline_vertical_navigation_clamps_at_document_edges() {
    let mut state = TextEditState::new("one\ntwo");
    state.set_caret(1);

    state.move_line_up();
    assert_eq!(state.caret(), 1);

    state.set_caret(5);
    state.move_line_down();
    assert_eq!(state.caret(), 5);
}

#[test]
fn multiline_vertical_navigation_clamps_to_shorter_lines_without_mutating_text() {
    let mut state = TextEditState::new("wide\né\nβeta");
    state.set_caret(3);

    state.move_line_down();

    assert_eq!(state.text, "wide\né\nβeta");
    assert_eq!(state.caret(), "wide\né".len());
    assert!(state.text.is_char_boundary(state.caret()));

    state.move_line_down();
    assert_eq!(state.caret(), "wide\né\nβ".len());
    assert!(state.text.is_char_boundary(state.caret()));
}

#[test]
fn multiline_vertical_navigation_preserves_columns_through_trailing_empty_line() {
    let mut state = TextEditState::new("ab\né\nwide\n");
    state.set_caret(1);

    state.move_line_down();
    assert_eq!(state.caret(), "ab\né".len());
    assert!(state.text.is_char_boundary(state.caret()));

    state.move_line_down();
    assert_eq!(state.caret(), "ab\né\nw".len());
    assert!(state.text.is_char_boundary(state.caret()));

    state.move_line_down();
    assert_eq!(state.caret(), state.text.len());
    assert!(state.text.is_char_boundary(state.caret()));
}

#[test]
fn multiline_shift_vertical_navigation_extends_selection() {
    let mut state = TextEditState::new("ab\ncde\nfg");
    state.set_caret(4);

    state.extend_line_down();

    assert_eq!(state.text, "ab\ncde\nfg");
    assert_eq!(state.selection, TextSelection::new(4, 8));
}

#[test]
fn multiline_home_and_end_target_current_line() {
    let mut state = TextEditState::new("one\ntwé\nthree");
    state.set_caret(5);

    state.move_line_home();
    assert_eq!(state.caret(), 4);

    state.set_caret(5);
    state.move_line_end();
    assert_eq!(state.caret(), "one\ntwé".len());

    state.set_caret(5);
    state.extend_line_home();
    assert_eq!(state.selection, TextSelection::new(5, 4));

    state.set_caret(5);
    state.extend_line_end();
    assert_eq!(state.selection, TextSelection::new(5, "one\ntwé".len()));
}

#[test]
fn multiline_input_uses_explicit_line_navigation_without_changing_text() {
    let mut state = TextEditState::new("alpha\nβ\nomega");
    state.set_caret(3);
    let shift = Modifiers::new(true, false, false, false);

    state.apply_multiline_input(
        &[],
        &[KeyEvent::new(
            Key::ArrowDown,
            KeyState::Pressed,
            shift,
            false,
        )],
    );

    assert_eq!(state.text, "alpha\nβ\nomega");
    assert_eq!(state.selection, TextSelection::new(3, "alpha\nβ".len()));
    assert!(state.text.is_char_boundary(state.selection.active));

    state.apply_multiline_input(
        &[],
        &[KeyEvent::new(
            Key::Home,
            KeyState::Pressed,
            Modifiers::default(),
            false,
        )],
    );
    assert_eq!(state.caret(), "alpha\n".len());
}

#[test]
fn shift_movement_extends_selection_from_existing_anchor() {
    let mut state = TextEditState::new("abcd");
    let shift = Modifiers::new(true, false, false, false);

    state.apply_input(
        &[],
        &[KeyEvent::new(
            Key::ArrowLeft,
            KeyState::Pressed,
            shift,
            false,
        )],
    );
    assert_eq!(state.selection, TextSelection::new(4, 3));

    state.apply_input(
        &[],
        &[KeyEvent::new(
            Key::ArrowLeft,
            KeyState::Pressed,
            shift,
            false,
        )],
    );
    assert_eq!(state.selection, TextSelection::new(4, 2));

    state.apply_input(
        &[],
        &[KeyEvent::new(
            Key::ArrowRight,
            KeyState::Pressed,
            shift,
            false,
        )],
    );
    assert_eq!(state.selection, TextSelection::new(4, 3));
}

#[test]
fn shift_right_at_end_boundary_keeps_selection_at_buffer_end() {
    let mut state = TextEditState::new("aéz");
    let shift = Modifiers::new(true, false, false, false);

    state.apply_input(
        &[],
        &[KeyEvent::new(
            Key::ArrowRight,
            KeyState::Pressed,
            shift,
            false,
        )],
    );

    assert_eq!(state.text, "aéz");
    assert_eq!(state.selection, TextSelection::new(4, 4));
    assert!(state.text.is_char_boundary(state.selection.active));
}

#[test]
fn shift_home_and_end_extend_selection_to_buffer_edges() {
    let mut state = TextEditState::new("abcd");
    let shift = Modifiers::new(true, false, false, false);
    state.set_caret(2);

    state.apply_input(
        &[],
        &[KeyEvent::new(Key::Home, KeyState::Pressed, shift, false)],
    );
    assert_eq!(state.selection, TextSelection::new(2, 0));

    state.apply_input(
        &[],
        &[KeyEvent::new(Key::End, KeyState::Pressed, shift, false)],
    );
    assert_eq!(state.selection, TextSelection::new(2, 4));
}

#[test]
fn shift_movement_clamps_to_utf8_boundaries_and_buffer_edges() {
    let mut state = TextEditState::new("aéz");
    let shift = Modifiers::new(true, false, false, false);

    state.apply_input(
        &[],
        &[KeyEvent::new(
            Key::ArrowLeft,
            KeyState::Pressed,
            shift,
            false,
        )],
    );
    assert_eq!(state.selection, TextSelection::new(4, 3));

    state.apply_input(
        &[],
        &[KeyEvent::new(
            Key::ArrowLeft,
            KeyState::Pressed,
            shift,
            false,
        )],
    );
    assert_eq!(state.selection, TextSelection::new(4, 1));

    state.apply_input(
        &[],
        &[KeyEvent::new(
            Key::ArrowLeft,
            KeyState::Pressed,
            shift,
            false,
        )],
    );
    assert_eq!(state.selection, TextSelection::new(4, 0));

    state.apply_input(
        &[],
        &[KeyEvent::new(
            Key::ArrowLeft,
            KeyState::Pressed,
            shift,
            false,
        )],
    );
    assert_eq!(state.selection, TextSelection::new(4, 0));
    assert!(state.text.is_char_boundary(state.selection.active));
}

#[test]
fn unmodified_movement_collapses_shift_selection_to_expected_endpoint() {
    let mut state = TextEditState::new("abcd");
    let shift = Modifiers::new(true, false, false, false);

    state.apply_input(
        &[],
        &[KeyEvent::new(
            Key::ArrowLeft,
            KeyState::Pressed,
            shift,
            false,
        )],
    );
    state.apply_input(
        &[],
        &[KeyEvent::new(
            Key::ArrowLeft,
            KeyState::Pressed,
            Modifiers::default(),
            false,
        )],
    );
    assert_eq!(state.selection, TextSelection::new(3, 3));

    state.set_selection(TextSelection::new(1, 3));
    state.apply_input(
        &[],
        &[KeyEvent::new(
            Key::ArrowRight,
            KeyState::Pressed,
            Modifiers::default(),
            false,
        )],
    );
    assert_eq!(state.selection, TextSelection::new(3, 3));
}

#[test]
fn tracks_composition_lifecycle_without_committing_preedit() {
    let mut state = TextEditState::new("");

    state.apply_input(
        &[
            TextInputEvent::CompositionStart,
            TextInputEvent::Composition {
                text: "pre".to_owned(),
                selection: Some(TextRange::new(1, 2)),
            },
        ],
        &[],
    );

    assert_eq!(
        state.composition,
        Some(TextComposition::new("pre", Some(TextRange::new(1, 2))))
    );
    assert_eq!(state.text, "");

    state.apply_input(&[TextInputEvent::Commit("done".to_owned())], &[]);
    assert_eq!(state.text, "done");
    assert_eq!(state.composition, None);
}

#[test]
fn composition_selection_clamps_to_preedit_utf8_boundaries() {
    let mut state = TextEditState::new("base");

    state.apply_input(
        &[
            TextInputEvent::Composition {
                text: "éa".to_owned(),
                selection: Some(TextRange::new(1, 99)),
            },
            TextInputEvent::CompositionEnd,
        ],
        &[],
    );

    assert_eq!(state.text, "base");
    assert_eq!(state.composition, None);

    state.apply_input(
        &[TextInputEvent::Composition {
            text: "éa".to_owned(),
            selection: Some(TextRange::new(1, 99)),
        }],
        &[],
    );

    assert_eq!(
        state.composition,
        Some(TextComposition::new(
            "éa",
            Some(TextRange::new(0, "éa".len()))
        ))
    );
}

#[test]
fn keyboard_shortcuts_select_all_undo_and_redo() {
    let modifiers = Modifiers::new(false, true, false, false);
    let mut state = TextEditState::new("abc");

    state.apply_input(
        &[],
        &[KeyEvent::new(
            Key::Character("a".to_owned()),
            KeyState::Pressed,
            modifiers,
            false,
        )],
    );
    assert_eq!(state.selection, TextSelection::new(0, 3));

    state.apply_input(&[TextInputEvent::Commit("X".to_owned())], &[]);
    assert_eq!(state.text, "X");
    state.apply_input(
        &[],
        &[KeyEvent::new(
            Key::Character("z".to_owned()),
            KeyState::Pressed,
            modifiers,
            false,
        )],
    );
    assert_eq!(state.text, "abc");
    state.apply_input(
        &[],
        &[KeyEvent::new(
            Key::Character("y".to_owned()),
            KeyState::Pressed,
            modifiers,
            false,
        )],
    );
    assert_eq!(state.text, "X");
}

#[test]
fn undo_and_redo_are_local_to_text_state() {
    let mut state = TextEditState::new("");

    state.insert_text("a");
    state.insert_text("b");
    assert_eq!(state.text, "ab");

    assert!(state.undo());
    assert_eq!(state.text, "a");
    assert!(state.redo());
    assert_eq!(state.text, "ab");
}

#[test]
fn undo_and_redo_preserve_repeated_selection_replacements() {
    let mut state = TextEditState::new("alpha beta");

    state.set_selection(TextSelection::new(0, 5));
    state.insert_text("one");
    state.set_selection(TextSelection::new(4, 8));
    state.insert_text("two");

    assert_eq!(state.text, "one two");
    assert!(state.undo());
    assert_eq!(state.text, "one beta");
    assert!(state.undo());
    assert_eq!(state.text, "alpha beta");
    assert!(state.redo());
    assert_eq!(state.text, "one beta");
    assert!(state.redo());
    assert_eq!(state.text, "one two");
    assert!(!state.redo());
}

fn query_font_bytes<'a>(
    engine: &mut CosmicTextEngine,
    families: &'a [fontdb::Family<'a>],
) -> Vec<u8> {
    let id = engine
        .font_system
        .db()
        .query(&fontdb::Query {
            families,
            ..fontdb::Query::default()
        })
        .expect("font query resolves");
    let weight = engine.font_system.db().face(id).expect("font face").weight;
    let font = engine
        .font_system
        .get_font(id, weight)
        .expect("font object");
    font.data().to_vec()
}
