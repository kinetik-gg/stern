//! Frame-local layout tree with measure and arrange passes.
//!
//! This module implements Phase L0 of RFC 0001 (`docs/rfcs/0001-layout-engine.md`):
//! a frame-local tree of layout nodes solved in two deterministic passes:
//!
//! 1. **Measure (bottom-up).** Each node reports its intrinsic size under a
//!    given available size. Leaves answer through a [`LeafMeasure`] measurer;
//!    containers combine children per their [`SizeRule`]s.
//! 2. **Arrange (top-down).** Parents allocate final rectangles using the same
//!    distribution arithmetic as [`row_layout`](super::row_layout),
//!    [`column_layout`](super::column_layout), and
//!    [`grid_layout`], whose behavior is pinned by the
//!    existing kernel tests.
//!
//! The tree is rebuilt per frame; the only retained state is the
//! [`MeasureCache`], which stores small `(constraint key, Measurement)` pairs
//! keyed by [`WidgetId`] and follows the staged commit pattern used by scroll
//! offsets in `UiMemory`.
//!
//! This API is public but not yet stable; no widget consumes it until Phase L1.

use std::collections::HashMap;

use crate::{Rect, Size, WidgetId};

use super::{
    Alignment, Axis, Insets, LayoutItem, Measurement, SizeDimension, SizeRule, fit_box,
    grid_layout, linear_layout, pad_rect, sanitize_insets, sanitize_rect, sanitize_size,
};

/// Handle to a node inside a [`LayoutTree`].
///
/// Node ids are only meaningful for the tree that created them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LayoutNodeId(usize);

/// Retained identity for a cached leaf measurement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MeasureIdentity {
    /// Widget owning the cached measurement.
    pub widget: WidgetId,
    /// Revision of the content and style inputs that affect measurement.
    ///
    /// Callers bump this value whenever text content, style tokens, or any
    /// other measurement input changes; a stale revision misses the cache.
    pub content_revision: u64,
}

impl MeasureIdentity {
    /// Creates a measure identity.
    #[must_use]
    pub const fn new(widget: WidgetId, content_revision: u64) -> Self {
        Self {
            widget,
            content_revision,
        }
    }
}

/// Content measurer carried by a leaf node.
pub trait LeafMeasure {
    /// Returns the intrinsic measurement under the given available size.
    fn measure(&self, available: Size) -> Measurement;
}

impl<F: Fn(Size) -> Measurement> LeafMeasure for F {
    fn measure(&self, available: Size) -> Measurement {
        self(available)
    }
}

/// A measurer that always returns a fixed measurement.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FixedMeasure(Measurement);

impl FixedMeasure {
    /// Creates a measurer returning `measurement` regardless of availability.
    #[must_use]
    pub const fn new(measurement: Measurement) -> Self {
        Self(measurement)
    }
}

impl LeafMeasure for FixedMeasure {
    fn measure(&self, _available: Size) -> Measurement {
        self.0
    }
}

enum NodeKind {
    Leaf {
        measurer: Box<dyn LeafMeasure>,
        identity: Option<MeasureIdentity>,
    },
    Row {
        spacing: f32,
    },
    Column {
        spacing: f32,
    },
    Stack,
    Grid {
        columns: Vec<SizeRule>,
        rows: Vec<SizeRule>,
        column_spacing: f32,
        row_spacing: f32,
    },
    Padding {
        insets: Insets,
    },
    Align {
        horizontal: Alignment,
        vertical: Alignment,
    },
}

struct Node {
    kind: NodeKind,
    width: SizeRule,
    height: SizeRule,
    children: Vec<LayoutNodeId>,
}

/// A frame-local tree of layout nodes over an index arena.
///
/// Nodes are created children-first; containers receive the ids of already
/// created children. The tree is intended to be built, solved, and discarded
/// within a single frame.
#[derive(Default)]
pub struct LayoutTree {
    nodes: Vec<Node>,
    root: Option<LayoutNodeId>,
}

impl std::fmt::Debug for LayoutTree {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LayoutTree")
            .field("nodes", &self.nodes.len())
            .field("root", &self.root)
            .finish()
    }
}

impl LayoutTree {
    /// Creates an empty tree.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the number of nodes in the tree.
    #[must_use]
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Returns whether the tree has no nodes.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Returns the root node, if one was set.
    #[must_use]
    pub const fn root(&self) -> Option<LayoutNodeId> {
        self.root
    }

    /// Sets the root node solved against the bounds passed to [`Self::solve`].
    ///
    /// # Panics
    ///
    /// Panics if `id` does not belong to this tree.
    pub fn set_root(&mut self, id: LayoutNodeId) {
        assert!(id.0 < self.nodes.len(), "layout node id out of bounds");
        self.root = Some(id);
    }

    fn push(&mut self, node: Node) -> LayoutNodeId {
        let id = LayoutNodeId(self.nodes.len());
        self.nodes.push(node);
        id
    }

    fn push_container(
        &mut self,
        kind: NodeKind,
        width: SizeRule,
        height: SizeRule,
        children: Vec<LayoutNodeId>,
    ) -> LayoutNodeId {
        for child in &children {
            assert!(child.0 < self.nodes.len(), "layout child id out of bounds");
        }
        self.push(Node {
            kind,
            width,
            height,
            children,
        })
    }

    /// Adds an uncached leaf node.
    pub fn leaf(
        &mut self,
        width: SizeRule,
        height: SizeRule,
        measurer: impl LeafMeasure + 'static,
    ) -> LayoutNodeId {
        self.push(Node {
            kind: NodeKind::Leaf {
                measurer: Box::new(measurer),
                identity: None,
            },
            width,
            height,
            children: Vec::new(),
        })
    }

    /// Adds a leaf node whose measurement is cached under `identity`.
    pub fn cached_leaf(
        &mut self,
        width: SizeRule,
        height: SizeRule,
        measurer: impl LeafMeasure + 'static,
        identity: MeasureIdentity,
    ) -> LayoutNodeId {
        self.push(Node {
            kind: NodeKind::Leaf {
                measurer: Box::new(measurer),
                identity: Some(identity),
            },
            width,
            height,
            children: Vec::new(),
        })
    }

    /// Adds a row container laying out `children` horizontally.
    ///
    /// # Panics
    ///
    /// Panics if any child id does not belong to this tree.
    pub fn row(
        &mut self,
        width: SizeRule,
        height: SizeRule,
        spacing: f32,
        children: Vec<LayoutNodeId>,
    ) -> LayoutNodeId {
        self.push_container(NodeKind::Row { spacing }, width, height, children)
    }

