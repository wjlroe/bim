use crate::buffer::Buffer;
use crate::commands::{self, Cmd, MoveCursor, SearchCmd};
use crate::config::BIM_QUIT_TIMES;
use crate::gui::draw_state::DrawState;
use crate::keycodes::Key;
use crate::search::Search;
use crate::status::Status;
use cgmath::{Matrix4, Vector2};
use gfx_glyph::SectionText;
use glutin::dpi::{LogicalPosition, LogicalSize};
use glutin::{MonitorId, WindowedContext};
use std::time::Duration;

pub struct Window<'a> {
    logical_size: LogicalSize,
    dpi: f32,
    resized: bool,
    pub fullscreen: bool,
    draw_state: DrawState<'a>,
    search: Option<Search>,
    quit_times: i8,
    pub in_focus: bool,
    pub status_message: Option<Status>,
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
            quit_times: BIM_QUIT_TIMES + 1,
            in_focus: true,
            status_message: None,
        }
    }

    pub fn has_resized(&self) -> bool {
        self.resized
    }

    pub fn should_quit(&self) -> bool {
        self.quit_times <= 0
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

    pub fn inner_dimensions(&self) -> Vector2<f32> {
        (
            self.draw_state.inner_width(),
            self.draw_state.inner_height(),
        )
            .into()
    }

    pub fn window_dimensions(&self) -> Vector2<f32> {
        (
            self.draw_state.window_width(),
            self.draw_state.window_height(),
        )
            .into()
    }

    pub fn window_height(&self) -> f32 {
        self.draw_state.window_height()
    }

    pub fn window_width(&self) -> f32 {
        self.draw_state.window_width()
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

    pub fn transform_from_width_height(&self, width: f32, height: f32) -> Matrix4<f32> {
        self.draw_state.transform_from_width_height(width, height)
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
        self.draw_state.update_cursor();
    }

    pub fn inc_font_size(&mut self) {
        self.draw_state.inc_font_size();
        self.resized = true;
    }

    pub fn dec_font_size(&mut self) {
        self.draw_state.dec_font_size();
        self.resized = true;
    }

    fn print_info(&mut self) {
        self.draw_state.print_info();
    }

    pub fn handle_key(&mut self, key: Key) {
        if self.search.is_some() {
            self.handle_search_key(key);
        } else {
            self.handle_buffer_key(key);
        }
    }

    fn handle_search_key(&mut self, key: Key) {
        let cmd = match key {
            Key::ArrowLeft | Key::ArrowUp => Some(SearchCmd::PrevMatch),
            Key::ArrowRight | Key::ArrowDown => Some(SearchCmd::NextMatch),
            Key::Escape => Some(SearchCmd::Quit),
            Key::Return => Some(SearchCmd::Exit),
            Key::Other(typed_char) => Some(SearchCmd::InsertChar(typed_char)),
            Key::Backspace | Key::Delete => Some(SearchCmd::DeleteChar),
            _ => None,
        };
        if let Some(search_cmd) = cmd {
            self.handle_search_cmd(search_cmd);
        }
    }

    fn handle_search_cmd(&mut self, cmd: SearchCmd) {
        if let Some(search) = &mut self.search {
            match cmd {
                SearchCmd::Quit => search.stop(true),
                SearchCmd::Exit => search.stop(false),
                SearchCmd::NextMatch => search.go_forwards(),
                SearchCmd::PrevMatch => search.go_backwards(),
                SearchCmd::InsertChar(typed_char) => search.push_char(typed_char),
                SearchCmd::DeleteChar => search.del_char(),
            }

            if search.run_search() {
                let last_match = self.draw_state.search_for(
                    search.last_match(),
                    search.direction(),
                    search.needle(),
                );
                search.set_last_match(last_match);
            } else {
                if search.restore_cursor() {
                    self.draw_state.buffer.cursor.restore_saved();
                    self.draw_state.row_offset = search.saved_row_offset();
                    self.draw_state.col_offset = search.saved_col_offset();
                }
                self.search = None;
                self.draw_state.search_visible = false;
                self.draw_state.stop_search();
            }
        }
    }

    fn handle_buffer_key(&mut self, key: Key) {
        let buffer_cmd = match key {
            Key::ArrowLeft => Some(Cmd::Move(MoveCursor::left(1))),
            Key::ArrowRight => Some(Cmd::Move(MoveCursor::right(1))),
            Key::ArrowUp => Some(Cmd::Move(MoveCursor::up(1))),
            Key::ArrowDown => Some(Cmd::Move(MoveCursor::down(1))),
            Key::PageDown => Some(Cmd::Move(MoveCursor::page_down(1))),
            Key::PageUp => Some(Cmd::Move(MoveCursor::page_up(1))),
            Key::Home => Some(Cmd::Move(MoveCursor::home())),
            Key::End => Some(Cmd::Move(MoveCursor::end())),
            Key::Delete => Some(Cmd::DeleteCharForward),
            Key::Backspace => Some(Cmd::DeleteCharBackward),
            Key::Return => Some(Cmd::Linebreak),
            Key::Other(typed_char) => Some(Cmd::InsertChar(typed_char)),
            Key::Function(fn_key) => {
                println!("Unrecognised key: F{}", fn_key);
                None
            }
            Key::Control(Some('-')) => None,
            Key::Control(Some('+')) => None,
            Key::Control(Some(' ')) => Some(Cmd::CloneCursor),
            Key::Control(Some('m')) => Some(Cmd::PrintInfo),
            Key::Control(Some('f')) => Some(Cmd::Search),
            Key::Control(Some('q')) => Some(Cmd::Quit),
            Key::Control(Some('s')) => Some(Cmd::Save),
            Key::Control(Some(ctrl_char)) => {
                println!("Unrecognised keypress: Ctrl-{}", ctrl_char);
                None
            }
            Key::Control(None) => None,
            Key::Escape => None,
        };
        if let Some(cmd) = buffer_cmd {
            self.handle_buffer_cmd(cmd);
        }
    }

    fn handle_buffer_cmd(&mut self, cmd: Cmd) {
        match cmd {
            Cmd::Move(movement) => self.move_cursor(movement),
            Cmd::DeleteCharBackward => self.delete_char_backward(),
            Cmd::DeleteCharForward => self.delete_char_forward(),
            Cmd::Linebreak => self.insert_newline_and_return(),
            Cmd::InsertChar(typed_char) => self.insert_char(typed_char),
            Cmd::Search => self.start_search(),
            Cmd::CloneCursor => self.clone_cursor(),
            Cmd::Quit => self.try_quit(),
            Cmd::PrintInfo => self.print_info(),
            Cmd::Escape => {}
            Cmd::Save => self.save_file(),
        }
    }

    fn save_file(&mut self) {
        match self.draw_state.buffer.save_to_file() {
            Ok(bytes_saved) => {
                self.set_status_msg(format!("{} bytes written to disk", bytes_saved))
            }
            Err(err) => {
                self.set_status_msg(format!("Can't save! Error: {}", err));
            }
        }
    }

    fn start_search(&mut self) {
        let search = Search::new(self.draw_state.col_offset(), self.draw_state.row_offset());
        self.draw_state.buffer.cursor.save_cursor();
        self.search = Some(search);
        self.draw_state.search_visible = true;
        self.draw_state.update_search();
    }

    fn try_quit(&mut self) {
        if self.draw_state.buffer.is_dirty() {
            self.quit_times -= 1;
            self.set_status_msg(format!(
                "{} {} {} {}",
                "WARNING! File has unsaved changes.",
                "Press Ctrl-Q",
                self.quit_times,
                "more times to quit"
            ));
        } else {
            self.quit_times = 0;
        }
    }

    fn set_status_msg(&mut self, msg: String) {
        self.status_message = Some(Status::new_with_timeout(msg, Duration::from_secs(5)));
    }

    fn move_cursor(&mut self, movement: commands::MoveCursor) {
        self.draw_state
            .buffer
            .move_cursor(movement, self.draw_state.screen_rows() as usize);
        self.draw_state.update_cursor();
    }

    fn clone_cursor(&mut self) {
        self.draw_state.clone_cursor();
    }

    fn delete_char_backward(&mut self) {
        self.draw_state.delete_char();
    }

    fn delete_char_forward(&mut self) {
        self.move_cursor(commands::MoveCursor::right(1));
        self.draw_state.delete_char();
    }

    fn insert_newline_and_return(&mut self) {
        self.draw_state.insert_newline_and_return();
    }

    fn insert_char(&mut self, typed_char: char) {
        self.draw_state.insert_char(typed_char);
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
