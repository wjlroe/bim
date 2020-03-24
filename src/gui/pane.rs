use crate::action::{BufferAction, GuiAction, PaneAction, WindowAction};
use crate::buffer::{Buffer, FileSaveStatus};
use crate::colours::Colour;
use crate::commands::{Direction, MoveCursor};
use crate::cursor::{Cursor, CursorT};
use crate::gui::animation::{Animation, AnimationState};
use crate::gui::gl_renderer::GlRenderer;
use crate::gui::window;
use crate::highlight::HighlightedSection;
use crate::highlight::{highlight_to_color, Highlight};
use crate::input::Input;
use crate::mouse::MouseMove;
use crate::prompt::PromptAction;
use crate::rect::{Rect, RectBuilder};
use crate::search::Search;
use crate::status_line::StatusLine;
use crate::utils::char_position_to_byte_position;
use gfx_glyph::{Scale, Section, SectionText, VariedSection};
use glam::{vec2, vec3, Mat4, Vec2};
use lazy_static::lazy_static;
use std::error::Error;
use std::time::Duration;

const LINE_COLS_AT: [u32; 2] = [80, 120];
const CURSOR_BLINK_INTERVAL: u64 = 500;

lazy_static! {
    static ref LINE_COL_BG: Colour = Colour::rgb_from_int_tuple((0, 0, 0));
    static ref STATUS_FOCUSED_BG: Colour = Colour::rgb_from_int_tuple((215, 0, 135));
    static ref STATUS_UNFOCUS_BG: Colour = Colour::rgb_from_int_tuple((215, 0, 135));
    static ref STATUS_FOCUSED_FG: Colour = Colour::rgb_from_int_tuple((255, 255, 255));
    static ref STATUS_UNFOCUS_FG: Colour = STATUS_FOCUSED_FG.darken(0.2);
    static ref CURSOR_FOCUSED_BG: Colour = Colour::rgb_from_int_tuple((250, 250, 250));
    static ref CURSOR_UNFOCUS_BG: Colour = Colour::rgb_from_int_tuple((150, 150, 150));
    static ref OTHER_CURSOR_BG: Colour = Colour::rgb_from_int_tuple((255, 165, 0));
    static ref LINE_HIGHLIGHT_FOCUSED_BG: Colour = window::BG_COLOR.lighten(0.2);
    static ref LINE_HIGHLIGHT_UNFOCUS_BG: Colour = LINE_HIGHLIGHT_FOCUSED_BG.darken(0.1);
}

pub struct Pane<'a> {
    other_cursor: Option<Cursor>,
    pub buffer: Buffer<'a>,
    pub highlighted_sections: Vec<HighlightedSection>,
    pub status_line: StatusLine,
    screen_rows: i32,
    pub prompt: Option<Input<'a>>,
    pub search: Option<Search>,
    focused: bool,
    pub bounds: Vec2,
    position: Vec2,
    line_height: f32,
    character_width: f32,
    pub font_size: f32,
    pub ui_scale: f32,
    left_padding: f32,
    pub row_offset: f32,
    pub col_offset: f32,
    cursor_animation: Animation,
}

impl<'a> Default for Pane<'a> {
    fn default() -> Self {
        Self {
            other_cursor: None,
            buffer: Buffer::default(),
            highlighted_sections: Vec::new(),
            status_line: StatusLine::default(),
            screen_rows: 0,
            prompt: None,
            search: None,
            focused: false,
            bounds: vec2(0.0, 0.0),
            position: vec2(0.0, 0.0),
            line_height: 0.0,
            character_width: 0.0,
            font_size: 0.0,
            ui_scale: 0.0,
            left_padding: 12.0,
            row_offset: 0.0,
            col_offset: 0.0,
            cursor_animation: Animation::new(Duration::from_millis(CURSOR_BLINK_INTERVAL)),
        }
    }
}

impl<'a> Pane<'a> {
    pub fn new(font_size: f32, ui_scale: f32, buffer: Buffer<'a>, focused: bool) -> Self {
        let mut pane = Self {
            buffer,
            font_size,
            ui_scale,
            focused,
            ..Pane::default()
        };
        pane.update();
        pane
    }

