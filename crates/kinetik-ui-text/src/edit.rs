use kinetik_ui_core::{
    Key, KeyEvent, KeyState, PhysicalKey, PlatformRequest, TextInputEvent, UiInputEvent, WidgetId,
};

use crate::boundary::{
    clamp_boundary, line_range_at_offset, next_boundary, next_word_boundary, previous_boundary,
    previous_word_boundary, vertical_line_target, word_segment_range_at,
};
use crate::{EditSnapshot, TextAffinity, TextCaret, TextComposition, TextSelection, TextUndoStack};

/// Editing policy used by ordered platform input application.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextEditMode {
    /// Remove line breaks and other control text.
    SingleLine,
    /// Preserve committed line breaks and handle Enter as an edit command.
    MultiLine,
}

/// Side effects and non-replayed command intent from one ordered input pass.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct OrderedTextInputResult {
    /// Platform work emitted at exact ordered event positions.
    pub platform_requests: Vec<PlatformRequest>,
    /// An unmodified, non-repeated Enter press occurred before focus loss.
    pub commit_requested: bool,
    /// An unmodified, non-repeated Escape press occurred before focus loss.
    pub revert_requested: bool,
}

/// Editable single-line text state.
#[derive(Debug, Clone)]
pub struct TextEditState {
    /// Text buffer.
    pub text: String,
    /// Current selection.
    pub selection: TextSelection,
    /// Active text composition, if any.
    pub composition: Option<TextComposition>,
    caret_affinity: TextAffinity,
    affinity_offset: usize,
    undo: TextUndoStack,
}

impl PartialEq for TextEditState {
    fn eq(&self, other: &Self) -> bool {
        self.text == other.text
            && self.selection == other.selection
            && self.composition == other.composition
            && self.caret_position().affinity == other.caret_position().affinity
            && self.undo == other.undo
    }
}

impl Eq for TextEditState {}

impl TextEditState {
    /// Creates text editing state.
    #[must_use]
    pub fn new(text: impl Into<String>) -> Self {
        let text = text.into();
        let caret = text.len();
        Self {
            text,
            selection: TextSelection::new(caret, caret),
            composition: None,
            caret_affinity: if caret == 0 {
                TextAffinity::After
            } else {
                TextAffinity::Before
            },
            affinity_offset: caret,
            undo: TextUndoStack::new(),
        }
    }

    /// Returns the caret byte offset.
    #[must_use]
    pub const fn caret(&self) -> usize {
        self.selection.active
    }

    /// Returns the grapheme-clamped caret offset and its effective affinity.
    #[must_use]
    pub fn caret_position(&self) -> TextCaret {
        let offset = clamp_boundary(&self.text, self.selection.active);
        let affinity =
            if self.affinity_offset == self.selection.active && offset == self.selection.active {
                Self::canonical_affinity(&self.text, TextCaret::new(offset, self.caret_affinity))
                    .affinity
            } else {
                Self::default_affinity(&self.text, offset)
            };
        TextCaret::new(offset, affinity)
    }

    /// Sets a collapsed caret.
    pub fn set_caret(&mut self, caret: usize) {
        let caret = clamp_boundary(&self.text, caret);
        self.set_caret_position(TextCaret::new(
            caret,
            Self::default_affinity(&self.text, caret),
        ));
    }

    /// Sets a collapsed grapheme-safe caret with explicit affinity.
    pub fn set_caret_position(&mut self, caret: TextCaret) {
        let caret = Self::canonical_affinity(&self.text, caret);
        self.selection = TextSelection::new(caret.offset, caret.offset);
        self.set_affinity(caret);
    }

    /// Sets a selection after clamping both endpoints to grapheme boundaries.
    pub fn set_selection(&mut self, selection: TextSelection) {
        let selection = selection.clamp_to_text(&self.text);
        let affinity = Self::default_affinity(&self.text, selection.active);
        self.set_selection_with_affinity(selection, affinity);
    }

    /// Sets a grapheme-safe selection and explicit active-caret affinity.
    pub fn set_selection_with_affinity(
        &mut self,
        selection: TextSelection,
        affinity: TextAffinity,
    ) {
        self.selection = selection.clamp_to_text(&self.text);
        let caret =
            Self::canonical_affinity(&self.text, TextCaret::new(self.selection.active, affinity));
        self.set_affinity(caret);
    }

    /// Selects the full text buffer.
    pub fn select_all(&mut self) {
        self.set_selection(TextSelection::new(0, self.text.len()));
    }

