use cgmath::Vector2;

#[derive(Clone, Debug, PartialEq)]
pub enum MouseMove {
    Pixels(Vector2<f32>),
    Lines(Vector2<f32>),
}
