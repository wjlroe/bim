use highlight::{Highlight, DEFAULT_COLOUR, HL_TO_COLOUR};
use std::rc::Weak;
use syntax::Syntax;

const TAB_STOP: usize = 8;
const SEPARATORS: &str = ",.()+-/*=~%<>[];";

pub struct Row<'a> {
    chars: String,
    pub size: usize,
    render: String,
    rsize: usize,
    hl: Vec<Highlight>,
    overlay: Vec<Option<Highlight>>,
    syntax: Weak<Option<&'a Syntax<'a>>>,
}

impl<'a> Row<'a> {
    pub fn new(text: &str, syntax: Weak<Option<&'a Syntax<'a>>>) -> Self {
        let mut row = Row {
            chars: String::new(),
            size: 0,
            render: String::new(),
            rsize: 0,
            hl: vec![],
            overlay: vec![],
            syntax,
        };
        row.set_text(text);
        row
    }

    fn set_text(&mut self, text: &str) {
        self.chars.clear();
        self.chars.push_str(text);
        self.update();
    }

    pub fn set_syntax(&mut self, syntax: Weak<Option<&'a Syntax<'a>>>) {
        self.syntax = syntax;
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
        self.clear_overlay();
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

    fn is_separator(&self, c: char) -> bool {
        // TODO: Is null char '\0'?
        c.is_whitespace() || SEPARATORS.contains(c)
    }

    fn clear_overlay(&mut self) {
        self.overlay.clear();
        for _ in self.render.chars() {
            self.overlay.push(None);
        }
    }

    fn update_highlight(&mut self) {
        use self::Highlight::*;

        self.hl.clear();
        let syntax = self.syntax.upgrade();
        if syntax.is_none() {
            return;
        }
        let syntax = syntax.unwrap();
        if syntax.is_none() {
            return;
        }
        let syntax = syntax.unwrap();

        let mut prev_sep = true;
        let mut in_string: Option<char> = None;
        let mut escaped_quote = false;
        let mut in_keyword: Option<(Highlight, usize)> = None;
        for (idx, c) in self.render.chars().enumerate() {
            let mut cur_hl = None;
            let prev_hl = if idx > 0 {
                self.hl.get(idx - 1).cloned().unwrap_or(Normal)
            } else {
                Normal
            };

            if let Some((_, 0)) = in_keyword {
                in_keyword = None;
            }

            if let Some(val) = in_keyword.as_mut() {
                val.1 -= 1;
                self.hl.push(val.0);
                continue;
            }

            if syntax.highlight_comments() && in_string.is_none() {
                let rest_of_line = &self.render[idx..];
                if rest_of_line.starts_with(syntax.singleline_comment_start) {
                    for _ in idx..self.rsize {
                        self.hl.push(Comment);
                    }
                    break;
                }
            }

            if syntax.highlight_strings() {
                if let Some(string_char) = in_string {
                    cur_hl = Some(String);
                    if escaped_quote {
                        escaped_quote = false;
                    } else if c == '\\' && idx + 1 < self.rsize {
                        escaped_quote = true;
                    } else if string_char == c {
                        in_string = None;
                    }
                } else {
                    if c == '\'' || c == '"' {
                        in_string = Some(c);
                        cur_hl = Some(String);
                    }
                }
            }

            if syntax.highlight_numbers() && cur_hl.is_none() {
                if (c.is_digit(10) && (prev_sep || prev_hl == Number))
                    || (c == '.' && prev_hl == Number)
                {
                    cur_hl = Some(Number);
                }
            }

            if syntax.highlight_keywords() && prev_sep {
                let rest_of_line = &self.render[idx..];
                if let Some((highlight, keyword_len)) =
                    syntax.starts_with_keyword(rest_of_line)
                {
                    in_keyword = Some((highlight, keyword_len - 1));
                    cur_hl = Some(highlight);
                }
            }

            prev_sep = self.is_separator(c);
            self.hl.push(cur_hl.unwrap_or(Normal));
        }
    }

    pub fn clear_overlay_search(&mut self) {
        for elem in self.overlay.iter_mut() {
            *elem = None;
        }
    }

    pub fn set_overlay_search(&mut self, begin: usize, end: usize) {
        for x in begin..end {
            if let Some(elem) = self.overlay.get_mut(x) {
                *elem = Some(Highlight::SearchMatch);
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
        let mut highlights = self.hl.iter().skip(offset).take(cols);
        let mut overlays = self.overlay.iter().skip(offset).take(cols);
        let mut last_highlight = None;

        for c in characters {
            let hl = highlights.next().cloned();
            let overlay = overlays.next().cloned().unwrap_or(None);
            let hl_or_ol = overlay.or(hl).unwrap_or(Highlight::Normal);
            if last_highlight == Some(hl_or_ol) {
                onscreen.push(c);
            } else {
                onscreen.push_str(
                    format!(
                        "\x1b[{}m{}",
                        HL_TO_COLOUR.get(&hl_or_ol).unwrap_or(&DEFAULT_COLOUR),
                        c
                    ).as_str(),
                );
                last_highlight = Some(hl_or_ol);
            }
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

#[cfg(test)]
mod test {
    use highlight::Highlight;
    use row::Row;
    use std::rc::{Rc, Weak};
    use syntax::Syntax;

    lazy_static! {
        static ref SYNTAXES: Vec<Syntax<'static>> = {
            use syntax::SyntaxSetting::*;
            vec![Syntax::new("HLNumbers",
                             vec![],
                             "",
                             vec![],
                             vec![],
                             vec![HighlightNumbers]),
                 Syntax::new("HLStrings",
                             vec![],
                             "",
                             vec![],
                             vec![],
                             vec![HighlightStrings]),
                 Syntax::new("HLComments",
                             vec![],
                             "//",
                             vec![],
                             vec![],
                             vec![HighlightComments]),
                 Syntax::new("HLKeywords",
                             vec![],
                             "",
                             vec!["if", "else", "switch"],
                             vec!["int", "double", "void"],
                             vec![HighlightKeywords]),
                 Syntax::new("HLEverything",
                             vec![],
                             "//",
                             vec!["if", "else", "switch"],
                             vec!["int", "double", "void"],
                             vec![HighlightNumbers,
                                  HighlightStrings,
                                  HighlightKeywords])]
        };
    }

    fn new_row_without_syntax(text: &str) -> Row {
        let syntax: Weak<Option<&Syntax>> = Weak::new();
        Row::new(text, syntax)
    }

    macro_rules! row_with_text_and_filetype {
        ($text:expr, $filetype:expr, $syntax:ident, $row:ident) => (
            let syntax_val = SYNTAXES.iter().find(|s| s.filetype == $filetype);
            assert!(syntax_val.is_some(),
                    "Failed to find syntax with filetype: {}",
                    $filetype);
            let $syntax = Rc::new(syntax_val);
            #[allow(unused_mut)]
            let mut $row = Row::new($text, Rc::downgrade(&$syntax));
        )
    }

    #[test]
    fn test_row_with_highlighted_numbers() {
        row_with_text_and_filetype!("123\r\n", "HLNumbers", syntax, row);
        assert!(
            row.syntax.upgrade().unwrap().unwrap().highlight_numbers(),
            "Should have highlighted numbers"
        );
    }

    #[test]
    fn test_insert_char() {
        let mut row = new_row_without_syntax("a line of text\r\n");
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
        let mut row = new_row_without_syntax("£1.50\r\n");
        row.update();
        assert_eq!(5, row.size);
        assert_eq!(5, row.rsize);
    }

    #[test]
    fn test_set_text() {
        let mut row = new_row_without_syntax("");
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
        let mut row = new_row_without_syntax("this is a nice row\r\n");
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
        let mut row = new_row_without_syntax("££1.50\r\n");
        assert_eq!(6, row.size);
        row.delete_char(1);
        assert_eq!(5, row.size);
        assert_eq!("£1.50", row.render);
    }

    #[test]
    fn test_append_text() {
        let mut row = new_row_without_syntax("this is a line of text.\r\n");
        row.append_text("another line.\r\n");
        assert_eq!("this is a line of text.another line.\r\n", row.chars);
        let mut row = new_row_without_syntax("££\r\n");
        row.append_text("word\r\n");
        assert_eq!("££word\r\n", row.chars);
    }

    #[test]
    fn test_newline() {
        let row = new_row_without_syntax("this is a line.\r\n");
        assert_eq!("\r\n", row.newline());
        let row = new_row_without_syntax("another line.\n");
        assert_eq!("\n", row.newline());
        let row = new_row_without_syntax("££££\r\n");
        assert_eq!("\r\n", row.newline());
    }

    #[test]
    fn test_truncate() {
        let mut row = new_row_without_syntax("first.second.\r\n");
        row.truncate(6);
        assert_eq!("first.\r\n", row.chars);
        let mut row = new_row_without_syntax("£££££.second.\r\n");
        row.truncate(6);
        assert_eq!("£££££.\r\n", row.chars);
    }

    #[test]
    fn test_render_cursor_to_text() {
        {
            let row = new_row_without_syntax("nothing interesting\r\n");
            assert_eq!(5, row.render_cursor_to_text(5));
        }

        {
            let row = new_row_without_syntax("\tinteresting\r\n");
            assert_eq!("        interesting", row.rendered_str());
            assert_eq!(0, row.render_cursor_to_text(0));
            assert_eq!(1, row.render_cursor_to_text(8));
            assert_eq!(11, row.render_cursor_to_text(18));
            // the position after the text (EOL)
            assert_eq!(12, row.render_cursor_to_text(19));
        }

        {
            let row = new_row_without_syntax("\t£intersting\r\n");
            assert_eq!("        £intersting", row.rendered_str());
            assert_eq!(0, row.render_cursor_to_text(0));
            assert_eq!(1, row.render_cursor_to_text(8));
            assert_eq!(2, row.render_cursor_to_text(9));
        }
    }

    #[test]
    fn test_index_of() {
        {
            let row = new_row_without_syntax("nothing interesting\r\n");
            assert_eq!(Some(0), row.index_of("nothing"));
            assert_eq!(Some(8), row.index_of("interesting"));
        }

        {
            let row = new_row_without_syntax("\t£lots\r\n");
            assert_eq!("        £lots", row.rendered_str());
            assert_eq!(Some(9), row.index_of("lots"));
        }
    }

    #[test]
    fn test_onscreen_text_without_syntax() {
        let row = new_row_without_syntax("text\r\n");
        let onscreen = row.onscreen_text(0, 4);
        assert_eq!("\x1b[39mtext\x1b[39m", onscreen);
    }

    #[test]
    fn test_onscreen_text_with_highlighted_numbers_but_no_numbers() {
        row_with_text_and_filetype!(
            "no numbers here\r\n",
            "HLNumbers",
            syntax,
            row
        );
        let onscreen = row.onscreen_text(2, 9);
        assert!(onscreen.contains("\x1b[39m"));
        assert!(!onscreen.contains("\x1b[31m"));
        assert!(onscreen.ends_with("\x1b[39m"));
        assert!(onscreen.starts_with("\x1b[39m"));
        assert_eq!(2, onscreen.matches("\x1b[39m").count());
    }

    #[test]
    fn test_onscreen_text_with_highlighted_numbers_and_some_numbers() {
        row_with_text_and_filetype!(
            "number 19 here\r\n",
            "HLNumbers",
            syntax,
            row
        );
        assert!(row.syntax.upgrade().is_some(), "Should have valid syntax");
        let onscreen = row.onscreen_text(0, 11);
        assert!(onscreen.contains("\x1b[31m1"));
        assert!(onscreen.contains("\x1b[39m"));
        assert!(onscreen.ends_with("\x1b[39m"));
        assert_eq!(3, onscreen.matches("\x1b[39m").count());
        assert_eq!(1, onscreen.matches("\x1b[31m").count());
    }

    #[test]
    fn test_onscreen_text_with_highlighted_numbers_and_overlay_search() {
        row_with_text_and_filetype!(
            "abc123 TEXT zxc987\r\n",
            "HLNumbers",
            syntax,
            row
        );
        row.set_overlay_search(7, 11);
        let onscreen = row.onscreen_text(0, 18);
        assert!(onscreen.contains("\x1b[34mTEXT\x1b[39m"));
    }

    #[test]
    fn test_highlight_normal() {
        row_with_text_and_filetype!("normal\r\n", "HLNumbers", syntax, row);
        assert_eq!(vec![Highlight::Normal; 6], row.hl);
    }

    #[test]
    fn test_highlight_numbers() {
        row_with_text_and_filetype!("12345.6789\r\n", "HLNumbers", syntax, row);
        assert_eq!(vec![Highlight::Number; 10], row.hl);
    }

    #[test]
    fn test_highlight_mixed_numbers_words() {
        row_with_text_and_filetype!(
            "123 abc 456\r\n",
            "HLNumbers",
            syntax,
            row
        );
        let mut expected = vec![];
        expected.append(&mut vec![Highlight::Number; 3]);
        expected.append(&mut vec![Highlight::Normal; 5]);
        expected.append(&mut vec![Highlight::Number; 3]);
        assert_eq!(expected, row.hl);
    }

    #[test]
    fn test_highlight_numbers_in_words_are_normal() {
        row_with_text_and_filetype!("word9\r\n", "HLNumbers", syntax, row);
        assert_eq!(vec![Highlight::Normal; 5], row.hl);
    }

    #[test]
    fn test_highlight_double_quoted_strings() {
        row_with_text_and_filetype!(
            "nah \"STU'FF\" done\r\n",
            "HLStrings",
            syntax,
            row
        );
        let mut expected = vec![];
        expected.append(&mut vec![Highlight::Normal; 4]);
        expected.append(&mut vec![Highlight::String; 8]);
        expected.append(&mut vec![Highlight::Normal; 5]);
        assert_eq!(expected, row.hl);
    }

    #[test]
    fn test_highlight_single_quoted_strings() {
        row_with_text_and_filetype!(
            "nah 'ST\"UFF' done\r\n",
            "HLStrings",
            syntax,
            row
        );
        let mut expected = vec![];
        expected.append(&mut vec![Highlight::Normal; 4]);
        expected.append(&mut vec![Highlight::String; 8]);
        expected.append(&mut vec![Highlight::Normal; 5]);
        assert_eq!(expected, row.hl);
    }

    #[test]
    fn test_onscreen_double_quoted_strings() {
        row_with_text_and_filetype!(
            "nah \"STUFF\" done\r\n",
            "HLStrings",
            syntax,
            row
        );
        assert_eq!(
            "\x1b[39mnah \x1b[35m\"STUFF\"\x1b[39m done\x1b[39m",
            row.onscreen_text(0, 16)
        );
    }

    #[test]
    fn test_highlight_numbers_in_strings() {
        row_with_text_and_filetype!(
            "'abc.12.3zxc'\r\n",
            "HLEverything",
            syntax,
            row
        );
        assert_eq!(vec![Highlight::String; 13], row.hl);
    }

    #[test]
    fn test_highlight_escaped_quotes() {
        row_with_text_and_filetype!(
            "abc \"WO\\\"O\\\"T\" xyz\r\n",
            "HLStrings",
            syntax,
            row
        );
        let mut expected = vec![];
        expected.append(&mut vec![Highlight::Normal; 4]);
        expected.append(&mut vec![Highlight::String; 10]);
        expected.append(&mut vec![Highlight::Normal; 4]);
        assert_eq!(expected, row.hl);
    }

    #[test]
    fn test_highlight_comments() {
        row_with_text_and_filetype!(
            "nothing // and a comment\r\n",
            "HLComments",
            syntax,
            row
        );
        let mut expected = vec![];
        expected.append(&mut vec![Highlight::Normal; 8]);
        expected.append(&mut vec![Highlight::Comment; 16]);
        assert_eq!(expected, row.hl);
    }

    #[test]
    fn test_highlight_ignore_comments() {
        use syntax::SyntaxSetting::*;
        let syntax = Syntax::new(
            "test",
            vec![],
            "",
            vec![],
            vec![],
            vec![HighlightComments],
        );
        let rc = Rc::new(Some(&syntax));
        let row = Row::new("nothing // and a comment\r\n", Rc::downgrade(&rc));
        assert_eq!(vec![Highlight::Normal; 24], row.hl);
    }

    #[test]
    fn test_highlight_keywords1() {
        row_with_text_and_filetype!(
            "if NOTHING else THAT switch\r\n",
            "HLEverything",
            syntax,
            row
        );
        let mut expected = vec![];
        expected.append(&mut vec![Highlight::Keyword1; 2]);
        expected.append(&mut vec![Highlight::Normal; 9]);
        expected.append(&mut vec![Highlight::Keyword1; 4]);
        expected.append(&mut vec![Highlight::Normal; 6]);
        expected.append(&mut vec![Highlight::Keyword1; 6]);
        assert_eq!(expected, row.hl);
    }

    #[test]
    fn test_highlight_keywords2() {
        row_with_text_and_filetype!(
            "int hello; double another; void **\r\n",
            "HLEverything",
            syntax,
            row
        );
        let mut expected = vec![];
        expected.append(&mut vec![Highlight::Keyword2; 3]);
        expected.append(&mut vec![Highlight::Normal; 8]);
        expected.append(&mut vec![Highlight::Keyword2; 6]);
        expected.append(&mut vec![Highlight::Normal; 10]);
        expected.append(&mut vec![Highlight::Keyword2; 4]);
        expected.append(&mut vec![Highlight::Normal; 3]);
        assert_eq!(expected, row.hl);
    }

    #[test]
    fn test_onscreen_keywords1() {
        row_with_text_and_filetype!(
            " if something else {}\r\n",
            "HLEverything",
            syntax,
            row
        );
        let onscreen = row.onscreen_text(0, 21);
        assert!(onscreen.contains("\x1b[33mif\x1b[39m"));
        assert!(onscreen.contains("\x1b[39m something "));
        assert!(onscreen.contains("\x1b[33melse\x1b[39m"));
    }

    #[test]
    fn test_onscreen_keywords2() {
        row_with_text_and_filetype!(
            " int something; double woot; {}\r\n",
            "HLEverything",
            syntax,
            row
        );
        let onscreen = row.onscreen_text(0, 31);
        assert!(onscreen.contains("\x1b[32mint\x1b[39m"));
        assert!(onscreen.contains("\x1b[39m something; "));
        assert!(onscreen.contains("\x1b[32mdouble\x1b[39m"));
        assert!(onscreen.contains("\x1b[39m woot; {}\x1b[39m"));
    }

    #[test]
    fn test_highlight_strings_with_keywords_in_them() {
        row_with_text_and_filetype!("'else'\r\n", "HLEverything", syntax, row);
        assert_eq!(vec![Highlight::String; 6], row.hl);
        row_with_text_and_filetype!(
            "\"else\"\r\n",
            "HLEverything",
            syntax,
            row
        );
        assert_eq!(vec![Highlight::String; 6], row.hl);
    }
}