    /// Adds a column container laying out `children` vertically.
    ///
    /// # Panics
    ///
    /// Panics if any child id does not belong to this tree.
    pub fn column(
        &mut self,
        width: SizeRule,
        height: SizeRule,
        spacing: f32,
        children: Vec<LayoutNodeId>,
    ) -> LayoutNodeId {
        self.push_container(NodeKind::Column { spacing }, width, height, children)
    }

    /// Adds a stack container overlaying `children` at its own origin.
    ///
    /// Each child resolves its own size rules against the stack rectangle and
    /// is placed at the stack origin; children may overflow the stack the same
    /// way `Fit` children overflow a row. Use [`Self::align`] around a child
    /// for placement other than the origin.
    ///
    /// # Panics
    ///
    /// Panics if any child id does not belong to this tree.
    pub fn stack(
        &mut self,
        width: SizeRule,
        height: SizeRule,
        children: Vec<LayoutNodeId>,
    ) -> LayoutNodeId {
        self.push_container(NodeKind::Stack, width, height, children)
    }

    /// Adds a grid container placing `children` in row-major cells.
    ///
    /// Track sizing follows [`grid_layout`]; children fill
    /// their cells. Children beyond the grid's capacity receive zero rects.
    ///
    /// # Panics
    ///
    /// Panics if any child id does not belong to this tree.
    #[allow(clippy::too_many_arguments)]
    pub fn grid(
        &mut self,
        width: SizeRule,
        height: SizeRule,
        columns: Vec<SizeRule>,
        rows: Vec<SizeRule>,
        column_spacing: f32,
        row_spacing: f32,
        children: Vec<LayoutNodeId>,
    ) -> LayoutNodeId {
        self.push_container(
            NodeKind::Grid {
                columns,
                rows,
                column_spacing,
                row_spacing,
            },
            width,
            height,
            children,
        )
    }

    /// Adds a padding container around a single child.
    ///
    /// # Panics
    ///
    /// Panics if `child` does not belong to this tree.
    pub fn padding(
        &mut self,
        width: SizeRule,
        height: SizeRule,
        insets: Insets,
        child: LayoutNodeId,
    ) -> LayoutNodeId {
        self.push_container(NodeKind::Padding { insets }, width, height, vec![child])
    }

    /// Adds an alignment container around a single child.
    ///
    /// The child resolves its own size rules against the container rectangle
    /// and is aligned inside it with [`fit_box`] semantics,
    /// which clamp the child to the container.
    ///
    /// # Panics
    ///
    /// Panics if `child` does not belong to this tree.
    pub fn align(
        &mut self,
        width: SizeRule,
        height: SizeRule,
        horizontal: Alignment,
        vertical: Alignment,
        child: LayoutNodeId,
    ) -> LayoutNodeId {
        self.push_container(
            NodeKind::Align {
                horizontal,
                vertical,
            },
            width,
            height,
            vec![child],
        )
    }

    /// Solves the tree against `bounds` without a measure cache.
    #[must_use]
    pub fn solve(&self, bounds: Rect) -> LayoutSolution {
        self.solve_inner(bounds, None)
    }

    /// Solves the tree against `bounds`, consulting and staging `cache`.
    ///
    /// Cache reads see entries staged earlier in the same frame as well as
    /// entries committed by [`MeasureCache::commit`]. Fresh measurements are
    /// staged, never committed; committing is the frame boundary's job.
    #[must_use]
    pub fn solve_cached(&self, bounds: Rect, cache: &mut MeasureCache) -> LayoutSolution {
        self.solve_inner(bounds, Some(cache))
    }

    fn solve_inner(&self, bounds: Rect, cache: Option<&mut MeasureCache>) -> LayoutSolution {
        let bounds = sanitize_rect(bounds);
        let mut solver = Solver {
            tree: self,
            cache,
            measurements: vec![Measurement::default(); self.nodes.len()],
            rects: vec![Rect::ZERO; self.nodes.len()],
        };

        if let Some(root) = self.root {
            solver.measure(root, bounds.size());
            let size = solver.resolve_node_size(root, bounds.size());
            solver.arrange(root, Rect::new(bounds.x, bounds.y, size.width, size.height));
        }

        LayoutSolution {
            measurements: solver.measurements,
            rects: solver.rects,
        }
    }
}

/// Solved rectangles and measurements for every node of a [`LayoutTree`].
#[derive(Debug, Clone, PartialEq)]
pub struct LayoutSolution {
    measurements: Vec<Measurement>,
    rects: Vec<Rect>,
}

impl LayoutSolution {
    /// Returns the final rectangle solved for `id`.
    ///
    /// Nodes that were not reachable from the root keep [`Rect::ZERO`].
    ///
    /// # Panics
    ///
    /// Panics if `id` does not belong to the solved tree.
    #[must_use]
    pub fn rect(&self, id: LayoutNodeId) -> Rect {
        self.rects[id.0]
    }

    /// Returns the intrinsic measurement recorded for `id`.
    ///
    /// # Panics
    ///
    /// Panics if `id` does not belong to the solved tree.
    #[must_use]
    pub fn measurement(&self, id: LayoutNodeId) -> Measurement {
        self.measurements[id.0]
    }
}

struct Solver<'a> {
    tree: &'a LayoutTree,
    cache: Option<&'a mut MeasureCache>,
    measurements: Vec<Measurement>,
    rects: Vec<Rect>,
}

