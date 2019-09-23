use crate::buffer::{Buffer, BufferAction, FileSaveStatus};
use crate::commands::MoveCursor;
use crate::cursor::{Cursor, CursorT};
use crate::gui::actions::GuiAction;
use crate::gui::gl_renderer::GlRenderer;
use crate::gui::rect::{Rect, RectBuilder};
use crate::gui::window::WindowAction;
use crate::highlight::HighlightedSection;
use crate::highlight::{highlight_to_color, Highlight};
use crate::input::Input;
use crate::keycodes::Key;
use crate::prompt::PromptAction;
use crate::search::Search;
use crate::status_line::StatusLine;
use crate::utils::char_position_to_byte_position;
use cgmath::{vec2, Matrix4, Vector2, Vector3};
use flame;
use gfx_glyph::{Scale, Section, SectionText, VariedSection};
use std::error::Error;

const LINE_COLS_AT: [u32; 2] = [80, 120];
const LINE_COL_BG: [f32; 3] = [0.0, 0.0, 0.0];
const STATUS_FOCUSED_BG: [f32; 3] = [215.0 / 256.0, 0.0, 135.0 / 256.0];
const STATUS_UNFOCUS_BG: [f32; 3] = [215.0 / 256.0, 0.0, 135.0 / 256.0];
const STATUS_FOCUSED_FG: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const STATUS_UNFOCUS_FG: [f32; 4] = [0.8, 0.8, 0.8, 1.0];
const CURSOR_FOCUSED_BG: [f32; 3] = [250.0 / 256.0, 250.0 / 256.0, 250.0 / 256.0];
const CURSOR_UNFOCUS_BG: [f32; 3] = [150.0 / 256.0, 150.0 / 256.0, 150.0 / 256.0];
const OTHER_CURSOR_BG: [f32; 3] = [255.0 / 256.0, 165.0 / 256.0, 0.0];

pub struct DrawState<'a> {
    pub bounds: Vector2<f32>,
    position: Vector2<f32>,
    line_height: f32,
    character_width: f32,
    pub font_size: f32,
    pub ui_scale: f32,
    left_padding: f32,
    other_cursor: Option<Cursor>,
    pub buffer: Buffer<'a>,
    pub highlighted_sections: Vec<HighlightedSection>,
    pub status_line: StatusLine,
    pub row_offset: f32,
    pub col_offset: f32,
    screen_rows: i32,
    pub prompt: Option<Input<'a>>,
    pub search: Option<Search>,
}

impl<'a> Default for DrawState<'a> {
    fn default() -> Self {
        Self {
            bounds: vec2(0.0, 0.0),
            position: vec2(0.0, 0.0),
            line_height: 0.0,
            character_width: 0.0,
            font_size: 0.0,
            ui_scale: 0.0,
            left_padding: 0.0,
            other_cursor: None,
            buffer: Buffer::default(),
            highlighted_sections: vec![],
            status_line: StatusLine::default(),
            row_offset: 0.0,
            col_offset: 0.0,
            screen_rows: 0,
            prompt: None,
            search: None,
        }
    }
}

impl<'a> DrawState<'a> {
    pub fn new(font_size: f32, ui_scale: f32, buffer: Buffer<'a>) -> Self {
        let mut state = DrawState {
            font_size,
            ui_scale,
            left_padding: 12.0,
            buffer,
            ..DrawState::default()
        };
        state.update_highlighted_sections();
        state.update_status_line();
        state
    }

    fn update_size(&mut self, bounds: Vector2<f32>, position: Vector2<f32>) {
        self.bounds = bounds;
        self.position = position;
        self.update_screen_rows();
        self.scroll();
    }

    pub fn update_font_metrics(&mut self) {
        self.update_screen_rows();
        self.scroll();
    }

    pub fn update_cursor(&mut self) {
        self.update_screen_rows();
        self.scroll();
        self.update_status_line();
    }

    pub fn line_height(&self) -> f32 {
        self.line_height
    }

    pub fn inner_width(&self) -> f32 {
        self.bounds.x - self.left_padding
    }

    pub fn inner_height(&self) -> f32 {
        self.bounds.y - self.bottom_padding() - self.top_padding()
    }

    pub fn screen_rows(&self) -> i32 {
        self.screen_rows
    }

