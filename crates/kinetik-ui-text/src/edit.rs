use kinetik_ui_core::{
    Key, KeyEvent, KeyState, PhysicalKey, PlatformRequest, TextInputEvent, UiInputEvent, WidgetId,
};

use crate::boundary::{
    clamp_boundary, line_range_at_offset, next_boundary, previous_boundary, vertical_line_target,
};
use crate::{EditSnapshot, TextComposition, TextSelection, TextUndoStack};

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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextEditState {
    /// Text buffer.
    pub text: String,
    /// Current selection.
    pub selection: TextSelection,
    /// Active text composition, if any.
    pub composition: Option<TextComposition>,
    undo: TextUndoStack,
}

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
            undo: TextUndoStack::new(),
        }
    }

    /// Returns the caret byte offset.
    #[must_use]
    pub const fn caret(&self) -> usize {
        self.selection.active
    }

    /// Sets a collapsed caret.
    pub fn set_caret(&mut self, caret: usize) {
        let caret = clamp_boundary(&self.text, caret);
        self.selection = TextSelection::new(caret, caret);
    }

    /// Sets a selection after clamping both endpoints to UTF-8 boundaries.
    pub fn set_selection(&mut self, selection: TextSelection) {
        self.selection = selection.clamp_to_text(&self.text);
    }

    /// Selects the full text buffer.
    pub fn select_all(&mut self) {
        self.selection = TextSelection::new(0, self.text.len());
    }

    /// Returns the selected text, if the current selection is non-empty.
    #[must_use]
    pub fn selected_text(&self) -> Option<&str> {
        let range = self.selection.range_in(&self.text);
        (!range.is_empty()).then(|| &self.text[range])
    }

    /// Applies committed text input.
    pub fn insert_text(&mut self, text: &str) {
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
        if !self.selection.is_caret() {
            self.record_undo();
            self.replace_selection("");
        } else if let Some(previous) = previous_boundary(&self.text, self.caret()) {
            self.record_undo();
            self.text.replace_range(previous..self.caret(), "");
            self.set_caret(previous);
        }
    }

    /// Deletes forward from the current selection or caret.
    pub fn delete_forward(&mut self) {
        if !self.selection.is_caret() {
            self.record_undo();
            self.replace_selection("");
        } else if let Some(next) = next_boundary(&self.text, self.caret()) {
            self.record_undo();
            let caret = self.caret();
            self.text.replace_range(caret..next, "");
            self.set_caret(caret);
        }
    }

    /// Moves the caret left.
    pub fn move_left(&mut self) {
        if !self.selection.is_caret() {
            let start = self.selection.range_in(&self.text).start;
            self.set_caret(start);
            return;
        }
        if let Some(previous) = previous_boundary(&self.text, self.caret()) {
            self.set_caret(previous);
        }
    }

    /// Extends the selection left by one character boundary.
    pub fn extend_left(&mut self) {
        if let Some(previous) = previous_boundary(&self.text, self.selection.active) {
            self.selection.active = previous;
            self.selection = self.selection.clamp_to_text(&self.text);
        }
    }

    /// Moves the caret right.
    pub fn move_right(&mut self) {
        if !self.selection.is_caret() {
            let end = self.selection.range_in(&self.text).end;
            self.set_caret(end);
            return;
        }
        if let Some(next) = next_boundary(&self.text, self.caret()) {
            self.set_caret(next);
        }
    }

    /// Extends the selection right by one character boundary.
    pub fn extend_right(&mut self) {
        if let Some(next) = next_boundary(&self.text, self.selection.active) {
            self.selection.active = next;
            self.selection = self.selection.clamp_to_text(&self.text);
        }
    }

    /// Moves the caret to the start of the buffer.
    pub fn move_home(&mut self) {
        self.set_caret(0);
    }

    /// Extends the selection to the start of the buffer.
    pub fn extend_home(&mut self) {
        self.selection.active = 0;
        self.selection = self.selection.clamp_to_text(&self.text);
    }

    /// Moves the caret to the end of the buffer.
    pub fn move_end(&mut self) {
        self.set_caret(self.text.len());
    }

    /// Extends the selection to the end of the buffer.
    pub fn extend_end(&mut self) {
        self.selection.active = self.text.len();
        self.selection = self.selection.clamp_to_text(&self.text);
    }

    /// Moves the caret to the start of the current explicit line.
    pub fn move_line_home(&mut self) {
        self.set_caret(line_range_at_offset(&self.text, self.selection.active).start);
    }

    /// Extends the selection to the start of the current explicit line.
    pub fn extend_line_home(&mut self) {
        self.selection.active = line_range_at_offset(&self.text, self.selection.active).start;
        self.selection = self.selection.clamp_to_text(&self.text);
    }

    /// Moves the caret to the end of the current explicit line.
    pub fn move_line_end(&mut self) {
        self.set_caret(line_range_at_offset(&self.text, self.selection.active).end);
    }

    /// Extends the selection to the end of the current explicit line.
    pub fn extend_line_end(&mut self) {
        self.selection.active = line_range_at_offset(&self.text, self.selection.active).end;
        self.selection = self.selection.clamp_to_text(&self.text);
    }

    /// Moves the caret to the previous explicit line, preserving logical column for this event.
    pub fn move_line_up(&mut self) {
        let target = vertical_line_target(&self.text, self.selection.active, -1);
        self.set_caret(target);
    }

    /// Extends the selection to the previous explicit line.
    pub fn extend_line_up(&mut self) {
        self.selection.active = vertical_line_target(&self.text, self.selection.active, -1);
        self.selection = self.selection.clamp_to_text(&self.text);
    }

    /// Moves the caret to the next explicit line, preserving logical column for this event.
    pub fn move_line_down(&mut self) {
        let target = vertical_line_target(&self.text, self.selection.active, 1);
        self.set_caret(target);
    }

    /// Extends the selection to the next explicit line.
    pub fn extend_line_down(&mut self) {
        self.selection.active = vertical_line_target(&self.text, self.selection.active, 1);
        self.selection = self.selection.clamp_to_text(&self.text);
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
        let range = self.selection.range_in(&self.text);
        self.text.replace_range(range.clone(), replacement);
        self.set_caret(range.start + replacement.len());
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

        if self.apply_ordered_edit_command(event, mode) || has_command_modifier(event) {
            return;
        }
        if self.composition.is_some() {
            return;
        }
        if let Some(text) = event.text.as_deref().and_then(sanitize_hardware_text) {
            self.insert_text(&text);
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
