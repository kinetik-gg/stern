use super::{
    Axis, DEFAULT_SPLITTER_THICKNESS, DROP_EDGE_FRACTION, Dock, DockChromeStyle, DockDropTarget,
    DockDropZone, DockInteractionPolicy, DockNode, DockPathElement, DockPlacement, DockSplitPath,
    DockSplitter, DockSplitterSide, FrameId, Ordering, Point, Rect, Vec2,
};

/// Resolved frame rectangle.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrameLayout {
    /// Frame identity.
    pub frame: FrameId,
    /// Frame rectangle.
    pub rect: Rect,
}

/// Cardinal direction for dock frame neighbor lookup.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DockNeighborDirection {
    /// Candidate frame is to the left of the source frame.
    Left,
    /// Candidate frame is to the right of the source frame.
    Right,
    /// Candidate frame is above the source frame.
    Up,
    /// Candidate frame is below the source frame.
    Down,
}

/// Resolved neighboring frames for one dock frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameNeighbors {
    /// Source frame.
    pub frame: FrameId,
    /// Best neighboring frame to the left.
    pub left: Option<FrameId>,
    /// Best neighboring frame to the right.
    pub right: Option<FrameId>,
    /// Best neighboring frame above.
    pub up: Option<FrameId>,
    /// Best neighboring frame below.
    pub down: Option<FrameId>,
}

impl FrameNeighbors {
    /// Creates empty neighbor data for a frame.
    #[must_use]
    pub const fn empty(frame: FrameId) -> Self {
        Self {
            frame,
            left: None,
            right: None,
            up: None,
            down: None,
        }
    }

    /// Returns the neighbor for a cardinal direction.
    #[must_use]
    pub const fn neighbor(self, direction: DockNeighborDirection) -> Option<FrameId> {
        match direction {
            DockNeighborDirection::Left => self.left,
            DockNeighborDirection::Right => self.right,
            DockNeighborDirection::Up => self.up,
            DockNeighborDirection::Down => self.down,
        }
    }
}

/// Resolves a dock tree into frame rectangles.
#[must_use]
pub fn solve_dock_layout(area: &Dock, bounds: Rect) -> Vec<FrameLayout> {
    let mut frames = Vec::new();
    solve_node(&area.root, bounds, &mut frames);
    frames
}

/// Resolves cardinal frame neighbors from solved dock layout rectangles.
///
/// Ties are deterministic: nearest edge distance wins, then greatest
/// perpendicular edge overlap, then the lowest raw [`FrameId`].
#[must_use]
pub fn solve_dock_neighbors(area: &Dock, bounds: Rect) -> Vec<FrameNeighbors> {
    let frames = solve_dock_layout(area, bounds);
    frames
        .iter()
        .map(|layout| FrameNeighbors {
            frame: layout.frame,
            left: frame_neighbor(&frames, layout.frame, DockNeighborDirection::Left),
            right: frame_neighbor(&frames, layout.frame, DockNeighborDirection::Right),
            up: frame_neighbor(&frames, layout.frame, DockNeighborDirection::Up),
            down: frame_neighbor(&frames, layout.frame, DockNeighborDirection::Down),
        })
        .collect()
}

/// Resolves the best neighbor for one frame from solved dock layout rectangles.
///
/// Invalid or empty source/candidate rectangles are ignored. The source frame
/// is never returned as its own neighbor.
#[must_use]
pub fn frame_neighbor(
    frames: &[FrameLayout],
    frame: FrameId,
    direction: DockNeighborDirection,
) -> Option<FrameId> {
    let source = frames.iter().find(|layout| layout.frame == frame)?;
    if !valid_neighbor_rect(source.rect) {
        return None;
    }

    let mut best = None;
    for candidate in frames {
        if candidate.frame == frame || !valid_neighbor_rect(candidate.rect) {
            continue;
        }
        let Some(score) = neighbor_candidate_score(source.rect, candidate.rect, direction) else {
            continue;
        };
        if neighbor_candidate_is_better(best, (candidate.frame, score)) {
            best = Some((candidate.frame, score));
        }
    }

    best.map(|(frame, _)| frame)
}

