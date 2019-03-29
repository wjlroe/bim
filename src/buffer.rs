use crate::commands::{MoveCursor, SearchDirection};
use crate::cursor::{CursorT, CursorWithHistory};
use crate::row::{Row, DEFAULT_NEWLINE};
use crate::syntax::{Syntax, SYNTAXES};
use std::error::Error;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::rc::Rc;

#[derive(Default)]
pub struct Buffer<'a> {
    pub filename: Option<String>,
    pub rows: Vec<Row<'a>>,
    syntax: Rc<Option<&'a Syntax<'a>>>,
    pub cursor: CursorWithHistory,
    dirty: i32,
}

impl<'a> Buffer<'a> {
    pub fn is_dirty(&self) -> bool {
        self.dirty.is_positive()
    }

    pub fn num_lines(&self) -> usize {
        self.rows.len()
    }

    pub fn line_len(&self, line_num: i32) -> Option<usize> {
        self.rows.get(line_num as usize).map(|row| row.size)
    }

    pub fn row_onscreen_text(&self, line_num: usize, offset: usize, cols: usize) -> Option<String> {
        self.rows
            .get(line_num)
            .map(|row| row.onscreen_text(offset, cols))
    }

    pub fn text_cursor_to_render(&self, cursor_x: i32, cursor_y: i32) -> i32 {
        self.rows
            .get(cursor_y as usize)
            .map(|row| row.text_cursor_to_render(cursor_x))
            .unwrap_or(0)
    }

    fn insert_row(&mut self, at: usize, text: &str) {
        if at <= self.num_lines() {
            let row = Row::new(text, Rc::downgrade(&self.syntax));
            self.rows.insert(at, row);
            self.update_from(at);
            self.dirty += 1;
        }
    }

    pub fn clear_search_overlay(&mut self) {
        for row in self.rows.iter_mut() {
            row.clear_overlay_search();
        }
    }

    pub fn clear(&mut self) {
        self.rows.clear();
        self.dirty += 1;
    }

    fn update_syntax_highlighting(&mut self) {
        self.rows
            .iter_mut()
            .fold(false, |prev, row| row.update_syntax_highlight(prev));
    }

    fn update(&mut self) {
        self.update_syntax_highlighting();
    }

    fn update_from(&mut self, at: usize) {
        let mut in_comment = if at > 0 {
            self.rows
                .get(at - 1)
                .map(|row| row.hl_open_comment)
                .unwrap_or(false)
        } else {
            false
        };
        for row in self.rows.iter_mut().skip(at) {
            let prev_ml_comment = row.hl_open_comment;
            in_comment = row.update_syntax_highlight(in_comment);
            if in_comment != prev_ml_comment {
                row.hl_open_comment = in_comment;
            } else {
                break;
            }
        }
    }

    fn select_syntax(&mut self) {
        if let Some(ref filename) = self.filename {
            *Rc::make_mut(&mut self.syntax) = SYNTAXES
                .iter()
                .find(|syntax| syntax.matches_filename(&filename));
            self.set_syntax();
        }
    }

    pub fn open_file(&mut self, file: File) {
        self.clear();

        let mut reader = BufReader::new(file);
        loop {
            let mut line = String::new();
            let read_info = reader.read_line(&mut line);
            match read_info {
                Ok(bytes_read) if bytes_read > 0 => {
                    self.append_row(&line);
                }
                _ => break,
            }
        }
        self.dirty = 0;

        self.select_syntax();
    }

    pub fn get_filetype(&self) -> String {
        self.syntax
            .map(|x| x.filetype.to_string())
            .unwrap_or_else(|| "no ft".to_string())
    }

    pub fn open(&mut self, filename: &str) -> Result<(), Box<dyn Error>> {
        let f = File::open(filename)?;
        self.filename = Some(filename.to_string());
        self.open_file(f);
        self.select_syntax();
        Ok(())
    }

    pub fn set_filename(&mut self, filename: String) {
        self.filename = Some(filename);
        self.select_syntax();
    }

    pub fn save_to_file(&mut self) -> Result<usize, Box<dyn Error>> {
        if let Some(filename) = self.filename.clone() {
            let mut bytes_saved: usize = 0;
            let mut buffer = BufWriter::new(File::create(filename)?);
            for line in &self.rows {
                bytes_saved += buffer.write(line.as_str().as_bytes())?;
            }
            buffer.flush()?;
            self.dirty = 0;
            Ok(bytes_saved)
        } else {
            Err(String::from("No filename!").into())
        }
    }

