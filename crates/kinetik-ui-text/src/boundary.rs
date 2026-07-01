use kinetik_ui_core::TextRange;

pub(crate) fn clamp_boundary(text: &str, offset: usize) -> usize {
    let mut offset = offset.min(text.len());
    while !text.is_char_boundary(offset) {
        offset -= 1;
    }
    offset
}

pub(crate) fn previous_boundary(text: &str, offset: usize) -> Option<usize> {
    let offset = offset.min(text.len());
    if offset == 0 {
        return None;
    }

    let mut previous = offset - 1;
    while !text.is_char_boundary(previous) {
        previous -= 1;
    }
    Some(previous)
}

pub(crate) fn next_boundary(text: &str, offset: usize) -> Option<usize> {
    if offset >= text.len() {
        return None;
    }

    let mut next = offset + 1;
    while !text.is_char_boundary(next) {
        next += 1;
    }
    Some(next)
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ExplicitLineCursor<'a> {
    text: &'a str,
    range: core::ops::Range<usize>,
}

impl<'a> ExplicitLineCursor<'a> {
    fn at(text: &'a str, offset: usize) -> Self {
        let offset = clamp_boundary(text, offset);
        let start = text[..offset]
            .rfind('\n')
            .map_or(0, |index| index + '\n'.len_utf8());
        let end = text[offset..]
            .find('\n')
            .map_or(text.len(), |index| offset + index);

        Self {
            text,
            range: start..end,
        }
    }

    fn column_at(&self, offset: usize) -> usize {
        let offset = clamp_boundary(self.text, offset).clamp(self.range.start, self.range.end);
        self.text[self.range.start..offset].chars().count()
    }

    fn offset_at_column(&self, column: usize) -> usize {
        let mut offset = self.range.start;
        let mut remaining = column;
        for character in self.text[self.range.clone()].chars() {
            if remaining == 0 {
                break;
            }
            offset += character.len_utf8();
            remaining -= 1;
        }
        offset.min(self.range.end)
    }

    fn previous_range(&self) -> Option<core::ops::Range<usize>> {
        if self.range.start == 0 {
            return None;
        }

        let end = self.range.start - '\n'.len_utf8();
        let start = self.text[..end]
            .rfind('\n')
            .map_or(0, |index| index + '\n'.len_utf8());
        Some(start..end)
    }

    fn next_range(&self) -> Option<core::ops::Range<usize>> {
        if self.range.end >= self.text.len() {
            return None;
        }

        let start = self.range.end + '\n'.len_utf8();
        let end = self.text[start..]
            .find('\n')
            .map_or(self.text.len(), |index| start + index);
        Some(start..end)
    }

    fn shifted(&self, delta: isize) -> Self {
        let mut cursor = self.clone();
        let mut remaining = delta;

        while remaining < 0 {
            if let Some(range) = cursor.previous_range() {
                cursor.range = range;
            }
            remaining += 1;
        }

        while remaining > 0 {
            if let Some(range) = cursor.next_range() {
                cursor.range = range;
            }
            remaining -= 1;
        }

        cursor
    }
}

pub(crate) fn line_range_at_offset(text: &str, offset: usize) -> core::ops::Range<usize> {
    ExplicitLineCursor::at(text, offset).range
}

pub(crate) fn vertical_line_target(text: &str, offset: usize, delta: isize) -> usize {
    let current = ExplicitLineCursor::at(text, offset);
    let column = current.column_at(offset);
    current.shifted(delta).offset_at_column(column)
}

pub(crate) fn clamp_text_range(text: &str, range: TextRange) -> TextRange {
    TextRange::new(
        clamp_boundary(text, range.start),
        clamp_boundary(text, range.end),
    )
}
