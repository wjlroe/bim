use crate::commands::SearchDirection;
use crate::editor::DEFAULT_NEWLINE;
use crate::row::Row;
use crate::syntax::Syntax;
use std::error::Error;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::rc::Rc;

pub struct Buffer<'a> {
    rows: Vec<Row<'a>>,
    syntax: Rc<Option<&'a Syntax<'a>>>,
}

impl<'a> Buffer<'a> {
    pub fn new(syntax: Rc<Option<&'a Syntax<'a>>>) -> Self {
        Buffer {
            rows: Vec::new(),
            syntax,
        }
    }

    pub fn num_lines(&self) -> usize {
        self.rows.len()
    }

    pub fn line_len(&self, line_num: i32) -> Option<usize> {
        self.rows.get(line_num as usize).map(|row| row.size)
    }

    pub fn row_onscreen_text(
        &self,
        line_num: usize,
        offset: usize,
        cols: usize,
    ) -> Option<String> {
        self.rows
            .get(line_num)
            .map(|row| row.onscreen_text(offset, cols))
    }

    pub fn text_cursor_to_render(&self, cursor_x: i32, cursor_y: i32) -> i32 {
        self.rows[cursor_y as usize].text_cursor_to_render(cursor_x)
    }

    fn insert_row(&mut self, at: usize, text: &str) {
        if at <= self.num_lines() {
            let row = Row::new(text, Rc::downgrade(&self.syntax));
            self.rows.insert(at, row);
            self.update_from(at);
        }
    }

    pub fn clear_search_overlay(&mut self) {
        for row in self.rows.iter_mut() {
            row.clear_overlay_search();
        }
    }

    pub fn clear(&mut self) {
        self.rows.clear();
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
    }

    pub fn save_to_file(
        &self,
        filename: &str,
    ) -> Result<usize, Box<dyn Error>> {
        let mut bytes_saved: usize = 0;
        let mut buffer = BufWriter::new(File::create(filename)?);
        for line in &self.rows {
            bytes_saved += buffer.write(line.as_str().as_bytes())?;
        }
        buffer.flush()?;
        Ok(bytes_saved)
    }

    pub fn search_for(
        &mut self,
        last_match: Option<(usize, usize)>,
        direction: SearchDirection,
        needle: &str,
    ) -> Option<(usize, usize)> {
        let add_amount = last_match.map(|(_, l)| l + 1).unwrap_or(0);
        let num_rows = self.num_lines();
        let lines = match direction {
            SearchDirection::Forwards => (0..num_rows)
                .map(|i| (i + add_amount) % num_rows)
                .collect::<Vec<_>>(),
            SearchDirection::Backwards => (0..num_rows)
                .map(|i| (i + add_amount - 1) % num_rows)
                .rev()
                .collect::<Vec<_>>(),
        };
        for y in lines {
            assert!(y < num_rows, "num_rows = {}, y = {}", num_rows, y);
            let row = &mut self.rows[y];
            if let Some(rx) = row.index_of(needle) {
                let x = row.render_cursor_to_text(rx);
                row.set_overlay_search(x, x + needle.len());
                return Some((x, y));
            }
        }
        None
    }

    pub fn set_syntax(&mut self, syntax: Rc<Option<&'a Syntax<'a>>>) {
        self.syntax = syntax;
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
            .unwrap_or(DEFAULT_NEWLINE.to_string());
        if col == 0 {
            self.insert_row(row, &newline);
        } else {
            let new_line_text = self.rows[row].truncate(col);
            self.insert_row(row + 1, &new_line_text);
            self.update_from(row);
        }
    }

    pub fn join_row(&mut self, at: usize) -> bool {
        if at > 0 && at < self.num_lines() {
            let row = self.rows.remove(at);
            if let Some(previous_row) = self.rows.get_mut(at - 1) {
                previous_row.append_text(row.as_str());
            }
            self.update_from(at - 1);
            true
        } else {
            false
        }
    }

    pub fn delete_char(&mut self, x: i32, y: i32) {
        self.rows[y as usize].delete_char((x - 1) as usize);
        self.update_from(y as usize);
    }

    pub fn insert_char(
        &mut self,
        character: char,
        cursor_x: i32,
        cursor_y: i32,
    ) {
        if cursor_y == self.rows.len() as i32 {
            self.rows.push(Row::new("", Rc::downgrade(&self.syntax)));
        }
        self.rows[cursor_y as usize].insert_char(cursor_x as usize, character);
        self.update_from(cursor_y as usize);
    }
}

#[test]
fn test_join_row() {
    let syntax = Rc::new(None);
    let mut buffer = Buffer::new(syntax);

    buffer.append_row("this is the first line. \r\n");
    buffer.append_row("this is the second line.\r\n");
    assert_eq!(2, buffer.num_lines());

    buffer.join_row(1);
    assert_eq!(1, buffer.num_lines());
    let first_row = buffer.rows.get(0).clone().unwrap();
    assert_eq!(
        "this is the first line. this is the second line.\r\n",
        first_row.as_str()
    );
}

#[test]
fn test_insert_newline() {
    let syntax = Rc::new(None);
    let mut buffer = Buffer::new(syntax);
    buffer.append_row("what a good first line.\r\n");
    buffer.append_row("not a bad second line\r\n");
    assert_eq!(2, buffer.num_lines());

    buffer.insert_newline(1, 0);

    assert_eq!(3, buffer.num_lines());
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
        vec!["what a good first line.", "", "not ", "a bad second line"],
        buffer
            .rows
            .iter()
            .map(|r| r.rendered_str().clone())
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_insert_newline_default() {
    use crate::editor::DEFAULT_NEWLINE;
    let syntax = Rc::new(None);
    let mut buffer = Buffer::new(syntax);
    buffer.insert_newline(0, 0);
    assert_eq!(1, buffer.num_lines());
    assert_eq!(DEFAULT_NEWLINE, buffer.rows[0].as_str());
}

#[test]
fn test_insert_newline_after_firstline() {
    use crate::editor::DEFAULT_NEWLINE;
    let syntax = Rc::new(None);
    let mut buffer = Buffer::new(syntax);
    buffer.insert_char('1', 0, 0);
    buffer.insert_newline(0, 1);
    assert_eq!(2, buffer.num_lines());
    assert!(buffer.rows[0].as_str().ends_with(DEFAULT_NEWLINE));
}

#[test]
fn test_insert_char() {
    let syntax = Rc::new(None);
    let mut buffer = Buffer::new(syntax);
    buffer.insert_char('£', 0, 0);
    buffer.insert_char('1', 1, 0);
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
    let syntax = Rc::new(None);
    let mut buffer = Buffer::new(syntax);
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
fn test_clearing_search_overlay() {
    let syntax = Rc::new(None);
    let mut buffer = Buffer::new(syntax);
    buffer.append_row("nothing abc123 nothing\r\n");
    let (_, row_idx) = buffer
        .search_for(None, SearchDirection::Forwards, "abc123")
        .unwrap();
    buffer.clear_search_overlay();
    let row = &buffer.rows[row_idx];
    let onscreen = row.onscreen_text(0, 22);
    assert!(!onscreen.contains("\x1b[34m"));
}
