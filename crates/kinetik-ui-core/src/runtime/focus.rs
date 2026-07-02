use crate::input::{Key, KeyEvent, KeyState, UiInput};
use crate::memory::UiMemory;
use crate::{FocusTraversal, Point, Rect, SemanticRole, SemanticTree, WidgetId};

pub(super) fn apply_escape_text_blur(input: &UiInput, memory: &mut UiMemory) -> bool {
    if memory.text_input_owner().is_none() {
        return false;
    }

    let escape_pressed = input.keyboard.events.iter().any(|event| {
        event.state == KeyState::Pressed && !event.repeat && matches!(event.key, Key::Escape)
    });
    if !escape_pressed {
        return false;
    }

    memory.clear_focus();
    true
}

pub(super) fn apply_window_focus_text_blur(input: &UiInput, memory: &mut UiMemory) -> bool {
    if memory.text_input_owner().is_none() {
        return false;
    }
    if input.window_focused {
        return false;
    }

    memory.clear_focus();
    true
}

pub(super) fn apply_pointer_text_owner_blur(
    input: &UiInput,
    memory: &mut UiMemory,
    semantics: &SemanticTree,
) -> bool {
    let Some(owner) = memory.text_input_owner() else {
        return false;
    };
    if !input.pointer.primary.pressed {
        return false;
    }
    let Some(position) = input.pointer.position else {
        return false;
    };

    let Some(bounds) = text_owner_bounds(owner, semantics) else {
        return false;
    };
    if bounds.contains_point(position) {
        return false;
    }
    if position_hits_disabled_text_entry(position, semantics) {
        return false;
    }

    memory.clear_focus();
    true
}

fn position_hits_disabled_text_entry(position: Point, semantics: &SemanticTree) -> bool {
    semantics.nodes().iter().any(|node| {
        matches!(
            node.role,
            SemanticRole::TextField | SemanticRole::SearchField
        ) && node.state.disabled
            && node.bounds.contains_point(position)
    })
}

fn text_owner_bounds(owner: WidgetId, semantics: &SemanticTree) -> Option<Rect> {
    semantics
        .nodes()
        .iter()
        .find(|node| node.id == owner)
        .map(|node| node.bounds)
}

pub(super) fn apply_keyboard_focus_traversal(
    input: &UiInput,
    memory: &mut UiMemory,
    semantics: &SemanticTree,
) -> bool {
    let directions: Vec<_> = input
        .keyboard
        .events
        .iter()
        .filter_map(tab_focus_direction)
        .collect();
    if directions.is_empty() {
        return false;
    }

    let order = semantics.focus_order();
    if order.is_empty() {
        return false;
    }

    let mut focused = memory.focused().filter(|id| order.contains(id));
    let initial = focused;
    for direction in directions {
        let traversal = FocusTraversal {
            order: order.clone(),
            focused,
        };
        focused = match direction {
            TabFocusDirection::Forward => traversal.next(),
            TabFocusDirection::Backward => traversal.previous(),
        };
    }

    if focused == initial {
        return false;
    }

    memory.set_focused(focused);
    true
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TabFocusDirection {
    Forward,
    Backward,
}

fn tab_focus_direction(event: &KeyEvent) -> Option<TabFocusDirection> {
    if event.state != KeyState::Pressed || event.repeat || !matches!(event.key, Key::Tab) {
        return None;
    }

    if event.modifiers.is_empty() {
        Some(TabFocusDirection::Forward)
    } else if event.modifiers.shift
        && !event.modifiers.ctrl
        && !event.modifiers.alt
        && !event.modifiers.super_key
    {
        Some(TabFocusDirection::Backward)
    } else {
        None
    }
}