    pub fn search_for(
        &mut self,
        last_match: Option<(usize, usize)>,
        direction: SearchDirection,
        needle: &str,
    ) -> Option<(usize, usize)> {
        self.clear_search_overlay();
        let first_row = if direction == SearchDirection::Backwards {
            1
        } else {
            0
        };
        let add_amount = last_match.map(|(_, l)| l as i32 + 1).unwrap_or(first_row);
        let num_rows = self.num_lines() as i32;
        let lines = match direction {
            SearchDirection::Forwards => (0..num_rows)
                .map(|i| (i + add_amount) % num_rows)
                .collect::<Vec<_>>(),
            SearchDirection::Backwards => (0..num_rows)
                .map(|i| (i + add_amount - 1) % num_rows)
                .rev()
                .collect::<Vec<_>>(),
        };
        let mut found_match = None;
        for y in lines {
            assert!(y < num_rows, "num_rows = {}, y = {}", num_rows, y);
            let row = &mut self.rows[y as usize];
            if let Some(rx) = row.index_of(needle) {
                let x = row.render_cursor_to_text(rx);
                row.set_overlay_search(x, x + needle.len());
                found_match = Some((x, y as usize));
                break;
            }
        }
        if let Some((x, y)) = found_match {
            self.cursor.change(|cursor| {
                cursor.text_col = x as i32;
                cursor.text_row = y as i32;
            });
        }
        found_match
    }

    pub fn set_syntax(&mut self) {
        for row in self.rows.iter_mut() {
            row.set_syntax(Rc::downgrade(&self.syntax));
        }
        self.update();
    }

    pub fn append_row(&mut self, text: &str) {
        let at = self.num_lines();
        self.insert_row(at, text);
    }

    pub fn insert_newline(&mut self, row: usize, col: usize) {
        let newline = self
            .rows
            .get(row)
            .map(|r| r.newline())
            .unwrap_or_else(|| DEFAULT_NEWLINE.to_string());
        if col == 0 {
            self.insert_row(row, &newline);
        } else {
            let new_line_text = self.rows[row].truncate(col);
            self.insert_row(row + 1, &new_line_text);
            self.update_from(row);
        }
    }

    pub fn insert_newline_and_return(&mut self) {
        self.insert_newline(
            self.cursor.text_row() as usize,
            self.cursor.text_col() as usize,
        );
        self.cursor.change(|cursor| {
            cursor.text_row += 1;
            cursor.text_col = 0;
        });
    }

    pub fn join_row(&mut self, at: usize) -> bool {
        if at > 0 && at < self.num_lines() {
            let row = self.rows.remove(at);
            if let Some(previous_row) = self.rows.get_mut(at - 1) {
                previous_row.append_text(row.as_str());
            }
            self.dirty += 1;
            self.update_from(at - 1);
            true
        } else {
            false
        }
    }

    fn delete_char(&mut self, x: i32, y: i32) {
        self.rows[y as usize].delete_char((x - 1) as usize);
        self.update_from(y as usize);
    }

    pub fn delete_char_at_cursor(&mut self) {
        let numrows = self.num_lines() as i32;
        if self.cursor.text_row() >= numrows {
            return;
        }
        if self.cursor.text_col() > 0 {
            self.delete_char(self.cursor.text_col(), self.cursor.text_row());
            self.cursor.change(|cursor| cursor.text_col -= 1);
            self.dirty += 1;
        } else if self.cursor.text_row() > 0 && self.cursor.text_col() == 0 {
            let at = self.cursor.text_row();
            let new_col = self.line_len(at - 1).unwrap_or(0) as i32;
            self.join_row(at as usize);
            self.cursor.change(|cursor| {
                cursor.text_col = new_col;
                cursor.text_row -= 1;
            });
        }
    }

    pub fn insert_char(&mut self, character: char, cursor_x: i32, cursor_y: i32) {
        if cursor_y == self.rows.len() as i32 {
            self.rows.push(Row::new("", Rc::downgrade(&self.syntax)));
        }
        self.rows[cursor_y as usize].insert_char(cursor_x as usize, character);
        self.dirty += 1;
        self.update_from(cursor_y as usize);
    }

