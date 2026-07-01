#[allow(clippy::wildcard_imports)]
use super::*;

/// Data-only descriptor for an add-node search entry.
///
/// This is intentionally application-owned metadata. It identifies a node kind
/// the application may create later, but it does not create or mutate graph
/// nodes by itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeGraphAddNodeSearchEntry {
    /// Stable application-owned descriptor identity.
    pub id: NodeGraphAddNodeDescriptorId,
    /// User-facing entry label.
    pub label: String,
    /// Optional user-facing category used for deterministic filtering.
    pub category: Option<String>,
    /// Optional user-facing description used for deterministic filtering.
    pub description: Option<String>,
    /// Additional application-owned search keywords.
    pub keywords: Vec<String>,
    /// Whether this entry may currently be selected.
    pub enabled: bool,
}

impl NodeGraphAddNodeSearchEntry {
    /// Creates an enabled add-node search entry.
    #[must_use]
    pub fn new(id: NodeGraphAddNodeDescriptorId, label: impl Into<String>) -> Self {
        Self {
            id,
            label: label.into(),
            category: None,
            description: None,
            keywords: Vec::new(),
            enabled: true,
        }
    }

    /// Sets the entry category.
    #[must_use]
    pub fn with_category(mut self, category: impl Into<String>) -> Self {
        self.category = Some(category.into());
        self
    }

    /// Sets the entry description.
    #[must_use]
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Sets additional search keywords.
    #[must_use]
    pub fn with_keywords<I, S>(mut self, keywords: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.keywords = keywords.into_iter().map(Into::into).collect();
        self
    }

    /// Sets whether the entry may currently be selected.
    #[must_use]
    pub const fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Returns true when this entry matches every non-empty query term.
    #[must_use]
    pub fn matches_query(&self, query: &str) -> bool {
        let terms = node_graph_add_node_query_terms(query);
        node_graph_add_node_entry_matches_terms(self, &terms)
    }

    /// Returns the first deterministic label highlight range for a query.
    ///
    /// Empty queries or matches found only in category, description, or
    /// keywords return `None`.
    #[must_use]
    pub fn label_highlight(&self, query: &str) -> Option<NodeGraphAddNodeSearchHighlight> {
        let terms = node_graph_add_node_query_terms(query);
        node_graph_add_node_label_highlight(&self.label, &terms)
    }
}

/// Byte range in an add-node search entry label that should be highlighted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NodeGraphAddNodeSearchHighlight {
    /// Inclusive byte offset where the highlight begins.
    pub start: usize,
    /// Exclusive byte offset where the highlight ends.
    pub end: usize,
}

impl NodeGraphAddNodeSearchHighlight {
    /// Creates a label highlight range.
    #[must_use]
    pub const fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }
}

/// One deterministic add-node search match.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NodeGraphAddNodeSearchMatch<'a> {
    /// Matched descriptor.
    pub entry: &'a NodeGraphAddNodeSearchEntry,
    /// First matching label range, if the query matched the label.
    pub label_highlight: Option<NodeGraphAddNodeSearchHighlight>,
}

/// Filters add-node search descriptors in input order.
///
/// Empty or whitespace-only queries return every entry. Disabled entries remain
/// visible in results; selection helpers skip them.
#[must_use]
pub fn filter_node_graph_add_node_search_entries<'a>(
    entries: &'a [NodeGraphAddNodeSearchEntry],
    query: &str,
) -> Vec<NodeGraphAddNodeSearchMatch<'a>> {
    let terms = node_graph_add_node_query_terms(query);
    entries
        .iter()
        .filter(|entry| node_graph_add_node_entry_matches_terms(entry, &terms))
        .map(|entry| NodeGraphAddNodeSearchMatch {
            entry,
            label_highlight: node_graph_add_node_label_highlight(&entry.label, &terms),
        })
        .collect()
}

/// Selection metadata for add-node search results.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NodeGraphAddNodeSearchSelection {
    /// Selected application-owned add-node descriptor ID.
    pub selected: Option<NodeGraphAddNodeDescriptorId>,
}

impl NodeGraphAddNodeSearchSelection {
    /// Creates an empty add-node search selection.
    #[must_use]
    pub const fn new() -> Self {
        Self { selected: None }
    }

    /// Creates a selection from an existing descriptor ID.
    #[must_use]
    pub const fn from_selected(selected: NodeGraphAddNodeDescriptorId) -> Self {
        Self {
            selected: Some(selected),
        }
    }

    /// Selects the first enabled entry matching the query.
    #[must_use]
    pub fn select_first(entries: &[NodeGraphAddNodeSearchEntry], query: &str) -> Self {
        Self {
            selected: enabled_add_node_search_matches(entries, query)
                .first()
                .map(|result| result.entry.id),
        }
    }

    /// Selects the next enabled entry matching the query, wrapping at the end.
    #[must_use]
    pub fn select_next(&self, entries: &[NodeGraphAddNodeSearchEntry], query: &str) -> Self {
        let matches = enabled_add_node_search_matches(entries, query);
        let Some(selected) = next_add_node_search_selection(
            &matches,
            self.selected,
            AddNodeSearchSelectionDirection::Next,
        ) else {
            return Self::new();
        };

        Self {
            selected: Some(selected),
        }
    }

