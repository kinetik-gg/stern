use stern::core::{ActionDescriptor, ActionInvocation};

const EDIT_ACTION: &str = "workspace.edit";
const GRAPH_ACTION: &str = "workspace.graph";
const APPLY_ACTION: &str = "shared.apply";

/// Stable identity of a maintained demo workspace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DemoWorkspace {
    /// Document editing workspace.
    Edit,
    /// Graph editing workspace.
    Graph,
}

impl DemoWorkspace {
    /// Returns the pinned workspace identity.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::Edit => "edit-workspace",
            Self::Graph => "graph-workspace",
        }
    }
}

/// Shared deterministic application model used by every demo workspace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DemoApplicationModel {
    workspace: DemoWorkspace,
    applied_revision: u32,
}

impl DemoApplicationModel {
    /// Creates the deterministic initial application state.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            workspace: DemoWorkspace::Edit,
            applied_revision: 0,
        }
    }

    /// Returns the active workspace.
    #[must_use]
    pub const fn workspace(&self) -> DemoWorkspace {
        self.workspace
    }

    /// Returns the shared applied revision.
    #[must_use]
    pub const fn applied_revision(&self) -> u32 {
        self.applied_revision
    }

    /// Executes one recognized application action.
    pub fn execute(&mut self, invocation: &ActionInvocation) -> bool {
        match invocation.action_id.as_str() {
            EDIT_ACTION => self.workspace = DemoWorkspace::Edit,
            GRAPH_ACTION => self.workspace = DemoWorkspace::Graph,
            APPLY_ACTION => {
                self.applied_revision = self.applied_revision.saturating_add(1);
            }
            _ => return false,
        }
        true
    }
}

impl Default for DemoApplicationModel {
    fn default() -> Self {
        Self::new()
    }
}

/// Single descriptor registry for the demo's existing application actions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DemoActionRegistry {
    descriptors: [ActionDescriptor; 3],
}

impl DemoActionRegistry {
    /// Creates the exact existing demo action set in stable order.
    #[must_use]
    pub fn new() -> Self {
        Self {
            descriptors: [
                ActionDescriptor::new(EDIT_ACTION, "Edit Workspace"),
                ActionDescriptor::new(GRAPH_ACTION, "Graph Workspace"),
                ActionDescriptor::new(APPLY_ACTION, "Apply Shared State"),
            ],
        }
    }

    /// Returns the Edit workspace action descriptor.
    #[must_use]
    pub const fn edit_workspace(&self) -> &ActionDescriptor {
        &self.descriptors[0]
    }

    /// Returns the Graph workspace action descriptor.
    #[must_use]
    pub const fn graph_workspace(&self) -> &ActionDescriptor {
        &self.descriptors[1]
    }

    /// Returns the shared-state apply action descriptor.
    #[must_use]
    pub const fn apply_shared_state(&self) -> &ActionDescriptor {
        &self.descriptors[2]
    }

    /// Iterates over descriptors in stable registry order.
    #[must_use]
    pub fn iter(&self) -> impl ExactSizeIterator<Item = &ActionDescriptor> {
        self.descriptors.iter()
    }
}

impl Default for DemoActionRegistry {
    fn default() -> Self {
        Self::new()
    }
}
