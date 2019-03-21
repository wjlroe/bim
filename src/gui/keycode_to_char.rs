use glutin::VirtualKeyCode;
use lazy_static::lazy_static;
use std::collections::HashMap;

lazy_static! {
    pub static ref KEYCODE_TO_CHAR: HashMap<VirtualKeyCode, char> = {
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
        map
    };
}
