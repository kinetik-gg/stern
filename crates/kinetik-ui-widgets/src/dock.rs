//! `Dock`, `Frame`, and `Panel` models for editor layouts.

use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet},
};

use kinetik_ui_core::{ActionId, Axis, IconId, Point, Rect, Size, Vec2};

const DEFAULT_SPLIT_RATIO: f32 = 0.5;
const DEFAULT_SPLIT_MINIMUM: f32 = 100.0;
const DEFAULT_SPLITTER_THICKNESS: f32 = 6.0;
const DROP_EDGE_FRACTION: f32 = 0.25;

/// Data-only policy for dock interaction affordances.
///
/// The default policy preserves the built-in dock behavior. Invalid numeric
/// values are sanitized by policy-aware helpers before use.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct DockInteractionPolicy {
    /// Policy for drag-to-dock drop target resolution.
    pub drop_targets: DockDropTargetPolicy,
    /// Policy for splitter drag and context action affordances.
    pub splitters: DockSplitterInteractionPolicy,
}

/// Data-only policy for drag-to-dock target resolution.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DockDropTargetPolicy {
    /// Fraction of a frame edge that resolves to split insertion.
    pub edge_fraction: f32,
    /// Whether center drop targets may resolve to tab merge targets.
    pub allow_tab_merge: bool,
    /// Whether edge drop targets may resolve to split insertion targets.
    pub allow_split_insertion: bool,
}

impl Default for DockDropTargetPolicy {
    fn default() -> Self {
        Self {
            edge_fraction: DROP_EDGE_FRACTION,
            allow_tab_merge: true,
            allow_split_insertion: true,
        }
    }
}

/// Data-only policy for splitter interaction affordances.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DockSplitterInteractionPolicy {
    /// Whether splitter drags may resize split ratios.
    pub allow_resize: bool,
    /// Whether splitter context metadata may enable join actions.
    pub allow_join: bool,
    /// Whether splitter context metadata may enable swap actions.
    pub allow_swap: bool,
}

impl Default for DockSplitterInteractionPolicy {
    fn default() -> Self {
        Self {
            allow_resize: true,
            allow_join: true,
            allow_swap: true,
        }
    }
}

impl DockInteractionPolicy {
    /// Returns a copy with deterministic, valid numeric values.
    #[must_use]
    pub fn sanitized(self) -> Self {
        Self {
            drop_targets: DockDropTargetPolicy {
                edge_fraction: sanitize_drop_edge_fraction(self.drop_targets.edge_fraction),
                ..self.drop_targets
            },
            ..self
        }
    }

    /// Sets the drop-edge fraction.
    #[must_use]
    pub const fn with_drop_edge_fraction(mut self, fraction: f32) -> Self {
        self.drop_targets.edge_fraction = fraction;
        self
    }

    /// Sets whether tab merge targets are allowed.
    #[must_use]
    pub const fn with_tab_merge(mut self, allowed: bool) -> Self {
        self.drop_targets.allow_tab_merge = allowed;
        self
    }

    /// Sets whether split insertion targets are allowed.
    #[must_use]
    pub const fn with_split_insertion(mut self, allowed: bool) -> Self {
        self.drop_targets.allow_split_insertion = allowed;
        self
    }

    /// Sets whether splitter drag resize is allowed.
    #[must_use]
    pub const fn with_splitter_resize(mut self, allowed: bool) -> Self {
        self.splitters.allow_resize = allowed;
        self
    }

    /// Sets whether splitter join context actions are allowed.
    #[must_use]
    pub const fn with_splitter_join(mut self, allowed: bool) -> Self {
        self.splitters.allow_join = allowed;
        self
    }

    /// Sets whether splitter swap context actions are allowed.
    #[must_use]
    pub const fn with_splitter_swap(mut self, allowed: bool) -> Self {
        self.splitters.allow_swap = allowed;
        self
    }

    const fn allows_splitter_action(self, kind: DockSplitterContextActionKind) -> bool {
        match kind {
            DockSplitterContextActionKind::Join => self.splitters.allow_join,
            DockSplitterContextActionKind::Swap => self.splitters.allow_swap,
        }
    }
}

/// Data-only style for dock chrome hit metadata.
///
/// The default style preserves the built-in splitter hit rectangle thickness.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DockChromeStyle {
    /// Logical thickness used to solve splitter hit rectangles.
    pub splitter_hit_thickness: f32,
}

impl Default for DockChromeStyle {
    fn default() -> Self {
        Self {
            splitter_hit_thickness: DEFAULT_SPLITTER_THICKNESS,
        }
    }
}

impl DockChromeStyle {
    /// Returns a copy with deterministic, valid numeric values.
    #[must_use]
    pub fn sanitized(self) -> Self {
        Self {
            splitter_hit_thickness: splitter_thickness(self.splitter_hit_thickness),
        }
    }

    /// Sets the splitter hit thickness in logical units.
    #[must_use]
    pub const fn with_splitter_hit_thickness(mut self, thickness: f32) -> Self {
        self.splitter_hit_thickness = thickness;
        self
    }
}

/// Stable panel identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PanelId(u64);

impl PanelId {
    /// Creates a panel ID from raw bits.
    #[must_use]
    pub const fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    /// Returns raw ID bits.
    #[must_use]
    pub const fn raw(self) -> u64 {
        self.0
    }

    /// Creates a panel ID from a panel instance ID.
    #[must_use]
    pub const fn from_instance_id(id: PanelInstanceId) -> Self {
        Self(id.raw())
    }

    /// Returns this legacy panel ID as the panel instance vocabulary.
    #[must_use]
    pub const fn instance_id(self) -> PanelInstanceId {
        PanelInstanceId::from_raw(self.0)
    }
}

impl From<PanelInstanceId> for PanelId {
    fn from(value: PanelInstanceId) -> Self {
        Self::from_instance_id(value)
    }
}

impl From<PanelId> for PanelInstanceId {
    fn from(value: PanelId) -> Self {
        value.instance_id()
    }
}

/// Stable identity for a developer-declared panel kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PanelTypeId(u64);

impl PanelTypeId {
    /// Creates a panel type ID from raw bits.
    #[must_use]
    pub const fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    /// Returns raw ID bits.
    #[must_use]
    pub const fn raw(self) -> u64 {
        self.0
    }
}

/// Stable identity for one open instance of a panel type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PanelInstanceId(u64);

impl PanelInstanceId {
    /// Creates a panel instance ID from raw bits.
    #[must_use]
    pub const fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    /// Returns raw ID bits.
    #[must_use]
    pub const fn raw(self) -> u64 {
        self.0
    }
}

/// Stable frame identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FrameId(u64);

impl FrameId {
    /// Creates a frame ID from raw bits.
    #[must_use]
    pub const fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    /// Returns raw ID bits.
    #[must_use]
    pub const fn raw(self) -> u64 {
        self.0
    }
}

/// Address of a split node inside a dock tree.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct DockSplitPath(Vec<DockPathElement>);

impl DockSplitPath {
    /// Returns the root split path.
    #[must_use]
    pub const fn root() -> Self {
        Self(Vec::new())
    }

    /// Creates a path from child traversal elements.
    #[must_use]
    pub fn new(elements: impl IntoIterator<Item = DockPathElement>) -> Self {
        Self(elements.into_iter().collect())
    }

    /// Returns a child path under this split.
    #[must_use]
    pub fn child(&self, element: DockPathElement) -> Self {
        let mut path = self.clone();
        path.0.push(element);
        path
    }

    /// Returns the traversal elements.
    #[must_use]
    pub fn elements(&self) -> &[DockPathElement] {
        &self.0
    }
}

/// Traversal element for a [`DockSplitPath`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DockPathElement {
    /// Descend into the first split child.
    First,
    /// Descend into the second split child.
    Second,
}

/// Dock placement used when splitting a frame with a dragged tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DockPlacement {
    /// Insert a new frame to the left of the target frame.
    Left,
    /// Insert a new frame to the right of the target frame.
    Right,
    /// Insert a new frame above the target frame.
    Top,
    /// Insert a new frame below the target frame.
    Bottom,
}

impl DockPlacement {
    const fn axis(self) -> Axis {
        match self {
            Self::Left | Self::Right => Axis::Horizontal,
            Self::Top | Self::Bottom => Axis::Vertical,
        }
    }

    const fn insert_before_target(self) -> bool {
        matches!(self, Self::Left | Self::Top)
    }
}

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
    /// Optional symbolic icon for panel picker and tab chrome.
    pub icon: Option<IconId>,
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

    /// Sets the optional symbolic icon.
    #[must_use]
    pub const fn with_icon(mut self, icon: IconId) -> Self {
        self.icon = Some(icon);
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
    /// Optional symbolic icon from the descriptor.
    pub icon: Option<IconId>,
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

/// Application-owned metadata carried by panel policy requests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PanelPolicyMetadata {
    /// Developer-declared panel type identity.
    pub panel_type: PanelTypeId,
    /// Descriptor title used by default app surfaces.
    pub title: String,
    /// Optional application-owned default open action from the descriptor.
    pub default_open_action: Option<ActionId>,
}

impl PanelPolicyMetadata {
    /// Creates request metadata from a panel type descriptor.
    #[must_use]
    pub fn from_descriptor(descriptor: &PanelTypeDescriptor) -> Self {
        Self {
            panel_type: descriptor.id,
            title: descriptor.title.clone(),
            default_open_action: descriptor.default_open_action.clone(),
        }
    }
}

/// Location of an open panel instance in the current dock tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PanelInstanceLocation {
    /// Stable open panel instance identity.
    pub panel_instance: PanelInstanceId,
    /// Compatibility panel identity used by current dock callers.
    pub panel: PanelId,
    /// Frame currently containing the panel.
    pub frame: FrameId,
}

impl PanelInstanceLocation {
    /// Creates a location from panel instance vocabulary.
    #[must_use]
    pub const fn new(panel_instance: PanelInstanceId, frame: FrameId) -> Self {
        Self {
            panel_instance,
            panel: PanelId::from_instance_id(panel_instance),
            frame,
        }
    }
}

/// Resolved tab and panel affordances for a specific panel instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PanelAffordances {
    /// Panel type the affordances were resolved from.
    pub panel_type: PanelTypeId,
    /// Open panel instance identity.
    pub panel_instance: PanelInstanceId,
    /// Whether close chrome should be visible.
    pub close_visible: bool,
    /// Whether duplicate/open-another should be available.
    pub duplicate_available: bool,
    /// Whether future floating-surface affordances should be available.
    pub float_available: bool,
}

/// Request for the application to open a new panel instance.
#[derive(Debug, Clone, PartialEq)]
pub struct PanelOpenRequest {
    /// Application-owned metadata for the request.
    pub metadata: PanelPolicyMetadata,
    /// Workspace context requested by the caller.
    pub context: PanelWorkspaceContext,
    /// Preferred dock placement hint, when the descriptor provides one.
    pub dock_hint: Option<PanelDockHint>,
    /// Preferred logical size from the descriptor.
    pub default_size: Size,
}

/// Request for the application to focus an already-open singleton instance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PanelFocusRequest {
    /// Application-owned metadata for the request.
    pub metadata: PanelPolicyMetadata,
    /// Existing panel instance to focus.
    pub target: PanelInstanceLocation,
}

/// Decision produced when the user asks to open a panel type.
#[derive(Debug, Clone, PartialEq)]
pub enum PanelOpenDecision {
    /// Focus an existing singleton instance instead of opening another one.
    FocusExisting(PanelFocusRequest),
    /// Ask the application to open a new panel instance.
    OpenNew(PanelOpenRequest),
}

/// Request for the application to close a panel instance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PanelCloseRequest {
    /// Application-owned metadata for the request.
    pub metadata: PanelPolicyMetadata,
    /// Panel instance the application may close.
    pub target: PanelInstanceLocation,
}

/// Request for the application to duplicate a panel instance.
#[derive(Debug, Clone, PartialEq)]
pub struct PanelDuplicateRequest {
    /// Application-owned metadata for the request.
    pub metadata: PanelPolicyMetadata,
    /// Source panel instance to duplicate.
    pub source: PanelInstanceLocation,
    /// Workspace context requested by the caller.
    pub context: PanelWorkspaceContext,
    /// Preferred dock placement hint, when the descriptor provides one.
    pub dock_hint: Option<PanelDockHint>,
    /// Preferred logical size from the descriptor.
    pub default_size: Size,
}

/// Request for a future floating surface without creating a native window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PanelFloatRequest {
    /// Application-owned metadata for the request.
    pub metadata: PanelPolicyMetadata,
    /// Panel instance that may be floated by the application/platform layer.
    pub source: PanelInstanceLocation,
}

/// Inputs used to resolve policy metadata for one panel instance.
///
/// This context borrows the current application-owned registry and panel
/// instance records, plus the dock/frame state needed to resolve frame-owned
/// affordances. It is intentionally read-only.
#[derive(Debug, Clone, Copy)]
pub struct PanelPolicyContext<'a> {
    /// Registry containing developer-declared panel descriptors.
    pub registry: &'a PanelRegistry,
    /// Current application-owned open panel instance records.
    pub panel_instances: &'a [PanelInstanceSnapshot],
    /// Current dock tree used for location and singleton focus lookup.
    pub dock: &'a Dock,
    /// Open panel instance to resolve.
    pub panel_instance: PanelInstanceId,
    /// Frame expected to currently contain the panel instance.
    pub frame: FrameId,
    /// Workspace context requested by the caller.
    pub workspace_context: PanelWorkspaceContext,
}

impl<'a> PanelPolicyContext<'a> {
    /// Creates a read-only panel policy context.
    #[must_use]
    pub const fn new(
        registry: &'a PanelRegistry,
        panel_instances: &'a [PanelInstanceSnapshot],
        dock: &'a Dock,
        panel_instance: PanelInstanceId,
        frame: FrameId,
        workspace_context: PanelWorkspaceContext,
    ) -> Self {
        Self {
            registry,
            panel_instances,
            dock,
            panel_instance,
            frame,
            workspace_context,
        }
    }
}

/// Deterministic reason a panel policy context could not produce requests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelPolicyUnavailableReason {
    /// No application-owned instance record exists for the requested panel.
    MissingPanelInstance,
    /// The instance record references a panel type missing from the registry.
    MissingDescriptor,
    /// The instance record exists, but the panel is not present in the dock.
    MissingPanelLocation,
    /// The requested frame does not currently own the panel instance.
    MissingFrameMembership,
    /// The requested workspace context is not allowed by the descriptor.
    DisallowedContext,
}

/// Pure result for one resolved panel policy context.
#[derive(Debug, Clone, PartialEq)]
pub struct PanelPolicyResolution {
    /// Requested panel instance.
    pub panel_instance: PanelInstanceId,
    /// Resolved panel type from the instance record, when available.
    pub panel_type: Option<PanelTypeId>,
    /// Frame requested by the caller.
    pub frame: FrameId,
    /// Dock location found for the panel instance, when available.
    pub location: Option<PanelInstanceLocation>,
    /// Workspace context requested by the caller.
    pub workspace_context: PanelWorkspaceContext,
    /// Deterministic unavailable reason. `None` means all requests were
    /// resolved against a valid descriptor, instance, frame, and context.
    pub unavailable: Option<PanelPolicyUnavailableReason>,
    /// Descriptor/frame-derived affordances, when enough context exists.
    pub affordances: Option<PanelAffordances>,
    /// Optional open or focus metadata for the requested context.
    pub open_decision: Option<PanelOpenDecision>,
    /// Optional close metadata for the current panel instance.
    pub close_request: Option<PanelCloseRequest>,
    /// Optional duplicate metadata for the current panel instance.
    pub duplicate_request: Option<PanelDuplicateRequest>,
    /// Optional future floating-surface metadata for the current panel instance.
    pub float_request: Option<PanelFloatRequest>,
}

impl PanelPolicyResolution {
    /// Returns true when the resolver produced requests for an available
    /// descriptor, panel instance, frame membership, and workspace context.
    #[must_use]
    pub const fn is_available(&self) -> bool {
        self.unavailable.is_none()
    }
}

/// Request for an application-owned frame edge split affordance.
///
/// This is separate from tab drag/drop: it describes split intent only and does
/// not mutate the dock tree, create panels, or execute application commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameSplitAffordanceRequest {
    /// Frame that owns the active panel or command source.
    pub source_frame: FrameId,
    /// Frame whose edge/corner affordance was targeted.
    pub target_frame: FrameId,
    /// Placement of the new frame relative to the target frame.
    pub placement: DockPlacement,
    /// Active source panel identity when the source frame has one.
    pub active_panel: Option<PanelInstanceLocation>,
    /// Application-supplied identity for the frame to be created.
    pub new_frame: FrameId,
}

/// Topology-validated request to join one frame into an adjacent neighbor.
///
/// The request is resolved from frame neighbor topology and is applied by
/// moving the source frame's tabs into the target frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DockJoinRequest {
    source_frame: FrameId,
    direction: DockNeighborDirection,
    target_frame: FrameId,
}

impl DockJoinRequest {
    /// Returns the frame whose tabs will move into the target frame.
    #[must_use]
    pub const fn source_frame(self) -> FrameId {
        self.source_frame
    }

    /// Returns the requested neighbor direction from the source frame.
    #[must_use]
    pub const fn direction(self) -> DockNeighborDirection {
        self.direction
    }

    /// Returns the resolved neighboring frame that will survive the join.
    #[must_use]
    pub const fn target_frame(self) -> FrameId {
        self.target_frame
    }
}

/// Topology-validated request to swap one frame with an adjacent neighbor.
///
/// The request is resolved from frame neighbor topology and is applied by
/// swapping whole frame leaves in the dock tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DockSwapRequest {
    source_frame: FrameId,
    direction: DockNeighborDirection,
    target_frame: FrameId,
}

impl DockSwapRequest {
    /// Returns the source frame that will trade dock-tree positions.
    #[must_use]
    pub const fn source_frame(self) -> FrameId {
        self.source_frame
    }

    /// Returns the requested neighbor direction from the source frame.
    #[must_use]
    pub const fn direction(self) -> DockNeighborDirection {
        self.direction
    }

    /// Returns the resolved neighboring frame that will trade positions.
    #[must_use]
    pub const fn target_frame(self) -> FrameId {
        self.target_frame
    }
}

/// Operation represented by splitter context action metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DockSplitterContextActionKind {
    /// Join one side of the splitter into the opposite side.
    Join,
    /// Swap the two resolved frame leaves adjacent to the splitter.
    Swap,
}

/// Logical side of a splitter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DockSplitterSide {
    /// First split child: left for horizontal splits, top for vertical splits.
    First,
    /// Second split child: right for horizontal splits, bottom for vertical splits.
    Second,
}

/// Target context shared by splitter context actions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DockSplitterActionContext {
    /// Split path addressed by the context menu source.
    pub path: DockSplitPath,
    /// Split axis addressed by the context menu source.
    pub axis: Axis,
    /// Resolved frame leaf on the first side of the splitter.
    pub first_frame: Option<FrameId>,
    /// Resolved frame leaf on the second side of the splitter.
    pub second_frame: Option<FrameId>,
}

/// Pure app-dispatch metadata for a splitter context action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DockSplitterContextAction {
    /// Operation kind the application may present.
    pub kind: DockSplitterContextActionKind,
    /// Splitter target context.
    pub context: DockSplitterActionContext,
    /// Side that supplies the source frame for the operation.
    pub source_side: DockSplitterSide,
    /// Side that supplies the target frame for the operation.
    pub target_side: DockSplitterSide,
    /// Resolved source frame when available.
    pub source_frame: Option<FrameId>,
    /// Resolved target frame when available.
    pub target_frame: Option<FrameId>,
    /// Direction from the source side toward the target side.
    pub direction: DockNeighborDirection,
    /// Whether the action can be safely dispatched against the current layout.
    pub enabled: bool,
}

