use super::{
    ActionId, BTreeMap, Dock, DockPlacement, FrameId, PanelInstanceId, PanelInstanceSnapshot,
    PanelOpenDecision, PanelPolicyContext, PanelPolicyResolution, PanelTypeId, Size, StaticIcon,
    resolve_panel_open_decision, resolve_panel_policy_context,
};

/// Broad grouping used when presenting available panel types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PanelTypeCategory {
    /// General editor workspace panels.
    General,
    /// Scene, hierarchy, outliner, or object tree panels.
    Hierarchy,
    /// Property, inspector, or details panels.
    Inspector,
    /// Viewport or preview panels.
    Viewport,
    /// Asset, file, media, or library panels.
    Assets,
    /// Timeline, graph, sequencer, or animation panels.
    Timeline,
    /// Console, log, diagnostics, or job panels.
    Diagnostics,
    /// Application-defined category label.
    Custom(String),
}

/// Whether a panel type can have one or many open instances.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelInstancePolicy {
    /// Only one open instance of this panel type should exist in a workspace.
    Singleton,
    /// Multiple open instances of this panel type may exist in a workspace.
    MultiInstance,
}

/// Workspace contexts where a panel type may be opened.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PanelWorkspaceContext {
    /// A docked frame tab inside the editor dock.
    Docked,
    /// A modal-like toolkit context.
    Modal,
    /// A future floating editor surface.
    Floating,
}

/// Workspace placement hint for a panel type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelDockHint {
    /// Prefer opening as a tab in an existing frame.
    Tab,
    /// Prefer opening as a new split relative to the active frame.
    Split(DockPlacement),
}

/// Whether a panel type may expose close affordances.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelClosePolicy {
    /// The panel may be closed or dismissed by the user.
    Closable,
    /// The panel is required by the workspace and should not expose close.
    Required,
}

/// Whether a panel type may expose duplicate/open-another affordances.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelDuplicatePolicy {
    /// The panel type may be duplicated.
    Allowed,
    /// The panel type should not expose duplicate.
    Denied,
}

/// Whether a panel type may use future floating-surface affordances.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelFloatPolicy {
    /// Floating is not currently available for this panel type.
    Unavailable,
    /// The panel type may be floated when a platform surface exists.
    Allowed,
}

/// Editor workspace metadata for a developer-declared panel kind.
///
/// The descriptor is a UI contract only: applications own panel content,
/// instance creation, action execution, and persistence decisions.
#[derive(Debug, Clone, PartialEq)]
pub struct PanelTypeDescriptor {
    /// Stable developer-declared panel type identity.
    pub id: PanelTypeId,
    /// Display title used in menus, palettes, and default tabs.
    pub title: String,
    /// Optional static icon for panel picker and tab chrome.
    pub icon: Option<StaticIcon>,
    /// Presentation category for panel pickers and command palettes.
    pub category: PanelTypeCategory,
    /// Singleton or multi-instance workspace policy.
    pub instance_policy: PanelInstancePolicy,
    /// Initial preferred logical size when the workspace needs one.
    pub default_size: Size,
    /// Contexts where this panel type may be opened.
    pub allowed_contexts: Vec<PanelWorkspaceContext>,
    /// Dock placement preferences in priority order.
    pub dock_hints: Vec<PanelDockHint>,
    /// Close/dismiss affordance policy.
    pub close_policy: PanelClosePolicy,
    /// Duplicate/open-another affordance policy.
    pub duplicate_policy: PanelDuplicatePolicy,
    /// Future floating-surface affordance policy.
    pub float_policy: PanelFloatPolicy,
    /// Optional application-owned action that opens this panel type.
    pub default_open_action: Option<ActionId>,
}

impl PanelTypeDescriptor {
    /// Creates a panel type descriptor with deterministic editor defaults.
    #[must_use]
    pub fn new(id: PanelTypeId, title: impl Into<String>) -> Self {
        Self {
            id,
            title: title.into(),
            icon: None,
            category: PanelTypeCategory::General,
            instance_policy: PanelInstancePolicy::MultiInstance,
            default_size: Size::new(320.0, 240.0),
            allowed_contexts: vec![PanelWorkspaceContext::Docked],
            dock_hints: vec![PanelDockHint::Tab],
            close_policy: PanelClosePolicy::Closable,
            duplicate_policy: PanelDuplicatePolicy::Allowed,
            float_policy: PanelFloatPolicy::Unavailable,
            default_open_action: None,
        }
    }

