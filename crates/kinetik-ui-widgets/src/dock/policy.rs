use super::{
    DEFAULT_SPLITTER_THICKNESS, DROP_EDGE_FRACTION, DockSplitterContextActionKind,
    sanitize_drop_edge_fraction, splitter_thickness,
};

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

    pub(crate) const fn allows_splitter_action(self, kind: DockSplitterContextActionKind) -> bool {
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