impl DockSplitterContextAction {
    /// Returns a validated join request when this enabled action is a join.
    #[must_use]
    pub fn join_request(&self) -> Option<DockJoinRequest> {
        if !self.enabled || !matches!(self.kind, DockSplitterContextActionKind::Join) {
            return None;
        }

        let source_frame = self.source_frame?;
        let target_frame = self.target_frame?;

        Some(DockJoinRequest {
            source_frame,
            direction: self.direction,
            target_frame,
        })
    }

    /// Returns a validated swap request when this enabled action is a swap.
    #[must_use]
    pub fn swap_request(&self) -> Option<DockSwapRequest> {
        if !self.enabled || !matches!(self.kind, DockSplitterContextActionKind::Swap) {
            return None;
        }

        let source_frame = self.source_frame?;
        let target_frame = self.target_frame?;

        Some(DockSwapRequest {
            source_frame,
            direction: self.direction,
            target_frame,
        })
    }
}

/// Finds an open panel instance in deterministic dock tree order.
#[must_use]
pub fn locate_panel_instance(
    dock: &Dock,
    panel_instance: PanelInstanceId,
) -> Option<PanelInstanceLocation> {
    let panel = PanelId::from_instance_id(panel_instance);
    dock.frames()
        .into_iter()
        .find(|frame| frame.panels.iter().any(|item| item.id == panel))
        .map(|frame| PanelInstanceLocation {
            panel_instance,
            panel,
            frame: frame.id,
        })
}

/// Resolves tab and panel affordances without mutating dock or app state.
#[must_use]
pub fn resolve_panel_affordances(
    descriptor: &PanelTypeDescriptor,
    panel_instance: PanelInstanceId,
    frame: &Frame,
) -> PanelAffordances {
    let panel = PanelId::from_instance_id(panel_instance);
    let panel_in_frame = frame.panels.iter().any(|item| item.id == panel);
    PanelAffordances {
        panel_type: descriptor.id,
        panel_instance,
        close_visible: descriptor.close_policy == PanelClosePolicy::Closable
            && frame.panel_dismissible(panel),
        duplicate_available: panel_in_frame
            && descriptor.instance_policy == PanelInstancePolicy::MultiInstance
            && descriptor.duplicate_policy == PanelDuplicatePolicy::Allowed,
        float_available: panel_in_frame && descriptor.float_policy == PanelFloatPolicy::Allowed,
    }
}

/// Resolves whether opening a panel type should focus an existing singleton or
/// ask the application to create a new instance.
#[must_use]
pub fn resolve_panel_open_decision(
    descriptor: &PanelTypeDescriptor,
    panel_instances: &[PanelInstanceSnapshot],
    dock: &Dock,
    context: PanelWorkspaceContext,
) -> Option<PanelOpenDecision> {
    if !descriptor.allowed_contexts.contains(&context) {
        return None;
    }

    if descriptor.instance_policy == PanelInstancePolicy::Singleton
        && let Some(target) = locate_first_panel_type_instance(dock, panel_instances, descriptor.id)
    {
        return Some(PanelOpenDecision::FocusExisting(PanelFocusRequest {
            metadata: PanelPolicyMetadata::from_descriptor(descriptor),
            target,
        }));
    }

    Some(PanelOpenDecision::OpenNew(PanelOpenRequest {
        metadata: PanelPolicyMetadata::from_descriptor(descriptor),
        context,
        dock_hint: descriptor.dock_hints.first().copied(),
        default_size: descriptor.default_size,
    }))
}

/// Produces an app-owned close request when descriptor and frame policy allow it.
#[must_use]
pub fn resolve_panel_close_request(
    descriptor: &PanelTypeDescriptor,
    panel_instance: PanelInstanceId,
    frame: &Frame,
) -> Option<PanelCloseRequest> {
    resolve_panel_affordances(descriptor, panel_instance, frame)
        .close_visible
        .then(|| PanelCloseRequest {
            metadata: PanelPolicyMetadata::from_descriptor(descriptor),
            target: PanelInstanceLocation::new(panel_instance, frame.id),
        })
}

/// Produces an app-owned duplicate request without creating a panel.
#[must_use]
pub fn resolve_panel_duplicate_request(
    descriptor: &PanelTypeDescriptor,
    panel_instance: PanelInstanceId,
    frame: &Frame,
    context: PanelWorkspaceContext,
) -> Option<PanelDuplicateRequest> {
    if !resolve_panel_affordances(descriptor, panel_instance, frame).duplicate_available
        || !descriptor.allowed_contexts.contains(&context)
    {
        return None;
    }

    Some(PanelDuplicateRequest {
        metadata: PanelPolicyMetadata::from_descriptor(descriptor),
        source: PanelInstanceLocation::new(panel_instance, frame.id),
        context,
        dock_hint: descriptor.dock_hints.first().copied(),
        default_size: descriptor.default_size,
    })
}

/// Produces an app-owned future float request without creating a native window.
#[must_use]
pub fn resolve_panel_float_request(
    descriptor: &PanelTypeDescriptor,
    panel_instance: PanelInstanceId,
    frame: &Frame,
) -> Option<PanelFloatRequest> {
    resolve_panel_affordances(descriptor, panel_instance, frame)
        .float_available
        .then(|| PanelFloatRequest {
            metadata: PanelPolicyMetadata::from_descriptor(descriptor),
            source: PanelInstanceLocation::new(panel_instance, frame.id),
        })
}

/// Resolves panel affordances and app-owned request metadata from registry,
/// instance, dock, frame, and workspace context.
///
/// The resolver is metadata-only. It composes the focused panel policy helpers
/// and does not mutate dock state, create panels, close panels, duplicate
/// panels, open native windows, or execute application commands.
#[must_use]
pub fn resolve_panel_policy_context(context: PanelPolicyContext<'_>) -> PanelPolicyResolution {
    let Some(instance) = context
        .panel_instances
        .iter()
        .find(|instance| instance.id == context.panel_instance)
    else {
        return unavailable_panel_policy_resolution(
            &context,
            None,
            None,
            None,
            PanelPolicyUnavailableReason::MissingPanelInstance,
        );
    };

    let panel_type = Some(instance.panel_type);
    let location = locate_panel_instance(context.dock, context.panel_instance);

    let Some(descriptor) = context.registry.descriptor(instance.panel_type) else {
        return unavailable_panel_policy_resolution(
            &context,
            panel_type,
            location,
            None,
            PanelPolicyUnavailableReason::MissingDescriptor,
        );
    };

    let Some(location) = location else {
        return unavailable_panel_policy_resolution(
            &context,
            panel_type,
            None,
            None,
            PanelPolicyUnavailableReason::MissingPanelLocation,
        );
    };

    let panel = PanelId::from_instance_id(context.panel_instance);
    let Some(frame) = context.dock.frame(context.frame) else {
        return unavailable_panel_policy_resolution(
            &context,
            panel_type,
            Some(location),
            None,
            PanelPolicyUnavailableReason::MissingFrameMembership,
        );
    };

    if location.frame != context.frame || !frame.panels.iter().any(|item| item.id == panel) {
        return unavailable_panel_policy_resolution(
            &context,
            panel_type,
            Some(location),
            None,
            PanelPolicyUnavailableReason::MissingFrameMembership,
        );
    }

    let affordances = resolve_panel_affordances(descriptor, context.panel_instance, frame);

    if !descriptor
        .allowed_contexts
        .contains(&context.workspace_context)
    {
        return unavailable_panel_policy_resolution(
            &context,
            panel_type,
            Some(location),
            Some(affordances),
            PanelPolicyUnavailableReason::DisallowedContext,
        );
    }

    PanelPolicyResolution {
        panel_instance: context.panel_instance,
        panel_type,
        frame: context.frame,
        location: Some(location),
        workspace_context: context.workspace_context,
        unavailable: None,
        affordances: Some(affordances),
        open_decision: resolve_panel_open_decision(
            descriptor,
            context.panel_instances,
            context.dock,
            context.workspace_context,
        ),
        close_request: resolve_panel_close_request(descriptor, context.panel_instance, frame),
        duplicate_request: resolve_panel_duplicate_request(
            descriptor,
            context.panel_instance,
            frame,
            context.workspace_context,
        ),
        float_request: resolve_panel_float_request(descriptor, context.panel_instance, frame),
    }
}

fn unavailable_panel_policy_resolution(
    context: &PanelPolicyContext<'_>,
    panel_type: Option<PanelTypeId>,
    location: Option<PanelInstanceLocation>,
    affordances: Option<PanelAffordances>,
    reason: PanelPolicyUnavailableReason,
) -> PanelPolicyResolution {
    PanelPolicyResolution {
        panel_instance: context.panel_instance,
        panel_type,
        frame: context.frame,
        location,
        workspace_context: context.workspace_context,
        unavailable: Some(reason),
        affordances,
        open_decision: None,
        close_request: None,
        duplicate_request: None,
        float_request: None,
    }
}

/// Resolves an app-owned frame edge split request from frame layouts.
///
/// Center/tab-merge zones and invalid geometry return `None`. The returned
/// request is metadata only; callers decide what panel content to create or
/// move before applying any future dock mutation.
#[must_use]
pub fn resolve_frame_split_affordance_request(
    dock: &Dock,
    frames: &[FrameLayout],
    source_frame: FrameId,
    point: Point,
    new_frame: FrameId,
) -> Option<FrameSplitAffordanceRequest> {
    resolve_frame_split_affordance_request_with_policy(
        dock,
        frames,
        source_frame,
        point,
        new_frame,
        DockInteractionPolicy::default(),
    )
}

/// Resolves a pure frame split affordance request using dock interaction policy.
#[must_use]
pub fn resolve_frame_split_affordance_request_with_policy(
    dock: &Dock,
    frames: &[FrameLayout],
    source_frame: FrameId,
    point: Point,
    new_frame: FrameId,
    policy: DockInteractionPolicy,
) -> Option<FrameSplitAffordanceRequest> {
    let source = dock.frame(source_frame)?;
    let active_panel = active_panel_location(source);
    let (target_frame, placement) =
        resolve_frame_split_affordance_with_policy(frames, point, policy)?;
    dock.frame(target_frame)?;

    Some(FrameSplitAffordanceRequest {
        source_frame,
        target_frame,
        placement,
        active_panel,
        new_frame,
    })
}

/// Resolves a neighbor join request from solved frame neighbor topology.
///
/// The source frame must have a distinct resolved target in the requested
/// direction, and that target must also appear in the supplied topology.
#[must_use]
pub fn resolve_dock_join_request(
    neighbors: &[FrameNeighbors],
    source_frame: FrameId,
    direction: DockNeighborDirection,
) -> Option<DockJoinRequest> {
    let source_neighbors = neighbors
        .iter()
        .find(|neighbors| neighbors.frame == source_frame)?;
    let target_frame = source_neighbors.neighbor(direction)?;
    if target_frame == source_frame
        || !neighbors
            .iter()
            .any(|neighbors| neighbors.frame == target_frame)
    {
        return None;
    }

    Some(DockJoinRequest {
        source_frame,
        direction,
        target_frame,
    })
}

/// Resolves a neighbor swap request from solved frame neighbor topology.
///
/// The source frame must have a distinct resolved target in the requested
/// direction, and that target must also appear in the supplied topology.
#[must_use]
pub fn resolve_dock_swap_request(
    neighbors: &[FrameNeighbors],
    source_frame: FrameId,
    direction: DockNeighborDirection,
) -> Option<DockSwapRequest> {
    let source_neighbors = neighbors
        .iter()
        .find(|neighbors| neighbors.frame == source_frame)?;
    let target_frame = source_neighbors.neighbor(direction)?;
    if target_frame == source_frame
        || !neighbors
            .iter()
            .any(|neighbors| neighbors.frame == target_frame)
    {
        return None;
    }

    Some(DockSwapRequest {
        source_frame,
        direction,
        target_frame,
    })
}

/// Resolves pure context action metadata for a dock splitter.
///
/// The returned actions are stable and do not mutate dock state, enqueue
/// application actions, or execute commands. Invalid paths, stale splitters,
/// invalid geometry, or missing adjacent frames produce disabled actions with
/// the unresolved frame context preserved as `None`.
#[must_use]
pub fn resolve_dock_splitter_context_actions(
    dock: &Dock,
    frames: &[FrameLayout],
    splitter: &DockSplitter,
) -> Vec<DockSplitterContextAction> {
    resolve_dock_splitter_context_actions_with_policy(
        dock,
        frames,
        splitter,
        DockInteractionPolicy::default(),
    )
}

/// Resolves pure context action metadata using dock interaction policy.
///
/// Disabled join or swap policy leaves action metadata present but disabled.
#[must_use]
pub fn resolve_dock_splitter_context_actions_with_policy(
    dock: &Dock,
    frames: &[FrameLayout],
    splitter: &DockSplitter,
    policy: DockInteractionPolicy,
) -> Vec<DockSplitterContextAction> {
    let context = resolve_dock_splitter_action_context(dock, frames, splitter);
    let (first_to_second, second_to_first) = splitter_context_directions(splitter.axis);
    let policy = policy.sanitized();

    vec![
        dock_splitter_context_action(
            dock,
            frames,
            policy,
            DockSplitterActionSpec {
                kind: DockSplitterContextActionKind::Join,
                source_side: DockSplitterSide::First,
                target_side: DockSplitterSide::Second,
                direction: first_to_second,
            },
            context.clone(),
        ),
        dock_splitter_context_action(
            dock,
            frames,
            policy,
            DockSplitterActionSpec {
                kind: DockSplitterContextActionKind::Join,
                source_side: DockSplitterSide::Second,
                target_side: DockSplitterSide::First,
                direction: second_to_first,
            },
            context.clone(),
        ),
        dock_splitter_context_action(
            dock,
            frames,
            policy,
            DockSplitterActionSpec {
                kind: DockSplitterContextActionKind::Swap,
                source_side: DockSplitterSide::First,
                target_side: DockSplitterSide::Second,
                direction: first_to_second,
            },
            context.clone(),
        ),
        dock_splitter_context_action(
            dock,
            frames,
            policy,
            DockSplitterActionSpec {
                kind: DockSplitterContextActionKind::Swap,
                source_side: DockSplitterSide::Second,
                target_side: DockSplitterSide::First,
                direction: second_to_first,
            },
            context,
        ),
    ]
}

fn resolve_dock_splitter_action_context(
    dock: &Dock,
    frames: &[FrameLayout],
    splitter: &DockSplitter,
) -> DockSplitterActionContext {
    let Some((axis, first, second)) = split_children_at_path(&dock.root, splitter.path.elements())
    else {
        return DockSplitterActionContext {
            path: splitter.path.clone(),
            axis: splitter.axis,
            first_frame: None,
            second_frame: None,
        };
    };

    if axis != splitter.axis {
        return DockSplitterActionContext {
            path: splitter.path.clone(),
            axis: splitter.axis,
            first_frame: None,
            second_frame: None,
        };
    }

    let first_frames = collect_frame_ids(first);
    let second_frames = collect_frame_ids(second);

    DockSplitterActionContext {
        path: splitter.path.clone(),
        axis: splitter.axis,
        first_frame: splitter_adjacent_frame(
            frames,
            &first_frames,
            splitter,
            DockSplitterSide::First,
        ),
        second_frame: splitter_adjacent_frame(
            frames,
            &second_frames,
            splitter,
            DockSplitterSide::Second,
        ),
    }
}

#[derive(Debug, Clone, Copy)]
struct DockSplitterActionSpec {
    kind: DockSplitterContextActionKind,
    source_side: DockSplitterSide,
    target_side: DockSplitterSide,
    direction: DockNeighborDirection,
}

fn dock_splitter_context_action(
    dock: &Dock,
    frames: &[FrameLayout],
    policy: DockInteractionPolicy,
    spec: DockSplitterActionSpec,
    context: DockSplitterActionContext,
) -> DockSplitterContextAction {
    let source_frame = splitter_context_frame(&context, spec.source_side);
    let target_frame = splitter_context_frame(&context, spec.target_side);
    let enabled = policy.allows_splitter_action(spec.kind)
        && source_frame
            .zip(target_frame)
            .is_some_and(|(source_frame, target_frame)| {
                if source_frame == target_frame
                    || !dock.frame(source_frame).is_some_and(frame_is_valid)
                    || !dock.frame(target_frame).is_some_and(frame_is_valid)
                {
                    return false;
                }

                match spec.kind {
                    DockSplitterContextActionKind::Join => join_request_matches_layout(
                        frames,
                        DockJoinRequest {
                            source_frame,
                            direction: spec.direction,
                            target_frame,
                        },
                    ),
                    DockSplitterContextActionKind::Swap => swap_request_matches_layout(
                        frames,
                        DockSwapRequest {
                            source_frame,
                            direction: spec.direction,
                            target_frame,
                        },
                    ),
                }
            });

    DockSplitterContextAction {
        kind: spec.kind,
        context,
        source_side: spec.source_side,
        target_side: spec.target_side,
        source_frame,
        target_frame,
        direction: spec.direction,
        enabled,
    }
}

fn splitter_context_frame(
    context: &DockSplitterActionContext,
    side: DockSplitterSide,
) -> Option<FrameId> {
    match side {
        DockSplitterSide::First => context.first_frame,
        DockSplitterSide::Second => context.second_frame,
    }
}

fn splitter_context_directions(axis: Axis) -> (DockNeighborDirection, DockNeighborDirection) {
    match axis {
        Axis::Horizontal => (DockNeighborDirection::Right, DockNeighborDirection::Left),
        Axis::Vertical => (DockNeighborDirection::Down, DockNeighborDirection::Up),
    }
}

fn join_request_matches_layout(frames: &[FrameLayout], request: DockJoinRequest) -> bool {
    request.source_frame != request.target_frame
        && frame_neighbor(frames, request.source_frame, request.direction)
            == Some(request.target_frame)
}

fn swap_request_matches_layout(frames: &[FrameLayout], request: DockSwapRequest) -> bool {
    request.source_frame != request.target_frame
        && frame_neighbor(frames, request.source_frame, request.direction)
            == Some(request.target_frame)
}

fn locate_first_panel_type_instance(
    dock: &Dock,
    panel_instances: &[PanelInstanceSnapshot],
    panel_type: PanelTypeId,
) -> Option<PanelInstanceLocation> {
    dock.frames().into_iter().find_map(|frame| {
        frame.panels.iter().find_map(|panel| {
            let panel_instance = panel.instance_id();
            panel_instances
                .iter()
                .any(|instance| instance.id == panel_instance && instance.panel_type == panel_type)
                .then_some(PanelInstanceLocation {
                    panel_instance,
                    panel: panel.id,
                    frame: frame.id,
                })
        })
    })
}

fn active_panel_location(frame: &Frame) -> Option<PanelInstanceLocation> {
    frame.active_panel().map(|panel| PanelInstanceLocation {
        panel_instance: panel.instance_id(),
        panel: panel.id,
        frame: frame.id,
    })
}

/// Resolved dock drop zone inside a frame rectangle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DockDropZone {
    /// Merge the dragged tab into the target frame.
    Center,
    /// Split to the target frame's left.
    Left,
    /// Split to the target frame's right.
    Right,
    /// Split above the target frame.
    Top,
    /// Split below the target frame.
    Bottom,
}

impl DockDropZone {
    const fn placement(self) -> Option<DockPlacement> {
        match self {
            Self::Center => None,
            Self::Left => Some(DockPlacement::Left),
            Self::Right => Some(DockPlacement::Right),
            Self::Top => Some(DockPlacement::Top),
            Self::Bottom => Some(DockPlacement::Bottom),
        }
    }
}

/// Tab drag state emitted by frame chrome.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DockTabDrag {
    /// Source frame that owns the dragged tab.
    pub source_frame: FrameId,
    /// Dragged panel tab.
    pub panel: PanelId,
}

/// Explicit target for dropping a dragged frame tab.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DockDropTarget {
    /// Merge the panel into an existing frame tab group.
    Tab {
        /// Target frame.
        frame: FrameId,
    },
    /// Insert the panel as a new frame split adjacent to an existing frame.
    Split {
        /// Existing frame to split around.
        frame: FrameId,
        /// Placement of the new frame relative to the target frame.
        placement: DockPlacement,
        /// Frame ID for the newly inserted frame.
        new_frame: FrameId,
        /// Initial split ratio.
        ratio: f32,
        /// Minimum first child size.
        min_first: f32,
        /// Minimum second child size.
        min_second: f32,
    },
}

