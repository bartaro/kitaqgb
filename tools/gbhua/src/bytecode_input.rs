use plita::prelude::*;
use std::ops::Range;

const SCALE: f32 = 0.72;
const LINE_HEIGHT: f32 = 16.0;

pub(super) struct BytecodeInput {
    pub input: TextInputState,
    anchor: Option<usize>,
}

impl BytecodeInput {
    pub fn new() -> Self {
        Self {
            input: TextInputState::new(""),
            anchor: None,
        }
    }

    pub fn set_text(&mut self, text: String) {
        let focused = self.input.focused;
        self.input = TextInputState::new(text);
        self.input.focused = focused;
        self.anchor = None;
    }

    pub fn select_all(&mut self) {
        self.anchor = Some(0);
        self.input.move_end();
    }

    fn selection(&self) -> Option<Range<usize>> {
        let anchor = self.anchor?.min(self.input.text.len());
        let cursor = self.input.cursor.min(self.input.text.len());
        (anchor != cursor).then_some(anchor.min(cursor)..anchor.max(cursor))
    }

    pub fn selected_text(&self) -> Option<String> {
        self.selection()
            .map(|range| self.input.text[range].to_owned())
    }

    pub fn delete_selection(&mut self) -> bool {
        if let Some(range) = self.selection() {
            self.input.text.replace_range(range.clone(), "");
            self.input.cursor = range.start;
            self.anchor = None;
            true
        } else {
            self.anchor = None;
            false
        }
    }

    pub fn insert(&mut self, text: &str) {
        self.delete_selection();
        // A tile is sixteen bytes; cap accidental clipboard floods without parsing on every key.
        let remaining = 4096usize.saturating_sub(self.input.text.len());
        let accepted: String = text.chars().take(remaining).collect();
        self.input.insert_text(&accepted);
    }

    pub fn move_cursor(&mut self, key: &str, shift: bool) {
        self.input.normalize_cursor();
        if shift {
            self.anchor.get_or_insert(self.input.cursor);
        } else {
            if let Some(range) = self.selection() {
                self.input.cursor = if key == "ARROWLEFT" {
                    range.start
                } else {
                    range.end
                };
                self.anchor = None;
                if matches!(key, "ARROWLEFT" | "ARROWRIGHT") {
                    return;
                }
            }
            self.anchor = None;
        }
        match key {
            "ARROWLEFT" => self.input.move_left(),
            "ARROWRIGHT" => self.input.move_right(),
            "HOME" => self.input.move_home(),
            "END" => self.input.move_end(),
            _ => {}
        }
    }

    fn lines(&self, width: f32) -> Vec<Range<usize>> {
        let mut result = Vec::new();
        let mut start = 0;
        for (index, ch) in self.input.text.char_indices() {
            if ch == '\n' || ch == '\r' {
                result.push(start..index);
                start = index + ch.len_utf8();
            } else if index > start
                && measure_text_width(&self.input.text[start..index + ch.len_utf8()], SCALE) > width
            {
                result.push(start..index);
                start = index;
            }
        }
        result.push(start..self.input.text.len());
        result
    }

    fn visible_lines(&self, rect: Rect) -> (Vec<Range<usize>>, usize) {
        let lines = self.lines((rect.width - 14.0).max(10.0));
        let current = lines
            .iter()
            .rposition(|r| r.start <= self.input.cursor)
            .unwrap_or(0);
        let count = ((rect.height - 10.0) / LINE_HEIGHT).floor().max(1.0) as usize;
        let first = if self.input.focused {
            current.saturating_sub(count - 1)
        } else {
            0
        };
        (lines.into_iter().skip(first).take(count).collect(), first)
    }

    pub fn focus_at(&mut self, rect: Rect, point: Point) {
        let (lines, _) = self.visible_lines(rect);
        let row = ((point.y - rect.y - 6.0).max(0.0) / LINE_HEIGHT) as usize;
        if let Some(line) = lines.get(row.min(lines.len().saturating_sub(1))) {
            let text = &self.input.text[line.clone()];
            let x = (point.x - rect.x - 7.0).max(0.0);
            let closest = text
                .char_indices()
                .map(|(i, _)| i)
                .chain(Some(text.len()))
                .min_by(|a, b| {
                    (measure_text_width(&text[..*a], SCALE) - x)
                        .abs()
                        .total_cmp(&(measure_text_width(&text[..*b], SCALE) - x).abs())
                })
                .unwrap_or(0);
            self.input.cursor = line.start + closest;
        }
        self.anchor = None;
        self.input.focused = true;
    }

    pub fn draw(&self, list: &mut DrawList, rect: Rect, text_color: Color, accent: Color) {
        let (lines, _) = self.visible_lines(rect);
        list.push_clip(rect.inset(3.0));
        for (row, range) in lines.iter().enumerate() {
            let text = &self.input.text[range.clone()];
            let x = rect.x + 7.0;
            let y = rect.y + 6.0 + row as f32 * LINE_HEIGHT;
            if let Some(selection) = self.selection().filter(|_| self.input.focused) {
                let start = selection.start.max(range.start).min(range.end);
                let end = selection.end.min(range.end).max(start);
                if start < end {
                    let left = measure_text_width(&self.input.text[range.start..start], SCALE);
                    let width = measure_text_width(&self.input.text[start..end], SCALE);
                    list.fill_rect(
                        Rect::new(x + left, y, width, LINE_HEIGHT),
                        Color::rgba8(66, 151, 131, 120),
                    );
                }
            }
            draw_text(list, x, y, text, SCALE, text_color);
            let next_starts_here = lines
                .get(row + 1)
                .is_some_and(|r| r.start == self.input.cursor);
            if self.input.focused && range.contains(&self.input.cursor)
                || self.input.focused && self.input.cursor == range.end && !next_starts_here
            {
                let width =
                    measure_text_width(&self.input.text[range.start..self.input.cursor], SCALE);
                list.fill_rect(Rect::new(x + width, y, 1.5, 13.0), accent);
            }
        }
        list.pop_clip();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cursor_selection_and_replacement() {
        let mut field = BytecodeInput::new();
        field.set_text("0x78,0xCC".into());
        field.move_cursor("HOME", false);
        field.move_cursor("ARROWRIGHT", true);
        assert_eq!(field.selected_text().as_deref(), Some("0"));
        field.insert("$");
        assert_eq!(field.input.text, "$x78,0xCC");
        field.select_all();
        field.insert("0x00");
        assert_eq!(field.input.text, "0x00");
        assert_eq!(field.input.cursor, 4);
    }
}