    /// Returns the selected text, if the current selection is non-empty.
    #[must_use]
    pub fn selected_text(&self) -> Option<&str> {
        let range = self.selection.range_in(&self.text);
        (!range.is_empty()).then(|| &self.text[range])
    }

    /// Applies committed text input.
    pub fn insert_text(&mut self, text: &str) {
        self.canonicalize_selection();
        self.record_undo();
        self.composition = None;
        self.replace_selection(text);
    }

    /// Inserts pasted text and records it in the local undo stack.
    pub fn paste_text(&mut self, text: &str) {
        self.insert_text(text);
    }

    /// Removes and returns the current selected text.
    pub fn cut_selection(&mut self) -> Option<String> {
        let selected = self.selected_text()?.to_owned();
        self.insert_text("");
        Some(selected)
    }

    /// Deletes backward from the current selection or caret.
    pub fn backspace(&mut self) {
        self.canonicalize_selection();
        if !self.selection.is_caret() {
            self.record_undo();
            self.replace_selection("");
        } else if let Some(previous) = previous_boundary(&self.text, self.caret()) {
            self.record_undo();
            self.text.replace_range(previous..self.caret(), "");
            self.set_caret_after_edit(previous);
        }
    }

    /// Deletes forward from the current selection or caret.
    pub fn delete_forward(&mut self) {
        self.canonicalize_selection();
        if !self.selection.is_caret() {
            self.record_undo();
            self.replace_selection("");
        } else if let Some(next) = next_boundary(&self.text, self.caret()) {
            self.record_undo();
            let caret = self.caret();
            self.text.replace_range(caret..next, "");
            self.set_caret_after_edit(caret);
        }
    }

    /// Moves the caret backward by one extended grapheme cluster.
    pub fn move_left(&mut self) {
        self.canonicalize_selection();
        if !self.selection.is_caret() {
            let start = self.selection.range_in(&self.text).start;
            self.set_caret_position(TextCaret::new(start, TextAffinity::After));
            return;
        }
        if let Some(previous) = previous_boundary(&self.text, self.caret()) {
            self.set_caret_position(TextCaret::new(previous, TextAffinity::After));
        }
    }

    /// Extends the selection backward by one extended grapheme cluster.
    pub fn extend_left(&mut self) {
        self.canonicalize_selection();
        if let Some(previous) = previous_boundary(&self.text, self.selection.active) {
            self.set_active(previous, TextAffinity::After);
        }
    }

    /// Moves the caret forward by one extended grapheme cluster.
    pub fn move_right(&mut self) {
        self.canonicalize_selection();
        if !self.selection.is_caret() {
            let end = self.selection.range_in(&self.text).end;
            self.set_caret_position(TextCaret::new(end, TextAffinity::Before));
            return;
        }
        if let Some(next) = next_boundary(&self.text, self.caret()) {
            self.set_caret_position(TextCaret::new(next, TextAffinity::Before));
        }
    }

    /// Extends the selection forward by one extended grapheme cluster.
    pub fn extend_right(&mut self) {
        self.canonicalize_selection();
        if let Some(next) = next_boundary(&self.text, self.selection.active) {
            self.set_active(next, TextAffinity::Before);
        }
    }

    /// Moves backward across whitespace and one full-buffer UAX #29 segment.
    pub fn move_word_left(&mut self) {
        self.canonicalize_selection();
        if !self.selection.is_caret() {
            let start = self.selection.range_in(&self.text).start;
            self.set_caret_position(TextCaret::new(start, TextAffinity::After));
            return;
        }

        let target = previous_word_boundary(&self.text, self.caret());
        if target != self.caret() {
            self.set_caret_position(TextCaret::new(target, TextAffinity::After));
        }
    }

    /// Moves forward across one UAX #29 segment and following whitespace.
    pub fn move_word_right(&mut self) {
        self.canonicalize_selection();
        if !self.selection.is_caret() {
            let end = self.selection.range_in(&self.text).end;
            self.set_caret_position(TextCaret::new(end, TextAffinity::Before));
            return;
        }

        let target = next_word_boundary(&self.text, self.caret());
        if target != self.caret() {
            self.set_caret_position(TextCaret::new(target, TextAffinity::Before));
        }
    }

    /// Extends the selection left using [`Self::move_word_left`] boundary policy.
    pub fn extend_word_left(&mut self) {
        self.canonicalize_selection();
        let target = previous_word_boundary(&self.text, self.selection.active);
        if target != self.selection.active {
            self.set_active(target, TextAffinity::After);
        }
    }

