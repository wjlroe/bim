#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Key {
    ArrowLeft,
    ArrowRight,
    ArrowUp,
    ArrowDown,
    PageUp,
    PageDown,
    Home,
    End,
    Delete,
    Return,
    Backspace,
    Escape,
    Other(char),
}

pub fn ctrl_key(key: char, keycode: u32) -> bool {
    (key as u32 & 0x1f) == keycode
}

#[test]
fn test_ctrl_key() {
    assert!(ctrl_key('q', 17u32));
    assert!(!ctrl_key('q', 'w' as u32));
}
