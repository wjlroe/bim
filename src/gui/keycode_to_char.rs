use crate::keycodes::Key;
use glutin::{ElementState, KeyboardInput, VirtualKeyCode};
use lazy_static::lazy_static;
use std::collections::HashMap;

lazy_static! {
    static ref KEYCODE_TO_CHAR: HashMap<VirtualKeyCode, char> = {
        let mut map = HashMap::new();
        map.insert(VirtualKeyCode::A, 'a');
        map.insert(VirtualKeyCode::B, 'b');
        map.insert(VirtualKeyCode::C, 'c');
        map.insert(VirtualKeyCode::D, 'd');
        map.insert(VirtualKeyCode::E, 'e');
        map.insert(VirtualKeyCode::F, 'f');
        map.insert(VirtualKeyCode::G, 'g');
        map.insert(VirtualKeyCode::H, 'h');
        map.insert(VirtualKeyCode::I, 'i');
        map.insert(VirtualKeyCode::J, 'j');
        map.insert(VirtualKeyCode::K, 'k');
        map.insert(VirtualKeyCode::L, 'l');
        map.insert(VirtualKeyCode::M, 'm');
        map.insert(VirtualKeyCode::N, 'n');
        map.insert(VirtualKeyCode::O, 'o');
        map.insert(VirtualKeyCode::P, 'p');
        map.insert(VirtualKeyCode::Q, 'q');
        map.insert(VirtualKeyCode::R, 'r');
        map.insert(VirtualKeyCode::S, 's');
        map.insert(VirtualKeyCode::T, 't');
        map.insert(VirtualKeyCode::U, 'u');
        map.insert(VirtualKeyCode::V, 'v');
        map.insert(VirtualKeyCode::W, 'w');
        map.insert(VirtualKeyCode::X, 'x');
        map.insert(VirtualKeyCode::Y, 'y');
        map.insert(VirtualKeyCode::Z, 'z');
        map.insert(VirtualKeyCode::Key0, '0');
        map.insert(VirtualKeyCode::Key1, '1');
        map.insert(VirtualKeyCode::Key2, '2');
        map.insert(VirtualKeyCode::Key3, '3');
        map.insert(VirtualKeyCode::Key4, '4');
        map.insert(VirtualKeyCode::Key5, '5');
        map.insert(VirtualKeyCode::Key6, '6');
        map.insert(VirtualKeyCode::Key7, '7');
        map.insert(VirtualKeyCode::Key8, '8');
        map.insert(VirtualKeyCode::Key9, '9');
        map.insert(VirtualKeyCode::Numpad0, '0');
        map.insert(VirtualKeyCode::Numpad1, '1');
        map.insert(VirtualKeyCode::Numpad2, '2');
        map.insert(VirtualKeyCode::Numpad3, '3');
        map.insert(VirtualKeyCode::Numpad4, '4');
        map.insert(VirtualKeyCode::Numpad5, '5');
        map.insert(VirtualKeyCode::Numpad6, '6');
        map.insert(VirtualKeyCode::Numpad7, '7');
        map.insert(VirtualKeyCode::Numpad8, '8');
        map.insert(VirtualKeyCode::Numpad9, '9');
        map.insert(VirtualKeyCode::Space, ' ');
        map.insert(VirtualKeyCode::Semicolon, ';');
        map.insert(VirtualKeyCode::Apostrophe, '\'');
        map.insert(VirtualKeyCode::LBracket, '[');
        map.insert(VirtualKeyCode::RBracket, ']');
        map.insert(VirtualKeyCode::Backslash, '\\');
        map.insert(VirtualKeyCode::Slash, '/');
        map.insert(VirtualKeyCode::Period, '.');
        map.insert(VirtualKeyCode::Comma, ',');
        map.insert(VirtualKeyCode::Equals, '=');
        map.insert(VirtualKeyCode::Minus, '-');
        map.insert(VirtualKeyCode::Add, '+');
        map.insert(VirtualKeyCode::Subtract, '-');
        map.insert(VirtualKeyCode::Multiply, '*');
        map.insert(VirtualKeyCode::Divide, '/');
        map
    };
}

pub fn keyboard_event_to_keycode(event: KeyboardInput) -> Option<Key> {
    if event.state == ElementState::Pressed {
        #[allow(clippy::collapsible_if)]
        match event.virtual_keycode {
            Some(VirtualKeyCode::Escape) => Some(Key::Escape),
            Some(VirtualKeyCode::Left) => Some(Key::ArrowLeft),
            Some(VirtualKeyCode::Right) => Some(Key::ArrowRight),
            Some(VirtualKeyCode::Up) => Some(Key::ArrowUp),
            Some(VirtualKeyCode::Down) => Some(Key::ArrowDown),
            Some(VirtualKeyCode::PageDown) => Some(Key::PageDown),
            Some(VirtualKeyCode::PageUp) => Some(Key::PageUp),
            Some(VirtualKeyCode::Home) => Some(Key::Home),
            Some(VirtualKeyCode::End) => Some(Key::End),
            Some(VirtualKeyCode::Back) => Some(Key::Backspace),
            Some(VirtualKeyCode::Delete) => Some(Key::Delete),
            Some(VirtualKeyCode::Return) => Some(Key::Return),
            Some(VirtualKeyCode::F11) => Some(Key::Function(11)),
            Some(VirtualKeyCode::LControl) => None,
            Some(VirtualKeyCode::RControl) => None,
            Some(VirtualKeyCode::LAlt) => None,
            Some(VirtualKeyCode::RAlt) => None,
            Some(VirtualKeyCode::LShift) => None,
            Some(VirtualKeyCode::RShift) => None,
            Some(keycode) => {
                if event.modifiers.ctrl || event.modifiers.alt || event.modifiers.logo {
                    if keycode == VirtualKeyCode::Minus && event.modifiers.ctrl {
                        Some(Key::Control(Some('-')))
                    } else if keycode == VirtualKeyCode::Equals
                        && event.modifiers.shift
                        && event.modifiers.ctrl
                    {
                        Some(Key::Control(Some('+')))
                    } else if keycode == VirtualKeyCode::Space && event.modifiers.ctrl {
                        Some(Key::Control(Some(' ')))
                    } else if keycode == VirtualKeyCode::M && event.modifiers.ctrl {
                        Some(Key::Control(Some('m')))
                    } else if keycode == VirtualKeyCode::F && event.modifiers.ctrl {
                        Some(Key::Control(Some('f')))
                    } else if keycode == VirtualKeyCode::Q && event.modifiers.ctrl {
                        Some(Key::Control(Some('q')))
                    } else if keycode == VirtualKeyCode::S && event.modifiers.ctrl {
                        Some(Key::Control(Some('s')))
                    } else if keycode == VirtualKeyCode::V && event.modifiers.ctrl {
                        Some(Key::Control(Some('v')))
                    } else {
                        println!("Don't know what to do with received: {:?}", event);
                        None
                    }
                } else {
                    None
                }
            }
            _ => {
                println!("No virtual keycode received: {:?}", event);
                None
            }
        }
    } else {
        None
    }
}