    pub fn character_width(&self) -> f32 {
        self.character_width
    }

    pub fn ui_scale(&self) -> f32 {
        self.ui_scale
    }

    pub fn font_scale(&self) -> f32 {
        self.ui_scale * self.font_size
    }

    pub fn top_padding(&self) -> f32 {
        if self.top_prompt_visible() {
            self.line_height() // if search is on
        } else {
            0.0
        }
    }

    pub fn bottom_padding(&self) -> f32 {
        self.line_height() // status line
    }

    pub fn left_padding(&self) -> f32 {
        self.left_padding
    }

    pub fn row_offset(&self) -> f32 {
        self.row_offset
    }

    pub fn col_offset(&self) -> f32 {
        self.col_offset
    }

    pub fn cursor(&self) -> (usize, usize) {
        (
            self.buffer.cursor.text_row() as usize,
            self.buffer.cursor.text_col() as usize,
        )
    }

    pub fn screen_position_vertical_offset(&self) -> f32 {
        self.row_offset.fract() * self.line_height
    }

    pub fn row_offset_as_transform(&self) -> Matrix4<f32> {
        let y_move = self.screen_position_vertical_offset() / (self.bounds.y / 2.0);
        Matrix4::from_translation(Vector3::new(0.0, y_move, 0.0))
    }

    fn scroll(&mut self) {
        if self.line_height > 0.0 {
            if self.buffer.cursor.text_row() >= self.row_offset.floor() as i32 + self.screen_rows()
            {
                self.row_offset = (self.buffer.cursor.text_row() - self.screen_rows() + 1) as f32;
            }

            if self.buffer.cursor.text_row() < self.row_offset.ceil() as i32 {
                self.row_offset = self.buffer.cursor.text_row() as f32;
            }
        }
    }

    fn update_status_line(&mut self) {
        let filename = self
            .buffer
            .filename
            .clone()
            .unwrap_or_else(|| String::from("[No Name]"));
        self.status_line.filename = filename;
        self.status_line.filetype = self.buffer.get_filetype();
        self.status_line.cursor = format!(
            "{}:{}",
            self.buffer.cursor.text_row() + 1,
            self.buffer.cursor.text_col() + 1,
        );
    }

    fn update_highlighted_sections(&mut self) {
        self.highlighted_sections.clear();
        for (row_idx, row) in self.buffer.rows.iter().enumerate() {
            // We don't want to push a 0->0 Normal highlight at the beginning of every line
            let mut first_char_seen = false;
            let mut current_section = HighlightedSection::default();
            current_section.text_row = row_idx;
            let mut overlay = row.overlay.iter();

            for (col_idx, hl) in row.hl.iter().enumerate() {
                let char_overlay: Option<Highlight> =
                    overlay.next().cloned().unwrap_or_else(|| None);
                let overlay_or_hl = char_overlay.unwrap_or_else(|| *hl);
                if current_section.highlight == overlay_or_hl {
                    current_section.last_col_idx = col_idx;
                } else {
                    if first_char_seen {
                        self.highlighted_sections.push(current_section);
                    }
                    current_section.highlight = overlay_or_hl;
                    current_section.first_col_idx = col_idx;
                    current_section.last_col_idx = col_idx;
                }
                first_char_seen = true;
            }

            if first_char_seen {
                self.highlighted_sections.push(current_section);
            }
        }
    }

    fn cursor_from_mouse_position(&self, mouse: Vector2<f32>) -> Vector2<i32> {
        let row_on_screen =
            ((mouse.y - self.top_padding()) / self.line_height() + self.row_offset).floor() as i32;
        let col_on_screen =
            ((mouse.x - self.left_padding()) / self.character_width()).floor() as i32;
        vec2(col_on_screen, row_on_screen)
    }

    pub fn onscreen_cursor<C>(&self, cursor: &C) -> Rect
    where
        C: CursorT,
    {
        let rcursor_x = self
            .buffer
            .text_cursor_to_render(cursor.text_col(), cursor.text_row());
        let cursor_width = self.character_width();
        let cursor_height = self.line_height();

        let cursor_y = cursor.text_row() as f32;
        let cursor_x = rcursor_x as f32;
        let x_on_screen = (cursor_width * cursor_x) + self.left_padding;
        let y_on_screen = (cursor_height * (cursor_y - self.row_offset)) + self.top_padding();
        RectBuilder::new()
            .bounds(vec2(cursor_width, cursor_height))
            .top_left(self.position + vec2(x_on_screen, y_on_screen))
            .build()
    }

