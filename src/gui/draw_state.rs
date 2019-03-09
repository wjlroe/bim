use crate::buffer::Buffer;
use crate::highlight::{Highlight, HighlightedSection};
use cgmath::{Matrix4, SquareMatrix, Vector3};

#[derive(Copy, Clone, Default)]
pub struct RenderedCursor {
    pub text_row: i32,
    pub text_col: i32,
    pub moved: bool,
}

impl RenderedCursor {
    pub fn move_col(&mut self, amount: i32) {
        self.text_col += amount;
        self.moved = true;
    }

    pub fn move_row(&mut self, amount: i32) {
        self.text_row += amount;
        self.moved = true;
    }
}

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
    cursor: RenderedCursor,
    cursor_transform: Matrix4<f32>,
    other_cursor: Option<RenderedCursor>,
    other_cursor_transform: Option<Matrix4<f32>>,
    status_transform: Matrix4<f32>,
    buffer: Buffer<'a>,
    pub highlighted_sections: Vec<HighlightedSection>,
    pub status_line: StatusLine,
    row_offset: f32,
    col_offset: f32,
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
            cursor: RenderedCursor::default(),
            cursor_transform: Matrix4::identity(),
            other_cursor: None,
            other_cursor_transform: None,
            status_transform: Matrix4::identity(),
            buffer: Buffer::default(),
            highlighted_sections: vec![],
            status_line: StatusLine::default(),
            row_offset: 0.0,
            col_offset: 0.0,
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
        state.update();
        state
    }

    pub fn update(&mut self) {
        self.scroll();
        self.update_status_line();
        self.update_highlighted_sections();
        self.update_status_transform();
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
        self.window_height - self.line_height as f32
    }

    pub fn screen_rows(&self) -> i32 {
        (self.inner_height() / self.line_height as f32).floor() as i32
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

    pub fn screen_position_vertical_offset(&self) -> f32 {
        self.row_offset * self.line_height
    }

    pub fn row_offset_as_transform(&self) -> [[f32; 4]; 4] {
        let y_move =
            self.screen_position_vertical_offset() / (self.window_height / 2.0);
        let text_transform =
            Matrix4::from_translation(Vector3::new(0.0, y_move, 0.0));
        text_transform.into()
    }

    pub fn col_offset(&self) -> f32 {
        self.col_offset
    }

    fn scroll(&mut self) {
        if self.line_height > 0.0 {
            let screen_rows = self.screen_rows();
            if self.cursor.text_row
                >= self.row_offset.floor() as i32 + screen_rows
            {
                self.row_offset =
                    (self.cursor.text_row - screen_rows + 1) as f32;
            }

            if self.cursor.text_row < self.row_offset.ceil() as i32 {
                self.row_offset = self.cursor.text_row as f32;
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
            self.cursor.text_row + 1,
            self.cursor.text_col + 1,
        );
    }

    fn update_highlighted_sections(&mut self) {
        self.highlighted_sections.clear();
        let mut current_section = HighlightedSection {
            text: String::new(),
            highlight: None,
            start_row_idx: 0,
            end_row_idx: 0,
        };
        for (row_idx, row) in self.buffer.rows.iter().enumerate() {
            let mut highlights = row.hl.iter();
            #[allow(clippy::useless_let_if_seq)]
            for (col_idx, c) in row.render.chars().enumerate() {
                let mut hl =
                    highlights.next().cloned().unwrap_or(Highlight::Normal);
                if row_idx as i32 == self.cursor.text_row
                    && col_idx as i32 == self.cursor.text_col
                {
                    println!(
                        "Cursor is at: ({},{}) on char '{}'",
                        col_idx, row_idx, c
                    );
                    hl = Highlight::Cursor;
                }
                if current_section.highlight.is_none() {
                    current_section.highlight = Some(hl);
                }
                if current_section.highlight == Some(hl) {
                    current_section.text.push(c);
                } else {
                    current_section.end_row_idx = row_idx;
                    self.highlighted_sections.push(current_section.clone());
                    current_section.text.clear();
                    current_section.highlight = None;
                    current_section.text.push(c);
                    current_section.start_row_idx = row_idx;
                }
            }
            current_section.end_row_idx = row_idx;
            current_section.text.push('\n');
        }
        if current_section.text != "" {
            self.highlighted_sections.push(current_section.clone());
        }
    }

    fn update_status_transform(&mut self) {
        let status_height = self.line_height() as f32;
        let status_scale = Matrix4::from_nonuniform_scale(
            1.0,
            status_height / self.window_height,
            1.0,
        );
        let y_move = -((self.window_height - status_height) / status_height);
        let status_move =
            Matrix4::from_translation(Vector3::new(0.0, y_move, 0.0));
        self.status_transform = status_scale * status_move;
    }

    fn update_cursor_transform(&mut self) {
        self.cursor_transform = self.transform_for_cursor(&self.cursor);
        if let Some(other_cursor) = self.other_cursor {
            self.other_cursor_transform =
                Some(self.transform_for_cursor(&other_cursor));
        } else {
            self.other_cursor_transform = None;
        }
    }

    pub fn onscreen_cursor(&self, cursor: &RenderedCursor) -> (f32, f32) {
        let cursor_width = self.character_width();
        let cursor_height = self.line_height();

        let cursor_y = cursor.text_row as f32;
        let cursor_x = cursor.text_col as f32;
        let x_on_screen =
            (cursor_width * cursor_x) + cursor_width / 2.0 + self.left_padding;
        let y_on_screen = (cursor_height * cursor_y) + cursor_height / 2.0;
        println!(
            "Cursor ({},{}) is on screen at: ({},{})",
            cursor_x, cursor_y, x_on_screen, y_on_screen
        );
        (x_on_screen, y_on_screen)
    }

    fn transform_for_cursor(&self, cursor: &RenderedCursor) -> Matrix4<f32> {
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
        let cursor_move =
            Matrix4::from_translation(Vector3::new(x_move, y_move, 0.2));
        cursor_move * cursor_scale
    }

    pub fn print_info(&self) {
        println!(
            "status_height: {}, inner: ({}, {}), status_transform: {:?}",
            self.line_height(),
            self.inner_width(),
            self.inner_height(),
            self.status_transform
        );
    }

    pub fn inc_font_size(&mut self) {
        self.font_size += 1.0;
        self.update();
    }

    pub fn dec_font_size(&mut self) {
        self.font_size -= 1.0;
        self.update();
    }

    pub fn set_window_dimensions(&mut self, (width, height): (u16, u16)) {
        self.window_height = height.into();
        self.window_width = width.into();
        self.update();
    }

    pub fn move_cursor_col(&mut self, amount: i32) {
        self.cursor.move_col(amount);
        if self.cursor.text_col < 0 {
            self.cursor.text_col = 0;
        }
        self.update();
    }

    pub fn move_cursor_row(&mut self, amount: i32) {
        self.cursor.move_row(amount);
        if self.cursor.text_row < 0 {
            self.cursor.text_row = 0;
        }
        self.update();
    }

    pub fn clone_cursor(&mut self) {
        self.other_cursor = Some(self.cursor);
        self.update();
    }

    pub fn set_ui_scale(&mut self, dpi: f32) {
        self.ui_scale = dpi;
        self.update();
    }

    pub fn set_line_height(&mut self, height: f32) {
        self.line_height = height;
        self.update();
    }

    pub fn set_character_width(&mut self, width: f32) {
        self.character_width = width;
        self.update();
    }
}
