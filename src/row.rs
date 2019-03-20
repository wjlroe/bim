use crate::config::TAB_STOP;
use crate::highlight::{Highlight, DEFAULT_COLOUR, HL_TO_COLOUR};
use crate::syntax::Syntax;
use crate::utils::char_position_to_byte_position;
use std::fmt;
use std::rc::Weak;

const SEPARATORS: &str = ",.()+-/*=~%<>[];";
const UNIX_NEWLINE: &str = "\n";
const DOS_NEWLINE: &str = "\r\n";

#[allow(dead_code)]
pub enum Newline {
    Unix,
    Dos,
    Unknown,
}

impl fmt::Display for Newline {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Newline::Unix => write!(f, "{}", UNIX_NEWLINE),
            Newline::Dos => write!(f, "{}", DOS_NEWLINE),
            Newline::Unknown => write!(f, "{}", DEFAULT_NEWLINE.to_string()),
        }
    }
}

#[cfg(windows)]
pub const DEFAULT_NEWLINE: Newline = Newline::Dos;
#[cfg(not(windows))]
pub const DEFAULT_NEWLINE: Newline = Newline::Unix;

struct RenderCursor {
    text_cursor: i32,
    render_cursor: i32,
}

impl RenderCursor {
    fn new(text_cursor: i32, render_cursor: i32) -> Self {
        Self {
            text_cursor,
            render_cursor,
        }
    }
}

struct RenderCursorIter<'a> {
    text_cursor: i32,
    render_cursor: i32,
    source: std::str::Chars<'a>,
}

impl<'a> RenderCursorIter<'a> {
    fn new(source: std::str::Chars<'a>) -> Self {
        Self {
            source,
            text_cursor: 0,
            render_cursor: 0,
        }
    }
}

impl<'a> Iterator for RenderCursorIter<'a> {
    type Item = RenderCursor;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(source_char) = self.source.next() {
            let item = RenderCursor::new(self.text_cursor, self.render_cursor);
            if source_char == '\t' {
                self.render_cursor +=
                    (TAB_STOP as i32 - 1) - (self.render_cursor % TAB_STOP as i32);
            }
            self.render_cursor += 1;
            self.text_cursor += 1;
            Some(item)
        } else {
            None
        }
    }
}