impl DockDropTarget {
    /// Creates a tab merge target.
    #[must_use]
    pub const fn tab(frame: FrameId) -> Self {
        Self::Tab { frame }
    }

    /// Creates a split insertion target with editor-friendly defaults.
    #[must_use]
    pub const fn split(frame: FrameId, placement: DockPlacement, new_frame: FrameId) -> Self {
        Self::Split {
            frame,
            placement,
            new_frame,
            ratio: DEFAULT_SPLIT_RATIO,
            min_first: DEFAULT_SPLIT_MINIMUM,
            min_second: DEFAULT_SPLIT_MINIMUM,
        }
    }
}

/// Resolved splitter hit target.
#[derive(Debug, Clone, PartialEq)]
pub struct DockSplitter {
    /// Split path addressed by this splitter.
    pub path: DockSplitPath,
    /// Split axis.
    pub axis: Axis,
    /// Splitter interaction rectangle.
    pub rect: Rect,
    /// Current split ratio.
    pub ratio: f32,
    /// Minimum first child size.
    pub min_first: f32,
    /// Minimum second child size.
    pub min_second: f32,
}

/// Passive panel metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Panel {
    /// Panel identity.
    pub id: PanelId,
    /// Display title used by frame tabs.
    pub title: String,
}

impl Panel {
    /// Creates a panel.
    #[must_use]
    pub fn new(id: PanelId, title: impl Into<String>) -> Self {
        Self {
            id,
            title: title.into(),
        }
    }

    /// Creates a panel from the panel instance vocabulary.
    #[must_use]
    pub fn from_instance_id(id: PanelInstanceId, title: impl Into<String>) -> Self {
        Self::new(PanelId::from_instance_id(id), title)
    }

    /// Returns this panel identity as a panel instance ID.
    #[must_use]
    pub const fn instance_id(&self) -> PanelInstanceId {
        self.id.instance_id()
    }
}

/// Docked frame containing tabbed panels.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    /// Frame identity.
    pub id: FrameId,
    /// Panels in tab order.
    pub panels: Vec<Panel>,
    /// Active panel index.
    pub active: usize,
    /// Panels whose frame tabs expose close/dismiss affordances.
    dismissible_panels: BTreeSet<PanelId>,
}

impl Frame {
    /// Creates a frame with panels.
    #[must_use]
    pub fn new(id: FrameId, panels: Vec<Panel>) -> Self {
        let dismissible_panels = panels.iter().map(|panel| panel.id).collect();
        Self {
            id,
            panels,
            active: 0,
            dismissible_panels,
        }
    }

    /// Returns the active panel.
    #[must_use]
    pub fn active_panel(&self) -> Option<&Panel> {
        self.panels.get(self.active)
    }

    /// Selects a panel by ID.
    pub fn select_panel(&mut self, panel: PanelId) -> bool {
        let Some(index) = self.panels.iter().position(|item| item.id == panel) else {
            return false;
        };
        self.active = index;
        true
    }

    /// Removes a panel by ID.
    pub fn remove_panel(&mut self, panel: PanelId) -> Option<Panel> {
        let (removed, _) = self.remove_panel_with_policy(panel)?;
        Some(removed)
    }

    fn remove_panel_with_policy(&mut self, panel: PanelId) -> Option<(Panel, bool)> {
        let index = self.panels.iter().position(|item| item.id == panel)?;
        let dismissible = self.dismissible_panels.remove(&panel);
        let removed = self.panels.remove(index);
        self.active = self.active.min(self.panels.len().saturating_sub(1));
        Some((removed, dismissible))
    }

    /// Adds a panel at the end.
    pub fn push_panel(&mut self, panel: Panel) {
        self.push_panel_with_policy(panel, true);
    }

    fn push_panel_with_policy(&mut self, panel: Panel, dismissible: bool) {
        let id = panel.id;
        self.panels.push(panel);
        self.set_panel_dismissible(id, dismissible);
    }

    /// Sets whether a frame tab can expose close/dismiss affordances.
    ///
    /// Returns `false` when the panel is not in this frame.
    pub fn set_panel_dismissible(&mut self, panel: PanelId, dismissible: bool) -> bool {
        if !self.panels.iter().any(|item| item.id == panel) {
            return false;
        }
        if dismissible {
            self.dismissible_panels.insert(panel);
        } else {
            self.dismissible_panels.remove(&panel);
        }
        true
    }

    /// Returns true when a frame tab can expose close/dismiss affordances.
    #[must_use]
    pub fn panel_dismissible(&self, panel: PanelId) -> bool {
        self.dismissible_panels.contains(&panel)
    }
}

/// Dock tree node.
#[derive(Debug, Clone, PartialEq)]
pub enum DockNode {
    /// Leaf frame.
    Frame(Frame),
    /// Split between two nodes.
    Split {
        /// Split axis.
        axis: Axis,
        /// First child ratio.
        ratio: f32,
        /// Minimum first child size.
        min_first: f32,
        /// Minimum second child size.
        min_second: f32,
        /// First child.
        first: Box<DockNode>,
        /// Second child.
        second: Box<DockNode>,
    },
}

/// Root dock.
#[derive(Debug, Clone, PartialEq)]
pub struct Dock {
    /// Root dock node.
    pub root: DockNode,
    /// Root-owned active frame identity.
    active_frame: Option<FrameId>,
}

impl Dock {
    /// Creates a dock.
    #[must_use]
    pub fn new(root: DockNode) -> Self {
        let active_frame = first_valid_frame_id(&root);
        Self { root, active_frame }
    }

    /// Returns the root-owned active frame identity.
    #[must_use]
    pub fn active_frame(&self) -> Option<FrameId> {
        self.active_frame
            .filter(|frame| self.frame(*frame).is_some_and(frame_is_valid))
    }

    /// Sets the root-owned active frame when the frame exists.
    pub fn set_active_frame(&mut self, frame: FrameId) -> bool {
        if !self.frame(frame).is_some_and(frame_is_valid) {
            return false;
        }
        self.active_frame = Some(frame);
        true
    }

    /// Visits all frames in deterministic tree order.
    #[must_use]
    pub fn frames(&self) -> Vec<&Frame> {
        let mut frames = Vec::new();
        collect_frames(&self.root, &mut frames);
        frames
    }

    /// Finds an immutable frame.
    #[must_use]
    pub fn frame(&self, frame: FrameId) -> Option<&Frame> {
        find_frame(&self.root, frame)
    }

    /// Finds a mutable frame.
    pub fn frame_mut(&mut self, frame: FrameId) -> Option<&mut Frame> {
        find_frame_mut(&mut self.root, frame)
    }

    /// Selects a panel in a frame.
    pub fn select_panel(&mut self, frame: FrameId, panel: PanelId) -> bool {
        let selected = self
            .frame_mut(frame)
            .is_some_and(|frame| frame.select_panel(panel));
        if selected {
            self.active_frame = Some(frame);
        }
        selected
    }

    /// Moves a panel between frames.
    pub fn move_panel(&mut self, from: FrameId, to: FrameId, panel: PanelId) -> bool {
        if from == to || self.frame(to).is_none() {
            return false;
        }
        let Some((panel, dismissible)) = self
            .frame_mut(from)
            .and_then(|frame| frame.remove_panel_with_policy(panel))
        else {
            return false;
        };
        let Some(target) = self.frame_mut(to) else {
            return false;
        };
        target.push_panel_with_policy(panel, dismissible);
        target.active = target.panels.len().saturating_sub(1);
        prune_empty_frames(&mut self.root);
        self.active_frame = Some(to);
        self.refresh_active_frame();
        true
    }

    /// Merges all source frame panels into target frame.
    pub fn merge_frames(&mut self, source: FrameId, target: FrameId) -> bool {
        if source == target || self.frame(source).is_none() || self.frame(target).is_none() {
            return false;
        }
        let Some((source_panels, dismissible_panels)) = self.frame_mut(source).map(|frame| {
            frame.active = 0;
            (
                core::mem::take(&mut frame.panels),
                core::mem::take(&mut frame.dismissible_panels),
            )
        }) else {
            return false;
        };
        let Some(target_frame) = self.frame_mut(target) else {
            return false;
        };
        target_frame.panels.extend(source_panels);
        target_frame.dismissible_panels.extend(dismissible_panels);
        target_frame.active = target_frame.panels.len().saturating_sub(1);
        prune_empty_frames(&mut self.root);
        self.active_frame = Some(target);
        self.refresh_active_frame();
        true
    }

    /// Applies a resolved neighbor join request against the current dock layout.
    pub fn apply_join_request(&mut self, bounds: Rect, request: DockJoinRequest) -> bool {
        self.apply_join_request_with_policy(bounds, request, DockInteractionPolicy::default())
    }

    /// Applies a resolved neighbor join request when policy allows join actions.
    pub fn apply_join_request_with_policy(
        &mut self,
        bounds: Rect,
        request: DockJoinRequest,
        policy: DockInteractionPolicy,
    ) -> bool {
        if !policy.sanitized().splitters.allow_join {
            return false;
        }

        let layout = solve_dock_layout(self, bounds);
        if !join_request_matches_layout(&layout, request) {
            return false;
        }

        self.merge_frames(request.source_frame, request.target_frame)
    }

    /// Resolves and applies a neighbor join against the current dock layout.
    pub fn join_neighbor(
        &mut self,
        bounds: Rect,
        source_frame: FrameId,
        direction: DockNeighborDirection,
    ) -> bool {
        self.join_neighbor_with_policy(
            bounds,
            source_frame,
            direction,
            DockInteractionPolicy::default(),
        )
    }

    /// Resolves and applies a neighbor join when policy allows join actions.
    pub fn join_neighbor_with_policy(
        &mut self,
        bounds: Rect,
        source_frame: FrameId,
        direction: DockNeighborDirection,
        policy: DockInteractionPolicy,
    ) -> bool {
        if !policy.sanitized().splitters.allow_join {
            return false;
        }

        let layout = solve_dock_layout(self, bounds);
        let Some(target_frame) = frame_neighbor(&layout, source_frame, direction) else {
            return false;
        };
        let request = DockJoinRequest {
            source_frame,
            direction,
            target_frame,
        };

        if !join_request_matches_layout(&layout, request) {
            return false;
        }

        self.merge_frames(source_frame, target_frame)
    }

    /// Applies a resolved neighbor swap request against the current dock layout.
    pub fn apply_swap_request(&mut self, bounds: Rect, request: DockSwapRequest) -> bool {
        self.apply_swap_request_with_policy(bounds, request, DockInteractionPolicy::default())
    }

    /// Applies a resolved neighbor swap request when policy allows swap actions.
    pub fn apply_swap_request_with_policy(
        &mut self,
        bounds: Rect,
        request: DockSwapRequest,
        policy: DockInteractionPolicy,
    ) -> bool {
        if !policy.sanitized().splitters.allow_swap {
            return false;
        }

        let layout = solve_dock_layout(self, bounds);
        if !swap_request_matches_layout(&layout, request) {
            return false;
        }

        swap_frame_leaves(&mut self.root, request.source_frame, request.target_frame)
    }

    /// Resolves and applies a neighbor swap against the current dock layout.
    pub fn swap_neighbor(
        &mut self,
        bounds: Rect,
        source_frame: FrameId,
        direction: DockNeighborDirection,
    ) -> bool {
        self.swap_neighbor_with_policy(
            bounds,
            source_frame,
            direction,
            DockInteractionPolicy::default(),
        )
    }

    /// Resolves and applies a neighbor swap when policy allows swap actions.
    pub fn swap_neighbor_with_policy(
        &mut self,
        bounds: Rect,
        source_frame: FrameId,
        direction: DockNeighborDirection,
        policy: DockInteractionPolicy,
    ) -> bool {
        if !policy.sanitized().splitters.allow_swap {
            return false;
        }

        let layout = solve_dock_layout(self, bounds);
        let Some(target_frame) = frame_neighbor(&layout, source_frame, direction) else {
            return false;
        };
        let request = DockSwapRequest {
            source_frame,
            direction,
            target_frame,
        };

        if !swap_request_matches_layout(&layout, request) {
            return false;
        }

        swap_frame_leaves(&mut self.root, source_frame, target_frame)
    }

    /// Starts a tab drag when the frame owns the panel.
    #[must_use]
    pub fn begin_tab_drag(&self, source_frame: FrameId, panel: PanelId) -> Option<DockTabDrag> {
        self.frame(source_frame)
            .filter(|frame| frame.panels.iter().any(|item| item.id == panel))
            .map(|_| DockTabDrag {
                source_frame,
                panel,
            })
    }

    /// Applies a dragged tab to an explicit dock target.
    pub fn drop_tab(&mut self, drag: DockTabDrag, target: DockDropTarget) -> bool {
        match target {
            DockDropTarget::Tab { frame } => {
                if drag.source_frame == frame {
                    return self.select_panel(frame, drag.panel);
                }
                self.move_panel(drag.source_frame, frame, drag.panel)
            }
            DockDropTarget::Split {
                frame,
                placement,
                new_frame,
                ratio,
                min_first,
                min_second,
            } => self.split_panel(
                drag.source_frame,
                drag.panel,
                DockSplitInsertion {
                    target_frame: frame,
                    placement,
                    new_frame,
                    ratio,
                    min_first,
                    min_second,
                },
            ),
        }
    }

    /// Inserts one panel as a new frame split adjacent to an existing frame.
    pub fn split_panel(
        &mut self,
        source_frame: FrameId,
        panel: PanelId,
        insertion: DockSplitInsertion,
    ) -> bool {
        if self.frame(insertion.target_frame).is_none()
            || self.frame(insertion.new_frame).is_some()
            || !insertion.ratio.is_finite()
            || !(0.0..=1.0).contains(&insertion.ratio)
            || !insertion.min_first.is_finite()
            || insertion.min_first < 0.0
            || !insertion.min_second.is_finite()
            || insertion.min_second < 0.0
        {
            return false;
        }

        if source_frame == insertion.target_frame
            && self
                .frame(source_frame)
                .is_none_or(|frame| frame.panels.len() <= 1)
        {
            return false;
        }

        let Some((panel, dismissible)) = self
            .frame_mut(source_frame)
            .and_then(|frame| frame.remove_panel_with_policy(panel))
        else {
            return false;
        };

        let mut inserted = Frame::new(insertion.new_frame, vec![panel]);
        inserted.set_panel_dismissible(inserted.panels[0].id, dismissible);
        prune_empty_frames(&mut self.root);

        let inserted = insert_frame_split(&mut self.root, insertion, inserted);
        if inserted {
            self.active_frame = Some(insertion.new_frame);
        } else {
            self.refresh_active_frame();
        }
        inserted
    }

    /// Resizes a split addressed by path using a drag delta in logical units.
    pub fn resize_split(&mut self, path: &DockSplitPath, bounds: Rect, delta: Vec2) -> bool {
        self.resize_split_with_policy(path, bounds, delta, DockInteractionPolicy::default())
    }

    /// Resizes a split when policy allows splitter drag resize.
    pub fn resize_split_with_policy(
        &mut self,
        path: &DockSplitPath,
        bounds: Rect,
        delta: Vec2,
        policy: DockInteractionPolicy,
    ) -> bool {
        if !policy.sanitized().splitters.allow_resize {
            return false;
        }

        resize_split_at_path(&mut self.root, path.elements(), bounds, delta)
    }

    /// Creates a snapshot for persistence.
    #[must_use]
    pub fn snapshot(&self) -> DockSnapshot {
        DockSnapshot {
            active_frame: self.active_frame(),
            root: snapshot_node(&self.root),
        }
    }

    /// Creates an additive workspace snapshot shell around the dock snapshot.
    ///
    /// Panel instance records are supplied by the application because the UI
    /// toolkit does not own panel type registration or panel state storage.
    #[must_use]
    pub fn workspace_snapshot(
        &self,
        panel_instances: Vec<PanelInstanceSnapshot>,
    ) -> WorkspaceSnapshot {
        WorkspaceSnapshot {
            dock: self.snapshot(),
            panel_instances,
        }
    }

    /// Restores a snapshot after validation.
    ///
    /// # Errors
    ///
    /// Returns [`DockRestoreError`] when persisted dock data is structurally
    /// invalid, contains duplicate identities, or stores invalid split values.
    pub fn restore(snapshot: DockSnapshot) -> Result<Self, DockRestoreError> {
        validate_dock_snapshot(&snapshot)?;
        let root = restore_node(snapshot.root);
        let active_frame = snapshot
            .active_frame
            .or_else(|| first_valid_frame_id(&root));
        Ok(Self { root, active_frame })
    }

    /// Restores a workspace snapshot after validating its panel instance shell.
    ///
    /// # Errors
    ///
    /// Returns [`WorkspaceRestoreError`] when the dock snapshot is invalid, an
    /// instance record is missing or stale, or the supplied panel type
    /// descriptors are not a deterministic set.
    pub fn restore_workspace(
        snapshot: WorkspaceSnapshot,
        descriptors: &[PanelTypeDescriptor],
    ) -> Result<Self, WorkspaceRestoreError> {
        snapshot.restore_dock(descriptors)
    }

    fn refresh_active_frame(&mut self) {
        if self.active_frame().is_none() {
            self.active_frame = first_valid_frame_id(&self.root);
        }
    }
}

/// Request for splitting a dragged panel into a new frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DockSplitInsertion {
    /// Existing frame to split around.
    pub target_frame: FrameId,
    /// Placement of the new frame relative to the target frame.
    pub placement: DockPlacement,
    /// Frame ID for the newly inserted frame.
    pub new_frame: FrameId,
    /// Initial split ratio.
    pub ratio: f32,
    /// Minimum first child size.
    pub min_first: f32,
    /// Minimum second child size.
    pub min_second: f32,
}

impl DockSplitInsertion {
    /// Creates a split insertion request with editor-friendly defaults.
    #[must_use]
    pub const fn new(target_frame: FrameId, placement: DockPlacement, new_frame: FrameId) -> Self {
        Self {
            target_frame,
            placement,
            new_frame,
            ratio: DEFAULT_SPLIT_RATIO,
            min_first: DEFAULT_SPLIT_MINIMUM,
            min_second: DEFAULT_SPLIT_MINIMUM,
        }
    }
}

fn collect_frames<'a>(node: &'a DockNode, frames: &mut Vec<&'a Frame>) {
    match node {
        DockNode::Frame(frame) => frames.push(frame),
        DockNode::Split { first, second, .. } => {
            collect_frames(first, frames);
            collect_frames(second, frames);
        }
    }
}

fn collect_frame_ids(node: &DockNode) -> Vec<FrameId> {
    let mut frames = Vec::new();
    collect_frame_ids_inner(node, &mut frames);
    frames
}

fn collect_frame_ids_inner(node: &DockNode, frames: &mut Vec<FrameId>) {
    match node {
        DockNode::Frame(frame) => frames.push(frame.id),
        DockNode::Split { first, second, .. } => {
            collect_frame_ids_inner(first, frames);
            collect_frame_ids_inner(second, frames);
        }
    }
}

fn split_children_at_path<'a>(
    node: &'a DockNode,
    path: &[DockPathElement],
) -> Option<(Axis, &'a DockNode, &'a DockNode)> {
    match (node, path) {
        (
            DockNode::Split {
                axis,
                first,
                second,
                ..
            },
            [],
        ) => Some((*axis, first, second)),
        (DockNode::Split { first, .. }, [DockPathElement::First, rest @ ..]) => {
            split_children_at_path(first, rest)
        }
        (DockNode::Split { second, .. }, [DockPathElement::Second, rest @ ..]) => {
            split_children_at_path(second, rest)
        }
        _ => None,
    }
}

fn find_frame_mut(node: &mut DockNode, id: FrameId) -> Option<&mut Frame> {
    match node {
        DockNode::Frame(frame) if frame.id == id => Some(frame),
        DockNode::Frame(_) => None,
        DockNode::Split { first, second, .. } => {
            find_frame_mut(first, id).or_else(|| find_frame_mut(second, id))
        }
    }
}