    /// Selects the previous enabled entry matching the query, wrapping at the start.
    #[must_use]
    pub fn select_previous(&self, entries: &[NodeGraphAddNodeSearchEntry], query: &str) -> Self {
        let matches = enabled_add_node_search_matches(entries, query);
        let Some(selected) = next_add_node_search_selection(
            &matches,
            self.selected,
            AddNodeSearchSelectionDirection::Previous,
        ) else {
            return Self::new();
        };

        Self {
            selected: Some(selected),
        }
    }

    /// Returns the selected enabled search match, if it is still present.
    #[must_use]
    pub fn selected_entry<'a>(
        &self,
        entries: &'a [NodeGraphAddNodeSearchEntry],
        query: &str,
    ) -> Option<NodeGraphAddNodeSearchMatch<'a>> {
        let selected = self.selected?;
        enabled_add_node_search_matches(entries, query)
            .into_iter()
            .find(|result| result.entry.id == selected)
    }

    /// Emits application-owned add-node request metadata for the selected entry.
    #[must_use]
    pub fn add_request(
        &self,
        entries: &[NodeGraphAddNodeSearchEntry],
        query: &str,
        insertion_point: GraphPoint,
    ) -> Option<NodeGraphAddNodeRequest> {
        let selected = self.selected_entry(entries, query)?.entry.id;
        Some(NodeGraphAddNodeRequest::new(selected, insertion_point))
    }
}

/// Application-owned request metadata for adding a node at a graph-space point.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NodeGraphAddNodeRequest {
    /// Selected application-owned add-node descriptor ID.
    pub descriptor_id: NodeGraphAddNodeDescriptorId,
    /// Graph-space insertion point, sanitized to finite coordinates.
    pub insertion_point: GraphPoint,
}

impl NodeGraphAddNodeRequest {
    /// Creates add-node request metadata.
    #[must_use]
    pub fn new(descriptor_id: NodeGraphAddNodeDescriptorId, insertion_point: GraphPoint) -> Self {
        Self {
            descriptor_id,
            insertion_point: insertion_point.sanitized(),
        }
    }
}

pub(crate) fn node_graph_add_node_query_terms(query: &str) -> Vec<String> {
    query
        .split_whitespace()
        .map(str::to_ascii_lowercase)
        .filter(|term| !term.is_empty())
        .collect()
}

pub(crate) fn node_graph_add_node_entry_matches_terms(
    entry: &NodeGraphAddNodeSearchEntry,
    terms: &[String],
) -> bool {
    terms
        .iter()
        .all(|term| node_graph_add_node_entry_contains_term(entry, term))
}

pub(crate) fn node_graph_add_node_entry_contains_term(
    entry: &NodeGraphAddNodeSearchEntry,
    term: &str,
) -> bool {
    find_ascii_case_insensitive(&entry.label, term).is_some()
        || entry
            .category
            .as_deref()
            .is_some_and(|category| find_ascii_case_insensitive(category, term).is_some())
        || entry
            .description
            .as_deref()
            .is_some_and(|description| find_ascii_case_insensitive(description, term).is_some())
        || entry
            .keywords
            .iter()
            .any(|keyword| find_ascii_case_insensitive(keyword, term).is_some())
}

pub(crate) fn node_graph_add_node_label_highlight(
    label: &str,
    terms: &[String],
) -> Option<NodeGraphAddNodeSearchHighlight> {
    terms
        .iter()
        .filter_map(|term| {
            find_ascii_case_insensitive(label, term)
                .map(|(start, end)| NodeGraphAddNodeSearchHighlight::new(start, end))
        })
        .min_by_key(|highlight| (highlight.start, highlight.end))
}

pub(crate) fn enabled_add_node_search_matches<'a>(
    entries: &'a [NodeGraphAddNodeSearchEntry],
    query: &str,
) -> Vec<NodeGraphAddNodeSearchMatch<'a>> {
    filter_node_graph_add_node_search_entries(entries, query)
        .into_iter()
        .filter(|result| result.entry.enabled)
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AddNodeSearchSelectionDirection {
    Next,
    Previous,
}

pub(crate) fn next_add_node_search_selection(
    matches: &[NodeGraphAddNodeSearchMatch<'_>],
    selected: Option<NodeGraphAddNodeDescriptorId>,
    direction: AddNodeSearchSelectionDirection,
) -> Option<NodeGraphAddNodeDescriptorId> {
    let len = matches.len();
    if len == 0 {
        return None;
    }

    let selected_index = selected.and_then(|selected| {
        matches
            .iter()
            .position(|result| result.entry.id == selected)
    });
    let next_index = match (selected_index, direction) {
        (Some(index), AddNodeSearchSelectionDirection::Next) => (index + 1) % len,
        (Some(0) | None, AddNodeSearchSelectionDirection::Previous) => len - 1,
        (Some(index), AddNodeSearchSelectionDirection::Previous) => index - 1,
        (None, AddNodeSearchSelectionDirection::Next) => 0,
    };

    Some(matches[next_index].entry.id)
}

pub(crate) fn find_ascii_case_insensitive(haystack: &str, needle: &str) -> Option<(usize, usize)> {
    if needle.is_empty() {
        return Some((0, 0));
    }

    let haystack = haystack.as_bytes();
    let needle = needle.as_bytes();
    haystack
        .windows(needle.len())
        .position(|window| window.eq_ignore_ascii_case(needle))
        .map(|start| (start, start + needle.len()))
}
