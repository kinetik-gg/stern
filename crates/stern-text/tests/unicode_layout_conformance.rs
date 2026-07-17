//! Public conformance for source-bound shaped Unicode navigation.

use stern_core::{Size, TextRange};
use stern_text::{
    CosmicTextEngine, SHAPED_TEXT_GEOMETRY_EPSILON, ShapedGlyph, ShapedGlyphRun, ShapedTextLayout,
    ShapedTextLine, ShapedTextNavigation, TextAffinity, TextCaret, TextComposition, TextEditState,
    TextLayoutKey, TextNavigationError, TextNavigationOutcome, TextOverflow, TextSelection,
    TextStyle,
};
use unicode_segmentation::UnicodeSegmentation;

type Operation = fn(&mut TextEditState, &ShapedTextNavigation) -> TextNavigationOutcome;

fn shape(text: &str, width: f32, wrap: bool) -> ShapedTextLayout {
    let mut engine = CosmicTextEngine::new();
    engine.shape_text(&TextLayoutKey::new(
        text,
        TextStyle::new("Inter", 18.0, 24.0),
        width,
        wrap,
    ))
}

fn caret(offset: usize, affinity: TextAffinity) -> TextCaret {
    TextCaret::new(offset, affinity)
}

fn assert_near(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() <= SHAPED_TEXT_GEOMETRY_EPSILON,
        "expected {expected}, got {actual}"
    );
}

#[derive(Clone, Copy)]
struct LineSpec {
    source_line: usize,
    start: usize,
    end: usize,
    rtl: bool,
}

#[derive(Clone, Copy)]
struct CellSpec {
    line: usize,
    start: usize,
    end: usize,
    left: f32,
    right: f32,
    rtl: bool,
}

const fn line(source_line: usize, start: usize, end: usize, rtl: bool) -> LineSpec {
    LineSpec {
        source_line,
        start,
        end,
        rtl,
    }
}

const fn cell(line: usize, start: usize, end: usize, left: f32, right: f32, rtl: bool) -> CellSpec {
    CellSpec {
        line,
        start,
        end,
        left,
        right,
        rtl,
    }
}

fn visual_top(visual_line: usize) -> f32 {
    f32::from(u16::try_from(visual_line).expect("synthetic visual-line index fits u16")) * 24.0
}

fn synthetic_layout(lines: &[LineSpec], cells: &[CellSpec]) -> ShapedTextLayout {
    let template = shape("a", 400.0, false);
    let font = template.runs[0].font.clone();
    let glyph = template.runs[0].glyphs[0];
    let shaped_lines = lines
        .iter()
        .enumerate()
        .map(|(visual_index, spec)| {
            let top_y = visual_top(visual_index);
            let width = cells
                .iter()
                .filter(|cell| cell.line == visual_index)
                .map(|cell| cell.right)
                .max_by(f32::total_cmp)
                .unwrap_or(0.0);
            ShapedTextLine {
                visual_index,
                source_line_index: spec.source_line,
                text_start: spec.start,
                text_end: spec.end,
                top_y,
                baseline_y: top_y + 18.0,
                height: 24.0,
                width,
                rtl: spec.rtl,
            }
        })
        .collect::<Vec<_>>();
    let runs = lines
        .iter()
        .enumerate()
        .filter_map(|(visual_line, spec)| {
            let glyphs = cells
                .iter()
                .filter(|cell| cell.line == visual_line)
                .map(|cell| ShapedGlyph {
                    id: glyph.id,
                    x: cell.left,
                    y: visual_top(visual_line) + 18.0,
                    start: cell.start,
                    end: cell.end,
                    width: cell.right - cell.left,
                    rtl: cell.rtl,
                    elided: false,
                })
                .collect::<Vec<_>>();
            (!glyphs.is_empty()).then(|| ShapedGlyphRun {
                font: font.clone(),
                normalized_coords: Vec::new(),
                font_size: 18.0,
                line_index: spec.source_line,
                visual_line,
                line_y: visual_top(visual_line) + 18.0,
                glyphs,
            })
        })
        .collect::<Vec<_>>();
    let width = shaped_lines
        .iter()
        .map(|line| line.width)
        .max_by(f32::total_cmp)
        .unwrap_or(0.0);

    ShapedTextLayout {
        size: Size::new(width, visual_top(lines.len())),
        line_count: lines.len(),
        lines: shaped_lines,
        runs,
    }
}

fn one_cluster_layout(source: &str, width: f32) -> ShapedTextLayout {
    let mut layout = shape(source, 400.0, false);
    layout.lines.truncate(1);
    layout.line_count = 1;
    layout.lines[0].visual_index = 0;
    layout.lines[0].source_line_index = 0;
    layout.lines[0].text_start = 0;
    layout.lines[0].text_end = source.len();
    layout.lines[0].width = width;
    layout.size.width = width;
    layout.runs.truncate(1);
    layout.runs[0].visual_line = 0;
    layout.runs[0].line_index = 0;
    let template = layout.runs[0].glyphs[0];
    layout.runs[0].glyphs = vec![ShapedGlyph {
        x: 0.0,
        start: 0,
        end: source.len(),
        width,
        rtl: false,
        ..template
    }];
    layout
}

fn two_cluster_layout(gap: f32) -> ShapedTextLayout {
    let mut layout = shape("ab", 400.0, false);
    let first = layout.runs[0].glyphs[0];
    let second = layout.runs[0].glyphs[1];
    layout.runs[0].glyphs = vec![
        ShapedGlyph {
            x: 0.0,
            start: 0,
            end: 1,
            width: 1.0,
            rtl: false,
            ..first
        },
        ShapedGlyph {
            x: 1.0 + gap,
            start: 1,
            end: 2,
            width: 1.0,
            rtl: false,
            ..second
        },
    ];
    layout.lines[0].width = 2.0 + gap;
    layout.size.width = 2.0 + gap;
    layout
}

#[test]
fn emoji_combining_and_zwj_clusters_have_only_egc_stops() {
    for text in ["Ae\u{301}B", "👍🏽", "🇮🇩", "A👩‍🚀B"] {
        let layout = shape(text, 400.0, false);
        let navigation = layout.navigation(text).expect("valid shaped navigation");
        let expected = text
            .grapheme_indices(true)
            .map(|(offset, _)| offset)
            .chain(std::iter::once(text.len()))
            .collect::<Vec<_>>();
        let actual = navigation
            .caret_stops()
            .iter()
            .map(|stop| stop.caret.offset)
            .collect::<Vec<_>>();

        assert_eq!(actual, expected, "unexpected stops for {text:?}");
        for stop in navigation.caret_stops() {
            let rect = navigation.caret_rect(stop.caret);
            assert!(rect.x.is_finite());
            assert!(rect.y.is_finite());
            assert!(rect.height.is_finite());
            assert_eq!(
                navigation.hit_test_caret(rect.x, rect.y + rect.height * 0.5),
                stop.caret
            );
        }

        let first = navigation.caret_stops()[0];
        let second = navigation.caret_stops()[1];
        assert_eq!(
            navigation.hit_test_caret(
                (first.x + second.x) * 0.5,
                navigation.caret_rect(first.caret).y
            ),
            first.caret,
            "midpoint ties choose the smaller x"
        );
        assert_eq!(
            navigation.hit_test_caret(f32::NAN, 0.0),
            navigation.caret_stops()[0].caret
        );
        assert_eq!(
            navigation.hit_test_caret(0.0, f32::INFINITY),
            navigation.caret_stops()[0].caret
        );
    }
}

