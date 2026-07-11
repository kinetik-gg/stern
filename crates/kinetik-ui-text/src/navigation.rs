use std::collections::BTreeSet;
use std::ops::Range;

use kinetik_ui_core::Rect;
use unicode_segmentation::UnicodeSegmentation;

use crate::boundary::{clamp_boundary, next_word_boundary, previous_word_boundary};
use crate::{ShapedTextLayout, TextAffinity, TextCaret};

/// Logical-coordinate tolerance used when shaped caret edges share a position.
pub const SHAPED_TEXT_GEOMETRY_EPSILON: f32 = 1.0e-4;

/// Structural error returned while deriving shaped text navigation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextNavigationError {
    /// The layout has no visual line or skips a visual-line index.
    MissingVisualLine,
    /// More than one line claims the same visual-line index.
    DuplicateVisualLine,
    /// A line count or source range is inconsistent.
    InvalidLineRange,
    /// Layout or line geometry is non-finite, negative, or overflowing.
    InvalidLineGeometry,
    /// A glyph run does not name its owning visual and source line.
    OrphanGlyphRun,
    /// A glyph source range is empty, outside its line, or not grapheme-aligned.
    InvalidGlyphRange,
    /// Glyph or derived cluster geometry is non-finite, negative, or overflowing.
    InvalidGlyphGeometry,
    /// Glyphs that share one cluster range disagree about its direction.
    InconsistentClusterDirection,
    /// Distinct cluster ranges overlap logically on one visual line.
    OverlappingClusters,
    /// A source grapheme is not covered by exactly one shaped cluster.
    UncoveredGrapheme,
}

/// Result of applying shaped visual navigation to editable text state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextNavigationOutcome {
    /// The canonical selection or its active affinity changed.
    Moved,
    /// The matching state was already at the requested visual position.
    Unchanged,
    /// The navigation map belongs to different source text.
    SourceMismatch,
}

/// One canonical caret exposed at a shaped visual coordinate.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShapedCaretStop {
    /// Logical source offset and canonical visual affinity.
    pub caret: TextCaret,
    /// Visual line in layout order.
    pub visual_line: usize,
    /// Logical x coordinate within the shaped layout.
    pub x: f32,
}

#[derive(Debug, Clone, PartialEq)]
struct NavigationLine {
    visual_index: usize,
    text_start: usize,
    text_end: usize,
    top_y: f32,
    height: f32,
}

#[derive(Debug, Clone, PartialEq)]
struct GraphemeCell {
    visual_line: usize,
    start: usize,
    end: usize,
    left: f32,
    right: f32,
    rtl: bool,
}

#[derive(Debug, Clone, PartialEq)]
struct CaretNode {
    visual_line: usize,
    offset: usize,
    x: f32,
    has_before: bool,
    has_after: bool,
}

impl CaretNode {
    fn has_affinity(&self, affinity: TextAffinity) -> bool {
        match affinity {
            TextAffinity::Before => self.has_before,
            TextAffinity::After => self.has_after,
        }
    }

    fn canonical_caret(&self, source_len: usize) -> TextCaret {
        let affinity = if self.offset == 0 {
            TextAffinity::After
        } else if self.offset == source_len {
            TextAffinity::Before
        } else if self.has_after {
            TextAffinity::After
        } else {
            TextAffinity::Before
        };
        TextCaret::new(self.offset, affinity)
    }
}

#[derive(Debug, Clone, Copy)]
struct Edge {
    visual_line: usize,
    offset: usize,
    x: f32,
    affinity: TextAffinity,
}

#[derive(Debug, Clone, Copy)]
struct Cluster {
    visual_line: usize,
    start: usize,
    end: usize,
    left: f32,
    right: f32,
    rtl: bool,
}