    pub fn update_screen_rows(&mut self) {
        self.screen_rows = (self.inner_height() / self.line_height()).floor() as i32;
    }

    pub fn print_info(&self) {
        println!("status_height: {}", self.line_height());
        println!("inner: ({}, {})", self.inner_width(), self.inner_height());
        println!(
            "cursor on screen: {:?}",
            self.onscreen_cursor(&self.buffer.cursor)
        );
        println!("screen_rows: {}", self.screen_rows);
        println!("bounds: {:?}", self.bounds);
        println!("position: {:?}", self.position);
    }

    fn set_font_size(&mut self, font_size: f32) {
        self.font_size = font_size;
        self.update_font_metrics();
    }

    fn mouse_scroll(&mut self, delta: Vector2<f32>) {
        self.scroll_window_vertically(delta.y);
        self.scroll_window_horizontally(delta.x);
        self.update_cursor();
    }

    fn mouse_click(&mut self, location: Vector2<f32>) {
        println!("mouse click: {:?}", location);
        self.move_cursor_to_mouse_position(location);
    }

    pub fn move_cursor_to_mouse_position(&mut self, mouse: Vector2<f32>) {
        let cursor = self.cursor_from_mouse_position(mouse);
        let clicked_line = i32::min((self.buffer.num_lines() as i32) - 1, cursor.y);
        let clicked_line_length = self.buffer.line_len(clicked_line).unwrap_or(0) as i32;
        let clicked_line_x = i32::min(clicked_line_length, cursor.x);
        let move_y = clicked_line - self.buffer.cursor.text_row();
        let move_x = clicked_line_x - self.buffer.cursor.text_col();
        self.buffer.cursor.change(|cursor| {
            cursor.text_row += move_y;
            cursor.text_col += move_x;
        });
        self.update_cursor();
    }

    pub fn reset_cursor_col(&mut self, to: i32) {
        self.buffer.cursor.change(|cursor| cursor.text_col = to);
        self.update_cursor();
    }

    pub fn move_cursor_to_end_of_line(&mut self) {
        if self.buffer.cursor.text_row() < self.buffer.num_lines() as i32 {
            self.reset_cursor_col(
                self.buffer
                    .line_len(self.buffer.cursor.text_row())
                    .unwrap_or(0) as i32,
            );
        }
    }

    fn is_cursor_onscreen(&self) -> bool {
        let row = self.buffer.cursor.text_row();
        let row_offset = self.row_offset.floor() as i32;
        row >= row_offset && row < row_offset + self.screen_rows
    }

    pub fn update_buffer(&mut self, action: BufferAction) {
        use BufferAction::*;

        match action {
            InsertNewlineAndReturn => self.insert_newline_and_return(),
            InsertChar(typed_char) => self.insert_char(typed_char),
            DeleteChar => self.delete_char(),
            CloneCursor => self.clone_cursor(),
            MoveCursor(movement) => self.move_cursor(movement),
            MouseScroll(delta) => self.mouse_scroll(delta),
            MouseClick(location) => self.mouse_click(location),
            SetFilename(filename) => self.buffer.set_filename(filename),
            StartSearch => self.start_search(),
            PrintDebugInfo => self.print_info(),
        }
    }

    pub fn update_gui(&mut self, action: GuiAction) {
        use GuiAction::*;

        match action {
            UpdateSize(bounds, position) => self.update_size(bounds, position),
            SetFontSize(font_size) => self.set_font_size(font_size),
            SetUiScale(dpi) => self.set_ui_scale(dpi),
            SetLineHeight(line_height) => self.set_line_height(line_height),
            SetCharacterWidth(character_width) => self.set_character_width(character_width),
        }
    }

    fn move_cursor(&mut self, movement: MoveCursor) {
        self.buffer
            .move_cursor(movement, self.screen_rows() as usize);
        self.update_cursor();
    }

