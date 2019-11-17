use glam::Vec2;

#[derive(Clone, Debug, PartialEq)]
pub enum MouseMove {
    Pixels(Vec2),
    Lines(Vec2),
}