#[test]
fn end_ellipsis_seams_are_empty_extended_grapheme_boundaries() {
    let cases = [
        ("A long left to right label that overflows", false),
        ("مرحبا بالعالم مرحبا بالعالم مرحبا بالعالم", true),
        (
            "Cafe\u{301} Cafe\u{301} Cafe\u{301} Cafe\u{301} Cafe\u{301}",
            false,
        ),
        ("👩‍🚀👩‍🚀👩‍🚀👩‍🚀👩‍🚀👩‍🚀", false),
    ];

    for (source, expected_rtl) in cases {
        let mut engine = CosmicTextEngine::new();
        let layout = engine.shape_text(
            &TextLayoutKey::new(source, TextStyle::new("Inter", 18.0, 24.0), 84.0, false)
                .with_overflow(TextOverflow::EndEllipsis),
        );
        let markers = layout
            .runs
            .iter()
            .flat_map(|run| &run.glyphs)
            .filter(|glyph| glyph.elided)
            .collect::<Vec<_>>();
        let boundaries = source
            .grapheme_indices(true)
            .map(|(offset, _)| offset)
            .chain(std::iter::once(source.len()))
            .collect::<Vec<_>>();

        assert!(layout.is_elided(), "expected elision for {source:?}");
        assert_eq!(markers.len(), 1, "unexpected markers for {source:?}");
        assert_eq!(markers[0].start, markers[0].end);
        assert!(boundaries.contains(&markers[0].start));
        assert_eq!(layout.lines[0].rtl, expected_rtl);
    }
}

#[test]
fn elided_navigation_fails_before_cluster_validation() {
    let source = "Navigation must not interpolate through hidden source graphemes 👩‍🚀";
    let mut engine = CosmicTextEngine::new();
    let elided = engine.shape_text(
        &TextLayoutKey::new(source, TextStyle::new("Inter", 18.0, 24.0), 96.0, false)
            .with_overflow(TextOverflow::EndEllipsis),
    );
    assert!(elided.is_elided());
    assert_eq!(
        elided.navigation(source),
        Err(TextNavigationError::ElidedLayout)
    );
    let mut malformed_elided = elided.clone();
    malformed_elided.lines.clear();
    assert_ne!(malformed_elided.line_count, malformed_elided.lines.len());
    assert_eq!(
        malformed_elided
            .runs
            .iter()
            .flat_map(|run| &run.glyphs)
            .filter(|glyph| glyph.elided)
            .count(),
        1
    );
    assert_eq!(
        malformed_elided.navigation(source),
        Err(TextNavigationError::ElidedLayout)
    );

    let visible_source = "e\u{301}👩‍🚀";
    let visible = engine.shape_text(&TextLayoutKey::new(
        visible_source,
        TextStyle::new("Inter", 18.0, 24.0),
        400.0,
        false,
    ));
    let navigation = visible
        .navigation(visible_source)
        .expect("full-fit layout remains navigable");
    let offsets = navigation
        .caret_stops()
        .iter()
        .map(|stop| stop.caret.offset)
        .collect::<Vec<_>>();
    assert_eq!(offsets, vec![0, "e\u{301}".len(), visible_source.len()]);
}

#[test]
fn real_and_synthetic_multi_grapheme_clusters_use_grapheme_counts() {
    let ligature = shape("->", 400.0, false);
    assert!(
        ligature
            .runs
            .iter()
            .flat_map(|run| &run.glyphs)
            .any(|glyph| glyph.start == 0 && glyph.end == 2),
        "pinned bundled Inter/cosmic-text must expose the 0..2 witness"
    );
    let navigation = ligature.navigation("->").expect("ligature map");
    assert_eq!(
        navigation
            .caret_stops()
            .iter()
            .map(|stop| stop.caret.offset)
            .collect::<Vec<_>>(),
        vec![0, 1, 2]
    );
    assert_near(
        navigation.caret_stops()[1].x,
        (navigation.caret_stops()[0].x + navigation.caret_stops()[2].x) * 0.5,
    );
    let first = caret(0, TextAffinity::After);
    let middle_before = caret(1, TextAffinity::Before);
    let middle_after = caret(1, TextAffinity::After);
    let end = caret(2, TextAffinity::Before);
    assert_eq!(navigation.visual_right(first), middle_before);
    assert_eq!(navigation.visual_right(middle_before), end);
    assert_eq!(navigation.visual_left(end), middle_after);
    assert_eq!(navigation.visual_left(middle_after), first);

    let synthetic = one_cluster_layout("éa", 12.0);
    let navigation = synthetic.navigation("éa").expect("synthetic map");
    assert_eq!(
        navigation
            .caret_stops()
            .iter()
            .map(|stop| (stop.caret.offset, stop.x))
            .collect::<Vec<_>>(),
        vec![(0, 0.0), (2, 6.0), (3, 12.0)]
    );
    assert!((navigation.caret_stops()[1].x - 8.0).abs() > SHAPED_TEXT_GEOMETRY_EPSILON);
    assert_eq!(
        navigation.visual_right(caret(0, TextAffinity::After)),
        caret(2, TextAffinity::Before)
    );
    assert_eq!(
        navigation.visual_right(caret(2, TextAffinity::Before)),
        caret(3, TextAffinity::Before)
    );
    assert_eq!(
        navigation.visual_left(caret(3, TextAffinity::Before)),
        caret(2, TextAffinity::After)
    );
    assert_eq!(
        navigation.visual_left(caret(2, TextAffinity::After)),
        caret(0, TextAffinity::After)
    );

    let office = shape("office", 400.0, false);
    assert!(
        office
            .runs
            .iter()
            .flat_map(|run| &run.glyphs)
            .all(|glyph| glyph.end - glyph.start == 1)
    );
    let navigation = office.navigation("office").expect("ordinary seam map");
    assert_eq!(navigation.caret_stops().len(), 7);
    assert_eq!(
        navigation.visual_right(caret(1, TextAffinity::Before)),
        caret(2, TextAffinity::Before),
        "switching an alias must not consume a zero-distance step"
    );
}

