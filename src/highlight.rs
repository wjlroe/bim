use std::collections::HashMap;

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub enum Highlight {
    Normal,
    Number,
}

lazy_static! {
    pub static ref HL_TO_COLOUR: HashMap<Highlight, u8> = {
        use self::Highlight::*;

        let mut m = HashMap::new();
        m.insert(Normal, 39);
        m.insert(Number, 31);
        m
    };
}
