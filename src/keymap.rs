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
        self.bindings.get(key).cloned().or_else(|| {
            if let Key::Other(typed_char) = key {
                self.bindings
                    .get(&Key::TypedChar)
                    .cloned()
                    .map(|map_or_action| {
                        if let MapOrAction::Action(Action::OnBuffer(
                            BufferAction::InsertTypedChar,
                        )) = map_or_action
                        {
                            MapOrAction::Action(Action::OnBuffer(BufferAction::InsertChar(
                                *typed_char,
                            )))
                        } else {
                            map_or_action
                        }
                    })
            } else {
                None
            }
        })
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
        bindings.insert(
            Key::PageDown,
            MapOrAction::Action(Action::OnBuffer(BufferAction::MoveCursor(
                MoveCursor::page_down(1),
            ))),
        );
        bindings.insert(
            Key::PageUp,
            MapOrAction::Action(Action::OnBuffer(BufferAction::MoveCursor(
                MoveCursor::page_up(1),
            ))),
        );
        bindings.insert(
            Key::Home,
            MapOrAction::Action(Action::OnBuffer(BufferAction::MoveCursor(
                MoveCursor::home(),
            ))),
        );
        bindings.insert(
            Key::End,
            MapOrAction::Action(Action::OnBuffer(
                BufferAction::MoveCursor(MoveCursor::end()),
            )),
        );
        bindings.insert(
            Key::Delete,
            MapOrAction::Action(Action::OnBuffer(BufferAction::DeleteChar(Direction::Right))),
        );
        bindings.insert(
            Key::Backspace,
            MapOrAction::Action(Action::OnBuffer(BufferAction::DeleteChar(Direction::Left))),
        );
        bindings.insert(
            Key::Return,
            MapOrAction::Action(Action::OnBuffer(BufferAction::InsertNewlineAndReturn)),
        );
        bindings.insert(
            Key::TypedChar,
            MapOrAction::Action(Action::OnBuffer(BufferAction::InsertTypedChar)),
        );

        let mut window_bindings = HashMap::new();
        window_bindings.insert(
            Key::ArrowRight,
            MapOrAction::Action(Action::OnWindow(WindowAction::FocusPane(Direction::Right))),
        );
        window_bindings.insert(
            Key::ArrowLeft,
            MapOrAction::Action(Action::OnWindow(WindowAction::FocusPane(Direction::Left))),
        );
        let window_keymap = Keymap {
            bindings: window_bindings,
        };

        bindings.insert(Key::Control(Some('w')), MapOrAction::Map(window_keymap));

        Keymap { bindings }
    };
}