#[test]
fn public_seam_threshold_matches_bounded_epsilon_grouping() {
    let within_gap = SHAPED_TEXT_GEOMETRY_EPSILON * 0.75;
    let within = two_cluster_layout(within_gap)
        .navigation("ab")
        .expect("within-epsilon map");
    assert_eq!(within.caret_stops().len(), 3);
    let seam = within
        .caret_stops()
        .iter()
        .find(|stop| stop.caret.offset == 1)
        .expect("coalesced public seam");
    assert_near(seam.x, 1.0);

    let outside_gap = SHAPED_TEXT_GEOMETRY_EPSILON * 1.5;
    let outside = two_cluster_layout(outside_gap)
        .navigation("ab")
        .expect("outside-epsilon map");
    let seam_stops = outside
        .caret_stops()
        .iter()
        .filter(|stop| stop.caret.offset == 1)
        .collect::<Vec<_>>();
    assert_eq!(outside.caret_stops().len(), 4);
    assert_eq!(seam_stops.len(), 2);
    assert_near(seam_stops[0].x, 1.0);
    assert_near(seam_stops[1].x, 1.0 + outside_gap);
}

#[test]
fn pure_rtl_stops_adjacent_motion_and_word_motion_are_exact() {
    let navigation = shape("אבג", 400.0, false)
        .navigation("אבג")
        .expect("rtl map");
    assert_eq!(
        navigation
            .caret_stops()
            .iter()
            .map(|stop| stop.caret)
            .collect::<Vec<_>>(),
        vec![
            caret(6, TextAffinity::Before),
            caret(4, TextAffinity::After),
            caret(2, TextAffinity::After),
            caret(0, TextAffinity::After),
        ]
    );

    let mut left = caret(0, TextAffinity::After);
    let mut left_sequence = vec![left];
    for _ in 0..3 {
        left = navigation.visual_left(left);
        left_sequence.push(left);
    }
    assert_eq!(
        left_sequence,
        vec![
            caret(0, TextAffinity::After),
            caret(2, TextAffinity::After),
            caret(4, TextAffinity::After),
            caret(6, TextAffinity::Before),
        ]
    );
    assert_eq!(navigation.visual_left(left), left);

    let mut right = caret(6, TextAffinity::Before);
    let mut right_sequence = vec![right];
    for _ in 0..3 {
        right = navigation.visual_right(right);
        right_sequence.push(right);
    }
    assert_eq!(
        right_sequence,
        vec![
            caret(6, TextAffinity::Before),
            caret(4, TextAffinity::Before),
            caret(2, TextAffinity::Before),
            caret(0, TextAffinity::After),
        ]
    );
    assert_eq!(navigation.visual_right(right), right);
    assert_eq!(
        navigation.visual_word_left(caret(0, TextAffinity::After)),
        caret(6, TextAffinity::Before)
    );
    assert_eq!(
        navigation.visual_word_right(caret(6, TextAffinity::Before)),
        caret(0, TextAffinity::After)
    );
}

#[test]
fn mixed_bidi_uses_physical_order_hit_ties_and_disjoint_selection_spans() {
    let text = "abc אבג def";
    let navigation = shape(text, 400.0, false)
        .navigation(text)
        .expect("mixed map");
    let offsets = navigation
        .caret_stops()
        .iter()
        .map(|stop| stop.caret.offset)
        .collect::<Vec<_>>();
    assert!(offsets.windows(3).any(|window| window == [10, 8, 6]));

    let spans = navigation.selection_rects(3..6);
    assert_eq!(spans.len(), 2);
    assert!(spans[0].x + spans[0].width < spans[1].x);

    let same_x = navigation
        .caret_stops()
        .windows(2)
        .find(|pair| (pair[0].x - pair[1].x).abs() <= SHAPED_TEXT_GEOMETRY_EPSILON)
        .expect("bidi seam exposes two offsets at one x");
    let rect = navigation.caret_rect(same_x[0].caret);
    assert_eq!(
        navigation.hit_test_caret(same_x[0].x, rect.y + rect.height * 0.5),
        same_x[0].caret,
        "same-x hit ties choose the lower offset"
    );

    let start = caret(8, TextAffinity::After);
    let right = navigation.visual_right(start);
    assert_eq!(right, caret(6, TextAffinity::Before));
    assert_eq!(navigation.visual_left(right), start);

    let mut shifted = TextEditState::new(text);
    shifted.set_caret_position(start);
    shifted.extend_visual_right(&navigation);
    assert_eq!(shifted.selection, TextSelection::new(8, 6));
    assert_eq!(shifted.caret_position().affinity, TextAffinity::Before);
    shifted.extend_visual_left(&navigation);
    assert_eq!(shifted.selection, TextSelection::new(8, 8));
    assert_eq!(shifted.caret_position().affinity, TextAffinity::After);
}

#[test]
fn wrap_seam_affinity_selects_lines_and_cross_line_motion_is_reversible() {
    let text = "abc אבג";
    let layout = shape(text, 40.0, true);
    assert!(layout.lines.len() >= 2);
    let (before, after) = layout
        .lines
        .windows(2)
        .find_map(|pair| (pair[0].text_end == pair[1].text_start).then_some((pair[0], pair[1])))
        .expect("fixture has a wrapped logical seam");
    let seam = before.text_end;
    let navigation = layout.navigation(text).expect("wrapped map");
    let before_rect = navigation.caret_rect(caret(seam, TextAffinity::Before));
    let after_rect = navigation.caret_rect(caret(seam, TextAffinity::After));
    assert!((before_rect.y - after_rect.y).abs() > SHAPED_TEXT_GEOMETRY_EPSILON);
    assert_near(before_rect.y, before.top_y);
    assert_near(after_rect.y, after.top_y);

    let last_on_first = *navigation
        .caret_stops()
        .iter()
        .rev()
        .find(|stop| stop.visual_line == before.visual_index)
        .expect("first line stop");
    let first_on_second = navigation.visual_right(last_on_first.caret);
    assert_near(navigation.caret_rect(first_on_second).y, after.top_y);
    assert_eq!(navigation.visual_left(first_on_second), last_on_first.caret);

    let mut from_after = TextEditState::new(text);
    from_after.set_caret_position(caret(seam, TextAffinity::After));
    from_after.extend_visual_left(&navigation);
    assert_eq!(from_after.selection.anchor, seam);
    assert_eq!(from_after.caret_position().affinity, TextAffinity::After);
    from_after.extend_visual_right(&navigation);
    assert_eq!(from_after.selection, TextSelection::new(seam, seam));
    assert_eq!(from_after.caret_position().affinity, TextAffinity::After);

    let mut from_before = TextEditState::new(text);
    from_before.set_caret_position(caret(seam, TextAffinity::Before));
    from_before.extend_visual_right(&navigation);
    assert_eq!(from_before.selection.anchor, seam);
    assert_eq!(from_before.caret_position().affinity, TextAffinity::Before);
    from_before.extend_visual_left(&navigation);
    assert_eq!(from_before.selection, TextSelection::new(seam, seam));
    assert_eq!(from_before.caret_position().affinity, TextAffinity::Before);

    let multiline = "abc אבג\nA👩‍🚀B";
    let multiline_layout = shape(multiline, 60.0, true);
    let multiline_navigation = multiline_layout
        .navigation(multiline)
        .expect("multiline Unicode map");
    assert!(
        multiline_navigation
            .caret_stops()
            .iter()
            .all(|stop| multiline.is_char_boundary(stop.caret.offset))
    );
}