    fn get_row_offset_int(&self) -> i32 {
        self.row_offset.floor() as i32
    }

    fn set_search(&mut self, search: Option<Search>) {
        self.search = search;
    }

    fn set_prompt(&mut self, prompt: Option<Input<'a>>) {
        self.prompt = prompt;
    }

    pub fn do_action(&mut self, action: PaneAction) {
        use PaneAction::*;

        match action {
            UpdateSize(bounds, position) => self.update_size(bounds, position),
            MouseScroll(delta) => self.mouse_scroll(delta),
            MouseClick(location) => self.mouse_click(location),
            PrintDebugInfo => self.print_info(),
        }
    }

    fn scroll(&mut self) {
        if self.line_height > 0.0 {
            if self.buffer.cursor.text_row() >= self.row_offset.floor() as i32 + self.screen_rows {
                self.row_offset = (self.buffer.cursor.text_row() - self.screen_rows + 1) as f32;
            }

            if self.buffer.cursor.text_row() < self.row_offset.ceil() as i32 {
                self.row_offset = self.buffer.cursor.text_row() as f32;
            }
        }
    }

    fn print_info(&self) {
        println!("status_height: {}", self.line_height);
        println!("inner: ({}, {})", self.inner_width(), self.inner_height());
        println!(
            "cursor on screen: {:?}",
            self.onscreen_cursor(&self.buffer.cursor)
        );
        println!("screen_rows: {}", self.screen_rows);
        println!("bounds: {:?}", self.bounds);
        println!("position: {:?}", self.position);
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

    fn set_highlighted_sections(&mut self, mut highlighted_sections: Vec<HighlightedSection>) {
        self.highlighted_sections.clear();
        self.highlighted_sections.append(&mut highlighted_sections);
    }

    fn update_cursor(&mut self) {
        self.update_screen_rows();
        self.scroll();
        self.update_status_line();
    }

    fn mouse_scroll(&mut self, delta: MouseMove) {
        match delta {
            MouseMove::Lines(lines) => {
                self.scroll_window_vertically(lines.y());
                self.scroll_window_horizontally(lines.x());
            }
            MouseMove::Pixels(pixels) => {
                self.scroll_window_vertically(f32::ceil(pixels.y() / self.line_height));
                self.scroll_window_horizontally(pixels.x() / self.character_width);
            }
        }
        self.update_cursor();
    }

    pub fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
    }

    fn new_search(&self) -> Search {
        Search::new(self.col_offset, self.row_offset)
    }

    fn restore_from_search(&mut self, search: Search) {
        self.row_offset = search.saved_row_offset();
        self.col_offset = search.saved_col_offset();
    }

    fn move_cursor<F>(&mut self, func: F)
    where
        F: Fn(&mut Cursor),
    {
        self.buffer.cursor.change(func);
        self.cursor_animation.cancel();
    }

    fn update_size(&mut self, bounds: Vec2, position: Vec2) {
        self.bounds = bounds;
        self.position = position;
        self.update_screen_rows();
        self.scroll();
    }

    fn row_offset_as_transform(&self) -> Mat4 {
        let y_move = self.screen_position_vertical_offset() / (self.bounds.y() / 2.0);
        Mat4::from_translation(vec3(0.0, y_move, 0.0))
    }

    pub fn section_texts(&self) -> Vec<SectionText<'_>> {
        let _guard = flame::start_guard("highlighted_sections -> section_texts");

        let mut section_texts = vec![];

