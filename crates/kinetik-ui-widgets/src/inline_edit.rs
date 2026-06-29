//! Data-only inline edit contracts for app-owned item renaming.

use kinetik_ui_core::{Key, KeyEvent, KeyState, UiInput, WidgetId};

use crate::{ItemId, Selection};

/// Focus-loss behavior for an active inline edit session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InlineEditFocusLossPolicy {
    /// Keep the session open and emit no request.
    KeepEditing,
    /// Commit the current draft on focus loss.
    Commit,
    /// Cancel the current session on focus loss.
    Cancel,
}

/// Policy action used for empty or unchanged drafts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InlineEditDraftDisposition {
    /// Emit a commit request.
    Commit,
    /// Emit a cancel request.
    Cancel,
    /// Emit no request and keep the session open.
    Reject,
}

/// Explicit policy for drafts that are empty or unchanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InlineEditDraftPolicy {
    /// Disposition for drafts whose trimmed text is empty.
    pub empty: InlineEditDraftDisposition,
    /// Disposition for drafts that exactly match the initial text.
    pub unchanged: InlineEditDraftDisposition,
}

impl InlineEditDraftPolicy {
    /// Creates an explicit draft policy.
    #[must_use]
    pub const fn new(
        empty: InlineEditDraftDisposition,
        unchanged: InlineEditDraftDisposition,
    ) -> Self {
        Self { empty, unchanged }
    }
}

/// Current draft classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InlineEditDraftStatus {
    /// The trimmed draft is empty.
    Empty,
    /// The draft exactly matches the initial text.
    Unchanged,
    /// The draft is non-empty and differs from the initial text.
    Changed,
}

/// Item eligibility for inline editing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InlineEditEligibility {
    /// Item is unavailable for interaction.
    pub disabled: bool,
    /// Item is visible but not mutable.
    pub read_only: bool,
    /// Item exposes a rename affordance.
    pub renamable: bool,
}

impl InlineEditEligibility {
    /// Creates an eligibility contract.
    #[must_use]
    pub const fn new(disabled: bool, read_only: bool, renamable: bool) -> Self {
        Self {
            disabled,
            read_only,
            renamable,
        }
    }

    /// Returns true when inline edit can begin.
    #[must_use]
    pub const fn can_begin(self) -> bool {
        !self.disabled && !self.read_only && self.renamable
    }
}

/// Begin-edit metadata emitted by an item surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InlineEditBeginRequest {
    /// Target item ID.
    pub target: ItemId,
    /// Initial app-owned display text.
    pub initial_text: String,
    /// Text widget ID that owns text input for this session.
    pub text_widget_id: WidgetId,
}

impl InlineEditBeginRequest {
    /// Creates a begin-edit request when eligibility allows it.
    #[must_use]
    pub fn new(
        target: ItemId,
        initial_text: impl Into<String>,
        text_widget_id: WidgetId,
        eligibility: InlineEditEligibility,
    ) -> Option<Self> {
        eligibility.can_begin().then(|| Self {
            target,
            initial_text: initial_text.into(),
            text_widget_id,
        })
    }
}

/// Draft-edit metadata emitted while the text field changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InlineEditDraftEdit {
    /// Target item ID.
    pub target: ItemId,
    /// Current draft text.
    pub draft_text: String,
    /// Text widget ID that owns text input for this session.
    pub text_widget_id: WidgetId,
}

/// Commit reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InlineEditCommitReason {
    /// Enter key committed the draft.
    Enter,
    /// Focus loss committed the draft.
    FocusLost,
    /// Caller explicitly requested commit.
    Explicit,
}

/// Commit request emitted by an inline edit session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InlineEditCommitRequest {
    /// Target item ID.
    pub target: ItemId,
    /// Current draft text.
    pub draft_text: String,
    /// Text widget ID that owns text input for this session.
    pub text_widget_id: WidgetId,
    /// Commit trigger.
    pub reason: InlineEditCommitReason,
}

/// Cancel reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InlineEditCancelReason {
    /// Escape key canceled the draft.
    Escape,
    /// Focus loss canceled the draft.
    FocusLost,
    /// Empty or unchanged draft policy canceled the draft.
    DraftPolicy,
    /// Caller explicitly requested cancel.
    Explicit,
}

/// Cancel request emitted by an inline edit session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InlineEditCancelRequest {
    /// Target item ID.
    pub target: ItemId,
    /// Current draft text at cancel time.
    pub draft_text: String,
    /// Text widget ID that owns text input for this session.
    pub text_widget_id: WidgetId,
    /// Cancel trigger.
    pub reason: InlineEditCancelReason,
}

/// Request emitted by inline edit contract helpers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InlineEditRequest {
    /// Begin editing an item.
    Begin(InlineEditBeginRequest),
    /// Draft text changed.
    DraftEdit(InlineEditDraftEdit),
    /// Commit the draft to the application.
    Commit(InlineEditCommitRequest),
    /// Cancel the edit session.
    Cancel(InlineEditCancelRequest),
}

/// Result of resolving a commit attempt through the draft policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InlineEditResolution {
    /// Draft classification used for the decision.
    pub draft_status: InlineEditDraftStatus,
    /// Policy disposition applied to this draft.
    pub disposition: InlineEditDraftDisposition,
    /// Request emitted by the decision, if any.
    pub request: Option<InlineEditRequest>,
}

