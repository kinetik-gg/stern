use stern_core::{Modifiers, RepaintRequest, SelectionGesturePhase};

use super::Ui;
use crate::node_graph::{
    NodeGraphEmissionError, NodeGraphHitTarget, NodeGraphHitTestError, NodeGraphSelectionOperation,
    NodeGraphSelectionTarget, NodeGraphWidget, NodeGraphWidgetConfig, NodeGraphWidgetIntent,
    NodeGraphWidgetOutput,
};

impl Ui<'_> {
    /// Prepares and validates one immutable node graph snapshot.
    ///
    /// # Errors
    ///
    /// Returns the static graph emission failure without producing partial output.
    pub fn prepare_node_graph_widget<'graph>(
        &self,
        config: NodeGraphWidgetConfig<'graph>,
    ) -> Result<NodeGraphWidget<'graph>, NodeGraphEmissionError> {
        NodeGraphWidget::prepare(config)
    }

    /// Evaluates one prepared graph, forwards its static presentation, and emits typed intents.
    ///
    /// # Errors
    ///
    /// Returns a structured hit test failure for an accepted pointer release.
    pub fn node_graph_widget(
        &mut self,
        widget: &NodeGraphWidget<'_>,
    ) -> Result<NodeGraphWidgetOutput, NodeGraphHitTestError> {
        let root = widget.widget_id();
        let bounds = widget.viewport().effective_bounds();
        let (mut gesture, mut clicked_releases) = self
            .runtime
            .captured_selection_gesture_with_clicked_releases(root, bounds, widget.disabled());

        if gesture.response.clicked {
            self.runtime.memory_mut().focus(root);
        }
        gesture.response.state.focused = self.memory().is_focused(root);
        gesture.response.state.selected = !widget.selection().is_empty();

        let mut hit = None;
        let mut intents = Vec::new();
        for action in gesture
            .actions
            .iter()
            .filter(|action| action.phase == SelectionGesturePhase::Release)
        {
            let Some(clicked_index) = clicked_releases
                .iter()
                .position(|ordinal| *ordinal == action.ordinal)
            else {
                continue;
            };
            clicked_releases.remove(clicked_index);
            let Some(position) = action.position else {
                continue;
            };
            let release_hit = widget.view().graph.hit_test_with_config(
                widget.viewport(),
                position,
                widget.hit_test(),
            )?;
            hit = Some(release_hit);
            if let Some(operation) = selection_operation(release_hit, action.modifiers) {
                intents.push(NodeGraphWidgetIntent::Selection(operation));
            }
        }

        self.extend(widget.output().primitives.iter().cloned());
        for mut node in widget.output().semantics.iter().cloned() {
            if node.id == root {
                node.focusable = !widget.disabled();
                node.state.focused = gesture.response.state.focused;
                node.state.disabled = widget.disabled();
            }
            self.push_semantic_node(node);
        }

        if !intents.is_empty() {
            self.request_repaint(RepaintRequest::NextFrame);
        }

        Ok(NodeGraphWidgetOutput {
            response: gesture.response,
            hit,
            intents,
        })
    }
}

fn selection_operation(
    hit: NodeGraphHitTarget,
    modifiers: Modifiers,
) -> Option<NodeGraphSelectionOperation> {
    if hit == NodeGraphHitTarget::Canvas {
        return Some(NodeGraphSelectionOperation::Clear);
    }
    let target = NodeGraphSelectionTarget::from_hit_target(hit)?;
    Some(if modifiers.shift {
        NodeGraphSelectionOperation::Extend(target)
    } else if modifiers.ctrl || modifiers.super_key {
        NodeGraphSelectionOperation::Toggle(target)
    } else {
        NodeGraphSelectionOperation::Replace(target)
    })
}
