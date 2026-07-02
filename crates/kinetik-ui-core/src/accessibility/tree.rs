use std::collections::{BTreeMap, BTreeSet};

use super::{AccessibilitySnapshot, SemanticNode};
use crate::WidgetId;

/// Semantic tree for one UI frame.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SemanticTree {
    pub(super) nodes: Vec<SemanticNode>,
    pub(super) root: Option<WidgetId>,
}

#[derive(Debug, Default)]
pub(super) struct SemanticTreeIndex {
    node_by_id: BTreeMap<WidgetId, usize>,
    parent_by_child: BTreeMap<WidgetId, WidgetId>,
    children_by_parent: BTreeMap<WidgetId, Vec<WidgetId>>,
}

impl SemanticTreeIndex {
    pub(super) fn from_tree(tree: &SemanticTree) -> Self {
        let mut index = Self::default();
        for (node_index, node) in tree.nodes.iter().enumerate() {
            index.node_by_id.entry(node.id).or_insert(node_index);
            for child in &node.children {
                index.parent_by_child.entry(*child).or_insert(node.id);
            }
            index
                .children_by_parent
                .entry(node.id)
                .or_insert_with(|| node.children.clone());
        }
        index
    }

    pub(super) fn validate(tree: &SemanticTree) -> Result<Self, SemanticTreeError> {
        if tree.nodes.is_empty() {
            return Ok(Self::default());
        }

        let mut ids = BTreeSet::new();
        let mut node_by_id = BTreeMap::new();
        for (node_index, node) in tree.nodes.iter().enumerate() {
            if !ids.insert(node.id) {
                return Err(SemanticTreeError::DuplicateNodeId { id: node.id });
            }
            node_by_id.insert(node.id, node_index);
        }

        let Some(root) = tree.root else {
            return Err(SemanticTreeError::MissingRoot);
        };
        if !ids.contains(&root) {
            return Err(SemanticTreeError::UnknownRoot { id: root });
        }

        let mut parent_by_child = BTreeMap::new();
        let mut children_by_parent = BTreeMap::new();
        for node in &tree.nodes {
            let mut child_ids = BTreeSet::new();
            for child in &node.children {
                if *child == node.id {
                    return Err(SemanticTreeError::SelfChild { id: node.id });
                }
                if !ids.contains(child) {
                    return Err(SemanticTreeError::UnknownChild {
                        parent: node.id,
                        child: *child,
                    });
                }
                if !child_ids.insert(*child) {
                    return Err(SemanticTreeError::DuplicateChild {
                        parent: node.id,
                        child: *child,
                    });
                }
                if let Some(first_parent) = parent_by_child.insert(*child, node.id) {
                    return Err(SemanticTreeError::MultipleParents {
                        child: *child,
                        first_parent,
                        second_parent: node.id,
                    });
                }
            }
            children_by_parent.insert(node.id, node.children.clone());
        }

        let index = Self {
            node_by_id,
            parent_by_child,
            children_by_parent,
        };
        let mut visited = BTreeSet::new();
        let mut visiting = BTreeSet::new();
        validate_semantic_cycles(root, &index.children_by_parent, &mut visiting, &mut visited)?;
        for node in &tree.nodes {
            validate_semantic_cycles(
                node.id,
                &index.children_by_parent,
                &mut visiting,
                &mut visited,
            )?;
        }

        Ok(index)
    }

    fn contains(&self, id: WidgetId) -> bool {
        self.node_by_id.contains_key(&id)
    }

    pub(super) fn node<'a>(
        &self,
        tree: &'a SemanticTree,
        id: WidgetId,
    ) -> Option<&'a SemanticNode> {
        self.node_by_id
            .get(&id)
            .and_then(|index| tree.nodes.get(*index))
    }

    pub(super) fn parent_of(&self, child: WidgetId) -> Option<WidgetId> {
        self.parent_by_child.get(&child).copied()
    }
}

impl SemanticTree {
    /// Creates an empty semantic tree.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the root node ID.
    pub fn set_root(&mut self, root: WidgetId) {
        self.root = Some(root);
    }

    /// Returns true when the tree contains no semantic nodes.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Returns the number of semantic nodes in the tree.
    #[must_use]
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Returns the root node ID.
    #[must_use]
    pub const fn root(&self) -> Option<WidgetId> {
        self.root
    }

    /// Pushes a node in traversal order.
    pub fn push(&mut self, node: SemanticNode) {
        if self.root.is_none() {
            self.root = Some(node.id);
        }
        self.nodes.push(node);
    }

    /// Returns all nodes in insertion order.
    #[must_use]
    pub fn nodes(&self) -> &[SemanticNode] {
        &self.nodes
    }

    /// Finds a node by ID.
    #[must_use]
    pub fn get(&self, id: WidgetId) -> Option<&SemanticNode> {
        self.nodes.iter().find(|node| node.id == id)
    }

    /// Returns the parent node ID for a child.
    #[must_use]
    pub fn parent_of(&self, child: WidgetId) -> Option<WidgetId> {
        self.nodes
            .iter()
            .find(|node| node.children.contains(&child))
            .map(|node| node.id)
    }

    /// Returns node IDs in semantic child order, appending unparented nodes in insertion order.
    #[must_use]
    pub fn traversal_order(&self) -> Vec<WidgetId> {
        let index = SemanticTreeIndex::from_tree(self);
        self.traversal_order_with_index(&index)
    }