/// Resolves splitter interaction rectangles for a dock tree.
#[must_use]
pub fn solve_dock_splitters(area: &Dock, bounds: Rect, thickness: f32) -> Vec<DockSplitter> {
    solve_dock_splitters_with_style(
        area,
        bounds,
        DockChromeStyle::default().with_splitter_hit_thickness(thickness),
    )
}

/// Resolves splitter interaction rectangles using dock chrome style.
#[must_use]
pub fn solve_dock_splitters_with_style(
    area: &Dock,
    bounds: Rect,
    style: DockChromeStyle,
) -> Vec<DockSplitter> {
    let mut splitters = Vec::new();
    solve_splitters(
        &area.root,
        bounds,
        style.sanitized().splitter_hit_thickness,
        &DockSplitPath::root(),
        &mut splitters,
    );
    splitters
}

fn solve_splitters(
    node: &DockNode,
    bounds: Rect,
    thickness: f32,
    path: &DockSplitPath,
    splitters: &mut Vec<DockSplitter>,
) {
    let DockNode::Split {
        axis,
        ratio,
        min_first,
        min_second,
        first,
        second,
    } = node
    else {
        return;
    };

    let (first_rect, second_rect) =
        split_child_rects(*axis, bounds, *ratio, *min_first, *min_second);
    splitters.push(DockSplitter {
        path: path.clone(),
        axis: *axis,
        rect: splitter_rect(*axis, first_rect, bounds, thickness),
        ratio: finite_ratio(*ratio),
        min_first: finite_non_negative(*min_first),
        min_second: finite_non_negative(*min_second),
    });
    solve_splitters(
        first,
        first_rect,
        thickness,
        &path.child(DockPathElement::First),
        splitters,
    );
    solve_splitters(
        second,
        second_rect,
        thickness,
        &path.child(DockPathElement::Second),
        splitters,
    );
}

fn solve_node(node: &DockNode, bounds: Rect, frames: &mut Vec<FrameLayout>) {
    match node {
        DockNode::Frame(frame) => frames.push(FrameLayout {
            frame: frame.id,
            rect: bounds,
        }),
        DockNode::Split {
            axis,
            ratio,
            min_first,
            min_second,
            first,
            second,
        } => {
            let (first_rect, second_rect) =
                split_child_rects(*axis, bounds, *ratio, *min_first, *min_second);
            solve_node(first, first_rect, frames);
            solve_node(second, second_rect, frames);
        }
    }
}

pub(crate) fn split_child_rects(
    axis: Axis,
    bounds: Rect,
    ratio: f32,
    min_first: f32,
    min_second: f32,
) -> (Rect, Rect) {
    let total = split_total(axis, bounds);
    let first_size = split_first_size(total, ratio, min_first, min_second);
    let second_size = (total - first_size).max(0.0);
    let x = finite_coordinate(bounds.x);
    let y = finite_coordinate(bounds.y);
    let width = finite_non_negative(bounds.width);
    let height = finite_non_negative(bounds.height);
    match axis {
        Axis::Horizontal => (
            Rect::new(x, y, first_size, height),
            Rect::new(x + first_size, y, second_size, height),
        ),
        Axis::Vertical => (
            Rect::new(x, y, width, first_size),
            Rect::new(x, y + first_size, width, second_size),
        ),
    }
}

fn split_total(axis: Axis, bounds: Rect) -> f32 {
    match axis {
        Axis::Horizontal => finite_non_negative(bounds.width),
        Axis::Vertical => finite_non_negative(bounds.height),
    }
}