    fn move_cursor_onscreen(&mut self) {
        let row_offset = self.row_offset.floor() as i32;
        self.buffer.cursor.change(|cursor| {
            cursor.text_row = row_offset;
        });
    }

    pub fn scroll_window_vertically(&mut self, amount: f32) {
        self.row_offset += amount;
        if self.row_offset < 0.0 {
            self.row_offset = 0.0;
        }
        let max_offset = self.buffer.num_lines() as f32;
        if self.row_offset >= max_offset {
            self.row_offset = max_offset - 1.0;
        }
        if !self.is_cursor_onscreen() {
            self.move_cursor_onscreen();
        }
    }

    pub fn scroll_window_horizontally(&mut self, amount: f32) {
        self.col_offset += amount;
        if self.col_offset < 0.0 {
            self.col_offset = 0.0;
        }
    }

    pub fn clone_cursor(&mut self) {
        self.other_cursor = Some(self.buffer.cursor.current());
        self.update_cursor();
    }

    pub fn delete_char(&mut self) {
        self.buffer.delete_char_at_cursor();
        self.mark_buffer_changed();
        self.update_cursor();
    }

    pub fn insert_newline_and_return(&mut self) {
        self.buffer.insert_newline_and_return();
        self.mark_buffer_changed();
        self.update_cursor();
    }

    pub fn insert_char(&mut self, typed_char: char) {
        self.buffer.insert_char_at_cursor(typed_char);
        self.mark_buffer_changed();
        self.update_cursor();
    }

    pub fn run_search(&mut self) {
        let mut update_search = false;

        if let Some(search) = &mut self.search {
            let last_match =
                self.buffer
                    .search_for(search.last_match(), search.direction(), search.needle());
            search.set_last_match(last_match);
            update_search = true;
        }

        if update_search {
            self.update_search();
        }
    }

    pub fn update_search(&mut self) {
        self.update_cursor();
        self.update_highlighted_sections();
    }

    pub fn start_search(&mut self) {
        self.search = Some(Search::new(self.col_offset(), self.row_offset()));
        self.buffer.cursor.save_cursor();
        self.update_search();
    }

    pub fn stop_search(&mut self) {
        self.search = None;
        self.buffer.clear_search_overlay();
        self.update_highlighted_sections();
        self.update_cursor();
    }

    fn mark_buffer_changed(&mut self) {
        self.update_highlighted_sections();
    }

    pub fn set_ui_scale(&mut self, dpi: f32) {
        self.ui_scale = dpi;
    }

    pub fn set_line_height(&mut self, height: f32) {
        self.line_height = height;
        self.update_font_metrics();
    }

    pub fn set_character_width(&mut self, width: f32) {
        self.character_width = width;
        self.update_font_metrics();
    }

    pub fn section_texts(&self) -> Vec<SectionText> {
        let _guard = flame::start_guard("highlighted_sections -> section_texts");

        let mut section_texts = vec![];

        let (cursor_text_row, cursor_text_col) = self.cursor();
        let rcursor_x = self
            .buffer
            .text_cursor_to_render(cursor_text_col as i32, cursor_text_row as i32)
            as usize;
        for highlighted_section in self.highlighted_sections.iter() {
            if highlighted_section.text_row as i32
                > self.screen_rows() + self.row_offset().floor() as i32
            {
                break;
            }
            if (highlighted_section.text_row as i32) < (self.row_offset().floor() as i32) {
                continue;
            }

            let hl = highlighted_section.highlight;
            let row_text = &self.buffer.rows[highlighted_section.text_row].render;
            let first_col_byte =
                char_position_to_byte_position(row_text, highlighted_section.first_col_idx);
            let last_col_byte =
                char_position_to_byte_position(row_text, highlighted_section.last_col_idx);
            let render_text = &row_text[first_col_byte..=last_col_byte];
            if highlighted_section.text_row == cursor_text_row
                && highlighted_section.first_col_idx <= rcursor_x
                && highlighted_section.last_col_idx >= rcursor_x
            {
                let cursor_offset = rcursor_x - highlighted_section.first_col_idx;
                let cursor_byte_offset = char_position_to_byte_position(render_text, cursor_offset);
                let next_byte_offset =
                    char_position_to_byte_position(render_text, cursor_offset + 1);
                section_texts.push(SectionText {
                    text: &render_text[0..cursor_byte_offset],
                    scale: Scale::uniform(self.font_scale()),
                    color: highlight_to_color(hl),
                    ..SectionText::default()
                });
                section_texts.push(SectionText {
                    text: &render_text[cursor_byte_offset..next_byte_offset],
                    scale: Scale::uniform(self.font_scale()),
                    color: highlight_to_color(Highlight::Cursor),
                    ..SectionText::default()
                });
                section_texts.push(SectionText {
                    text: &render_text[next_byte_offset..],
                    scale: Scale::uniform(self.font_scale()),
                    color: highlight_to_color(hl),
                    ..SectionText::default()
                });
            } else {
                section_texts.push(SectionText {
                    text: &render_text,
                    scale: Scale::uniform(self.font_scale()),
                    color: highlight_to_color(hl),
                    ..SectionText::default()
                });
            };
        }
        section_texts
    }

