use stern_core::{
    DomainDragGestureAction, DomainDragGesturePhase, Key, KeyState, Modifiers, RepaintRequest,
    UiInputEvent,
};

use super::Ui;
use crate::node_graph::{
    NodeGraphConnectionBegin, NodeGraphConnectionCancel, NodeGraphConnectionCancelReason,
    NodeGraphConnectionCapture, NodeGraphConnectionController, NodeGraphConnectionIntent,
    NodeGraphConnectionPreview, NodeGraphConnectionRejection, NodeGraphCreateLinkRequest,
    NodeGraphEmissionError, NodeGraphHitTarget, NodeGraphHitTestError, NodeGraphLinkDraftOutcome,
    NodeGraphLinkDraftTargetError, NodeGraphLinkEditRequest, NodeGraphSelectionOperation,
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

    /// Evaluates one prepared graph without retaining connection-drag state.
    ///
    /// # Errors
    ///
    /// Returns a structured hit test failure for an accepted pointer release.
    pub fn node_graph_widget(
        &mut self,
        widget: &NodeGraphWidget<'_>,
    ) -> Result<NodeGraphWidgetOutput, NodeGraphHitTestError> {
        self.resolve_node_graph_widget(widget, None)
    }

    /// Evaluates one prepared graph with retained typed connection editing.
    ///
    /// # Errors
    ///
    /// Returns a structured hit test failure while selecting or targeting a connection.
    pub fn node_graph_widget_with_connections(
        &mut self,
        widget: &NodeGraphWidget<'_>,
        controller: &mut NodeGraphConnectionController,
    ) -> Result<NodeGraphWidgetOutput, NodeGraphHitTestError> {
        self.resolve_node_graph_widget(widget, Some(controller))
    }

    fn resolve_node_graph_widget(
        &mut self,
        widget: &NodeGraphWidget<'_>,
        controller: Option<&mut NodeGraphConnectionController>,
    ) -> Result<NodeGraphWidgetOutput, NodeGraphHitTestError> {
        let root = widget.widget_id();
        let bounds = widget.viewport().effective_bounds();
        let gesture = self
            .runtime
            .captured_domain_drag_gesture(root, bounds, widget.disabled());

        let mut response = gesture.response;
        if response.clicked {
            self.runtime.memory_mut().focus(root);
        }
        response.state.focused = self.memory().is_focused(root);
        response.state.selected = !widget.selection().is_empty();

        let mut hit = None;
        let mut intents = Vec::new();
        for action in gesture.actions.iter().filter(|action| {
            action.phase == DomainDragGesturePhase::Release && action.release_clicked
        }) {
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

        let connection_intents = if let Some(controller) = controller {
            self.resolve_node_graph_connections(widget, controller, &gesture.actions)?
        } else {
            Vec::new()
        };

        self.extend(widget.output().primitives.iter().cloned());
        for mut node in widget.output().semantics.iter().cloned() {
            if node.id == root {
                node.focusable = !widget.disabled();
                node.state.focused = response.state.focused;
                node.state.disabled = widget.disabled();
                if widget.read_only() {
                    node.description = Some("Read-only".to_owned());
                }
            }
            self.push_semantic_node(node);
        }

        if !intents.is_empty() || !connection_intents.is_empty() {
            self.request_repaint(RepaintRequest::NextFrame);
        }

        Ok(NodeGraphWidgetOutput {
            response,
            hit,
            intents,
            connection_intents,
        })
    }

    fn resolve_node_graph_connections(
        &mut self,
        widget: &NodeGraphWidget<'_>,
        controller: &mut NodeGraphConnectionController,
        actions: &[DomainDragGestureAction],
    ) -> Result<Vec<NodeGraphConnectionIntent>, NodeGraphHitTestError> {
        let mut intents = Vec::new();
        if controller
            .capture
            .as_ref()
            .is_some_and(|capture| capture.owner != widget.widget_id())
        {
            push_connection_cancel(
                controller,
                NodeGraphConnectionCancelReason::CaptureLost,
                &mut intents,
            );
        }

        if widget.disabled() || widget.read_only() {
            let reason = if widget.disabled() {
                NodeGraphConnectionCancelReason::Disabled
            } else {
                NodeGraphConnectionCancelReason::ReadOnly
            };
            if controller.capture.is_some() {
                push_connection_cancel(controller, reason, &mut intents);
                self.cancel_pointer_interaction();
            }
            return Ok(intents);
        }

        let escape_ordinal = self.input().events.iter().position(|event| {
            matches!(event, UiInputEvent::Key(event) if event.state == KeyState::Pressed && !event.repeat && matches!(event.key, Key::Escape))
        });
        let drag_crossed_threshold = self.memory().is_drag_source(widget.widget_id())
            || self.memory().released_drag_source() == Some(widget.widget_id());

        for action in actions {
            if escape_ordinal
                .is_some_and(|escape| action.ordinal.is_some_and(|ordinal| ordinal >= escape))
            {
                break;
            }
            Self::apply_node_graph_connection_action(
                widget,
                controller,
                action,
                drag_crossed_threshold,
                &mut intents,
            )?;
        }

        if escape_ordinal.is_some() && controller.capture.is_some() {
            push_connection_cancel(
                controller,
                NodeGraphConnectionCancelReason::Escape,
                &mut intents,
            );
            self.cancel_pointer_interaction();
        }

        if controller.capture.is_some()
            && self.memory().pointer_capture() != Some(widget.widget_id())
        {
            push_connection_cancel(
                controller,
                NodeGraphConnectionCancelReason::CaptureLost,
                &mut intents,
            );
        }

        Ok(intents)
    }

    fn apply_node_graph_connection_action(
        widget: &NodeGraphWidget<'_>,
        controller: &mut NodeGraphConnectionController,
        action: &DomainDragGestureAction,
        drag_crossed_threshold: bool,
        intents: &mut Vec<NodeGraphConnectionIntent>,
    ) -> Result<(), NodeGraphHitTestError> {
        match action.phase {
            DomainDragGesturePhase::Press => {
                let Some(position) = action.position else {
                    return Ok(());
                };
                let hit = widget.view().graph.hit_test_with_config(
                    widget.viewport(),
                    position,
                    widget.hit_test(),
                )?;
                let NodeGraphHitTarget::Port(start) = hit else {
                    return Ok(());
                };
                if let Ok(draft) = widget.view().graph.start_link_draft(start, position) {
                    controller.capture = Some(NodeGraphConnectionCapture {
                        owner: widget.widget_id(),
                        viewport: widget.viewport(),
                        hit_test: widget.hit_test(),
                        draft,
                        started: false,
                    });
                }
            }
            DomainDragGesturePhase::Move => {
                let Some(capture) = controller.capture.as_mut() else {
                    return Ok(());
                };
                if !capture.started && !drag_crossed_threshold {
                    return Ok(());
                }
                push_connection_begin(capture, intents);
                resolve_connection_candidate(widget, capture, action.position, intents)?;
            }
            DomainDragGesturePhase::Release => {
                let Some(mut capture) = controller.capture.take() else {
                    return Ok(());
                };
                if !capture.started && !drag_crossed_threshold {
                    return Ok(());
                }
                push_connection_begin(&mut capture, intents);
                let accepted =
                    resolve_connection_candidate(widget, &mut capture, action.position, intents)?;
                if let Some(request) = accepted {
                    intents.push(NodeGraphConnectionIntent::Commit(request));
                }
            }
            DomainDragGesturePhase::Cancel => push_connection_cancel(
                controller,
                NodeGraphConnectionCancelReason::CaptureLost,
                intents,
            ),
        }
        Ok(())
    }
}

fn push_connection_begin(
    capture: &mut NodeGraphConnectionCapture,
    intents: &mut Vec<NodeGraphConnectionIntent>,
) {
    if capture.started {
        return;
    }
    capture.started = true;
    intents.push(NodeGraphConnectionIntent::Begin(NodeGraphConnectionBegin {
        graph: capture.owner,
        start: capture.draft.start,
        viewport: capture.viewport,
    }));
}

fn resolve_connection_candidate(
    widget: &NodeGraphWidget<'_>,
    capture: &mut NodeGraphConnectionCapture,
    position: Option<stern_core::Point>,
    intents: &mut Vec<NodeGraphConnectionIntent>,
) -> Result<Option<NodeGraphCreateLinkRequest>, NodeGraphHitTestError> {
    let position = position.unwrap_or(capture.draft.current_pointer);
    capture.draft = match capture.draft.resolve_hover_target_with_config(
        widget.view().graph,
        capture.viewport,
        position,
        capture.hit_test,
    ) {
        Ok(draft) => draft,
        Err(NodeGraphLinkDraftTargetError::HitTest(error)) => return Err(error),
        Err(NodeGraphLinkDraftTargetError::Endpoint(error)) => {
            intents.push(NodeGraphConnectionIntent::Rejected(
                NodeGraphConnectionRejection::Endpoint(error),
            ));
            return Ok(None);
        }
    };
    intents.push(NodeGraphConnectionIntent::Preview(
        NodeGraphConnectionPreview {
            graph: capture.owner,
            draft: capture.draft.clone(),
            viewport: capture.viewport,
        },
    ));

    let completed = match capture.draft.complete() {
        NodeGraphLinkDraftOutcome::Completed(completed) => completed,
        NodeGraphLinkDraftOutcome::Rejected(rejected) => {
            intents.push(NodeGraphConnectionIntent::Rejected(
                NodeGraphConnectionRejection::Draft(rejected),
            ));
            return Ok(None);
        }
        NodeGraphLinkDraftOutcome::Cancelled(_) => return Ok(None),
    };
    let request = match widget
        .view()
        .graph
        .create_link_request(completed.from.endpoint, completed.to.endpoint)
    {
        Ok(NodeGraphLinkEditRequest::CreateLink(request)) => request,
        Ok(_) => return Ok(None),
        Err(error) => {
            intents.push(NodeGraphConnectionIntent::Rejected(
                NodeGraphConnectionRejection::Link(error),
            ));
            return Ok(None);
        }
    };
    intents.push(NodeGraphConnectionIntent::Accepted(request));
    Ok(Some(request))
}

fn push_connection_cancel(
    controller: &mut NodeGraphConnectionController,
    reason: NodeGraphConnectionCancelReason,
    intents: &mut Vec<NodeGraphConnectionIntent>,
) {
    let Some(capture) = controller.capture.take() else {
        return;
    };
    if !capture.started {
        return;
    }
    let NodeGraphLinkDraftOutcome::Cancelled(draft) = capture.draft.cancel() else {
        return;
    };
    intents.push(NodeGraphConnectionIntent::Cancel(
        NodeGraphConnectionCancel {
            graph: capture.owner,
            reason,
            draft,
            viewport: capture.viewport,
        },
    ));
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
