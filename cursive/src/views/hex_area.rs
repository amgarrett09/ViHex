// Modified version of TextArea, more suited to hex editing.
// Modifications by Alex Garrett <alex@alexgarrett.tech>.

use crate::direction::Direction;
use crate::event::{Event, EventResult, Key, MouseButton, MouseEvent};
use crate::rect::Rect;
use crate::theme::{ColorStyle, Effect};
use crate::types::EditorMode;
use crate::utils::lines::simple::{prefix, simple_prefix, LinesIterator, Row};
use crate::vec::Vec2;
use crate::view::{ScrollBase, SizeCache, View};
use crate::{Printer, With, XY};
use log::debug;
use std::cmp::min;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

const VALID_CHARS: [char; 16] = [
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'A', 'B', 'C', 'D', 'E',
    'F',
];

// Number of chars dedicated to displaying adddress offset
const ADDRESS_LENGTH: usize = 10;

/// Multi-line hex editor which can be navigated similarly to Vim.
pub struct HexArea {
    content: String,

    /// Byte offsets within `content` representing text rows
    ///
    /// Invariant: never empty.
    rows: Vec<Row>,

    /// When `false`, we don't take any input.
    enabled: bool,

    /// Base for scrolling features
    scrollbase: ScrollBase,

    /// Cache to avoid re-computing layout on no-op events
    size_cache: Option<XY<SizeCache>>,
    last_size: Vec2,

    /// Byte offset of the currently selected grapheme.
    cursor: usize,

    /// Current editor mode.
    ///
    /// User inputs have different effects in different modes, much like Vim.
    mode: EditorMode,

    bytes_per_line: usize,
}

fn make_rows(text: &str, width: usize) -> Vec<Row> {
    // We can't make rows with width=0, so force at least width=1.
    let width = usize::max(width, 1);
    LinesIterator::new(text, width).show_spaces().collect()
}

impl HexArea {
    /// Creates a new HexArea from a vector of hex values.
    pub fn from(hex_values: &Vec<&str>) -> Self {
        let content: String = hex_values.join(" ");
        let mut hex_area = HexArea {
            content: String::new(),
            rows: Vec::new(),
            enabled: true,
            scrollbase: ScrollBase::new().right_padding(0),
            size_cache: None,
            last_size: Vec2::zero(),
            cursor: 0,
            mode: EditorMode::Normal,
            bytes_per_line: 0,
        };

        hex_area.set_content(content);

        hex_area
    }

    /// Retrieves the content of the view.
    pub fn get_content(&self) -> &str {
        &self.content
    }

    fn invalidate(&mut self) {
        self.size_cache = None;
    }

    /// Returns the position of the cursor in the content string.
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// Moves the cursor to the given position.
    ///
    /// # Panics
    ///
    /// This method panics if `cursor` is not the beginning of a character in
    /// the content string.
    pub fn set_cursor(&mut self, cursor: usize) {
        self.cursor = cursor;

        let focus = self.selected_row();
        self.scrollbase.scroll_to(focus);
    }

    /// Sets the content of the view.
    pub fn set_content<S: Into<String>>(&mut self, content: S) {
        self.content = content.into();

        // First, make sure we are within the bounds.
        self.cursor = min(self.cursor, self.content.len());

        // We have no guarantee cursor is now at a correct UTF8 location.
        // So look backward until we find a valid grapheme start.
        while !self.content.is_char_boundary(self.cursor) {
            self.cursor -= 1;
        }

        if let Some(size) = self.size_cache.map(|s| s.map(|s| s.value)) {
            self.invalidate();
            self.compute_rows(size);
        }
    }

    /// Sets the content of the view.
    ///
    /// Chainable variant.
    pub fn content<S: Into<String>>(self, content: S) -> Self {
        self.with(|s| s.set_content(content))
    }

    /// Disables this view.
    ///
    /// A disabled view cannot be selected.
    pub fn disable(&mut self) {
        self.enabled = false;
    }

    /// Disables this view.
    ///
    /// Chainable variant.
    pub fn disabled(self) -> Self {
        self.with(Self::disable)
    }

    /// Re-enables this view.
    pub fn enable(&mut self) {
        self.enabled = true;
    }

    /// Re-enables this view.
    ///
    /// Chainable variant.
    pub fn enabled(self) -> Self {
        self.with(Self::enable)
    }

    /// Returns `true` if this view is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Finds the row containing the grapheme at the given offset
    fn row_at(&self, offset: usize) -> usize {
        debug!("Offset: {}", offset);

        assert!(!self.rows.is_empty());
        assert!(offset >= self.rows[0].start);

        self.rows
            .iter()
            .enumerate()
            .take_while(|&(_, row)| row.start <= offset)
            .map(|(i, _)| i)
            .last()
            .unwrap()
    }