    /// Extends the selection right using [`Self::move_word_right`] boundary policy.
    pub fn extend_word_right(&mut self) {
        self.canonicalize_selection();
        let target = next_word_boundary(&self.text, self.selection.active);
        if target != self.selection.active {
            self.set_active(target, TextAffinity::Before);
        }
    }

    /// Deletes the current selection or the span to [`Self::move_word_left`]'s target.
    pub fn backspace_word(&mut self) {
        self.canonicalize_selection();
        if !self.selection.is_caret() {
            self.record_undo();
            self.replace_selection("");
            return;
        }

        let caret = self.caret();
        let target = previous_word_boundary(&self.text, caret);
        if target == caret {
            return;
        }

        self.record_undo();
        self.text.replace_range(target..caret, "");
        self.set_caret_after_edit(target);
    }

    /// Deletes the current selection or the span to [`Self::move_word_right`]'s target.
    pub fn delete_word_forward(&mut self) {
        self.canonicalize_selection();
        if !self.selection.is_caret() {
            self.record_undo();
            self.replace_selection("");
            return;
        }

        let caret = self.caret();
        let target = next_word_boundary(&self.text, caret);
        if target == caret {
            return;
        }

        self.record_undo();
        self.text.replace_range(caret..target, "");
        self.set_caret_after_edit(caret);
    }

    /// Selects the full-buffer UAX #29 segment containing the clamped offset.
    pub fn select_word_at(&mut self, offset: usize) {
        let range = word_segment_range_at(&self.text, offset);
        self.set_selection(TextSelection::new(range.start, range.end));
    }

    /// Moves the caret to the start of the buffer.
    pub fn move_home(&mut self) {
        self.canonicalize_selection();
        if self.caret() != 0 || !self.selection.is_caret() {
            self.set_caret_position(TextCaret::new(0, TextAffinity::After));
        }
    }

    /// Extends the selection to the start of the buffer.
    pub fn extend_home(&mut self) {
        self.canonicalize_selection();
        if self.selection.active != 0 {
            self.set_active(0, TextAffinity::After);
        }
    }

    /// Moves the caret to the end of the buffer.
    pub fn move_end(&mut self) {
        self.canonicalize_selection();
        if self.caret() != self.text.len() || !self.selection.is_caret() {
            self.set_caret_position(TextCaret::new(self.text.len(), TextAffinity::Before));
        }
    }

    /// Extends the selection to the end of the buffer.
    pub fn extend_end(&mut self) {
        self.canonicalize_selection();
        if self.selection.active != self.text.len() {
            self.set_active(self.text.len(), TextAffinity::Before);
        }
    }

    /// Moves the caret to the start of the current explicit line.
    pub fn move_line_home(&mut self) {
        self.canonicalize_selection();
        let target = line_range_at_offset(&self.text, self.selection.active).start;
        if target != self.caret() || !self.selection.is_caret() {
            self.set_caret_position(TextCaret::new(target, TextAffinity::After));
        }
    }

    /// Extends the selection to the start of the current explicit line.
    pub fn extend_line_home(&mut self) {
        self.canonicalize_selection();
        let target = line_range_at_offset(&self.text, self.selection.active).start;
        if target != self.selection.active {
            self.set_active(target, TextAffinity::After);
        }
    }

    /// Moves the caret to the end of the current explicit line.
    pub fn move_line_end(&mut self) {
        self.canonicalize_selection();
        let target = line_range_at_offset(&self.text, self.selection.active).end;
        if target != self.caret() || !self.selection.is_caret() {
            self.set_caret_position(TextCaret::new(target, TextAffinity::Before));
        }
    }

    /// Extends the selection to the end of the current explicit line.
    pub fn extend_line_end(&mut self) {
        self.canonicalize_selection();
        let target = line_range_at_offset(&self.text, self.selection.active).end;
        if target != self.selection.active {
            self.set_active(target, TextAffinity::Before);
        }
    }

    /// Moves the caret to the previous explicit line, preserving logical column for this event.
    pub fn move_line_up(&mut self) {
        self.canonicalize_selection();
        let target = vertical_line_target(&self.text, self.selection.active, -1);
        if target != self.caret() || !self.selection.is_caret() {
            self.set_caret(target);
        }
    }

