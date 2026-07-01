#[allow(clippy::wildcard_imports)]
use super::common::*;

#[test]
fn add_node_search_filters_queries_deterministically() {
    let entries = vec![
        NodeGraphAddNodeSearchEntry::new(NodeGraphAddNodeDescriptorId::from_raw(1), "Color Ramp")
            .with_category("Color")
            .with_keywords(["gradient", "lookup"]),
        NodeGraphAddNodeSearchEntry::new(NodeGraphAddNodeDescriptorId::from_raw(2), "Add Number")
            .with_category("Math")
            .with_description("Sum numeric values")
            .with_keywords(["plus", "sum"]),
        NodeGraphAddNodeSearchEntry::new(NodeGraphAddNodeDescriptorId::from_raw(3), "Vector Add")
            .with_category("Math")
            .with_enabled(false),
        NodeGraphAddNodeSearchEntry::new(NodeGraphAddNodeDescriptorId::from_raw(4), "Blur")
            .with_category("Filter"),
    ];

    let first = filter_node_graph_add_node_search_entries(&entries, "ADD");
    let second = filter_node_graph_add_node_search_entries(&entries, "ADD");
    assert_eq!(first, second);
    assert_eq!(
        first
            .iter()
            .map(|result| result.entry.id)
            .collect::<Vec<_>>(),
        vec![
            NodeGraphAddNodeDescriptorId::from_raw(2),
            NodeGraphAddNodeDescriptorId::from_raw(3),
        ]
    );
    assert_eq!(
        first[0].label_highlight,
        Some(NodeGraphAddNodeSearchHighlight::new(0, 3))
    );
    assert_eq!(
        first[1].label_highlight,
        Some(NodeGraphAddNodeSearchHighlight::new(7, 10))
    );

    let cross_field = filter_node_graph_add_node_search_entries(&entries, " math sum ");
    assert_eq!(
        cross_field
            .iter()
            .map(|result| result.entry.id)
            .collect::<Vec<_>>(),
        vec![NodeGraphAddNodeDescriptorId::from_raw(2)]
    );
    assert!(entries[1].matches_query("PLUS"));
    assert_eq!(
        entries[1].label_highlight("number"),
        Some(NodeGraphAddNodeSearchHighlight::new(4, 10))
    );
}

#[test]
fn add_node_search_selection_helpers_skip_disabled_entries() {
    let entries = vec![
        NodeGraphAddNodeSearchEntry::new(NodeGraphAddNodeDescriptorId::from_raw(1), "Add Number"),
        NodeGraphAddNodeSearchEntry::new(NodeGraphAddNodeDescriptorId::from_raw(2), "Add Disabled")
            .with_enabled(false),
        NodeGraphAddNodeSearchEntry::new(NodeGraphAddNodeDescriptorId::from_raw(3), "Add Vector"),
    ];

    let first = NodeGraphAddNodeSearchSelection::select_first(&entries, "add");
    assert_eq!(
        first.selected,
        Some(NodeGraphAddNodeDescriptorId::from_raw(1))
    );

    let next = first.select_next(&entries, "add");
    assert_eq!(
        next.selected,
        Some(NodeGraphAddNodeDescriptorId::from_raw(3))
    );
    let wrapped = next.select_next(&entries, "add");
    assert_eq!(
        wrapped.selected,
        Some(NodeGraphAddNodeDescriptorId::from_raw(1))
    );
    let previous = wrapped.select_previous(&entries, "add");
    assert_eq!(
        previous.selected,
        Some(NodeGraphAddNodeDescriptorId::from_raw(3))
    );

    let disabled =
        NodeGraphAddNodeSearchSelection::from_selected(NodeGraphAddNodeDescriptorId::from_raw(2));
    assert!(disabled.selected_entry(&entries, "add").is_none());
    assert!(
        disabled
            .add_request(&entries, "add", GraphPoint::new(1.0, 2.0))
            .is_none()
    );
}

#[test]
fn selected_add_node_entry_emits_request_with_graph_insertion_point() {
    let entries = vec![
        NodeGraphAddNodeSearchEntry::new(NodeGraphAddNodeDescriptorId::from_raw(1), "Blur"),
        NodeGraphAddNodeSearchEntry::new(NodeGraphAddNodeDescriptorId::from_raw(2), "Merge"),
    ];
    let selection = NodeGraphAddNodeSearchSelection::select_first(&entries, "blur");

    let request = selection
        .add_request(&entries, "blur", GraphPoint::new(12.5, -7.25))
        .expect("selected add-node request");
    assert_eq!(
        request.descriptor_id,
        NodeGraphAddNodeDescriptorId::from_raw(1)
    );
    assert_graph_point_close(request.insertion_point, GraphPoint::new(12.5, -7.25));

    let sanitized = selection
        .add_request(&entries, "blur", GraphPoint::new(f32::NAN, f32::INFINITY))
        .expect("sanitized add-node request");
    assert_graph_point_close(sanitized.insertion_point, GraphPoint::ZERO);
}

#[test]
fn add_node_search_empty_query_and_no_match_states_are_deterministic() {
    let entries = vec![
        NodeGraphAddNodeSearchEntry::new(
            NodeGraphAddNodeDescriptorId::from_raw(1),
            "Disabled First",
        )
        .with_enabled(false),
        NodeGraphAddNodeSearchEntry::new(
            NodeGraphAddNodeDescriptorId::from_raw(2),
            "Visible Second",
        ),
    ];

    let empty = filter_node_graph_add_node_search_entries(&entries, "");
    let whitespace = filter_node_graph_add_node_search_entries(&entries, "   ");
    assert_eq!(empty, whitespace);
    assert_eq!(
        empty
            .iter()
            .map(|result| (result.entry.id, result.label_highlight))
            .collect::<Vec<_>>(),
        vec![
            (NodeGraphAddNodeDescriptorId::from_raw(1), None),
            (NodeGraphAddNodeDescriptorId::from_raw(2), None),
        ]
    );
    assert_eq!(
        NodeGraphAddNodeSearchSelection::select_first(&entries, "").selected,
        Some(NodeGraphAddNodeDescriptorId::from_raw(2))
    );

    let no_match = filter_node_graph_add_node_search_entries(&entries, "missing");
    assert!(no_match.is_empty());
    let selection = NodeGraphAddNodeSearchSelection::select_first(&entries, "missing");
    assert_eq!(selection.selected, None);
    assert!(
        selection
            .add_request(&entries, "missing", GraphPoint::new(1.0, 2.0))
            .is_none()
    );
    assert_eq!(
        selection.select_next(&entries, "missing"),
        NodeGraphAddNodeSearchSelection::new()
    );
}
