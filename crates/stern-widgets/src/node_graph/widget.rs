use stern_core::{Response, WidgetId};

use super::{
    NodeGraphEmissionError, NodeGraphHitTarget, NodeGraphHitTestConfig, NodeGraphSelection,
    NodeGraphSelectionOperation, NodeGraphStaticOutput, NodeGraphStaticView, NodeGraphViewport,
};

/// Caller-owned configuration for one retained node graph widget.
#[derive(Debug, Clone, PartialEq)]
pub struct NodeGraphWidgetConfig<'graph> {
    view: NodeGraphStaticView<'graph>,
    disabled: bool,
    hit_test: NodeGraphHitTestConfig,
}

impl<'graph> NodeGraphWidgetConfig<'graph> {
    /// Creates an enabled widget from a caller-owned static graph snapshot.
    #[must_use]
    pub fn new(view: NodeGraphStaticView<'graph>) -> Self {
        Self {
            view,
            disabled: false,
            hit_test: NodeGraphHitTestConfig::new(),
        }
    }

    /// Sets whether graph interaction is disabled.
    #[must_use]
    pub const fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Replaces the screen-space hit testing recipe.
    #[must_use]
    pub const fn with_hit_test(mut self, hit_test: NodeGraphHitTestConfig) -> Self {
        self.hit_test = hit_test;
        self
    }
}

/// Immutable frame-local node graph widget.
#[derive(Debug, Clone, PartialEq)]
pub struct NodeGraphWidget<'graph> {
    view: NodeGraphStaticView<'graph>,
    output: NodeGraphStaticOutput,
    disabled: bool,
    hit_test: NodeGraphHitTestConfig,
}

impl<'graph> NodeGraphWidget<'graph> {
    pub(crate) fn prepare(
        config: NodeGraphWidgetConfig<'graph>,
    ) -> Result<Self, NodeGraphEmissionError> {
        let output = config.view.emit()?;
        Ok(Self {
            view: config.view,
            output,
            disabled: config.disabled,
            hit_test: config.hit_test,
        })
    }

    /// Returns the stable graph root identity.
    #[must_use]
    pub const fn widget_id(&self) -> WidgetId {
        self.view.id
    }

    /// Returns the frozen viewport shared by paint, hit testing, and semantics.
    #[must_use]
    pub const fn viewport(&self) -> NodeGraphViewport {
        self.view.viewport
    }

    /// Returns the caller-owned selection snapshot used for this frame.
    #[must_use]
    pub const fn selection(&self) -> &NodeGraphSelection {
        &self.view.selection
    }

    /// Returns whether graph interaction is disabled.
    #[must_use]
    pub const fn disabled(&self) -> bool {
        self.disabled
    }

    pub(crate) const fn view(&self) -> &NodeGraphStaticView<'graph> {
        &self.view
    }

    pub(crate) const fn output(&self) -> &NodeGraphStaticOutput {
        &self.output
    }

    pub(crate) const fn hit_test(&self) -> NodeGraphHitTestConfig {
        self.hit_test
    }
}

/// Typed application intent emitted by the retained node graph widget.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeGraphWidgetIntent {
    /// Apply a pure operation to caller-owned graph selection state.
    Selection(NodeGraphSelectionOperation),
}

/// Output from one retained node graph widget evaluation.
#[derive(Debug, Clone, PartialEq)]
pub struct NodeGraphWidgetOutput {
    /// Common interaction response for the graph root.
    pub response: Response,
    /// Exact target under the accepted release, when one was emitted.
    pub hit: Option<NodeGraphHitTarget>,
    /// Ordered typed application intents.
    pub intents: Vec<NodeGraphWidgetIntent>,
}
