#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
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
    Control(Option<char>),
    Function(u8),
    Other(char),
    TypedChar, // any typed char, not specific
}

pub fn ctrl_key(key: char, keycode: u32) -> bool {
    (key as u32 & 0x1f) == keycode
}

// TODO: Replace this with something sensible
pub fn is_printable(key: char) -> bool {
    if key.is_control() {
        return false;
    }

    // Arrow keys
    if key >= '\u{f700}' && key <= '\u{f703}' {
        return false;
    }

    // Backspace
    if key == '\x7f' {
        return false;
    }

    // Delete
    if key == '\u{f728}' {
        return false;
    }

    // Return
    if key == '\u{d}' {
        return false;
    }

    // Home
    if key == '\u{f729}' {
        return false;
    }

    // End
    if key == '\u{f72b}' {
        return false;
    }

    // PageUp
    if key == '\u{f72c}' {
        return false;
    }

    // PageDown
    if key == '\u{f72d}' {
        return false;
    }

    true
}

#[test]
fn test_ctrl_key() {
    assert!(ctrl_key('q', 17u32));
    assert!(!ctrl_key('q', 'w' as u32));
}