/// Owned, source-bound shaped authority for visual text geometry and movement.
///
/// The caller must pass the exact source used to produce the public
/// [`ShapedTextLayout`]. Existing shaped structs intentionally carry no source
/// provenance, so construction validates structural consistency but cannot
/// independently prove that historical pairing. Later source checks use exact
/// string equality.
#[derive(Debug, Clone, PartialEq)]
pub struct ShapedTextNavigation {
    source: String,
    lines: Vec<NavigationLine>,
    cells: Vec<GraphemeCell>,
    nodes: Vec<CaretNode>,
    stops: Vec<ShapedCaretStop>,
    word_targets: BTreeSet<usize>,
}

impl ShapedTextLayout {
    /// Derives one owned visual-navigation authority from positioned clusters.
    ///
    /// Construction is all-or-nothing. It validates the complete public layout
    /// against `source` and never exposes a partial map.
    ///
    /// # Errors
    ///
    /// Returns [`TextNavigationError`] when any public line, run, glyph,
    /// cluster, geometry, or grapheme-coverage invariant is invalid.
    pub fn navigation(&self, source: &str) -> Result<ShapedTextNavigation, TextNavigationError> {
        ShapedTextNavigation::from_layout(self, source)
    }
}

impl ShapedTextNavigation {
    fn from_layout(layout: &ShapedTextLayout, source: &str) -> Result<Self, TextNavigationError> {
        validate_visual_lines(layout, source)?;
        let lines = navigation_lines(layout);
        let clusters = validated_clusters(layout, source)?;
        validate_cluster_overlap(&clusters)?;
        let cells = cluster_cells(source, &clusters)?;
        validate_cell_unions(&cells)?;
        validate_coverage(source, &lines, &cells)?;
        let nodes = coordinate_nodes(source.len(), &lines, &cells);
        let stops = nodes
            .iter()
            .map(|node| ShapedCaretStop {
                caret: node.canonical_caret(source.len()),
                visual_line: node.visual_line,
                x: node.x,
            })
            .collect();
        let word_targets = word_targets(source);

        Ok(Self {
            source: source.to_owned(),
            lines,
            cells,
            nodes,
            stops,
            word_targets,
        })
    }

