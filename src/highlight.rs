use lazy_static::lazy_static;
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
    Cursor,
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

pub fn highlight_to_color(hl: Highlight) -> [f32; 4] {
    use self::Highlight::*;

    match hl {
        Normal => [232.0 / 255.0, 230.0 / 255.0, 237.0 / 255.0, 1.0],
        Number => [221.0 / 255.0, 119.0 / 255.0, 85.0 / 255.0, 1.0],
        String => [191.0 / 255.0, 156.0 / 255.0, 249.0 / 255.0, 1.0],
        Comment | MultilineComment => {
            [86.0 / 255.0, 211.0 / 255.0, 194.0 / 255.0, 1.0]
        }
        Keyword1 => [242.0 / 255.0, 231.0 / 255.0, 183.0 / 255.0, 1.0],
        Keyword2 => [4.0 / 255.0, 219.0 / 255.0, 181.0 / 255.0, 1.0],
        Cursor => [2.0 / 255.0, 200.0 / 255.0, 200.0 / 255.0, 1.0],
        _ => [0.9, 0.4, 0.2, 1.0],
    }
}

#[derive(Clone)]
pub struct HighlightedSection {
    pub text: String,
    pub highlight: Option<Highlight>,
    pub text_row: usize,
}