fn find_frame(node: &DockNode, id: FrameId) -> Option<&Frame> {
    match node {
        DockNode::Frame(frame) if frame.id == id => Some(frame),
        DockNode::Frame(_) => None,
        DockNode::Split { first, second, .. } => {
            find_frame(first, id).or_else(|| find_frame(second, id))
        }
    }
}

fn frame_is_valid(frame: &Frame) -> bool {
    !frame.panels.is_empty()
}

fn first_valid_frame_id(node: &DockNode) -> Option<FrameId> {
    match node {
        DockNode::Frame(frame) if frame_is_valid(frame) => Some(frame.id),
        DockNode::Frame(_) => None,
        DockNode::Split { first, second, .. } => {
            first_valid_frame_id(first).or_else(|| first_valid_frame_id(second))
        }
    }
}

fn prune_empty_frames(node: &mut DockNode) -> bool {
    match node {
        DockNode::Frame(frame) => !frame.panels.is_empty(),
        DockNode::Split { first, second, .. } => {
            let first_has_panels = prune_empty_frames(first);
            let second_has_panels = prune_empty_frames(second);
            match (first_has_panels, second_has_panels) {
                (true, true) => true,
                (true, false) => {
                    *node = (**first).clone();
                    true
                }
                (false, true) => {
                    *node = (**second).clone();
                    true
                }
                (false, false) => false,
            }
        }
    }
}

fn insert_frame_split(node: &mut DockNode, insertion: DockSplitInsertion, inserted: Frame) -> bool {
    match node {
        DockNode::Frame(frame) if frame.id == insertion.target_frame => {
            let target = DockNode::Frame(frame.clone());
            let inserted = DockNode::Frame(inserted);
            let (first, second) = if insertion.placement.insert_before_target() {
                (inserted, target)
            } else {
                (target, inserted)
            };
            *node = DockNode::Split {
                axis: insertion.placement.axis(),
                ratio: insertion.ratio,
                min_first: insertion.min_first,
                min_second: insertion.min_second,
                first: Box::new(first),
                second: Box::new(second),
            };
            true
        }
        DockNode::Frame(_) => false,
        DockNode::Split { first, second, .. } => {
            insert_frame_split(first, insertion, inserted.clone())
                || insert_frame_split(second, insertion, inserted)
        }
    }
}

fn swap_frame_leaves(root: &mut DockNode, first: FrameId, second: FrameId) -> bool {
    if first == second {
        return false;
    }

    let Some(first_path) = find_frame_path(root, first) else {
        return false;
    };
    let Some(second_path) = find_frame_path(root, second) else {
        return false;
    };
    if first_path == second_path {
        return false;
    }

    let Some(first_frame) = frame_at_path(root, &first_path).cloned() else {
        return false;
    };
    let Some(second_frame) = frame_at_path(root, &second_path).cloned() else {
        return false;
    };

    let Some(target) = frame_at_path_mut(root, &first_path) else {
        return false;
    };
    *target = second_frame;

    let Some(target) = frame_at_path_mut(root, &second_path) else {
        return false;
    };
    *target = first_frame;
    true
}

fn find_frame_path(root: &DockNode, frame: FrameId) -> Option<Vec<DockPathElement>> {
    let mut path = Vec::new();
    find_frame_path_inner(root, frame, &mut path).then_some(path)
}

fn find_frame_path_inner(node: &DockNode, frame: FrameId, path: &mut Vec<DockPathElement>) -> bool {
    match node {
        DockNode::Frame(candidate) => candidate.id == frame,
        DockNode::Split { first, second, .. } => {
            path.push(DockPathElement::First);
            if find_frame_path_inner(first, frame, path) {
                return true;
            }
            path.pop();

            path.push(DockPathElement::Second);
            if find_frame_path_inner(second, frame, path) {
                return true;
            }
            path.pop();
            false
        }
    }
}

fn frame_at_path<'a>(node: &'a DockNode, path: &[DockPathElement]) -> Option<&'a Frame> {
    match (node, path) {
        (DockNode::Frame(frame), []) => Some(frame),
        (DockNode::Split { first, .. }, [DockPathElement::First, rest @ ..]) => {
            frame_at_path(first, rest)
        }
        (DockNode::Split { second, .. }, [DockPathElement::Second, rest @ ..]) => {
            frame_at_path(second, rest)
        }
        _ => None,
    }
}

fn frame_at_path_mut<'a>(
    node: &'a mut DockNode,
    path: &[DockPathElement],
) -> Option<&'a mut Frame> {
    match (node, path) {
        (DockNode::Frame(frame), []) => Some(frame),
        (DockNode::Split { first, .. }, [DockPathElement::First, rest @ ..]) => {
            frame_at_path_mut(first, rest)
        }
        (DockNode::Split { second, .. }, [DockPathElement::Second, rest @ ..]) => {
            frame_at_path_mut(second, rest)
        }
        _ => None,
    }
}

fn resize_split_at_path(
    node: &mut DockNode,
    path: &[DockPathElement],
    bounds: Rect,
    delta: Vec2,
) -> bool {
    let DockNode::Split {
        axis,
        ratio,
        min_first,
        min_second,
        first,
        second,
    } = node
    else {
        return false;
    };

    if path.is_empty() {
        *ratio = split_ratio_from_drag(*axis, bounds, *ratio, *min_first, *min_second, delta);
        return true;
    }

    let (first_rect, second_rect) =
        split_child_rects(*axis, bounds, *ratio, *min_first, *min_second);
    match path[0] {
        DockPathElement::First => resize_split_at_path(first, &path[1..], first_rect, delta),
        DockPathElement::Second => resize_split_at_path(second, &path[1..], second_rect, delta),
    }
}

/// Resolved frame rectangle.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrameLayout {
    /// Frame identity.
    pub frame: FrameId,
    /// Frame rectangle.
    pub rect: Rect,
}

/// Cardinal direction for dock frame neighbor lookup.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DockNeighborDirection {
    /// Candidate frame is to the left of the source frame.
    Left,
    /// Candidate frame is to the right of the source frame.
    Right,
    /// Candidate frame is above the source frame.
    Up,
    /// Candidate frame is below the source frame.
    Down,
}

/// Resolved neighboring frames for one dock frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameNeighbors {
    /// Source frame.
    pub frame: FrameId,
    /// Best neighboring frame to the left.
    pub left: Option<FrameId>,
    /// Best neighboring frame to the right.
    pub right: Option<FrameId>,
    /// Best neighboring frame above.
    pub up: Option<FrameId>,
    /// Best neighboring frame below.
    pub down: Option<FrameId>,
}

impl FrameNeighbors {
    /// Creates empty neighbor data for a frame.
    #[must_use]
    pub const fn empty(frame: FrameId) -> Self {
        Self {
            frame,
            left: None,
            right: None,
            up: None,
            down: None,
        }
    }

    /// Returns the neighbor for a cardinal direction.
    #[must_use]
    pub const fn neighbor(self, direction: DockNeighborDirection) -> Option<FrameId> {
        match direction {
            DockNeighborDirection::Left => self.left,
            DockNeighborDirection::Right => self.right,
            DockNeighborDirection::Up => self.up,
            DockNeighborDirection::Down => self.down,
        }
    }
}

/// Resolves a dock tree into frame rectangles.
#[must_use]
pub fn solve_dock_layout(area: &Dock, bounds: Rect) -> Vec<FrameLayout> {
    let mut frames = Vec::new();
    solve_node(&area.root, bounds, &mut frames);
    frames
}

/// Resolves cardinal frame neighbors from solved dock layout rectangles.
///
/// Ties are deterministic: nearest edge distance wins, then greatest
/// perpendicular edge overlap, then the lowest raw [`FrameId`].
#[must_use]
pub fn solve_dock_neighbors(area: &Dock, bounds: Rect) -> Vec<FrameNeighbors> {
    let frames = solve_dock_layout(area, bounds);
    frames
        .iter()
        .map(|layout| FrameNeighbors {
            frame: layout.frame,
            left: frame_neighbor(&frames, layout.frame, DockNeighborDirection::Left),
            right: frame_neighbor(&frames, layout.frame, DockNeighborDirection::Right),
            up: frame_neighbor(&frames, layout.frame, DockNeighborDirection::Up),
            down: frame_neighbor(&frames, layout.frame, DockNeighborDirection::Down),
        })
        .collect()
}

/// Resolves the best neighbor for one frame from solved dock layout rectangles.
///
/// Invalid or empty source/candidate rectangles are ignored. The source frame
/// is never returned as its own neighbor.
#[must_use]
pub fn frame_neighbor(
    frames: &[FrameLayout],
    frame: FrameId,
    direction: DockNeighborDirection,
) -> Option<FrameId> {
    let source = frames.iter().find(|layout| layout.frame == frame)?;
    if !valid_neighbor_rect(source.rect) {
        return None;
    }

    let mut best = None;
    for candidate in frames {
        if candidate.frame == frame || !valid_neighbor_rect(candidate.rect) {
            continue;
        }
        let Some(score) = neighbor_candidate_score(source.rect, candidate.rect, direction) else {
            continue;
        };
        if neighbor_candidate_is_better(best, (candidate.frame, score)) {
            best = Some((candidate.frame, score));
        }
    }

    best.map(|(frame, _)| frame)
}

/// Resolves splitter interaction rectangles for a dock tree.
#[must_use]
pub fn solve_dock_splitters(area: &Dock, bounds: Rect, thickness: f32) -> Vec<DockSplitter> {
    solve_dock_splitters_with_style(
        area,
        bounds,
        DockChromeStyle::default().with_splitter_hit_thickness(thickness),
    )
}

/// Resolves splitter interaction rectangles using dock chrome style.
#[must_use]
pub fn solve_dock_splitters_with_style(
    area: &Dock,
    bounds: Rect,
    style: DockChromeStyle,
) -> Vec<DockSplitter> {
    let mut splitters = Vec::new();
    solve_splitters(
        &area.root,
        bounds,
        style.sanitized().splitter_hit_thickness,
        &DockSplitPath::root(),
        &mut splitters,
    );
    splitters
}

fn solve_splitters(
    node: &DockNode,
    bounds: Rect,
    thickness: f32,
    path: &DockSplitPath,
    splitters: &mut Vec<DockSplitter>,
) {
    let DockNode::Split {
        axis,
        ratio,
        min_first,
        min_second,
        first,
        second,
    } = node
    else {
        return;
    };

    let (first_rect, second_rect) =
        split_child_rects(*axis, bounds, *ratio, *min_first, *min_second);
    splitters.push(DockSplitter {
        path: path.clone(),
        axis: *axis,
        rect: splitter_rect(*axis, first_rect, bounds, thickness),
        ratio: finite_ratio(*ratio),
        min_first: finite_non_negative(*min_first),
        min_second: finite_non_negative(*min_second),
    });
    solve_splitters(
        first,
        first_rect,
        thickness,
        &path.child(DockPathElement::First),
        splitters,
    );
    solve_splitters(
        second,
        second_rect,
        thickness,
        &path.child(DockPathElement::Second),
        splitters,
    );
}

fn solve_node(node: &DockNode, bounds: Rect, frames: &mut Vec<FrameLayout>) {
    match node {
        DockNode::Frame(frame) => frames.push(FrameLayout {
            frame: frame.id,
            rect: bounds,
        }),
        DockNode::Split {
            axis,
            ratio,
            min_first,
            min_second,
            first,
            second,
        } => {
            let (first_rect, second_rect) =
                split_child_rects(*axis, bounds, *ratio, *min_first, *min_second);
            solve_node(first, first_rect, frames);
            solve_node(second, second_rect, frames);
        }
    }
}

fn split_child_rects(
    axis: Axis,
    bounds: Rect,
    ratio: f32,
    min_first: f32,
    min_second: f32,
) -> (Rect, Rect) {
    let total = split_total(axis, bounds);
    let first_size = split_first_size(total, ratio, min_first, min_second);
    let second_size = (total - first_size).max(0.0);
    let x = finite_coordinate(bounds.x);
    let y = finite_coordinate(bounds.y);
    let width = finite_non_negative(bounds.width);
    let height = finite_non_negative(bounds.height);
    match axis {
        Axis::Horizontal => (
            Rect::new(x, y, first_size, height),
            Rect::new(x + first_size, y, second_size, height),
        ),
        Axis::Vertical => (
            Rect::new(x, y, width, first_size),
            Rect::new(x, y + first_size, width, second_size),
        ),
    }
}

fn split_total(axis: Axis, bounds: Rect) -> f32 {
    match axis {
        Axis::Horizontal => finite_non_negative(bounds.width),
        Axis::Vertical => finite_non_negative(bounds.height),
    }
}

fn split_first_size(total: f32, ratio: f32, min_first: f32, min_second: f32) -> f32 {
    let total = finite_non_negative(total);
    let min_first = finite_non_negative(min_first);
    let min_second = finite_non_negative(min_second);
    let desired = total * finite_ratio(ratio);
    if total >= min_first + min_second {
        desired.clamp(min_first, total - min_second)
    } else {
        desired.max(min_first.min(total)).min(total)
    }
}

fn splitter_rect(axis: Axis, first_rect: Rect, bounds: Rect, thickness: f32) -> Rect {
    let half = thickness * 0.5;
    let x = finite_coordinate(bounds.x);
    let y = finite_coordinate(bounds.y);
    let width = finite_non_negative(bounds.width);
    let height = finite_non_negative(bounds.height);
    match axis {
        Axis::Horizontal => Rect::new(
            finite_coordinate(first_rect.max_x()) - half,
            y,
            thickness,
            height,
        ),
        Axis::Vertical => Rect::new(
            x,
            finite_coordinate(first_rect.max_y()) - half,
            width,
            thickness,
        ),
    }
}

fn splitter_thickness(thickness: f32) -> f32 {
    if thickness.is_finite() && thickness > 0.0 {
        thickness
    } else {
        DEFAULT_SPLITTER_THICKNESS
    }
}

fn sanitize_drop_edge_fraction(fraction: f32) -> f32 {
    if fraction.is_finite() {
        fraction.clamp(0.0, 0.5)
    } else {
        DROP_EDGE_FRACTION
    }
}

/// Maps a splitter drag delta to a clamped split ratio.
#[must_use]
pub fn split_ratio_from_drag(
    axis: Axis,
    bounds: Rect,
    ratio: f32,
    min_first: f32,
    min_second: f32,
    delta: Vec2,
) -> f32 {
    let total = split_total(axis, bounds);
    if total <= 0.0 {
        return finite_ratio(ratio);
    }
    let delta = match axis {
        Axis::Horizontal => delta.x,
        Axis::Vertical => delta.y,
    };
    let current = split_first_size(total, ratio, min_first, min_second);
    let desired = current + if delta.is_finite() { delta } else { 0.0 };
    split_first_size(total, desired / total, min_first, min_second) / total
}

/// Resolves a frame-local drop zone for a pointer position.
#[must_use]
pub fn resolve_frame_drop_zone(rect: Rect, point: Point) -> Option<DockDropZone> {
    resolve_frame_drop_zone_with_policy(rect, point, DockInteractionPolicy::default())
}

/// Resolves a frame-local drop zone using dock interaction policy.
#[must_use]
pub fn resolve_frame_drop_zone_with_policy(
    rect: Rect,
    point: Point,
    policy: DockInteractionPolicy,
) -> Option<DockDropZone> {
    if !valid_drop_rect(rect) || !valid_drop_point(point) {
        return None;
    }

    if !rect.contains_point(point) {
        return None;
    }

    let left = (point.x - rect.min_x()).max(0.0);
    let right = (rect.max_x() - point.x).max(0.0);
    let top = (point.y - rect.min_y()).max(0.0);
    let bottom = (rect.max_y() - point.y).max(0.0);
    let edge_fraction = policy.sanitized().drop_targets.edge_fraction;
    let edge_x = finite_non_negative(rect.width) * edge_fraction;
    let edge_y = finite_non_negative(rect.height) * edge_fraction;

    let mut best = (DockDropZone::Center, f32::INFINITY);
    for (zone, distance, limit) in [
        (DockDropZone::Left, left, edge_x),
        (DockDropZone::Right, right, edge_x),
        (DockDropZone::Top, top, edge_y),
        (DockDropZone::Bottom, bottom, edge_y),
    ] {
        if distance <= limit && distance < best.1 {
            best = (zone, distance);
        }
    }

    Some(best.0)
}

/// Resolves a dock drop target from frame layout and a pointer position.
#[must_use]
pub fn resolve_dock_drop_target(
    frames: &[FrameLayout],
    point: Point,
    new_frame: FrameId,
) -> Option<DockDropTarget> {
    resolve_dock_drop_target_with_policy(frames, point, new_frame, DockInteractionPolicy::default())
}

/// Resolves a dock drop target using dock interaction policy.
#[must_use]
pub fn resolve_dock_drop_target_with_policy(
    frames: &[FrameLayout],
    point: Point,
    new_frame: FrameId,
    policy: DockInteractionPolicy,
) -> Option<DockDropTarget> {
    let policy = policy.sanitized();
    frames.iter().find_map(|layout| {
        let zone = resolve_frame_drop_zone_with_policy(layout.rect, point, policy)?;
        match zone.placement() {
            Some(placement) if policy.drop_targets.allow_split_insertion => {
                Some(DockDropTarget::split(layout.frame, placement, new_frame))
            }
            None if policy.drop_targets.allow_tab_merge => Some(DockDropTarget::tab(layout.frame)),
            _ => None,
        }
    })
}

fn resolve_frame_split_affordance_with_policy(
    frames: &[FrameLayout],
    point: Point,
    policy: DockInteractionPolicy,
) -> Option<(FrameId, DockPlacement)> {
    for layout in frames {
        let Some(zone) = resolve_frame_drop_zone_with_policy(layout.rect, point, policy) else {
            continue;
        };
        return policy
            .sanitized()
            .drop_targets
            .allow_split_insertion
            .then_some(zone)
            .and_then(DockDropZone::placement)
            .map(|placement| (layout.frame, placement));
    }

    None
}

fn splitter_adjacent_frame(
    frames: &[FrameLayout],
    candidates: &[FrameId],
    splitter: &DockSplitter,
    side: DockSplitterSide,
) -> Option<FrameId> {
    if !valid_neighbor_rect(splitter.rect) {
        return None;
    }

    let mut best = None;
    for candidate in candidates {
        let Some(layout) = frames.iter().find(|layout| layout.frame == *candidate) else {
            continue;
        };
        if !valid_neighbor_rect(layout.rect) {
            continue;
        }
        let Some(score) = splitter_adjacent_score(layout.rect, splitter, side) else {
            continue;
        };
        if splitter_adjacent_is_better(best, (*candidate, score)) {
            best = Some((*candidate, score));
        }
    }

    best.map(|(frame, _)| frame)
}

#[derive(Debug, Clone, Copy)]
struct SplitterAdjacentScore {
    overlap: f32,
    distance: f32,
}

fn splitter_adjacent_score(
    rect: Rect,
    splitter: &DockSplitter,
    side: DockSplitterSide,
) -> Option<SplitterAdjacentScore> {
    let center_x = splitter.rect.x + splitter.rect.width * 0.5;
    let center_y = splitter.rect.y + splitter.rect.height * 0.5;
    let (overlap, distance) = match (splitter.axis, side) {
        (Axis::Horizontal, DockSplitterSide::First) => (
            range_overlap(
                rect.min_y(),
                rect.max_y(),
                splitter.rect.min_y(),
                splitter.rect.max_y(),
            ),
            (center_x - rect.max_x()).abs(),
        ),
        (Axis::Horizontal, DockSplitterSide::Second) => (
            range_overlap(
                rect.min_y(),
                rect.max_y(),
                splitter.rect.min_y(),
                splitter.rect.max_y(),
            ),
            (rect.min_x() - center_x).abs(),
        ),
        (Axis::Vertical, DockSplitterSide::First) => (
            range_overlap(
                rect.min_x(),
                rect.max_x(),
                splitter.rect.min_x(),
                splitter.rect.max_x(),
            ),
            (center_y - rect.max_y()).abs(),
        ),
        (Axis::Vertical, DockSplitterSide::Second) => (
            range_overlap(
                rect.min_x(),
                rect.max_x(),
                splitter.rect.min_x(),
                splitter.rect.max_x(),
            ),
            (rect.min_y() - center_y).abs(),
        ),
    };

    (overlap > 0.0 && distance.is_finite()).then_some(SplitterAdjacentScore { overlap, distance })
}