    fn col_at(&self, offset: usize) -> usize {
        let row_id = self.row_at(offset);
        let row = self.rows[row_id];
        // Number of cells to the left of the cursor
        self.content[row.start..offset].width()
    }

    /// Finds the row containing the cursor
    fn selected_row(&self) -> usize {
        assert!(!self.rows.is_empty(), "Rows should never be empty.");
        self.row_at(self.cursor)
    }

    fn selected_col(&self) -> usize {
        self.col_at(self.cursor)
    }

    fn page_up(&mut self) {
        for _ in 0..5 {
            self.move_up();
        }
    }

    fn page_down(&mut self) {
        for _ in 0..5 {
            self.move_down();
        }
    }

    fn move_up(&mut self) {
        let row_id = self.selected_row();
        if row_id == 0 {
            return;
        }

        // Number of cells to the left of the cursor
        let x = self.col_at(self.cursor);

        let prev_row = self.rows[row_id - 1];
        let prev_text = &self.content[prev_row.start..prev_row.end];
        let offset = prefix(prev_text.graphemes(true), x, "").length;
        self.cursor = prev_row.start + offset;
    }

    fn move_down(&mut self) {
        let row_id = self.selected_row();
        if row_id + 1 == self.rows.len() {
            return;
        }
        let x = self.col_at(self.cursor);

        let next_row = self.rows[row_id + 1];
        let next_text = &self.content[next_row.start..next_row.end];
        let offset = prefix(next_text.graphemes(true), x, "").length;
        self.cursor = next_row.start + offset;
    }

    /// Moves the cursor to the left.
    ///
    /// Wraps the previous line if required.
    fn move_left(&mut self) {
        let len = {
            // We don't want to utf8-parse the entire content.
            // So only consider the last row.
            let mut row = self.selected_row();
            if self.rows[row].start == self.cursor {
                row = row.saturating_sub(1);
            }

            let text = &self.content[self.rows[row].start..self.cursor];
            text.graphemes(true).last().unwrap().len()
        };
        self.cursor -= len;
    }

    /// Moves the cursor to the right.
    ///
    /// Jumps to the next line is required.
    fn move_right(&mut self) {
        let len = self.content[self.cursor..]
            .graphemes(true)
            .next()
            .unwrap()
            .len();
        self.cursor += len;
    }

    fn is_cache_valid(&self, size: Vec2) -> bool {
        match self.size_cache {
            None => false,
            Some(ref last) => last.x.accept(size.x) && last.y.accept(size.y),
        }
    }

    // If we are editing the text, we add a fake "space" character for the
    // cursor to indicate where the next character will appear.
    // If the current line is full, adding a character will overflow into the
    // next line. To show that, we need to add a fake "ghost" row, just for
    // the cursor.
    fn fix_ghost_row(&mut self) {
        if self.rows.is_empty()
            || self.rows.last().unwrap().end != self.content.len()
        {
            // Add a fake, empty row at the end.
            self.rows.push(Row {
                start: self.content.len(),
                end: self.content.len(),
                width: 0,
            });
        }
    }

    fn soft_compute_rows(&mut self, size: Vec2) {
        if self.is_cache_valid(size) {
            debug!("Cache is still valid.");
            return;
        }
        debug!("Computing! Oh yeah!");

        let mut available = size.x - ADDRESS_LENGTH;

        self.rows = make_rows(&self.content, available);
        self.fix_ghost_row();

        if self.rows.len() > size.y {
            available = available.saturating_sub(1);
            // Apparently we'll need a scrollbar. Doh :(
            self.rows = make_rows(&self.content, available);
            self.fix_ghost_row();
        }

        if !self.rows.is_empty() {
            self.size_cache = Some(SizeCache::build(size, size));
        }
    }

    fn compute_rows(&mut self, size: Vec2) {
        self.soft_compute_rows(size);

        // Subtracting 1 from size.y so that we have room to display editor
        // status below the editing area.
        self.scrollbase.set_heights(size.y - 1, self.rows.len());

        let start = self.rows[0].start;
        let end = self.rows[0].end;
        self.bytes_per_line =
            self.content[start..end].trim().split(" ").count();
    }

    fn move_to_next_hex(&mut self) {
        self.move_right();

        if self.cursor + 1 < self.content.len() {
            while &self.content[self.cursor..(self.cursor + 1)] == " "
                || &self.content[self.cursor..(self.cursor + 1)] == "\n"
            {
                self.move_right();
                if self.cursor + 1 == self.content.len() {
                    break;
                }
            }
        }
    }

    fn move_to_prev_hex(&mut self) {
        self.move_left();

        if self.cursor > 0 {
            while &self.content[self.cursor..self.cursor + 1] == " "
                || &self.content[self.cursor..self.cursor + 1] == "\n"
            {
                self.move_left();
                if self.cursor == 0 {
                    break;
                }
            }
        }
    }