#[test]
fn empty_visual_lines_have_both_aliases_and_real_line_geometry() {
    let text = "\nA\n";
    let layout = shape(text, 400.0, false);
    assert_eq!(layout.lines.len(), 3);
    let navigation = layout.navigation(text).expect("empty line map");
    assert_eq!(
        navigation.caret_stops()[0].caret,
        caret(0, TextAffinity::After)
    );
    assert_eq!(
        navigation
            .caret_stops()
            .last()
            .expect("trailing stop")
            .caret,
        caret(text.len(), TextAffinity::Before)
    );
    assert_eq!(
        navigation.visual_left(caret(0, TextAffinity::Before)),
        caret(0, TextAffinity::Before),
        "exact leading Before alias survives an outer no-op"
    );
    assert_eq!(
        navigation.visual_right(caret(text.len(), TextAffinity::After)),
        caret(text.len(), TextAffinity::After),
        "exact trailing After alias survives an outer no-op"
    );
    let first_content = navigation.visual_right(caret(0, TextAffinity::Before));
    assert_eq!(first_content, caret(1, TextAffinity::After));
    assert_eq!(
        navigation.visual_left(first_content),
        caret(0, TextAffinity::After)
    );
    let trailing = navigation.visual_right(caret(2, TextAffinity::Before));
    assert_eq!(trailing, caret(3, TextAffinity::Before));
    assert_eq!(
        navigation.visual_left(trailing),
        caret(2, TextAffinity::Before)
    );
    let leading = navigation.caret_rect(caret(0, TextAffinity::After));
    assert_near(leading.y, layout.lines[0].top_y);
    assert_near(leading.height, layout.lines[0].height);

    let internal_text = "A\n\nB";
    let internal_layout = shape(internal_text, 400.0, false);
    let internal = internal_layout
        .navigation(internal_text)
        .expect("internal empty line map");
    assert_eq!(
        internal.caret_rect(caret(2, TextAffinity::Before)),
        internal.caret_rect(caret(2, TextAffinity::After))
    );
    let empty_rect = internal.caret_rect(caret(2, TextAffinity::After));
    assert_eq!(
        internal.hit_test_caret(0.0, empty_rect.y + empty_rect.height * 0.5),
        caret(2, TextAffinity::After)
    );
    assert_eq!(
        internal.visual_left(caret(2, TextAffinity::After)),
        caret(1, TextAffinity::Before)
    );
    assert_eq!(
        internal.visual_right(caret(2, TextAffinity::Before)),
        caret(3, TextAffinity::After)
    );

    let empty_layout = shape("", 400.0, false);
    let empty = empty_layout.navigation("").expect("empty source map");
    assert_eq!(empty.caret_stops().len(), 1);
    assert_eq!(empty.caret_stops()[0].caret, caret(0, TextAffinity::After));
}

#[test]
fn visual_word_targets_follow_full_buffer_policy_without_changing_deletion() {
    let text = "café אבג crème";
    let navigation = shape(text, 400.0, false)
        .navigation(text)
        .expect("word map");

    let mut right = caret(0, TextAffinity::After);
    let mut right_offsets = vec![right.offset];
    for _ in 0..3 {
        right = navigation.visual_word_right(right);
        right_offsets.push(right.offset);
    }
    assert_eq!(right_offsets, vec![0, 6, 13, 19]);

    let mut left = caret(text.len(), TextAffinity::Before);
    let mut left_offsets = vec![left.offset];
    for _ in 0..3 {
        left = navigation.visual_word_left(left);
        left_offsets.push(left.offset);
    }
    assert_eq!(left_offsets, vec![19, 13, 6, 0]);

    let mut logical = TextEditState::new(text);
    logical.set_caret(0);
    logical.delete_word_forward();
    assert_eq!(logical.text, "אבג crème");
}

#[test]
fn edit_state_canonicalizes_before_visual_branching_and_preserves_affinity() {
    let text = "Ae\u{301}B";
    let navigation = shape(text, 400.0, false)
        .navigation(text)
        .expect("combining map");

    let mut forward = TextEditState::new(text);
    forward.selection = TextSelection::new(2, 3);
    assert_eq!(
        forward.move_visual_right(&navigation),
        TextNavigationOutcome::Moved
    );
    assert_eq!(forward.selection, TextSelection::new(4, 4));
    assert_eq!(forward.caret_position().affinity, TextAffinity::Before);

    let mut backward = TextEditState::new(text);
    backward.selection = TextSelection::new(3, 2);
    backward.move_visual_left(&navigation);
    assert_eq!(backward.selection, TextSelection::new(0, 0));
    assert_eq!(backward.caret_position().affinity, TextAffinity::After);

    let mut extend = TextEditState::new(text);
    extend.selection = TextSelection::new(2, 3);
    extend.extend_visual_right(&navigation);
    assert_eq!(extend.selection, TextSelection::new(1, 4));
    assert_eq!(extend.caret_position().affinity, TextAffinity::Before);

    let mut extend_left = TextEditState::new(text);
    extend_left.selection = TextSelection::new(3, 2);
    extend_left.extend_visual_left(&navigation);
    assert_eq!(extend_left.selection, TextSelection::new(1, 0));
    assert_eq!(extend_left.caret_position().affinity, TextAffinity::After);

    let mut word_right = TextEditState::new(text);
    word_right.selection = TextSelection::new(2, 3);
    word_right.move_visual_word_right(&navigation);
    assert_eq!(word_right.selection, TextSelection::new(5, 5));

    let mut word_left = TextEditState::new(text);
    word_left.selection = TextSelection::new(3, 2);
    word_left.move_visual_word_left(&navigation);
    assert_eq!(word_left.selection, TextSelection::new(0, 0));

    let mut invalid_anchor = TextEditState::new(text);
    invalid_anchor.selection = TextSelection::new(2, 4);
    invalid_anchor.extend_visual_right(&navigation);
    assert_eq!(invalid_anchor.selection, TextSelection::new(1, 5));

    let mut invalid_anchor_move = TextEditState::new(text);
    invalid_anchor_move.selection = TextSelection::new(2, 4);
    invalid_anchor_move.move_visual_left(&navigation);
    assert_eq!(invalid_anchor_move.selection, TextSelection::new(1, 1));

    let mut invalid_anchor_word_move = TextEditState::new(text);
    invalid_anchor_word_move.selection = TextSelection::new(2, 4);
    invalid_anchor_word_move.move_visual_word_right(&navigation);
    assert_eq!(invalid_anchor_word_move.selection, TextSelection::new(4, 4));

    let mut word_extend = TextEditState::new(text);
    word_extend.selection = TextSelection::new(2, 3);
    word_extend.extend_visual_word_left(&navigation);
    assert_eq!(word_extend.selection, TextSelection::new(1, 0));

    let mut word_extend_right = TextEditState::new(text);
    word_extend_right.selection = TextSelection::new(2, 3);
    word_extend_right.extend_visual_word_right(&navigation);
    assert_eq!(word_extend_right.selection, TextSelection::new(1, 5));

    let mut invalid_anchor_word_extend = TextEditState::new(text);
    invalid_anchor_word_extend.selection = TextSelection::new(2, 4);
    invalid_anchor_word_extend.extend_visual_word_right(&navigation);
    assert_eq!(
        invalid_anchor_word_extend.selection,
        TextSelection::new(1, 5)
    );

    let mut outer = TextEditState::new(text);
    assert_eq!(
        outer.move_visual_right(&navigation),
        TextNavigationOutcome::Unchanged
    );
    assert!(!outer.undo(), "visual navigation never records text undo");
}

