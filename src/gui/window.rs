use crate::buffer::Buffer;
use crate::commands::{self, SearchDirection};
use crate::gui::draw_state::DrawState;
use cgmath::Matrix4;
use gfx_glyph::SectionText;
use glutin::dpi::{LogicalPosition, LogicalSize};
use glutin::{MonitorId, WindowedContext};

#[derive(Clone, Default, PartialEq)]
struct Search {
    needle: String,
    direction: SearchDirection,
}

impl Search {
    fn as_string(&self) -> String {
        format!("Search ({}): {}", self.direction, self.needle)
    }
}

pub struct Window<'a> {
    logical_size: LogicalSize,
    dpi: f32,
    resized: bool,
    pub fullscreen: bool,
    draw_state: DrawState<'a>,
    search: Option<Search>,
}

impl<'a> Window<'a> {
    pub fn new(
        logical_size: LogicalSize,
        dpi: f32,
        window_width: f32,
        window_height: f32,
        font_size: f32,
        ui_scale: f32,
        buffer: Buffer<'a>,
    ) -> Self {
        Self {
            logical_size,
            dpi,
            resized: false,
            fullscreen: false,
            draw_state: DrawState::new(window_width, window_height, font_size, ui_scale, buffer),
            search: None,
        }
    }

    pub fn has_resized(&self) -> bool {
        self.resized
    }

    pub fn next_frame(&mut self) {
        self.resized = false;
    }

    pub fn toggle_fullscreen(&mut self, gfx_window: &WindowedContext, monitor: MonitorId) {
        if self.fullscreen {
            gfx_window.set_fullscreen(None);
            self.fullscreen = false;
            self.resized = true;
        } else {
            gfx_window.set_fullscreen(Some(monitor));
            self.fullscreen = true;
            self.resized = true;
        }
    }

    pub fn inner_dimensions(&self) -> (f32, f32) {
        (
            self.draw_state.inner_width(),
            self.draw_state.inner_height(),
        )
    }

    pub fn font_scale(&self) -> f32 {
        self.draw_state.font_scale()
    }

    pub fn left_padding(&self) -> f32 {
        self.draw_state.left_padding()
    }

    pub fn top_padding(&self) -> f32 {
        self.draw_state.top_padding()
    }

    pub fn row_offset_as_transform(&self) -> Matrix4<f32> {
        self.draw_state.row_offset_as_transform()
    }

    pub fn cursor_transform(&self) -> Matrix4<f32> {
        self.draw_state.cursor_transform()
    }

    pub fn other_cursor_transform(&self) -> Option<Matrix4<f32>> {
        self.draw_state.other_cursor_transform()
    }

    pub fn line_transforms(&self) -> Vec<Matrix4<f32>> {
        self.draw_state.line_transforms()
    }

    pub fn status_transform(&self) -> Matrix4<f32> {
        self.draw_state.status_transform()
    }

    pub fn status_text(&self) -> String {
        format!(
            "{} | {} | {}",
            self.draw_state.status_line.filename,
            self.draw_state.status_line.filetype,
            self.draw_state.status_line.cursor
        )
    }

    pub fn section_texts(&self) -> Vec<SectionText> {
        self.draw_state.section_texts()
    }

    pub fn update_mouse_position(&mut self, mouse: (f64, f64)) {
        self.draw_state.mouse_position = mouse;
    }

    pub fn mouse_click(&mut self) {
        let real_position: (f64, f64) = LogicalPosition::from(self.draw_state.mouse_position)
            .to_physical(self.draw_state.ui_scale().into())
            .into();
        self.draw_state.move_cursor_to_mouse_position(real_position);
    }

    pub fn mouse_scroll(&mut self, delta_x: f32, delta_y: f32) {
        self.draw_state.scroll_window_vertically(-delta_y);
        self.draw_state.scroll_window_horizontally(-delta_x);
    }

    pub fn inc_font_size(&mut self) {
        self.draw_state.inc_font_size();
        self.resized = true;
    }

    pub fn dec_font_size(&mut self) {
        self.draw_state.dec_font_size();
        self.resized = true;
    }

    pub fn print_info(&mut self) {
        self.draw_state.print_info();
    }

    pub fn move_cursor(&mut self, movement: commands::MoveCursor) {
        self.draw_state
            .buffer
            .move_cursor(movement, self.draw_state.screen_rows() as usize);
        self.draw_state.update_cursor();
    }

    pub fn clone_cursor(&mut self) {
        self.draw_state.clone_cursor();
    }

    pub fn delete_char_backward(&mut self) {
        self.draw_state.delete_char();
    }

    pub fn delete_char_forward(&mut self) {
        self.move_cursor(commands::MoveCursor::right(1));
        self.draw_state.delete_char();
    }

    pub fn insert_newline_and_return(&mut self) {
        self.draw_state.insert_newline_and_return();
    }

    pub fn insert_char(&mut self, typed_char: char) {
        if let Some(search) = &mut self.search {
            search.needle.push(typed_char);
        } else {
            self.draw_state.insert_char(typed_char);
        }
    }

    pub fn start_search(&mut self) {
        self.search = Some(Search::default());
        self.draw_state.search_visible = true;
    }

    pub fn search_text(&self) -> Option<String> {
        if let Some(search) = &self.search {
            Some(search.as_string())
        } else {
            None
        }
    }

    pub fn resize(&mut self, logical_size: LogicalSize) {
        self.logical_size = logical_size;
    }

    pub fn set_window_dimensions(&mut self, dimensions: (u16, u16)) {
        self.draw_state.set_window_dimensions(dimensions);
        self.resized = true;
    }

    pub fn set_dpi(&mut self, dpi: f32) {
        println!("DPI changed: {}", dpi);
        self.dpi = dpi;
        self.draw_state.set_ui_scale(dpi);
    }

    pub fn set_line_height(&mut self, line_height: f32) {
        self.draw_state.set_line_height(line_height);
    }

    pub fn set_character_width(&mut self, character_width: f32) {
        self.draw_state.set_character_width(character_width);
    }
}