fn splitter_adjacent_is_better(
    best: Option<(FrameId, SplitterAdjacentScore)>,
    candidate: (FrameId, SplitterAdjacentScore),
) -> bool {
    let Some((best_frame, best_score)) = best else {
        return true;
    };
    let (candidate_frame, candidate_score) = candidate;
    match candidate_score.distance.total_cmp(&best_score.distance) {
        Ordering::Less => true,
        Ordering::Greater => false,
        Ordering::Equal => match candidate_score.overlap.total_cmp(&best_score.overlap) {
            Ordering::Greater => true,
            Ordering::Less => false,
            Ordering::Equal => candidate_frame.raw() < best_frame.raw(),
        },
    }
}

fn valid_drop_rect(rect: Rect) -> bool {
    rect.x.is_finite()
        && rect.y.is_finite()
        && rect.width.is_finite()
        && rect.height.is_finite()
        && !rect.is_empty()
}

fn valid_drop_point(point: Point) -> bool {
    point.x.is_finite() && point.y.is_finite()
}

#[derive(Debug, Clone, Copy)]
struct NeighborCandidateScore {
    overlap: f32,
    distance: f32,
}

fn neighbor_candidate_score(
    source: Rect,
    candidate: Rect,
    direction: DockNeighborDirection,
) -> Option<NeighborCandidateScore> {
    let (overlap, distance) = match direction {
        DockNeighborDirection::Left => (
            range_overlap(
                source.min_y(),
                source.max_y(),
                candidate.min_y(),
                candidate.max_y(),
            ),
            source.min_x() - candidate.max_x(),
        ),
        DockNeighborDirection::Right => (
            range_overlap(
                source.min_y(),
                source.max_y(),
                candidate.min_y(),
                candidate.max_y(),
            ),
            candidate.min_x() - source.max_x(),
        ),
        DockNeighborDirection::Up => (
            range_overlap(
                source.min_x(),
                source.max_x(),
                candidate.min_x(),
                candidate.max_x(),
            ),
            source.min_y() - candidate.max_y(),
        ),
        DockNeighborDirection::Down => (
            range_overlap(
                source.min_x(),
                source.max_x(),
                candidate.min_x(),
                candidate.max_x(),
            ),
            candidate.min_y() - source.max_y(),
        ),
    };

    (overlap > 0.0 && distance >= 0.0).then_some(NeighborCandidateScore { overlap, distance })
}

fn neighbor_candidate_is_better(
    best: Option<(FrameId, NeighborCandidateScore)>,
    candidate: (FrameId, NeighborCandidateScore),
) -> bool {
    let Some((best_frame, best_score)) = best else {
        return true;
    };
    let (candidate_frame, candidate_score) = candidate;
    match candidate_score.distance.total_cmp(&best_score.distance) {
        Ordering::Less => true,
        Ordering::Greater => false,
        Ordering::Equal => match candidate_score.overlap.total_cmp(&best_score.overlap) {
            Ordering::Greater => true,
            Ordering::Less => false,
            Ordering::Equal => candidate_frame.raw() < best_frame.raw(),
        },
    }
}

fn range_overlap(first_min: f32, first_max: f32, second_min: f32, second_max: f32) -> f32 {
    first_max.min(second_max) - first_min.max(second_min)
}

fn valid_neighbor_rect(rect: Rect) -> bool {
    rect.x.is_finite()
        && rect.y.is_finite()
        && rect.width.is_finite()
        && rect.height.is_finite()
        && !rect.is_empty()
}

fn finite_ratio(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        0.5
    }
}

fn finite_non_negative(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}

fn finite_coordinate(value: f32) -> f32 {
    if value.is_finite() { value } else { 0.0 }
}

/// Tab presentation data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameTab {
    /// Panel identity.
    pub panel: PanelId,
    /// Tab title.
    pub title: String,
    /// Whether this tab is active.
    pub active: bool,
    /// Whether this tab can be closed.
    pub close_visible: bool,
    /// Whether this tab can begin a drag operation.
    pub draggable: bool,
}

/// Produces frame tab presentation records.
#[must_use]
pub fn frame_tabs(frame: &Frame) -> Vec<FrameTab> {
    frame
        .panels
        .iter()
        .enumerate()
        .map(|(index, panel)| FrameTab {
            panel: panel.id,
            title: panel.title.clone(),
            active: index == frame.active,
            close_visible: frame.panel_dismissible(panel.id),
            draggable: true,
        })
        .collect()
}

/// Persistable dock snapshot.
#[derive(Debug, Clone, PartialEq)]
pub struct DockSnapshot {
    /// Root-owned active frame identity.
    pub active_frame: Option<FrameId>,
    /// Root snapshot node.
    pub root: DockSnapshotNode,
}

impl DockSnapshot {
    /// Returns structured diagnostics for this snapshot.
    #[must_use]
    pub fn diagnostics(&self) -> DockSnapshotDiagnostics {
        validate_dock_snapshot_diagnostics(self)
    }
}

/// Snapshot node.
#[derive(Debug, Clone, PartialEq)]
pub enum DockSnapshotNode {
    /// Frame snapshot.
    Frame {
        /// Frame identity.
        id: FrameId,
        /// Panels.
        panels: Vec<Panel>,
        /// Active panel index.
        active: usize,
        /// Panels whose frame tabs expose close/dismiss affordances.
        dismissible_panels: Vec<PanelId>,
    },
    /// Split snapshot.
    Split {
        /// Split axis.
        axis: Axis,
        /// First child ratio.
        ratio: f32,
        /// Minimum first size.
        min_first: f32,
        /// Minimum second size.
        min_second: f32,
        /// First child.
        first: Box<DockSnapshotNode>,
        /// Second child.
        second: Box<DockSnapshotNode>,
    },
}

/// Snapshot restore error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DockRestoreError {
    /// Frame contains no panels.
    EmptyFrame,
    /// Active tab index is outside the panel list.
    InvalidActiveIndex,
    /// Two frames use the same stable frame identity.
    DuplicateFrameId,
    /// Two panels use the same stable panel identity.
    DuplicatePanelId,
    /// Dismissible panel policy references a panel missing from the frame.
    InvalidDismissiblePanel,
    /// Dismissible panel policy contains the same panel more than once.
    DuplicateDismissiblePanel,
    /// Active frame identity references a frame missing from the dock tree.
    InvalidActiveFrame,
    /// Split ratio is not finite or is outside the inclusive 0.0..=1.0 range.
    InvalidSplitRatio,
    /// Split minimum is not finite or is negative.
    InvalidSplitMinimum,
}

/// Snapshot diagnostic severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotDiagnosticSeverity {
    /// Validation error that prevents restore.
    Error,
    /// Non-fatal issue that should be visible to debug tooling.
    Warning,
}

/// Stable diagnostic code for dock snapshot validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DockSnapshotDiagnosticCode {
    /// Frame contains no panels.
    EmptyFrame,
    /// Two frames use the same stable frame identity.
    DuplicateFrameId,
    /// Two panels use the same stable panel identity.
    DuplicatePanelId,
    /// Active frame identity references a frame missing from the dock tree.
    InvalidActiveFrame,
    /// Active tab index is outside the panel list.
    InvalidActivePanelIndex,
    /// Split ratio is not finite or is outside the inclusive 0.0..=1.0 range.
    InvalidSplitRatio,
    /// Split minimum is not finite or is negative.
    InvalidSplitMinimum,
    /// Dismissible panel policy references a panel missing from the frame.
    InvalidDismissiblePanel,
    /// Dismissible panel policy contains the same panel more than once.
    DuplicateDismissiblePolicy,
}

impl DockSnapshotDiagnosticCode {
    /// Returns the stable string code for this diagnostic.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::EmptyFrame => "dock.empty_frame",
            Self::DuplicateFrameId => "dock.duplicate_frame_id",
            Self::DuplicatePanelId => "dock.duplicate_panel_id",
            Self::InvalidActiveFrame => "dock.invalid_active_frame",
            Self::InvalidActivePanelIndex => "dock.invalid_active_panel_index",
            Self::InvalidSplitRatio => "dock.invalid_split_ratio",
            Self::InvalidSplitMinimum => "dock.invalid_split_minimum",
            Self::InvalidDismissiblePanel => "dock.invalid_dismissible_panel",
            Self::DuplicateDismissiblePolicy => "dock.duplicate_dismissible_policy",
        }
    }
}

/// Split value identified by a dock snapshot diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DockSnapshotSplitValue {
    /// Split ratio.
    Ratio,
    /// Minimum size for the first child.
    MinFirst,
    /// Minimum size for the second child.
    MinSecond,
}

/// Structured diagnostic for dock snapshot validation.
#[derive(Debug, Clone, PartialEq)]
pub struct DockSnapshotDiagnostic {
    /// Stable diagnostic code.
    pub code: DockSnapshotDiagnosticCode,
    /// Diagnostic severity.
    pub severity: SnapshotDiagnosticSeverity,
    /// Tree path to the split or frame where the diagnostic was found.
    pub path: DockSplitPath,
    /// Frame identity when the diagnostic is frame-scoped.
    pub frame: Option<FrameId>,
    /// Panel identity when the diagnostic is panel-scoped.
    pub panel: Option<PanelId>,
    /// Invalid active panel index when applicable.
    pub active_index: Option<usize>,
    /// Panel count used to judge an active panel index.
    pub panel_count: Option<usize>,
    /// Split value involved in split diagnostics.
    pub split_value: Option<DockSnapshotSplitValue>,
}

impl DockSnapshotDiagnostic {
    fn new(code: DockSnapshotDiagnosticCode, path: DockSplitPath) -> Self {
        Self {
            code,
            severity: SnapshotDiagnosticSeverity::Error,
            path,
            frame: None,
            panel: None,
            active_index: None,
            panel_count: None,
            split_value: None,
        }
    }

    /// Returns the stable string code for this diagnostic.
    #[must_use]
    pub const fn stable_code(&self) -> &'static str {
        self.code.as_str()
    }
}

/// Structured dock snapshot diagnostics.
#[derive(Debug, Clone, PartialEq)]
pub struct DockSnapshotDiagnostics {
    /// Diagnostics in deterministic validation order.
    pub diagnostics: Vec<DockSnapshotDiagnostic>,
}

impl DockSnapshotDiagnostics {
    /// Returns true when no error diagnostics were emitted.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        !self.has_errors()
    }

    /// Returns true when at least one error diagnostic was emitted.
    #[must_use]
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == SnapshotDiagnosticSeverity::Error)
    }
}

/// Persistable metadata for one open panel instance.
///
/// This keeps the workspace shell typed while leaving panel content,
/// application state serialization, and factory behavior application-owned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PanelInstanceSnapshot {
    /// Stable identity for one open panel instance.
    pub id: PanelInstanceId,
    /// Developer-declared panel type identity for this instance.
    pub panel_type: PanelTypeId,
    /// Display title used by workspace tabs or persisted custom labels.
    pub title: String,
    /// Optional application-owned key for looking up persisted panel state.
    pub state_key: Option<String>,
}

impl PanelInstanceSnapshot {
    /// Creates a panel instance snapshot.
    #[must_use]
    pub fn new(id: PanelInstanceId, panel_type: PanelTypeId, title: impl Into<String>) -> Self {
        Self {
            id,
            panel_type,
            title: title.into(),
            state_key: None,
        }
    }

    /// Sets the optional application-owned state key.
    #[must_use]
    pub fn with_state_key(mut self, state_key: impl Into<String>) -> Self {
        self.state_key = Some(state_key.into());
        self
    }
}

/// Additive workspace snapshot shell around a dock snapshot.
///
/// `DockSnapshot` remains usable on its own. This type adds enough typed
/// metadata for applications to validate panel instance references without
/// introducing panel factories or app state serialization into the widget layer.
#[derive(Debug, Clone, PartialEq)]
pub struct WorkspaceSnapshot {
    /// Persisted dock tree and active frame state.
    pub dock: DockSnapshot,
    /// Persisted open panel instance records.
    pub panel_instances: Vec<PanelInstanceSnapshot>,
}

impl WorkspaceSnapshot {
    /// Creates a workspace snapshot shell.
    #[must_use]
    pub const fn new(dock: DockSnapshot, panel_instances: Vec<PanelInstanceSnapshot>) -> Self {
        Self {
            dock,
            panel_instances,
        }
    }

    /// Validates the workspace shell against supplied panel type descriptors.
    ///
    /// # Errors
    ///
    /// Returns [`WorkspaceRestoreError`] for invalid dock snapshots, duplicate
    /// descriptors, duplicate instances, missing records, unknown panel types,
    /// or stale records.
    pub fn validate(
        &self,
        descriptors: &[PanelTypeDescriptor],
    ) -> Result<(), WorkspaceRestoreError> {
        let dock_validation = validate_dock_snapshot(&self.dock)?;
        validate_workspace_snapshot(self, descriptors, &dock_validation)
    }

    /// Returns structured diagnostics for this workspace snapshot.
    #[must_use]
    pub fn diagnostics(&self, descriptors: &[PanelTypeDescriptor]) -> WorkspaceSnapshotDiagnostics {
        validate_workspace_snapshot_diagnostics(self, descriptors)
    }

    /// Returns an explicit metadata-only repair plan for this workspace snapshot.
    ///
    /// The plan keeps strict validation unchanged: hard identity or dock
    /// corruption yields no repaired snapshot. Recoverable stale, missing, or
    /// unknown panel metadata remains visible through diagnostics and actions.
    #[must_use]
    pub fn repair_plan(&self, descriptors: &[PanelTypeDescriptor]) -> WorkspaceRepairPlan {
        plan_workspace_snapshot_repair(self, descriptors)
    }

    /// Returns the metadata-only repaired workspace snapshot when planning found
    /// no hard repair error.
    ///
    /// # Errors
    ///
    /// Returns [`WorkspaceRestoreError`] when the dock snapshot is invalid,
    /// duplicate identity metadata exists, or a missing panel instance cannot be
    /// represented by safe placeholder metadata.
    pub fn repair_snapshot(
        &self,
        descriptors: &[PanelTypeDescriptor],
    ) -> Result<WorkspaceSnapshot, WorkspaceRestoreError> {
        self.repair_plan(descriptors).into_repaired_snapshot()
    }

    /// Validates this workspace snapshot and restores its dock.
    ///
    /// # Errors
    ///
    /// Returns [`WorkspaceRestoreError`] when validation fails or dock restore
    /// rejects the dock snapshot.
    pub fn restore_dock(
        self,
        descriptors: &[PanelTypeDescriptor],
    ) -> Result<Dock, WorkspaceRestoreError> {
        self.validate(descriptors)?;
        Dock::restore(self.dock).map_err(WorkspaceRestoreError::Dock)
    }
}

/// Workspace snapshot validation and restore error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceRestoreError {
    /// The wrapped dock snapshot is invalid.
    Dock(DockRestoreError),
    /// A dock panel does not have a matching panel instance record.
    MissingPanelInstance {
        /// Missing panel instance identity.
        panel_instance: PanelInstanceId,
    },
    /// A panel instance references a panel type absent from the supplied descriptors.
    UnknownPanelType {
        /// Panel instance with the unknown type.
        panel_instance: PanelInstanceId,
        /// Unknown panel type identity.
        panel_type: PanelTypeId,
    },
    /// Two panel instance records use the same stable identity.
    DuplicatePanelInstanceId {
        /// Duplicated panel instance identity.
        panel_instance: PanelInstanceId,
    },
    /// Two panel type descriptors use the same stable identity.
    DuplicatePanelTypeDescriptor {
        /// Duplicated panel type identity.
        panel_type: PanelTypeId,
    },
    /// A panel instance record is not referenced by the dock snapshot.
    StalePanelInstance {
        /// Stale panel instance identity.
        panel_instance: PanelInstanceId,
    },
}

/// Stable diagnostic code for workspace snapshot validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceSnapshotDiagnosticCode {
    /// Two panel instance records use the same stable identity.
    DuplicatePanelInstanceId,
    /// Two panel type descriptors use the same stable identity.
    DuplicatePanelTypeDescriptor,
    /// A dock panel does not have a matching panel instance record.
    MissingPanelInstance,
    /// A panel instance record is not referenced by the dock snapshot.
    StalePanelInstance,
    /// A panel instance references a panel type absent from the supplied descriptors.
    UnknownPanelType,
    /// A dock tab title differs from its panel instance title.
    PanelTitleDrift,
}

impl WorkspaceSnapshotDiagnosticCode {
    /// Returns the stable string code for this diagnostic.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::DuplicatePanelInstanceId => "workspace.duplicate_panel_instance_id",
            Self::DuplicatePanelTypeDescriptor => "workspace.duplicate_panel_type_descriptor",
            Self::MissingPanelInstance => "workspace.missing_panel_instance",
            Self::StalePanelInstance => "workspace.stale_panel_instance",
            Self::UnknownPanelType => "workspace.unknown_panel_type",
            Self::PanelTitleDrift => "workspace.panel_title_drift",
        }
    }
}

/// Structured diagnostic for workspace snapshot validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceSnapshotDiagnostic {
    /// Stable diagnostic code.
    pub code: WorkspaceSnapshotDiagnosticCode,
    /// Diagnostic severity.
    pub severity: SnapshotDiagnosticSeverity,
    /// Panel instance identity when the diagnostic is instance-scoped.
    pub panel_instance: Option<PanelInstanceId>,
    /// Panel type identity when the diagnostic is type-scoped.
    pub panel_type: Option<PanelTypeId>,
    /// Dock frame containing the panel instance when known.
    pub frame: Option<FrameId>,
    /// Legacy dock panel identity when known.
    pub panel: Option<PanelId>,
    /// Title stored on the dock panel when relevant.
    pub dock_title: Option<String>,
    /// Title stored on the panel instance when relevant.
    pub instance_title: Option<String>,
}

impl WorkspaceSnapshotDiagnostic {
    fn new(code: WorkspaceSnapshotDiagnosticCode) -> Self {
        Self {
            code,
            severity: SnapshotDiagnosticSeverity::Error,
            panel_instance: None,
            panel_type: None,
            frame: None,
            panel: None,
            dock_title: None,
            instance_title: None,
        }
    }

    /// Returns the stable string code for this diagnostic.
    #[must_use]
    pub const fn stable_code(&self) -> &'static str {
        self.code.as_str()
    }
}

/// Structured workspace snapshot diagnostics.
#[derive(Debug, Clone, PartialEq)]
pub struct WorkspaceSnapshotDiagnostics {
    /// Diagnostics for the wrapped dock snapshot.
    pub dock: DockSnapshotDiagnostics,
    /// Diagnostics for the workspace panel instance shell.
    pub workspace: Vec<WorkspaceSnapshotDiagnostic>,
}

impl WorkspaceSnapshotDiagnostics {
    /// Returns true when no error diagnostics were emitted.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        !self.has_errors()
    }

    /// Returns true when at least one error diagnostic was emitted.
    #[must_use]
    pub fn has_errors(&self) -> bool {
        self.dock.has_errors()
            || self
                .workspace
                .iter()
                .any(|diagnostic| diagnostic.severity == SnapshotDiagnosticSeverity::Error)
    }
}

/// Stable repair action code for workspace snapshot repair planning.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceRepairActionCode {
    /// A missing panel instance record was filled with placeholder metadata.
    AddMissingPanelInstancePlaceholder,
    /// A panel instance record not referenced by the dock snapshot was dropped.
    DropStalePanelInstance,
    /// An unknown panel type was preserved as explicit unresolved metadata.
    KeepUnknownPanelType,
}