fn split_first_size(total: f32, ratio: f32, min_first: f32, min_second: f32) -> f32 {
    let total = finite_non_negative(total);
    let min_first = finite_non_negative(min_first);
    let min_second = finite_non_negative(min_second);
    let desired = total * finite_ratio(ratio);
    if total >= min_first + min_second {
        desired.clamp(min_first, total - min_second)
    } else {
        desired.max(min_first.min(total)).min(total)
    }
}

fn splitter_rect(axis: Axis, first_rect: Rect, bounds: Rect, thickness: f32) -> Rect {
    let half = thickness * 0.5;
    let x = finite_coordinate(bounds.x);
    let y = finite_coordinate(bounds.y);
    let width = finite_non_negative(bounds.width);
    let height = finite_non_negative(bounds.height);
    match axis {
        Axis::Horizontal => Rect::new(
            finite_coordinate(first_rect.max_x()) - half,
            y,
            thickness,
            height,
        ),
        Axis::Vertical => Rect::new(
            x,
            finite_coordinate(first_rect.max_y()) - half,
            width,
            thickness,
        ),
    }
}

pub(crate) fn splitter_thickness(thickness: f32) -> f32 {
    if thickness.is_finite() && thickness > 0.0 {
        thickness
    } else {
        DEFAULT_SPLITTER_THICKNESS
    }
}

pub(crate) fn sanitize_drop_edge_fraction(fraction: f32) -> f32 {
    if fraction.is_finite() {
        fraction.clamp(0.0, 0.5)
    } else {
        DROP_EDGE_FRACTION
    }
}

/// Maps a splitter drag delta to a clamped split ratio.
#[must_use]
pub fn split_ratio_from_drag(
    axis: Axis,
    bounds: Rect,
    ratio: f32,
    min_first: f32,
    min_second: f32,
    delta: Vec2,
) -> f32 {
    let total = split_total(axis, bounds);
    if total <= 0.0 {
        return finite_ratio(ratio);
    }
    let delta = match axis {
        Axis::Horizontal => delta.x,
        Axis::Vertical => delta.y,
    };
    let current = split_first_size(total, ratio, min_first, min_second);
    let desired = current + if delta.is_finite() { delta } else { 0.0 };
    split_first_size(total, desired / total, min_first, min_second) / total
}

/// Resolves a frame-local drop zone for a pointer position.
#[must_use]
pub fn resolve_frame_drop_zone(rect: Rect, point: Point) -> Option<DockDropZone> {
    resolve_frame_drop_zone_with_policy(rect, point, DockInteractionPolicy::default())
}

/// Resolves a frame-local drop zone using dock interaction policy.
#[must_use]
pub fn resolve_frame_drop_zone_with_policy(
    rect: Rect,
    point: Point,
    policy: DockInteractionPolicy,
) -> Option<DockDropZone> {
    if !valid_drop_rect(rect) || !valid_drop_point(point) {
        return None;
    }

    if !rect.contains_point(point) {
        return None;
    }

    let left = (point.x - rect.min_x()).max(0.0);
    let right = (rect.max_x() - point.x).max(0.0);
    let top = (point.y - rect.min_y()).max(0.0);
    let bottom = (rect.max_y() - point.y).max(0.0);
    let edge_fraction = policy.sanitized().drop_targets.edge_fraction;
    let edge_x = finite_non_negative(rect.width) * edge_fraction;
    let edge_y = finite_non_negative(rect.height) * edge_fraction;

    let mut best = (DockDropZone::Center, f32::INFINITY);
    for (zone, distance, limit) in [
        (DockDropZone::Left, left, edge_x),
        (DockDropZone::Right, right, edge_x),
        (DockDropZone::Top, top, edge_y),
        (DockDropZone::Bottom, bottom, edge_y),
    ] {
        if distance <= limit && distance < best.1 {
            best = (zone, distance);
        }
    }

    Some(best.0)
}