    fn replace(&mut self, ch: char) {
        let range = self.cursor..(self.cursor + 1);
        let st = ch.to_string();

        self.content.replace_range(range, &st);

        self.move_to_next_hex();
    }

    fn handle_normal_input(&mut self, ch: char) {
        match ch {
            'i' => self.mode = EditorMode::Insert,
            'l' if self.cursor < self.content.len() - 1 => {
                self.move_to_next_hex()
            }
            'h' if self.cursor > 0 => self.move_to_prev_hex(),
            'j' if self.selected_row() + 1 < self.rows.len() => {
                self.move_down();
                if self.cursor == self.content.len() {
                    self.move_left();
                }
            }
            'k' if self.selected_row() > 0 => self.move_up(),
            '0' => {
                // Go to start of line
                self.cursor = self.rows[self.selected_row()].start
            }
            '$' => {
                // Go to end of line
                let row = self.selected_row();
                self.cursor = self.rows[row].end - 1;
                let selected_char = &self.content[self.cursor..]
                    .chars()
                    .next()
                    .expect("Expected cursor to be highlighting a char");

                if *selected_char == ' ' || *selected_char == '\n' {
                    self.move_left();
                }
            }
            'w' if self.cursor < self.content.len() - 3 => {
                self.move_right();
                self.move_right();

                while &self.content[self.cursor..self.cursor + 1] == " "
                    || &self.content[self.cursor..self.cursor + 1] == "\n"
                {
                    self.move_right();
                }
            }
            'b' if self.cursor > 0 => {
                self.move_left();
                let selected_char = &self.content[self.cursor..]
                    .chars()
                    .next()
                    .expect("Expected char to be selected");

                if (selected_char == &' ' || selected_char == &'\n')
                    && self.cursor > 0
                {
                    self.move_to_prev_hex();
                    if self.cursor > 0 {
                        self.move_left();
                    }
                }
            }
            _ => (),
        }
    }

    /// Moves the cursor to the start of a memory address given in hex
    pub fn goto(&mut self, address: &str) {
        match hex_to_cursor_pos(address) {
            Some(i) => self.set_cursor(i),
            None => (),
        }
    }
}

impl View for HexArea {
    fn required_size(&mut self, constraint: Vec2) -> Vec2 {
        // Make sure our structure is up to date
        self.soft_compute_rows(constraint);

        // Ideally, we'd want x = the longest row + 1
        // (we always keep a space at the end)
        // And y = number of rows
        debug!("{:?}", self.rows);
        let scroll_width = if self.rows.len() > constraint.y { 1 } else { 0 };
        Vec2::new(
            scroll_width
                + 1
                + self.rows.iter().map(|r| r.width).max().unwrap_or(1),
            self.rows.len(),
        )
    }

    fn draw(&self, printer: &Printer<'_, '_>) {
        // Display editor status below the editing area
        printer.print((0, printer.size.y - 1), &self.mode.to_string());

        // Cropping printer so that we don't draw over status info
        let printer = &printer.cropped((printer.size.x, printer.size.y - 1));
        printer.with_color(ColorStyle::secondary(), |printer| {
            let effect = if self.enabled && printer.enabled {
                Effect::Reverse
            } else {
                Effect::Simple
            };

            let w = if self.scrollbase.scrollable() {
                printer.size.x.saturating_sub(1)
            } else {
                printer.size.x
            };
            printer.with_effect(effect, |printer| {
                for y in 0..printer.size.y {
                    printer.print_hline((0, y), w, " ");
                }
            });

            debug!("Content: `{}`", &self.content);
            self.scrollbase.draw(printer, |printer, i| {
                debug!("Drawing row {}", i);
                let row = &self.rows[i];
                debug!("row: {:?}", row);
                let text = &self.content[row.start..row.end];
                let address = to_32bit_hex(i * self.bytes_per_line);
                debug!("row text: `{}`", text);
                printer.with_effect(effect, |printer| {
                    printer.print((0, 0), &format!("{}{}", address, text));
                });

                if printer.focused && i == self.selected_row() {
                    let cursor_offset = self.cursor - row.start;
                    let c = if cursor_offset == text.len() {
                        "_"
                    } else {
                        text[(cursor_offset)..]
                            .graphemes(true)
                            .next()
                            .expect("Found no char!")
                    };
                    let offset =
                        text[..(cursor_offset)].width() + ADDRESS_LENGTH;
                    printer.print((offset, 0), c);
                }
            });
        });
    }

