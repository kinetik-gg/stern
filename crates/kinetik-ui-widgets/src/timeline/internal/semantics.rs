#[allow(clippy::wildcard_imports)]
use super::*;

pub(crate) fn apply_timeline_semantic_state(
    node: &mut SemanticNode,
    state: TimelineDescriptorState,
) {
    node.state.disabled = state.disabled;
    node.state.selected = state.selected;
    if state.read_only {
        node.description = Some("Read-only".to_owned());
    }
}
