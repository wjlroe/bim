use crate::commands::SearchDirection;
use crate::cursor::{CursorT, CursorWithHistory};
use crate::row::{Row, DEFAULT_NEWLINE, DEFAULT_NEWLINE_STR, DOS_NEWLINE, UNIX_NEWLINE};
use crate::syntax::{Syntax, SYNTAXES};
use std::error::Error;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::rc::Rc;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum FileSaveStatus {
    // FileExists,
    NoFilename,
    Saved(usize),
}

#[derive(Default)]
pub struct Buffer<'a> {
    pub filename: Option<String>,
    pub rows: Vec<Row<'a>>,
    syntax: Rc<Option<&'a Syntax<'a>>>,
    pub cursor: CursorWithHistory,
    dirty: i32,
    newline: &'a str,
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

    // TODO: maybe introduce a RenderCursor and return it without params
    // we will still need to translate positions in the text to render
    // positions probably, but this just returns the column... it doesn't
    // care about line wrapping for instance...
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

    fn update_newline(&mut self) {
        if self.newline == "" {
            if self.rows.len() > 0 {
                if self.rows[0].as_str().ends_with(UNIX_NEWLINE) {
                    self.newline = UNIX_NEWLINE;
                } else if self.rows[0].as_str().ends_with(DOS_NEWLINE) {
                    self.newline = DOS_NEWLINE;
                } else {
                    self.newline = DEFAULT_NEWLINE_STR;
                }
            } else {
                self.newline = DEFAULT_NEWLINE_STR;
            }
        }
    }

    fn update_syntax_highlighting(&mut self) {
        self.rows
            .iter_mut()
            .fold(false, |prev, row| row.update_syntax_highlight(prev));
    }

    fn update(&mut self) {
        self.update_newline();
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
        if let Some(filename) = &self.filename {
            *Rc::make_mut(&mut self.syntax) = Syntax::for_filename(filename);
            self.set_syntax();
        }
    }

    pub fn set_filetype(&mut self, syntax_name: &str) {
        *Rc::make_mut(&mut self.syntax) = SYNTAXES
            .iter()
            .find(|syntax| syntax.filetype == syntax_name);
        self.set_syntax();
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

    pub fn save_file(&mut self) -> Result<FileSaveStatus, Box<dyn Error>> {
        if let Some(filename) = self.filename.clone() {
            let mut bytes_saved: usize = 0;
            let mut buffer = BufWriter::new(File::create(filename)?);
            for line in &self.rows {
                bytes_saved += buffer.write(line.as_str().as_bytes())?;
            }
            buffer.flush()?;
            self.dirty = 0;
            Ok(FileSaveStatus::Saved(bytes_saved))
        } else {
            Ok(FileSaveStatus::NoFilename)
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
        if at == 0 {
            self.update_newline();
        }
    }

    pub fn insert_newline(&mut self, row: usize, col: usize) -> i32 {
        let newline = self
            .rows
            .get(row)
            .map(|r| r.newline())
            .unwrap_or_else(|| DEFAULT_NEWLINE.to_string());
        if col == 0 {
            self.insert_row(row, &newline);
            0
        } else {
            let new_line_text = self.rows[row].truncate(col);
            let prev_indent = self.rows[row].get_indent();
            self.insert_row(row + 1, &new_line_text);
            self.rows[row + 1].set_indent(prev_indent);
            self.update_from(row);
            self.update_from(row + 1);
            prev_indent
        }
    }

    pub fn insert_newline_and_return(&mut self) {
        let indent = self.insert_newline(
            self.cursor.text_row() as usize,
            self.cursor.text_col() as usize,
        );
        self.cursor.change(|cursor| {
            cursor.text_row += 1;
            cursor.text_col = indent;
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
        let num_rows = self.num_lines() as i32;
        if self.cursor.text_row() >= num_rows {
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
        if self.newline == "" {
            self.update_newline();
        }
        if cursor_y == self.rows.len() as i32 {
            self.rows
                .push(Row::new(self.newline, Rc::downgrade(&self.syntax)));
        }
        self.rows[cursor_y as usize].insert_char(cursor_x as usize, character);
        self.dirty += 1;
        self.update_from(cursor_y as usize);
    }

    pub fn insert_char_at_cursor(&mut self, character: char) {
        self.insert_char(character, self.cursor.text_col(), self.cursor.text_row());
        self.cursor.change(|cursor| cursor.text_col += 1);
    }

    pub fn check_cursor(&mut self) {
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

        let row_len = self.line_len(new_cursor.text_row).unwrap_or(0);

        if new_cursor.text_col > row_len as i32 {
            new_cursor.text_col = row_len as i32;
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
    let mut buffer = Buffer::default();
    buffer.insert_newline(0, 0);
    assert_eq!(1, buffer.dirty);
    assert_eq!(1, buffer.num_lines());
    assert_eq!(DEFAULT_NEWLINE_STR, buffer.rows[0].as_str());
}

#[test]
fn test_insert_newline_after_firstline() {
    let mut buffer = Buffer::default();
    buffer.insert_char('1', 0, 0);
    assert_eq!(1, buffer.dirty);
    buffer.insert_newline(0, 1);
    assert_eq!(2, buffer.dirty);
    assert_eq!(2, buffer.num_lines());
    println!("{:?}", buffer.rows[0].as_str());
    assert!(buffer.rows[0].as_str().ends_with(DEFAULT_NEWLINE_STR));
    println!("{:?}", buffer.rows[1].as_str());
    assert!(buffer.rows[1].as_str().ends_with(DEFAULT_NEWLINE_STR));
}

#[test]
fn test_insert_char() {
    let mut buffer = Buffer::default();
    assert_eq!(0, buffer.dirty);
    buffer.insert_char('£', 0, 0);
    assert_eq!(1, buffer.dirty);
    buffer.insert_char('1', 1, 0);
    assert_eq!(2, buffer.dirty);
    let expected = format!("£1{}", DEFAULT_NEWLINE_STR);
    assert_eq!(
        vec![&expected],
        buffer
            .rows
            .iter()
            .map(|r| r.as_str().clone())
            .collect::<Vec<_>>()
    );
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
    let mut buffer = Buffer::default();
    buffer.append_row("\t£lots");
    assert_eq!(0, buffer.cursor.text_col());
    assert_eq!(
        0,
        buffer.text_cursor_to_render(buffer.cursor.text_col(), buffer.cursor.text_row())
    );
    buffer.cursor.change(|cursor| cursor.text_col += 1);
    assert_eq!(1, buffer.cursor.text_col());
    assert_eq!(
        8,
        buffer.text_cursor_to_render(buffer.cursor.text_col(), buffer.cursor.text_row())
    );
    buffer.cursor.change(|cursor| cursor.text_col += 1);
    assert_eq!(2, buffer.cursor.text_col());
    assert_eq!(
        9,
        buffer.text_cursor_to_render(buffer.cursor.text_col(), buffer.cursor.text_row())
    );
}

#[test]
fn test_move_cursor_with_inserted_text() {
    use crate::cursor::Cursor;

    let mut buffer = Buffer::default();
    assert_eq!(Cursor::default(), buffer.cursor.current());
    assert_eq!(0, buffer.cursor.text_row());
    assert_eq!(0, buffer.cursor.text_col());

    buffer.insert_char_at_cursor('H');
    assert_eq!(Cursor::new(0, 1), buffer.cursor.current());
    assert_eq!(0, buffer.cursor.text_row());
    assert_eq!(1, buffer.cursor.text_col());
    assert_eq!(1, buffer.text_cursor_to_render(1, 0));
}

#[test]
fn test_basic_auto_indent_on_return_no_syntax() {
    let mut buffer = Buffer::default();
    let file_contents = vec!["void main() {", "  int a_var = 10;"];
    for line in file_contents {
        for c in line.chars() {
            buffer.insert_char_at_cursor(c);
        }
        buffer.insert_newline_and_return();
    }
    assert_eq!(DEFAULT_NEWLINE_STR, buffer.rows[2].as_str());
    assert_eq!(0, buffer.cursor.text_col());
    assert_eq!(2, buffer.cursor.text_row());
}

#[test]
fn test_basic_auto_indent_on_return_c_syntax() {
    let mut buffer = Buffer::default();
    let file_contents = vec!["void main() {", "  int a_var = 10;", "  int b_var = 20;"];
    for line in file_contents {
        for c in line.chars() {
            buffer.insert_char_at_cursor(c);
        }
        buffer.insert_newline_and_return();
    }
    buffer.set_filetype("C");

    buffer.cursor.change(|cursor| {
        cursor.text_row = 1;
        cursor.text_col = 17;
    });
    buffer.insert_newline_and_return();

    use crate::highlight::Highlight::*;

    assert_eq!(vec![Normal; 3], buffer.rows[2].hl);

    let line_to_type = "int c_var = 5;";
    for c in line_to_type.chars() {
        buffer.insert_char_at_cursor(c);
    }

    let expected_contents = vec![
        format!("void main() {{{}", DEFAULT_NEWLINE_STR),
        format!("  int a_var = 10;{}", DEFAULT_NEWLINE_STR),
        format!("  int c_var = 5;{}", DEFAULT_NEWLINE_STR),
        format!("  int b_var = 20;{}", DEFAULT_NEWLINE_STR),
    ];
    for (i, row) in expected_contents.iter().enumerate() {
        assert_eq!(&row, &buffer.rows[i].as_str(), "Line {} is WRONG!", i);
    }
    assert_eq!(16, buffer.cursor.text_col());
    assert_eq!(2, buffer.cursor.text_row());

    let mut hl = vec![Normal; 2];
    hl.extend_from_slice(&[Keyword2; 3]);
    hl.extend_from_slice(&[Normal; 9]);
    hl.extend_from_slice(&[Number; 1]);
    hl.extend_from_slice(&[Normal; 2]);
    assert_eq!(hl, buffer.rows[2].hl);
}

// TODO: need a case for auto indent (or not) when inserting newline in the middle of a statement
// TODO: case for tab indents