impl WorkspaceRepairActionCode {
    /// Returns the stable string code for this repair action.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AddMissingPanelInstancePlaceholder => {
                "workspace_repair.add_missing_panel_instance_placeholder"
            }
            Self::DropStalePanelInstance => "workspace_repair.drop_stale_panel_instance",
            Self::KeepUnknownPanelType => "workspace_repair.keep_unknown_panel_type",
        }
    }
}

/// Structured metadata-only action emitted by workspace repair planning.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceRepairAction {
    /// Stable repair action code.
    pub code: WorkspaceRepairActionCode,
    /// Panel instance identity affected by this action.
    pub panel_instance: Option<PanelInstanceId>,
    /// Panel type identity affected by this action.
    pub panel_type: Option<PanelTypeId>,
    /// Dock frame containing the panel instance when known.
    pub frame: Option<FrameId>,
    /// Legacy dock panel identity when known.
    pub panel: Option<PanelId>,
    /// Title stored on the dock panel when relevant.
    pub dock_title: Option<String>,
    /// Title stored on the panel instance when relevant.
    pub instance_title: Option<String>,
}

impl WorkspaceRepairAction {
    fn new(code: WorkspaceRepairActionCode) -> Self {
        Self {
            code,
            panel_instance: None,
            panel_type: None,
            frame: None,
            panel: None,
            dock_title: None,
            instance_title: None,
        }
    }

    /// Returns the stable string code for this repair action.
    #[must_use]
    pub const fn stable_code(&self) -> &'static str {
        self.code.as_str()
    }
}

/// Deterministic report for explicit workspace snapshot repair planning.
#[derive(Debug, Clone, PartialEq)]
pub struct WorkspaceRepairPlan {
    /// Strict validation diagnostics collected before repair planning.
    pub diagnostics: WorkspaceSnapshotDiagnostics,
    /// Metadata-only repair actions the plan would apply.
    pub actions: Vec<WorkspaceRepairAction>,
    outcome: WorkspaceRepairPlanOutcome,
}

#[derive(Debug, Clone, PartialEq)]
enum WorkspaceRepairPlanOutcome {
    Repaired(WorkspaceSnapshot),
    HardError(WorkspaceRestoreError),
}

impl WorkspaceRepairPlan {
    fn repaired(
        diagnostics: WorkspaceSnapshotDiagnostics,
        actions: Vec<WorkspaceRepairAction>,
        snapshot: WorkspaceSnapshot,
    ) -> Self {
        Self {
            diagnostics,
            actions,
            outcome: WorkspaceRepairPlanOutcome::Repaired(snapshot),
        }
    }

    fn with_hard_error(
        diagnostics: WorkspaceSnapshotDiagnostics,
        error: WorkspaceRestoreError,
    ) -> Self {
        Self {
            diagnostics,
            actions: Vec::new(),
            outcome: WorkspaceRepairPlanOutcome::HardError(error),
        }
    }

    /// Returns true when this plan can produce a repaired snapshot.
    #[must_use]
    pub const fn is_repairable(&self) -> bool {
        matches!(self.outcome, WorkspaceRepairPlanOutcome::Repaired(_))
    }

    /// Returns true when repair planning found a hard error.
    #[must_use]
    pub const fn has_hard_error(&self) -> bool {
        matches!(self.outcome, WorkspaceRepairPlanOutcome::HardError(_))
    }

    /// Returns the repaired workspace snapshot when planning found no hard
    /// repair error.
    #[must_use]
    pub const fn repaired_snapshot(&self) -> Option<&WorkspaceSnapshot> {
        match &self.outcome {
            WorkspaceRepairPlanOutcome::Repaired(snapshot) => Some(snapshot),
            WorkspaceRepairPlanOutcome::HardError(_) => None,
        }
    }

    /// Returns the hard repair error when planning could not safely produce a
    /// repaired snapshot.
    #[must_use]
    pub const fn hard_error(&self) -> Option<&WorkspaceRestoreError> {
        match &self.outcome {
            WorkspaceRepairPlanOutcome::Repaired(_) => None,
            WorkspaceRepairPlanOutcome::HardError(error) => Some(error),
        }
    }

    /// Consumes the plan and returns the repaired snapshot.
    ///
    /// # Errors
    ///
    /// Returns the hard repair error when planning could not safely produce a
    /// repaired snapshot.
    pub fn into_repaired_snapshot(self) -> Result<WorkspaceSnapshot, WorkspaceRestoreError> {
        match self.outcome {
            WorkspaceRepairPlanOutcome::Repaired(snapshot) => Ok(snapshot),
            WorkspaceRepairPlanOutcome::HardError(error) => Err(error),
        }
    }
}

impl From<DockRestoreError> for WorkspaceRestoreError {
    fn from(value: DockRestoreError) -> Self {
        Self::Dock(value)
    }
}

fn snapshot_node(node: &DockNode) -> DockSnapshotNode {
    match node {
        DockNode::Frame(frame) => DockSnapshotNode::Frame {
            id: frame.id,
            panels: frame.panels.clone(),
            active: frame.active,
            dismissible_panels: frame.dismissible_panels.iter().copied().collect(),
        },
        DockNode::Split {
            axis,
            ratio,
            min_first,
            min_second,
            first,
            second,
        } => DockSnapshotNode::Split {
            axis: *axis,
            ratio: *ratio,
            min_first: *min_first,
            min_second: *min_second,
            first: Box::new(snapshot_node(first)),
            second: Box::new(snapshot_node(second)),
        },
    }
}

fn restore_node(snapshot: DockSnapshotNode) -> DockNode {
    match snapshot {
        DockSnapshotNode::Frame {
            id,
            panels,
            active,
            dismissible_panels,
        } => DockNode::Frame(Frame {
            id,
            panels,
            active,
            dismissible_panels: dismissible_panels.into_iter().collect(),
        }),
        DockSnapshotNode::Split {
            axis,
            ratio,
            min_first,
            min_second,
            first,
            second,
        } => DockNode::Split {
            axis,
            ratio,
            min_first,
            min_second,
            first: Box::new(restore_node(*first)),
            second: Box::new(restore_node(*second)),
        },
    }
}

#[derive(Default)]
struct DockSnapshotValidation {
    frame_ids: BTreeSet<FrameId>,
    panel_ids: BTreeSet<PanelId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DockPanelReference {
    panel: PanelId,
    frame: FrameId,
    title: String,
}

#[derive(Default)]
struct DockSnapshotDiagnosticState {
    frame_ids: BTreeSet<FrameId>,
    panel_ids: BTreeSet<PanelId>,
    panel_references: BTreeMap<PanelId, DockPanelReference>,
    diagnostics: Vec<DockSnapshotDiagnostic>,
}

/// Returns structured diagnostics for a dock snapshot.
#[must_use]
pub fn validate_dock_snapshot_diagnostics(snapshot: &DockSnapshot) -> DockSnapshotDiagnostics {
    let mut state = DockSnapshotDiagnosticState::default();
    collect_dock_snapshot_diagnostics(&snapshot.root, &DockSplitPath::root(), &mut state);
    if let Some(active_frame) = snapshot.active_frame
        && !state.frame_ids.contains(&active_frame)
    {
        let mut diagnostic = DockSnapshotDiagnostic::new(
            DockSnapshotDiagnosticCode::InvalidActiveFrame,
            DockSplitPath::root(),
        );
        diagnostic.frame = Some(active_frame);
        state.diagnostics.push(diagnostic);
    }
    DockSnapshotDiagnostics {
        diagnostics: state.diagnostics,
    }
}

/// Returns structured diagnostics for a workspace snapshot.
#[must_use]
pub fn validate_workspace_snapshot_diagnostics(
    snapshot: &WorkspaceSnapshot,
    descriptors: &[PanelTypeDescriptor],
) -> WorkspaceSnapshotDiagnostics {
    let dock = validate_dock_snapshot_diagnostics(&snapshot.dock);
    let dock_references = collect_dock_panel_references(&snapshot.dock.root);
    let workspace = collect_workspace_snapshot_diagnostics(snapshot, descriptors, &dock_references);
    WorkspaceSnapshotDiagnostics { dock, workspace }
}

fn plan_workspace_snapshot_repair(
    snapshot: &WorkspaceSnapshot,
    descriptors: &[PanelTypeDescriptor],
) -> WorkspaceRepairPlan {
    let diagnostics = validate_workspace_snapshot_diagnostics(snapshot, descriptors);
    if let Some(error) = workspace_repair_hard_error(snapshot, descriptors, &diagnostics) {
        return WorkspaceRepairPlan::with_hard_error(diagnostics, error);
    }

    let mut actions = Vec::new();
    let stale_panel_instances = collect_stale_panel_instances(&diagnostics);
    let mut repaired = WorkspaceSnapshot::new(
        snapshot.dock.clone(),
        snapshot
            .panel_instances
            .iter()
            .filter(|instance| !stale_panel_instances.contains(&instance.id))
            .cloned()
            .collect(),
    );

    for diagnostic in &diagnostics.workspace {
        match diagnostic.code {
            WorkspaceSnapshotDiagnosticCode::MissingPanelInstance => {
                if let Some(placeholder) =
                    placeholder_panel_instance_from_diagnostic(diagnostic, descriptors)
                {
                    let mut action = WorkspaceRepairAction::new(
                        WorkspaceRepairActionCode::AddMissingPanelInstancePlaceholder,
                    );
                    action.panel_instance = Some(placeholder.id);
                    action.panel_type = Some(placeholder.panel_type);
                    action.frame = diagnostic.frame;
                    action.panel = diagnostic.panel;
                    action.dock_title.clone_from(&diagnostic.dock_title);
                    repaired.panel_instances.push(placeholder);
                    actions.push(action);
                }
            }
            WorkspaceSnapshotDiagnosticCode::StalePanelInstance => {
                let mut action =
                    WorkspaceRepairAction::new(WorkspaceRepairActionCode::DropStalePanelInstance);
                action.panel_instance = diagnostic.panel_instance;
                action.panel_type = diagnostic.panel_type;
                action.instance_title.clone_from(&diagnostic.instance_title);
                actions.push(action);
            }
            WorkspaceSnapshotDiagnosticCode::UnknownPanelType => {
                if diagnostic
                    .panel_instance
                    .is_some_and(|panel_instance| stale_panel_instances.contains(&panel_instance))
                {
                    continue;
                }
                let mut action =
                    WorkspaceRepairAction::new(WorkspaceRepairActionCode::KeepUnknownPanelType);
                action.panel_instance = diagnostic.panel_instance;
                action.panel_type = diagnostic.panel_type;
                actions.push(action);
            }
            WorkspaceSnapshotDiagnosticCode::DuplicatePanelInstanceId
            | WorkspaceSnapshotDiagnosticCode::DuplicatePanelTypeDescriptor
            | WorkspaceSnapshotDiagnosticCode::PanelTitleDrift => {}
        }
    }

    WorkspaceRepairPlan::repaired(diagnostics, actions, repaired)
}

fn workspace_repair_hard_error(
    snapshot: &WorkspaceSnapshot,
    descriptors: &[PanelTypeDescriptor],
    diagnostics: &WorkspaceSnapshotDiagnostics,
) -> Option<WorkspaceRestoreError> {
    if let Err(error) = validate_dock_snapshot(&snapshot.dock) {
        return Some(WorkspaceRestoreError::Dock(error));
    }

    for diagnostic in &diagnostics.workspace {
        match diagnostic.code {
            WorkspaceSnapshotDiagnosticCode::DuplicatePanelTypeDescriptor => {
                return diagnostic.panel_type.map(|panel_type| {
                    WorkspaceRestoreError::DuplicatePanelTypeDescriptor { panel_type }
                });
            }
            WorkspaceSnapshotDiagnosticCode::DuplicatePanelInstanceId => {
                return diagnostic.panel_instance.map(|panel_instance| {
                    WorkspaceRestoreError::DuplicatePanelInstanceId { panel_instance }
                });
            }
            WorkspaceSnapshotDiagnosticCode::MissingPanelInstance => {
                if placeholder_panel_instance_from_diagnostic(diagnostic, descriptors).is_none() {
                    return diagnostic.panel_instance.map(|panel_instance| {
                        WorkspaceRestoreError::MissingPanelInstance { panel_instance }
                    });
                }
            }
            WorkspaceSnapshotDiagnosticCode::StalePanelInstance
            | WorkspaceSnapshotDiagnosticCode::UnknownPanelType
            | WorkspaceSnapshotDiagnosticCode::PanelTitleDrift => {}
        }
    }

    None
}

fn collect_stale_panel_instances(
    diagnostics: &WorkspaceSnapshotDiagnostics,
) -> BTreeSet<PanelInstanceId> {
    diagnostics
        .workspace
        .iter()
        .filter_map(|diagnostic| {
            (diagnostic.code == WorkspaceSnapshotDiagnosticCode::StalePanelInstance)
                .then_some(diagnostic.panel_instance)
                .flatten()
        })
        .collect()
}

fn placeholder_panel_instance_from_diagnostic(
    diagnostic: &WorkspaceSnapshotDiagnostic,
    descriptors: &[PanelTypeDescriptor],
) -> Option<PanelInstanceSnapshot> {
    let panel_instance = diagnostic.panel_instance?;
    let dock_title = diagnostic.dock_title.as_ref()?;
    let panel_type = unique_panel_type_for_title(dock_title, descriptors)?;
    Some(PanelInstanceSnapshot::new(
        panel_instance,
        panel_type,
        dock_title.clone(),
    ))
}

fn unique_panel_type_for_title(
    title: &str,
    descriptors: &[PanelTypeDescriptor],
) -> Option<PanelTypeId> {
    let mut matches = descriptors
        .iter()
        .filter(|descriptor| descriptor.title == title)
        .map(|descriptor| descriptor.id);
    let panel_type = matches.next()?;
    matches.next().is_none().then_some(panel_type)
}

fn validate_dock_snapshot(
    snapshot: &DockSnapshot,
) -> Result<DockSnapshotValidation, DockRestoreError> {
    let mut validation = DockSnapshotValidation::default();
    validate_snapshot_node(&snapshot.root, &mut validation)?;
    if let Some(active_frame) = snapshot.active_frame
        && !validation.frame_ids.contains(&active_frame)
    {
        return Err(DockRestoreError::InvalidActiveFrame);
    }
    Ok(validation)
}

fn collect_workspace_snapshot_diagnostics(
    snapshot: &WorkspaceSnapshot,
    descriptors: &[PanelTypeDescriptor],
    dock_references: &BTreeMap<PanelId, DockPanelReference>,
) -> Vec<WorkspaceSnapshotDiagnostic> {
    let mut diagnostics = Vec::new();
    let mut panel_types = BTreeSet::new();
    for descriptor in descriptors {
        if !panel_types.insert(descriptor.id) {
            let mut diagnostic = WorkspaceSnapshotDiagnostic::new(
                WorkspaceSnapshotDiagnosticCode::DuplicatePanelTypeDescriptor,
            );
            diagnostic.panel_type = Some(descriptor.id);
            diagnostics.push(diagnostic);
        }
    }

    let mut snapshot_panel_instances = BTreeMap::new();
    for instance in &snapshot.panel_instances {
        match snapshot_panel_instances.entry(instance.id) {
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(instance);
            }
            std::collections::btree_map::Entry::Occupied(_) => {
                let mut diagnostic = WorkspaceSnapshotDiagnostic::new(
                    WorkspaceSnapshotDiagnosticCode::DuplicatePanelInstanceId,
                );
                diagnostic.panel_instance = Some(instance.id);
                diagnostic.panel_type = Some(instance.panel_type);
                diagnostics.push(diagnostic);
            }
        }
    }

    for (panel_instance, instance) in &snapshot_panel_instances {
        if !panel_types.contains(&instance.panel_type) {
            let mut diagnostic =
                WorkspaceSnapshotDiagnostic::new(WorkspaceSnapshotDiagnosticCode::UnknownPanelType);
            diagnostic.panel_instance = Some(*panel_instance);
            diagnostic.panel_type = Some(instance.panel_type);
            diagnostics.push(diagnostic);
        }
    }

    let dock_panel_instances: BTreeMap<_, _> = dock_references
        .iter()
        .map(|(panel, reference)| (panel.instance_id(), reference))
        .collect();
    for (panel_instance, reference) in &dock_panel_instances {
        if !snapshot_panel_instances.contains_key(panel_instance) {
            let mut diagnostic = WorkspaceSnapshotDiagnostic::new(
                WorkspaceSnapshotDiagnosticCode::MissingPanelInstance,
            );
            diagnostic.panel_instance = Some(*panel_instance);
            diagnostic.frame = Some(reference.frame);
            diagnostic.panel = Some(reference.panel);
            diagnostic.dock_title = Some(reference.title.clone());
            diagnostics.push(diagnostic);
        }
    }

    for panel_instance in snapshot_panel_instances.keys() {
        if !dock_panel_instances.contains_key(panel_instance) {
            let mut diagnostic = WorkspaceSnapshotDiagnostic::new(
                WorkspaceSnapshotDiagnosticCode::StalePanelInstance,
            );
            diagnostic.panel_instance = Some(*panel_instance);
            if let Some(instance) = snapshot_panel_instances.get(panel_instance) {
                diagnostic.panel_type = Some(instance.panel_type);
                diagnostic.instance_title = Some(instance.title.clone());
            }
            diagnostics.push(diagnostic);
        }
    }

    for (panel_instance, reference) in &dock_panel_instances {
        let Some(instance) = snapshot_panel_instances.get(panel_instance) else {
            continue;
        };
        if reference.title != instance.title {
            let mut diagnostic =
                WorkspaceSnapshotDiagnostic::new(WorkspaceSnapshotDiagnosticCode::PanelTitleDrift);
            diagnostic.severity = SnapshotDiagnosticSeverity::Warning;
            diagnostic.panel_instance = Some(*panel_instance);
            diagnostic.panel_type = Some(instance.panel_type);
            diagnostic.frame = Some(reference.frame);
            diagnostic.panel = Some(reference.panel);
            diagnostic.dock_title = Some(reference.title.clone());
            diagnostic.instance_title = Some(instance.title.clone());
            diagnostics.push(diagnostic);
        }
    }

    diagnostics
}

fn validate_workspace_snapshot(
    snapshot: &WorkspaceSnapshot,
    descriptors: &[PanelTypeDescriptor],
    dock_validation: &DockSnapshotValidation,
) -> Result<(), WorkspaceRestoreError> {
    let mut panel_types = BTreeSet::new();
    for descriptor in descriptors {
        if !panel_types.insert(descriptor.id) {
            return Err(WorkspaceRestoreError::DuplicatePanelTypeDescriptor {
                panel_type: descriptor.id,
            });
        }
    }

    let dock_panel_instances: BTreeSet<_> = dock_validation
        .panel_ids
        .iter()
        .map(|panel| panel.instance_id())
        .collect();
    let mut snapshot_panel_instances = BTreeMap::new();

    for instance in &snapshot.panel_instances {
        if snapshot_panel_instances
            .insert(instance.id, instance.panel_type)
            .is_some()
        {
            return Err(WorkspaceRestoreError::DuplicatePanelInstanceId {
                panel_instance: instance.id,
            });
        }
    }

    for (panel_instance, panel_type) in &snapshot_panel_instances {
        if !panel_types.contains(panel_type) {
            return Err(WorkspaceRestoreError::UnknownPanelType {
                panel_instance: *panel_instance,
                panel_type: *panel_type,
            });
        }
    }

    for panel_instance in &dock_panel_instances {
        if !snapshot_panel_instances.contains_key(panel_instance) {
            return Err(WorkspaceRestoreError::MissingPanelInstance {
                panel_instance: *panel_instance,
            });
        }
    }

    for panel_instance in snapshot_panel_instances.keys() {
        if !dock_panel_instances.contains(panel_instance) {
            return Err(WorkspaceRestoreError::StalePanelInstance {
                panel_instance: *panel_instance,
            });
        }
    }

    Ok(())
}

fn collect_dock_panel_references(
    snapshot: &DockSnapshotNode,
) -> BTreeMap<PanelId, DockPanelReference> {
    let mut state = DockSnapshotDiagnosticState::default();
    collect_dock_snapshot_diagnostics(snapshot, &DockSplitPath::root(), &mut state);
    state.panel_references
}

