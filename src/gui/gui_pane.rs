use crate::action::{GuiAction, PaneAction};
use crate::buffer::Buffer;
use crate::cursor::{Cursor, CursorT};
use crate::gui::gl_renderer::GlRenderer;
use crate::highlight::HighlightedSection;
use crate::highlight::{highlight_to_color, Highlight};
use crate::input::Input;
use crate::mouse::MouseMove;
use crate::pane::Pane;
use crate::rect::{Rect, RectBuilder};
use crate::search::Search;
use crate::status_line::StatusLine;
use crate::utils::char_position_to_byte_position;
use gfx_glyph::{Scale, Section, SectionText, VariedSection};
use glam::{vec2, vec3, Mat4, Vec2};
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

pub struct GuiPane<'a> {
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
}

impl<'a> Default for GuiPane<'a> {
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
        }
    }
}

impl<'a> Pane<'a> for GuiPane<'a> {
    type Output = GuiPane<'a>;

    fn get_buffer(&self) -> &Buffer<'a> {
        &self.buffer
    }

    fn get_buffer_mut(&mut self) -> &mut Buffer<'a> {
        &mut self.buffer
    }

    fn get_screen_rows(&self) -> i32 {
        self.screen_rows
    }

    fn get_row_offset_int(&self) -> i32 {
        self.row_offset.floor() as i32
    }

    fn get_search(&self) -> Option<&Search> {
        self.search.as_ref()
    }

    fn get_search_mut(&mut self) -> Option<&mut Search> {
        self.search.as_mut()
    }

    fn set_search(&mut self, search: Option<Search>) {
        self.search = search;
    }

    fn get_prompt(&self) -> Option<&Input<'a>> {
        self.prompt.as_ref()
    }

    fn get_prompt_mut(&mut self) -> Option<&mut Input<'a>> {
        self.prompt.as_mut()
    }

    fn set_prompt(&mut self, prompt: Option<Input<'a>>) {
        self.prompt = prompt;
    }

    fn get_other_cursor(&self) -> Option<&Cursor> {
        self.other_cursor.as_ref()
    }

    fn get_other_cursor_mut(&mut self) -> Option<&mut Cursor> {
        self.other_cursor.as_mut()
    }

    fn set_other_cursor(&mut self, cursor: Option<Cursor>) {
        self.other_cursor = cursor;
    }

    fn do_action(&mut self, action: PaneAction) {
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
            if self.buffer.cursor.text_row() >= self.row_offset.floor() as i32 + self.screen_rows()
            {
                self.row_offset = (self.buffer.cursor.text_row() - self.screen_rows() + 1) as f32;
            }

            if self.buffer.cursor.text_row() < self.row_offset.ceil() as i32 {
                self.row_offset = self.buffer.cursor.text_row() as f32;
            }
        }
    }

    fn print_info(&self) {
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

    fn get_status_line(&self) -> &StatusLine {
        &self.status_line
    }

    fn update_status_line(&mut self) {
        let filename = self
            .get_buffer()
            .filename
            .clone()
            .unwrap_or_else(|| String::from("[No Name]"));
        self.status_line.filename = filename;
        self.status_line.filetype = self.get_buffer().get_filetype();
        self.status_line.cursor = format!(
            "{}:{}",
            self.get_buffer().cursor.text_row() + 1,
            self.get_buffer().cursor.text_col() + 1,
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

    fn get_focused(&self) -> bool {
        self.focused
    }

    fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
    }

    fn new_search(&self) -> Search {
        Search::new(self.col_offset, self.row_offset)
    }

    fn restore_from_search(&mut self, search: Search) {
        self.row_offset = search.saved_row_offset();
        self.col_offset = search.saved_col_offset();
    }
}

impl<'a> GuiPane<'a> {
    pub fn new(font_size: f32, ui_scale: f32, buffer: Buffer<'a>, focused: bool) -> Self {
        let mut pane = Self {
            buffer,
            font_size,
            ui_scale,
            focused,
            ..GuiPane::default()
        };
        pane.update();
        pane
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
                > self.screen_rows() + self.row_offset.floor() as i32
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

    fn render_status_text(
        &self,
        renderer: &mut GlRenderer,
        bounds: Vec2,
        _position: Vec2,
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
                self.position.x(),
                self.position.y() + self.bounds.y() - self.line_height(),
            ))
            .bounds(vec2(self.bounds.x(), self.line_height()))
            .build();
        {
            let _guard = flame::start_guard("render status quad");
            // Render status background
            renderer.draw_quad(status_bg, status_rect, 0.5);
        }

        {
            let _guard = flame::start_guard("render status text");
            let status_section = Section {
                bounds: bounds.into(),
                screen_position: status_rect.top_left.into(),
                text: &self.status_text(),
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
        _bounds: Vec2,
        _position: Vec2,
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
        bounds: Vec2,
        position: Vec2,
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
        renderer: &mut GlRenderer,
        bounds: Vec2,
        position: Vec2,
    ) -> Result<(), Box<dyn Error>> {
        let _guard = flame::start_guard("render lines");

        for line in LINE_COLS_AT.iter() {
            let x_in_bounds = *line as f32 * self.character_width();
            if x_in_bounds < bounds.x() {
                let x_on_screen = position.x() + x_in_bounds;
                let rect = RectBuilder::new()
                    .bounds(vec2(1.0, bounds.y()))
                    .top_left(vec2(x_on_screen, 0.0))
                    .build();
                renderer.draw_quad(LINE_COL_BG, rect, 0.2);
            }
        }

        Ok(())
    }

    fn render_search(
        &self,
        renderer: &mut GlRenderer,
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
        renderer: &mut GlRenderer,
        bounds: Vec2,
        position: Vec2,
    ) -> Result<(), Box<dyn Error>> {
        if let Some(top_left_text) = self.get_prompt().map(|prompt| prompt.display_text()) {
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

    pub fn render(&self, renderer: &mut GlRenderer, focused: bool) -> Result<(), Box<dyn Error>> {
        let padded_position = self.position + vec2(self.left_padding(), 0.0);
        let new_bounds = self.bounds - vec2(self.left_padding(), 0.0);

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

    fn line_height(&self) -> f32 {
        self.line_height
    }

    fn inner_width(&self) -> f32 {
        self.bounds.x() - self.left_padding
    }

    fn inner_height(&self) -> f32 {
        self.bounds.y() - self.bottom_padding() - self.top_padding()
    }

    fn screen_rows(&self) -> i32 {
        self.screen_rows
    }

    fn character_width(&self) -> f32 {
        self.character_width
    }

    fn font_scale(&self) -> f32 {
        self.ui_scale * self.font_size
    }

    fn top_padding(&self) -> f32 {
        if self.top_prompt_visible() {
            self.line_height() // if search is on
        } else {
            0.0
        }
    }

    fn bottom_padding(&self) -> f32 {
        self.line_height() // status line
    }

    fn left_padding(&self) -> f32 {
        self.left_padding
    }

    fn screen_position_vertical_offset(&self) -> f32 {
        self.row_offset.fract() * self.line_height
    }

    fn cursor_from_mouse_position(&self, mouse: Vec2) -> (i32, i32) {
        let row_on_screen = ((mouse.y() - self.top_padding()) / self.line_height()
            + self.row_offset)
            .floor() as i32;
        let col_on_screen =
            ((mouse.x() - self.left_padding()) / self.character_width()).floor() as i32;
        (col_on_screen, row_on_screen)
    }

    fn move_cursor_to_mouse_position(&mut self, mouse: Vec2) {
        let cursor = self.cursor_from_mouse_position(mouse);
        let clicked_line = i32::min((self.get_buffer().num_lines() as i32) - 1, cursor.1);
        let clicked_line_length = self.get_buffer().line_len(clicked_line).unwrap_or(0) as i32;
        let clicked_line_x = i32::min(clicked_line_length, cursor.0);
        let move_y = clicked_line - self.get_buffer().cursor.text_row();
        let move_x = clicked_line_x - self.get_buffer().cursor.text_col();
        self.get_buffer_mut().cursor.change(|cursor| {
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
        let row = self.get_buffer().cursor.text_row();
        let row_offset = self.row_offset.floor() as i32;
        row >= row_offset && row < row_offset + self.screen_rows
    }

    fn update_screen_rows(&mut self) {
        self.screen_rows = (self.inner_height() / self.line_height()).floor() as i32;
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
        let max_offset = self.get_buffer().num_lines() as f32;
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
}

#[test]
fn test_update_highlighted_sections() {
    use crate::highlight::Highlight;

    let mut buffer = Buffer::default();
    buffer.set_filename("testfile.c".to_string());
    buffer.append_row("#include <ctype.h>\r\n");
    buffer.append_row("#define KILO_VERSION \"0.0.1\"\r\n");
    buffer.append_row("enum SomeEnum {};\r\n");
    let mut pane = GuiPane::new(18.0, 1.0, buffer, true);
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
    let mut pane = GuiPane::new(12.0, 1.0, buffer, true);
    pane.update_highlighted_sections();
    let expected_highlights = vec![HighlightedSection {
        highlight: Highlight::Normal,
        text_row: 0,
        first_col_idx: 0,
        last_col_idx: 19,
    }];
    assert_eq!(expected_highlights, pane.highlighted_sections);
}