/// Resolves a dock drop target from frame layout and a pointer position.
#[must_use]
pub fn resolve_dock_drop_target(
    frames: &[FrameLayout],
    point: Point,
    new_frame: FrameId,
) -> Option<DockDropTarget> {
    resolve_dock_drop_target_with_policy(frames, point, new_frame, DockInteractionPolicy::default())
}

/// Resolves a dock drop target using dock interaction policy.
#[must_use]
pub fn resolve_dock_drop_target_with_policy(
    frames: &[FrameLayout],
    point: Point,
    new_frame: FrameId,
    policy: DockInteractionPolicy,
) -> Option<DockDropTarget> {
    let policy = policy.sanitized();
    frames.iter().find_map(|layout| {
        let zone = resolve_frame_drop_zone_with_policy(layout.rect, point, policy)?;
        match zone.placement() {
            Some(placement) if policy.drop_targets.allow_split_insertion => {
                Some(DockDropTarget::split(layout.frame, placement, new_frame))
            }
            None if policy.drop_targets.allow_tab_merge => Some(DockDropTarget::tab(layout.frame)),
            _ => None,
        }
    })
}

pub(crate) fn resolve_frame_split_affordance_with_policy(
    frames: &[FrameLayout],
    point: Point,
    policy: DockInteractionPolicy,
) -> Option<(FrameId, DockPlacement)> {
    for layout in frames {
        let Some(zone) = resolve_frame_drop_zone_with_policy(layout.rect, point, policy) else {
            continue;
        };
        return policy
            .sanitized()
            .drop_targets
            .allow_split_insertion
            .then_some(zone)
            .and_then(DockDropZone::placement)
            .map(|placement| (layout.frame, placement));
    }

    None
}

pub(crate) fn splitter_adjacent_frame(
    frames: &[FrameLayout],
    candidates: &[FrameId],
    splitter: &DockSplitter,
    side: DockSplitterSide,
) -> Option<FrameId> {
    if !valid_neighbor_rect(splitter.rect) {
        return None;
    }

    let mut best = None;
    for candidate in candidates {
        let Some(layout) = frames.iter().find(|layout| layout.frame == *candidate) else {
            continue;
        };
        if !valid_neighbor_rect(layout.rect) {
            continue;
        }
        let Some(score) = splitter_adjacent_score(layout.rect, splitter, side) else {
            continue;
        };
        if splitter_adjacent_is_better(best, (*candidate, score)) {
            best = Some((*candidate, score));
        }
    }

    best.map(|(frame, _)| frame)
}

#[derive(Debug, Clone, Copy)]
struct SplitterAdjacentScore {
    overlap: f32,
    distance: f32,
}

fn splitter_adjacent_score(
    rect: Rect,
    splitter: &DockSplitter,
    side: DockSplitterSide,
) -> Option<SplitterAdjacentScore> {
    let center_x = splitter.rect.x + splitter.rect.width * 0.5;
    let center_y = splitter.rect.y + splitter.rect.height * 0.5;
    let (overlap, distance) = match (splitter.axis, side) {
        (Axis::Horizontal, DockSplitterSide::First) => (
            range_overlap(
                rect.min_y(),
                rect.max_y(),
                splitter.rect.min_y(),
                splitter.rect.max_y(),
            ),
            (center_x - rect.max_x()).abs(),
        ),
        (Axis::Horizontal, DockSplitterSide::Second) => (
            range_overlap(
                rect.min_y(),
                rect.max_y(),
                splitter.rect.min_y(),
                splitter.rect.max_y(),
            ),
            (rect.min_x() - center_x).abs(),
        ),
        (Axis::Vertical, DockSplitterSide::First) => (
            range_overlap(
                rect.min_x(),
                rect.max_x(),
                splitter.rect.min_x(),
                splitter.rect.max_x(),
            ),
            (center_y - rect.max_y()).abs(),
        ),
        (Axis::Vertical, DockSplitterSide::Second) => (
            range_overlap(
                rect.min_x(),
                rect.max_x(),
                splitter.rect.min_x(),
                splitter.rect.max_x(),
            ),
            (rect.min_y() - center_y).abs(),
        ),
    };

    (overlap > 0.0 && distance.is_finite()).then_some(SplitterAdjacentScore { overlap, distance })
}

