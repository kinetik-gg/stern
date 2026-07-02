use super::{SemanticTree, tree::SemanticTreeIndex};
use crate::WidgetId;

/// Deterministic focus traversal snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FocusTraversal {
    /// Focusable widgets in traversal order.
    pub order: Vec<WidgetId>,
    /// Currently focused widget.
    pub focused: Option<WidgetId>,
}

impl FocusTraversal {
    /// Creates traversal from a semantic tree.
    #[must_use]
    pub fn from_tree(tree: &SemanticTree, focused: Option<WidgetId>) -> Self {
        let index = SemanticTreeIndex::from_tree(tree);
        Self::from_index(tree, &index, focused)
    }

    pub(super) fn from_index(
        tree: &SemanticTree,
        index: &SemanticTreeIndex,
        focused: Option<WidgetId>,
    ) -> Self {
        let order = tree.focus_order_with_index(index);
        let focused = focused.filter(|id| order.contains(id));
        Self { order, focused }
    }

    /// Returns the next focus target, wrapping at the end.
    #[must_use]
    pub fn next(&self) -> Option<WidgetId> {
        cycle_focus(&self.order, self.focused, FocusDirection::Forward)
    }

    /// Returns the previous focus target, wrapping at the start.
    #[must_use]
    pub fn previous(&self) -> Option<WidgetId> {
        cycle_focus(&self.order, self.focused, FocusDirection::Backward)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FocusDirection {
    Forward,
    Backward,
}

fn cycle_focus(
    order: &[WidgetId],
    focused: Option<WidgetId>,
    direction: FocusDirection,
) -> Option<WidgetId> {
    if order.is_empty() {
        return None;
    }
    let index = focused
        .and_then(|id| order.iter().position(|candidate| *candidate == id))
        .unwrap_or_else(|| match direction {
            FocusDirection::Forward => order.len() - 1,
            FocusDirection::Backward => 0,
        });
    let next = match direction {
        FocusDirection::Forward => (index + 1) % order.len(),
        FocusDirection::Backward => (index + order.len() - 1) % order.len(),
    };
    Some(order[next])
}
