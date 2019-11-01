use cgmath::Vector2;

#[derive(Clone, Debug, PartialEq)]
pub enum MouseMove {
    // FIXME: don't use cgmath here, because it'll be pulled into Terminal-only builds
    Pixels(Vector2<f32>),
    Lines(Vector2<f32>),
}