impl<'a> Solver<'a> {
    fn node(&self, id: LayoutNodeId) -> &'a Node {
        &self.tree.nodes[id.0]
    }

    /// Bottom-up pass: records the intrinsic measurement of `id` under
    /// `available` and returns it.
    fn measure(&mut self, id: LayoutNodeId, available: Size) -> Measurement {
        let available = sanitize_available(available);
        let node = self.node(id);
        let measured = match &node.kind {
            NodeKind::Leaf { measurer, identity } => {
                self.measure_leaf(measurer.as_ref(), *identity, available)
            }
            NodeKind::Row { spacing } => {
                self.measure_linear(Axis::Horizontal, *spacing, &node.children, available)
            }
            NodeKind::Column { spacing } => {
                self.measure_linear(Axis::Vertical, *spacing, &node.children, available)
            }
            NodeKind::Stack => self.measure_stack(&node.children, available),
            NodeKind::Grid {
                columns,
                rows,
                column_spacing,
                row_spacing,
            } => self.measure_grid(
                columns,
                rows,
                *column_spacing,
                *row_spacing,
                &node.children,
                available,
            ),
            NodeKind::Padding { insets } => {
                let insets = sanitize_insets(*insets);
                let child = node.children[0];
                let inner = Size::new(
                    (available.width - insets.left - insets.right).max(0.0),
                    (available.height - insets.top - insets.bottom).max(0.0),
                );
                self.measure(child, inner);
                let contribution = self.child_contribution(child, inner);
                Measurement::new(Size::new(
                    contribution.width + insets.left + insets.right,
                    contribution.height + insets.top + insets.bottom,
                ))
            }
            NodeKind::Align { .. } => {
                let child = node.children[0];
                self.measure(child, available);
                Measurement::new(self.child_contribution(child, available))
            }
        };

        let measured = sanitize_measurement(measured);
        self.measurements[id.0] = measured;
        measured
    }

    fn measure_leaf(
        &mut self,
        measurer: &dyn LeafMeasure,
        identity: Option<MeasureIdentity>,
        available: Size,
    ) -> Measurement {
        let Some(identity) = identity else {
            return measurer.measure(available);
        };
        let Some(cache) = self.cache.as_deref_mut() else {
            return measurer.measure(available);
        };

        let key = constraint_key(identity.content_revision, available);
        if let Some(measurement) = cache.lookup(identity.widget, key) {
            return measurement;
        }
        let measurement = sanitize_measurement(measurer.measure(available));
        cache.stage(identity.widget, key, measurement);
        measurement
    }

    /// A child's size contribution to its parent's intrinsic measurement.
    ///
    /// `Fill` contributes the child's own intrinsic size — the fill share is
    /// unknowable until arrange — while every other rule resolves normally.
    fn child_contribution(&self, id: LayoutNodeId, available: Size) -> Size {
        let node = self.node(id);
        let measured = self.measurements[id.0].desired;
        let width = if node.width == SizeRule::Fill {
            measured.width
        } else {
            node.width
                .resolve(available.width, measured.width, available.height)
        };
        let height = if node.height == SizeRule::Fill {
            measured.height
        } else {
            resolve_height(node.height, available.height, measured.height, width)
        };
        Size::new(sanitize_size(width), sanitize_size(height))
    }

    #[allow(clippy::cast_precision_loss)]
    fn measure_linear(
        &mut self,
        axis: Axis,
        spacing: f32,
        children: &[LayoutNodeId],
        available: Size,
    ) -> Measurement {
        let spacing = sanitize_size(spacing);
        let total_spacing = spacing * children.len().saturating_sub(1) as f32;
        let child_available = match axis {
            Axis::Horizontal => {
                Size::new((available.width - total_spacing).max(0.0), available.height)
            }
            Axis::Vertical => {
                Size::new(available.width, (available.height - total_spacing).max(0.0))
            }
        };

        let mut main = 0.0_f32;
        let mut cross = 0.0_f32;
        for child in children {
            self.measure(*child, child_available);
            let contribution = self.child_contribution(*child, child_available);
            match axis {
                Axis::Horizontal => {
                    main += contribution.width;
                    cross = cross.max(contribution.height);
                }
                Axis::Vertical => {
                    main += contribution.height;
                    cross = cross.max(contribution.width);
                }
            }
        }
        main += total_spacing;

        match axis {
            Axis::Horizontal => Measurement::new(Size::new(main, cross)),
            Axis::Vertical => Measurement::new(Size::new(cross, main)),
        }
    }

    fn measure_stack(&mut self, children: &[LayoutNodeId], available: Size) -> Measurement {
        let mut size = Size::ZERO;
        for child in children {
            self.measure(*child, available);
            let contribution = self.child_contribution(*child, available);
            size.width = size.width.max(contribution.width);
            size.height = size.height.max(contribution.height);
        }
        Measurement::new(size)
    }

    fn measure_grid(
        &mut self,
        columns: &[SizeRule],
        rows: &[SizeRule],
        column_spacing: f32,
        row_spacing: f32,
        children: &[LayoutNodeId],
        available: Size,
    ) -> Measurement {
        if columns.is_empty() || rows.is_empty() || children.is_empty() {
            for child in children {
                self.measure(*child, available);
            }
            return Measurement::default();
        }

        let column_spacing = sanitize_size(column_spacing);
        let row_spacing = sanitize_size(row_spacing);
        let capacity = columns.len().saturating_mul(rows.len());
        let mut measured_columns = vec![0.0_f32; columns.len()];
        let mut measured_rows = vec![0.0_f32; rows.len()];

        for (index, child) in children.iter().enumerate() {
            self.measure(*child, available);
            if index < capacity {
                let contribution = self.child_contribution(*child, available);
                let column = index % columns.len();
                let row = index / columns.len();
                measured_columns[column] = measured_columns[column].max(contribution.width);
                measured_rows[row] = measured_rows[row].max(contribution.height);
            }
        }

        let width = track_extent(
            columns,
            &measured_columns,
            available.width,
            available.height,
            column_spacing,
        );
        let height = track_extent(
            rows,
            &measured_rows,
            available.height,
            available.width,
            row_spacing,
        );
        Measurement::new(Size::new(width, height))
    }

    /// Resolves a node's own size rules against an available size, for nodes
    /// arranged outside a linear kernel (root, stack and single-child hosts).
    fn resolve_node_size(&self, id: LayoutNodeId, available: Size) -> Size {
        let node = self.node(id);
        let measured = self.measurements[id.0].desired;
        let width = sanitize_size(node.width.resolve(
            available.width,
            measured.width,
            available.height,
        ));
        let height = sanitize_size(resolve_height(
            node.height,
            available.height,
            measured.height,
            width,
        ));
        Size::new(width, height)
    }

    /// Top-down pass: records the final rect of `id` and recurses.
    fn arrange(&mut self, id: LayoutNodeId, rect: Rect) {
        let rect = sanitize_rect(rect);
        self.rects[id.0] = rect;

        let node = self.node(id);
        match &node.kind {
            NodeKind::Leaf { .. } => {}
            NodeKind::Row { spacing } => {
                self.arrange_linear(Axis::Horizontal, *spacing, &node.children, rect);
            }
            NodeKind::Column { spacing } => {
                self.arrange_linear(Axis::Vertical, *spacing, &node.children, rect);
            }
            NodeKind::Stack => {
                for child in &node.children {
                    let size = self.resolve_node_size(*child, rect.size());
                    self.arrange(*child, Rect::new(rect.x, rect.y, size.width, size.height));
                }
            }
            NodeKind::Grid {
                columns,
                rows,
                column_spacing,
                row_spacing,
            } => {
                let measurements: Vec<Measurement> = node
                    .children
                    .iter()
                    .map(|child| self.measurements[child.0])
                    .collect();
                let cells = grid_layout(
                    rect,
                    columns,
                    rows,
                    &measurements,
                    *column_spacing,
                    *row_spacing,
                );
                let placed = cells.len();
                for (child, cell) in node.children.iter().zip(cells) {
                    self.arrange(*child, cell);
                }
                for child in node.children.iter().skip(placed) {
                    self.arrange(*child, Rect::ZERO);
                }
            }
            NodeKind::Padding { insets } => {
                let child = node.children[0];
                let content = pad_rect(rect, *insets);
                let size = self.resolve_node_size(child, content.size());
                self.arrange(
                    child,
                    Rect::new(content.x, content.y, size.width, size.height),
                );
            }
            NodeKind::Align {
                horizontal,
                vertical,
            } => {
                let child = node.children[0];
                let size = self.resolve_node_size(child, rect.size());
                self.arrange(child, fit_box(rect, size, *horizontal, *vertical));
            }
        }
    }

    fn arrange_linear(&mut self, axis: Axis, spacing: f32, children: &[LayoutNodeId], rect: Rect) {
        let items: Vec<LayoutItem> = children
            .iter()
            .map(|child| {
                let node = self.node(*child);
                LayoutItem::new(node.width, node.height, self.measurements[child.0])
            })
            .collect();
        let rects = linear_layout(axis, rect, &items, spacing);
        for (child, child_rect) in children.iter().zip(rects) {
            self.arrange(*child, child_rect);
        }
    }
}

