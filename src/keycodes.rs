#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Key {
    ArrowLeft,
    ArrowRight,
    ArrowUp,
    ArrowDown,
    Other(char),
}

pub fn ctrl_key(key: char, keycode: u32) -> bool {
    (key as u32 & 0x1f) == keycode
}
