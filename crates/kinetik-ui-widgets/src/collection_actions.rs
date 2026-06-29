//! Shared data-only drag/drop and context-action contracts for collections.

use std::collections::BTreeSet;

use kinetik_ui_core::{ActionDescriptor, ActionId};

use crate::{ItemId, Selection};

/// Stable drag source metadata for a collection item drag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollectionDragSource {
    /// Item where the drag gesture started.
    pub source: ItemId,
    /// Items represented by the drag, captured in deterministic order.
    pub items: Vec<ItemId>,
}

impl CollectionDragSource {
    /// Creates drag source metadata for one item.
    #[must_use]
    pub fn new(source: ItemId) -> Self {
        Self {
            source,
            items: vec![source],
        }
    }

    /// Creates drag source metadata from selection state.
    ///
    /// If the source item is part of the current selection, the whole selected
    /// set is captured in sorted order. Otherwise the drag represents only the
    /// source item.
    #[must_use]
    pub fn from_selection(source: ItemId, selection: &Selection) -> Self {
        if !selection.contains(source) {
            return Self::new(source);
        }

        let mut items = selection.selected();
        if items.is_empty() {
            items.push(source);
        }

        Self { source, items }
    }

    /// Returns true when this drag source includes an item.
    #[must_use]
    pub fn contains(&self, item: ItemId) -> bool {
        self.items.contains(&item)
    }
}

/// Context-menu target for one collection item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CollectionItemContextTarget {
    /// Addressed item.
    pub item: ItemId,
}

impl CollectionItemContextTarget {
    /// Creates an item context target.
    #[must_use]
    pub const fn new(item: ItemId) -> Self {
        Self { item }
    }
}

/// Context-menu target for the active collection selection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollectionSelectionContextTarget {
    /// Selected item IDs in deterministic order.
    pub items: Vec<ItemId>,
}

impl CollectionSelectionContextTarget {
    /// Creates a selection context target from item IDs.
    #[must_use]
    pub fn new(items: impl IntoIterator<Item = ItemId>) -> Option<Self> {
        let items = items.into_iter().collect::<BTreeSet<_>>();
        (!items.is_empty()).then(|| Self {
            items: items.into_iter().collect(),
        })
    }

    /// Creates a selection context target from selection state.
    #[must_use]
    pub fn from_selection(selection: &Selection) -> Option<Self> {
        Self::new(selection.selected())
    }
}

/// Context-menu target for collection background or empty space.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CollectionBackgroundContextTarget;

impl CollectionBackgroundContextTarget {
    /// Creates a background context target.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

/// Stable collection context-menu target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CollectionContextTarget {
    /// An individual item target.
    Item(CollectionItemContextTarget),
    /// The active multi-selection target.
    Selection(CollectionSelectionContextTarget),
    /// Collection background or empty-space target.
    Background(CollectionBackgroundContextTarget),
}

impl CollectionContextTarget {
    /// Creates an item context target.
    #[must_use]
    pub const fn item(item: ItemId) -> Self {
        Self::Item(CollectionItemContextTarget::new(item))
    }

    /// Creates a selection context target.
    #[must_use]
    pub fn selection(items: impl IntoIterator<Item = ItemId>) -> Option<Self> {
        CollectionSelectionContextTarget::new(items).map(Self::Selection)
    }

    /// Creates a background context target.
    #[must_use]
    pub const fn background() -> Self {
        Self::Background(CollectionBackgroundContextTarget::new())
    }

    /// Returns stable target item IDs carried by this context.
    #[must_use]
    pub fn target_ids(&self) -> Vec<ItemId> {
        match self {
            Self::Item(target) => vec![target.item],
            Self::Selection(target) => target.items.clone(),
            Self::Background(_) => Vec::new(),
        }
    }
}

/// Data-only descriptor for a context action bound to a collection target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollectionContextAction {
    /// Application-owned action presentation metadata.
    pub descriptor: ActionDescriptor,
    /// Context target captured by the UI request.
    pub target: CollectionContextTarget,
}

impl CollectionContextAction {
    /// Creates context-action metadata.
    #[must_use]
    pub const fn new(descriptor: ActionDescriptor, target: CollectionContextTarget) -> Self {
        Self { descriptor, target }
    }

    /// Returns true when the action can produce request metadata.
    #[must_use]
    pub const fn can_request(&self) -> bool {
        self.descriptor.can_invoke()
    }

    /// Creates app-owned request metadata without executing the action.
    #[must_use]
    pub fn request(&self) -> Option<CollectionContextActionRequest> {
        self.can_request()
            .then(|| CollectionContextActionRequest::new(self.descriptor.id.clone(), &self.target))
    }
}

/// UI request metadata for a collection context action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollectionContextActionRequest {
    /// Action identity requested by the context surface.
    pub action_id: ActionId,
    /// Context target captured by the UI request.
    pub target: CollectionContextTarget,
    /// Stable target item IDs for application dispatch.
    pub target_ids: Vec<ItemId>,
}

impl CollectionContextActionRequest {
    /// Creates request metadata for an action and target.
    #[must_use]
    pub fn new(action_id: ActionId, target: &CollectionContextTarget) -> Self {
        Self {
            action_id,
            target: target.clone(),
            target_ids: target.target_ids(),
        }
    }
}

/// Builds context-action metadata in descriptor order.
#[must_use]
pub fn collection_context_actions(
    target: &CollectionContextTarget,
    descriptors: impl IntoIterator<Item = ActionDescriptor>,
) -> Vec<CollectionContextAction> {
    descriptors
        .into_iter()
        .filter(|descriptor| descriptor.state.visible)
        .map(|descriptor| CollectionContextAction::new(descriptor, target.clone()))
        .collect()
}