    /// Extends the selection to the previous explicit line.
    pub fn extend_line_up(&mut self) {
        self.canonicalize_selection();
        let target = vertical_line_target(&self.text, self.selection.active, -1);
        if target != self.selection.active {
            let affinity = Self::default_affinity(&self.text, target);
            self.set_active(target, affinity);
        }
    }

    /// Moves the caret to the next explicit line, preserving logical column for this event.
    pub fn move_line_down(&mut self) {
        self.canonicalize_selection();
        let target = vertical_line_target(&self.text, self.selection.active, 1);
        if target != self.caret() || !self.selection.is_caret() {
            self.set_caret(target);
        }
    }

    /// Extends the selection to the next explicit line.
    pub fn extend_line_down(&mut self) {
        self.canonicalize_selection();
        let target = vertical_line_target(&self.text, self.selection.active, 1);
        if target != self.selection.active {
            let affinity = Self::default_affinity(&self.text, target);
            self.set_active(target, affinity);
        }
    }

    /// Applies one authoritative ordered platform-input stream.
    ///
    /// Keyboard edit commands run before text carried by the same hardware key.
    /// Clipboard effects are returned at their exact event positions. Once a
    /// focus-loss event is observed, composition ends and later editing events
    /// in the frame are ignored even if focus subsequently returns.
    #[must_use]
    pub fn apply_ordered_input(
        &mut self,
        events: &[UiInputEvent],
        target: WidgetId,
        mode: TextEditMode,
    ) -> Vec<PlatformRequest> {
        self.apply_ordered_input_with_result(events, target, mode)
            .platform_requests
    }

    /// Applies one ordered stream and returns its platform effects and command intent.
    ///
    /// Callers must validate the stream before invoking this method. The result
    /// belongs to this one application and must not be reconstructed from legacy
    /// projections after the stream has been claimed.
    #[must_use]
    pub fn apply_ordered_input_with_result(
        &mut self,
        events: &[UiInputEvent],
        target: WidgetId,
        mode: TextEditMode,
    ) -> OrderedTextInputResult {
        let mut result = OrderedTextInputResult::default();
        let mut accepts_editing = true;
        for event in events {
            match event {
                UiInputEvent::WindowFocusChanged(false) => {
                    self.composition = None;
                    accepts_editing = false;
                }
                UiInputEvent::WindowFocusChanged(true) => {}
                _ if !accepts_editing => {}
                UiInputEvent::Key(event) => {
                    if event.state == KeyState::Pressed
                        && !event.repeat
                        && event.modifiers.is_empty()
                    {
                        match event.key {
                            Key::Enter => result.commit_requested = true,
                            Key::Escape => result.revert_requested = true,
                            _ => {}
                        }
                    }
                    self.apply_ordered_key(event, target, mode, &mut result.platform_requests);
                }
                UiInputEvent::Text(event) => self.apply_ordered_text(event, mode),
                UiInputEvent::ClipboardText(clipboard) if clipboard.target == target => {
                    if let Some(text) = sanitize_clipboard_text(&clipboard.text, mode) {
                        self.paste_text(&text);
                    }
                }
                UiInputEvent::PointerMoved { .. }
                | UiInputEvent::PointerLeft
                | UiInputEvent::PointerButton { .. }
                | UiInputEvent::PointerReleaseAll { .. }
                | UiInputEvent::Wheel { .. }
                | UiInputEvent::ModifiersChanged(_)
                | UiInputEvent::ClipboardText(_)
                | UiInputEvent::ImeEnabled(_) => {}
            }
        }
        result
    }

    /// Applies the non-mutating subset of one authoritative ordered input stream.
    ///
    /// Read-only input may move or extend the selection, select all, and copy a
    /// non-empty selection. Text insertion, composition, deletion, cut, paste,
    /// undo, redo, and commit/revert intent are ignored. A focus-loss event
    /// fences every later event in the supplied stream, even after focus gain.
    /// Callers that split one authoritative stream into pointer-interleaved
    /// chunks must retain that focus-loss fence across calls; each invocation
    /// otherwise represents a new independent stream.
    #[must_use]
    pub fn apply_read_only_ordered_input(
        &mut self,
        events: &[UiInputEvent],
        mode: TextEditMode,
    ) -> Vec<PlatformRequest> {
        self.composition = None;
        let mut platform_requests = Vec::new();
        let mut accepts_input = true;

        for event in events {
            match event {
                UiInputEvent::WindowFocusChanged(false) => accepts_input = false,
                UiInputEvent::WindowFocusChanged(true) => {}
                _ if !accepts_input => {}
                UiInputEvent::Key(event) => {
                    self.apply_read_only_key(event, mode, &mut platform_requests);
                }
                UiInputEvent::PointerMoved { .. }
                | UiInputEvent::PointerLeft
                | UiInputEvent::PointerButton { .. }
                | UiInputEvent::PointerReleaseAll { .. }
                | UiInputEvent::Wheel { .. }
                | UiInputEvent::ModifiersChanged(_)
                | UiInputEvent::Text(_)
                | UiInputEvent::ClipboardText(_)
                | UiInputEvent::ImeEnabled(_) => {}
            }
        }

        platform_requests
    }

