use std::time::Duration;

use stern_core::{ActionContext, ActionDescriptor, ActionInvocation, ActionSource, RepaintRequest};

/// Stable identity for transient feedback, toast, or notification items.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FeedbackId(u64);

impl FeedbackId {
    /// Creates a feedback ID from raw bits.
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

/// User-facing feedback kind for non-blocking presentation surfaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FeedbackKind {
    /// Neutral information.
    Info,
    /// Successful completion or confirmation.
    Success,
    /// Recoverable warning.
    Warning,
    /// Error state or failed operation.
    Error,
}

/// Deterministic lifetime policy for one feedback item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FeedbackLifetime {
    /// Feedback remains active until explicitly dismissed or replaced by the application.
    Pinned,
    /// Feedback expires after a fixed duration from an explicit shown time.
    Timed {
        /// Application-provided timestamp at which this feedback became visible.
        shown_at: Duration,
        /// Time this feedback should remain active.
        duration: Duration,
    },
}

impl FeedbackLifetime {
    /// Creates timed feedback from explicit time and duration inputs.
    #[must_use]
    pub const fn timed(shown_at: Duration, duration: Duration) -> Self {
        Self::Timed { shown_at, duration }
    }

    /// Returns the deterministic expiry time for timed feedback.
    #[must_use]
    pub fn expires_at(self) -> Option<Duration> {
        match self {
            Self::Pinned => None,
            Self::Timed { shown_at, duration } => Some(shown_at.saturating_add(duration)),
        }
    }

    /// Returns true when this lifetime is still active at `now`.
    #[must_use]
    pub fn is_active(self, now: Duration) -> bool {
        match self {
            Self::Pinned => true,
            Self::Timed { shown_at, duration } => now < shown_at.saturating_add(duration),
        }
    }

    /// Returns remaining time for active timed feedback.
    #[must_use]
    pub fn remaining(self, now: Duration) -> Option<Duration> {
        let expires_at = self.expires_at()?;
        if now < expires_at {
            expires_at.checked_sub(now)
        } else {
            None
        }
    }
}

/// Optional application-owned action metadata shown on a feedback item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeedbackAction {
    /// Action metadata shared with menus, command palettes, shortcuts, and buttons.
    pub action: ActionDescriptor,
    /// Context emitted with the action invocation.
    pub context: ActionContext,
}

impl FeedbackAction {
    /// Creates feedback action metadata from an application-owned action descriptor.
    #[must_use]
    pub const fn new(action: ActionDescriptor, context: ActionContext) -> Self {
        Self { action, context }
    }

    /// Returns true when the action affordance should be presented.
    #[must_use]
    pub const fn visible(&self) -> bool {
        self.action.state.visible
    }

    /// Returns true when the action affordance can currently be invoked.
    #[must_use]
    pub const fn enabled(&self) -> bool {
        self.action.state.enabled
    }

    /// Returns true when this feedback action is both visible and enabled.
    #[must_use]
    pub const fn can_request(&self) -> bool {
        self.action.can_invoke()
    }

    /// Creates an invocation for an enabled visible feedback action.
    #[must_use]
    pub fn invocation(&self) -> Option<ActionInvocation> {
        self.can_request().then(|| {
            ActionInvocation::new(
                self.action.id.clone(),
                ActionSource::Button,
                self.context.clone(),
            )
        })
    }
}

/// Optional application-owned dismiss action metadata shown on a feedback item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeedbackDismiss {
    /// Action metadata used by a dismiss affordance.
    pub action: ActionDescriptor,
    /// Context emitted with the dismiss action invocation.
    pub context: ActionContext,
}

impl FeedbackDismiss {
    /// Creates dismiss metadata from an application-owned action descriptor.
    #[must_use]
    pub const fn new(action: ActionDescriptor, context: ActionContext) -> Self {
        Self { action, context }
    }

    /// Returns true when the dismiss affordance should be presented.
    #[must_use]
    pub const fn visible(&self) -> bool {
        self.action.state.visible
    }

    /// Returns true when the dismiss affordance can currently be invoked.
    #[must_use]
    pub const fn enabled(&self) -> bool {
        self.action.state.enabled
    }

