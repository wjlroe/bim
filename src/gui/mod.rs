mod animation;
mod container;
pub mod gfx_ui;
mod gl_renderer;
mod keycode_to_char;
mod pane;
mod persist_window_state;
mod transforms;
mod window;

pub type ColorFormat = gfx::format::Rgba8;
pub type DepthFormat = gfx::format::Depth;