fn splitter_adjacent_is_better(
    best: Option<(FrameId, SplitterAdjacentScore)>,
    candidate: (FrameId, SplitterAdjacentScore),
) -> bool {
    let Some((best_frame, best_score)) = best else {
        return true;
    };
    let (candidate_frame, candidate_score) = candidate;
    match candidate_score.distance.total_cmp(&best_score.distance) {
        Ordering::Less => true,
        Ordering::Greater => false,
        Ordering::Equal => match candidate_score.overlap.total_cmp(&best_score.overlap) {
            Ordering::Greater => true,
            Ordering::Less => false,
            Ordering::Equal => candidate_frame.raw() < best_frame.raw(),
        },
    }
}

fn valid_drop_rect(rect: Rect) -> bool {
    rect.x.is_finite()
        && rect.y.is_finite()
        && rect.width.is_finite()
        && rect.height.is_finite()
        && !rect.is_empty()
}

fn valid_drop_point(point: Point) -> bool {
    point.x.is_finite() && point.y.is_finite()
}

#[derive(Debug, Clone, Copy)]
struct NeighborCandidateScore {
    overlap: f32,
    distance: f32,
}

fn neighbor_candidate_score(
    source: Rect,
    candidate: Rect,
    direction: DockNeighborDirection,
) -> Option<NeighborCandidateScore> {
    let (overlap, distance) = match direction {
        DockNeighborDirection::Left => (
            range_overlap(
                source.min_y(),
                source.max_y(),
                candidate.min_y(),
                candidate.max_y(),
            ),
            source.min_x() - candidate.max_x(),
        ),
        DockNeighborDirection::Right => (
            range_overlap(
                source.min_y(),
                source.max_y(),
                candidate.min_y(),
                candidate.max_y(),
            ),
            candidate.min_x() - source.max_x(),
        ),
        DockNeighborDirection::Up => (
            range_overlap(
                source.min_x(),
                source.max_x(),
                candidate.min_x(),
                candidate.max_x(),
            ),
            source.min_y() - candidate.max_y(),
        ),
        DockNeighborDirection::Down => (
            range_overlap(
                source.min_x(),
                source.max_x(),
                candidate.min_x(),
                candidate.max_x(),
            ),
            candidate.min_y() - source.max_y(),
        ),
    };

    (overlap > 0.0 && distance >= 0.0).then_some(NeighborCandidateScore { overlap, distance })
}

fn neighbor_candidate_is_better(
    best: Option<(FrameId, NeighborCandidateScore)>,
    candidate: (FrameId, NeighborCandidateScore),
) -> bool {
    let Some((best_frame, best_score)) = best else {
        return true;
    };
    let (candidate_frame, candidate_score) = candidate;
    match candidate_score.distance.total_cmp(&best_score.distance) {
        Ordering::Less => true,
        Ordering::Greater => false,
        Ordering::Equal => match candidate_score.overlap.total_cmp(&best_score.overlap) {
            Ordering::Greater => true,
            Ordering::Less => false,
            Ordering::Equal => candidate_frame.raw() < best_frame.raw(),
        },
    }
}

fn range_overlap(first_min: f32, first_max: f32, second_min: f32, second_max: f32) -> f32 {
    first_max.min(second_max) - first_min.max(second_min)
}

fn valid_neighbor_rect(rect: Rect) -> bool {
    rect.x.is_finite()
        && rect.y.is_finite()
        && rect.width.is_finite()
        && rect.height.is_finite()
        && !rect.is_empty()
}

fn finite_ratio(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        0.5
    }
}

fn finite_non_negative(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}

fn finite_coordinate(value: f32) -> f32 {
    if value.is_finite() { value } else { 0.0 }
}
