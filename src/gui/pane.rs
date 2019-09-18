use crate::buffer::{Buffer, BufferAction, FileSaveStatus};
use crate::gui::actions::GuiAction;
use crate::gui::draw_state::DrawState;
use crate::gui::gl_renderer::GlRenderer;
use crate::gui::window::WindowAction;
use crate::keycodes::Key;
use std::error::Error;

pub struct Pane<'a> {
    pub draw_state: DrawState<'a>,
    focused: bool,
}

impl<'a> Pane<'a> {
    pub fn new(font_size: f32, ui_scale: f32, buffer: Buffer<'a>, focused: bool) -> Self {
        Self {
            draw_state: DrawState::new(font_size, ui_scale, buffer),
            focused,
        }
    }

    pub fn render(&self, renderer: &mut GlRenderer) -> Result<(), Box<dyn Error>> {
        self.draw_state.render(renderer, self.focused)
    }

    pub fn update_buffer(&mut self, action: BufferAction) {
        self.draw_state.update_buffer(action);
    }

    pub fn update_gui(&mut self, action: GuiAction) {
        self.draw_state.update_gui(action);
    }

    pub fn handle_key(&mut self, key: Key) -> (bool, Option<WindowAction>) {
        self.draw_state.handle_key(key)
    }

    pub fn save_file(&mut self) -> Result<FileSaveStatus, Box<dyn Error>> {
        self.draw_state.save_file()
    }

    pub fn is_dirty(&self) -> bool {
        self.draw_state.buffer.is_dirty()
    }

    pub fn font_size(&self) -> f32 {
        self.draw_state.font_size
    }

    pub fn ui_scale(&self) -> f32 {
        self.draw_state.ui_scale
    }

    pub fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
    }
}