fn collect_dock_snapshot_diagnostics(
    snapshot: &DockSnapshotNode,
    path: &DockSplitPath,
    state: &mut DockSnapshotDiagnosticState,
) {
    match snapshot {
        DockSnapshotNode::Frame {
            id,
            panels,
            active,
            dismissible_panels,
        } => collect_frame_snapshot_diagnostics(
            *id,
            panels,
            *active,
            dismissible_panels,
            path,
            state,
        ),
        DockSnapshotNode::Split {
            ratio,
            min_first,
            min_second,
            first,
            second,
            ..
        } => collect_split_snapshot_diagnostics(
            *ratio,
            *min_first,
            *min_second,
            first,
            second,
            path,
            state,
        ),
    }
}

fn collect_frame_snapshot_diagnostics(
    id: FrameId,
    panels: &[Panel],
    active: usize,
    dismissible_panels: &[PanelId],
    path: &DockSplitPath,
    state: &mut DockSnapshotDiagnosticState,
) {
    if !state.frame_ids.insert(id) {
        let mut diagnostic =
            DockSnapshotDiagnostic::new(DockSnapshotDiagnosticCode::DuplicateFrameId, path.clone());
        diagnostic.frame = Some(id);
        state.diagnostics.push(diagnostic);
    }
    if panels.is_empty() {
        let mut diagnostic =
            DockSnapshotDiagnostic::new(DockSnapshotDiagnosticCode::EmptyFrame, path.clone());
        diagnostic.frame = Some(id);
        state.diagnostics.push(diagnostic);
    }
    if active >= panels.len() {
        let mut diagnostic = DockSnapshotDiagnostic::new(
            DockSnapshotDiagnosticCode::InvalidActivePanelIndex,
            path.clone(),
        );
        diagnostic.frame = Some(id);
        diagnostic.active_index = Some(active);
        diagnostic.panel_count = Some(panels.len());
        state.diagnostics.push(diagnostic);
    }

    let frame_panel_ids = collect_frame_panel_diagnostics(id, panels, path, state);
    collect_frame_dismissible_diagnostics(id, dismissible_panels, &frame_panel_ids, path, state);
}

fn collect_frame_panel_diagnostics(
    frame: FrameId,
    panels: &[Panel],
    path: &DockSplitPath,
    state: &mut DockSnapshotDiagnosticState,
) -> BTreeSet<PanelId> {
    let mut frame_panel_ids = BTreeSet::new();
    for panel in panels {
        if !frame_panel_ids.insert(panel.id) || !state.panel_ids.insert(panel.id) {
            let mut diagnostic = DockSnapshotDiagnostic::new(
                DockSnapshotDiagnosticCode::DuplicatePanelId,
                path.clone(),
            );
            diagnostic.frame = Some(frame);
            diagnostic.panel = Some(panel.id);
            state.diagnostics.push(diagnostic);
        }
        state
            .panel_references
            .entry(panel.id)
            .or_insert_with(|| DockPanelReference {
                panel: panel.id,
                frame,
                title: panel.title.clone(),
            });
    }
    frame_panel_ids
}

fn collect_frame_dismissible_diagnostics(
    frame: FrameId,
    dismissible_panels: &[PanelId],
    frame_panel_ids: &BTreeSet<PanelId>,
    path: &DockSplitPath,
    state: &mut DockSnapshotDiagnosticState,
) {
    let mut frame_dismissible_ids = BTreeSet::new();
    for panel in dismissible_panels {
        if !frame_dismissible_ids.insert(*panel) {
            let mut diagnostic = DockSnapshotDiagnostic::new(
                DockSnapshotDiagnosticCode::DuplicateDismissiblePolicy,
                path.clone(),
            );
            diagnostic.frame = Some(frame);
            diagnostic.panel = Some(*panel);
            state.diagnostics.push(diagnostic);
        }
        if !frame_panel_ids.contains(panel) {
            let mut diagnostic = DockSnapshotDiagnostic::new(
                DockSnapshotDiagnosticCode::InvalidDismissiblePanel,
                path.clone(),
            );
            diagnostic.frame = Some(frame);
            diagnostic.panel = Some(*panel);
            state.diagnostics.push(diagnostic);
        }
    }
}

fn collect_split_snapshot_diagnostics(
    ratio: f32,
    min_first: f32,
    min_second: f32,
    first: &DockSnapshotNode,
    second: &DockSnapshotNode,
    path: &DockSplitPath,
    state: &mut DockSnapshotDiagnosticState,
) {
    if !ratio.is_finite() || !(0.0..=1.0).contains(&ratio) {
        let mut diagnostic = DockSnapshotDiagnostic::new(
            DockSnapshotDiagnosticCode::InvalidSplitRatio,
            path.clone(),
        );
        diagnostic.split_value = Some(DockSnapshotSplitValue::Ratio);
        state.diagnostics.push(diagnostic);
    }
    if !min_first.is_finite() || min_first < 0.0 {
        let mut diagnostic = DockSnapshotDiagnostic::new(
            DockSnapshotDiagnosticCode::InvalidSplitMinimum,
            path.clone(),
        );
        diagnostic.split_value = Some(DockSnapshotSplitValue::MinFirst);
        state.diagnostics.push(diagnostic);
    }
    if !min_second.is_finite() || min_second < 0.0 {
        let mut diagnostic = DockSnapshotDiagnostic::new(
            DockSnapshotDiagnosticCode::InvalidSplitMinimum,
            path.clone(),
        );
        diagnostic.split_value = Some(DockSnapshotSplitValue::MinSecond);
        state.diagnostics.push(diagnostic);
    }
    collect_dock_snapshot_diagnostics(first, &path.child(DockPathElement::First), state);
    collect_dock_snapshot_diagnostics(second, &path.child(DockPathElement::Second), state);
}

