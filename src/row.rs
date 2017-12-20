use highlight::{Highlight, DEFAULT_COLOUR, HL_TO_COLOUR};

const TAB_STOP: usize = 8;

#[derive(PartialEq, Eq)]
pub struct Row {
    chars: String,
    pub size: usize,
    render: String,
    rsize: usize,
    hl: Vec<Highlight>,
}

impl Row {
    pub fn new(text: &str) -> Self {
        let mut row = Row {
            chars: String::new(),
            size: 0,
            render: String::new(),
            rsize: 0,
            hl: vec![],
        };
        row.set_text(text);
        row
    }

    fn set_text(&mut self, text: &str) {
        self.chars.clear();
        self.chars.push_str(text);
        self.update();
    }

    fn update(&mut self) {
        let mut string_end = self.chars.chars().count();
        while string_end > 0
            && (self.chars.chars().nth(string_end - 1).unwrap() == '\n'
                || self.chars.chars().nth(string_end - 1).unwrap() == '\r')
        {
            string_end -= 1;
        }
        self.size = string_end;
        self.update_render();
        self.update_highlight();
    }

    fn update_render(&mut self) {
        self.render.clear();
        let mut rsize = 0;
        for source_char in self.chars.chars() {
            if source_char == '\t' {
                self.render.push(' ');
                rsize += 1;
                while rsize % TAB_STOP != 0 {
                    self.render.push(' ');
                    rsize += 1;
                }
            } else if source_char == '\n' || source_char == '\r' {
                continue;
            } else {
                self.render.push(source_char);
                rsize += 1;
            }
        }
        self.rsize = rsize;
    }

    fn update_highlight(&mut self) {
        use self::Highlight::*;

        self.hl.clear();
        for c in self.render.chars() {
            if c.is_digit(10) {
                self.hl.push(Number);
            } else {
                self.hl.push(Normal);
            }
        }
    }

    pub fn text_cursor_to_render(&self, cidx: i32) -> i32 {
        let tab_stop = TAB_STOP as i32;
        let mut ridx: i32 = 0;
        for (i, source_char) in self.chars.chars().enumerate() {
            if i == cidx as usize {
                break;
            }
            if source_char == '\t' {
                ridx += (tab_stop - 1) - (ridx % tab_stop);
            }
            ridx += 1;
        }
        ridx
    }

    pub fn render_cursor_to_text(&self, ridx: i32) -> i32 {
        let tab_stop = TAB_STOP as i32;
        let mut cur_cx: i32 = 0;
        let mut cur_rx: i32 = 0;
        for source_char in self.chars.chars() {
            if source_char == '\t' {
                cur_rx += (tab_stop - 1) - (cur_rx % tab_stop);
            }
            cur_rx += 1;
            if cur_rx > ridx {
                break;
            }
            cur_cx += 1;
        }
        cur_cx
    }

    fn render_cursor_to_byte_position(&self, at: usize) -> usize {
        self.chars.chars().take(at).map(|c| c.len_utf8()).sum()
    }

    fn byte_position_to_char_position(&self, at: usize) -> usize {
        self.render[0..at + 1].chars().count() - 1
    }

    pub fn insert_char(&mut self, at: usize, character: char) {
        let at = if at > self.size {
            self.size
        } else {
            at
        };
        let byte_pos = self.render_cursor_to_byte_position(at);
        self.chars.insert(byte_pos, character);
        self.update();
    }

    pub fn append_text(&mut self, text: &str) {
        let byte_pos = self.render_cursor_to_byte_position(self.size);
        self.chars.truncate(byte_pos);
        self.chars.push_str(text);
        self.update();
    }

    pub fn delete_char(&mut self, at: usize) {
        let at = if at >= self.size { self.size - 1 } else { at };
        let byte_pos = self.render_cursor_to_byte_position(at);
        self.chars.remove(byte_pos);
        self.update();
    }