    /// Sets the optional static icon.
    #[must_use]
    pub fn with_icon(mut self, icon: impl Into<StaticIcon>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Sets the presentation category.
    #[must_use]
    pub fn with_category(mut self, category: PanelTypeCategory) -> Self {
        self.category = category;
        self
    }

    /// Sets singleton or multi-instance policy.
    #[must_use]
    pub const fn with_instance_policy(mut self, policy: PanelInstancePolicy) -> Self {
        self.instance_policy = policy;
        self
    }

    /// Sets the preferred default logical size.
    #[must_use]
    pub const fn with_default_size(mut self, size: Size) -> Self {
        self.default_size = size;
        self
    }

    /// Replaces the allowed workspace contexts.
    #[must_use]
    pub fn with_allowed_contexts(
        mut self,
        contexts: impl IntoIterator<Item = PanelWorkspaceContext>,
    ) -> Self {
        self.allowed_contexts = contexts.into_iter().collect();
        self
    }

    /// Replaces the dock placement hints.
    #[must_use]
    pub fn with_dock_hints(mut self, hints: impl IntoIterator<Item = PanelDockHint>) -> Self {
        self.dock_hints = hints.into_iter().collect();
        self
    }

    /// Sets the close/dismiss affordance policy.
    #[must_use]
    pub const fn with_close_policy(mut self, policy: PanelClosePolicy) -> Self {
        self.close_policy = policy;
        self
    }

    /// Sets the duplicate/open-another affordance policy.
    #[must_use]
    pub const fn with_duplicate_policy(mut self, policy: PanelDuplicatePolicy) -> Self {
        self.duplicate_policy = policy;
        self
    }

    /// Sets the future floating-surface affordance policy.
    #[must_use]
    pub const fn with_float_policy(mut self, policy: PanelFloatPolicy) -> Self {
        self.float_policy = policy;
        self
    }

    /// Sets the optional application-owned default open action.
    #[must_use]
    pub fn with_default_open_action(mut self, action: ActionId) -> Self {
        self.default_open_action = Some(action);
        self
    }
}

/// App-owned open action metadata derived from a panel type descriptor.
///
/// This is presentation data only. Applications still own action registration,
/// dispatch, and panel instance creation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PanelOpenActionMetadata {
    /// Panel type the action would open or focus.
    pub panel_type: PanelTypeId,
    /// Display label for menus, palettes, or pickers.
    pub title: String,
    /// Optional static icon from the descriptor.
    pub icon: Option<StaticIcon>,
    /// Presentation category from the descriptor.
    pub category: PanelTypeCategory,
    /// Optional application-owned action from the descriptor.
    pub default_open_action: Option<ActionId>,
}

impl PanelOpenActionMetadata {
    /// Creates open action metadata from a panel type descriptor.
    #[must_use]
    pub fn from_descriptor(descriptor: &PanelTypeDescriptor) -> Self {
        Self {
            panel_type: descriptor.id,
            title: descriptor.title.clone(),
            icon: descriptor.icon,
            category: descriptor.category.clone(),
            default_open_action: descriptor.default_open_action.clone(),
        }
    }
}

/// Error returned when building a [`PanelRegistry`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelRegistryError {
    /// Two descriptors used the same panel type ID.
    DuplicatePanelTypeDescriptor {
        /// Duplicated panel type identity.
        panel_type: PanelTypeId,
        /// First descriptor position using this panel type ID.
        first_index: usize,
        /// Later descriptor position using this panel type ID.
        duplicate_index: usize,
    },
}

/// Deterministic metadata registry for developer-declared panel types.
///
/// The registry preserves descriptor order for presentation while providing
/// stable lookup by [`PanelTypeId`]. It does not execute actions, create panel
/// instances, or own application panel content.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PanelRegistry {
    descriptors: Vec<PanelTypeDescriptor>,
    descriptors_by_id: BTreeMap<PanelTypeId, usize>,
}

