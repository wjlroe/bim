use highlight::Highlight;
use std::path::Path;

#[derive(Debug, Eq, PartialEq)]
pub enum SyntaxSetting {
    HighlightNumbers,
    HighlightStrings,
    HighlightComments,
    HighlightKeywords,
}

#[derive(Debug, Eq, PartialEq)]
pub struct Syntax<'a> {
    pub filetype: &'a str,
    filematches: Vec<&'a str>,
    pub singleline_comment_start: &'a str,
    keywords1: Vec<&'a str>,
    keywords2: Vec<&'a str>,
    flags: Vec<SyntaxSetting>,
}

impl<'a> Syntax<'a> {
    pub fn new(
        filetype: &'a str,
        filematches: Vec<&'a str>,
        singleline_comment_start: &'a str,
        keywords1: Vec<&'a str>,
        keywords2: Vec<&'a str>,
        flags: Vec<SyntaxSetting>,
    ) -> Self {
        Syntax {
            filetype,
            filematches,
            singleline_comment_start,
            keywords1,
            keywords2,
            flags,
        }
    }

    pub fn highlight_numbers(&self) -> bool {
        self.flags.contains(&SyntaxSetting::HighlightNumbers)
    }

    pub fn highlight_strings(&self) -> bool {
        self.flags.contains(&SyntaxSetting::HighlightStrings)
    }

    pub fn highlight_comments(&self) -> bool {
        self.flags.contains(&SyntaxSetting::HighlightComments)
            && self.singleline_comment_start.len() > 0
    }

    pub fn highlight_keywords(&self) -> bool {
        self.flags.contains(&SyntaxSetting::HighlightKeywords)
    }

    pub fn highlight_keyword1(&self, haystack: &str) -> Option<usize> {
        self.keywords1
            .iter()
            .find(|keyword| haystack.starts_with(*keyword))
            .map(|keyword| keyword.len())
    }

    pub fn highlight_keyword2(&self, haystack: &str) -> Option<usize> {
        self.keywords2
            .iter()
            .find(|keyword| haystack.starts_with(*keyword))
            .map(|keyword| keyword.len())
    }

    pub fn starts_with_keyword(
        &self,
        haystack: &str,
    ) -> Option<(Highlight, usize)> {
        self.highlight_keyword1(haystack)
            .map(|size| (Highlight::Keyword1, size))
            .or(self.highlight_keyword2(haystack)
                .map(|size| (Highlight::Keyword2, size)))
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
                         "//",
                         vec!["switch", "if", "while", "for", "break",
                              "continue", "return", "else", "struct", "union",
                              "typedef", "static", "enum", "class", "case"],
                         vec!["int", "long", "double", "float", "char",
                              "unsigned", "signed", "void"],
                         vec![HighlightNumbers,
                              HighlightStrings,
                              HighlightComments,
                              HighlightKeywords])]
    };
}

#[test]
fn test_matches_filename() {
    let syntax = Syntax::new("C", vec![".c"], "", vec![], vec![], vec![]);
    assert!(syntax.matches_filename("test.c"));
    assert!(!syntax.matches_filename("test.r"));
}

#[test]
fn test_highlight_numbers() {
    let syntax = Syntax::new(
        "test",
        vec![],
        "",
        vec![],
        vec![],
        vec![SyntaxSetting::HighlightNumbers],
    );
    assert!(syntax.highlight_numbers());
    let syntax = Syntax::new("test", vec![], "", vec![], vec![], vec![]);
    assert!(!syntax.highlight_numbers());
}

#[test]
fn test_highlight_strings() {
    let syntax = Syntax::new(
        "test",
        vec![],
        "",
        vec![],
        vec![],
        vec![SyntaxSetting::HighlightStrings],
    );
    assert!(syntax.highlight_strings());
    let syntax = Syntax::new("test", vec![], "", vec![], vec![], vec![]);
    assert!(!syntax.highlight_strings());
}

#[test]
fn test_highlight_comments() {
    let syntax = Syntax::new(
        "test",
        vec![],
        "//",
        vec![],
        vec![],
        vec![SyntaxSetting::HighlightComments],
    );
    assert!(syntax.highlight_comments());
    let syntax = Syntax::new(
        "test",
        vec![],
        "",
        vec![],
        vec![],
        vec![SyntaxSetting::HighlightComments],
    );
    assert!(!syntax.highlight_comments());
    let syntax = Syntax::new("test", vec![], "", vec![], vec![], vec![]);
    assert!(!syntax.highlight_comments());
}

#[test]
fn test_highlight_keywords() {
    let syntax = Syntax::new(
        "test",
        vec![],
        "",
        vec![],
        vec![],
        vec![SyntaxSetting::HighlightKeywords],
    );
    assert!(syntax.highlight_keywords());
    let syntax = Syntax::new("test", vec![], "", vec![], vec![], vec![]);
    assert!(!syntax.highlight_keywords());
}

#[test]
fn test_starts_with_keyword_keyword1() {
    let syntax = Syntax::new(
        "test",
        vec![],
        "",
        vec!["if"],
        vec![],
        vec![SyntaxSetting::HighlightKeywords],
    );
    assert_eq!(
        Some((Highlight::Keyword1, 2)),
        syntax.starts_with_keyword("if something")
    );
    assert_eq!(None, syntax.starts_with_keyword(" if else blah"));
}

#[test]
fn test_starts_with_keyword_keyword2() {
    let syntax = Syntax::new(
        "test",
        vec![],
        "",
        vec![],
        vec!["int"],
        vec![SyntaxSetting::HighlightKeywords],
    );
    assert_eq!(
        Some((Highlight::Keyword2, 3)),
        syntax.starts_with_keyword("int woot;")
    );
    assert_eq!(None, syntax.starts_with_keyword(" int woot;"));
}
