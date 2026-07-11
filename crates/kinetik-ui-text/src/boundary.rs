use kinetik_ui_core::TextRange;
use unicode_segmentation::UnicodeSegmentation;

pub(crate) fn clamp_boundary(text: &str, offset: usize) -> usize {
    let offset = offset.min(text.len());
    if offset == text.len() {
        return offset;
    }

    text.grapheme_indices(true)
        .map(|(index, _)| index)
        .take_while(|index| *index <= offset)
        .last()
        .unwrap_or(0)
}

pub(crate) fn previous_boundary(text: &str, offset: usize) -> Option<usize> {
    let requested = offset.min(text.len());
    let offset = clamp_boundary(text, requested);
    if requested != offset {
        return Some(offset);
    }
    if offset == 0 {
        return None;
    }

    text[..offset]
        .grapheme_indices(true)
        .next_back()
        .map(|(index, _)| index)
}

pub(crate) fn next_boundary(text: &str, offset: usize) -> Option<usize> {
    let offset = clamp_boundary(text, offset);
    if offset >= text.len() {
        return None;
    }

    text[offset..]
        .graphemes(true)
        .next()
        .map(|grapheme| offset + grapheme.len())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct WordSegment {
    start: usize,
    end: usize,
    whitespace: bool,
}

fn word_segments(text: &str) -> Vec<WordSegment> {
    text.split_word_bound_indices()
        .map(|(start, segment)| WordSegment {
            start,
            end: start + segment.len(),
            whitespace: !segment.is_empty() && segment.chars().all(char::is_whitespace),
        })
        .collect()
}

pub(crate) fn previous_word_boundary(text: &str, offset: usize) -> usize {
    let cursor = clamp_boundary(text, offset);
    if cursor == 0 {
        return 0;
    }
    let segments = word_segments(text);
    let containing = segments
        .iter()
        .position(|segment| segment.start < cursor && cursor < segment.end);

    if let Some(index) = containing
        && !segments[index].whitespace
    {
        return segments[index].start;
    }

    let mut preceding = containing.map_or_else(
        || segments.partition_point(|segment| segment.end <= cursor),
        |index| index + 1,
    );
    while preceding > 0 && segments[preceding - 1].whitespace {
        preceding -= 1;
    }

    segments
        .get(preceding.saturating_sub(1))
        .map_or(0, |segment| segment.start)
}

pub(crate) fn next_word_boundary(text: &str, offset: usize) -> usize {
    let cursor = clamp_boundary(text, offset);
    if cursor >= text.len() {
        return text.len();
    }
    let segments = word_segments(text);
    let Some(mut index) = segments
        .iter()
        .position(|segment| segment.start <= cursor && cursor < segment.end)
    else {
        return text.len();
    };

    index += 1;
    while index < segments.len() && segments[index].whitespace {
        index += 1;
    }

    segments
        .get(index.saturating_sub(1))
        .map_or(text.len(), |segment| segment.end)
}

pub(crate) fn word_segment_range_at(text: &str, offset: usize) -> core::ops::Range<usize> {
    if text.is_empty() {
        return 0..0;
    }

    let offset = clamp_boundary(text, offset);
    let segments = word_segments(text);
    if offset == text.len() {
        return segments
            .last()
            .map_or(0..0, |segment| segment.start..segment.end);
    }

    segments
        .into_iter()
        .find(|segment| segment.start <= offset && offset < segment.end)
        .map_or(0..0, |segment| segment.start..segment.end)
}

fn line_end_before_newline(text: &str, newline: usize) -> usize {
    if newline > 0 && text.as_bytes()[newline - 1] == b'\r' {
        newline - 1
    } else {
        newline
    }
}

fn line_end_from(text: &str, start: usize) -> usize {
    text[start..].find('\n').map_or(text.len(), |relative| {
        line_end_before_newline(text, start + relative)
    })
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
        let end = line_end_from(text, offset);

        Self {
            text,
            range: start..end,
        }
    }

    fn column_at(&self, offset: usize) -> usize {
        let offset = clamp_boundary(self.text, offset).clamp(self.range.start, self.range.end);
        self.text[self.range.start..offset].graphemes(true).count()
    }

    fn offset_at_column(&self, column: usize) -> usize {
        let mut offset = self.range.start;
        let mut remaining = column;
        for grapheme in self.text[self.range.clone()].graphemes(true) {
            if remaining == 0 {
                break;
            }
            offset += grapheme.len();
            remaining -= 1;
        }
        offset.min(self.range.end)
    }

    fn previous_range(&self) -> Option<core::ops::Range<usize>> {
        if self.range.start == 0 {
            return None;
        }

        let newline = self.range.start - '\n'.len_utf8();
        let end = line_end_before_newline(self.text, newline);
        let start = self.text[..end]
            .rfind('\n')
            .map_or(0, |index| index + '\n'.len_utf8());
        Some(start..end)
    }

    fn next_range(&self) -> Option<core::ops::Range<usize>> {
        let relative_newline = self.text[self.range.end..].find('\n')?;

        let start = self.range.end + relative_newline + '\n'.len_utf8();
        let end = line_end_from(self.text, start);
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