    /// Applies legacy separate text and key slices.
    ///
    /// This compatibility helper cannot recover interleaving. Production text
    /// widgets use [`Self::apply_ordered_input`].
    pub fn apply_input(&mut self, text_events: &[TextInputEvent], key_events: &[KeyEvent]) {
        for event in text_events {
            match event {
                TextInputEvent::CompositionStart => {
                    self.composition = Some(TextComposition::default());
                }
                TextInputEvent::Composition { text, selection } => {
                    self.composition = Some(TextComposition::new(text.clone(), *selection));
                }
                TextInputEvent::Commit(text) => {
                    self.insert_text(text);
                }
                TextInputEvent::CompositionEnd => {
                    self.composition = None;
                }
            }
        }
        for event in key_events {
            if event.state != KeyState::Pressed {
                continue;
            }
            if self.apply_shortcut_event(event) {
                continue;
            }
            if self.apply_word_edit_command(event) {
                continue;
            }
            match event.key {
                Key::Backspace => self.backspace(),
                Key::Delete => self.delete_forward(),
                Key::ArrowLeft if event.modifiers.shift => self.extend_left(),
                Key::ArrowRight if event.modifiers.shift => self.extend_right(),
                Key::Home if event.modifiers.shift => self.extend_home(),
                Key::End if event.modifiers.shift => self.extend_end(),
                Key::ArrowLeft => self.move_left(),
                Key::ArrowRight => self.move_right(),
                Key::Home => self.move_home(),
                Key::End => self.move_end(),
                _ => {}
            }
        }
    }

    /// Applies legacy separate slices using explicit-line multiline navigation.
    ///
    /// This compatibility helper cannot recover interleaving. Production text
    /// widgets use [`Self::apply_ordered_input`].
    pub fn apply_multiline_input(
        &mut self,
        text_events: &[TextInputEvent],
        key_events: &[KeyEvent],
    ) {
        for event in text_events {
            match event {
                TextInputEvent::CompositionStart => {
                    self.composition = Some(TextComposition::default());
                }
                TextInputEvent::Composition { text, selection } => {
                    self.composition = Some(TextComposition::new(text.clone(), *selection));
                }
                TextInputEvent::Commit(text) => {
                    self.insert_text(text);
                }
                TextInputEvent::CompositionEnd => {
                    self.composition = None;
                }
            }
        }
        for event in key_events {
            if event.state != KeyState::Pressed {
                continue;
            }
            if self.apply_shortcut_event(event) {
                continue;
            }
            if self.apply_word_edit_command(event) {
                continue;
            }
            match event.key {
                Key::Backspace => self.backspace(),
                Key::Delete => self.delete_forward(),
                Key::ArrowLeft if event.modifiers.shift => self.extend_left(),
                Key::ArrowRight if event.modifiers.shift => self.extend_right(),
                Key::ArrowUp if event.modifiers.shift => self.extend_line_up(),
                Key::ArrowDown if event.modifiers.shift => self.extend_line_down(),
                Key::Home if event.modifiers.shift => self.extend_line_home(),
                Key::End if event.modifiers.shift => self.extend_line_end(),
                Key::ArrowLeft => self.move_left(),
                Key::ArrowRight => self.move_right(),
                Key::ArrowUp => self.move_line_up(),
                Key::ArrowDown => self.move_line_down(),
                Key::Home => self.move_line_home(),
                Key::End => self.move_line_end(),
                _ => {}
            }
        }
    }

    /// Performs local undo.
    pub fn undo(&mut self) -> bool {
        if let Some(previous) = self.undo.undo(EditSnapshot::from_state(self)) {
            self.restore(previous);
            true
        } else {
            false
        }
    }