fn sanitize_available(size: Size) -> Size {
    Size::new(sanitize_size(size.width), sanitize_size(size.height))
}

fn sanitize_measurement(measurement: Measurement) -> Measurement {
    Measurement {
        desired: Size::new(
            sanitize_size(measurement.desired.width),
            sanitize_size(measurement.desired.height),
        ),
        baseline: measurement.baseline.filter(|baseline| baseline.is_finite()),
    }
}

/// Resolves a height rule, feeding the already-resolved width as the cross
/// input so `AspectRatio` heights derive from actual widths.
fn resolve_height(rule: SizeRule, available: f32, measured: f32, resolved_width: f32) -> f32 {
    rule.resolve_dimension(SizeDimension::Height, available, measured, resolved_width)
}

/// Sum of resolved track sizes plus spacing, with `Fill` tracks contributing
/// their measured extent (the fill share is unknowable before arrange).
#[allow(clippy::cast_precision_loss)]
fn track_extent(
    rules: &[SizeRule],
    measured: &[f32],
    available: f32,
    cross_available: f32,
    spacing: f32,
) -> f32 {
    let total_spacing = spacing * rules.len().saturating_sub(1) as f32;
    let tracks = rules
        .iter()
        .enumerate()
        .map(|(index, rule)| {
            let measured = measured.get(index).copied().unwrap_or_default();
            if *rule == SizeRule::Fill {
                sanitize_size(measured)
            } else {
                rule.resolve(available, measured, cross_available)
            }
        })
        .sum::<f32>();
    sanitize_size(tracks + total_spacing)
}

const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// FNV-1a over the words that can change a leaf's measurement.
///
/// This deliberately avoids `DefaultHasher` so constraint keys are stable
/// across processes (RFC 0001 §2.4 determinism footnote).
fn fnv1a(words: &[u64]) -> u64 {
    let mut hash = FNV_OFFSET_BASIS;
    for word in words {
        for byte in word.to_le_bytes() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(FNV_PRIME);
        }
    }
    hash
}

/// Quantizes an available extent to 0.25 logical pixels.
///
/// Bucketing prevents float drift from missing the cache without changing
/// results: the arrange pass always re-solves exact rects; the cache only
/// short-circuits measurement (RFC 0001 §6).
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]
fn availability_bucket(value: f32) -> u64 {
    let quarters = (sanitize_size(value) * 4.0).round();
    if quarters >= u64::MAX as f32 {
        u64::MAX
    } else {
        quarters as u64
    }
}

fn constraint_key(content_revision: u64, available: Size) -> u64 {
    fnv1a(&[
        content_revision,
        availability_bucket(available.width),
        availability_bucket(available.height),
    ])
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct MeasureCacheEntry {
    constraint_key: u64,
    measurement: Measurement,
}

/// Retained measure cache keyed by [`WidgetId`].
///
/// Lives in `UiMemory` and follows the staged pattern of scroll offsets:
/// solves stage fresh measurements, the frame boundary commits them. Entries
/// hold only `(constraint key, Measurement)` pairs; shaped text payloads stay
/// in the text subsystem's own bounded store.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MeasureCache {
    committed: HashMap<WidgetId, MeasureCacheEntry>,
    pending: HashMap<WidgetId, MeasureCacheEntry>,
}

