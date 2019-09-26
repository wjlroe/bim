use cgmath::Vector2;

#[derive(Debug)]
pub enum MouseMove {
    Pixels(Vector2<f32>),
    Lines(Vector2<f32>),
}