#[test]
fn edit_state_collapses_nonempty_selection_by_physical_rtl_rank() {
    let text = "אבג";
    let navigation = shape(text, 400.0, false).navigation(text).expect("rtl map");

    let mut left = TextEditState::new(text);
    left.set_selection(TextSelection::new(0, text.len()));
    left.move_visual_left(&navigation);
    assert_eq!(left.selection, TextSelection::new(6, 6));
    assert_eq!(left.caret_position().affinity, TextAffinity::Before);

    let mut right = TextEditState::new(text);
    right.set_selection(TextSelection::new(0, text.len()));
    right.move_visual_word_right(&navigation);
    assert_eq!(right.selection, TextSelection::new(0, 0));
    assert_eq!(right.caret_position().affinity, TextAffinity::After);

    let mut shifted = TextEditState::new(text);
    shifted.set_caret_position(caret(0, TextAffinity::After));
    shifted.extend_visual_left(&navigation);
    assert_eq!(shifted.selection, TextSelection::new(0, 2));
    assert_eq!(shifted.caret_position().affinity, TextAffinity::After);
    shifted.extend_visual_right(&navigation);
    assert_eq!(shifted.selection, TextSelection::new(0, 0));
    assert_eq!(shifted.caret_position().affinity, TextAffinity::After);
}

#[test]
fn stale_navigation_rejection_is_transactional_for_every_state_method() {
    let stale_text = "Xe\u{301}B";
    let navigation = shape(stale_text, 400.0, false)
        .navigation(stale_text)
        .expect("source map");
    let operations: [Operation; 8] = [
        TextEditState::move_visual_left,
        TextEditState::move_visual_right,
        TextEditState::extend_visual_left,
        TextEditState::extend_visual_right,
        TextEditState::move_visual_word_left,
        TextEditState::move_visual_word_right,
        TextEditState::extend_visual_word_left,
        TextEditState::extend_visual_word_right,
    ];

    for selection in [
        TextSelection::new(1, 1),
        TextSelection::new(2, 3),
        TextSelection::new(3, 2),
        TextSelection::new(2, 4),
    ] {
        let mut expected = TextEditState::new("Ae\u{301}B");
        expected.insert_text("x");
        assert!(expected.undo(), "establish a nonempty redo history");
        expected.selection = selection;
        expected.composition = Some(TextComposition::new(
            "候補",
            Some(TextRange::new(0, "候".len())),
        ));

        for operation in operations {
            let mut actual = expected.clone();
            assert_eq!(
                operation(&mut actual, &navigation),
                TextNavigationOutcome::SourceMismatch
            );
            assert_eq!(actual, expected);
        }
    }
}

#[test]
fn real_cosmic_wrap_omissions_become_exact_zero_width_navigation_cells() {
    let source = "alpha אבג beta gamma delta";
    let mut engine = CosmicTextEngine::new();
    let layout = engine.shape_text(&TextLayoutKey::new(
        source,
        TextStyle::new("Inter", 12.0, 17.0),
        66.0,
        true,
    ));
    assert_eq!(
        layout
            .lines
            .iter()
            .map(|line| {
                (
                    line.text_start,
                    line.text_end,
                    line.source_line_index,
                    line.rtl,
                )
            })
            .collect::<Vec<_>>(),
        vec![
            (0, 12, 0, false),
            (13, 17, 0, false),
            (18, 23, 0, false),
            (24, 29, 0, false),
        ]
    );
    for gap in [12..13, 17..18, 23..24] {
        assert_eq!(&source[gap], " ");
    }
    let first_max_right = layout
        .runs
        .iter()
        .filter(|run| run.visual_line == 0)
        .flat_map(|run| &run.glyphs)
        .map(|glyph| glyph.x + glyph.width)
        .max_by(f32::total_cmp)
        .expect("first line has glyph geometry");
    assert_near(first_max_right, 57.826_17);

    let navigation = layout.navigation(source).expect("wrapped navigation");
    for boundary in source
        .grapheme_indices(true)
        .map(|(offset, _)| offset)
        .chain(std::iter::once(source.len()))
    {
        assert!(
            navigation
                .caret_stops()
                .iter()
                .any(|stop| stop.caret.offset == boundary),
            "missing source boundary {boundary}"
        );
    }
}

#[test]
fn ltr_one_space_wrap_has_exact_affinity_geometry_and_selection() {
    let one = synthetic_layout(
        &[line(0, 0, 1, false), line(0, 2, 3, false)],
        &[
            cell(0, 0, 1, 0.0, 10.0, false),
            cell(1, 2, 3, 0.0, 10.0, false),
        ],
    )
    .navigation("a b")
    .expect("one omitted space");
    let one_start = caret(1, TextAffinity::After);
    let one_end_before = caret(2, TextAffinity::Before);
    let one_end_after = caret(2, TextAffinity::After);
    assert_eq!(one.visual_right(one_start), one_end_before);
    assert_eq!(one.visual_right(one_end_before), one_end_after);
    assert_eq!(one.visual_left(one_end_after), one_end_before);
    assert_eq!(one.visual_left(one_end_before), one_start);
    let prior = one.caret_rect(one_end_before);
    let next = one.caret_rect(one_end_after);
    assert_near(prior.x, 10.0);
    assert_near(prior.y, 0.0);
    assert_near(next.x, 0.0);
    assert_near(next.y, 24.0);
    assert_eq!(one.hit_test_caret(10.0, 12.0), one_start);
    assert_eq!(one.hit_test_caret(0.0, 36.0), one_end_after);
    assert!(one.selection_rects(1..2).is_empty());
    assert_near(
        one.selection_rects(0..3)
            .iter()
            .map(|rect| rect.width)
            .sum(),
        20.0,
    );
}