pub struct Row<'a> {
    chars: String,
    pub size: usize,
    pub render: String,
    rsize: usize,
    pub hl: Vec<Highlight>,
    overlay: Vec<Option<Highlight>>,
    syntax: Weak<Option<&'a Syntax<'a>>>,
    pub hl_open_comment: bool,
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
            hl_open_comment: false,
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
        self.render.push('\n'); // Internally use unix line endings ignoring source line endings
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

    pub fn update_syntax_highlight(&mut self, previous_ml_comment: bool) -> bool {
        use crate::highlight::Highlight::*;

        self.hl.clear();
        let syntax = self
            .syntax
            .upgrade()
            .unwrap_or_else(|| std::rc::Rc::new(None));
        if syntax.is_none() {
            for _ in 0..=self.rsize {
                self.hl.push(Normal);
            }
            return false;
        }
        let syntax = syntax.unwrap();

        let mut prev_sep = true;
        let mut in_string: Option<char> = None;
        let mut escaped_quote = false;
        let mut in_highlight: Option<(Highlight, usize)> = None;
        let mut in_comment = previous_ml_comment;
        for (hl_idx, (idx, c)) in (0..).zip(self.render.char_indices()) {
            let mut cur_hl = None;
            let prev_hl = if hl_idx > 0 {
                self.hl.get(hl_idx - 1).cloned().unwrap_or(Normal)
            } else {
                Normal
            };

            if let Some((_, 0)) = in_highlight {
                in_highlight = None;
            }

            if let Some(val) = in_highlight.as_mut() {
                val.1 -= 1;
                self.hl.push(val.0);
                continue;
            }

            if syntax.highlight_singleline_comments() && in_string.is_none() && !in_comment {
                let rest_of_line = &self.render[idx..];
                if rest_of_line.starts_with(syntax.singleline_comment_start) {
                    for _ in idx..self.rsize {
                        self.hl.push(Comment);
                    }
                    self.hl.push(Normal); // newline
                    break;
                }
            }

            if syntax.highlight_multiline_comments() && in_string.is_none() {
                let rest_of_line = &self.render[idx..];
                if in_comment && rest_of_line.starts_with(syntax.multiline_comment_end) {
                    in_comment = false;
                    in_highlight = Some((MultilineComment, syntax.multiline_comment_end.len() - 1));
                    self.hl.push(MultilineComment);
                    continue;
                }
                if rest_of_line.starts_with(syntax.multiline_comment_start) {
                    in_comment = true;
                    in_highlight =
                        Some((MultilineComment, syntax.multiline_comment_start.len() - 1));
                    self.hl.push(MultilineComment);
                    continue;
                }
                if in_comment {
                    let hl = if c == '\n' || c == '\r' {
                        Normal
                    } else {
                        MultilineComment
                    };
                    self.hl.push(hl);
                    continue;
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
                } else if c == '\'' || c == '"' {
                    in_string = Some(c);
                    cur_hl = Some(String);
                }
            }

            if syntax.highlight_numbers()
                && cur_hl.is_none()
                && ((c.is_digit(10) && (prev_sep || prev_hl == Number))
                    || (c == '.' && prev_hl == Number))
            {
                cur_hl = Some(Number);
            }

            if syntax.highlight_keywords() && prev_sep {
                let rest_of_line = &self.render[idx..];
                if let Some((highlight, keyword_len)) = syntax.starts_with_keyword(rest_of_line) {
                    let next_char = self.render.chars().skip(idx + keyword_len).nth(0);
                    if next_char.is_none() || self.is_separator(next_char.unwrap()) {
                        in_highlight = Some((highlight, keyword_len - 1));
                        cur_hl = Some(highlight);
                    }
                }
            }

            prev_sep = self.is_separator(c);
            self.hl.push(cur_hl.unwrap_or(Normal));
        }
        in_comment
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

    fn to_render_cursor_iter(&self) -> RenderCursorIter {
        RenderCursorIter::new(self.as_str().chars())
    }

    pub fn text_cursor_to_render(&self, cidx: i32) -> i32 {
        self.to_render_cursor_iter()
            .find(|render_cursor| render_cursor.text_cursor == cidx)
            .map(|render_cursor| render_cursor.render_cursor)
            .unwrap_or(0)
    }

    pub fn render_cursor_to_text(&self, ridx: usize) -> usize {
        self.to_render_cursor_iter()
            .find(|render_cursor| render_cursor.render_cursor == ridx as i32)
            .map(|render_cursor| render_cursor.text_cursor)
            .unwrap_or(0) as usize
    }

    fn render_cursor_to_byte_position(&self, at: usize) -> usize {
        char_position_to_byte_position(&self.chars, at)
    }

    fn byte_position_to_char_position(&self, at: usize) -> usize {
        self.render[0..=at].chars().count() - 1
    }

    pub fn insert_char(&mut self, at: usize, character: char) {
        let at = if at > self.size { self.size } else { at };
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
        let newline = String::from(&self.chars[byte_pos..]);
        if newline.is_empty() {
            DEFAULT_NEWLINE.to_string()
        } else {
            newline
        }
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

    // FIXME: This doesn't belong here - it's terminal-specific
    pub fn onscreen_text(&self, offset: usize, cols: usize) -> String {
        let mut onscreen = String::new();
        // FIXME: call rendered_str here and slice it up!
        let characters = self.render.chars().skip(offset).take(cols);
        let mut highlights = self.hl.iter().skip(offset).take(cols);
        let mut overlays = self.overlay.iter().skip(offset).take(cols);
        let mut last_highlight = None;

        for c in characters {
            if c == '\n' {
                break;
            }

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
                    )
                    .as_str(),
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
    use crate::highlight::Highlight;
    use crate::row::{Row, DEFAULT_NEWLINE};
    use crate::syntax::Syntax;
    use lazy_static::lazy_static;
    use std::rc::{Rc, Weak};

    fn test_syntaxes() -> Vec<Syntax<'static>> {
        use crate::syntax::SyntaxSetting::*;
        vec![
            Syntax::new("HLNumbers").flag(HighlightNumbers),
            Syntax::new("HLStrings").flag(HighlightStrings),
            Syntax::new("HLComments")
                .flag(HighlightComments)
                .singleline_comment_start("//"),
            Syntax::new("HLMLComments")
                .flag(HighlightComments)
                .multiline_comment_start("/*")
                .multiline_comment_end("*/"),
            Syntax::new("HLKeywords")
                .flag(HighlightKeywords)
                .keywords1(&["if", "else", "switch"])
                .keywords2(&["int", "double", "void"]),
            Syntax::new("HLEverything")
                .flag(HighlightNumbers)
                .flag(HighlightStrings)
                .flag(HighlightKeywords)
                .flag(HighlightComments)
                .singleline_comment_start("//")
                .multiline_comment_start("/*")
                .multiline_comment_end("*/")
                .keywords1(&["if", "else", "switch"])
                .keywords2(&["int", "double", "void"]),
        ]
    }

    lazy_static! {
        static ref SYNTAXES: Vec<Syntax<'static>> = test_syntaxes();
    }

    fn new_row_without_syntax(text: &str) -> Row<'_> {
        let syntax: Weak<Option<&Syntax<'_>>> = Weak::new();
        Row::new(text, syntax)
    }

    #[rustfmt::skip]
    macro_rules! row_with_text_and_filetype {
        ($text:expr, $filetype:expr, $syntax:ident, $row:ident) => (
            let syntax_val = SYNTAXES.iter().find(|s| s.filetype == $filetype);
            assert!(
                syntax_val.is_some(),
                "Failed to find syntax with filetype: {}",
                $filetype
            );
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
        assert_eq!(row.chars.trim(), row.render.trim());
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
        assert_eq!("this is a nice row\n", row.render);

        row.delete_char(0);
        assert_eq!("his is a nice row\r\n", row.chars);
        assert_eq!(17, row.size);
        assert_eq!("his is a nice row\n", row.render);

        row.delete_char(17);
        assert_eq!("his is a nice ro\r\n", row.chars);
        assert_eq!(16, row.size);
        assert_eq!("his is a nice ro\n", row.render);
    }

    #[test]
    fn test_delete_char_utf8() {
        let mut row = new_row_without_syntax("££1.50\r\n");
        assert_eq!(6, row.size);
        row.delete_char(1);
        assert_eq!(5, row.size);
        assert_eq!("£1.50\n", row.render);
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
        let row = new_row_without_syntax("no newline");
        assert_eq!(DEFAULT_NEWLINE.to_string(), row.newline());
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
            assert_eq!("        interesting\n", row.rendered_str());
            assert_eq!(0, row.render_cursor_to_text(0));
            assert_eq!(1, row.render_cursor_to_text(8));
            assert_eq!(11, row.render_cursor_to_text(18));
            // the position after the text (EOL)
            assert_eq!(12, row.render_cursor_to_text(19));
        }

        {
            let row = new_row_without_syntax("\t£intersting\r\n");
            assert_eq!("        £intersting\n", row.rendered_str());
            assert_eq!(0, row.render_cursor_to_text(0));
            assert_eq!(1, row.render_cursor_to_text(8));
            assert_eq!(2, row.render_cursor_to_text(9));
        }
    }

    #[test]
    fn test_text_cursor_to_render() {
        {
            let row = new_row_without_syntax("nothing interesting\r\n");
            assert_eq!(5, row.text_cursor_to_render(5));
        }

        {
            let row = new_row_without_syntax("\tinteresting\r\n");
            assert_eq!(0, row.text_cursor_to_render(0));
            assert_eq!(8, row.text_cursor_to_render(1));
            assert_eq!(9, row.text_cursor_to_render(2));
        }

        {
            let row = new_row_without_syntax("\t£interesting\r\n");
            assert_eq!(0, row.text_cursor_to_render(0));
            assert_eq!(8, row.text_cursor_to_render(1));
            assert_eq!(9, row.text_cursor_to_render(2));
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
            assert_eq!("        £lots\n", row.rendered_str());
            assert_eq!(Some(9), row.index_of("lots"));
        }
    }

    #[test]
    fn test_onscreen_text_without_syntax() {
        let mut row = new_row_without_syntax("text\r\n");
        row.update_syntax_highlight(false);
        let onscreen = row.onscreen_text(0, 4);
        assert_eq!("\x1b[39mtext\x1b[39m", onscreen);
    }

    #[test]
    fn test_onscreen_text_with_highlighted_numbers_but_no_numbers() {
        row_with_text_and_filetype!("no numbers here\r\n", "HLNumbers", syntax, row);
        row.update_syntax_highlight(false);
        let onscreen = row.onscreen_text(2, 9);
        assert!(onscreen.contains("\x1b[39m"));
        assert!(!onscreen.contains("\x1b[31m"));
        assert!(onscreen.ends_with("\x1b[39m"));
        assert!(onscreen.starts_with("\x1b[39m"));
        assert_eq!(2, onscreen.matches("\x1b[39m").count());
    }

    #[test]
    fn test_onscreen_text_with_highlighted_numbers_and_some_numbers() {
        row_with_text_and_filetype!("number 19 here\r\n", "HLNumbers", syntax, row);
        row.update_syntax_highlight(false);
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
        row_with_text_and_filetype!("abc123 TEXT zxc987\r\n", "HLNumbers", syntax, row);
        row.update_syntax_highlight(false);
        row.set_overlay_search(7, 11);
        let onscreen = row.onscreen_text(0, 18);
        assert!(onscreen.contains("\x1b[34mTEXT\x1b[39m"));
    }

    #[test]
    fn test_highlight_normal() {
        row_with_text_and_filetype!("normal\r\n", "HLNumbers", syntax, row);
        row.update_syntax_highlight(false);
        let mut highlights = vec![Highlight::Normal; 6];
        highlights.push(Highlight::Normal); // newline
        assert_eq!(highlights, row.hl);
    }

    #[test]
    fn test_highlight_numbers() {
        row_with_text_and_filetype!("12345.6789\r\n", "HLNumbers", syntax, row);
        row.update_syntax_highlight(false);
        let mut highlights = vec![Highlight::Number; 10];
        highlights.push(Highlight::Normal); // newline
        assert_eq!(highlights, row.hl);
    }

    #[test]
    fn test_highlight_mixed_numbers_words() {
        row_with_text_and_filetype!("123 £abc 456\r\n", "HLNumbers", syntax, row);
        row.update_syntax_highlight(false);
        let mut expected = vec![];
        expected.append(&mut vec![Highlight::Number; 3]);
        expected.append(&mut vec![Highlight::Normal; 6]);
        expected.append(&mut vec![Highlight::Number; 3]);
        expected.push(Highlight::Normal); // newline
        assert_eq!(expected, row.hl);
    }

    #[test]
    fn test_highlight_numbers_in_words_are_normal() {
        row_with_text_and_filetype!("word9\r\n", "HLNumbers", syntax, row);
        row.update_syntax_highlight(false);
        let mut highlights = vec![Highlight::Normal; 5];
        highlights.push(Highlight::Normal); // newline
        assert_eq!(highlights, row.hl);
    }

    #[test]
    fn test_highlight_double_quoted_strings() {
        row_with_text_and_filetype!("nah \"STU'FF\" done\r\n", "HLStrings", syntax, row);
        row.update_syntax_highlight(false);
        let mut expected = vec![];
        expected.append(&mut vec![Highlight::Normal; 4]);
        expected.append(&mut vec![Highlight::String; 8]);
        expected.append(&mut vec![Highlight::Normal; 5]);
        expected.push(Highlight::Normal); // newline
        assert_eq!(expected, row.hl);
    }

    #[test]
    fn test_highlight_single_quoted_strings() {
        row_with_text_and_filetype!("nah 'ST\"UFF' done\r\n", "HLStrings", syntax, row);
        row.update_syntax_highlight(false);
        let mut expected = vec![];
        expected.append(&mut vec![Highlight::Normal; 4]);
        expected.append(&mut vec![Highlight::String; 8]);
        expected.append(&mut vec![Highlight::Normal; 5]);
        expected.push(Highlight::Normal); // newline
        assert_eq!(expected, row.hl);
    }

    #[test]
    fn test_onscreen_double_quoted_strings() {
        row_with_text_and_filetype!("nah \"STUFF\" done\r\n", "HLStrings", syntax, row);
        row.update_syntax_highlight(false);
        assert_eq!(
            "\x1b[39mnah \x1b[35m\"STUFF\"\x1b[39m done\x1b[39m",
            row.onscreen_text(0, 16)
        );
    }

    #[test]
    fn test_highlight_numbers_in_strings() {
        row_with_text_and_filetype!("'abc.12.3zxc'\r\n", "HLEverything", syntax, row);
        row.update_syntax_highlight(false);
        let mut highlights = vec![Highlight::String; 13];
        highlights.push(Highlight::Normal); // newline
        assert_eq!(highlights, row.hl);
    }

    #[test]
    fn test_highlight_escaped_quotes() {
        row_with_text_and_filetype!("abc \"WO\\\"O\\\"T\" xyz\r\n", "HLStrings", syntax, row);
        row.update_syntax_highlight(false);
        let mut expected = vec![];
        expected.append(&mut vec![Highlight::Normal; 4]);
        expected.append(&mut vec![Highlight::String; 10]);
        expected.append(&mut vec![Highlight::Normal; 4]);
        expected.push(Highlight::Normal); // newline
        assert_eq!(expected, row.hl);
    }

    #[test]
    fn test_highlight_singleline_comments() {
        row_with_text_and_filetype!("nothing // and a comment\r\n", "HLComments", syntax, row);
        row.update_syntax_highlight(false);
        let mut expected = vec![];
        expected.append(&mut vec![Highlight::Normal; 8]);
        expected.append(&mut vec![Highlight::Comment; 16]);
        expected.push(Highlight::Normal); // newline
        assert_eq!(expected, row.hl);
    }

    #[test]
    fn test_highlight_ignore_singleline_comments() {
        use crate::syntax::SyntaxSetting::*;
        let syntax = Syntax::new("test").flag(HighlightComments);
        let rc = Rc::new(Some(&syntax));
        let mut row = Row::new("nothing // and a comment\r\n", Rc::downgrade(&rc));
        row.update_syntax_highlight(false);
        let mut highlights = vec![Highlight::Normal; 24];
        highlights.push(Highlight::Normal); // newline
        assert_eq!(highlights, row.hl);
    }

    #[test]
    fn test_highlight_keywords1() {
        row_with_text_and_filetype!(
            "if NOTHING else THAT switch\r\n",
            "HLEverything",
            syntax,
            row
        );
        row.update_syntax_highlight(false);
        let mut expected = vec![];
        expected.append(&mut vec![Highlight::Keyword1; 2]);
        expected.append(&mut vec![Highlight::Normal; 9]);
        expected.append(&mut vec![Highlight::Keyword1; 4]);
        expected.append(&mut vec![Highlight::Normal; 6]);
        expected.append(&mut vec![Highlight::Keyword1; 6]);
        expected.push(Highlight::Normal); // newline
        assert_eq!(expected, row.hl);
    }

    #[test]
    fn test_highlight_keywords_whole_words() {
        row_with_text_and_filetype!("row->ints = switchAroo;\r\n", "HLEverything", syntax, row);
        row.update_syntax_highlight(false);
        assert!(row.hl.contains(&Highlight::Normal));
        assert!(!row.hl.contains(&Highlight::Keyword1));
        assert!(!row.hl.contains(&Highlight::Keyword2));
    }

    #[test]
    fn test_highlight_keywords2() {
        row_with_text_and_filetype!(
            "int hello; double another; void **\r\n",
            "HLEverything",
            syntax,
            row
        );
        row.update_syntax_highlight(false);
        let mut expected = vec![];
        expected.append(&mut vec![Highlight::Keyword2; 3]);
        expected.append(&mut vec![Highlight::Normal; 8]);
        expected.append(&mut vec![Highlight::Keyword2; 6]);
        expected.append(&mut vec![Highlight::Normal; 10]);
        expected.append(&mut vec![Highlight::Keyword2; 4]);
        expected.append(&mut vec![Highlight::Normal; 3]);
        expected.push(Highlight::Normal); // newline
        assert_eq!(expected, row.hl);
    }

    #[test]
    fn test_onscreen_keywords1() {
        row_with_text_and_filetype!(" if something else {}\r\n", "HLEverything", syntax, row);
        row.update_syntax_highlight(false);
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
        row.update_syntax_highlight(false);
        let onscreen = row.onscreen_text(0, 31);
        assert!(onscreen.contains("\x1b[32mint\x1b[39m"));
        assert!(onscreen.contains("\x1b[39m something; "));
        assert!(onscreen.contains("\x1b[32mdouble\x1b[39m"));
        assert!(onscreen.contains("\x1b[39m woot; {}\x1b[39m"));
    }

    #[test]
    fn test_highlight_strings_with_keywords_in_them() {
        {
            row_with_text_and_filetype!("'else'\r\n", "HLEverything", syntax, row);
            row.update_syntax_highlight(false);
            let mut highlights = vec![Highlight::String; 6];
            highlights.push(Highlight::Normal); // newline
            assert_eq!(highlights, row.hl);
        }

        {
            row_with_text_and_filetype!("\"else\"\r\n", "HLEverything", syntax, row);
            row.update_syntax_highlight(false);
            let mut highlights = vec![Highlight::String; 6];
            highlights.push(Highlight::Normal); // newline
            assert_eq!(highlights, row.hl);
        }
    }

    #[test]
    fn test_highlight_multiline_comments_on_one_line() {
        row_with_text_and_filetype!("int 1; /* blah */\r\n", "HLMLComments", syntax, row);
        assert_eq!(false, row.update_syntax_highlight(false));
        let mut expected = vec![Highlight::Normal; 7];
        expected.append(&mut vec![Highlight::MultilineComment; 10]);
        expected.push(Highlight::Normal); // newline
        assert_eq!(expected, row.hl);
    }

    #[test]
    fn test_highlight_multiline_comments_start() {
        row_with_text_and_filetype!("int 1; /* blah\r\n", "HLMLComments", syntax, row);
        assert_eq!(true, row.update_syntax_highlight(false));
        let mut expected = vec![Highlight::Normal; 7];
        expected.append(&mut vec![Highlight::MultilineComment; 7]);
        expected.push(Highlight::Normal); // newline
        assert_eq!(expected, row.hl);
    }

    #[test]
    fn test_highlight_multiline_comments_end() {
        row_with_text_and_filetype!("blah */ int 1;\r\n", "HLMLComments", syntax, row);
        assert_eq!(false, row.update_syntax_highlight(true));
        let mut expected = vec![Highlight::MultilineComment; 7];
        expected.append(&mut vec![Highlight::Normal; 7]);
        expected.push(Highlight::Normal); // newline
        assert_eq!(expected, row.hl);
    }

    #[test]
    fn test_highlight_multiline_comments_continue() {
        row_with_text_and_filetype!("this is in a comment\r\n", "HLMLComments", syntax, row);
        assert_eq!(true, row.update_syntax_highlight(true));
        let mut expected = vec![Highlight::MultilineComment; 20];
        expected.push(Highlight::Normal); // newline
        assert_eq!(expected, row.hl);
    }

    #[test]
    fn test_highlight_multiline_comments_then_keywords() {
        row_with_text_and_filetype!("blah */ int 1;\r\n", "HLEverything", syntax, row);
        assert_eq!(false, row.update_syntax_highlight(true));
        let mut expected = vec![Highlight::MultilineComment; 7];
        expected.append(&mut vec![Highlight::Normal]);
        expected.append(&mut vec![Highlight::Keyword2; 3]);
        expected.append(&mut vec![Highlight::Normal]);
        expected.append(&mut vec![Highlight::Number]);
        expected.append(&mut vec![Highlight::Normal]);
        expected.push(Highlight::Normal); // newline
        assert_eq!(expected, row.hl);
    }

    #[test]
    fn test_highlight_comments_inside_strings() {
        row_with_text_and_filetype!("\"/* blah */\"\r\n", "HLEverything", syntax, row);
        assert_eq!(false, row.update_syntax_highlight(false));
        let mut expected = vec![Highlight::String; 12];
        expected.push(Highlight::Normal); // newline
        assert_eq!(expected, row.hl);
    }

    #[test]
    fn test_highlight_singleline_comments_inside_multiline_comments() {
        row_with_text_and_filetype!("/* // blah */\r\n", "HLEverything", syntax, row);
        assert_eq!(false, row.update_syntax_highlight(false));
        let mut highlights = vec![Highlight::MultilineComment; 13];
        highlights.push(Highlight::Normal); // newline
        assert_eq!(highlights, row.hl);
    }
}
