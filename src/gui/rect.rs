use cgmath::{vec2, Vector2};

#[derive(Debug, Clone, Copy)]
pub struct Rect {
    pub top_left: Vector2<f32>,
    pub bounds: Vector2<f32>,
}

impl Rect {
    pub fn new(position: Vector2<f32>, bounds: Vector2<f32>) -> Self {
        Self {
            top_left: position,
            bounds,
        }
    }

    pub fn center(&self) -> Vector2<f32> {
        vec2(
            self.top_left.x + self.bounds.x / 2.0,
            self.top_left.y + self.bounds.y / 2.0,
        )
    }
}
