use crate::{TextEditState, TextSelection};

/// Text-field-local undo/redo history.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TextUndoStack {
    undo: Vec<EditSnapshot>,
    redo: Vec<EditSnapshot>,
}

impl TextUndoStack {
    /// Creates an empty undo stack.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            undo: Vec::new(),
            redo: Vec::new(),
        }
    }

    /// Pushes a new undo snapshot and clears redo history.
    pub(crate) fn push(&mut self, snapshot: EditSnapshot) {
        if self.undo.last() != Some(&snapshot) {
            self.undo.push(snapshot);
            self.redo.clear();
        }
    }

    /// Returns the previous snapshot and stores the current snapshot for redo.
    pub(crate) fn undo(&mut self, current: EditSnapshot) -> Option<EditSnapshot> {
        let previous = self.undo.pop()?;
        self.redo.push(current);
        Some(previous)
    }

    /// Returns the redo snapshot and stores the current snapshot for undo.
    pub(crate) fn redo(&mut self, current: EditSnapshot) -> Option<EditSnapshot> {
        let next = self.redo.pop()?;
        self.undo.push(current);
        Some(next)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EditSnapshot {
    pub(crate) text: String,
    pub(crate) selection: TextSelection,
}

impl EditSnapshot {
    pub(crate) fn from_state(state: &TextEditState) -> Self {
        Self {
            text: state.text.clone(),
            selection: state.selection,
        }
    }
}