    pub fn newline(&self) -> String {
        let byte_pos = self.render_cursor_to_byte_position(self.size);
        String::from(&self.chars[byte_pos..])
    }

    pub fn truncate(&mut self, at: usize) -> String {
        let newline = self.newline();
        let byte_pos = self.render_cursor_to_byte_position(at);
        let new_line_text = String::from(&self.chars[byte_pos..]);
        self.chars.truncate(byte_pos);
        self.chars.push_str(&newline);
        self.update();
        new_line_text
    }

    pub fn onscreen_text(&self, offset: usize, cols: usize) -> String {
        let mut onscreen = String::new();
        // FIXME: call rendered_str here and slice it up!
        let characters = self.render.chars().skip(offset).take(cols);
        let highlights = self.hl.iter().skip(offset).take(cols);

        for (c, hl) in characters.zip(highlights) {
            onscreen.push_str(
                format!(
                    "\x1b[{}m{}",
                    HL_TO_COLOUR.get(hl).unwrap_or(&DEFAULT_COLOUR),
                    c
                ).as_str(),
            );
        }
        onscreen.push_str(format!("\x1b[{}m", DEFAULT_COLOUR).as_str());
        onscreen
    }

    pub fn as_str(&self) -> &str {
        self.chars.as_str()
    }

    #[allow(dead_code)]
    pub fn rendered_str(&self) -> &str {
        self.render.as_str()
    }

    pub fn index_of(&self, needle: &str) -> Option<usize> {
        self.render
            .find(needle)
            .map(|at| self.byte_position_to_char_position(at))
    }
}

#[test]
fn test_insert_char() {
    let mut row = Row::new("a line of text\r\n");
    assert_eq!(14, row.size);
    assert_eq!(14, row.rsize);
    assert_eq!(row.chars.trim(), row.render);
    row.insert_char(2, 'z');
    assert_eq!(15, row.size);
    assert_eq!(15, row.rsize);
    assert_eq!("a zline of text\r\n", row.chars);
    row.insert_char(0, '_');
    assert_eq!(16, row.size);
    assert_eq!(16, row.rsize);
    assert_eq!("_a zline of text\r\n", row.chars);
    row.insert_char(16, '_');
    assert_eq!(17, row.size);
    assert_eq!(17, row.rsize);
    assert_eq!("_a zline of text_\r\n", row.chars);
    row.insert_char(0, '£');
    assert_eq!(18, row.size);
    assert_eq!(18, row.rsize);
    assert_eq!("£_a zline of text_\r\n", row.chars);
    row.insert_char(1, '1');
    assert_eq!(19, row.size);
    assert_eq!(19, row.rsize);
    assert_eq!("£1_a zline of text_\r\n", row.chars);
    row.insert_char(0, '£');
    row.insert_char(0, '£');
    assert_eq!("£££1_a zline of text_\r\n", row.chars);
    row.insert_char(2, '¬');
    assert_eq!("££¬£1_a zline of text_\r\n", row.chars);
}

#[test]
fn test_update() {
    let mut row = Row::new("£1.50\r\n");
    row.update();
    assert_eq!(5, row.size);
    assert_eq!(5, row.rsize);
}

#[test]
fn test_set_text() {
    let mut row = Row::new("");
    assert_eq!(0, row.size);
    assert_eq!(0, row.rsize);

    row.set_text("a row\n");

    assert_eq!(5, row.size);
    assert_eq!(5, row.rsize);

    row.set_text("another row\r\n");

    assert_eq!(11, row.size);
    assert_eq!(11, row.rsize);
}

#[test]
fn test_delete_char() {
    let mut row = Row::new("this is a nice row\r\n");
    assert_eq!(18, row.size);
    assert_eq!("this is a nice row", row.render);

    row.delete_char(0);
    assert_eq!("his is a nice row\r\n", row.chars);
    assert_eq!(17, row.size);
    assert_eq!("his is a nice row", row.render);

    row.delete_char(17);
    assert_eq!("his is a nice ro\r\n", row.chars);
    assert_eq!(16, row.size);
    assert_eq!("his is a nice ro", row.render);
}