    fn render_search(
        &self,
        renderer: &mut GlRenderer,
        bounds: Vector2<f32>,
        position: Vector2<f32>,
    ) -> Result<(), Box<dyn Error>> {
        if let Some(search) = self.search.as_ref() {
            let _guard = flame::start_guard("render top left search prompt");

            let top_left_section = Section {
                bounds: bounds.into(),
                screen_position: position.into(),
                text: &search.as_string(),
                color: [0.7, 0.6, 0.5, 1.0],
                scale: Scale::uniform(self.font_scale()),
                z: 0.5,
                ..Section::default()
            };
            renderer.glyph_brush.queue(top_left_section);
            renderer
                .glyph_brush
                .use_queue()
                .depth_target(&renderer.quad_bundle.data.out_depth)
                .draw(&mut renderer.encoder, &renderer.quad_bundle.data.out_color)?;
        }

        Ok(())
    }

    fn render_prompt(
        &self,
        renderer: &mut GlRenderer,
        bounds: Vector2<f32>,
        position: Vector2<f32>,
    ) -> Result<(), Box<dyn Error>> {
        if let Some(top_left_text) = self.prompt_text() {
            let _guard = flame::start_guard("render top left prompt text");

            let text_position: Vector2<f32> = position + vec2(0.0, self.top_padding());
            let top_left_section = Section {
                bounds: bounds.into(),
                screen_position: text_position.into(),
                text: &top_left_text,
                color: [0.7, 0.6, 0.5, 1.0],
                scale: Scale::uniform(self.font_scale()),
                z: 0.5,
                ..Section::default()
            };

            renderer.glyph_brush.queue(top_left_section);

            renderer
                .glyph_brush
                .use_queue()
                .depth_target(&renderer.quad_bundle.data.out_depth)
                .draw(&mut renderer.encoder, &renderer.quad_bundle.data.out_color)?;
        }

        Ok(())
    }

    fn status_text(&self) -> String {
        format!(
            "{} | {} | {}",
            self.status_line.filename, self.status_line.filetype, self.status_line.cursor
        )
    }

    fn render_status_text(
        &self,
        renderer: &mut GlRenderer,
        bounds: Vector2<f32>,
        _position: Vector2<f32>,
        focused: bool,
    ) -> Result<(), Box<dyn Error>> {
        let status_bg = if focused {
            STATUS_FOCUSED_BG
        } else {
            STATUS_UNFOCUS_BG
        };
        let status_fg = if focused {
            STATUS_FOCUSED_FG
        } else {
            STATUS_UNFOCUS_FG
        };

        let status_rect = RectBuilder::new()
            .top_left(vec2(
                self.position.x,
                self.position.y + self.bounds.y - self.line_height(),
            ))
            .bounds(vec2(self.bounds.x, self.line_height()))
            .build();
        {
            let _guard = flame::start_guard("render status quad");
            // Render status background
            renderer.draw_quad(status_bg, status_rect, 0.5);
        }

        {
            let _guard = flame::start_guard("render status text");
            let status_text = self.status_text();
            let status_section = Section {
                bounds: bounds.into(),
                screen_position: status_rect.top_left.into(),
                text: &status_text,
                color: status_fg,
                scale: Scale::uniform(self.font_scale()),
                z: 0.5,
                ..Section::default()
            };

            renderer.glyph_brush.queue(status_section);

            renderer
                .glyph_brush
                .use_queue()
                .depth_target(&renderer.quad_bundle.data.out_depth)
                .draw(&mut renderer.encoder, &renderer.quad_bundle.data.out_color)?;
        }

        Ok(())
    }

