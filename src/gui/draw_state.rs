use crate::buffer::Buffer;
use crate::commands::SearchDirection;
use crate::cursor::{Cursor, CursorT};
use crate::highlight::HighlightedSection;
use crate::highlight::{highlight_to_color, Highlight};
use crate::utils::char_position_to_byte_position;
use cgmath::{Matrix4, SquareMatrix, Vector3};
use flame;
use gfx_glyph::{Scale, SectionText};

const LINE_COLS_AT: [u32; 2] = [80, 120];

#[derive(Clone, Default)]
pub struct StatusLine {
    pub filename: String,
    pub num_lines: String,
    pub filetype: String,
    pub cursor: String,
}

pub struct DrawState<'a> {
    window_width: f32,
    window_height: f32,
    line_height: f32,
    character_width: f32,
    font_size: f32,
    ui_scale: f32,
    left_padding: f32,
    pub mouse_position: (f64, f64),
    cursor_transform: Matrix4<f32>,
    other_cursor: Option<Cursor>,
    other_cursor_transform: Option<Matrix4<f32>>,
    status_transform: Matrix4<f32>,
    pub buffer: Buffer<'a>,
    pub highlighted_sections: Vec<HighlightedSection>,
    pub status_line: StatusLine,
    pub row_offset: f32,
    pub col_offset: f32,
    screen_rows: i32,
    pub search_visible: bool,
}

impl<'a> Default for DrawState<'a> {
    fn default() -> Self {
        Self {
            window_width: 0.0,
            window_height: 0.0,
            line_height: 0.0,
            character_width: 0.0,
            font_size: 0.0,
            ui_scale: 0.0,
            left_padding: 0.0,
            mouse_position: (0.0, 0.0),
            cursor_transform: Matrix4::identity(),
            other_cursor: None,
            other_cursor_transform: None,
            status_transform: Matrix4::identity(),
            buffer: Buffer::default(),
            highlighted_sections: vec![],
            status_line: StatusLine::default(),
            row_offset: 0.0,
            col_offset: 0.0,
            screen_rows: 0,
            search_visible: false,
        }
    }
}

