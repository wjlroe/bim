use crate::action::*;
use crate::commands::*;
use crate::keycodes::Key;
use lazy_static::lazy_static;
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq)]
pub enum MapOrAction {
    Map(Keymap),
    Action(Action),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Keymap {
    bindings: HashMap<Key, MapOrAction>,
}

impl Keymap {
    pub fn lookup(&self, key: &Key) -> Option<MapOrAction> {
        self.bindings.get(key).cloned()
    }
}

lazy_static! {
    pub static ref DEFAULT_KEYMAP: Keymap = {
        let mut bindings = HashMap::new();
        bindings.insert(
            Key::ArrowLeft,
            MapOrAction::Action(Action::OnBuffer(BufferAction::MoveCursor(
                MoveCursor::left(1),
            ))),
        );
        bindings.insert(
            Key::ArrowRight,
            MapOrAction::Action(Action::OnBuffer(BufferAction::MoveCursor(
                MoveCursor::right(1),
            ))),
        );
        bindings.insert(
            Key::ArrowUp,
            MapOrAction::Action(Action::OnBuffer(BufferAction::MoveCursor(MoveCursor::up(
                1,
            )))),
        );
        bindings.insert(
            Key::ArrowDown,
            MapOrAction::Action(Action::OnBuffer(BufferAction::MoveCursor(
                MoveCursor::down(1),
            ))),
        );

        let window_bindings = HashMap::new();
        let window_keymap = Keymap {
            bindings: window_bindings,
        };

        bindings.insert(Key::Control(Some('w')), MapOrAction::Map(window_keymap));

        Keymap { bindings }
    };
}