    /// Returns true when this dismiss action is both visible and enabled.
    #[must_use]
    pub const fn can_request(&self) -> bool {
        self.action.can_invoke()
    }

    /// Creates a dismiss invocation for an enabled visible dismiss affordance.
    #[must_use]
    pub fn invocation(&self) -> Option<ActionInvocation> {
        self.can_request().then(|| {
            ActionInvocation::new(
                self.action.id.clone(),
                ActionSource::Button,
                self.context.clone(),
            )
        })
    }
}

/// Action request metadata emitted by a feedback action affordance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeedbackActionRequest {
    /// Stable feedback identity supplied by the application.
    pub feedback_id: FeedbackId,
    /// Action invocation for the application to execute.
    pub invocation: ActionInvocation,
}

impl FeedbackActionRequest {
    /// Creates feedback action request metadata.
    #[must_use]
    pub const fn new(feedback_id: FeedbackId, invocation: ActionInvocation) -> Self {
        Self {
            feedback_id,
            invocation,
        }
    }
}

/// Dismiss request metadata emitted by a feedback dismiss affordance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeedbackDismissRequest {
    /// Stable feedback identity supplied by the application.
    pub feedback_id: FeedbackId,
    /// Dismiss action invocation for the application to execute.
    pub invocation: ActionInvocation,
}

impl FeedbackDismissRequest {
    /// Creates feedback dismiss request metadata.
    #[must_use]
    pub const fn new(feedback_id: FeedbackId, invocation: ActionInvocation) -> Self {
        Self {
            feedback_id,
            invocation,
        }
    }
}

/// Data-only non-blocking feedback item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeedbackItem {
    /// Stable feedback identity.
    pub id: FeedbackId,
    /// Feedback severity/kind.
    pub kind: FeedbackKind,
    /// Short label for compact presentation or accessibility.
    pub label: String,
    /// User-facing feedback text.
    pub text: String,
    /// Deterministic lifetime policy.
    pub lifetime: FeedbackLifetime,
    /// Whether the item has been dismissed by application state.
    pub dismissed: bool,
    /// Optional primary action metadata.
    pub action: Option<FeedbackAction>,
    /// Optional dismiss action metadata.
    pub dismiss: Option<FeedbackDismiss>,
}

impl FeedbackItem {
    /// Creates an active feedback item.
    #[must_use]
    pub fn new(
        id: FeedbackId,
        kind: FeedbackKind,
        label: impl Into<String>,
        text: impl Into<String>,
        lifetime: FeedbackLifetime,
    ) -> Self {
        Self {
            id,
            kind,
            label: label.into(),
            text: text.into(),
            lifetime,
            dismissed: false,
            action: None,
            dismiss: None,
        }
    }

    /// Creates pinned feedback that remains active until dismissed.
    #[must_use]
    pub fn pinned(
        id: FeedbackId,
        kind: FeedbackKind,
        label: impl Into<String>,
        text: impl Into<String>,
    ) -> Self {
        Self::new(id, kind, label, text, FeedbackLifetime::Pinned)
    }

    /// Creates timed feedback from explicit time and duration inputs.
    #[must_use]
    pub fn timed(
        id: FeedbackId,
        kind: FeedbackKind,
        label: impl Into<String>,
        text: impl Into<String>,
        shown_at: Duration,
        duration: Duration,
    ) -> Self {
        Self::new(
            id,
            kind,
            label,
            text,
            FeedbackLifetime::timed(shown_at, duration),
        )
    }

    /// Sets dismissed state metadata.
    #[must_use]
    pub const fn with_dismissed(mut self, dismissed: bool) -> Self {
        self.dismissed = dismissed;
        self
    }

    /// Sets optional primary action metadata.
    #[must_use]
    pub fn with_action(mut self, action: FeedbackAction) -> Self {
        self.action = Some(action);
        self
    }

    /// Sets optional dismiss action metadata.
    #[must_use]
    pub fn with_dismiss(mut self, dismiss: FeedbackDismiss) -> Self {
        self.dismiss = Some(dismiss);
        self
    }

