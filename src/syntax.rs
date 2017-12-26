use std::path::Path;

#[derive(Debug, Eq, PartialEq)]
pub enum SyntaxSetting {
    HighlightNumbers,
    HighlightStrings,
}

#[derive(Debug, Eq, PartialEq)]
pub struct Syntax<'a> {
    pub filetype: &'a str,
    filematches: Vec<&'a str>,
    flags: Vec<SyntaxSetting>,
}

impl<'a> Syntax<'a> {
    pub fn new(
        filetype: &'a str,
        filematches: Vec<&'a str>,
        flags: Vec<SyntaxSetting>,
    ) -> Self {
        Syntax {
            filetype,
            filematches,
            flags,
        }
    }

    pub fn highlight_numbers(&self) -> bool {
        self.flags.contains(&SyntaxSetting::HighlightNumbers)
    }

    pub fn highlight_strings(&self) -> bool {
        self.flags.contains(&SyntaxSetting::HighlightStrings)
    }

    pub fn matches_filename(&self, filename: &str) -> bool {
        let ext = Path::new(filename).extension();
        self.filematches.iter().any(|filematch| {
            if filematch.starts_with('.') {
                ext.map(|e1| {
                    filematch
                        .split('.')
                        .nth(1)
                        .map(|e2| e1 == e2)
                        .unwrap_or(false)
                }).unwrap_or(false)
            } else {
                false
            }
        })
    }
}

lazy_static! {
    pub static ref SYNTAXES: Vec<Syntax<'static>> = {
        use self::SyntaxSetting::*;
        vec![Syntax::new("C",
                         vec![".c", ".cpp", ".h"],
                         vec![HighlightNumbers, HighlightStrings])]
    };
}

#[test]
fn test_matches_filename() {
    let syntax = Syntax::new("C", vec![".c"], vec![]);
    assert!(syntax.matches_filename("test.c"));
    assert!(!syntax.matches_filename("test.r"));
}

#[test]
fn test_highlight_numbers() {
    let syntax =
        Syntax::new("test", vec![], vec![SyntaxSetting::HighlightNumbers]);
    assert!(syntax.highlight_numbers());
    let syntax = Syntax::new("test", vec![], vec![]);
    assert!(!syntax.highlight_numbers());
}

#[test]
fn test_highlight_strings() {
    let syntax =
        Syntax::new("test", vec![], vec![SyntaxSetting::HighlightStrings]);
    assert!(syntax.highlight_strings());
    let syntax = Syntax::new("test", vec![], vec![]);
    assert!(!syntax.highlight_strings());
}
