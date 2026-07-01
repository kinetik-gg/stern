#[allow(clippy::wildcard_imports)]
use super::*;

/// Stable backend-independent node graph hit target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeGraphHitTarget {
    /// A hittable port on a node.
    Port(PortEndpoint),
    /// The node title bar.
    NodeTitle(NodeId),
    /// The node body below the title bar.
    NodeBody(NodeId),
    /// A reroute handle.
    Reroute(RerouteId),
    /// A resolved edge segment.
    Edge(EdgeId),
    /// A frame surface.
    Frame(NodeFrameId),
    /// A group surface.
    Group(NodeGroupId),
    /// The graph canvas or an out-of-viewport point.
    Canvas,
}

/// Stable selectable node graph target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum NodeGraphSelectionTarget {
    /// A node, independent from whether the title or body was hit.
    Node(NodeId),
    /// A graph edge.
    Edge(EdgeId),
    /// A reroute handle.
    Reroute(RerouteId),
    /// A node port endpoint.
    Port(PortEndpoint),
}

impl NodeGraphSelectionTarget {
    /// Converts a hit target into a selectable graph target.
    ///
    /// Canvas, frames, and groups are not selectable by this selection model.
    #[must_use]
    pub const fn from_hit_target(hit: NodeGraphHitTarget) -> Option<Self> {
        match hit {
            NodeGraphHitTarget::Port(endpoint) => Some(Self::Port(endpoint)),
            NodeGraphHitTarget::NodeTitle(node) | NodeGraphHitTarget::NodeBody(node) => {
                Some(Self::Node(node))
            }
            NodeGraphHitTarget::Reroute(reroute) => Some(Self::Reroute(reroute)),
            NodeGraphHitTarget::Edge(edge) => Some(Self::Edge(edge)),
            NodeGraphHitTarget::Frame(_)
            | NodeGraphHitTarget::Group(_)
            | NodeGraphHitTarget::Canvas => None,
        }
    }
}

/// Pure node graph selection operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeGraphSelectionOperation {
    /// Replace the selection with one target.
    Replace(NodeGraphSelectionTarget),
    /// Toggle one target in or out of the selection.
    Toggle(NodeGraphSelectionTarget),
    /// Add one target to the selection.
    Extend(NodeGraphSelectionTarget),
    /// Remove one target from the selection.
    Remove(NodeGraphSelectionTarget),
    /// Clear all selected targets.
    Clear,
}

/// Data-only node graph selection metadata.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NodeGraphSelection {
    selected: BTreeSet<NodeGraphSelectionTarget>,
    active: Option<NodeGraphSelectionTarget>,
}

impl NodeGraphSelection {
    /// Creates an empty selection.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a selection from graph targets.
    #[must_use]
    pub fn from_targets(targets: impl IntoIterator<Item = NodeGraphSelectionTarget>) -> Self {
        let selected = targets.into_iter().collect::<BTreeSet<_>>();
        Self {
            active: selected.iter().next_back().copied(),
            selected,
        }
    }

    /// Returns true when no graph targets are selected.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.selected.is_empty()
    }

    /// Returns true when the target is selected.
    #[must_use]
    pub fn contains(&self, target: NodeGraphSelectionTarget) -> bool {
        self.selected.contains(&target)
    }

    /// Returns selected targets in deterministic sorted order.
    #[must_use]
    pub fn selected(&self) -> Vec<NodeGraphSelectionTarget> {
        self.selected.iter().copied().collect()
    }

    /// Returns selected node IDs in deterministic sorted order.
    #[must_use]
    pub fn selected_nodes(&self) -> Vec<NodeId> {
        self.selected
            .iter()
            .filter_map(|target| match target {
                NodeGraphSelectionTarget::Node(node) => Some(*node),
                NodeGraphSelectionTarget::Edge(_)
                | NodeGraphSelectionTarget::Reroute(_)
                | NodeGraphSelectionTarget::Port(_) => None,
            })
            .collect()
    }

    /// Returns the most recent operation target, when one is present.
    #[must_use]
    pub const fn active(&self) -> Option<NodeGraphSelectionTarget> {
        self.active
    }

    /// Applies a pure selection operation and returns the resulting selection.
    #[must_use]
    pub fn apply(&self, operation: NodeGraphSelectionOperation) -> Self {
        match operation {
            NodeGraphSelectionOperation::Replace(target) => self.replace(target),
            NodeGraphSelectionOperation::Toggle(target) => self.toggle(target),
            NodeGraphSelectionOperation::Extend(target) => self.extend(target),
            NodeGraphSelectionOperation::Remove(target) => self.remove(target),
            NodeGraphSelectionOperation::Clear => self.clear(),
        }
    }

    /// Returns a selection containing only one target.
    #[must_use]
    pub fn replace(&self, target: NodeGraphSelectionTarget) -> Self {
        Self {
            selected: BTreeSet::from([target]),
            active: Some(target),
        }
    }

    /// Returns a selection with one target toggled in or out.
    #[must_use]
    pub fn toggle(&self, target: NodeGraphSelectionTarget) -> Self {
        let mut selected = self.selected.clone();
        if !selected.remove(&target) {
            selected.insert(target);
        }
        Self {
            active: Some(target),
            selected,
        }
    }

    /// Returns a selection with one target added.
    #[must_use]
    pub fn extend(&self, target: NodeGraphSelectionTarget) -> Self {
        let mut selected = self.selected.clone();
        selected.insert(target);
        Self {
            active: Some(target),
            selected,
        }
    }

    /// Returns a selection with one target removed.
    #[must_use]
    pub fn remove(&self, target: NodeGraphSelectionTarget) -> Self {
        let mut selected = self.selected.clone();
        selected.remove(&target);
        let active = if selected.is_empty() {
            None
        } else if self.active == Some(target) {
            selected.iter().next_back().copied()
        } else {
            self.active
        };

        Self { selected, active }
    }

    /// Returns an empty selection.
    #[must_use]
    pub fn clear(&self) -> Self {
        Self::new()
    }

    /// Replaces selection from a hit target, clearing explicitly on canvas.
    ///
    /// Frame and group hits are ignored by this selection model.
    #[must_use]
    pub fn replace_from_hit(&self, hit: NodeGraphHitTarget) -> Self {
        if hit == NodeGraphHitTarget::Canvas {
            return self.clear();
        }

        NodeGraphSelectionTarget::from_hit_target(hit)
            .map_or_else(|| self.clone(), |target| self.replace(target))
    }
}