impl PanelRegistry {
    /// Creates an empty panel registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a registry from descriptors in deterministic presentation order.
    ///
    /// # Errors
    ///
    /// Returns [`PanelRegistryError::DuplicatePanelTypeDescriptor`] when a
    /// later descriptor repeats an earlier [`PanelTypeId`].
    pub fn from_descriptors(
        descriptors: impl IntoIterator<Item = PanelTypeDescriptor>,
    ) -> Result<Self, PanelRegistryError> {
        let mut registry = Self::new();

        for descriptor in descriptors {
            registry.register(descriptor)?;
        }

        Ok(registry)
    }

    /// Registers one descriptor at the end of the presentation order.
    ///
    /// # Errors
    ///
    /// Returns [`PanelRegistryError::DuplicatePanelTypeDescriptor`] when the
    /// descriptor repeats an existing [`PanelTypeId`].
    pub fn register(&mut self, descriptor: PanelTypeDescriptor) -> Result<(), PanelRegistryError> {
        let index = self.descriptors.len();
        match self.descriptors_by_id.entry(descriptor.id) {
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(index);
                self.descriptors.push(descriptor);
                Ok(())
            }
            std::collections::btree_map::Entry::Occupied(entry) => {
                Err(PanelRegistryError::DuplicatePanelTypeDescriptor {
                    panel_type: descriptor.id,
                    first_index: *entry.get(),
                    duplicate_index: index,
                })
            }
        }
    }

    /// Returns descriptors in presentation order.
    #[must_use]
    pub fn descriptors(&self) -> &[PanelTypeDescriptor] {
        &self.descriptors
    }

    /// Iterates descriptors in presentation order.
    pub fn iter(&self) -> impl Iterator<Item = &PanelTypeDescriptor> {
        self.descriptors.iter()
    }

    /// Returns the descriptor for a panel type ID.
    #[must_use]
    pub fn descriptor(&self, panel_type: PanelTypeId) -> Option<&PanelTypeDescriptor> {
        self.descriptors_by_id
            .get(&panel_type)
            .map(|index| &self.descriptors[*index])
    }

    /// Returns unique categories in first-seen descriptor order.
    #[must_use]
    pub fn categories(&self) -> Vec<&PanelTypeCategory> {
        let mut categories = Vec::new();
        for descriptor in &self.descriptors {
            if !categories.contains(&&descriptor.category) {
                categories.push(&descriptor.category);
            }
        }
        categories
    }

    /// Iterates descriptors in a category while preserving descriptor order.
    pub fn descriptors_in_category<'a>(
        &'a self,
        category: &'a PanelTypeCategory,
    ) -> impl Iterator<Item = &'a PanelTypeDescriptor> + 'a {
        self.descriptors
            .iter()
            .filter(move |descriptor| &descriptor.category == category)
    }

    /// Iterates app-owned open action metadata in descriptor order.
    pub fn open_actions(&self) -> impl Iterator<Item = PanelOpenActionMetadata> + '_ {
        self.descriptors
            .iter()
            .map(PanelOpenActionMetadata::from_descriptor)
    }

    /// Resolves whether opening a registered panel type should focus or open.
    ///
    /// The returned decision is metadata only; applications decide whether and
    /// how to execute any action or create any panel instance.
    #[must_use]
    pub fn resolve_open_decision(
        &self,
        panel_type: PanelTypeId,
        panel_instances: &[PanelInstanceSnapshot],
        dock: &Dock,
        context: PanelWorkspaceContext,
    ) -> Option<PanelOpenDecision> {
        resolve_panel_open_decision(self.descriptor(panel_type)?, panel_instances, dock, context)
    }

    /// Resolves panel affordances and app-owned request metadata for one open
    /// panel instance.
    ///
    /// The returned value is pure metadata. It does not create, close,
    /// duplicate, float, focus, or otherwise mutate panels or application state.
    #[must_use]
    pub fn resolve_policy_context(
        &self,
        panel_instances: &[PanelInstanceSnapshot],
        dock: &Dock,
        panel_instance: PanelInstanceId,
        frame: FrameId,
        workspace_context: PanelWorkspaceContext,
    ) -> PanelPolicyResolution {
        resolve_panel_policy_context(PanelPolicyContext::new(
            self,
            panel_instances,
            dock,
            panel_instance,
            frame,
            workspace_context,
        ))
    }
}
