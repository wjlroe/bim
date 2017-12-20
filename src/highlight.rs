use std::collections::HashMap;

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub enum Highlight {
    Normal,
    Number,
}

pub const DEFAULT_COLOUR: u8 = 39;

lazy_static! {
    pub static ref HL_TO_COLOUR: HashMap<Highlight, u8> = {
        use self::Highlight::*;

        let mut m = HashMap::new();
        m.insert(Normal, DEFAULT_COLOUR);
        m.insert(Number, 31);
        m
    };
}