/// Active inline edit session metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InlineEditSession {
    /// Target item ID.
    pub target: ItemId,
    /// Initial app-owned display text.
    pub initial_text: String,
    /// Current draft text.
    pub draft_text: String,
    /// Text widget ID that owns text input for this session.
    pub text_widget_id: WidgetId,
    /// Explicit focus-loss policy.
    pub focus_loss_policy: InlineEditFocusLossPolicy,
    /// Explicit draft policy.
    pub draft_policy: InlineEditDraftPolicy,
}

impl InlineEditSession {
    /// Creates a session from begin-edit metadata.
    #[must_use]
    pub fn new(
        begin: InlineEditBeginRequest,
        focus_loss_policy: InlineEditFocusLossPolicy,
        draft_policy: InlineEditDraftPolicy,
    ) -> Self {
        Self {
            target: begin.target,
            draft_text: begin.initial_text.clone(),
            initial_text: begin.initial_text,
            text_widget_id: begin.text_widget_id,
            focus_loss_policy,
            draft_policy,
        }
    }

    /// Returns true when the active selection still contains the edited item.
    #[must_use]
    pub fn target_selected(&self, selection: &Selection) -> bool {
        selection.contains(self.target)
    }

    /// Classifies the current draft.
    #[must_use]
    pub fn draft_status(&self) -> InlineEditDraftStatus {
        classify_inline_edit_draft(&self.initial_text, &self.draft_text)
    }

    /// Updates draft text and returns draft-edit metadata.
    pub fn set_draft(&mut self, draft_text: impl Into<String>) -> InlineEditDraftEdit {
        self.draft_text = draft_text.into();
        self.draft_edit()
    }

    /// Returns draft-edit metadata for the current draft.
    #[must_use]
    pub fn draft_edit(&self) -> InlineEditDraftEdit {
        InlineEditDraftEdit {
            target: self.target,
            draft_text: self.draft_text.clone(),
            text_widget_id: self.text_widget_id,
        }
    }

    /// Returns commit metadata for the current draft.
    #[must_use]
    pub fn commit_request(&self, reason: InlineEditCommitReason) -> InlineEditCommitRequest {
        InlineEditCommitRequest {
            target: self.target,
            draft_text: self.draft_text.clone(),
            text_widget_id: self.text_widget_id,
            reason,
        }
    }

    /// Returns cancel metadata for the current draft.
    #[must_use]
    pub fn cancel_request(&self, reason: InlineEditCancelReason) -> InlineEditCancelRequest {
        InlineEditCancelRequest {
            target: self.target,
            draft_text: self.draft_text.clone(),
            text_widget_id: self.text_widget_id,
            reason,
        }
    }

    /// Resolves a commit attempt through the explicit draft policy.
    #[must_use]
    pub fn resolve_commit(&self, reason: InlineEditCommitReason) -> InlineEditResolution {
        let draft_status = self.draft_status();
        let disposition = match draft_status {
            InlineEditDraftStatus::Empty => self.draft_policy.empty,
            InlineEditDraftStatus::Unchanged => self.draft_policy.unchanged,
            InlineEditDraftStatus::Changed => InlineEditDraftDisposition::Commit,
        };
        let request = match disposition {
            InlineEditDraftDisposition::Commit => {
                Some(InlineEditRequest::Commit(self.commit_request(reason)))
            }
            InlineEditDraftDisposition::Cancel => Some(InlineEditRequest::Cancel(
                self.cancel_request(InlineEditCancelReason::DraftPolicy),
            )),
            InlineEditDraftDisposition::Reject => None,
        };

        InlineEditResolution {
            draft_status,
            disposition,
            request,
        }
    }

    /// Resolves focus loss through the explicit focus-loss policy.
    #[must_use]
    pub fn focus_loss_request(&self) -> Option<InlineEditRequest> {
        match self.focus_loss_policy {
            InlineEditFocusLossPolicy::KeepEditing => None,
            InlineEditFocusLossPolicy::Commit => {
                self.resolve_commit(InlineEditCommitReason::FocusLost)
                    .request
            }
            InlineEditFocusLossPolicy::Cancel => Some(InlineEditRequest::Cancel(
                self.cancel_request(InlineEditCancelReason::FocusLost),
            )),
        }
    }

    /// Resolves one keyboard event into a commit or cancel request.
    #[must_use]
    pub fn keyboard_event_request(&self, event: &KeyEvent) -> Option<InlineEditRequest> {
        if event.state != KeyState::Pressed || event.repeat || !event.modifiers.is_empty() {
            return None;
        }

        match event.key {
            Key::Enter => self.resolve_commit(InlineEditCommitReason::Enter).request,
            Key::Escape => Some(InlineEditRequest::Cancel(
                self.cancel_request(InlineEditCancelReason::Escape),
            )),
            _ => None,
        }
    }

    /// Resolves keyboard input into the first commit or cancel request.
    #[must_use]
    pub fn keyboard_request(&self, input: &UiInput) -> Option<InlineEditRequest> {
        input
            .keyboard
            .events
            .iter()
            .find_map(|event| self.keyboard_event_request(event))
    }
}

/// Classifies an inline edit draft against its initial text.
#[must_use]
pub fn classify_inline_edit_draft(initial_text: &str, draft_text: &str) -> InlineEditDraftStatus {
    if draft_text.trim().is_empty() {
        InlineEditDraftStatus::Empty
    } else if draft_text == initial_text {
        InlineEditDraftStatus::Unchanged
    } else {
        InlineEditDraftStatus::Changed
    }
}

/// Derives the stable text widget ID used by an inline edit session.
#[must_use]
pub fn inline_edit_widget_id(root: WidgetId, item: ItemId) -> WidgetId {
    root.child(("inline-edit", item.raw()))
}