    /// Performs local redo.
    pub fn redo(&mut self) -> bool {
        if let Some(next) = self.undo.redo(EditSnapshot::from_state(self)) {
            self.restore(next);
            true
        } else {
            false
        }
    }

    fn replace_selection(&mut self, replacement: &str) {
        self.canonicalize_selection();
        let range = self.selection.range_in(&self.text);
        self.text.replace_range(range.clone(), replacement);
        self.set_caret_after_edit(range.start + replacement.len());
    }

    fn canonicalize_selection(&mut self) {
        let caret = self.caret_position();
        self.selection = self.selection.clamp_to_text(&self.text);
        self.set_affinity(Self::canonical_affinity(
            &self.text,
            TextCaret::new(self.selection.active, caret.affinity),
        ));
    }

    fn set_active(&mut self, active: usize, affinity: TextAffinity) {
        let active = clamp_boundary(&self.text, active);
        self.selection.active = active;
        self.set_affinity(Self::canonical_affinity(
            &self.text,
            TextCaret::new(active, affinity),
        ));
    }

    fn set_caret_after_edit(&mut self, offset: usize) {
        self.set_caret_position(TextCaret::new(offset, TextAffinity::Before));
    }

    fn set_affinity(&mut self, caret: TextCaret) {
        self.caret_affinity = caret.affinity;
        self.affinity_offset = caret.offset;
    }

    fn default_affinity(text: &str, offset: usize) -> TextAffinity {
        if offset == 0 {
            TextAffinity::After
        } else if offset >= text.len() {
            TextAffinity::Before
        } else {
            TextAffinity::After
        }
    }

    fn canonical_affinity(text: &str, caret: TextCaret) -> TextCaret {
        let offset = clamp_boundary(text, caret.offset);
        let affinity = if offset == 0 {
            TextAffinity::After
        } else if offset == text.len() {
            TextAffinity::Before
        } else {
            caret.affinity
        };
        TextCaret::new(offset, affinity)
    }

    fn apply_ordered_text(&mut self, event: &TextInputEvent, mode: TextEditMode) {
        match event {
            TextInputEvent::CompositionStart => {
                self.composition = Some(TextComposition::default());
            }
            TextInputEvent::Composition { text, selection } => {
                self.composition = Some(TextComposition::new(
                    sanitize_composition_text(text, mode),
                    *selection,
                ));
            }
            TextInputEvent::Commit(text) => {
                if let Some(text) = sanitize_text_commit(text, mode) {
                    self.insert_text(&text);
                } else {
                    self.composition = None;
                }
            }
            TextInputEvent::CompositionEnd => {
                self.composition = None;
            }
        }
    }

    fn apply_ordered_key(
        &mut self,
        event: &KeyEvent,
        target: WidgetId,
        mode: TextEditMode,
        platform_requests: &mut Vec<PlatformRequest>,
    ) {
        if event.state != KeyState::Pressed {
            return;
        }
        if self.apply_clipboard_shortcut(event, target, platform_requests)
            || self.apply_shortcut_event(event)
        {
            return;
        }

        if self.apply_word_edit_command(event)
            || self.apply_ordered_edit_command(event, mode)
            || has_command_modifier(event)
        {
            return;
        }
        if self.composition.is_some() {
            return;
        }
        if let Some(text) = event.text.as_deref().and_then(sanitize_hardware_text) {
            self.insert_text(&text);
        }
    }