    fn render_cursors(
        &self,
        renderer: &mut GlRenderer,
        _bounds: Vector2<f32>,
        _position: Vector2<f32>,
        focused: bool,
    ) -> Result<(), Box<dyn Error>> {
        let _guard = flame::start_guard("render cursors");

        let cursor_bg = if focused {
            CURSOR_FOCUSED_BG
        } else {
            CURSOR_UNFOCUS_BG
        };

        let cursor_rect = self.onscreen_cursor(&self.buffer.cursor);
        renderer.draw_quad(cursor_bg, cursor_rect, 0.2);

        if let Some(other_cursor) = self.other_cursor {
            let other_cursor_rect = self.onscreen_cursor(&other_cursor);
            renderer.draw_quad(OTHER_CURSOR_BG, other_cursor_rect, 0.2);
        }

        Ok(())
    }

    fn render_text(
        &self,
        renderer: &mut GlRenderer,
        bounds: Vector2<f32>,
        position: Vector2<f32>,
    ) -> Result<(), Box<dyn Error>> {
        let _guard = flame::start_guard("render buffer text");

        let text_pos = vec2(self.left_padding(), self.top_padding()) + position;

        let section = VariedSection {
            bounds: bounds.into(),
            screen_position: text_pos.into(),
            text: self.section_texts(),
            z: 1.0,
            ..VariedSection::default()
        };
        renderer.glyph_brush.queue(section);

        let default_transform: Matrix4<f32> =
            gfx_glyph::default_transform(&renderer.quad_bundle.data.out_color).into();
        let transform = self.row_offset_as_transform() * default_transform;
        renderer
            .glyph_brush
            .use_queue()
            .transform(transform)
            .depth_target(&renderer.quad_bundle.data.out_depth)
            .draw(&mut renderer.encoder, &renderer.quad_bundle.data.out_color)?;

        Ok(())
    }

    fn render_lines(
        &self,
        renderer: &mut GlRenderer,
        _bounds: Vector2<f32>,
        _position: Vector2<f32>,
    ) -> Result<(), Box<dyn Error>> {
        let _guard = flame::start_guard("render lines");

        for line in LINE_COLS_AT.iter() {
            let x_on_screen = self.left_padding() + (*line as f32 * self.character_width());
            let rect = RectBuilder::new()
                .bounds(vec2(1.0, self.bounds.y))
                .top_left(vec2(x_on_screen, 0.0))
                .build();
            renderer.draw_quad(LINE_COL_BG, rect, 0.2);
        }

        Ok(())
    }

    pub fn render(&self, renderer: &mut GlRenderer, focused: bool) -> Result<(), Box<dyn Error>> {
        let padded_position = self.position + vec2(self.left_padding(), 0.0);

        self.render_text(renderer, self.bounds, self.position)?;
        self.render_cursors(renderer, self.bounds, padded_position, focused)?;
        self.render_lines(renderer, self.bounds, padded_position)?;
        self.render_prompt(renderer, self.bounds, padded_position)?;
        self.render_search(renderer, self.bounds, padded_position)?;
        self.render_status_text(renderer, self.bounds, self.position, focused)?;

        Ok(())
    }

    pub fn start_prompt(&mut self, prompt: Input<'a>) {
        self.prompt = Some(prompt);
        self.buffer.cursor.save_cursor();
        self.update_cursor();
    }

    fn top_prompt_visible(&self) -> bool {
        self.prompt.is_some() || self.search.is_some()
    }

    pub fn handle_key(&mut self, key: Key) -> (bool, Option<WindowAction>) {
        let mut window_action = None;
        let mut prompt_handled = false;
        let mut search_handled = false;

        if let Some(prompt) = self.prompt.as_mut() {
            prompt_handled = prompt.handle_key(key);
            window_action = self.check_prompt();
        }

        if !prompt_handled {
            if let Some(search) = self.search.as_mut() {
                search_handled = search.handle_key(key);
                if search_handled {
                    self.check_search();
                }
            }
        }

        (prompt_handled || search_handled, window_action)
    }