    pub fn insert_char_at_cursor(&mut self, character: char) {
        self.insert_char(character, self.cursor.text_col(), self.cursor.text_row());
        self.cursor.change(|cursor| cursor.text_col += 1);
    }

    pub fn move_cursor(&mut self, move_cursor: MoveCursor, page_size: usize) {
        use crate::commands::Direction::*;
        use crate::commands::MoveUnit::*;

        match move_cursor {
            MoveCursor {
                unit: Rows,
                direction: Up,
                amount,
            } => {
                let max_amount = self.cursor.text_row();
                let possible_amount = std::cmp::min(amount as i32, max_amount);
                self.cursor
                    .change(|cursor| cursor.text_row -= possible_amount);
            }
            MoveCursor {
                unit: Rows,
                direction: Down,
                amount,
            } => {
                let max_movement = self.num_lines() as i32 - 1 - self.cursor.text_row();
                let possible_amount = std::cmp::min(amount as i32, max_movement);
                self.cursor
                    .change(|cursor| cursor.text_row += possible_amount);
            }
            MoveCursor {
                unit: Cols,
                direction: Left,
                amount,
            } => {
                let mut new_cursor = self.cursor.current();
                let mut left_amount = amount as i32;
                while left_amount > 0 {
                    if new_cursor.text_col != 0 {
                        new_cursor.text_col -= 1;
                    } else if new_cursor.text_row > 0 {
                        new_cursor.text_row -= 1;
                        new_cursor.text_col =
                            self.line_len(new_cursor.text_row).unwrap_or(0) as i32;
                    } else {
                        break;
                    }
                    left_amount -= 1;
                }
                self.cursor.change(|cursor| {
                    cursor.text_col = new_cursor.text_col();
                    cursor.text_row = new_cursor.text_row();
                });
            }
            MoveCursor {
                unit: Cols,
                direction: Right,
                amount,
            } => {
                let mut new_cursor = self.cursor.current();
                let mut right_amount = amount as i32;
                let num_lines = self.num_lines() as i32;
                while right_amount > 0 {
                    if let Some(row_size) = self.line_len(new_cursor.text_row) {
                        if new_cursor.text_col < row_size as i32 {
                            new_cursor.text_col += 1;
                        } else if new_cursor.text_col == row_size as i32
                            && new_cursor.text_row < num_lines - 1
                        {
                            new_cursor.text_row += 1;
                            new_cursor.text_col = 0;
                        } else {
                            break;
                        }
                        right_amount -= 1;
                    } else {
                        break;
                    }
                }
                self.cursor.change(|cursor| {
                    cursor.text_col = new_cursor.text_col();
                    cursor.text_row = new_cursor.text_row();
                });
            }
            MoveCursor {
                unit: Start,
                direction: Left,
                ..
            } => self.cursor.change(|cursor| cursor.text_col = 0),
            MoveCursor {
                unit: End,
                direction: Right,
                ..
            } => {
                let new_x = self.line_len(self.cursor.text_row()).unwrap_or(0) as i32;
                self.cursor.change(|cursor| {
                    cursor.text_col = new_x;
                });
            }
            MoveCursor {
                unit: Pages,
                direction: Down,
                amount,
            } => {
                let amount = amount * page_size;
                self.move_cursor(MoveCursor::down(amount), page_size);
            }
            MoveCursor {
                unit: Pages,
                direction: Up,
                amount,
            } => {
                let amount = amount * page_size;
                self.move_cursor(MoveCursor::up(amount), page_size);
            }
            _ => {}
        }

        self.check_cursor();
    }

    fn check_cursor(&mut self) {
        let current_cursor = self.cursor.current();
        let mut new_cursor = self.cursor.current();
        if new_cursor.text_row < 0 {
            new_cursor.text_row = 0;
        }

        if new_cursor.text_row > self.num_lines() as i32 {
            new_cursor.text_row = self.num_lines() as i32;
        }

        if new_cursor.text_col < 0 {
            new_cursor.text_col = 0;
        }

        let rowlen = self.line_len(new_cursor.text_row).unwrap_or(0);

        if new_cursor.text_col > rowlen as i32 {
            new_cursor.text_col = rowlen as i32;
        }

        if current_cursor != new_cursor {
            self.cursor.change(|cursor| {
                cursor.text_col = new_cursor.text_col();
                cursor.text_row = new_cursor.text_row();
            });
        }
    }
}