#[test]
fn ltr_multiple_wrap_delimiters_have_exact_affinity_hits_and_words() {
    let multiple = synthetic_layout(
        &[line(0, 0, 1, false), line(0, 4, 5, false)],
        &[
            cell(0, 0, 1, 0.0, 10.0, false),
            cell(1, 4, 5, 0.0, 10.0, false),
        ],
    )
    .navigation("a  \tb")
    .expect("multiple omitted delimiters");
    let mut current = caret(1, TextAffinity::After);
    let mut forward = vec![current];
    for _ in 0..4 {
        current = multiple.visual_right(current);
        forward.push(current);
    }
    assert_eq!(
        forward,
        vec![
            caret(1, TextAffinity::After),
            caret(2, TextAffinity::Before),
            caret(3, TextAffinity::Before),
            caret(4, TextAffinity::Before),
            caret(4, TextAffinity::After),
        ]
    );
    let mut current = caret(4, TextAffinity::After);
    let mut reverse = vec![current];
    for _ in 0..4 {
        current = multiple.visual_left(current);
        reverse.push(current);
    }
    assert_eq!(
        reverse,
        vec![
            caret(4, TextAffinity::After),
            caret(4, TextAffinity::Before),
            caret(3, TextAffinity::After),
            caret(2, TextAffinity::After),
            caret(1, TextAffinity::After),
        ]
    );
    for offset in 1..=4 {
        let affinity = if offset == 1 {
            TextAffinity::After
        } else {
            TextAffinity::Before
        };
        assert_near(multiple.caret_rect(caret(offset, affinity)).x, 10.0);
    }
    assert_eq!(
        multiple.hit_test_caret(10.0, 12.0),
        caret(1, TextAffinity::After)
    );
    assert_eq!(
        multiple.hit_test_caret(0.0, 36.0),
        caret(4, TextAffinity::After)
    );
    assert_eq!(
        multiple.visual_word_right(caret(1, TextAffinity::After)),
        caret(4, TextAffinity::Before)
    );
    assert_eq!(
        multiple.visual_word_left(caret(4, TextAffinity::After)),
        caret(0, TextAffinity::After)
    );
}

#[test]
fn consecutive_ltr_wrap_gaps_keep_both_seams_reversible() {
    let consecutive = synthetic_layout(
        &[
            line(0, 0, 1, false),
            line(0, 2, 3, false),
            line(0, 4, 5, false),
        ],
        &[
            cell(0, 0, 1, 0.0, 10.0, false),
            cell(1, 2, 3, 0.0, 10.0, false),
            cell(2, 4, 5, 0.0, 10.0, false),
        ],
    )
    .navigation("a b c")
    .expect("two omitted spaces");
    for (start, end) in [(1, 2), (3, 4)] {
        let start = caret(start, TextAffinity::After);
        let before = caret(end, TextAffinity::Before);
        let after = caret(end, TextAffinity::After);
        assert_eq!(consecutive.visual_right(start), before);
        assert_eq!(consecutive.visual_right(before), after);
        assert_eq!(consecutive.visual_left(after), before);
        assert_eq!(consecutive.visual_left(before), start);
    }
}

#[test]
fn embedded_and_opposite_bidi_runs_keep_line_major_wrap_truth() {
    let embedded = synthetic_layout(
        &[line(0, 0, 5, false), line(0, 6, 7, false)],
        &[
            cell(0, 0, 1, 0.0, 10.0, false),
            cell(0, 1, 3, 20.0, 30.0, true),
            cell(0, 3, 5, 10.0, 20.0, true),
            cell(1, 6, 7, 0.0, 10.0, false),
        ],
    )
    .navigation("aאב b")
    .expect("ltr paragraph with rtl tail");
    let start = caret(5, TextAffinity::After);
    let end_before = caret(6, TextAffinity::Before);
    let end_after = caret(6, TextAffinity::After);
    assert_near(embedded.caret_rect(start).x, 30.0);
    assert_near(embedded.caret_rect(end_before).x, 30.0);
    assert_near(embedded.caret_rect(end_before).y, 0.0);
    assert_near(embedded.caret_rect(end_after).x, 0.0);
    assert_near(embedded.caret_rect(end_after).y, 24.0);
    assert_eq!(embedded.visual_right(start), end_before);
    assert_eq!(embedded.visual_right(end_before), end_after);
    assert_eq!(embedded.visual_left(end_after), end_before);
    assert_eq!(embedded.visual_left(end_before), start);
    assert_eq!(
        embedded.hit_test_caret(30.0, 12.0),
        caret(1, TextAffinity::After)
    );

    let opposite = synthetic_layout(
        &[line(0, 0, 1, false), line(0, 2, 6, false)],
        &[
            cell(0, 0, 1, 0.0, 10.0, false),
            cell(1, 2, 4, 10.0, 20.0, true),
            cell(1, 4, 6, 0.0, 10.0, true),
        ],
    )
    .navigation("a אב")
    .expect("opposite-direction next-line run remains valid");
    let prior_end = caret(2, TextAffinity::Before);
    let physical_next = caret(6, TextAffinity::Before);
    assert_eq!(opposite.visual_right(prior_end), physical_next);
    assert_eq!(opposite.visual_left(physical_next), prior_end);
    let logical_next = opposite.caret_rect(caret(2, TextAffinity::After));
    assert_near(logical_next.x, 20.0);
    assert_near(logical_next.y, 24.0);
}