    fn apply_read_only_key(
        &mut self,
        event: &KeyEvent,
        mode: TextEditMode,
        platform_requests: &mut Vec<PlatformRequest>,
    ) {
        if event.state != KeyState::Pressed {
            return;
        }

        if let Some(shortcut) = clipboard_shortcut(event) {
            if shortcut == ClipboardShortcut::Copy
                && let Some(selected) = self.selected_text()
            {
                platform_requests.push(PlatformRequest::CopyToClipboard(selected.to_owned()));
            }
            return;
        }

        if has_command_modifier(event)
            && let Key::Character(character) = &event.key
            && character.eq_ignore_ascii_case("a")
        {
            self.select_all();
            return;
        }

        if has_word_modifier(event) {
            let handled = match event.key {
                Key::ArrowLeft if event.modifiers.shift => {
                    self.extend_word_left();
                    true
                }
                Key::ArrowRight if event.modifiers.shift => {
                    self.extend_word_right();
                    true
                }
                Key::ArrowLeft => {
                    self.move_word_left();
                    true
                }
                Key::ArrowRight => {
                    self.move_word_right();
                    true
                }
                _ => false,
            };
            if handled {
                return;
            }
        }

        match event.key {
            Key::ArrowLeft if event.modifiers.shift => self.extend_left(),
            Key::ArrowRight if event.modifiers.shift => self.extend_right(),
            Key::ArrowUp if event.modifiers.shift && mode == TextEditMode::MultiLine => {
                self.extend_line_up();
            }
            Key::ArrowDown if event.modifiers.shift && mode == TextEditMode::MultiLine => {
                self.extend_line_down();
            }
            Key::Home if event.modifiers.shift && mode == TextEditMode::MultiLine => {
                self.extend_line_home();
            }
            Key::End if event.modifiers.shift && mode == TextEditMode::MultiLine => {
                self.extend_line_end();
            }
            Key::Home if event.modifiers.shift => self.extend_home(),
            Key::End if event.modifiers.shift => self.extend_end(),
            Key::ArrowLeft => self.move_left(),
            Key::ArrowRight => self.move_right(),
            Key::ArrowUp if mode == TextEditMode::MultiLine => self.move_line_up(),
            Key::ArrowDown if mode == TextEditMode::MultiLine => self.move_line_down(),
            Key::Home if mode == TextEditMode::MultiLine => self.move_line_home(),
            Key::End if mode == TextEditMode::MultiLine => self.move_line_end(),
            Key::Home => self.move_home(),
            Key::End => self.move_end(),
            Key::Character(_)
            | Key::Enter
            | Key::Escape
            | Key::Tab
            | Key::Backspace
            | Key::Delete
            | Key::Insert
            | Key::PageUp
            | Key::PageDown
            | Key::ArrowUp
            | Key::ArrowDown
            | Key::Space
            | Key::Function(_)
            | Key::Unidentified => {}
        }
    }

    fn apply_ordered_edit_command(&mut self, event: &KeyEvent, mode: TextEditMode) -> bool {
        match event.key {
            Key::Backspace => {
                self.backspace();
                true
            }
            Key::Delete => {
                self.delete_forward();
                true
            }
            Key::ArrowLeft if event.modifiers.shift => {
                self.extend_left();
                true
            }
            Key::ArrowRight if event.modifiers.shift => {
                self.extend_right();
                true
            }
            Key::ArrowUp if event.modifiers.shift && mode == TextEditMode::MultiLine => {
                self.extend_line_up();
                true
            }
            Key::ArrowDown if event.modifiers.shift && mode == TextEditMode::MultiLine => {
                self.extend_line_down();
                true
            }
            Key::Home if event.modifiers.shift && mode == TextEditMode::MultiLine => {
                self.extend_line_home();
                true
            }
            Key::End if event.modifiers.shift && mode == TextEditMode::MultiLine => {
                self.extend_line_end();
                true
            }
            Key::Home if event.modifiers.shift => {
                self.extend_home();
                true
            }
            Key::End if event.modifiers.shift => {
                self.extend_end();
                true
            }
            Key::ArrowLeft => {
                self.move_left();
                true
            }
            Key::ArrowRight => {
                self.move_right();
                true
            }
            Key::ArrowUp if mode == TextEditMode::MultiLine => {
                self.move_line_up();
                true
            }
            Key::ArrowDown if mode == TextEditMode::MultiLine => {
                self.move_line_down();
                true
            }
            Key::Home if mode == TextEditMode::MultiLine => {
                self.move_line_home();
                true
            }
            Key::End if mode == TextEditMode::MultiLine => {
                self.move_line_end();
                true
            }
            Key::Home => {
                self.move_home();
                true
            }
            Key::End => {
                self.move_end();
                true
            }
            Key::Enter if mode == TextEditMode::MultiLine && event.modifiers.is_empty() => {
                self.insert_text("\n");
                true
            }
            Key::Enter
            | Key::Escape
            | Key::Tab
            | Key::Insert
            | Key::PageUp
            | Key::PageDown
            | Key::ArrowUp
            | Key::ArrowDown
            | Key::Function(_) => true,
            Key::Character(_) | Key::Space | Key::Unidentified => false,
        }
    }

    fn apply_word_edit_command(&mut self, event: &KeyEvent) -> bool {
        if !has_word_modifier(event) {
            return false;
        }

        match event.key {
            Key::ArrowLeft if event.modifiers.shift => self.extend_word_left(),
            Key::ArrowRight if event.modifiers.shift => self.extend_word_right(),
            Key::ArrowLeft => self.move_word_left(),
            Key::ArrowRight => self.move_word_right(),
            Key::Backspace => self.backspace_word(),
            Key::Delete => self.delete_word_forward(),
            _ => return false,
        }
        true
    }

