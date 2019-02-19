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

pub struct DrawState<'a> {
    window_width: f32,
    window_height: f32,
    line_height: i32,
    character_width: i32,
    font_size: f32,
    ui_scale: f32,
    left_padding: f32,
    cursor: RenderedCursor,
    cursor_transform: Matrix4<f32>,
    status_transform: Matrix4<f32>,
    buffer: Buffer<'a>,
    pub highlighted_sections: Vec<HighlightedSection>,
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
            line_height: 0,
            character_width: 0,
            font_size,
            ui_scale,
            left_padding: 12.0,
            cursor: RenderedCursor::default(),
            cursor_transform: Matrix4::identity(),
            status_transform: Matrix4::identity(),
            buffer,
            highlighted_sections: vec![],
        };
        state.update_highlighted_sections();
        state
    }

    pub fn update(&mut self) {
        self.update_highlighted_sections();
        self.update_status_transform();
        self.update_cursor_transform();
    }

    pub fn line_height(&self) -> i32 {
        self.line_height
    }

    pub fn character_width(&self) -> i32 {
        self.character_width
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
        let cursor_y = self.cursor.text_row as f32;
        let cursor_x = self.cursor.text_col as f32;
        let cursor_height = self.line_height() as f32;
        let cursor_width = self.character_width() as f32;
        let cursor_scale = Matrix4::from_nonuniform_scale(
            cursor_width / self.window_width,
            cursor_height / self.window_height,
            1.0,
        );
        let y_move = -((((cursor_height * cursor_y) + cursor_height / 2.0)
            / self.window_height)
            * 2.0
            - 1.0);
        println!("cursor row: {}, cursor y_move: {:?}", cursor_y, y_move);
        let x_move = (((cursor_width * cursor_x)
            + cursor_width / 2.0
            + self.left_padding)
            / self.window_width)
            * 2.0
            - 1.0;
        println!("cursor col: {}, cursor x_move: {:?}", cursor_x, x_move);
        let cursor_move =
            Matrix4::from_translation(Vector3::new(x_move, y_move, 0.2));
        self.cursor_transform = cursor_move * cursor_scale;
    }

    pub fn inner_width(&self) -> f32 {
        self.window_width - self.left_padding
    }

    pub fn inner_height(&self) -> f32 {
        self.window_height
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

    pub fn set_ui_scale(&mut self, dpi: f32) {
        self.ui_scale = dpi;
        self.update();
    }

    pub fn set_line_height(&mut self, height: i32) {
        self.line_height = height;
        self.update();
    }

    pub fn set_character_width(&mut self, width: i32) {
        self.character_width = width;
        self.update();
    }
}