#[test]
fn rtl_one_space_wrap_is_local_reversible_and_preserves_external_ranks() {
    let source = "אב ב";
    let navigation = synthetic_layout(
        &[line(0, 0, 4, true), line(0, 5, 7, true)],
        &[
            cell(0, 0, 2, 20.0, 30.0, true),
            cell(0, 2, 4, 10.0, 20.0, true),
            cell(1, 5, 7, 20.0, 30.0, true),
        ],
    )
    .navigation(source)
    .expect("rtl omitted space");
    let collapsed = navigation
        .caret_stops()
        .iter()
        .filter(|stop| stop.visual_line == 0 && stop.x.to_bits() == 10.0_f32.to_bits())
        .map(|stop| stop.caret)
        .collect::<Vec<_>>();
    assert_eq!(
        collapsed,
        vec![
            caret(5, TextAffinity::Before),
            caret(4, TextAffinity::After)
        ]
    );
    let gap_start = caret(4, TextAffinity::After);
    let gap_end = caret(5, TextAffinity::Before);
    assert_eq!(navigation.visual_left(gap_start), gap_end);
    assert_eq!(navigation.visual_right(gap_end), gap_start);
    assert_eq!(navigation.visual_left(gap_end), gap_end);
    assert_eq!(
        navigation.visual_right(caret(0, TextAffinity::After)),
        caret(7, TextAffinity::Before)
    );
    assert_eq!(
        navigation.visual_left(caret(7, TextAffinity::Before)),
        caret(0, TextAffinity::After)
    );
    let source_alias = navigation.caret_rect(caret(5, TextAffinity::After));
    assert_near(source_alias.x, 30.0);
    assert_near(source_alias.y, 24.0);
    assert_eq!(navigation.hit_test_caret(10.0, 12.0), gap_start);
    assert!(navigation.selection_rects(4..5).is_empty());

    let mut collapse_left = TextEditState::new(source);
    collapse_left.set_selection_with_affinity(TextSelection::new(4, 5), TextAffinity::After);
    assert_eq!(
        collapse_left.move_visual_left(&navigation),
        TextNavigationOutcome::Moved
    );
    assert_eq!(collapse_left.caret_position(), gap_start);
    let mut collapse_right = TextEditState::new(source);
    collapse_right.set_selection_with_affinity(TextSelection::new(4, 5), TextAffinity::After);
    assert_eq!(
        collapse_right.move_visual_right(&navigation),
        TextNavigationOutcome::Moved
    );
    assert_eq!(
        collapse_right.caret_position(),
        caret(5, TextAffinity::After)
    );
}

#[test]
fn rtl_multiple_wrap_delimiters_have_exact_local_words_and_affinities() {
    let multiple_source = "א  \tב";
    let multiple = synthetic_layout(
        &[line(0, 0, 2, true), line(0, 5, 7, true)],
        &[
            cell(0, 0, 2, 10.0, 20.0, true),
            cell(1, 5, 7, 10.0, 20.0, true),
        ],
    )
    .navigation(multiple_source)
    .expect("rtl multiple omitted delimiters");
    let mut current = caret(2, TextAffinity::After);
    let mut forward = vec![current];
    for _ in 0..3 {
        current = multiple.visual_left(current);
        forward.push(current);
    }
    assert_eq!(
        forward,
        vec![
            caret(2, TextAffinity::After),
            caret(3, TextAffinity::After),
            caret(4, TextAffinity::After),
            caret(5, TextAffinity::Before),
        ]
    );
    let mut current = caret(5, TextAffinity::Before);
    let mut reverse = vec![current];
    for _ in 0..3 {
        current = multiple.visual_right(current);
        reverse.push(current);
    }
    assert_eq!(
        reverse,
        vec![
            caret(5, TextAffinity::Before),
            caret(4, TextAffinity::Before),
            caret(3, TextAffinity::Before),
            caret(2, TextAffinity::After),
        ]
    );
    assert_eq!(
        multiple.hit_test_caret(10.0, 12.0),
        caret(2, TextAffinity::After)
    );
    assert_eq!(
        multiple.visual_word_left(caret(2, TextAffinity::After)),
        caret(5, TextAffinity::Before)
    );
    assert_eq!(
        multiple.visual_word_right(caret(5, TextAffinity::Before)),
        caret(0, TextAffinity::After)
    );
    assert_eq!(
        multiple.visual_word_right(caret(5, TextAffinity::After)),
        caret(5, TextAffinity::After)
    );
}

#[test]
fn wrap_gap_exception_rejects_every_ineligible_provenance_class() {
    assert!(shape("a\nb", 400.0, false).navigation("a\nb").is_ok());

    let cases = [
        (
            "a b",
            vec![line(0, 0, 1, false), line(1, 2, 3, false)],
            vec![
                cell(0, 0, 1, 0.0, 10.0, false),
                cell(1, 2, 3, 0.0, 10.0, false),
            ],
        ),
        (
            "a b",
            vec![line(0, 0, 1, false), line(0, 2, 3, true)],
            vec![
                cell(0, 0, 1, 0.0, 10.0, false),
                cell(1, 2, 3, 0.0, 10.0, false),
            ],
        ),
        (
            "a\u{00a0}b",
            vec![line(0, 0, 1, false), line(0, 3, 4, false)],
            vec![
                cell(0, 0, 1, 0.0, 10.0, false),
                cell(1, 3, 4, 0.0, 10.0, false),
            ],
        ),
        (
            "a\u{2003}b",
            vec![line(0, 0, 1, false), line(0, 4, 5, false)],
            vec![
                cell(0, 0, 1, 0.0, 10.0, false),
                cell(1, 4, 5, 0.0, 10.0, false),
            ],
        ),
        (
            "a x b",
            vec![line(0, 0, 1, false), line(0, 4, 5, false)],
            vec![
                cell(0, 0, 1, 0.0, 10.0, false),
                cell(1, 4, 5, 0.0, 10.0, false),
            ],
        ),
        (
            "a b",
            vec![line(0, 0, 3, false)],
            vec![
                cell(0, 0, 1, 0.0, 10.0, false),
                cell(0, 2, 3, 10.0, 20.0, false),
            ],
        ),
        (
            " a",
            vec![line(0, 1, 2, false)],
            vec![cell(0, 1, 2, 0.0, 10.0, false)],
        ),
        (
            "a ",
            vec![line(0, 0, 1, false)],
            vec![cell(0, 0, 1, 0.0, 10.0, false)],
        ),
        (
            "a ",
            vec![line(0, 0, 1, false), line(0, 2, 2, false)],
            vec![cell(0, 0, 1, 0.0, 10.0, false)],
        ),
        (
            "a b",
            vec![line(0, 0, 1, false), line(0, 2, 3, false)],
            vec![cell(1, 2, 3, 0.0, 10.0, false)],
        ),
    ];
    for (source, lines, cells) in cases {
        assert_eq!(
            synthetic_layout(&lines, &cells).navigation(source),
            Err(TextNavigationError::UncoveredGrapheme),
            "unexpected result for {source:?}"
        );
    }
}

