const TAB_STOP: usize = 8;

#[derive(PartialEq, Eq)]
pub struct Row {
    chars: String,
    pub size: usize,
    render: String,
    rsize: usize,
}

impl Row {
    pub fn new(text: &str) -> Self {
        let mut row = Row {
            chars: String::new(),
            size: 0,
            render: String::new(),
            rsize: 0,
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
        let mut string_end = self.chars.len();
        while string_end > 0 &&
            (self.chars.chars().nth(string_end - 1).unwrap() == '\n' ||
                self.chars.chars().nth(string_end - 1).unwrap() == '\r')
        {
            string_end -= 1;
        }
        self.size = string_end;
        self.update_render();
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

    pub fn insert_char(&mut self, at: usize, character: char) {
        let at = if at > self.size { self.size } else { at };
        self.chars.insert(at, character);
        self.size += 1;
        self.update_render();
    }

    pub fn append_text(&mut self, text: &str) {
        self.chars.truncate(self.size);
        self.chars.push_str(text);
        self.update();
    }

    pub fn delete_char(&mut self, at: usize) {
        let at = if at >= self.size { self.size - 1 } else { at };
        self.chars.remove(at);
        self.update();
    }

    pub fn newline(&self) -> String {
        String::from(&self.chars[self.size..])
    }

    pub fn truncate(&mut self, at: usize) -> String {
        let newline = self.newline();
        let new_line_text = String::from(&self.chars[at..]);
        self.chars.truncate(at);
        self.chars.push_str(&newline);
        self.update();
        new_line_text
    }

    pub fn onscreen_text(&self, offset: usize, cols: usize) -> String {
        // FIXME: call rendered_str here and slice it up!
        self.render
            .chars()
            .skip(offset)
            .take(cols)
            .collect::<String>()
    }

    pub fn as_str(&self) -> &str {
        self.chars.as_str()
    }

    #[allow(dead_code)]
    pub fn rendered_str(&self) -> &str {
        self.render.as_str()
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
fn test_append_text() {
    let mut row = Row::new("this is a line of text.\r\n");
    row.append_text("another line.\r\n");
    assert_eq!("this is a line of text.another line.\r\n", row.chars);
}

#[test]
fn test_newline() {
    let row = Row::new("this is a line.\r\n");
    assert_eq!("\r\n", row.newline());
    let row = Row::new("another line.\n");
    assert_eq!("\n", row.newline());
}