    /// Returns the exact source snapshot owned by this map.
    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }

    /// Returns whether `source` exactly matches this map's source snapshot.
    #[must_use]
    pub fn matches_source(&self, source: &str) -> bool {
        self.source == source
    }

    /// Returns canonical shaped caret stops in physical visual order.
    #[must_use]
    pub fn caret_stops(&self) -> &[ShapedCaretStop] {
        &self.stops
    }

    /// Returns the shaped caret rectangle for a logical caret and affinity.
    #[must_use]
    pub fn caret_rect(&self, caret: TextCaret) -> Rect {
        let (index, _) = self.resolve_caret_with_rank(caret);
        let node = &self.nodes[index];
        let line = &self.lines[node.visual_line];
        Rect::new(node.x, line.top_y, 1.0, line.height)
    }

    /// Returns the nearest canonical shaped caret for a logical point.
    #[must_use]
    pub fn hit_test_caret(&self, x: f32, y: f32) -> TextCaret {
        if !x.is_finite() || !y.is_finite() {
            return self.stops[0].caret;
        }

        let Some(line) = self.lines.iter().min_by(|left, right| {
            line_distance(left, y)
                .total_cmp(&line_distance(right, y))
                .then_with(|| left.visual_index.cmp(&right.visual_index))
        }) else {
            return self.stops[0].caret;
        };

        self.nodes
            .iter()
            .filter(|node| node.visual_line == line.visual_index)
            .min_by(|left, right| {
                point_distance(x, left.x)
                    .total_cmp(&point_distance(x, right.x))
                    .then_with(|| left.x.total_cmp(&right.x))
                    .then_with(|| left.offset.cmp(&right.offset))
                    .then_with(|| {
                        affinity_order(left.canonical_caret(self.source.len()).affinity).cmp(
                            &affinity_order(right.canonical_caret(self.source.len()).affinity),
                        )
                    })
            })
            .map_or(self.stops[0].caret, |node| {
                node.canonical_caret(self.source.len())
            })
    }

    /// Returns shaped visual rectangles for one logical source range.
    #[must_use]
    pub fn selection_rects(&self, range: Range<usize>) -> Vec<Rect> {
        let start = clamp_boundary(&self.source, range.start);
        let end = clamp_boundary(&self.source, range.end);
        if start >= end {
            return Vec::new();
        }

        let mut spans = self
            .cells
            .iter()
            .filter(|cell| cell.start < end && cell.end > start)
            .filter_map(|cell| {
                let width = cell.right - cell.left;
                (width > 0.0).then_some((cell.visual_line, cell.left, cell.right))
            })
            .collect::<Vec<_>>();
        spans.sort_by(|left, right| {
            left.0
                .cmp(&right.0)
                .then_with(|| left.1.total_cmp(&right.1))
                .then_with(|| left.2.total_cmp(&right.2))
        });

        let mut merged = Vec::<(usize, f32, f32)>::new();
        for (visual_line, left, right) in spans {
            if let Some((last_line, _, last_right)) = merged.last_mut()
                && *last_line == visual_line
                && visual_gap(left, *last_right) <= f64::from(SHAPED_TEXT_GEOMETRY_EPSILON)
            {
                *last_right = (*last_right).max(right);
            } else {
                merged.push((visual_line, left, right));
            }
        }

        merged
            .into_iter()
            .map(|(visual_line, left, right)| {
                let line = &self.lines[visual_line];
                Rect::new(left, line.top_y, right - left, line.height)
            })
            .collect()
    }

    /// Moves one shaped caret coordinate toward physical visual left.
    #[must_use]
    pub fn visual_left(&self, caret: TextCaret) -> TextCaret {
        self.adjacent(caret, VisualDirection::Left)
    }

    /// Moves one shaped caret coordinate toward physical visual right.
    #[must_use]
    pub fn visual_right(&self, caret: TextCaret) -> TextCaret {
        self.adjacent(caret, VisualDirection::Right)
    }

    /// Moves to the next full-buffer word target toward physical visual left.
    #[must_use]
    pub fn visual_word_left(&self, caret: TextCaret) -> TextCaret {
        self.word(caret, VisualDirection::Left)
    }

    /// Moves to the next full-buffer word target toward physical visual right.
    #[must_use]
    pub fn visual_word_right(&self, caret: TextCaret) -> TextCaret {
        self.word(caret, VisualDirection::Right)
    }

    pub(crate) fn resolve_caret_with_rank(&self, caret: TextCaret) -> (usize, TextCaret) {
        if let Some(index) = self.find_alias(caret.offset, caret.affinity) {
            return (index, caret);
        }

        let offset = clamp_boundary(&self.source, caret.offset);
        if let Some(index) = self.find_alias(offset, caret.affinity) {
            return (index, TextCaret::new(offset, caret.affinity));
        }

        let affinity = default_affinity(&self.source, offset);
        if let Some(index) = self.find_alias(offset, affinity) {
            return (index, TextCaret::new(offset, affinity));
        }

        if let Some((index, node)) = self
            .nodes
            .iter()
            .enumerate()
            .find(|(_, node)| node.offset == offset)
        {
            return (index, node.canonical_caret(self.source.len()));
        }

        (0, self.nodes[0].canonical_caret(self.source.len()))
    }

    fn find_alias(&self, offset: usize, affinity: TextAffinity) -> Option<usize> {
        self.nodes
            .iter()
            .position(|node| node.offset == offset && node.has_affinity(affinity))
    }

    fn adjacent(&self, caret: TextCaret, direction: VisualDirection) -> TextCaret {
        let (index, resolved) = self.resolve_caret_with_rank(caret);
        let target = match direction {
            VisualDirection::Left => index.checked_sub(1),
            VisualDirection::Right => (index + 1 < self.nodes.len()).then_some(index + 1),
        };
        target.map_or(resolved, |target| self.arrival_caret(target, direction))
    }

    fn word(&self, caret: TextCaret, direction: VisualDirection) -> TextCaret {
        let (index, resolved) = self.resolve_caret_with_rank(caret);
        let current_offset = self.nodes[index].offset;
        let candidate = match direction {
            VisualDirection::Left => (0..index).rev().find(|candidate| {
                let offset = self.nodes[*candidate].offset;
                offset != current_offset && self.word_targets.contains(&offset)
            }),
            VisualDirection::Right => ((index + 1)..self.nodes.len()).find(|candidate| {
                let offset = self.nodes[*candidate].offset;
                offset != current_offset && self.word_targets.contains(&offset)
            }),
        };
        candidate.map_or(resolved, |target| self.arrival_caret(target, direction))
    }

    fn arrival_caret(&self, index: usize, direction: VisualDirection) -> TextCaret {
        let node = &self.nodes[index];
        if node.offset == 0 {
            return TextCaret::new(0, TextAffinity::After);
        }
        if node.offset == self.source.len() && !self.source.is_empty() {
            return TextCaret::new(node.offset, TextAffinity::Before);
        }

        let affinity = match direction {
            VisualDirection::Left if node.has_after => TextAffinity::After,
            VisualDirection::Right if node.has_before => TextAffinity::Before,
            _ => return node.canonical_caret(self.source.len()),
        };
        TextCaret::new(node.offset, affinity)
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum VisualDirection {
    Left,
    Right,
}

pub(crate) fn default_affinity(text: &str, offset: usize) -> TextAffinity {
    if !text.is_empty() && offset >= text.len() {
        TextAffinity::Before
    } else {
        TextAffinity::After
    }
}

fn validate_visual_lines(
    layout: &ShapedTextLayout,
    source: &str,
) -> Result<(), TextNavigationError> {
    if layout.lines.is_empty() {
        return Err(TextNavigationError::MissingVisualLine);
    }

    let mut seen = vec![false; layout.lines.len()];
    for line in &layout.lines {
        let Some(slot) = seen.get_mut(line.visual_index) else {
            return Err(TextNavigationError::MissingVisualLine);
        };
        if *slot {
            return Err(TextNavigationError::DuplicateVisualLine);
        }
        *slot = true;
    }
    if seen.iter().any(|seen| !seen) {
        return Err(TextNavigationError::MissingVisualLine);
    }
    if layout
        .lines
        .iter()
        .enumerate()
        .any(|(index, line)| line.visual_index != index)
    {
        return Err(TextNavigationError::MissingVisualLine);
    }
    if layout.line_count != layout.lines.len() {
        return Err(TextNavigationError::InvalidLineRange);
    }

    let boundaries = grapheme_boundaries(source);
    for line in &layout.lines {
        if line.text_start > line.text_end
            || line.text_end > source.len()
            || boundaries.binary_search(&line.text_start).is_err()
            || boundaries.binary_search(&line.text_end).is_err()
        {
            return Err(TextNavigationError::InvalidLineRange);
        }
    }

    let mut nonempty = layout
        .lines
        .iter()
        .filter(|line| line.text_start < line.text_end)
        .collect::<Vec<_>>();
    nonempty.sort_by_key(|line| (line.text_start, line.text_end));
    if nonempty
        .windows(2)
        .any(|pair| pair[1].text_start < pair[0].text_end)
    {
        return Err(TextNavigationError::InvalidLineRange);
    }

    if !layout.size.width.is_finite()
        || !layout.size.height.is_finite()
        || layout.size.width < 0.0
        || layout.size.height < 0.0
    {
        return Err(TextNavigationError::InvalidLineGeometry);
    }
    for line in &layout.lines {
        if !line.top_y.is_finite()
            || !line.baseline_y.is_finite()
            || !line.height.is_finite()
            || !line.width.is_finite()
            || line.height <= 0.0
            || line.width < 0.0
            || !(line.top_y + line.height).is_finite()
        {
            return Err(TextNavigationError::InvalidLineGeometry);
        }
    }

    Ok(())
}

fn navigation_lines(layout: &ShapedTextLayout) -> Vec<NavigationLine> {
    let mut lines = layout
        .lines
        .iter()
        .map(|line| NavigationLine {
            visual_index: line.visual_index,
            text_start: line.text_start,
            text_end: line.text_end,
            top_y: line.top_y,
            height: line.height,
        })
        .collect::<Vec<_>>();
    lines.sort_by_key(|line| line.visual_index);
    lines
}

fn validated_clusters(
    layout: &ShapedTextLayout,
    source: &str,
) -> Result<Vec<Cluster>, TextNavigationError> {
    let boundaries = grapheme_boundaries(source);
    let mut glyphs = Vec::new();
    for run in &layout.runs {
        let Some(line) = layout
            .lines
            .iter()
            .find(|line| line.visual_index == run.visual_line)
        else {
            return Err(TextNavigationError::OrphanGlyphRun);
        };
        if run.line_index != line.source_line_index {
            return Err(TextNavigationError::OrphanGlyphRun);
        }

        for glyph in &run.glyphs {
            if glyph.start >= glyph.end
                || glyph.end > source.len()
                || glyph.start < line.text_start
                || glyph.end > line.text_end
                || boundaries.binary_search(&glyph.start).is_err()
                || boundaries.binary_search(&glyph.end).is_err()
            {
                return Err(TextNavigationError::InvalidGlyphRange);
            }
            let right = glyph.x + glyph.width;
            if !glyph.x.is_finite()
                || !glyph.y.is_finite()
                || !glyph.width.is_finite()
                || glyph.width < 0.0
                || !right.is_finite()
            {
                return Err(TextNavigationError::InvalidGlyphGeometry);
            }
            glyphs.push((run.visual_line, glyph, right));
        }
    }
    glyphs.sort_by(|left, right| {
        left.0
            .cmp(&right.0)
            .then_with(|| left.1.start.cmp(&right.1.start))
            .then_with(|| left.1.end.cmp(&right.1.end))
            .then_with(|| left.1.x.total_cmp(&right.1.x))
    });

    let mut clusters = Vec::<Cluster>::new();
    for (visual_line, glyph, right) in glyphs {
        if let Some(cluster) = clusters.last_mut()
            && cluster.visual_line == visual_line
            && cluster.start == glyph.start
            && cluster.end == glyph.end
        {
            if cluster.rtl != glyph.rtl {
                return Err(TextNavigationError::InconsistentClusterDirection);
            }
            cluster.left = cluster.left.min(glyph.x);
            cluster.right = cluster.right.max(right);
            if !(cluster.right - cluster.left).is_finite() {
                return Err(TextNavigationError::InvalidGlyphGeometry);
            }
            continue;
        }
        clusters.push(Cluster {
            visual_line,
            start: glyph.start,
            end: glyph.end,
            left: glyph.x,
            right,
            rtl: glyph.rtl,
        });
    }
    Ok(clusters)
}

fn validate_cluster_overlap(clusters: &[Cluster]) -> Result<(), TextNavigationError> {
    if clusters
        .windows(2)
        .any(|pair| pair[0].visual_line == pair[1].visual_line && pair[1].start < pair[0].end)
    {
        return Err(TextNavigationError::OverlappingClusters);
    }
    Ok(())
}

#[allow(clippy::cast_precision_loss)]
fn cluster_cells(
    source: &str,
    clusters: &[Cluster],
) -> Result<Vec<GraphemeCell>, TextNavigationError> {
    let mut cells = Vec::new();
    for cluster in clusters {
        let graphemes = source[cluster.start..cluster.end]
            .grapheme_indices(true)
            .collect::<Vec<_>>();
        let count = graphemes.len();
        if count == 0 {
            return Err(TextNavigationError::InvalidGlyphRange);
        }
        let unit = (cluster.right - cluster.left) / count as f32;
        for (logical_index, (relative_start, grapheme)) in graphemes.into_iter().enumerate() {
            let visual_index = if cluster.rtl {
                count - logical_index - 1
            } else {
                logical_index
            };
            let left = cluster.left + unit * visual_index as f32;
            let right = if visual_index + 1 == count {
                cluster.right
            } else {
                cluster.left + unit * (visual_index + 1) as f32
            };
            if !left.is_finite() || !right.is_finite() || !(right - left).is_finite() {
                return Err(TextNavigationError::InvalidGlyphGeometry);
            }
            let start = cluster.start + relative_start;
            cells.push(GraphemeCell {
                visual_line: cluster.visual_line,
                start,
                end: start + grapheme.len(),
                left,
                right,
                rtl: cluster.rtl,
            });
        }
    }
    Ok(cells)
}

fn validate_coverage(
    source: &str,
    lines: &[NavigationLine],
    cells: &[GraphemeCell],
) -> Result<(), TextNavigationError> {
    for line in lines {
        for (relative_start, grapheme) in
            source[line.text_start..line.text_end].grapheme_indices(true)
        {
            let start = line.text_start + relative_start;
            let end = start + grapheme.len();
            if coverage_count(cells, start, end) != 1 {
                return Err(TextNavigationError::UncoveredGrapheme);
            }
        }
    }

    for (start, grapheme) in source.grapheme_indices(true) {
        if grapheme
            .chars()
            .all(|character| matches!(character, '\r' | '\n'))
        {
            continue;
        }
        if coverage_count(cells, start, start + grapheme.len()) != 1 {
            return Err(TextNavigationError::UncoveredGrapheme);
        }
    }
    Ok(())
}

fn validate_cell_unions(cells: &[GraphemeCell]) -> Result<(), TextNavigationError> {
    let mut spans = cells
        .iter()
        .filter(|cell| cell.right > cell.left)
        .map(|cell| (cell.visual_line, cell.left, cell.right))
        .collect::<Vec<_>>();
    spans.sort_by(|left, right| {
        left.0
            .cmp(&right.0)
            .then_with(|| left.1.total_cmp(&right.1))
            .then_with(|| left.2.total_cmp(&right.2))
    });

    let mut merged = Vec::<(usize, f32, f32)>::new();
    for (visual_line, left, right) in spans {
        if let Some((last_line, last_left, last_right)) = merged.last_mut()
            && *last_line == visual_line
            && visual_gap(left, *last_right) <= f64::from(SHAPED_TEXT_GEOMETRY_EPSILON)
        {
            *last_right = (*last_right).max(right);
            if !(*last_right - *last_left).is_finite() {
                return Err(TextNavigationError::InvalidGlyphGeometry);
            }
        } else {
            merged.push((visual_line, left, right));
        }
    }
    Ok(())
}

fn coverage_count(cells: &[GraphemeCell], start: usize, end: usize) -> usize {
    cells
        .iter()
        .filter(|cell| cell.start == start && cell.end == end)
        .count()
}

fn coordinate_nodes(
    source_len: usize,
    lines: &[NavigationLine],
    cells: &[GraphemeCell],
) -> Vec<CaretNode> {
    let mut edges = Vec::with_capacity(cells.len() * 2 + lines.len() * 2);
    for cell in cells {
        edges.push(Edge {
            visual_line: cell.visual_line,
            offset: cell.start,
            x: if cell.rtl { cell.right } else { cell.left },
            affinity: TextAffinity::After,
        });
        edges.push(Edge {
            visual_line: cell.visual_line,
            offset: cell.end,
            x: if cell.rtl { cell.left } else { cell.right },
            affinity: TextAffinity::Before,
        });
    }
    for line in lines.iter().filter(|line| {
        line.text_start == line.text_end
            && !cells
                .iter()
                .any(|cell| cell.visual_line == line.visual_index)
    }) {
        edges.push(Edge {
            visual_line: line.visual_index,
            offset: line.text_start,
            x: 0.0,
            affinity: TextAffinity::After,
        });
        edges.push(Edge {
            visual_line: line.visual_index,
            offset: line.text_start,
            x: 0.0,
            affinity: TextAffinity::Before,
        });
    }
    group_edges(&mut edges, source_len)
}

fn group_edges(edges: &mut [Edge], source_len: usize) -> Vec<CaretNode> {
    edges.sort_by(|left, right| {
        left.visual_line
            .cmp(&right.visual_line)
            .then_with(|| left.offset.cmp(&right.offset))
            .then_with(|| left.x.total_cmp(&right.x))
            .then_with(|| affinity_order(left.affinity).cmp(&affinity_order(right.affinity)))
    });

    let mut nodes = Vec::new();
    let mut index = 0;
    while index < edges.len() {
        let first = edges[index];
        let mut node = CaretNode {
            visual_line: first.visual_line,
            offset: first.offset,
            x: first.x,
            has_before: first.affinity == TextAffinity::Before,
            has_after: first.affinity == TextAffinity::After,
        };
        index += 1;
        while let Some(edge) = edges.get(index)
            && edge.visual_line == node.visual_line
            && edge.offset == node.offset
            && edge.x - node.x <= SHAPED_TEXT_GEOMETRY_EPSILON
        {
            node.has_before |= edge.affinity == TextAffinity::Before;
            node.has_after |= edge.affinity == TextAffinity::After;
            index += 1;
        }
        nodes.push(node);
    }
    nodes.sort_by(|left, right| {
        left.visual_line
            .cmp(&right.visual_line)
            .then_with(|| left.x.total_cmp(&right.x))
            .then_with(|| left.offset.cmp(&right.offset))
            .then_with(|| {
                affinity_order(left.canonical_caret(source_len).affinity)
                    .cmp(&affinity_order(right.canonical_caret(source_len).affinity))
            })
    });
    nodes
}

fn word_targets(source: &str) -> BTreeSet<usize> {
    grapheme_boundaries(source)
        .into_iter()
        .flat_map(|boundary| {
            [
                previous_word_boundary(source, boundary),
                next_word_boundary(source, boundary),
            ]
        })
        .collect()
}

fn grapheme_boundaries(source: &str) -> Vec<usize> {
    source
        .grapheme_indices(true)
        .map(|(index, _)| index)
        .chain(std::iter::once(source.len()))
        .collect()
}

fn line_distance(line: &NavigationLine, y: f32) -> f64 {
    let y = f64::from(y);
    let top = f64::from(line.top_y);
    let bottom = top + f64::from(line.height);
    if y < top {
        top - y
    } else if y > bottom {
        y - bottom
    } else {
        0.0
    }
}

fn visual_gap(left: f32, right: f32) -> f64 {
    f64::from(left) - f64::from(right)
}

fn point_distance(left: f32, right: f32) -> f64 {
    (f64::from(left) - f64::from(right)).abs()
}

const fn affinity_order(affinity: TextAffinity) -> u8 {
    match affinity {
        TextAffinity::After => 0,
        TextAffinity::Before => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edge_grouping_uses_bounded_diameter_instead_of_epsilon_chaining() {
        let mut edges = [
            Edge {
                visual_line: 0,
                offset: 1,
                x: 0.0,
                affinity: TextAffinity::After,
            },
            Edge {
                visual_line: 0,
                offset: 1,
                x: SHAPED_TEXT_GEOMETRY_EPSILON * 0.75,
                affinity: TextAffinity::Before,
            },
            Edge {
                visual_line: 0,
                offset: 1,
                x: SHAPED_TEXT_GEOMETRY_EPSILON * 1.5,
                affinity: TextAffinity::After,
            },
        ];

        let nodes = group_edges(&mut edges, 2);

        assert_eq!(nodes.len(), 2);
        assert!(nodes[0].x.abs() <= f32::EPSILON);
        assert!(nodes[0].has_after);
        assert!(nodes[0].has_before);
        assert!((nodes[1].x - SHAPED_TEXT_GEOMETRY_EPSILON * 1.5).abs() <= f32::EPSILON);
        assert!(nodes[1].has_after);
        assert!(!nodes[1].has_before);
    }
}
