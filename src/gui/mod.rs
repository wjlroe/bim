use cgmath::{Matrix4, Vector2};

mod actions;
mod container;
pub mod draw_state;
pub mod gfx_ui;
mod gl_renderer;
mod keycode_to_char;
mod pane;
mod persist_window_state;
mod quad;
mod rect;
mod transforms;
mod window;

pub type ColorFormat = gfx::format::Rgba8;
pub type DepthFormat = gfx::format::Depth;

pub fn transform_from_width_height(shape: Vector2<f32>, within: Vector2<f32>) -> Matrix4<f32> {
    Matrix4::from_nonuniform_scale(shape.x / within.x, shape.y / within.y, 1.0)
}
