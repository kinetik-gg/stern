use super::{
    FocusTraversal, SemanticAction, SemanticActionKind, SemanticNode, SemanticRole, SemanticState,
    SemanticTree, SemanticTreeError, SemanticValue, tree::SemanticTreeIndex,
};
use crate::{Rect, WidgetId};

/// Semantic node data exported to platform accessibility adapters.
///
/// This is the stable, backend-neutral data contract for adapters. It is
/// derived from a validated [`SemanticTree`] and does not carry render
/// primitive or platform API state.
#[derive(Debug, Clone, PartialEq)]
pub struct AccessibilityNode {
    /// Stable widget identity.
    pub id: WidgetId,
    /// Parent node ID, if the node is nested in another semantic node.
    pub parent: Option<WidgetId>,
    /// Semantic role.
    pub role: SemanticRole,
    /// Logical bounds.
    pub bounds: Rect,
    /// Accessible name.
    pub label: Option<String>,
    /// Longer accessible description.
    pub description: Option<String>,
    /// Runtime state.
    pub state: SemanticState,
    /// Supported semantic actions.
    pub actions: Vec<SemanticAction>,
    /// Ordered child node IDs.
    pub children: Vec<WidgetId>,
    /// Whether the node participates in focus traversal.
    pub focusable: bool,
}

impl AccessibilityNode {
    fn from_semantic(node: &SemanticNode, parent: Option<WidgetId>) -> Self {
        Self {
            id: node.id,
            parent,
            role: node.role.clone(),
            bounds: node.bounds,
            label: node.label.clone(),
            description: node.description.clone(),
            state: node.state.clone(),
            actions: node.actions.clone(),
            children: node.children.clone(),
            focusable: node.focusable,
        }
    }
}

/// Validated accessibility snapshot exported for platform adapters.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AccessibilitySnapshot {
    /// Root node ID, if the frame emitted semantic content.
    pub root: Option<WidgetId>,
    /// Nodes in deterministic semantic traversal order.
    pub nodes: Vec<AccessibilityNode>,
    /// Focusable nodes in deterministic traversal order.
    pub focus_order: Vec<WidgetId>,
    /// Focused widget when it is present in `focus_order`.
    pub focused: Option<WidgetId>,
}

impl AccessibilitySnapshot {
    /// Builds a snapshot from a semantic tree after validating structure.
    ///
    /// # Errors
    ///
    /// Returns [`SemanticTreeError`] when the semantic tree is structurally
    /// invalid.
    pub fn from_tree(
        tree: &SemanticTree,
        focused: Option<WidgetId>,
    ) -> Result<Self, SemanticTreeError> {
        let index = SemanticTreeIndex::validate(tree)?;

        let focus = FocusTraversal::from_index(tree, &index, focused);
        let nodes = tree
            .traversal_order_with_index(&index)
            .into_iter()
            .filter_map(|id| {
                index
                    .node(tree, id)
                    .map(|node| AccessibilityNode::from_semantic(node, index.parent_of(id)))
            })
            .collect();

        Ok(Self {
            root: tree.root(),
            nodes,
            focus_order: focus.order,
            focused: focus.focused,
        })
    }

    /// Finds an exported node by ID.
    #[must_use]
    pub fn node(&self, id: WidgetId) -> Option<&AccessibilityNode> {
        self.nodes.iter().find(|node| node.id == id)
    }

    /// Finds an exported node by semantic widget ID.
    #[must_use]
    pub fn find_by_id(&self, id: WidgetId) -> Option<&AccessibilityNode> {
        self.node(id)
    }

    /// Returns exported nodes with the requested semantic role.
    pub fn nodes_by_role<'a>(
        &'a self,
        role: &'a SemanticRole,
    ) -> impl Iterator<Item = &'a AccessibilityNode> + 'a {
        self.nodes.iter().filter(move |node| &node.role == role)
    }

    /// Returns exported nodes with the requested exact accessible label.
    pub fn nodes_by_label<'a>(
        &'a self,
        label: &'a str,
    ) -> impl Iterator<Item = &'a AccessibilityNode> + 'a {
        self.nodes
            .iter()
            .filter(move |node| node.label.as_deref() == Some(label))
    }

    /// Finds the first exported node with the requested exact accessible label.
    #[must_use]
    pub fn find_by_label(&self, label: &str) -> Option<&AccessibilityNode> {
        self.nodes
            .iter()
            .find(|node| node.label.as_deref() == Some(label))
    }

    /// Returns exported nodes with both the requested role and exact label.
    pub fn nodes_by_role_and_label<'a>(
        &'a self,
        role: &'a SemanticRole,
        label: &'a str,
    ) -> impl Iterator<Item = &'a AccessibilityNode> + 'a {
        self.nodes
            .iter()
            .filter(move |node| &node.role == role && node.label.as_deref() == Some(label))
    }

    /// Finds the first exported node with both the requested role and exact label.
    #[must_use]
    pub fn find_by_role_and_label(
        &self,
        role: &SemanticRole,
        label: &str,
    ) -> Option<&AccessibilityNode> {
        self.nodes
            .iter()
            .find(|node| &node.role == role && node.label.as_deref() == Some(label))
    }

    /// Returns exported nodes with the requested semantic value.
    pub fn nodes_by_value<'a>(
        &'a self,
        value: &'a SemanticValue,
    ) -> impl Iterator<Item = &'a AccessibilityNode> + 'a {
        self.nodes
            .iter()
            .filter(move |node| node.state.value.as_ref() == Some(value))
    }

    /// Finds the first exported node with the requested semantic value.
    #[must_use]
    pub fn find_by_value(&self, value: &SemanticValue) -> Option<&AccessibilityNode> {
        self.nodes
            .iter()
            .find(|node| node.state.value.as_ref() == Some(value))
    }

    /// Returns exported nodes that support the requested semantic action kind.
    pub fn nodes_by_action<'a>(
        &'a self,
        action: &'a SemanticActionKind,
    ) -> impl Iterator<Item = &'a AccessibilityNode> + 'a {
        self.nodes.iter().filter(move |node| {
            node.actions
                .iter()
                .any(|candidate| &candidate.kind == action)
        })
    }

    /// Finds the first exported node that supports the requested semantic action kind.
    #[must_use]
    pub fn find_by_action(&self, action: &SemanticActionKind) -> Option<&AccessibilityNode> {
        self.nodes.iter().find(|node| {
            node.actions
                .iter()
                .any(|candidate| &candidate.kind == action)
        })
    }

    /// Returns exported nodes whose semantic state matches the predicate.
    pub fn nodes_by_state<'a>(
        &'a self,
        mut predicate: impl FnMut(&SemanticState) -> bool + 'a,
    ) -> impl Iterator<Item = &'a AccessibilityNode> + 'a {
        self.nodes.iter().filter(move |node| predicate(&node.state))
    }

    /// Finds the first exported node whose semantic state matches the predicate.
    #[must_use]
    pub fn find_by_state(
        &self,
        mut predicate: impl FnMut(&SemanticState) -> bool,
    ) -> Option<&AccessibilityNode> {
        self.nodes.iter().find(|node| predicate(&node.state))
    }

    /// Returns focusable exported nodes in deterministic focus traversal order.
    pub fn focus_order_nodes(&self) -> impl Iterator<Item = &AccessibilityNode> {
        self.focus_order.iter().filter_map(|id| self.node(*id))
    }
}