fn validate_snapshot_node(
    snapshot: &DockSnapshotNode,
    validation: &mut DockSnapshotValidation,
) -> Result<(), DockRestoreError> {
    match snapshot {
        DockSnapshotNode::Frame {
            id,
            panels,
            active,
            dismissible_panels,
        } => {
            if !validation.frame_ids.insert(*id) {
                return Err(DockRestoreError::DuplicateFrameId);
            }
            if panels.is_empty() {
                return Err(DockRestoreError::EmptyFrame);
            }
            if *active >= panels.len() {
                return Err(DockRestoreError::InvalidActiveIndex);
            }

            let mut frame_panel_ids = BTreeSet::new();
            for panel in panels {
                if !frame_panel_ids.insert(panel.id) || !validation.panel_ids.insert(panel.id) {
                    return Err(DockRestoreError::DuplicatePanelId);
                }
            }

            let mut frame_dismissible_ids = BTreeSet::new();
            for id in dismissible_panels {
                if !frame_dismissible_ids.insert(*id) {
                    return Err(DockRestoreError::DuplicateDismissiblePanel);
                }
                if !frame_panel_ids.contains(id) {
                    return Err(DockRestoreError::InvalidDismissiblePanel);
                }
            }
            Ok(())
        }
        DockSnapshotNode::Split {
            ratio,
            min_first,
            min_second,
            first,
            second,
            ..
        } => {
            if !ratio.is_finite() || !(0.0..=1.0).contains(ratio) {
                return Err(DockRestoreError::InvalidSplitRatio);
            }
            if !min_first.is_finite()
                || !min_second.is_finite()
                || *min_first < 0.0
                || *min_second < 0.0
            {
                return Err(DockRestoreError::InvalidSplitMinimum);
            }
            validate_snapshot_node(first, validation)?;
            validate_snapshot_node(second, validation)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Dock, DockDropTarget, DockDropZone, DockNode, DockPathElement, DockPlacement,
        DockRestoreError, DockSnapshot, DockSnapshotNode, DockSplitInsertion, DockSplitPath, Frame,
        FrameId, FrameLayout, Panel, PanelId, frame_tabs, resolve_dock_drop_target,
        resolve_frame_drop_zone, solve_dock_layout, solve_dock_splitters, split_ratio_from_drag,
    };
    use kinetik_ui_core::{Axis, Point, Rect, SemanticRole, Vec2};

    fn panel(id: u64, title: &str) -> Panel {
        Panel::new(PanelId::from_raw(id), title)
    }

    fn frame(id: u64, panels: Vec<Panel>) -> Frame {
        Frame::new(FrameId::from_raw(id), panels)
    }

    fn dock() -> Dock {
        Dock::new(DockNode::Split {
            axis: Axis::Horizontal,
            ratio: 0.25,
            min_first: 100.0,
            min_second: 100.0,
            first: Box::new(DockNode::Frame(frame(1, vec![panel(1, "Media")]))),
            second: Box::new(DockNode::Frame(frame(
                2,
                vec![panel(2, "Viewport"), panel(3, "Timeline")],
            ))),
        })
    }

    #[test]
    fn dock_tree_visits_frames_in_order() {
        let area = dock();
        let frames = area.frames();

        assert_eq!(frames[0].id, FrameId::from_raw(1));
        assert_eq!(frames[1].id, FrameId::from_raw(2));
    }

    #[test]
    fn dock_initializes_active_frame_from_tree_order() {
        let area = dock();

        assert_eq!(area.active_frame(), Some(FrameId::from_raw(1)));
    }

    #[test]
    fn set_active_frame_requires_existing_frame() {
        let mut area = dock();

        assert!(area.set_active_frame(FrameId::from_raw(2)));
        assert_eq!(area.active_frame(), Some(FrameId::from_raw(2)));
        assert!(!area.set_active_frame(FrameId::from_raw(99)));
        assert_eq!(area.active_frame(), Some(FrameId::from_raw(2)));
    }

    #[test]
    fn selects_and_removes_frame_tabs() {
        let mut frame = frame(1, vec![panel(1, "A"), panel(2, "B")]);

        assert!(frame.select_panel(PanelId::from_raw(2)));
        assert_eq!(
            frame.active_panel().expect("active").id,
            PanelId::from_raw(2)
        );
        assert_eq!(
            frame
                .remove_panel(PanelId::from_raw(2))
                .expect("removed")
                .title,
            "B"
        );
        assert_eq!(frame.active, 0);
    }

    #[test]
    fn moves_panels_between_frames() {
        let mut area = dock();

        assert!(area.move_panel(
            FrameId::from_raw(2),
            FrameId::from_raw(1),
            PanelId::from_raw(3)
        ));

        assert_eq!(
            area.frame_mut(FrameId::from_raw(1))
                .expect("frame")
                .panels
                .len(),
            2
        );
        assert_eq!(area.active_frame(), Some(FrameId::from_raw(1)));
    }

    #[test]
    fn moving_panels_preserves_frame_owned_dismissal_policy() {
        let mut area = dock();
        area.frame_mut(FrameId::from_raw(2))
            .expect("source")
            .set_panel_dismissible(PanelId::from_raw(3), false);

        assert!(area.move_panel(
            FrameId::from_raw(2),
            FrameId::from_raw(1),
            PanelId::from_raw(3)
        ));

        assert!(
            !area
                .frame(FrameId::from_raw(1))
                .expect("target")
                .panel_dismissible(PanelId::from_raw(3))
        );
    }

    #[test]
    fn moving_panel_to_missing_target_does_not_remove_it() {
        let mut area = dock();

        assert!(!area.move_panel(
            FrameId::from_raw(2),
            FrameId::from_raw(99),
            PanelId::from_raw(3)
        ));

        let source = area.frame(FrameId::from_raw(2)).expect("source");
        assert_eq!(source.panels.len(), 2);
        assert!(
            source
                .panels
                .iter()
                .any(|panel| panel.id == PanelId::from_raw(3))
        );
    }

    #[test]
    fn moving_last_panel_prunes_empty_source_frame() {
        let mut area = dock();
        assert!(area.set_active_frame(FrameId::from_raw(1)));

        assert!(area.move_panel(
            FrameId::from_raw(1),
            FrameId::from_raw(2),
            PanelId::from_raw(1)
        ));

        assert_eq!(area.frames().len(), 1);
        assert!(area.frame(FrameId::from_raw(1)).is_none());
        assert_eq!(
            area.frame(FrameId::from_raw(2))
                .expect("target")
                .panels
                .len(),
            3
        );
        assert_eq!(area.active_frame(), Some(FrameId::from_raw(2)));
    }

    #[test]
    fn merges_frames_into_target() {
        let mut area = dock();
        assert!(area.set_active_frame(FrameId::from_raw(1)));

        assert!(area.merge_frames(FrameId::from_raw(1), FrameId::from_raw(2)));

        assert_eq!(
            area.frame_mut(FrameId::from_raw(2))
                .expect("target")
                .panels
                .len(),
            3
        );
        assert_eq!(area.frames().len(), 1);
        assert!(area.frame(FrameId::from_raw(1)).is_none());
        assert_eq!(area.active_frame(), Some(FrameId::from_raw(2)));
    }

    #[test]
    fn merging_missing_target_does_not_remove_source_panels() {
        let mut area = dock();

        assert!(!area.merge_frames(FrameId::from_raw(1), FrameId::from_raw(99)));

        assert_eq!(
            area.frame(FrameId::from_raw(1))
                .expect("source")
                .panels
                .len(),
            1
        );
    }

    #[test]
    fn selecting_panel_updates_active_frame() {
        let mut area = dock();

        assert!(area.select_panel(FrameId::from_raw(2), PanelId::from_raw(3)));

        assert_eq!(area.active_frame(), Some(FrameId::from_raw(2)));
        assert_eq!(
            area.frame(FrameId::from_raw(2))
                .expect("frame")
                .active_panel()
                .expect("active")
                .id,
            PanelId::from_raw(3)
        );
    }

    #[test]
    fn solves_horizontal_split_layout() {
        let area = dock();
        let layout = solve_dock_layout(&area, Rect::new(0.0, 0.0, 1000.0, 500.0));

        assert_eq!(layout.len(), 2);
        assert!((layout[0].rect.width - 250.0).abs() < f32::EPSILON);
        assert!((layout[1].rect.x - 250.0).abs() < f32::EPSILON);
    }

    #[test]
    fn split_layout_respects_minimums() {
        let area = Dock::new(DockNode::Split {
            axis: Axis::Horizontal,
            ratio: 0.05,
            min_first: 100.0,
            min_second: 100.0,
            first: Box::new(DockNode::Frame(frame(1, vec![panel(1, "A")]))),
            second: Box::new(DockNode::Frame(frame(2, vec![panel(2, "B")]))),
        });
        let layout = solve_dock_layout(&area, Rect::new(0.0, 0.0, 500.0, 200.0));

        assert!((layout[0].rect.width - 100.0).abs() < f32::EPSILON);
    }

    #[test]
    fn split_layout_never_emits_negative_sizes_when_minimums_exceed_bounds() {
        let area = Dock::new(DockNode::Split {
            axis: Axis::Horizontal,
            ratio: 0.5,
            min_first: 100.0,
            min_second: 100.0,
            first: Box::new(DockNode::Frame(frame(1, vec![panel(1, "A")]))),
            second: Box::new(DockNode::Frame(frame(2, vec![panel(2, "B")]))),
        });
        let layout = solve_dock_layout(&area, Rect::new(0.0, 0.0, 120.0, 200.0));

        assert_eq!(layout.len(), 2);
        assert!(layout[0].rect.width >= 0.0);
        assert!(layout[1].rect.width >= 0.0);
        assert!((layout[0].rect.width + layout[1].rect.width - 120.0).abs() < f32::EPSILON);
    }

    #[test]
    fn split_layout_sanitizes_direct_non_finite_values() {
        let area = Dock::new(DockNode::Split {
            axis: Axis::Horizontal,
            ratio: f32::NAN,
            min_first: f32::INFINITY,
            min_second: -100.0,
            first: Box::new(DockNode::Frame(frame(1, vec![panel(1, "A")]))),
            second: Box::new(DockNode::Frame(frame(2, vec![panel(2, "B")]))),
        });
        let layout = solve_dock_layout(&area, Rect::new(0.0, 0.0, 120.0, 200.0));

        assert_eq!(layout.len(), 2);
        assert!(layout[0].rect.width.is_finite());
        assert!(layout[1].rect.width.is_finite());
        assert!((layout[0].rect.width + layout[1].rect.width - 120.0).abs() < f32::EPSILON);
    }

    #[test]
    fn splitter_drag_maps_delta_to_clamped_ratio() {
        let bounds = Rect::new(0.0, 0.0, 500.0, 200.0);

        let ratio = split_ratio_from_drag(
            Axis::Horizontal,
            bounds,
            0.5,
            100.0,
            100.0,
            Vec2::new(75.0, 0.0),
        );
        assert!((ratio - 0.65).abs() < f32::EPSILON);

        let clamped = split_ratio_from_drag(
            Axis::Horizontal,
            bounds,
            0.5,
            100.0,
            100.0,
            Vec2::new(-500.0, 0.0),
        );
        assert!((clamped - 0.2).abs() < f32::EPSILON);

        let vertical = split_ratio_from_drag(
            Axis::Vertical,
            bounds,
            0.5,
            50.0,
            50.0,
            Vec2::new(1000.0, 25.0),
        );
        assert!((vertical - 0.625).abs() < f32::EPSILON);
    }

    #[test]
    fn dock_resizes_root_and_nested_splits() {
        let mut area = Dock::new(DockNode::Split {
            axis: Axis::Horizontal,
            ratio: 0.5,
            min_first: 100.0,
            min_second: 100.0,
            first: Box::new(DockNode::Frame(frame(1, vec![panel(1, "A")]))),
            second: Box::new(DockNode::Split {
                axis: Axis::Vertical,
                ratio: 0.5,
                min_first: 50.0,
                min_second: 50.0,
                first: Box::new(DockNode::Frame(frame(2, vec![panel(2, "B")]))),
                second: Box::new(DockNode::Frame(frame(3, vec![panel(3, "C")]))),
            }),
        });
        let bounds = Rect::new(0.0, 0.0, 600.0, 400.0);

        assert!(area.resize_split(&DockSplitPath::root(), bounds, Vec2::new(60.0, 0.0)));
        assert!(area.resize_split(
            &DockSplitPath::new([DockPathElement::Second]),
            bounds,
            Vec2::new(0.0, -75.0)
        ));
        let layout = solve_dock_layout(&area, bounds);

        assert!((layout[0].rect.width - 360.0).abs() < f32::EPSILON);
        assert!((layout[1].rect.height - 125.0).abs() < f32::EPSILON);
    }

    #[test]
    fn dock_splitters_expose_paths_and_hit_rects() {
        let area = Dock::new(DockNode::Split {
            axis: Axis::Horizontal,
            ratio: 0.5,
            min_first: 100.0,
            min_second: 100.0,
            first: Box::new(DockNode::Frame(frame(1, vec![panel(1, "A")]))),
            second: Box::new(DockNode::Split {
                axis: Axis::Vertical,
                ratio: 0.25,
                min_first: 50.0,
                min_second: 50.0,
                first: Box::new(DockNode::Frame(frame(2, vec![panel(2, "B")]))),
                second: Box::new(DockNode::Frame(frame(3, vec![panel(3, "C")]))),
            }),
        });

        let splitters = solve_dock_splitters(&area, Rect::new(0.0, 0.0, 600.0, 400.0), 8.0);

        assert_eq!(splitters.len(), 2);
        assert_eq!(splitters[0].path, DockSplitPath::root());
        assert_eq!(splitters[0].rect, Rect::new(296.0, 0.0, 8.0, 400.0));
        assert_eq!(
            splitters[1].path,
            DockSplitPath::new([DockPathElement::Second])
        );
        assert_eq!(splitters[1].axis, Axis::Vertical);
    }

    #[test]
    fn dock_splitters_sanitize_non_finite_geometry() {
        let area = Dock::new(DockNode::Split {
            axis: Axis::Horizontal,
            ratio: f32::NAN,
            min_first: f32::INFINITY,
            min_second: -100.0,
            first: Box::new(DockNode::Frame(frame(1, vec![panel(1, "A")]))),
            second: Box::new(DockNode::Frame(frame(2, vec![panel(2, "B")]))),
        });

        let splitters = solve_dock_splitters(
            &area,
            Rect::new(f32::NAN, f32::INFINITY, f32::INFINITY, f32::NAN),
            f32::NAN,
        );

        assert_eq!(splitters.len(), 1);
        assert!(splitters[0].rect.x.is_finite());
        assert!(splitters[0].rect.y.is_finite());
        assert!(splitters[0].rect.width.is_finite());
        assert!(splitters[0].rect.height.is_finite());
        assert!(splitters[0].ratio.is_finite());
        assert!(splitters[0].min_first.is_finite());
        assert!(splitters[0].min_second.is_finite());
    }

    #[test]
    fn frame_tabs_expose_presentation_state() {
        let mut frame = frame(1, vec![panel(1, "A"), panel(2, "B")]);
        frame.select_panel(PanelId::from_raw(2));
        assert!(frame.set_panel_dismissible(PanelId::from_raw(1), false));

        let tabs = frame_tabs(&frame);

        assert!(!tabs[0].active);
        assert!(!tabs[0].close_visible);
        assert!(tabs[1].active);
        assert!(tabs[1].close_visible);
        assert!(tabs[1].draggable);
    }

    #[test]
    fn tab_drag_starts_only_for_panels_owned_by_the_frame() {
        let area = dock();

        assert_eq!(
            area.begin_tab_drag(FrameId::from_raw(2), PanelId::from_raw(3)),
            Some(super::DockTabDrag {
                source_frame: FrameId::from_raw(2),
                panel: PanelId::from_raw(3),
            })
        );
        assert_eq!(
            area.begin_tab_drag(FrameId::from_raw(1), PanelId::from_raw(3)),
            None
        );
    }

    #[test]
    fn drop_zone_resolution_prefers_edges_over_center() {
        let rect = Rect::new(10.0, 20.0, 200.0, 100.0);

        assert_eq!(
            resolve_frame_drop_zone(rect, Point::new(12.0, 70.0)),
            Some(DockDropZone::Left)
        );
        assert_eq!(
            resolve_frame_drop_zone(rect, Point::new(205.0, 70.0)),
            Some(DockDropZone::Right)
        );
        assert_eq!(
            resolve_frame_drop_zone(rect, Point::new(100.0, 23.0)),
            Some(DockDropZone::Top)
        );
        assert_eq!(
            resolve_frame_drop_zone(rect, Point::new(100.0, 116.0)),
            Some(DockDropZone::Bottom)
        );
        assert_eq!(
            resolve_frame_drop_zone(rect, Point::new(100.0, 70.0)),
            Some(DockDropZone::Center)
        );
        assert_eq!(resolve_frame_drop_zone(rect, Point::new(0.0, 0.0)), None);
    }

    #[test]
    fn drop_zone_resolution_rejects_invalid_geometry() {
        let rect = Rect::new(10.0, 20.0, 200.0, 100.0);
        for point in [
            Point::new(f32::NAN, 70.0),
            Point::new(f32::INFINITY, 70.0),
            Point::new(100.0, f32::NEG_INFINITY),
        ] {
            assert_eq!(resolve_frame_drop_zone(rect, point), None);
        }

        for rect in [
            Rect::new(f32::NAN, 20.0, 200.0, 100.0),
            Rect::new(10.0, f32::INFINITY, 200.0, 100.0),
            Rect::new(10.0, 20.0, f32::INFINITY, 100.0),
            Rect::new(10.0, 20.0, 200.0, f32::NAN),
            Rect::new(10.0, 20.0, 0.0, 100.0),
            Rect::new(10.0, 20.0, 200.0, -1.0),
        ] {
            assert_eq!(resolve_frame_drop_zone(rect, Point::new(100.0, 70.0)), None);
        }
    }

    #[test]
    fn dock_drop_target_resolution_returns_merge_or_split_targets() {
        let area = dock();
        let layout = solve_dock_layout(&area, Rect::new(0.0, 0.0, 1000.0, 500.0));
        let new_frame = FrameId::from_raw(9);

        assert_eq!(
            resolve_dock_drop_target(&layout, Point::new(500.0, 250.0), new_frame),
            Some(DockDropTarget::tab(FrameId::from_raw(2)))
        );
        assert_eq!(
            resolve_dock_drop_target(&layout, Point::new(995.0, 250.0), new_frame),
            Some(DockDropTarget::split(
                FrameId::from_raw(2),
                DockPlacement::Right,
                new_frame
            ))
        );
    }

    #[test]
    fn dock_drop_target_resolution_rejects_invalid_geometry() {
        let new_frame = FrameId::from_raw(9);
        let invalid_layout = [FrameLayout {
            frame: FrameId::from_raw(1),
            rect: Rect::new(0.0, 0.0, f32::INFINITY, 100.0),
        }];

        assert_eq!(
            resolve_dock_drop_target(&invalid_layout, Point::new(1.0, 50.0), new_frame),
            None
        );
        assert_eq!(
            resolve_dock_drop_target(
                &[FrameLayout {
                    frame: FrameId::from_raw(1),
                    rect: Rect::new(0.0, 0.0, 100.0, 100.0),
                }],
                Point::new(f32::NAN, 50.0),
                new_frame
            ),
            None
        );

        let mixed_layout = [
            FrameLayout {
                frame: FrameId::from_raw(1),
                rect: Rect::new(0.0, 0.0, f32::INFINITY, 100.0),
            },
            FrameLayout {
                frame: FrameId::from_raw(2),
                rect: Rect::new(100.0, 0.0, 100.0, 100.0),
            },
        ];

        let target = resolve_dock_drop_target(&mixed_layout, Point::new(198.0, 50.0), new_frame)
            .expect("target");
        match target {
            DockDropTarget::Split {
                frame,
                placement,
                new_frame: inserted_frame,
                ratio,
                min_first,
                min_second,
            } => {
                assert_eq!(frame, FrameId::from_raw(2));
                assert_eq!(placement, DockPlacement::Right);
                assert_eq!(inserted_frame, new_frame);
                assert!(ratio.is_finite());
                assert!(min_first.is_finite());
                assert!(min_second.is_finite());
            }
            DockDropTarget::Tab { .. } => panic!("expected split target"),
        }
    }

    #[test]
    fn dropping_tab_on_same_frame_selects_without_reordering() {
        let mut area = dock();
        assert!(area.set_active_frame(FrameId::from_raw(1)));
        let drag = area
            .begin_tab_drag(FrameId::from_raw(2), PanelId::from_raw(3))
            .expect("drag");

        assert!(area.drop_tab(drag, DockDropTarget::tab(FrameId::from_raw(2))));

        let frame = area.frame(FrameId::from_raw(2)).expect("frame");
        assert_eq!(frame.panels.len(), 2);
        assert_eq!(
            frame
                .panels
                .iter()
                .map(|panel| panel.id)
                .collect::<Vec<_>>(),
            vec![PanelId::from_raw(2), PanelId::from_raw(3)]
        );
        assert_eq!(
            frame.active_panel().expect("active").id,
            PanelId::from_raw(3)
        );
        assert_eq!(area.active_frame(), Some(FrameId::from_raw(2)));
    }

    #[test]
    fn dropping_tab_on_frame_merges_and_selects_panel() {
        let mut area = dock();
        let drag = area
            .begin_tab_drag(FrameId::from_raw(2), PanelId::from_raw(3))
            .expect("drag");

        assert!(area.drop_tab(drag, DockDropTarget::tab(FrameId::from_raw(1))));

        let target = area.frame(FrameId::from_raw(1)).expect("target");
        assert_eq!(target.panels.len(), 2);
        assert_eq!(
            target.active_panel().expect("active").id,
            PanelId::from_raw(3)
        );
        assert_eq!(area.active_frame(), Some(FrameId::from_raw(1)));
        assert_eq!(
            area.frame(FrameId::from_raw(2))
                .expect("source")
                .panels
                .len(),
            1
        );
    }

    #[test]
    fn dropping_tab_preserves_dismissible_policy_through_snapshot() {
        let mut area = dock();
        area.frame_mut(FrameId::from_raw(2))
            .expect("source")
            .set_panel_dismissible(PanelId::from_raw(3), false);
        let drag = area
            .begin_tab_drag(FrameId::from_raw(2), PanelId::from_raw(3))
            .expect("drag");

        assert!(area.drop_tab(drag, DockDropTarget::tab(FrameId::from_raw(1))));

        let target = area.frame(FrameId::from_raw(1)).expect("target");
        assert!(!target.panel_dismissible(PanelId::from_raw(3)));

        let restored = Dock::restore(area.snapshot()).expect("restore");
        assert!(
            !restored
                .frame(FrameId::from_raw(1))
                .expect("restored target")
                .panel_dismissible(PanelId::from_raw(3))
        );
    }

    #[test]
    fn dropping_tab_on_split_edge_inserts_new_frame_and_round_trips() {
        let mut area = dock();
        area.frame_mut(FrameId::from_raw(2))
            .expect("source")
            .set_panel_dismissible(PanelId::from_raw(3), false);
        let drag = area
            .begin_tab_drag(FrameId::from_raw(2), PanelId::from_raw(3))
            .expect("drag");
        let insertion = DockSplitInsertion {
            target_frame: FrameId::from_raw(1),
            placement: DockPlacement::Left,
            new_frame: FrameId::from_raw(9),
            ratio: 0.3,
            min_first: 80.0,
            min_second: 120.0,
        };

        assert!(area.drop_tab(
            drag,
            DockDropTarget::Split {
                frame: insertion.target_frame,
                placement: insertion.placement,
                new_frame: insertion.new_frame,
                ratio: insertion.ratio,
                min_first: insertion.min_first,
                min_second: insertion.min_second,
            }
        ));

        let frames = area.frames();
        assert_eq!(frames.len(), 3);
        assert_eq!(area.active_frame(), Some(FrameId::from_raw(9)));
        assert_eq!(
            area.frame(FrameId::from_raw(9))
                .expect("inserted")
                .active_panel()
                .expect("panel")
                .id,
            PanelId::from_raw(3)
        );
        assert!(
            !area
                .frame(FrameId::from_raw(9))
                .expect("inserted")
                .panel_dismissible(PanelId::from_raw(3))
        );
        let restored = Dock::restore(area.snapshot()).expect("restore");
        assert_eq!(restored.frames().len(), 3);
        assert_eq!(restored.active_frame(), Some(FrameId::from_raw(9)));
        assert_eq!(
            restored
                .frame(FrameId::from_raw(9))
                .expect("inserted")
                .active_panel()
                .expect("panel")
                .id,
            PanelId::from_raw(3)
        );
        assert!(
            !restored
                .frame(FrameId::from_raw(9))
                .expect("restored inserted")
                .panel_dismissible(PanelId::from_raw(3))
        );
    }

    #[test]
    fn invalid_tab_drop_does_not_remove_panel() {
        let mut area = dock();
        let drag = area
            .begin_tab_drag(FrameId::from_raw(2), PanelId::from_raw(3))
            .expect("drag");

        assert!(!area.drop_tab(drag, DockDropTarget::tab(FrameId::from_raw(99))));

        assert!(
            area.frame(FrameId::from_raw(2))
                .expect("source")
                .panels
                .iter()
                .any(|panel| panel.id == PanelId::from_raw(3))
        );
    }

    #[test]
    fn invalid_split_drop_does_not_remove_panel() {
        let mut area = dock();
        let drag = area
            .begin_tab_drag(FrameId::from_raw(2), PanelId::from_raw(3))
            .expect("drag");

        assert!(!area.drop_tab(
            drag,
            DockDropTarget::split(
                FrameId::from_raw(99),
                DockPlacement::Left,
                FrameId::from_raw(9)
            )
        ));

        assert!(
            area.frame(FrameId::from_raw(2))
                .expect("source")
                .panels
                .iter()
                .any(|panel| panel.id == PanelId::from_raw(3))
        );
    }

    #[test]
    fn invalid_split_numbers_do_not_remove_panel() {
        let mut area = dock();
        let drag = area
            .begin_tab_drag(FrameId::from_raw(2), PanelId::from_raw(3))
            .expect("drag");

        assert!(!area.drop_tab(
            drag,
            DockDropTarget::Split {
                frame: FrameId::from_raw(1),
                placement: DockPlacement::Left,
                new_frame: FrameId::from_raw(9),
                ratio: f32::NAN,
                min_first: 0.0,
                min_second: 0.0,
            }
        ));

        assert!(
            area.frame(FrameId::from_raw(2))
                .expect("source")
                .panels
                .iter()
                .any(|panel| panel.id == PanelId::from_raw(3))
        );
    }

    #[test]
    fn dock_semantic_role_uses_current_name() {
        assert_eq!(format!("{:?}", SemanticRole::Dock), "Dock");
    }

    #[test]
    fn snapshots_round_trip() {
        let mut area = dock();
        assert!(area.select_panel(FrameId::from_raw(2), PanelId::from_raw(3)));
        assert!(area.set_active_frame(FrameId::from_raw(1)));
        let snapshot = area.snapshot();
        let restored = Dock::restore(snapshot).expect("restore");

        assert_eq!(restored.frames().len(), 2);
        assert_eq!(restored.active_frame(), Some(FrameId::from_raw(1)));
        assert_eq!(
            restored
                .frame(FrameId::from_raw(2))
                .expect("frame")
                .active_panel()
                .expect("active")
                .id,
            PanelId::from_raw(3)
        );
    }

    #[test]
    fn restore_defaults_missing_active_frame_to_first_valid_frame() {
        let mut snapshot = dock().snapshot();
        snapshot.active_frame = None;

        let restored = Dock::restore(snapshot).expect("restore");

        assert_eq!(restored.active_frame(), Some(FrameId::from_raw(1)));
    }

    #[test]
    fn invalid_snapshots_are_rejected() {
        let snapshot = DockSnapshot {
            active_frame: Some(FrameId::from_raw(1)),
            root: DockSnapshotNode::Frame {
                id: FrameId::from_raw(1),
                panels: vec![],
                active: 0,
                dismissible_panels: vec![],
            },
        };

        assert_eq!(
            Dock::restore(snapshot).expect_err("error"),
            DockRestoreError::EmptyFrame
        );
    }

    #[test]
    fn invalid_snapshot_rejects_invalid_active_panel() {
        let snapshot = DockSnapshot {
            active_frame: Some(FrameId::from_raw(1)),
            root: DockSnapshotNode::Frame {
                id: FrameId::from_raw(1),
                panels: vec![panel(1, "A")],
                active: 1,
                dismissible_panels: vec![PanelId::from_raw(1)],
            },
        };

        assert_eq!(
            Dock::restore(snapshot).expect_err("error"),
            DockRestoreError::InvalidActiveIndex
        );
    }

    #[test]
    fn invalid_snapshot_rejects_unknown_dismissible_panel() {
        let snapshot = DockSnapshot {
            active_frame: Some(FrameId::from_raw(1)),
            root: DockSnapshotNode::Frame {
                id: FrameId::from_raw(1),
                panels: vec![panel(1, "A")],
                active: 0,
                dismissible_panels: vec![PanelId::from_raw(2)],
            },
        };

        assert_eq!(
            Dock::restore(snapshot).expect_err("error"),
            DockRestoreError::InvalidDismissiblePanel
        );
    }

    #[test]
    fn invalid_snapshot_rejects_unknown_active_frame() {
        let snapshot = DockSnapshot {
            active_frame: Some(FrameId::from_raw(99)),
            root: DockSnapshotNode::Frame {
                id: FrameId::from_raw(1),
                panels: vec![panel(1, "A")],
                active: 0,
                dismissible_panels: vec![PanelId::from_raw(1)],
            },
        };

        assert_eq!(
            Dock::restore(snapshot).expect_err("error"),
            DockRestoreError::InvalidActiveFrame
        );
    }

    #[test]
    fn invalid_snapshot_rejects_duplicate_frame_ids() {
        let snapshot = DockSnapshot {
            active_frame: Some(FrameId::from_raw(1)),
            root: DockSnapshotNode::Split {
                axis: Axis::Horizontal,
                ratio: 0.5,
                min_first: 0.0,
                min_second: 0.0,
                first: Box::new(DockSnapshotNode::Frame {
                    id: FrameId::from_raw(1),
                    panels: vec![panel(1, "A")],
                    active: 0,
                    dismissible_panels: vec![PanelId::from_raw(1)],
                }),
                second: Box::new(DockSnapshotNode::Frame {
                    id: FrameId::from_raw(1),
                    panels: vec![panel(2, "B")],
                    active: 0,
                    dismissible_panels: vec![PanelId::from_raw(2)],
                }),
            },
        };

        assert_eq!(
            Dock::restore(snapshot).expect_err("error"),
            DockRestoreError::DuplicateFrameId
        );
    }

    #[test]
    fn invalid_snapshot_rejects_duplicate_panel_ids() {
        let snapshot = DockSnapshot {
            active_frame: Some(FrameId::from_raw(1)),
            root: DockSnapshotNode::Split {
                axis: Axis::Horizontal,
                ratio: 0.5,
                min_first: 0.0,
                min_second: 0.0,
                first: Box::new(DockSnapshotNode::Frame {
                    id: FrameId::from_raw(1),
                    panels: vec![panel(1, "A")],
                    active: 0,
                    dismissible_panels: vec![PanelId::from_raw(1)],
                }),
                second: Box::new(DockSnapshotNode::Frame {
                    id: FrameId::from_raw(2),
                    panels: vec![panel(1, "B")],
                    active: 0,
                    dismissible_panels: vec![PanelId::from_raw(1)],
                }),
            },
        };

        assert_eq!(
            Dock::restore(snapshot).expect_err("error"),
            DockRestoreError::DuplicatePanelId
        );
    }

    #[test]
    fn invalid_snapshot_rejects_duplicate_dismissible_policy_entries() {
        let snapshot = DockSnapshot {
            active_frame: Some(FrameId::from_raw(1)),
            root: DockSnapshotNode::Frame {
                id: FrameId::from_raw(1),
                panels: vec![panel(1, "A")],
                active: 0,
                dismissible_panels: vec![PanelId::from_raw(1), PanelId::from_raw(1)],
            },
        };

        assert_eq!(
            Dock::restore(snapshot).expect_err("error"),
            DockRestoreError::DuplicateDismissiblePanel
        );
    }

    #[test]
    fn invalid_snapshot_rejects_invalid_split_numbers() {
        let invalid_ratio = DockSnapshot {
            active_frame: Some(FrameId::from_raw(1)),
            root: DockSnapshotNode::Split {
                axis: Axis::Horizontal,
                ratio: f32::NAN,
                min_first: 0.0,
                min_second: 0.0,
                first: Box::new(DockSnapshotNode::Frame {
                    id: FrameId::from_raw(1),
                    panels: vec![panel(1, "A")],
                    active: 0,
                    dismissible_panels: vec![PanelId::from_raw(1)],
                }),
                second: Box::new(DockSnapshotNode::Frame {
                    id: FrameId::from_raw(2),
                    panels: vec![panel(2, "B")],
                    active: 0,
                    dismissible_panels: vec![PanelId::from_raw(2)],
                }),
            },
        };
        assert_eq!(
            Dock::restore(invalid_ratio).expect_err("error"),
            DockRestoreError::InvalidSplitRatio
        );

        let invalid_minimum = DockSnapshot {
            active_frame: Some(FrameId::from_raw(1)),
            root: DockSnapshotNode::Split {
                axis: Axis::Horizontal,
                ratio: 0.5,
                min_first: -1.0,
                min_second: 0.0,
                first: Box::new(DockSnapshotNode::Frame {
                    id: FrameId::from_raw(1),
                    panels: vec![panel(1, "A")],
                    active: 0,
                    dismissible_panels: vec![PanelId::from_raw(1)],
                }),
                second: Box::new(DockSnapshotNode::Frame {
                    id: FrameId::from_raw(2),
                    panels: vec![panel(2, "B")],
                    active: 0,
                    dismissible_panels: vec![PanelId::from_raw(2)],
                }),
            },
        };

        assert_eq!(
            Dock::restore(invalid_minimum).expect_err("error"),
            DockRestoreError::InvalidSplitMinimum
        );
    }
}
