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
    let mut splitters = Vec::new();
    solve_splitters(
        &area.root,
        bounds,
        splitter_thickness(thickness),
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
    let edge_x = finite_non_negative(rect.width) * DROP_EDGE_FRACTION;
    let edge_y = finite_non_negative(rect.height) * DROP_EDGE_FRACTION;

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
    frames.iter().find_map(|layout| {
        let zone = resolve_frame_drop_zone(layout.rect, point)?;
        Some(match zone.placement() {
            Some(placement) => DockDropTarget::split(layout.frame, placement, new_frame),
            None => DockDropTarget::tab(layout.frame),
        })
    })
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