    fn apply_clipboard_shortcut(
        &mut self,
        event: &KeyEvent,
        target: WidgetId,
        platform_requests: &mut Vec<PlatformRequest>,
    ) -> bool {
        let Some(shortcut) = clipboard_shortcut(event) else {
            return false;
        };
        match shortcut {
            ClipboardShortcut::Copy => {
                if let Some(selected) = self.selected_text() {
                    platform_requests.push(PlatformRequest::CopyToClipboard(selected.to_owned()));
                }
            }
            ClipboardShortcut::Cut => {
                if let Some(selected) = self.cut_selection() {
                    platform_requests.push(PlatformRequest::CopyToClipboard(selected));
                }
            }
            ClipboardShortcut::Paste => {
                platform_requests.push(PlatformRequest::RequestClipboardText { target });
            }
        }
        true
    }

    fn apply_shortcut_event(&mut self, event: &KeyEvent) -> bool {
        if !has_command_modifier(event) {
            return false;
        }
        let Key::Character(character) = &event.key else {
            return false;
        };
        match character.to_ascii_lowercase().as_str() {
            "a" => {
                self.select_all();
                true
            }
            "z" => {
                self.undo();
                true
            }
            "y" => {
                self.redo();
                true
            }
            _ => false,
        }
    }

    fn record_undo(&mut self) {
        self.undo.push(EditSnapshot::from_state(self));
    }

    fn restore(&mut self, snapshot: EditSnapshot) {
        self.text = snapshot.text;
        self.selection = snapshot.selection;
        self.set_affinity(Self::canonical_affinity(
            &self.text,
            TextCaret::new(self.selection.active, snapshot.caret_affinity),
        ));
        self.composition = None;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ClipboardShortcut {
    Copy,
    Cut,
    Paste,
}

fn clipboard_shortcut(event: &KeyEvent) -> Option<ClipboardShortcut> {
    if event.state != KeyState::Pressed
        || event.repeat
        || event.modifiers.alt
        || !has_command_modifier(event)
    {
        return None;
    }
    if let Key::Character(character) = &event.key {
        match character.to_ascii_lowercase().as_str() {
            "c" => return Some(ClipboardShortcut::Copy),
            "x" => return Some(ClipboardShortcut::Cut),
            "v" => return Some(ClipboardShortcut::Paste),
            _ => {}
        }
    }
    match event.physical_key {
        PhysicalKey::KeyC => Some(ClipboardShortcut::Copy),
        PhysicalKey::KeyX => Some(ClipboardShortcut::Cut),
        PhysicalKey::KeyV => Some(ClipboardShortcut::Paste),
        _ => None,
    }
}

fn has_command_modifier(event: &KeyEvent) -> bool {
    event.modifiers.super_key || (event.modifiers.ctrl && !event.modifiers.alt)
}

fn has_word_modifier(event: &KeyEvent) -> bool {
    !event.modifiers.super_key && (event.modifiers.ctrl ^ event.modifiers.alt)
}

fn sanitize_hardware_text(text: &str) -> Option<String> {
    let text = text
        .chars()
        .filter(|character| !character.is_control())
        .collect::<String>();
    (!text.is_empty()).then_some(text)
}

fn sanitize_text_commit(text: &str, mode: TextEditMode) -> Option<String> {
    let text = match mode {
        TextEditMode::SingleLine => text
            .chars()
            .filter(|character| !character.is_control())
            .collect(),
        TextEditMode::MultiLine => text.to_owned(),
    };
    (!text.is_empty()).then_some(text)
}

fn sanitize_clipboard_text(text: &str, mode: TextEditMode) -> Option<String> {
    let text = match mode {
        TextEditMode::SingleLine => text
            .chars()
            .filter(|character| !character.is_control())
            .collect(),
        TextEditMode::MultiLine => text.replace("\r\n", "\n").replace('\r', "\n"),
    };
    (!text.is_empty()).then_some(text)
}

fn sanitize_composition_text(text: &str, mode: TextEditMode) -> String {
    if mode == TextEditMode::MultiLine {
        return text.to_owned();
    }
    text.chars()
        .map(|character| {
            if character.is_control() {
                ' '
            } else {
                character
            }
        })
        .collect()
}