    /// Returns the deterministic expiry time for timed feedback.
    #[must_use]
    pub fn expires_at(&self) -> Option<Duration> {
        self.lifetime.expires_at()
    }

    /// Returns true when this item is not dismissed and its lifetime remains active.
    #[must_use]
    pub fn is_active(&self, now: Duration) -> bool {
        !self.dismissed && self.lifetime.is_active(now)
    }

    /// Returns remaining time for active timed feedback.
    #[must_use]
    pub fn remaining_lifetime(&self, now: Duration) -> Option<Duration> {
        if self.dismissed {
            None
        } else {
            self.lifetime.remaining(now)
        }
    }

    /// Creates primary action request metadata when the action is visible and enabled.
    #[must_use]
    pub fn action_request(&self) -> Option<FeedbackActionRequest> {
        self.action
            .as_ref()?
            .invocation()
            .map(|invocation| FeedbackActionRequest::new(self.id, invocation))
    }

    /// Creates dismiss request metadata when the dismiss action is visible and enabled.
    #[must_use]
    pub fn dismiss_request(&self) -> Option<FeedbackDismissRequest> {
        self.dismiss
            .as_ref()?
            .invocation()
            .map(|invocation| FeedbackDismissRequest::new(self.id, invocation))
    }
}

/// Data-only stack of transient feedback, toasts, or notifications.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FeedbackStack {
    items: Vec<FeedbackItem>,
}

impl FeedbackStack {
    /// Creates an empty feedback stack.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a feedback stack from ordered item definitions.
    #[must_use]
    pub fn from_items(items: impl IntoIterator<Item = FeedbackItem>) -> Self {
        Self {
            items: items.into_iter().collect(),
        }
    }

    /// Returns feedback items in application-supplied presentation order.
    #[must_use]
    pub fn items(&self) -> &[FeedbackItem] {
        &self.items
    }

    /// Replaces feedback items while preserving application-supplied order.
    pub fn replace_items(&mut self, items: impl IntoIterator<Item = FeedbackItem>) {
        self.items = items.into_iter().collect();
    }

    /// Replaces the first matching feedback identity in place or appends a new identity.
    pub fn push_item(&mut self, item: FeedbackItem) {
        if let Some(existing) = self
            .items
            .iter_mut()
            .find(|existing| existing.id == item.id)
        {
            *existing = item;
        } else {
            self.items.push(item);
        }
    }

    /// Returns a feedback item by stable identity.
    #[must_use]
    pub fn item(&self, id: FeedbackId) -> Option<&FeedbackItem> {
        self.items.iter().find(|item| item.id == id)
    }

    /// Returns active feedback items in deterministic insertion order.
    #[must_use]
    pub fn active_items(&self, now: Duration) -> Vec<&FeedbackItem> {
        self.active_items_iter(now).collect()
    }

    /// Returns active feedback items as a borrowed iterator.
    pub fn active_items_iter(&self, now: Duration) -> impl Iterator<Item = &FeedbackItem> + '_ {
        self.items.iter().filter(move |item| item.is_active(now))
    }

    /// Creates primary action request metadata for one active feedback item.
    #[must_use]
    pub fn action_request(&self, id: FeedbackId, now: Duration) -> Option<FeedbackActionRequest> {
        let item = self.item(id)?;
        if item.is_active(now) {
            item.action_request()
        } else {
            None
        }
    }

    /// Creates dismiss request metadata for one active feedback item.
    #[must_use]
    pub fn dismiss_request(&self, id: FeedbackId, now: Duration) -> Option<FeedbackDismissRequest> {
        let item = self.item(id)?;
        if item.is_active(now) {
            item.dismiss_request()
        } else {
            None
        }
    }

    /// Returns the bounded repaint request needed for active timed feedback.
    #[must_use]
    pub fn repaint_request(&self, now: Duration) -> RepaintRequest {
        self.items
            .iter()
            .filter_map(|item| item.remaining_lifetime(now))
            .min()
            .map_or(RepaintRequest::None, RepaintRequest::After)
    }
}