impl<'a> DrawState<'a> {
    pub fn new(
        window_width: f32,
        window_height: f32,
        font_size: f32,
        ui_scale: f32,
        buffer: Buffer<'a>,
    ) -> Self {
        let mut state = DrawState {
            window_width,
            window_height,
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

    pub fn update_window(&mut self) {
        self.update_screen_rows();
        self.scroll();
    }

    pub fn update_font_metrics(&mut self) {
        self.update_screen_rows();
        self.update_status_transform();
        self.update_cursor_transform();
        self.scroll();
    }

    pub fn update_cursor(&mut self) {
        self.scroll();
        self.update_status_line();
        self.update_cursor_transform();
    }

    pub fn line_height(&self) -> f32 {
        self.line_height
    }

    pub fn window_width(&self) -> f32 {
        self.window_width
    }

    pub fn window_height(&self) -> f32 {
        self.window_height
    }

    pub fn inner_width(&self) -> f32 {
        self.window_width - self.left_padding
    }

    pub fn inner_height(&self) -> f32 {
        self.window_height - self.bottom_padding() - self.top_padding()
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
        if self.search_visible {
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

    pub fn status_transform(&self) -> Matrix4<f32> {
        self.status_transform
    }

    pub fn cursor_transform(&self) -> Matrix4<f32> {
        self.cursor_transform
    }

    pub fn other_cursor_transform(&self) -> Option<Matrix4<f32>> {
        self.other_cursor_transform
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
        let y_move = self.screen_position_vertical_offset() / (self.window_height / 2.0);
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

    fn update_status_transform(&mut self) {
        let status_height = self.line_height() as f32;
        let status_scale =
            Matrix4::from_nonuniform_scale(1.0, status_height / self.window_height, 1.0);
        let y_move = -((self.window_height - status_height) / status_height);
        let status_move = Matrix4::from_translation(Vector3::new(0.0, y_move, 0.0));
        self.status_transform = status_scale * status_move;
    }

    fn update_cursor_transform(&mut self) {
        self.cursor_transform = self.transform_for_cursor(&self.buffer.cursor);
        if let Some(other_cursor) = self.other_cursor {
            self.other_cursor_transform = Some(self.transform_for_cursor(&other_cursor));
        } else {
            self.other_cursor_transform = None;
        }
    }

    fn cursor_from_mouse_position(&self, mouse: (f64, f64)) -> (i32, i32) {
        let row_on_screen = ((mouse.1 - f64::from(self.top_padding()))
            / f64::from(self.line_height()))
        .floor() as i32;
        let col_on_screen = ((mouse.0 - f64::from(self.left_padding()))
            / f64::from(self.character_width()))
        .floor() as i32;
        (col_on_screen, row_on_screen)
    }

    pub fn onscreen_cursor<C>(&self, cursor: &C) -> (f32, f32)
    where
        C: CursorT,
    {
        let rcursor_x = self
            .buffer
            .text_cursor_to_render(self.buffer.cursor.text_col(), self.buffer.cursor.text_row());
        let cursor_width = self.character_width();
        let cursor_height = self.line_height();

        let cursor_y = cursor.text_row() as f32;
        let cursor_x = rcursor_x as f32;
        let x_on_screen = (cursor_width * cursor_x) + cursor_width / 2.0 + self.left_padding;
        let y_on_screen = (cursor_height * (cursor_y - self.row_offset))
            + cursor_height / 2.0
            + self.top_padding();
        (x_on_screen, y_on_screen)
    }

    fn transform_for_cursor<C>(&self, cursor: &C) -> Matrix4<f32>
    where
        C: CursorT,
    {
        let cursor_width = self.character_width();
        let cursor_height = self.line_height();

        let cursor_scale = Matrix4::from_nonuniform_scale(
            cursor_width / self.window_width,
            cursor_height / self.window_height,
            1.0,
        );
        let (x_on_screen, y_on_screen) = self.onscreen_cursor(cursor);
        let y_move = -((y_on_screen / self.window_height) * 2.0 - 1.0);
        let x_move = (x_on_screen / self.window_width) * 2.0 - 1.0;
        let cursor_move = Matrix4::from_translation(Vector3::new(x_move, y_move, 0.2));
        cursor_move * cursor_scale
    }

    pub fn line_transforms(&self) -> Vec<Matrix4<f32>> {
        let mut line_transforms = vec![];
        for line in LINE_COLS_AT.iter() {
            let scale = Matrix4::from_nonuniform_scale(1.0 / self.window_width(), 1.0, 1.0);
            let x_on_screen = self.left_padding() + (*line as f32 * self.character_width());
            let x_move = (x_on_screen / self.window_width()) * 2.0 - 1.0;
            let translate = Matrix4::from_translation(Vector3::new(x_move, 0.0, 0.2));
            line_transforms.push(translate * scale);
        }
        line_transforms
    }

    pub fn update_screen_rows(&mut self) {
        self.screen_rows = (self.inner_height() / self.line_height()).floor() as i32;
    }

    pub fn print_info(&self) {
        println!("status_height: {}", self.line_height());
        println!("inner: ({}, {})", self.inner_width(), self.inner_height());
        println!("status_transform: {:?}", self.status_transform);
        println!(
            "cursor on screen: {:?}",
            self.onscreen_cursor(&self.buffer.cursor)
        );
        println!("cursor_transform: {:?}", self.cursor_transform);
    }

    pub fn inc_font_size(&mut self) {
        self.font_size += 1.0;
        self.update_font_metrics();
    }

    pub fn dec_font_size(&mut self) {
        self.font_size -= 1.0;
        self.update_font_metrics();
    }

    pub fn set_window_dimensions(&mut self, (width, height): (u16, u16)) {
        self.window_height = height.into();
        self.window_width = width.into();
        self.update_window();
        // TODO: what happens when window resized so cursor not visible any more?
    }

    pub fn move_cursor_to_mouse_position(&mut self, mouse: (f64, f64)) {
        let (cursor_x, cursor_y) = self.cursor_from_mouse_position(mouse);
        let move_y = cursor_y - self.buffer.cursor.text_row();
        let move_x = cursor_x - self.buffer.cursor.text_col();
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

    pub fn scroll_window_vertically(&mut self, amount: f32) {
        self.row_offset += amount;
        if self.row_offset < 0.0 {
            self.row_offset = 0.0;
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

    pub fn search_for(
        &mut self,
        last_match: Option<(usize, usize)>,
        direction: SearchDirection,
        needle: &str,
    ) -> Option<(usize, usize)> {
        let next_match = self.buffer.search_for(last_match, direction, needle);
        self.update_search();
        next_match
    }

    pub fn update_search(&mut self) {
        self.update_cursor();
        self.update_highlighted_sections();
    }

    pub fn stop_search(&mut self) {
        self.search_visible = false;
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
}

#[test]
fn test_update_highlighted_sections() {
    use crate::highlight::Highlight;

    let mut buffer = Buffer::default();
    buffer.set_filename("testfile.c".to_string());
    buffer.append_row("#include <ctype.h>\r\n");
    buffer.append_row("#define KILO_VERSION \"0.0.1\"\r\n");
    buffer.append_row("enum SomeEnum {};\r\n");
    let mut draw_state = DrawState::new(100.0, 100.0, 18.0, 1.0, buffer);
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
    let mut draw_state = DrawState::new(100.0, 100.0, 18.0, 1.0, buffer);
    draw_state.update_highlighted_sections();
    let expected_highlights = vec![HighlightedSection {
        highlight: Highlight::Normal,
        text_row: 0,
        first_col_idx: 0,
        last_col_idx: 19,
    }];
    assert_eq!(expected_highlights, draw_state.highlighted_sections);
}