impl MeasureCache {
    /// Creates an empty cache.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the number of committed entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.committed.len()
    }

    /// Returns whether the cache holds no committed entries.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.committed.is_empty()
    }

    fn lookup(&self, widget: WidgetId, constraint_key: u64) -> Option<Measurement> {
        self.pending
            .get(&widget)
            .or_else(|| self.committed.get(&widget))
            .filter(|entry| entry.constraint_key == constraint_key)
            .map(|entry| entry.measurement)
    }

    fn stage(&mut self, widget: WidgetId, constraint_key: u64, measurement: Measurement) {
        self.pending.insert(
            widget,
            MeasureCacheEntry {
                constraint_key,
                measurement,
            },
        );
    }

    /// Promotes staged entries to committed state.
    ///
    /// Called at the frame boundary, mirroring how pending scroll offsets are
    /// committed by `UiMemory::end_frame`.
    pub fn commit(&mut self) {
        self.committed.extend(std::mem::take(&mut self.pending));
    }

    /// Keeps only entries whose widget satisfies `keep`.
    ///
    /// Hook for seen-this-frame reconciliation; wiring it to the frame
    /// runtime's widget-owner reconciliation is Phase L1 work.
    pub fn retain_widgets(&mut self, mut keep: impl FnMut(WidgetId) -> bool) {
        self.committed.retain(|widget, _| keep(*widget));
        self.pending.retain(|widget, _| keep(*widget));
    }

    /// Removes every entry.
    pub fn clear(&mut self) {
        self.committed.clear();
        self.pending.clear();
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;

    use super::{
        FixedMeasure, LayoutNodeId, LayoutTree, LeafMeasure, MeasureCache, MeasureIdentity,
    };
    use crate::layout::{Alignment, Insets, Measurement, SizeRule};
    use crate::{Rect, Size, UiMemory, WidgetId};

    fn fixed_leaf(
        tree: &mut LayoutTree,
        width: SizeRule,
        height: SizeRule,
        measured: Size,
    ) -> LayoutNodeId {
        tree.leaf(width, height, FixedMeasure::new(Measurement::new(measured)))
    }

    #[derive(Clone)]
    struct CountingMeasure {
        measured: Size,
        count: Rc<Cell<usize>>,
    }

    impl CountingMeasure {
        fn new(measured: Size) -> (Self, Rc<Cell<usize>>) {
            let count = Rc::new(Cell::new(0));
            (
                Self {
                    measured,
                    count: Rc::clone(&count),
                },
                count,
            )
        }
    }

    impl LeafMeasure for CountingMeasure {
        fn measure(&self, _available: Size) -> Measurement {
            self.count.set(self.count.get() + 1);
            Measurement::new(self.measured)
        }
    }

    #[test]
    fn row_tree_matches_linear_kernel() {
        let mut tree = LayoutTree::new();
        let a = fixed_leaf(&mut tree, SizeRule::Fixed(20.0), SizeRule::Fill, Size::ZERO);
        let b = fixed_leaf(
            &mut tree,
            SizeRule::Fit,
            SizeRule::Fill,
            Size::new(10.0, 5.0),
        );
        let c = fixed_leaf(&mut tree, SizeRule::Fill, SizeRule::Fill, Size::ZERO);
        let root = tree.row(SizeRule::Fill, SizeRule::Fill, 5.0, vec![a, b, c]);
        tree.set_root(root);

        let solution = tree.solve(Rect::new(0.0, 0.0, 100.0, 20.0));

        assert_eq!(solution.rect(root), Rect::new(0.0, 0.0, 100.0, 20.0));
        assert_eq!(solution.rect(a), Rect::new(0.0, 0.0, 20.0, 20.0));
        assert_eq!(solution.rect(b), Rect::new(25.0, 0.0, 10.0, 20.0));
        assert_eq!(solution.rect(c), Rect::new(40.0, 0.0, 60.0, 20.0));
    }

    #[test]
    fn column_tree_matches_linear_kernel() {
        let mut tree = LayoutTree::new();
        let a = fixed_leaf(&mut tree, SizeRule::Fill, SizeRule::Fixed(20.0), Size::ZERO);
        let b = fixed_leaf(
            &mut tree,
            SizeRule::Fill,
            SizeRule::Fit,
            Size::new(5.0, 10.0),
        );
        let c = fixed_leaf(&mut tree, SizeRule::Fill, SizeRule::Fill, Size::ZERO);
        let root = tree.column(SizeRule::Fill, SizeRule::Fill, 5.0, vec![a, b, c]);
        tree.set_root(root);

        let solution = tree.solve(Rect::new(0.0, 0.0, 20.0, 100.0));

        assert_eq!(solution.rect(a), Rect::new(0.0, 0.0, 20.0, 20.0));
        assert_eq!(solution.rect(b), Rect::new(0.0, 25.0, 20.0, 10.0));
        assert_eq!(solution.rect(c), Rect::new(0.0, 40.0, 20.0, 60.0));
    }

    #[test]
    fn fit_root_sizes_to_measured_content() {
        let mut tree = LayoutTree::new();
        let a = fixed_leaf(
            &mut tree,
            SizeRule::Fixed(30.0),
            SizeRule::Fixed(10.0),
            Size::ZERO,
        );
        let b = fixed_leaf(
            &mut tree,
            SizeRule::Fixed(20.0),
            SizeRule::Fixed(15.0),
            Size::ZERO,
        );
        let root = tree.row(SizeRule::Fit, SizeRule::Fit, 0.0, vec![a, b]);
        tree.set_root(root);

        let solution = tree.solve(Rect::new(0.0, 0.0, 200.0, 100.0));

        assert_eq!(solution.measurement(root).desired, Size::new(50.0, 15.0));
        assert_eq!(solution.rect(root), Rect::new(0.0, 0.0, 50.0, 15.0));
        assert_eq!(solution.rect(a), Rect::new(0.0, 0.0, 30.0, 10.0));
        assert_eq!(solution.rect(b), Rect::new(30.0, 0.0, 20.0, 15.0));
    }

    #[test]
    fn fit_row_reserves_spacing_in_measurement() {
        let mut tree = LayoutTree::new();
        let a = fixed_leaf(
            &mut tree,
            SizeRule::Fixed(30.0),
            SizeRule::Fixed(10.0),
            Size::ZERO,
        );
        let b = fixed_leaf(
            &mut tree,
            SizeRule::Fixed(20.0),
            SizeRule::Fixed(10.0),
            Size::ZERO,
        );
        let root = tree.row(SizeRule::Fit, SizeRule::Fit, 6.0, vec![a, b]);
        tree.set_root(root);

        let solution = tree.solve(Rect::new(0.0, 0.0, 200.0, 100.0));

        assert_eq!(solution.measurement(root).desired, Size::new(56.0, 10.0));
        assert_eq!(solution.rect(b), Rect::new(36.0, 0.0, 20.0, 10.0));
    }

    #[test]
    fn row_resolves_percent_minmax_and_fill() {
        let mut tree = LayoutTree::new();
        let a = fixed_leaf(
            &mut tree,
            SizeRule::Percent(0.5),
            SizeRule::Fill,
            Size::ZERO,
        );
        let b = fixed_leaf(
            &mut tree,
            SizeRule::MinMax {
                min: 10.0,
                max: 30.0,
            },
            SizeRule::Fill,
            Size::new(50.0, 0.0),
        );
        let c = fixed_leaf(&mut tree, SizeRule::Fill, SizeRule::Fill, Size::ZERO);
        let root = tree.row(SizeRule::Fill, SizeRule::Fill, 0.0, vec![a, b, c]);
        tree.set_root(root);

        let solution = tree.solve(Rect::new(0.0, 0.0, 100.0, 10.0));

        assert_eq!(solution.rect(a), Rect::new(0.0, 0.0, 50.0, 10.0));
        assert_eq!(solution.rect(b), Rect::new(50.0, 0.0, 30.0, 10.0));
        assert_eq!(solution.rect(c), Rect::new(80.0, 0.0, 20.0, 10.0));
    }

    #[test]
    fn row_resolves_aspect_ratio_width_from_cross_space() {
        let mut tree = LayoutTree::new();
        let a = fixed_leaf(
            &mut tree,
            SizeRule::AspectRatio(2.0),
            SizeRule::Fill,
            Size::ZERO,
        );
        let b = fixed_leaf(&mut tree, SizeRule::Fill, SizeRule::Fill, Size::ZERO);
        let root = tree.row(SizeRule::Fill, SizeRule::Fill, 0.0, vec![a, b]);
        tree.set_root(root);

        let solution = tree.solve(Rect::new(0.0, 0.0, 100.0, 40.0));

        assert_eq!(solution.rect(a), Rect::new(0.0, 0.0, 80.0, 40.0));
        assert_eq!(solution.rect(b), Rect::new(80.0, 0.0, 20.0, 40.0));
    }

    #[test]
    fn row_resolves_aspect_ratio_height_from_resolved_width() {
        let mut tree = LayoutTree::new();
        let a = fixed_leaf(
            &mut tree,
            SizeRule::Fixed(40.0),
            SizeRule::AspectRatio(2.0),
            Size::ZERO,
        );
        let root = tree.row(SizeRule::Fill, SizeRule::Fill, 0.0, vec![a]);
        tree.set_root(root);

        let solution = tree.solve(Rect::new(0.0, 0.0, 100.0, 50.0));

        assert_eq!(solution.rect(a), Rect::new(0.0, 0.0, 40.0, 20.0));
    }

    #[test]
    fn stack_children_resolve_against_stack_rect_and_may_overflow() {
        let mut tree = LayoutTree::new();
        let fixed = fixed_leaf(
            &mut tree,
            SizeRule::Fixed(30.0),
            SizeRule::Fixed(10.0),
            Size::ZERO,
        );
        let fill = fixed_leaf(&mut tree, SizeRule::Fill, SizeRule::Fill, Size::ZERO);
        let oversized = fixed_leaf(
            &mut tree,
            SizeRule::Fit,
            SizeRule::Fit,
            Size::new(120.0, 60.0),
        );
        let root = tree.stack(SizeRule::Fill, SizeRule::Fill, vec![fixed, fill, oversized]);
        tree.set_root(root);

        let solution = tree.solve(Rect::new(0.0, 0.0, 100.0, 50.0));

        assert_eq!(solution.rect(fixed), Rect::new(0.0, 0.0, 30.0, 10.0));
        assert_eq!(solution.rect(fill), Rect::new(0.0, 0.0, 100.0, 50.0));
        assert_eq!(solution.rect(oversized), Rect::new(0.0, 0.0, 120.0, 60.0));
        assert_eq!(solution.measurement(root).desired, Size::new(120.0, 60.0));
    }

    #[test]
    fn grid_tree_matches_grid_kernel() {
        let mut tree = LayoutTree::new();
        let children: Vec<LayoutNodeId> = (0..5)
            .map(|_| fixed_leaf(&mut tree, SizeRule::Fill, SizeRule::Fill, Size::ZERO))
            .collect();
        let root = tree.grid(
            SizeRule::Fill,
            SizeRule::Fill,
            vec![SizeRule::Fixed(20.0), SizeRule::Fill],
            vec![SizeRule::Fixed(10.0), SizeRule::Fill],
            0.0,
            0.0,
            children.clone(),
        );
        tree.set_root(root);

        let solution = tree.solve(Rect::new(0.0, 0.0, 100.0, 60.0));

        assert_eq!(solution.rect(children[0]), Rect::new(0.0, 0.0, 20.0, 10.0));
        assert_eq!(solution.rect(children[1]), Rect::new(20.0, 0.0, 80.0, 10.0));
        assert_eq!(solution.rect(children[2]), Rect::new(0.0, 10.0, 20.0, 50.0));
        assert_eq!(
            solution.rect(children[3]),
            Rect::new(20.0, 10.0, 80.0, 50.0)
        );
        assert_eq!(solution.rect(children[4]), Rect::ZERO);
    }

    #[test]
    fn grid_fit_tracks_use_largest_child_contribution() {
        let mut tree = LayoutTree::new();
        let sizes = [
            Size::new(30.0, 8.0),
            Size::new(5.0, 8.0),
            Size::new(12.0, 8.0),
            Size::new(5.0, 8.0),
        ];
        let children: Vec<LayoutNodeId> = sizes
            .iter()
            .map(|size| fixed_leaf(&mut tree, SizeRule::Fit, SizeRule::Fit, *size))
            .collect();
        let root = tree.grid(
            SizeRule::Fit,
            SizeRule::Fit,
            vec![SizeRule::Fit, SizeRule::Fixed(10.0)],
            vec![SizeRule::Fixed(8.0), SizeRule::Fixed(8.0)],
            0.0,
            0.0,
            children.clone(),
        );
        tree.set_root(root);

        let solution = tree.solve(Rect::new(0.0, 0.0, 200.0, 100.0));

        assert_eq!(solution.measurement(root).desired, Size::new(40.0, 16.0));
        assert_eq!(solution.rect(children[0]), Rect::new(0.0, 0.0, 30.0, 8.0));
        assert_eq!(solution.rect(children[1]), Rect::new(30.0, 0.0, 10.0, 8.0));
        assert_eq!(solution.rect(children[2]), Rect::new(0.0, 8.0, 30.0, 8.0));
        assert_eq!(solution.rect(children[3]), Rect::new(30.0, 8.0, 10.0, 8.0));
    }

    #[test]
    fn padding_shrinks_child_and_grows_measurement() {
        let mut tree = LayoutTree::new();
        let fill_child = fixed_leaf(&mut tree, SizeRule::Fill, SizeRule::Fill, Size::ZERO);
        let padded_fill =
            tree.padding(SizeRule::Fill, SizeRule::Fill, Insets::all(8.0), fill_child);
        tree.set_root(padded_fill);
        let solution = tree.solve(Rect::new(0.0, 0.0, 100.0, 50.0));
        assert_eq!(solution.rect(fill_child), Rect::new(8.0, 8.0, 84.0, 34.0));

        let mut tree = LayoutTree::new();
        let fit_child = fixed_leaf(
            &mut tree,
            SizeRule::Fit,
            SizeRule::Fit,
            Size::new(20.0, 10.0),
        );
        let padded_fit = tree.padding(SizeRule::Fit, SizeRule::Fit, Insets::all(8.0), fit_child);
        tree.set_root(padded_fit);
        let solution = tree.solve(Rect::new(0.0, 0.0, 100.0, 50.0));
        assert_eq!(
            solution.measurement(padded_fit).desired,
            Size::new(36.0, 26.0)
        );
        assert_eq!(solution.rect(padded_fit), Rect::new(0.0, 0.0, 36.0, 26.0));
        assert_eq!(solution.rect(fit_child), Rect::new(8.0, 8.0, 20.0, 10.0));
    }

    #[test]
    fn align_places_child_with_fit_box_semantics() {
        let mut tree = LayoutTree::new();
        let child = fixed_leaf(
            &mut tree,
            SizeRule::Fixed(20.0),
            SizeRule::Fixed(10.0),
            Size::ZERO,
        );
        let root = tree.align(
            SizeRule::Fill,
            SizeRule::Fill,
            Alignment::Center,
            Alignment::End,
            child,
        );
        tree.set_root(root);

        let solution = tree.solve(Rect::new(0.0, 0.0, 100.0, 50.0));

        assert_eq!(solution.rect(child), Rect::new(40.0, 40.0, 20.0, 10.0));
    }

    #[test]
    fn nested_composition_solves_exact_rects() {
        let mut tree = LayoutTree::new();
        let icon = fixed_leaf(&mut tree, SizeRule::Fixed(40.0), SizeRule::Fill, Size::ZERO);
        let label = fixed_leaf(
            &mut tree,
            SizeRule::Fit,
            SizeRule::Fill,
            Size::new(24.0, 24.0),
        );
        let flex = fixed_leaf(&mut tree, SizeRule::Fill, SizeRule::Fill, Size::ZERO);
        let toolbar = tree.row(
            SizeRule::Fill,
            SizeRule::Fixed(32.0),
            4.0,
            vec![icon, label, flex],
        );
        let body = fixed_leaf(&mut tree, SizeRule::Fill, SizeRule::Fill, Size::ZERO);
        let root = tree.column(SizeRule::Fill, SizeRule::Fill, 4.0, vec![toolbar, body]);
        tree.set_root(root);

        let solution = tree.solve(Rect::new(0.0, 0.0, 100.0, 200.0));

        assert_eq!(solution.rect(toolbar), Rect::new(0.0, 0.0, 100.0, 32.0));
        assert_eq!(solution.rect(icon), Rect::new(0.0, 0.0, 40.0, 32.0));
        assert_eq!(solution.rect(label), Rect::new(44.0, 0.0, 24.0, 32.0));
        assert_eq!(solution.rect(flex), Rect::new(72.0, 0.0, 28.0, 32.0));
        assert_eq!(solution.rect(body), Rect::new(0.0, 36.0, 100.0, 164.0));
    }

    #[test]
    fn solve_sanitizes_invalid_bounds_spacing_and_measurements() {
        let mut tree = LayoutTree::new();
        let nan_leaf = tree.leaf(
            SizeRule::Fit,
            SizeRule::Fit,
            FixedMeasure::new(Measurement::new(Size::new(f32::NAN, -5.0))),
        );
        let clamped = fixed_leaf(
            &mut tree,
            SizeRule::MinMax {
                min: 30.0,
                max: 10.0,
            },
            SizeRule::Fill,
            Size::new(20.0, 0.0),
        );
        let root = tree.row(
            SizeRule::Fill,
            SizeRule::Fill,
            f32::NAN,
            vec![nan_leaf, clamped],
        );
        tree.set_root(root);

        let solution = tree.solve(Rect::new(f32::NAN, 0.0, f32::INFINITY, 10.0));

        assert_eq!(solution.measurement(nan_leaf).desired, Size::ZERO);
        assert_eq!(solution.rect(root), Rect::new(0.0, 0.0, 0.0, 10.0));
        assert_eq!(solution.rect(nan_leaf), Rect::new(0.0, 0.0, 0.0, 0.0));
        assert_eq!(solution.rect(clamped), Rect::new(0.0, 0.0, 20.0, 10.0));
    }

    #[test]
    fn measurement_baseline_is_reported_and_sanitized() {
        let mut tree = LayoutTree::new();
        let with_baseline = tree.leaf(SizeRule::Fit, SizeRule::Fit, |_: Size| {
            Measurement::new(Size::new(10.0, 12.0)).with_baseline(9.0)
        });
        let invalid_baseline = tree.leaf(SizeRule::Fit, SizeRule::Fit, |_: Size| {
            Measurement::new(Size::new(10.0, 12.0)).with_baseline(f32::NAN)
        });
        let root = tree.row(
            SizeRule::Fill,
            SizeRule::Fill,
            0.0,
            vec![with_baseline, invalid_baseline],
        );
        tree.set_root(root);

        let solution = tree.solve(Rect::new(0.0, 0.0, 100.0, 20.0));

        assert_eq!(solution.measurement(with_baseline).baseline, Some(9.0));
        assert_eq!(solution.measurement(invalid_baseline).baseline, None);
        assert_eq!(solution.measurement(root).baseline, None);
    }

    #[test]
    fn leaf_measurer_receives_padded_available_size() {
        let seen = Rc::new(Cell::new(Size::ZERO));
        let observer = Rc::clone(&seen);
        let mut tree = LayoutTree::new();
        let leaf = tree.leaf(SizeRule::Fill, SizeRule::Fill, move |available: Size| {
            observer.set(available);
            Measurement::new(Size::ZERO)
        });
        let root = tree.padding(SizeRule::Fill, SizeRule::Fill, Insets::all(10.0), leaf);
        tree.set_root(root);

        let _ = tree.solve(Rect::new(0.0, 0.0, 100.0, 60.0));

        assert_eq!(seen.get(), Size::new(80.0, 40.0));
    }

    #[test]
    fn unreachable_nodes_keep_zero_rects() {
        let mut tree = LayoutTree::new();
        let orphan = fixed_leaf(&mut tree, SizeRule::Fill, SizeRule::Fill, Size::ZERO);
        let root = fixed_leaf(&mut tree, SizeRule::Fill, SizeRule::Fill, Size::ZERO);
        tree.set_root(root);

        let solution = tree.solve(Rect::new(0.0, 0.0, 50.0, 50.0));

        assert_eq!(solution.rect(orphan), Rect::ZERO);
        assert_eq!(solution.rect(root), Rect::new(0.0, 0.0, 50.0, 50.0));
    }

    #[test]
    fn empty_tree_solves_to_no_rects() {
        let tree = LayoutTree::new();
        let solution = tree.solve(Rect::new(0.0, 0.0, 50.0, 50.0));
        assert!(tree.is_empty());
        assert_eq!(
            solution,
            super::LayoutSolution {
                measurements: Vec::new(),
                rects: Vec::new(),
            }
        );
    }

    fn cached_tree(measurer: CountingMeasure, revision: u64) -> (LayoutTree, LayoutNodeId) {
        let mut tree = LayoutTree::new();
        let leaf = tree.cached_leaf(
            SizeRule::Fit,
            SizeRule::Fit,
            measurer,
            MeasureIdentity::new(WidgetId::from_key("cached"), revision),
        );
        let root = tree.row(SizeRule::Fill, SizeRule::Fill, 0.0, vec![leaf]);
        tree.set_root(root);
        (tree, leaf)
    }

    #[test]
    fn cached_solve_equals_uncached_solve_on_first_frame() {
        let (measurer, _) = CountingMeasure::new(Size::new(24.0, 12.0));
        let bounds = Rect::new(0.0, 0.0, 100.0, 40.0);

        let (tree, _) = cached_tree(measurer.clone(), 1);
        let uncached = tree.solve(bounds);

        let (tree, _) = cached_tree(measurer, 1);
        let mut cache = MeasureCache::new();
        let cached = tree.solve_cached(bounds, &mut cache);

        assert_eq!(cached, uncached);
    }

    #[test]
    fn cache_short_circuits_repeat_measurement_across_frames() {
        let (measurer, count) = CountingMeasure::new(Size::new(24.0, 12.0));
        let bounds = Rect::new(0.0, 0.0, 100.0, 40.0);
        let mut cache = MeasureCache::new();

        let (tree, leaf) = cached_tree(measurer.clone(), 1);
        let first = tree.solve_cached(bounds, &mut cache);
        assert_eq!(count.get(), 1);
        assert!(cache.is_empty());

        let second = tree.solve_cached(bounds, &mut cache);
        assert_eq!(count.get(), 1);
        assert_eq!(second, first);

        cache.commit();
        assert_eq!(cache.len(), 1);

        let (tree, _) = cached_tree(measurer, 1);
        let third = tree.solve_cached(bounds, &mut cache);
        assert_eq!(count.get(), 1);
        assert_eq!(third.rect(leaf), first.rect(leaf));
    }

    #[test]
    fn content_revision_change_invalidates_and_matches_cold_solve() {
        let (stale, _) = CountingMeasure::new(Size::new(24.0, 12.0));
        let (fresh, fresh_count) = CountingMeasure::new(Size::new(48.0, 12.0));
        let bounds = Rect::new(0.0, 0.0, 100.0, 40.0);
        let mut cache = MeasureCache::new();

        let (tree, _) = cached_tree(stale, 1);
        let _ = tree.solve_cached(bounds, &mut cache);
        cache.commit();

        let (tree, _) = cached_tree(fresh.clone(), 2);
        let warm = tree.solve_cached(bounds, &mut cache);
        assert_eq!(fresh_count.get(), 1);

        let (tree, _) = cached_tree(fresh, 2);
        let cold = tree.solve(bounds);
        assert_eq!(warm, cold);
    }

    #[test]
    fn availability_bucket_bounds_cache_reuse() {
        let (measurer, count) = CountingMeasure::new(Size::new(24.0, 12.0));
        let mut cache = MeasureCache::new();

        let (tree, _) = cached_tree(measurer, 1);
        let _ = tree.solve_cached(Rect::new(0.0, 0.0, 100.0, 40.0), &mut cache);
        assert_eq!(count.get(), 1);

        let _ = tree.solve_cached(Rect::new(0.0, 0.0, 100.1, 40.0), &mut cache);
        assert_eq!(count.get(), 1, "within-bucket availability change must hit");

        let _ = tree.solve_cached(Rect::new(0.0, 0.0, 200.0, 40.0), &mut cache);
        assert_eq!(count.get(), 2, "cross-bucket availability change must miss");
    }

    #[test]
    fn uncached_leaves_measure_every_solve() {
        let (measurer, count) = CountingMeasure::new(Size::new(24.0, 12.0));
        let mut tree = LayoutTree::new();
        let leaf = tree.leaf(SizeRule::Fit, SizeRule::Fit, measurer);
        let root = tree.row(SizeRule::Fill, SizeRule::Fill, 0.0, vec![leaf]);
        tree.set_root(root);
        let mut cache = MeasureCache::new();

        let _ = tree.solve_cached(Rect::new(0.0, 0.0, 100.0, 40.0), &mut cache);
        let _ = tree.solve_cached(Rect::new(0.0, 0.0, 100.0, 40.0), &mut cache);

        assert_eq!(count.get(), 2);
        assert!(cache.is_empty());
    }

    #[test]
    fn retain_widgets_evicts_committed_and_pending_entries() {
        let (measurer, count) = CountingMeasure::new(Size::new(24.0, 12.0));
        let bounds = Rect::new(0.0, 0.0, 100.0, 40.0);
        let mut cache = MeasureCache::new();

        let (tree, _) = cached_tree(measurer, 1);
        let _ = tree.solve_cached(bounds, &mut cache);
        cache.commit();
        assert_eq!(cache.len(), 1);

        cache.retain_widgets(|_| false);
        assert!(cache.is_empty());

        let _ = tree.solve_cached(bounds, &mut cache);
        assert_eq!(count.get(), 2);
    }

    #[test]
    fn ui_memory_commits_staged_measurements_at_end_frame() {
        let (measurer, count) = CountingMeasure::new(Size::new(24.0, 12.0));
        let bounds = Rect::new(0.0, 0.0, 100.0, 40.0);
        let mut memory = UiMemory::new();

        let (tree, _) = cached_tree(measurer, 1);
        let _ = tree.solve_cached(bounds, memory.measure_cache_mut());
        assert_eq!(count.get(), 1);
        assert!(memory.measure_cache().is_empty());

        memory.begin_frame();
        memory.end_frame();
        assert_eq!(memory.measure_cache().len(), 1);

        let _ = tree.solve_cached(bounds, memory.measure_cache_mut());
        assert_eq!(count.get(), 1);
    }
}
