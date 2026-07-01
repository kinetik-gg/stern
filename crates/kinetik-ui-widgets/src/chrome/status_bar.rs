/// Stable identity for a status bar item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct StatusItemId(u64);

impl StatusItemId {
    /// Creates a status item ID from raw bits.
    #[must_use]
    pub const fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    /// Returns the raw ID bits.
    #[must_use]
    pub const fn raw(self) -> u64 {
        self.0
    }
}

/// Data category for a status bar item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StatusItemKind {
    /// General non-blocking status text.
    Message,
    /// Count of available or unavailable actions.
    ActionCount,
    /// Count of queued, active, or completed jobs.
    JobCount,
    /// Normalized progress metadata.
    Progress,
    /// Ready state.
    Ready,
    /// Pending or queued state.
    Pending,
    /// Stale or out-of-date state.
    Stale,
    /// Error state.
    Error,
}

/// Normalized progress metadata for status bar presentation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StatusProgress {
    /// Clamped progress value in the inclusive `0.0..=1.0` range.
    pub value: f32,
}

impl StatusProgress {
    /// Creates progress metadata, replacing non-finite values with `0.0` and clamping to range.
    #[must_use]
    pub fn new(value: f32) -> Self {
        let value = if value.is_finite() { value } else { 0.0 };
        Self {
            value: value.clamp(0.0, 1.0),
        }
    }

    /// Creates progress metadata from a completed/total pair.
    #[must_use]
    pub fn from_fraction(completed: f32, total: f32) -> Self {
        if !completed.is_finite() || !total.is_finite() || total <= 0.0 {
            return Self::new(0.0);
        }
        Self::new(completed / total)
    }
}

/// Data-only status bar item.
#[derive(Debug, Clone, PartialEq)]
pub struct StatusItem {
    /// Stable status item identity.
    pub id: StatusItemId,
    /// Short label for compact presentation or accessibility.
    pub label: String,
    /// Status text shown by the application.
    pub text: String,
    /// Typed status category.
    pub kind: StatusItemKind,
    /// Optional sanitized count metadata.
    pub count: Option<u32>,
    /// Optional normalized progress metadata.
    pub progress: Option<StatusProgress>,
    /// Whether this item should be presented on visible status bar surfaces.
    pub visible: bool,
}

impl StatusItem {
    /// Creates a visible status bar item.
    #[must_use]
    pub fn new(
        id: StatusItemId,
        label: impl Into<String>,
        text: impl Into<String>,
        kind: StatusItemKind,
    ) -> Self {
        Self {
            id,
            label: label.into(),
            text: text.into(),
            kind,
            count: None,
            progress: None,
            visible: true,
        }
    }

    /// Sets count metadata for action, job, or diagnostic count presentation.
    #[must_use]
    pub const fn with_count(mut self, count: u32) -> Self {
        self.count = Some(count);
        self
    }

    /// Sets normalized progress metadata.
    #[must_use]
    pub const fn with_progress(mut self, progress: StatusProgress) -> Self {
        self.progress = Some(progress);
        self
    }

    /// Sets progress metadata from a raw value.
    #[must_use]
    pub fn with_progress_value(self, value: f32) -> Self {
        self.with_progress(StatusProgress::new(value))
    }

    /// Sets progress metadata from completed/total values.
    #[must_use]
    pub fn with_progress_fraction(self, completed: f32, total: f32) -> Self {
        self.with_progress(StatusProgress::from_fraction(completed, total))
    }

    /// Sets visibility metadata.
    #[must_use]
    pub const fn with_visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }
}

/// Data-only status bar model made of ordered items.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct StatusBar {
    items: Vec<StatusItem>,
}

impl StatusBar {
    /// Creates an empty status bar.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a status bar from ordered item definitions.
    #[must_use]
    pub fn from_items(items: impl IntoIterator<Item = StatusItem>) -> Self {
        Self {
            items: items.into_iter().collect(),
        }
    }

    /// Returns status items in presentation order.
    #[must_use]
    pub fn items(&self) -> &[StatusItem] {
        &self.items
    }

    /// Replaces status items.
    pub fn replace_items(&mut self, items: impl IntoIterator<Item = StatusItem>) {
        self.items = items.into_iter().collect();
    }

    /// Returns a status item by stable identity.
    #[must_use]
    pub fn item(&self, id: StatusItemId) -> Option<&StatusItem> {
        self.items.iter().find(|item| item.id == id)
    }

    /// Returns visible status items in presentation order.
    #[must_use]
    pub fn visible_items(&self) -> Vec<&StatusItem> {
        self.visible_items_iter().collect()
    }

    /// Returns visible status items as a borrowed iterator.
    pub fn visible_items_iter(&self) -> impl Iterator<Item = &StatusItem> + '_ {
        self.items.iter().filter(|item| item.visible)
    }
}
