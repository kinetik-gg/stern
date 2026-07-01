use super::Axis;

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
    pub(crate) const fn axis(self) -> Axis {
        match self {
            Self::Left | Self::Right => Axis::Horizontal,
            Self::Top | Self::Bottom => Axis::Vertical,
        }
    }

    pub(crate) const fn insert_before_target(self) -> bool {
        matches!(self, Self::Left | Self::Top)
    }
}