        let (cursor_text_row, cursor_text_col) = self.cursor();
        let rcursor_x = self
            .buffer
            .text_cursor_to_render(cursor_text_col as i32, cursor_text_row as i32)
            as usize;
        for highlighted_section in self.highlighted_sections.iter() {
            if highlighted_section.text_row as i32
                > self.screen_rows + self.row_offset.floor() as i32
            {
                break;
            }
            if (highlighted_section.text_row as i32) < (self.row_offset.floor() as i32) {
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

    pub fn onscreen_cursor<C>(&self, cursor: &C) -> Rect
    where
        C: CursorT,
    {
        let rcursor_x = self
            .buffer
            .text_cursor_to_render(cursor.text_col(), cursor.text_row());
        let cursor_width = self.character_width;
        let cursor_height = self.line_height;

        let cursor_y = cursor.text_row() as f32;
        let cursor_x = rcursor_x as f32;
        let x_on_screen = (cursor_width * cursor_x) + self.left_padding;
        let y_on_screen = (cursor_height * (cursor_y - self.row_offset)) + self.top_padding();
        RectBuilder::new()
            .bounds(vec2(cursor_width, cursor_height))
            .top_left(self.position + vec2(x_on_screen, y_on_screen))
            .build()
    }

    fn render_status_text(
        &self,
        renderer: &mut GlRenderer<'_>,
        bounds: Vec2,
        _position: Vec2,
        focused: bool,
    ) -> Result<(), Box<dyn Error>> {
        let status_bg = if focused {
            *STATUS_FOCUSED_BG
        } else {
            *STATUS_UNFOCUS_BG
        };
        let status_fg = if focused {
            *STATUS_FOCUSED_FG
        } else {
            *STATUS_UNFOCUS_FG
        };

        let status_rect = RectBuilder::new()
            .top_left(vec2(
                self.position.x(),
                self.position.y() + self.bounds.y() - self.line_height,
            ))
            .bounds(vec2(self.bounds.x(), self.line_height))
            .build();
        {
            let _guard = flame::start_guard("render status quad");
            // Render status background
            renderer.draw_quad(status_bg.rgb(), status_rect, 0.5);
        }

        {
            let _guard = flame::start_guard("render status text");
            let status_section = Section {
                bounds: bounds.into(),
                screen_position: status_rect.top_left.into(),
                text: &self.status_text(),
                color: status_fg.rgba(),
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

    fn render_highlight_line(
        &self,
        renderer: &mut GlRenderer<'_>,
        bounds: Vec2,
        position: Vec2,
        focused: bool,
    ) -> Result<(), Box<dyn Error>> {
        let _guard = flame::start_guard("render highlight line");

        let hl_colour = if focused {
            *LINE_HIGHLIGHT_FOCUSED_BG
        } else {
            *LINE_HIGHLIGHT_UNFOCUS_BG
        };
        let cursor_rect = self.onscreen_cursor(&self.buffer.cursor);
        let highlight_line_rect = RectBuilder::new()
            .bounds(vec2(bounds.x(), self.line_height))
            .top_left(vec2(position.x(), cursor_rect.top_left.y()))
            .build();
        renderer.draw_quad(hl_colour.rgb(), highlight_line_rect, 1.0);
        Ok(())
    }

    fn render_cursors(
        &self,
        renderer: &mut GlRenderer<'_>,
        _bounds: Vec2,
        _position: Vec2,
        focused: bool,
    ) -> Result<(), Box<dyn Error>> {
        let _guard = flame::start_guard("render cursors");

        if !focused || self.cursor_animation.state == AnimationState::Show {
            let cursor_bg = if focused {
                *CURSOR_FOCUSED_BG
            } else {
                *CURSOR_UNFOCUS_BG
            };

            let cursor_rect = self.onscreen_cursor(&self.buffer.cursor);
            renderer.draw_quad(cursor_bg.rgb(), cursor_rect, 0.2);
        }

        if let Some(other_cursor) = self.other_cursor {
            let other_cursor_rect = self.onscreen_cursor(&other_cursor);
            renderer.draw_quad(OTHER_CURSOR_BG.rgb(), other_cursor_rect, 0.2);
        }

        Ok(())
    }

    fn render_text(
        &self,
        renderer: &mut GlRenderer<'_>,
        bounds: Vec2,
        position: Vec2,
    ) -> Result<(), Box<dyn Error>> {
        let _guard = flame::start_guard("render buffer text");

        let padding = vec2(self.left_padding, self.top_padding());
        let text_pos = padding + position;
        let inner_bounds = bounds - padding;

        let section = VariedSection {
            bounds: inner_bounds.into(),
            screen_position: text_pos.into(),
            text: self.section_texts(),
            z: 1.0,
            ..VariedSection::default()
        };
        renderer.glyph_brush.queue(section);

        let default_transform: Mat4 = Mat4::from_cols_array_2d(&gfx_glyph::default_transform(
            &renderer.quad_bundle.data.out_color,
        ));
        let transform = self.row_offset_as_transform() * default_transform;
        renderer
            .glyph_brush
            .use_queue()
            .transform(transform.to_cols_array_2d())
            .depth_target(&renderer.quad_bundle.data.out_depth)
            .draw(&mut renderer.encoder, &renderer.quad_bundle.data.out_color)?;

        Ok(())
    }

    fn render_lines(
        &self,
        renderer: &mut GlRenderer<'_>,
        bounds: Vec2,
        position: Vec2,
    ) -> Result<(), Box<dyn Error>> {
        let _guard = flame::start_guard("render lines");

        for line in LINE_COLS_AT.iter() {
            let x_in_bounds = *line as f32 * self.character_width;
            if x_in_bounds < bounds.x() {
                let x_on_screen = position.x() + x_in_bounds;
                let rect = RectBuilder::new()
                    .bounds(vec2(1.0, bounds.y()))
                    .top_left(vec2(x_on_screen, 0.0))
                    .build();
                renderer.draw_quad(LINE_COL_BG.rgb(), rect, 0.2);
            }
        }

        Ok(())
    }

    fn render_search(
        &self,
        renderer: &mut GlRenderer<'_>,
        bounds: Vec2,
        position: Vec2,
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
        renderer: &mut GlRenderer<'_>,
        bounds: Vec2,
        position: Vec2,
    ) -> Result<(), Box<dyn Error>> {
        if let Some(top_left_text) = self.prompt.as_ref().map(|prompt| prompt.display_text()) {
            let _guard = flame::start_guard("render top left prompt text");

            let text_position: Vec2 = position + vec2(0.0, self.top_padding());
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

    pub fn render(
        &self,
        renderer: &mut GlRenderer<'_>,
        focused: bool,
    ) -> Result<(), Box<dyn Error>> {
        let padded_position = self.position + vec2(self.left_padding, 0.0);
        let new_bounds = self.bounds - vec2(self.left_padding, 0.0);

        self.render_highlight_line(renderer, self.bounds, self.position, focused)?;
        self.render_text(renderer, self.bounds, self.position)?;
        self.render_cursors(renderer, new_bounds, padded_position, focused)?;
        self.render_lines(renderer, new_bounds, padded_position)?;
        self.render_prompt(renderer, new_bounds, padded_position)?;
        self.render_search(renderer, new_bounds, padded_position)?;
        self.render_status_text(renderer, self.bounds, self.position, focused)?;

        Ok(())
    }

    pub fn update_gui(&mut self, action: GuiAction) {
        use GuiAction::*;

        match action {
            UpdateSize(bounds, position) => self.update_size(bounds, position),
            SetFontSize(font_size) => self.set_font_size(font_size),
            SetUiScale(dpi) => self.set_ui_scale(dpi),
            SetLineHeight(line_height) => self.set_line_height(line_height),
            SetCharacterWidth(character_width) => self.set_character_width(character_width),
            DumpFlameGraph => {}
            DecFontSize => {}
            IncFontSize => {}
            Quit => {}
            PrintInfo => {}
        }
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

    fn set_font_size(&mut self, font_size: f32) {
        self.font_size = font_size;
        self.update_font_metrics();
    }

    fn inner_width(&self) -> f32 {
        self.bounds.x() - self.left_padding
    }

    fn inner_height(&self) -> f32 {
        self.bounds.y() - self.bottom_padding() - self.top_padding()
    }

    fn font_scale(&self) -> f32 {
        self.ui_scale * self.font_size
    }

    fn top_padding(&self) -> f32 {
        if self.top_prompt_visible() {
            self.line_height // if search is on
        } else {
            0.0
        }
    }

    fn bottom_padding(&self) -> f32 {
        self.line_height // status line
    }

    fn screen_position_vertical_offset(&self) -> f32 {
        self.row_offset.fract() * self.line_height
    }

    fn cursor_from_mouse_position(&self, mouse: Vec2) -> (i32, i32) {
        let row_on_screen =
            ((mouse.y() - self.top_padding()) / self.line_height + self.row_offset).floor() as i32;
        let col_on_screen = ((mouse.x() - self.left_padding) / self.character_width).floor() as i32;
        (col_on_screen, row_on_screen)
    }

    fn move_cursor_to_mouse_position(&mut self, mouse: Vec2) {
        let cursor = self.cursor_from_mouse_position(mouse);
        let clicked_line = i32::min((self.buffer.num_lines() as i32) - 1, cursor.1);
        let clicked_line_length = self.buffer.line_len(clicked_line).unwrap_or(0) as i32;
        let clicked_line_x = i32::min(clicked_line_length, cursor.0);
        let move_y = clicked_line - self.buffer.cursor.text_row();
        let move_x = clicked_line_x - self.buffer.cursor.text_col();
        self.buffer.cursor.change(|cursor| {
            cursor.text_row += move_y;
            cursor.text_col += move_x;
        });
        self.update_cursor();
    }

    fn mouse_click(&mut self, location: Vec2) {
        println!("mouse click: {:?}", location);
        self.move_cursor_to_mouse_position(location);
    }

    fn is_cursor_onscreen(&self) -> bool {
        let row = self.buffer.cursor.text_row();
        let row_offset = self.row_offset.floor() as i32;
        row >= row_offset && row < row_offset + self.screen_rows
    }

    fn update_screen_rows(&mut self) {
        self.screen_rows = (self.inner_height() / self.line_height).floor() as i32;
    }

    fn update_font_metrics(&mut self) {
        self.update_screen_rows();
        self.scroll();
    }

    fn scroll_window_vertically(&mut self, amount: f32) {
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

    fn scroll_window_horizontally(&mut self, amount: f32) {
        self.col_offset += amount;
        if self.col_offset < 0.0 {
            self.col_offset = 0.0;
        }
    }

    pub fn update_dt(&mut self, duration: Duration) {
        self.cursor_animation.add_duration(duration);
    }

    pub fn is_dirty(&self) -> bool {
        self.buffer.is_dirty()
    }

    fn update(&mut self) {
        self.update_highlighted_sections();
        self.update_status_line();
    }

    fn cursor(&self) -> (usize, usize) {
        (
            self.buffer.cursor.text_row() as usize,
            self.buffer.cursor.text_col() as usize,
        )
    }

    pub fn update_buffer(&mut self, action: BufferAction) {
        use BufferAction::*;

        match action {
            InsertNewlineAndReturn => self.insert_newline_and_return(),
            InsertChar(typed_char) => self.insert_char(typed_char),
            DeleteChar(direction) => self.delete_char(direction),
            CloneCursor => self.clone_cursor(),
            MoveCursor(movement) => self.do_cursor_movement(movement),
            SetFilename(filename) => self.buffer.set_filename(filename),
            SetFiletype(filetype) => self.buffer.set_filetype(&filetype),
            StartSearch => self.start_search(),
            InsertTypedChar => {
                panic!("Insert typed char received in DrawState.update_buffer, this should not happen!");
            }
        }
    }

    fn update_highlighted_sections(&mut self) {
        let mut highlighted_sections = Vec::new();
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
                        highlighted_sections.push(current_section);
                    }
                    current_section.highlight = overlay_or_hl;
                    current_section.first_col_idx = col_idx;
                    current_section.last_col_idx = col_idx;
                }
                first_char_seen = true;
            }

            if first_char_seen {
                highlighted_sections.push(current_section);
            }
        }
        self.set_highlighted_sections(highlighted_sections);
    }

    fn do_cursor_movement(&mut self, movement: MoveCursor) {
        use crate::commands::Direction::*;
        use crate::commands::MoveUnit::*;

        let page_size = self.screen_rows as usize;
        let num_lines = self.buffer.num_lines();

        match movement {
            MoveCursor {
                unit: Rows,
                direction: Up,
                amount,
            } => {
                if let Some(search) = self.search.as_mut() {
                    search.go_backwards();
                } else {
                    self.move_cursor(|cursor| {
                        let max_amount = cursor.text_row();
                        let possible_amount = std::cmp::min(amount as i32, max_amount);
                        cursor.text_row -= possible_amount;
                    });
                }
            }
            MoveCursor {
                unit: Rows,
                direction: Down,
                amount,
            } => {
                if let Some(search) = self.search.as_mut() {
                    search.go_forwards();
                } else {
                    self.move_cursor(|cursor| {
                        let max_movement = num_lines as i32 - 1 - cursor.text_row();
                        let possible_amount = std::cmp::min(amount as i32, max_movement);
                        cursor.text_row += possible_amount;
                    });
                }
            }
            MoveCursor {
                unit: Cols,
                direction: Left,
                amount,
            } => {
                if let Some(search) = self.search.as_mut() {
                    search.go_backwards();
                } else {
                    let mut new_cursor = self.buffer.cursor.current();
                    let mut left_amount = amount as i32;
                    while left_amount > 0 {
                        if new_cursor.text_col != 0 {
                            new_cursor.text_col -= 1;
                        } else if new_cursor.text_row > 0 {
                            new_cursor.text_row -= 1;
                            new_cursor.text_col =
                                self.buffer.line_len(new_cursor.text_row).unwrap_or(0) as i32;
                        } else {
                            break;
                        }
                        left_amount -= 1;
                    }
                    self.move_cursor(|cursor| {
                        cursor.text_col = new_cursor.text_col();
                        cursor.text_row = new_cursor.text_row();
                    });
                }
            }
            MoveCursor {
                unit: Cols,
                direction: Right,
                amount,
            } => {
                if let Some(search) = self.search.as_mut() {
                    search.go_forwards();
                } else {
                    let mut new_cursor = self.buffer.cursor.current();
                    let mut right_amount = amount as i32;
                    let num_lines = self.buffer.num_lines() as i32;
                    while right_amount > 0 {
                        if let Some(row_size) = self.buffer.line_len(new_cursor.text_row) {
                            if new_cursor.text_col < row_size as i32 {
                                new_cursor.text_col += 1;
                            } else if new_cursor.text_col == row_size as i32
                                && new_cursor.text_row < num_lines - 1
                            {
                                new_cursor.text_row += 1;
                                new_cursor.text_col = 0;
                            } else {
                                break;
                            }
                            right_amount -= 1;
                        } else {
                            break;
                        }
                    }
                    self.move_cursor(|cursor| {
                        cursor.text_col = new_cursor.text_col();
                        cursor.text_row = new_cursor.text_row();
                    });
                }
            }
            MoveCursor {
                unit: Start,
                direction: Left,
                ..
            } => self.buffer.cursor.change(|cursor| cursor.text_col = 0),
            MoveCursor {
                unit: End,
                direction: Right,
                ..
            } => {
                let new_x = self
                    .buffer
                    .line_len(self.buffer.cursor.text_row())
                    .unwrap_or(0) as i32;
                self.move_cursor(|cursor| {
                    cursor.text_col = new_x;
                });
            }
            MoveCursor {
                unit: Pages,
                direction: Down,
                amount,
            } => {
                let amount = amount * page_size;
                self.do_cursor_movement(MoveCursor::down(amount));
            }
            MoveCursor {
                unit: Pages,
                direction: Up,
                amount,
            } => {
                let amount = amount * page_size;
                self.do_cursor_movement(MoveCursor::up(amount));
            }
            _ => {}
        }
        self.buffer.check_cursor();
        self.update_cursor();
    }

    fn move_cursor_onscreen(&mut self) {
        let row_offset = self.get_row_offset_int();
        self.move_cursor(|cursor| {
            cursor.text_row = row_offset;
        });
    }

    fn clone_cursor(&mut self) {
        self.other_cursor = Some(self.buffer.cursor.current());
        self.update_cursor();
    }

    fn delete_char(&mut self, direction: Direction) {
        if let Some(prompt) = self.prompt.as_mut() {
            prompt.del_char();
            return;
        }
        if let Some(search) = self.search.as_mut() {
            search.del_char();
            return;
        }

        if direction == Direction::Right {
            self.update_buffer(BufferAction::MoveCursor(MoveCursor::right(1)));
        }
        self.buffer.delete_char_at_cursor();
        self.mark_buffer_changed();
        self.update_cursor();
    }

    fn insert_newline_and_return(&mut self) {
        if let Some(prompt) = &mut self.prompt {
            prompt.done();
            return;
        }
        if let Some(search) = &mut self.search {
            search.stop(false);
            return;
        }
        self.buffer.insert_newline_and_return();
        self.mark_buffer_changed();
        self.update_cursor();
    }

    fn insert_char(&mut self, typed_char: char) {
        if let Some(prompt) = &mut self.prompt {
            prompt.type_char(typed_char);
            return;
        }
        if let Some(search) = &mut self.search {
            search.push_char(typed_char);
            return;
        }

        self.buffer.insert_char_at_cursor(typed_char);
        self.mark_buffer_changed();
        self.update_cursor();
    }

    fn run_search(&mut self) {
        let mut update_search = false;

        if let Some(search) = self.search.clone() {
            let last_match =
                self.buffer
                    .search_for(search.last_match(), search.direction(), search.needle());
            self.search
                .as_mut()
                .map(|search| search.set_last_match(last_match));
            update_search = true;
        }

        if update_search {
            self.update_search();
        }
    }

    fn update_search(&mut self) {
        self.update_cursor();
        self.update_highlighted_sections();
    }

    fn start_search(&mut self) {
        self.set_search(Some(self.new_search()));
        self.buffer.cursor.save_cursor();
        self.update_search();
    }

    fn stop_search(&mut self) {
        self.set_search(None);
        self.buffer.clear_search_overlay();
        self.update_highlighted_sections();
        self.update_cursor();
    }

    fn mark_buffer_changed(&mut self) {
        self.update_highlighted_sections();
    }

    fn status_text(&self) -> String {
        format!(
            "{} | {} | {}",
            self.status_line.filename, self.status_line.filetype, self.status_line.cursor
        )
    }

    fn start_prompt(&mut self, prompt: Input<'a>) {
        self.set_prompt(Some(prompt));
        self.buffer.cursor.save_cursor();
        self.update_cursor();
    }

    fn top_prompt_visible(&self) -> bool {
        self.prompt.is_some() || self.search.is_some()
    }

    fn stop_prompt(&mut self) {
        self.set_prompt(None);
        self.buffer.cursor.restore_saved();
        self.update_cursor();
    }

    fn check_prompt(&mut self) -> Option<WindowAction> {
        let mut window_action = None;
        let mut stop_prompt = false;

        if let Some(prompt) = self.prompt.as_ref() {
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

    fn check_search(&mut self) {
        if let Some(search) = self.search.clone() {
            if search.run_search() {
                self.run_search();
            } else {
                if search.restore_cursor() {
                    self.buffer.cursor.restore_saved();
                    self.restore_from_search(search);
                }
                self.stop_search();
            }
        }
    }

    pub fn check(&mut self) -> Vec<WindowAction> {
        let mut actions = vec![];

        if let Some(window_action) = self.check_prompt() {
            actions.push(window_action);
        }
        self.check_search();

        actions
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
    let mut pane = Pane::new(18.0, 1.0, buffer, true);
    pane.update_highlighted_sections();
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
    assert_eq!(expected_highlights, pane.highlighted_sections);
}

#[test]
fn test_update_highlighted_sections_no_syntax() {
    use crate::highlight::Highlight;

    let mut buffer = Buffer::default();
    buffer.set_filename("testfile.txt".to_string());
    buffer.append_row("This is a test file\r\n");
    let mut pane = Pane::new(12.0, 1.0, buffer, true);
    pane.update_highlighted_sections();
    let expected_highlights = vec![HighlightedSection {
        highlight: Highlight::Normal,
        text_row: 0,
        first_col_idx: 0,
        last_col_idx: 19,
    }];
    assert_eq!(expected_highlights, pane.highlighted_sections);
}
