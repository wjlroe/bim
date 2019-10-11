use crate::highlight::{Highlight, DEFAULT_COLOUR, HL_TO_COLOUR};
use crate::row::Row;

pub trait TerminalRow {
    fn onscreen_text(&self, offset: usize, cols: usize) -> String;
}

impl<'a> TerminalRow for Row<'a> {
    fn onscreen_text(&self, offset: usize, cols: usize) -> String {
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
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::row::*;
    use crate::syntax::*;
    use std::rc::Rc;

    fn row_with_syntax<'a>(
        text: &'a str,
        filetype: &str,
    ) -> (Row<'a>, Rc<Option<&'a Syntax<'static>>>) {
        let syntax = Syntax::for_filetype(filetype);
        assert!(syntax.is_some(), "Syntax for {} should be found", filetype);
        let mut row = Row::new_wo_syntax(text);
        let rc = Rc::new(syntax);
        row.set_syntax(Rc::downgrade(&rc));
        (row, rc)
    }

    #[test]
    fn test_onscreen_text_without_syntax() {
        let mut row = Row::new_wo_syntax("text\r\n");
        row.update_syntax_highlight(false);
        let onscreen = row.onscreen_text(0, 4);
        assert_eq!("\x1b[39mtext\x1b[39m", onscreen);
    }

    #[test]
    fn test_onscreen_text_with_highlighted_numbers_but_no_numbers() {
        let (mut row, _rc) = row_with_syntax("no numbers here\r\n", "C");
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
        let (mut row, _rc) = row_with_syntax("number 19 here\r\n", "C");
        row.update_syntax_highlight(false);
        let onscreen = row.onscreen_text(0, 11);
        assert!(onscreen.contains("\x1b[31m1"));
        assert!(onscreen.contains("\x1b[39m"));
        assert!(onscreen.ends_with("\x1b[39m"));
        assert_eq!(3, onscreen.matches("\x1b[39m").count());
        assert_eq!(1, onscreen.matches("\x1b[31m").count());
    }

    #[test]
    fn test_onscreen_text_with_highlighted_numbers_and_overlay_search() {
        let (mut row, _rc) = row_with_syntax("abc123 TEXT zxc987\r\n", "C");
        row.update_syntax_highlight(false);
        row.set_overlay_search(7, 11);
        let onscreen = row.onscreen_text(0, 18);
        assert!(onscreen.contains("\x1b[34mTEXT\x1b[39m"));
    }

    #[test]
    fn test_onscreen_double_quoted_strings() {
        let (mut row, _rc) = row_with_syntax("nah \"STUFF\" done\r\n", "C");
        row.update_syntax_highlight(false);
        assert_eq!(
            "\x1b[39mnah \x1b[35m\"STUFF\"\x1b[39m done\x1b[39m",
            row.onscreen_text(0, 16)
        );
    }

    #[test]
    fn test_onscreen_keywords1() {
        let (mut row, _rc) = row_with_syntax(" if something else {}\r\n", "C");
        row.update_syntax_highlight(false);
        let onscreen = row.onscreen_text(0, 21);
        assert!(onscreen.contains("\x1b[33mif\x1b[39m"));
        assert!(onscreen.contains("\x1b[39m something "));
        assert!(onscreen.contains("\x1b[33melse\x1b[39m"));
    }

    #[test]
    fn test_onscreen_keywords2() {
        let (mut row, _rc) = row_with_syntax(" int something; double woot; {}\r\n", "C");
        row.update_syntax_highlight(false);
        let onscreen = row.onscreen_text(0, 31);
        assert!(onscreen.contains("\x1b[32mint\x1b[39m"));
        assert!(onscreen.contains("\x1b[39m something; "));
        assert!(onscreen.contains("\x1b[32mdouble\x1b[39m"));
        assert!(onscreen.contains("\x1b[39m woot; {}\x1b[39m"));
    }
}
