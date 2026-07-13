/// Keyboard navigation input shared by menu-like overlays.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlayNavigationInput {
    /// Move to the previous enabled item, wrapping at the start.
    Previous,
    /// Move to the next enabled item, wrapping at the end.
    Next,
    /// Move to the first enabled item.
    First,
    /// Move to the last enabled item.
    Last,
    /// Activate the highlighted item, like Enter.
    Activate,
    /// Request that the overlay close, like Escape.
    Escape,
}

/// Bounded, caller-clocked typeahead state for menu-like overlays.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeaheadBuffer {
    query: String,
    last_input_millis: Option<u64>,
    timeout_millis: u64,
    max_chars: usize,
}

impl Default for TypeaheadBuffer {
    fn default() -> Self {
        Self::new(1_000, 32)
    }
}

impl TypeaheadBuffer {
    /// Creates typeahead state with a reset timeout and maximum Unicode scalar count.
    #[must_use]
    pub fn new(timeout_millis: u64, max_chars: usize) -> Self {
        Self {
            query: String::new(),
            last_input_millis: None,
            timeout_millis,
            max_chars,
        }
    }

    /// Returns the normalized, case-insensitive query.
    #[must_use]
    pub fn query(&self) -> &str {
        &self.query
    }

    /// Clears the pending query and timestamp.
    pub fn clear(&mut self) {
        self.query.clear();
        self.last_input_millis = None;
    }

    pub(crate) fn update(&mut self, text: &str, now_millis: u64) -> Option<TypeaheadQuery<'_>> {
        if text.is_empty() || self.max_chars == 0 {
            return None;
        }

        let reset = self.last_input_millis.is_none_or(|last| {
            now_millis < last || now_millis.saturating_sub(last) >= self.timeout_millis
        });
        if reset {
            self.query.clear();
        }

        let normalized = text.to_lowercase();
        if normalized.is_empty() {
            return None;
        }

        let repeated = !reset
            && normalized.chars().count() == 1
            && self.query.chars().count() == 1
            && self.query == normalized;
        if !repeated {
            let remaining = self.max_chars.saturating_sub(self.query.chars().count());
            self.query.extend(normalized.chars().take(remaining));
        }
        self.last_input_millis = Some(now_millis);

        Some(TypeaheadQuery {
            text: &self.query,
            advance_from_current: reset || repeated,
        })
    }
}

#[derive(Clone, Copy)]
pub(crate) struct TypeaheadQuery<'a> {
    pub(crate) text: &'a str,
    pub(crate) advance_from_current: bool,
}

pub(crate) fn moved_index(
    candidates: &[usize],
    current: Option<usize>,
    input: OverlayNavigationInput,
) -> Option<usize> {
    let first = *candidates.first()?;
    let last = *candidates.last()?;
    match input {
        OverlayNavigationInput::First => Some(first),
        OverlayNavigationInput::Last => Some(last),
        OverlayNavigationInput::Previous => current
            .and_then(|current| candidates.iter().position(|index| *index == current))
            .map_or(Some(last), |position| {
                Some(candidates[(position + candidates.len() - 1) % candidates.len()])
            }),
        OverlayNavigationInput::Next => current
            .and_then(|current| candidates.iter().position(|index| *index == current))
            .map_or(Some(first), |position| {
                Some(candidates[(position + 1) % candidates.len()])
            }),
        OverlayNavigationInput::Activate | OverlayNavigationInput::Escape => None,
    }
}

pub(crate) fn typeahead_index(
    candidates: &[(usize, &str)],
    current: Option<usize>,
    query: TypeaheadQuery<'_>,
) -> Option<usize> {
    if query.text.is_empty() {
        return None;
    }

    let start = current
        .and_then(|current| candidates.iter().position(|(index, _)| *index == current))
        .map_or(0, |position| {
            if query.advance_from_current {
                (position + 1) % candidates.len().max(1)
            } else {
                position
            }
        });

    (0..candidates.len()).find_map(|offset| {
        let candidate = candidates[(start + offset) % candidates.len()];
        candidate
            .1
            .to_lowercase()
            .starts_with(query.text)
            .then_some(candidate.0)
    })
}
