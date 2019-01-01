use crate::highlight::Highlight;
use lazy_static::lazy_static;
use std::collections::HashMap;
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
    pub multiline_comment_start: &'a str,
    pub multiline_comment_end: &'a str,
    keywords: HashMap<Highlight, Vec<&'a str>>,
    flags: Vec<SyntaxSetting>,
}

impl<'a> Syntax<'a> {
    pub fn new(filetype: &'a str) -> Self {
        Syntax {
            filetype,
            filematches: Vec::new(),
            singleline_comment_start: "",
            multiline_comment_start: "",
            multiline_comment_end: "",
            keywords: HashMap::new(),
            flags: Vec::new(),
        }
    }

    #[allow(dead_code)]
    pub fn filematch(mut self, filematch: &'a str) -> Syntax<'_> {
        self.filematches.push(filematch);
        self
    }

    pub fn filematches(mut self, filematches: &'a [&'a str]) -> Syntax<'_> {
        for filematch in filematches {
            self.filematches.push(filematch);
        }
        self
    }

    pub fn keywords1(mut self, keywords1: &'a [&'a str]) -> Syntax<'_> {
        {
            let keywords = self
                .keywords
                .entry(Highlight::Keyword1)
                .or_insert(Vec::new());
            for keyword in keywords1 {
                keywords.push(keyword);
            }
        }
        self
    }

    pub fn keywords2(mut self, keywords2: &'a [&'a str]) -> Syntax<'_> {
        {
            let keywords = self
                .keywords
                .entry(Highlight::Keyword2)
                .or_insert(Vec::new());
            for keyword in keywords2 {
                keywords.push(keyword);
            }
        }
        self
    }

    pub fn singleline_comment_start(
        mut self,
        singleline: &'a str,
    ) -> Syntax<'_> {
        self.singleline_comment_start = singleline;
        self
    }

    pub fn multiline_comment_start(mut self, marker: &'a str) -> Syntax<'_> {
        self.multiline_comment_start = marker;
        self
    }

    pub fn multiline_comment_end(mut self, marker: &'a str) -> Syntax<'_> {
        self.multiline_comment_end = marker;
        self
    }

    pub fn flag(mut self, flag: SyntaxSetting) -> Syntax<'a> {
        self.flags.push(flag);
        self
    }

    pub fn highlight_numbers(&self) -> bool {
        self.flags.contains(&SyntaxSetting::HighlightNumbers)
    }

    pub fn highlight_strings(&self) -> bool {
        self.flags.contains(&SyntaxSetting::HighlightStrings)
    }

    pub fn highlight_singleline_comments(&self) -> bool {
        self.flags.contains(&SyntaxSetting::HighlightComments)
            && self.singleline_comment_start.len() > 0
    }

    pub fn highlight_multiline_comments(&self) -> bool {
        self.flags.contains(&SyntaxSetting::HighlightComments)
            && self.multiline_comment_start.len() > 0
            && self.multiline_comment_end.len() > 0
    }

    pub fn highlight_keywords(&self) -> bool {
        self.flags.contains(&SyntaxSetting::HighlightKeywords)
    }

    pub fn starts_with_keyword(
        &self,
        haystack: &str,
    ) -> Option<(Highlight, usize)> {
        for (highlight, keywords) in &self.keywords {
            let found_keyword = keywords
                .iter()
                .find(|keyword| haystack.starts_with(*keyword))
                .map(|keyword| (*highlight, keyword.len()));
            if found_keyword.is_some() {
                return found_keyword;
            }
        }
        None
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
                })
                .unwrap_or(false)
            } else {
                false
            }
        })
    }
}