    pub(super) fn traversal_order_with_index(&self, index: &SemanticTreeIndex) -> Vec<WidgetId> {
        let mut order = Vec::new();
        let mut visited = BTreeSet::new();
        if let Some(root) = self.root.filter(|root| index.contains(*root)) {
            self.push_traversal(root, index, &mut visited, &mut order);
        }
        for node in &self.nodes {
            self.push_traversal(node.id, index, &mut visited, &mut order);
        }
        order
    }

    /// Returns focusable node IDs in traversal order.
    #[must_use]
    pub fn focus_order(&self) -> Vec<WidgetId> {
        let index = SemanticTreeIndex::from_tree(self);
        self.focus_order_with_index(&index)
    }

    pub(super) fn focus_order_with_index(&self, index: &SemanticTreeIndex) -> Vec<WidgetId> {
        let mut order = Vec::new();
        let mut visited = BTreeSet::new();
        if let Some(root) = self.root.filter(|root| index.contains(*root)) {
            self.push_focus_order(root, index, false, &mut visited, &mut order);
        }
        for node in &self.nodes {
            self.push_focus_order(node.id, index, false, &mut visited, &mut order);
        }
        order
    }

    /// Exports a validated accessibility snapshot for platform adapters.
    ///
    /// # Errors
    ///
    /// Returns [`SemanticTreeError`] when the semantic tree is structurally
    /// invalid.
    pub fn accessibility_snapshot(
        &self,
        focused: Option<WidgetId>,
    ) -> Result<AccessibilitySnapshot, SemanticTreeError> {
        AccessibilitySnapshot::from_tree(self, focused)
    }

    /// Validates structural semantic-tree invariants.
    ///
    /// Empty trees are valid. Non-empty trees must have a root that points at
    /// an existing node, unique node IDs, children that point at existing
    /// nodes, and no node may have multiple semantic parents.
    ///
    /// # Errors
    ///
    /// Returns [`SemanticTreeError`] for the first structural violation found.
    pub fn validate(&self) -> Result<(), SemanticTreeError> {
        SemanticTreeIndex::validate(self).map(|_| ())
    }

    fn push_traversal(
        &self,
        id: WidgetId,
        index: &SemanticTreeIndex,
        visited: &mut BTreeSet<WidgetId>,
        order: &mut Vec<WidgetId>,
    ) {
        if !visited.insert(id) {
            return;
        }
        let Some(node) = index.node(self, id) else {
            return;
        };
        order.push(id);
        for child in &node.children {
            self.push_traversal(*child, index, visited, order);
        }
    }

    fn push_focus_order(
        &self,
        id: WidgetId,
        index: &SemanticTreeIndex,
        disabled_ancestor: bool,
        visited: &mut BTreeSet<WidgetId>,
        order: &mut Vec<WidgetId>,
    ) {
        if !visited.insert(id) {
            return;
        }
        let Some(node) = index.node(self, id) else {
            return;
        };
        let disabled = disabled_ancestor || node.state.disabled;
        if node.focusable && !disabled {
            order.push(id);
        }
        for child in &node.children {
            self.push_focus_order(*child, index, disabled, visited, order);
        }
    }
}

/// Structural semantic tree validation error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemanticTreeError {
    /// Non-empty tree has no root node.
    MissingRoot,
    /// Root points at a node that is not present in the tree.
    UnknownRoot {
        /// Unknown root node ID.
        id: WidgetId,
    },
    /// Two nodes use the same stable node ID.
    DuplicateNodeId {
        /// Duplicate node ID.
        id: WidgetId,
    },
    /// A child edge points at a node that is not present in the tree.
    UnknownChild {
        /// Parent node ID.
        parent: WidgetId,
        /// Unknown child node ID.
        child: WidgetId,
    },
    /// A node lists the same child more than once.
    DuplicateChild {
        /// Parent node ID.
        parent: WidgetId,
        /// Duplicate child node ID.
        child: WidgetId,
    },
    /// A node lists itself as a child.
    SelfChild {
        /// Self-referential node ID.
        id: WidgetId,
    },
    /// A child has more than one semantic parent.
    MultipleParents {
        /// Child node ID.
        child: WidgetId,
        /// First parent encountered.
        first_parent: WidgetId,
        /// Second parent encountered.
        second_parent: WidgetId,
    },
    /// A semantic child cycle was detected.
    Cycle {
        /// Node where the cycle was detected.
        id: WidgetId,
    },
}

fn validate_semantic_cycles(
    id: WidgetId,
    children_by_parent: &BTreeMap<WidgetId, Vec<WidgetId>>,
    visiting: &mut BTreeSet<WidgetId>,
    visited: &mut BTreeSet<WidgetId>,
) -> Result<(), SemanticTreeError> {
    if visited.contains(&id) {
        return Ok(());
    }
    if !visiting.insert(id) {
        return Err(SemanticTreeError::Cycle { id });
    }
    if let Some(children) = children_by_parent.get(&id) {
        for child in children {
            validate_semantic_cycles(*child, children_by_parent, visiting, visited)?;
        }
    }
    visiting.remove(&id);
    visited.insert(id);
    Ok(())
}