#[test]
fn test_join_row() {
    let mut buffer = Buffer::default();

    buffer.append_row("this is the first line. \r\n");
    buffer.append_row("this is the second line.\r\n");
    buffer.dirty = 0;
    assert_eq!(2, buffer.num_lines());

    buffer.join_row(1);
    assert_eq!(1, buffer.dirty);
    assert_eq!(1, buffer.num_lines());
    let first_row = buffer.rows.get(0).clone().unwrap();
    assert_eq!(
        "this is the first line. this is the second line.\r\n",
        first_row.as_str()
    );
}

#[test]
fn test_insert_newline() {
    let mut buffer = Buffer::default();
    buffer.append_row("what a good first line.\r\n");
    buffer.append_row("not a bad second line\r\n");
    buffer.dirty = 0;
    assert_eq!(2, buffer.num_lines());

    buffer.insert_newline(1, 0);

    assert_eq!(3, buffer.num_lines());
    assert_eq!(1, buffer.dirty);
    assert_eq!(
        vec![
            "what a good first line.\r\n",
            "\r\n",
            "not a bad second line\r\n",
        ],
        buffer
            .rows
            .iter()
            .map(|r| r.as_str().clone())
            .collect::<Vec<_>>()
    );

    buffer.insert_newline(2, 4);

    assert_eq!(4, buffer.num_lines());
    assert_eq!(2, buffer.dirty);
    assert_eq!(
        vec![
            "what a good first line.\r\n",
            "\r\n",
            "not \r\n",
            "a bad second line\r\n",
        ],
        buffer
            .rows
            .iter()
            .map(|r| r.as_str().clone())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        vec![
            "what a good first line.\n",
            "\n",
            "not \n",
            "a bad second line\n"
        ],
        buffer
            .rows
            .iter()
            .map(|r| r.rendered_str().clone())
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_insert_newline_default() {
    use crate::row::DEFAULT_NEWLINE;

    let mut buffer = Buffer::default();
    buffer.insert_newline(0, 0);
    assert_eq!(1, buffer.dirty);
    assert_eq!(1, buffer.num_lines());
    assert_eq!(DEFAULT_NEWLINE.to_string(), buffer.rows[0].as_str());
}

#[test]
fn test_insert_newline_after_firstline() {
    use crate::row::DEFAULT_NEWLINE;

    let mut buffer = Buffer::default();
    buffer.insert_char('1', 0, 0);
    assert_eq!(1, buffer.dirty);
    buffer.insert_newline(0, 1);
    assert_eq!(2, buffer.dirty);
    assert_eq!(2, buffer.num_lines());
    assert!(buffer.rows[0]
        .as_str()
        .ends_with(&DEFAULT_NEWLINE.to_string()));
}

#[test]
fn test_insert_char() {
    let mut buffer = Buffer::default();
    assert_eq!(0, buffer.dirty);
    buffer.insert_char('£', 0, 0);
    assert_eq!(1, buffer.dirty);
    buffer.insert_char('1', 1, 0);
    assert_eq!(2, buffer.dirty);
    assert_eq!(
        vec!["£1"],
        buffer
            .rows
            .iter()
            .map(|r| r.as_str().clone())
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_search_match_highlighting() {
    let mut buffer = Buffer::default();
    buffer.append_row("nothing abc123 nothing\r\n");
    let match_coords = buffer
        .search_for(None, SearchDirection::Forwards, "abc123")
        .unwrap();
    let row_idx = match_coords.1;
    let row = &buffer.rows[row_idx];
    let onscreen = row.onscreen_text(0, 22);
    assert!(onscreen.contains("\x1b[34mabc123\x1b[39m"));
}

#[test]
fn test_clearing_search_overlay_from_onscreen_text() {
    let mut buffer = Buffer::default();
    buffer.append_row("nothing abc123 nothing\r\n");
    let (_, row_idx) = buffer
        .search_for(None, SearchDirection::Forwards, "abc123")
        .unwrap();
    buffer.clear_search_overlay();
    let row = &buffer.rows[row_idx];
    let onscreen = row.onscreen_text(0, 22);
    assert!(!onscreen.contains("\x1b[34m"));
}

#[test]
fn test_search_backwards_beyond_beginning_of_the_buffer() {
    let mut buffer = Buffer::default();
    buffer.append_row("nothing interesting here\r\n");
    buffer.append_row("nothing again\r\n");
    assert_eq!(
        Some((0, 1)),
        buffer.search_for(None, SearchDirection::Backwards, "nothing")
    );
    assert_eq!(
        Some((0, 1)),
        buffer.search_for(Some((0, 0)), SearchDirection::Backwards, "nothing")
    );
}

#[test]
fn test_search_clearing_previous_overlays() {
    let mut buffer = Buffer::default();
    buffer.append_row("#define _SOMETHING\r\n");
    buffer.append_row("#define _WOOT\r\n");
    buffer.append_row("#define _123\r\n");
    for row in &buffer.rows {
        assert_eq!(None, row.overlay.iter().find(|item| item.is_some()));
    }
    let mut last_match = buffer.search_for(None, SearchDirection::Forwards, "define");
    assert!(buffer.rows[0]
        .overlay
        .iter()
        .find(|item| item.is_some())
        .is_some());
    assert!(buffer.rows[1]
        .overlay
        .iter()
        .find(|item| item.is_some())
        .is_none());
    assert!(buffer.rows[2]
        .overlay
        .iter()
        .find(|item| item.is_some())
        .is_none());
    last_match = buffer.search_for(last_match, SearchDirection::Forwards, "define");
    assert!(buffer.rows[0]
        .overlay
        .iter()
        .find(|item| item.is_some())
        .is_none());
    assert!(buffer.rows[1]
        .overlay
        .iter()
        .find(|item| item.is_some())
        .is_some());
    assert!(buffer.rows[2]
        .overlay
        .iter()
        .find(|item| item.is_some())
        .is_none());
    last_match = buffer.search_for(last_match, SearchDirection::Forwards, "define");
    assert!(buffer.rows[0]
        .overlay
        .iter()
        .find(|item| item.is_some())
        .is_none());
    assert!(buffer.rows[1]
        .overlay
        .iter()
        .find(|item| item.is_some())
        .is_none());
    assert!(buffer.rows[2]
        .overlay
        .iter()
        .find(|item| item.is_some())
        .is_some());
    buffer.search_for(last_match, SearchDirection::Forwards, "define");
    assert!(buffer.rows[0]
        .overlay
        .iter()
        .find(|item| item.is_some())
        .is_some());
    assert!(buffer.rows[1]
        .overlay
        .iter()
        .find(|item| item.is_some())
        .is_none());
    assert!(buffer.rows[2]
        .overlay
        .iter()
        .find(|item| item.is_some())
        .is_none());
}

#[test]
fn test_move_cursor_to_search_match() {
    let mut buffer = Buffer::default();
    buffer.append_row("#define _SOMETHING\r\n");
    buffer.append_row("#define _123\r\n");
    buffer.append_row("123 #define _INDENT\r\n");
    let mut last_match = buffer.search_for(None, SearchDirection::Forwards, "define");
    assert_eq!(0, buffer.cursor.text_row());
    assert_eq!(1, buffer.cursor.text_col());
    last_match = buffer.search_for(last_match, SearchDirection::Forwards, "define");
    assert_eq!(1, buffer.cursor.text_row());
    assert_eq!(1, buffer.cursor.text_col());
    last_match = buffer.search_for(last_match, SearchDirection::Forwards, "define");
    assert_eq!(2, buffer.cursor.text_row());
    assert_eq!(5, buffer.cursor.text_col());
    buffer.search_for(last_match, SearchDirection::Forwards, "define");
    assert_eq!(0, buffer.cursor.text_row());
    assert_eq!(1, buffer.cursor.text_col());
}

#[test]
fn test_move_cursor() {
    let page_size = 100;
    let mut buffer = Buffer::default();
    buffer.append_row("\t£lots");
    assert_eq!(0, buffer.cursor.text_col());
    assert_eq!(
        0,
        buffer.text_cursor_to_render(buffer.cursor.text_col(), buffer.cursor.text_row())
    );
    buffer.move_cursor(MoveCursor::right(1), page_size);
    assert_eq!(1, buffer.cursor.text_col());
    assert_eq!(
        8,
        buffer.text_cursor_to_render(buffer.cursor.text_col(), buffer.cursor.text_row())
    );
    buffer.move_cursor(MoveCursor::right(1), page_size);
    assert_eq!(2, buffer.cursor.text_col());
    assert_eq!(
        9,
        buffer.text_cursor_to_render(buffer.cursor.text_col(), buffer.cursor.text_row())
    );
}