    fn on_event(&mut self, event: Event) -> EventResult {
        let mut fix_scroll = true;
        match event {
            Event::Char(ch) if self.mode.is_normal() => {
                self.handle_normal_input(ch);
            }
            Event::Char(ch) if self.mode.is_insert() => {
                let ch = ch.to_uppercase().next().unwrap();

                // Only replace a character if the input is a valid hex symbol
                if let Some(_) = VALID_CHARS.iter().position(|&s| s == ch) {
                    self.replace(ch);
                }
            }

            Event::Key(Key::Esc) => {
                self.mode = EditorMode::Normal;
            }

            Event::Ctrl(Key::Home) => self.cursor = 0,
            Event::Ctrl(Key::End) => self.cursor = self.content.len(),
            Event::Key(Key::Up) if self.selected_row() > 0 => self.move_up(),
            Event::Key(Key::Down)
                if self.selected_row() + 1 < self.rows.len() =>
            {
                self.move_down()
            }
            Event::Key(Key::PageUp) => self.page_up(),
            Event::Key(Key::PageDown) => self.page_down(),
            Event::Key(Key::Left) if self.cursor > 0 => self.move_left(),
            Event::Key(Key::Right) if self.cursor < self.content.len() => {
                self.move_right()
            }
            Event::Mouse {
                event: MouseEvent::WheelUp,
                ..
            } if self.scrollbase.can_scroll_up() => {
                fix_scroll = false;
                self.scrollbase.scroll_up(5);
            }
            Event::Mouse {
                event: MouseEvent::WheelDown,
                ..
            } if self.scrollbase.can_scroll_down() => {
                fix_scroll = false;
                self.scrollbase.scroll_down(5);
            }
            Event::Mouse {
                event: MouseEvent::Press(MouseButton::Left),
                position,
                offset,
            } if position
                .checked_sub(offset)
                .map(|position| {
                    self.scrollbase.start_drag(position, self.last_size.x)
                })
                .unwrap_or(false) =>
            {
                fix_scroll = false;
            }
            Event::Mouse {
                event: MouseEvent::Hold(MouseButton::Left),
                position,
                offset,
            } => {
                fix_scroll = false;
                let position = position.saturating_sub(offset);
                self.scrollbase.drag(position);
            }
            Event::Mouse {
                event: MouseEvent::Press(_),
                position,
                offset,
            } if !self.rows.is_empty()
                && position.fits_in_rect(offset, self.last_size) =>
            {
                if let Some(position) = position.checked_sub(offset) {
                    let y = position.y + self.scrollbase.start_line;
                    let y = min(y, self.rows.len() - 1);
                    let x = position.x;
                    let row = &self.rows[y];
                    let content = &self.content[row.start..row.end];

                    self.cursor = row.start + simple_prefix(content, x).length;
                }
            }
            _ => return EventResult::Ignored,
        }

        debug!("Rows: {:?}", self.rows);
        if fix_scroll {
            let focus = self.selected_row();
            self.scrollbase.scroll_to(focus);
        }

        EventResult::Consumed(None)
    }

    fn take_focus(&mut self, _: Direction) -> bool {
        self.enabled
    }

    fn layout(&mut self, size: Vec2) {
        self.last_size = size;
        self.compute_rows(size);
    }

    fn important_area(&self, _: Vec2) -> Rect {
        // The important area is a single character
        let char_width = if self.cursor >= self.content.len() {
            // If we're are the end of the content, it'll be a space
            1
        } else {
            // Otherwise it's the selected grapheme
            self.content[self.cursor..]
                .graphemes(true)
                .next()
                .unwrap()
                .width()
        };

        Rect::from_size(
            (self.selected_col(), self.selected_row()),
            (char_width, 1),
        )
    }
}

fn to_32bit_hex(num: usize) -> String {
    let mut acc: Vec<char> = Vec::new();

    let mut res: usize = num;
    while res > 0 {
        let remainder = res % 16;
        // Convert remainder into hex and push into accumulator
        acc.push(VALID_CHARS[remainder]);
        res = res / 16;
    }

    // Fill out with zeroes until we get to 32 bits
    while acc.len() < 8 {
        acc.push('0');
    }

    acc.reverse();
    acc.push(' ');
    acc.push(' ');
    acc.iter().collect()
}

fn hex_to_cursor_pos(hex: &str) -> Option<usize> {
    let mut dec_digits: Vec<usize> = Vec::new();

    // Convert hex digits to decimal, or return None if one is invalid
    for ch in hex.to_uppercase().chars() {
        match VALID_CHARS.iter().position(|&s| s == ch) {
            Some(digit) => dec_digits.push(digit),
            None => return None,
        }
    }

    // Convert to memory address to decimal
    let dec_address = dec_digits.iter().fold(0, |acc, num| {
        let mut acc = acc * 16;
        acc += num;
        acc
    });

    Some(dec_address * 3)
}