#[test]
fn test_delete_char_utf8() {
    let mut row = Row::new("££1.50\r\n");
    assert_eq!(6, row.size);
    row.delete_char(1);
    assert_eq!(5, row.size);
    assert_eq!("£1.50", row.render);
}

#[test]
fn test_append_text() {
    let mut row = Row::new("this is a line of text.\r\n");
    row.append_text("another line.\r\n");
    assert_eq!("this is a line of text.another line.\r\n", row.chars);
    let mut row = Row::new("££\r\n");
    row.append_text("word\r\n");
    assert_eq!("££word\r\n", row.chars);
}

#[test]
fn test_newline() {
    let row = Row::new("this is a line.\r\n");
    assert_eq!("\r\n", row.newline());
    let row = Row::new("another line.\n");
    assert_eq!("\n", row.newline());
    let row = Row::new("££££\r\n");
    assert_eq!("\r\n", row.newline());
}

#[test]
fn test_truncate() {
    let mut row = Row::new("first.second.\r\n");
    row.truncate(6);
    assert_eq!("first.\r\n", row.chars);
    let mut row = Row::new("£££££.second.\r\n");
    row.truncate(6);
    assert_eq!("£££££.\r\n", row.chars);
}

#[test]
fn test_render_cursor_to_text() {
    {
        let row = Row::new("nothing interesting\r\n");
        assert_eq!(5, row.render_cursor_to_text(5));
    }

    {
        let row = Row::new("\tinteresting\r\n");
        assert_eq!("        interesting", row.rendered_str());
        assert_eq!(0, row.render_cursor_to_text(0));
        assert_eq!(1, row.render_cursor_to_text(8));
        assert_eq!(11, row.render_cursor_to_text(18));
        // the position after the text (EOL)
        assert_eq!(12, row.render_cursor_to_text(19));
    }

    {
        let row = Row::new("\t£intersting\r\n");
        assert_eq!("        £intersting", row.rendered_str());
        assert_eq!(0, row.render_cursor_to_text(0));
        assert_eq!(1, row.render_cursor_to_text(8));
        assert_eq!(2, row.render_cursor_to_text(9));
    }
}

#[test]
fn test_index_of() {
    {
        let row = Row::new("nothing interesting\r\n");
        assert_eq!(Some(0), row.index_of("nothing"));
        assert_eq!(Some(8), row.index_of("interesting"));
    }

    {
        let row = Row::new("\t£lots\r\n");
        assert_eq!("        £lots", row.rendered_str());
        assert_eq!(Some(9), row.index_of("lots"));
    }
}

#[test]
fn test_onscreen_text() {
    {
        let row = Row::new("no numbers here\r\n");
        let onscreen = row.onscreen_text(2, 9);
        assert!(onscreen.contains("\x1b[39m"));
        assert!(!onscreen.contains("\x1b[31m"));
        assert!(onscreen.ends_with("\x1b[39m"));
    }

    {
        let row = Row::new("number 1 here\r\n");
        let onscreen = row.onscreen_text(0, 11);
        assert!(onscreen.contains("\x1b[31m1\x1b[39m "));
        assert!(onscreen.contains("\x1b[39m"));
        assert!(onscreen.ends_with("\x1b[39m"));
    }
}

#[test]
fn test_highlight() {
    use highlight::Highlight::*;

    {
        let mut row = Row::new("normal\r\n");
        assert_eq!(vec![Normal; 6], row.hl);
        row.insert_char(0, '1');
        assert_eq!(
            vec![Number, Normal, Normal, Normal, Normal, Normal, Normal],
            row.hl
        );
    }

    {
        let row = Row::new("1A2b34zz \r\n");
        assert_eq!(
            vec![
                Number, Normal, Number, Normal, Number, Number, Normal, Normal,
                Normal,
            ],
            row.hl
        );
    }
}