#[test]
fn constructor_documents_source_trust_and_rejects_every_malformed_class() {
    let layout = shape("ab", 400.0, false);
    let wrong_source = layout
        .navigation("cd")
        .expect("same-boundary source is structurally compatible");
    assert!(wrong_source.matches_source("cd"));
    assert!(!wrong_source.matches_source("ab"));
    let mut state = TextEditState::new("ab");
    let expected = state.clone();
    assert_eq!(
        state.move_visual_right(&wrong_source),
        TextNavigationOutcome::SourceMismatch
    );
    assert_eq!(state, expected);

    let mut malformed = layout.clone();
    malformed.lines.clear();
    assert_eq!(
        malformed.navigation("ab"),
        Err(TextNavigationError::MissingVisualLine)
    );

    let mut malformed = shape("a\nb", 400.0, false);
    malformed.lines.swap(0, 1);
    assert_eq!(
        malformed.navigation("a\nb"),
        Err(TextNavigationError::MissingVisualLine)
    );

    let mut malformed = shape("a\nb", 400.0, false);
    malformed.lines[1].visual_index = 0;
    assert_eq!(
        malformed.navigation("a\nb"),
        Err(TextNavigationError::DuplicateVisualLine)
    );

    let mut malformed = layout.clone();
    malformed.line_count = 2;
    assert_eq!(
        malformed.navigation("ab"),
        Err(TextNavigationError::InvalidLineRange)
    );

    let mut malformed = layout.clone();
    malformed.lines[0].height = 0.0;
    assert_eq!(
        malformed.navigation("ab"),
        Err(TextNavigationError::InvalidLineGeometry)
    );
    let mut malformed = layout.clone();
    malformed.lines[0].top_y = f32::MAX;
    malformed.lines[0].height = f32::MAX;
    assert_eq!(
        malformed.navigation("ab"),
        Err(TextNavigationError::InvalidLineGeometry)
    );

    let mut malformed = layout.clone();
    malformed.runs[0].visual_line = 99;
    assert_eq!(
        malformed.navigation("ab"),
        Err(TextNavigationError::OrphanGlyphRun)
    );

    let mut malformed = layout.clone();
    malformed.runs[0].glyphs[0].end = malformed.runs[0].glyphs[0].start;
    assert_eq!(
        malformed.navigation("ab"),
        Err(TextNavigationError::InvalidGlyphRange)
    );

    let mut malformed = layout.clone();
    malformed.runs[0].glyphs[0].width = -1.0;
    assert_eq!(
        malformed.navigation("ab"),
        Err(TextNavigationError::InvalidGlyphGeometry)
    );
    let mut malformed = layout.clone();
    malformed.runs[0].glyphs[0].x = f32::MAX;
    malformed.runs[0].glyphs[0].width = f32::MAX;
    assert_eq!(
        malformed.navigation("ab"),
        Err(TextNavigationError::InvalidGlyphGeometry)
    );

    let mut malformed = layout.clone();
    let mut duplicate = malformed.runs[0].glyphs[0];
    duplicate.rtl = !duplicate.rtl;
    malformed.runs[0].glyphs.push(duplicate);
    assert_eq!(
        malformed.navigation("ab"),
        Err(TextNavigationError::InconsistentClusterDirection)
    );

    let mut malformed = layout.clone();
    malformed.runs[0].glyphs[0].start = 0;
    malformed.runs[0].glyphs[0].end = 2;
    malformed.runs[0].glyphs[1].start = 1;
    malformed.runs[0].glyphs[1].end = 2;
    assert_eq!(
        malformed.navigation("ab"),
        Err(TextNavigationError::OverlappingClusters)
    );

    let mut malformed = layout.clone();
    malformed.runs[0].glyphs.pop();
    assert_eq!(
        malformed.navigation("ab"),
        Err(TextNavigationError::UncoveredGrapheme)
    );
}

#[test]
fn finite_extreme_hit_distances_choose_the_mathematically_nearest_stop_and_line() {
    let mut horizontal = one_cluster_layout("a", f32::MAX / 2.0);
    horizontal.runs[0].glyphs[0].x = -f32::MAX;
    horizontal.runs[0].glyphs[0].width = f32::MAX / 2.0;
    let horizontal = horizontal.navigation("a").expect("extreme x map");
    assert_eq!(
        horizontal.hit_test_caret(f32::MAX, 0.0),
        caret(1, TextAffinity::Before)
    );

    let mut vertical = shape("\n", 400.0, false);
    assert_eq!(vertical.lines.len(), 2);
    vertical.lines[0].top_y = -f32::MAX;
    vertical.lines[0].baseline_y = -f32::MAX;
    vertical.lines[0].height = 1.0;
    vertical.lines[1].top_y = -f32::MAX / 2.0;
    vertical.lines[1].baseline_y = -f32::MAX / 2.0;
    vertical.lines[1].height = 1.0;
    let vertical = vertical.navigation("\n").expect("extreme y map");
    assert_eq!(
        vertical.hit_test_caret(0.0, f32::MAX),
        caret(1, TextAffinity::Before)
    );
}

#[test]
fn cross_cluster_selection_union_overflow_is_rejected_during_construction() {
    let mut layout = two_cluster_layout(0.0);
    layout.runs[0].glyphs[0].x = -f32::MAX;
    layout.runs[0].glyphs[0].width = f32::MAX;
    layout.runs[0].glyphs[1].x = -1.0;
    layout.runs[0].glyphs[1].width = f32::MAX;
    layout.lines[0].width = f32::MAX;
    layout.size.width = f32::MAX;

    assert_eq!(
        layout.navigation("ab"),
        Err(TextNavigationError::InvalidGlyphGeometry)
    );
}

#[test]
fn duplicate_cluster_union_overflow_fails_before_nodes_escape() {
    let mut layout = shape("a", 400.0, false);
    layout.runs[0].glyphs[0].x = -f32::MAX;
    layout.runs[0].glyphs[0].width = 0.0;
    let mut duplicate = layout.runs[0].glyphs[0];
    duplicate.x = f32::MAX;
    layout.runs[0].glyphs.push(duplicate);

    assert_eq!(
        layout.navigation("a"),
        Err(TextNavigationError::InvalidGlyphGeometry)
    );
}

#[test]
fn nondefault_weight_preserves_unicode_bidi_and_multiline_source_topology() {
    let source = "Latin e\u{301} אבג 12038475\nSecond 👩‍🚀 line";
    let request = TextLayoutKey::new(
        source,
        TextStyle::new("Inter", 18.0, 24.0).with_weight(700),
        180.0,
        true,
    );
    let layout = CosmicTextEngine::new().shape_text(&request);

    assert_eq!(request.text, source);
    assert_eq!(request.style.weight, 700);
    assert!(layout.line_count >= 2);
    assert_eq!(layout.lines.first().expect("first line").text_start, 0);
    assert_eq!(
        layout.lines.last().expect("last line").text_end,
        source.len()
    );
    assert!(layout.navigation(source).is_ok());
    assert!(
        layout
            .runs
            .iter()
            .all(|run| run.normalized_coords == [0, 8_848])
    );
    assert!(layout.runs.iter().flat_map(|run| &run.glyphs).all(|glyph| {
        glyph.start <= glyph.end
            && glyph.end <= source.len()
            && source.is_char_boundary(glyph.start)
            && source.is_char_boundary(glyph.end)
    }));
}
