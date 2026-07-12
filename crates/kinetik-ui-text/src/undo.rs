#![cfg_attr(test, allow(clippy::missing_const_for_thread_local))]

use crate::{TextAffinity, TextEditState, TextSelection};

#[cfg(test)]
use std::cell::Cell;

const MAX_COMBINED_SNAPSHOTS: usize = 128;
const MAX_SNAPSHOT_TEXT_BYTES: usize = 4 * 1024 * 1024;
const MAX_COALESCED_RUN_BYTES: usize = 4096;

#[cfg(test)]
thread_local! {
    static SNAPSHOT_CREATIONS: Cell<usize> = const { Cell::new(0) };
}

/// Text-field-local undo/redo history.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TextUndoStack {
    undo: Vec<EditSnapshot>,
    redo: Vec<EditSnapshot>,
    active_run: Option<CoalescedRun>,
}

impl TextUndoStack {
    /// Creates an empty undo stack.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            undo: Vec::new(),
            redo: Vec::new(),
            active_run: None,
        }
    }

    /// Returns whether one snapshot text payload can be retained.
    pub(crate) const fn can_retain_snapshot_text(text_bytes: usize) -> bool {
        text_bytes <= MAX_SNAPSHOT_TEXT_BYTES
    }

    /// Returns whether an undo target exists without constructing a transfer snapshot.
    pub(crate) fn has_undo_target(&self) -> bool {
        !self.undo.is_empty()
    }

    /// Returns whether a redo target exists without constructing a transfer snapshot.
    pub(crate) fn has_redo_target(&self) -> bool {
        !self.redo.is_empty()
    }

    /// Records one public or semantic-boundary edit as an atomic undo unit.
    pub(crate) fn record_atomic(&mut self, snapshot: EditSnapshot) {
        self.active_run = None;
        self.redo.clear();
        self.push_new_undo(snapshot);
    }

    /// Records an oversized mutation barrier without constructing its snapshot.
    pub(crate) fn record_oversized_barrier(&mut self) {
        self.active_run = None;
        self.undo.clear();
        self.redo.clear();
    }

    /// Continues an eligible ordered run without retaining another full snapshot.
    pub(crate) fn try_continue_run(&mut self, before: HistoryState, edit: CoalescedEdit) -> bool {
        if edit.changed_bytes == 0
            || edit.changed_bytes > MAX_COALESCED_RUN_BYTES
            || !before.selection.is_caret()
            || !edit.expected_after.selection.is_caret()
        {
            return false;
        }
        let Some(run) = self.active_run.as_mut() else {
            return false;
        };
        let Some(changed_bytes) = run.changed_bytes.checked_add(edit.changed_bytes) else {
            return false;
        };
        if run.kind != edit.kind
            || run.expected_before != before
            || changed_bytes > MAX_COALESCED_RUN_BYTES
        {
            return false;
        }

        self.redo.clear();
        run.changed_bytes = changed_bytes;
        run.expected_before = edit.expected_after;
        true
    }

    /// Starts a new eligible ordered run and retains its pre-edit snapshot.
    pub(crate) fn start_run(&mut self, snapshot: EditSnapshot, edit: CoalescedEdit) {
        self.active_run = None;
        self.redo.clear();
        let retained = self.push_new_undo(snapshot);
        if retained
            && edit.changed_bytes > 0
            && edit.changed_bytes <= MAX_COALESCED_RUN_BYTES
            && edit.expected_after.selection.is_caret()
        {
            self.active_run = Some(CoalescedRun {
                kind: edit.kind,
                changed_bytes: edit.changed_bytes,
                expected_before: edit.expected_after,
            });
        }
    }

    /// Ends the active coalescing run without changing retained snapshots.
    pub(crate) fn fence(&mut self) {
        self.active_run = None;
    }

    /// Returns the previous snapshot and stores the current snapshot for redo.
    pub(crate) fn undo(&mut self, current: EditSnapshot) -> Option<EditSnapshot> {
        let current = Self::can_retain_snapshot_text(current.text_bytes()).then_some(current);
        self.undo_with_current(current)
    }

    /// Returns the previous snapshot when the current state cannot be retained.
    pub(crate) fn undo_without_retainable_current(&mut self) -> Option<EditSnapshot> {
        self.undo_with_current(None)
    }

    fn undo_with_current(&mut self, current: Option<EditSnapshot>) -> Option<EditSnapshot> {
        self.fence();
        let previous = self.undo.pop()?;
        if let Some(current) = current {
            self.redo.push(current);
            self.trim_after_undo();
        } else {
            self.redo.clear();
        }
        Some(previous)
    }

    /// Returns the redo snapshot and stores the current snapshot for undo.
    pub(crate) fn redo(&mut self, current: EditSnapshot) -> Option<EditSnapshot> {
        let current = Self::can_retain_snapshot_text(current.text_bytes()).then_some(current);
        self.redo_with_current(current)
    }

    /// Returns the redo snapshot when the current state cannot be retained.
    pub(crate) fn redo_without_retainable_current(&mut self) -> Option<EditSnapshot> {
        self.redo_with_current(None)
    }

    fn redo_with_current(&mut self, current: Option<EditSnapshot>) -> Option<EditSnapshot> {
        self.fence();
        let next = self.redo.pop()?;
        if let Some(current) = current {
            self.undo.push(current);
            self.trim_after_redo();
        } else {
            self.undo.clear();
        }
        Some(next)
    }

    fn push_new_undo(&mut self, snapshot: EditSnapshot) -> bool {
        if snapshot.text_bytes() > MAX_SNAPSHOT_TEXT_BYTES {
            self.undo.clear();
            self.redo.clear();
            return false;
        }
        self.undo.push(snapshot);
        while self.over_budget() {
            self.undo.remove(0);
        }
        true
    }

    fn trim_after_undo(&mut self) {
        while self.over_budget() {
            if !self.undo.is_empty() {
                self.undo.remove(0);
            } else if self.redo.len() > 1 {
                self.redo.remove(0);
            } else {
                break;
            }
        }
    }

    fn trim_after_redo(&mut self) {
        while self.over_budget() {
            if !self.redo.is_empty() {
                self.redo.remove(0);
            } else if self.undo.len() > 1 {
                self.undo.remove(0);
            } else {
                break;
            }
        }
    }

    fn over_budget(&self) -> bool {
        self.undo
            .len()
            .checked_add(self.redo.len())
            .is_none_or(|count| count > MAX_COMBINED_SNAPSHOTS)
            || self
                .retained_text_bytes()
                .is_none_or(|bytes| bytes > MAX_SNAPSHOT_TEXT_BYTES)
    }

    fn retained_text_bytes(&self) -> Option<usize> {
        self.undo
            .iter()
            .chain(&self.redo)
            .try_fold(0_usize, |total, snapshot| {
                total.checked_add(snapshot.text_bytes())
            })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CoalescedEditKind {
    Insert,
    DeleteBackward,
    DeleteForward,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CoalescedEdit {
    kind: CoalescedEditKind,
    changed_bytes: usize,
    expected_after: HistoryState,
}

impl CoalescedEdit {
    pub(crate) const fn new(
        kind: CoalescedEditKind,
        changed_bytes: usize,
        expected_after: HistoryState,
    ) -> Self {
        Self {
            kind,
            changed_bytes,
            expected_after,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct HistoryState {
    text_len: usize,
    selection: TextSelection,
    caret_affinity: TextAffinity,
}

impl HistoryState {
    pub(crate) const fn new(
        text_len: usize,
        selection: TextSelection,
        caret_affinity: TextAffinity,
    ) -> Self {
        Self {
            text_len,
            selection,
            caret_affinity,
        }
    }

    pub(crate) fn from_state(state: &TextEditState) -> Self {
        Self::new(
            state.text.len(),
            state.selection,
            state.caret_position().affinity,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CoalescedRun {
    kind: CoalescedEditKind,
    changed_bytes: usize,
    expected_before: HistoryState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EditSnapshot {
    pub(crate) text: Box<str>,
    pub(crate) selection: TextSelection,
    pub(crate) caret_affinity: TextAffinity,
}

impl EditSnapshot {
    pub(crate) fn from_state(state: &TextEditState) -> Self {
        #[cfg(test)]
        SNAPSHOT_CREATIONS.set(SNAPSHOT_CREATIONS.get() + 1);
        Self {
            text: state.text.as_str().into(),
            selection: state.selection,
            caret_affinity: state.caret_position().affinity,
        }
    }

    fn text_bytes(&self) -> usize {
        self.text.len()
    }
}

#[cfg(test)]
pub(crate) fn reset_snapshot_creation_count() {
    SNAPSHOT_CREATIONS.set(0);
}

#[cfg(test)]
pub(crate) fn snapshot_creation_count() -> usize {
    SNAPSHOT_CREATIONS.get()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TextEditMode;
    use kinetik_ui_core::{
        Key, KeyEvent, KeyState, Modifiers, PhysicalKey, UiInputEvent, WidgetId,
    };

    fn snapshot(text: String) -> EditSnapshot {
        let caret = text.len();
        EditSnapshot {
            text: text.into_boxed_str(),
            selection: TextSelection::new(caret, caret),
            caret_affinity: if caret == 0 {
                TextAffinity::After
            } else {
                TextAffinity::Before
            },
        }
    }

    fn state(text_len: usize, caret: usize) -> HistoryState {
        HistoryState::new(
            text_len,
            TextSelection::new(caret, caret),
            if caret == 0 {
                TextAffinity::After
            } else {
                TextAffinity::Before
            },
        )
    }

    #[test]
    fn combined_count_budget_keeps_the_nearest_128_snapshots() {
        let mut history = TextUndoStack::new();
        for len in 0..=MAX_COMBINED_SNAPSHOTS {
            history.record_atomic(snapshot("x".repeat(len)));
        }

        assert_eq!(history.undo.len(), MAX_COMBINED_SNAPSHOTS);
        assert!(history.redo.is_empty());
        assert_eq!(history.undo.first().map(EditSnapshot::text_bytes), Some(1));
        assert_eq!(
            history.undo.last().map(EditSnapshot::text_bytes),
            Some(MAX_COMBINED_SNAPSHOTS)
        );
        assert!(!history.over_budget());
    }

    #[test]
    fn long_alternating_atomic_history_stays_count_and_byte_bounded() {
        let mut history = TextUndoStack::new();
        for index in 0..10_000 {
            history.record_atomic(snapshot(if index % 2 == 0 {
                "a".to_owned()
            } else {
                "b".to_owned()
            }));
        }

        assert_eq!(history.undo.len(), MAX_COMBINED_SNAPSHOTS);
        assert!(history.redo.is_empty());
        assert_eq!(history.retained_text_bytes(), Some(MAX_COMBINED_SNAPSHOTS));
        assert!(!history.over_budget());
        for (index, snapshot) in history.undo.iter().enumerate() {
            assert_eq!(
                snapshot.text.as_ref(),
                if index % 2 == 0 { "a" } else { "b" }
            );
        }
    }

    #[test]
    fn full_combined_stack_transfers_both_directions_before_branching() {
        let mut history = TextUndoStack::new();
        for index in 0..MAX_COMBINED_SNAPSHOTS {
            history.record_atomic(snapshot(if index % 2 == 0 {
                "a".to_owned()
            } else {
                "b".to_owned()
            }));
        }
        let mut current = snapshot("c".to_owned());

        for index in (64..MAX_COMBINED_SNAPSHOTS).rev() {
            current = history.undo(current).expect("undo target");
            assert_eq!(
                current.text.as_ref(),
                if index % 2 == 0 { "a" } else { "b" }
            );
            assert_eq!(
                history.undo.len() + history.redo.len(),
                MAX_COMBINED_SNAPSHOTS
            );
            assert_eq!(history.retained_text_bytes(), Some(MAX_COMBINED_SNAPSHOTS));
        }
        for _ in 0..32 {
            current = history.redo(current).expect("redo target");
            assert_eq!(
                history.undo.len() + history.redo.len(),
                MAX_COMBINED_SNAPSHOTS
            );
            assert_eq!(history.retained_text_bytes(), Some(MAX_COMBINED_SNAPSHOTS));
        }
        for _ in 0..32 {
            current = history.undo(current).expect("undo target");
            assert_eq!(
                history.undo.len() + history.redo.len(),
                MAX_COMBINED_SNAPSHOTS
            );
            assert_eq!(history.retained_text_bytes(), Some(MAX_COMBINED_SNAPSHOTS));
        }

        history.record_atomic(current);
        assert!(history.redo.is_empty());
        assert_eq!(history.undo.len(), 65);
        assert_eq!(history.retained_text_bytes(), Some(65));
        assert!(!history.over_budget());
    }

    #[test]
    fn snapshots_are_built_only_for_retainable_units_and_real_transfers() {
        let mut empty = TextEditState::new("");
        reset_snapshot_creation_count();
        assert!(!empty.undo());
        assert!(!empty.redo());
        assert_eq!(snapshot_creation_count(), 0);

        let mut barrier = TextEditState::new("x".repeat(MAX_SNAPSHOT_TEXT_BYTES + 1));
        reset_snapshot_creation_count();
        barrier.insert_text("y");
        assert_eq!(snapshot_creation_count(), 0);
        assert!(!barrier.undo());
        assert_eq!(snapshot_creation_count(), 0);

        let target = WidgetId::from_key("field");
        let events = (0..100)
            .map(|index| {
                UiInputEvent::Key(
                    KeyEvent::with_physical_key(
                        Key::Character("x".to_owned()),
                        PhysicalKey::Unidentified,
                        KeyState::Pressed,
                        Modifiers::default(),
                        index != 0,
                    )
                    .with_text("x"),
                )
            })
            .collect::<Vec<_>>();
        let mut retained = TextEditState::new("");
        reset_snapshot_creation_count();
        let _ = retained.apply_ordered_input(&events, target, TextEditMode::SingleLine);
        assert_eq!(snapshot_creation_count(), 1);
        assert!(retained.undo());
        assert_eq!(snapshot_creation_count(), 2);
        assert!(retained.redo());
        assert_eq!(snapshot_creation_count(), 3);
    }

    #[test]
    fn byte_budget_is_inclusive_and_evicts_oldest_first() {
        let mut exact = TextUndoStack::new();
        exact.record_atomic(snapshot("x".repeat(MAX_SNAPSHOT_TEXT_BYTES)));
        assert_eq!(exact.retained_text_bytes(), Some(MAX_SNAPSHOT_TEXT_BYTES));
        assert_eq!(exact.undo.len(), 1);

        let half = MAX_SNAPSHOT_TEXT_BYTES / 2;
        let mut evicted = TextUndoStack::new();
        evicted.record_atomic(snapshot("a".repeat(half)));
        evicted.record_atomic(snapshot("b".repeat(half)));
        evicted.record_atomic(snapshot("c".to_owned()));
        assert_eq!(evicted.undo.len(), 2);
        assert_eq!(evicted.undo[0].text.as_ref(), "b".repeat(half));
        assert_eq!(evicted.undo[1].text.as_ref(), "c");
        assert_eq!(evicted.retained_text_bytes(), Some(half + 1));
        assert!(!evicted.over_budget());

        evicted.record_atomic(snapshot("z".repeat(MAX_SNAPSHOT_TEXT_BYTES + 1)));
        assert!(evicted.undo.is_empty());
        assert!(evicted.redo.is_empty());
        assert!(evicted.active_run.is_none());
    }

    #[test]
    fn snapshot_accounting_uses_utf8_payload_bytes() {
        let fixtures = ["A", "e\u{301}", "👍🏽", "👩‍🚀"];
        let mut history = TextUndoStack::new();
        for fixture in fixtures {
            history.record_atomic(snapshot(fixture.to_owned()));
        }
        assert_eq!(
            history.retained_text_bytes(),
            Some(fixtures.iter().map(|fixture| fixture.len()).sum())
        );
        assert!(!history.over_budget());
    }

    #[test]
    fn run_limit_is_checked_in_utf8_bytes_without_extra_snapshots() {
        let mut history = TextUndoStack::new();
        let first = CoalescedEdit::new(CoalescedEditKind::Insert, 1, state(1, 1));
        history.start_run(snapshot(String::new()), first);
        for len in 1..MAX_COALESCED_RUN_BYTES {
            let next = CoalescedEdit::new(CoalescedEditKind::Insert, 1, state(len + 1, len + 1));
            assert!(history.try_continue_run(state(len, len), next));
        }
        assert_eq!(history.undo.len(), 1);
        assert_eq!(
            history.active_run.as_ref().map(|run| run.changed_bytes),
            Some(MAX_COALESCED_RUN_BYTES)
        );

        let crossing = CoalescedEdit::new(
            CoalescedEditKind::Insert,
            "é".len(),
            state(
                MAX_COALESCED_RUN_BYTES + "é".len(),
                MAX_COALESCED_RUN_BYTES + "é".len(),
            ),
        );
        assert!(!history.try_continue_run(
            state(MAX_COALESCED_RUN_BYTES, MAX_COALESCED_RUN_BYTES),
            crossing
        ));
        assert_eq!(history.undo.len(), 1);
    }

    #[test]
    fn oversized_transfers_are_deliberately_one_way() {
        let oversized = snapshot("x".repeat(MAX_SNAPSHOT_TEXT_BYTES + 1));

        let mut undo = TextUndoStack::new();
        undo.record_atomic(snapshot("before".to_owned()));
        let previous = undo.undo(oversized.clone()).expect("undo target");
        assert_eq!(previous.text.as_ref(), "before");
        assert!(undo.redo.is_empty());
        assert!(undo.redo(snapshot("before".to_owned())).is_none());

        let mut redo = TextUndoStack::new();
        redo.record_atomic(snapshot("before".to_owned()));
        let _ = redo
            .undo(snapshot("after".to_owned()))
            .expect("establish redo");
        let next = redo.redo(oversized).expect("redo target");
        assert_eq!(next.text.as_ref(), "after");
        assert!(redo.undo.is_empty());
        assert!(redo.undo(snapshot("after".to_owned())).is_none());
    }

    #[test]
    fn transfer_eviction_preserves_the_nearest_reverse_target() {
        let mib = 1024 * 1024;
        let mut undo = TextUndoStack {
            undo: vec![
                snapshot("a".repeat(2 * mib)),
                snapshot("b".repeat(mib)),
                snapshot("c".repeat(mib)),
            ],
            redo: Vec::new(),
            active_run: None,
        };
        let previous = undo
            .undo(snapshot("d".repeat(2 * mib)))
            .expect("undo target");
        assert_eq!(previous.text_bytes(), mib);
        assert_eq!(undo.undo.len(), 1);
        assert_eq!(undo.undo[0].text.as_bytes()[0], b'b');
        assert_eq!(undo.redo.len(), 1);
        assert_eq!(undo.redo[0].text.as_bytes()[0], b'd');
        assert!(!undo.over_budget());

        let mut redo = TextUndoStack {
            undo: vec![snapshot("a".repeat(mib))],
            redo: vec![snapshot("f".repeat(2 * mib)), snapshot("n".repeat(mib))],
            active_run: None,
        };
        let next = redo
            .redo(snapshot("c".repeat(2 * mib)))
            .expect("redo target");
        assert_eq!(next.text.as_bytes()[0], b'n');
        assert!(redo.redo.is_empty(), "the farthest redo target is evicted");
        assert_eq!(redo.undo.len(), 2);
        assert_eq!(
            redo.undo.last().map(|snapshot| snapshot.text.as_bytes()[0]),
            Some(b'c')
        );
        assert!(!redo.over_budget());
    }
}