lazy_static! {
    pub static ref SYNTAXES: Vec<Syntax<'static>> = {
        use self::SyntaxSetting::*;
        vec![
            Syntax::new("C")
                .filematches(&[".c", ".cpp", ".h"])
                .flag(HighlightComments)
                .singleline_comment_start("//")
                .multiline_comment_start("/*")
                .multiline_comment_end("*/")
                .flag(HighlightKeywords)
                .keywords1(&[
                    "switch", "if", "while", "for", "break", "continue",
                    "return", "else", "struct", "union", "typedef", "static",
                    "enum", "class", "case",
                ])
                .keywords2(&[
                    "int", "long", "double", "float", "char", "unsigned",
                    "signed", "void",
                ])
                .flag(HighlightNumbers)
                .flag(HighlightStrings),
            Syntax::new("Rust")
                .filematches(&[".rs"])
                .flag(HighlightComments)
                .singleline_comment_start("//")
                .multiline_comment_start("/*")
                .multiline_comment_end("*/")
                .flag(HighlightKeywords)
                .keywords1(&[
                    "pub", "fn", "struct", "impl", "if", "else", "match",
                    "use", "const", "derive", "let",
                ])
                .keywords2(&[
                    "i8", "i32", "i64", "u32", "u64", "f32", "f64", "str",
                    "&str", "u8", "Self",
                ])
                .flag(HighlightNumbers)
                .flag(HighlightStrings),
        ]
    };
}

#[test]
fn test_matches_filename() {
    let syntax = Syntax::new("C").filematch(".c");
    assert!(syntax.matches_filename("test.c"));
    assert!(!syntax.matches_filename("test.r"));
}

#[test]
fn test_highlight_numbers() {
    let syntax = Syntax::new("test").flag(SyntaxSetting::HighlightNumbers);
    assert!(syntax.highlight_numbers());
    let syntax = Syntax::new("test");
    assert!(!syntax.highlight_numbers());
}

#[test]
fn test_highlight_strings() {
    let syntax = Syntax::new("test").flag(SyntaxSetting::HighlightStrings);
    assert!(syntax.highlight_strings());
    let syntax = Syntax::new("test");
    assert!(!syntax.highlight_strings());
}

#[test]
fn test_highlight_singleline_comments() {
    let syntax = Syntax::new("test")
        .flag(SyntaxSetting::HighlightComments)
        .singleline_comment_start("//");
    assert!(syntax.highlight_singleline_comments());
    let syntax = Syntax::new("test").flag(SyntaxSetting::HighlightComments);
    assert!(!syntax.highlight_singleline_comments());
    let syntax = Syntax::new("test").singleline_comment_start("//");
    assert!(!syntax.highlight_singleline_comments());
}

#[test]
fn test_highlight_keywords() {
    let syntax = Syntax::new("test").flag(SyntaxSetting::HighlightKeywords);
    assert!(syntax.highlight_keywords());
    let syntax = Syntax::new("test");
    assert!(!syntax.highlight_keywords());
}

#[test]
fn test_starts_with_keyword_keyword1() {
    let syntax = Syntax::new("test")
        .flag(SyntaxSetting::HighlightKeywords)
        .keywords1(&["if"]);
    assert_eq!(
        Some((Highlight::Keyword1, 2)),
        syntax.starts_with_keyword("if something")
    );
    assert_eq!(None, syntax.starts_with_keyword(" if else blah"));
}

#[test]
fn test_starts_with_keyword_keyword2() {
    let syntax = Syntax::new("test")
        .flag(SyntaxSetting::HighlightKeywords)
        .keywords2(&["int"]);
    assert_eq!(
        Some((Highlight::Keyword2, 3)),
        syntax.starts_with_keyword("int woot;")
    );
    assert_eq!(None, syntax.starts_with_keyword(" int woot;"));
}

#[test]
fn test_highlight_multiline_comments() {
    let syntax = Syntax::new("test")
        .flag(SyntaxSetting::HighlightComments)
        .multiline_comment_start("/*")
        .multiline_comment_end("*/");
    assert!(syntax.highlight_multiline_comments());
    let syntax = Syntax::new("test")
        .flag(SyntaxSetting::HighlightComments)
        .multiline_comment_end("*/");
    assert!(!syntax.highlight_multiline_comments());
    let syntax = Syntax::new("test")
        .flag(SyntaxSetting::HighlightComments)
        .multiline_comment_start("/*");
    assert!(!syntax.highlight_multiline_comments());
    let syntax = Syntax::new("test")
        .multiline_comment_start("/*")
        .multiline_comment_end("*/");
    assert!(!syntax.highlight_multiline_comments());
}
