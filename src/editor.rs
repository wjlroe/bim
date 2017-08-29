use keycodes::Key;
use terminal::Terminal;

pub trait Editor {
    fn enable_raw_mode(&self);
    fn get_window_size(&self) -> Terminal;
    fn read_a_character(&self) -> Option<Key>;

    fn process_keypress(&self, mut terminal: &mut Terminal) {
        if let Some(key) = self.read_a_character() {
            terminal.process_key(key);
        }
    }
}