    pub fn prompt_text(&self) -> Option<&str> {
        self.prompt.as_ref().map(|prompt| prompt.display_text())
    }

    pub fn stop_prompt(&mut self) {
        // FIXME: this has nothing to do with drawing/rendering, MOVE
        self.prompt = None;
        self.buffer.cursor.restore_saved();
        self.update_cursor();
    }

    pub fn check_prompt(&mut self) -> Option<WindowAction> {
        // FIXME: this has nothing to do with drawing/rendering, MOVE
        let mut window_action = None;
        let mut stop_prompt = false;

        if let Some(prompt) = self.prompt.as_mut() {
            if prompt.is_done() || prompt.is_cancelled() {
                stop_prompt = true;
            }
            if prompt.is_done() {
                match prompt.next_action() {
                    Some(PromptAction::SaveFile) => {
                        window_action =
                            Some(WindowAction::SaveFileAs(String::from(prompt.input())));
                    }
                    _ => {}
                }
            }
        }

        if stop_prompt {
            self.stop_prompt();
        }

        window_action
    }

    pub fn check_search(&mut self) {
        // FIXME: this has nothing to do with drawing/rendering, MOVE
        if let Some(search) = self.search.as_ref() {
            if search.run_search() {
                self.run_search();
            } else {
                if search.restore_cursor() {
                    self.buffer.cursor.restore_saved();
                    self.row_offset = search.saved_row_offset();
                    self.col_offset = search.saved_col_offset();
                }
                self.stop_search();
            }
        }
    }

    pub fn save_file(&mut self) -> Result<FileSaveStatus, Box<dyn Error>> {
        // FIXME: this has nothing to do with drawing/rendering, MOVE
        let file_save_status = self.buffer.save_file()?;
        if file_save_status == FileSaveStatus::NoFilename {
            self.start_prompt(Input::new_save_file_input("Save file as", true));
        }
        Ok(file_save_status)
    }
}

#[test]
fn test_update_highlighted_sections() {
    use crate::highlight::Highlight;

    let mut buffer = Buffer::default();
    buffer.set_filename("testfile.c".to_string());
    buffer.append_row("#include <ctype.h>\r\n");
    buffer.append_row("#define KILO_VERSION \"0.0.1\"\r\n");
    buffer.append_row("enum SomeEnum {};\r\n");
    let mut draw_state = DrawState::new(18.0, 1.0, buffer);
    draw_state.update_highlighted_sections();
    let expected_highlights = vec![
        HighlightedSection {
            highlight: Highlight::Normal,
            text_row: 0,
            first_col_idx: 0,
            last_col_idx: 18,
        },
        HighlightedSection {
            highlight: Highlight::Normal,
            text_row: 1,
            first_col_idx: 0,
            last_col_idx: 20,
        },
        HighlightedSection {
            highlight: Highlight::String,
            text_row: 1,
            first_col_idx: 21,
            last_col_idx: 27,
        },
        HighlightedSection {
            highlight: Highlight::Normal,
            text_row: 1,
            first_col_idx: 28,
            last_col_idx: 28,
        },
        HighlightedSection {
            highlight: Highlight::Keyword1,
            text_row: 2,
            first_col_idx: 0,
            last_col_idx: 3,
        },
        HighlightedSection {
            highlight: Highlight::Normal,
            text_row: 2,
            first_col_idx: 4,
            last_col_idx: 17,
        },
    ];
    assert_eq!(expected_highlights, draw_state.highlighted_sections);
}

#[test]
fn test_update_highlighted_sections_no_syntax() {
    use crate::highlight::Highlight;

    let mut buffer = Buffer::default();
    buffer.set_filename("testfile.txt".to_string());
    buffer.append_row("This is a test file\r\n");
    let mut draw_state = DrawState::new(18.0, 1.0, buffer);
    draw_state.update_highlighted_sections();
    let expected_highlights = vec![HighlightedSection {
        highlight: Highlight::Normal,
        text_row: 0,
        first_col_idx: 0,
        last_col_idx: 19,
    }];
    assert_eq!(expected_highlights, draw_state.highlighted_sections);
}
