use std::collections::HashMap;

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub enum Highlight {
    Normal,
    Number,
    SearchMatch,
    String,
    Comment,
    MultilineComment,
    Keyword1,
    Keyword2,
}

pub const DEFAULT_COLOUR: u8 = 39;

lazy_static! {
    pub static ref HL_TO_COLOUR: HashMap<Highlight, u8> = {
        use self::Highlight::*;

        let mut m = HashMap::new();
        m.insert(Normal, DEFAULT_COLOUR);
        m.insert(Number, 31);
        m.insert(SearchMatch, 34);
        m.insert(String, 35);
        m.insert(Comment, 36);
        m.insert(MultilineComment, 36);
        m.insert(Keyword1, 33);
        m.insert(Keyword2, 32);
        m
    };
}