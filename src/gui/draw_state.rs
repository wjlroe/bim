use cgmath::{Matrix4, SquareMatrix, Vector3};

#[derive(Copy, Clone, Default)]
pub struct RenderedCursor {
    pub text_row: i32,
    pub text_col: i32,
}

impl RenderedCursor {
    pub fn move_col(&mut self, amount: i32) {
        self.text_col += amount;
    }

    pub fn move_row(&mut self, amount: i32) {
        self.text_row += amount;
    }
}

#[derive(Copy, Clone)]
pub struct DrawState {
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
}

impl DrawState {
    pub fn new(
        window_width: f32,
        window_height: f32,
        font_size: f32,
        ui_scale: f32,
    ) -> Self {
        DrawState {
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
        }
    }
    pub fn update(&mut self) {
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
            Matrix4::from_translation(Vector3::new(x_move, y_move, 0.0));
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
        let (width, height) = (f32::from(width), f32::from(height));
        self.window_height = height;
        self.window_width = width;
        self.update();
    }

    pub fn move_cursor_col(&mut self, amount: i32) {
        self.cursor.move_col(amount);
        self.update();
    }

    pub fn move_cursor_row(&mut self, amount: i32) {
        self.cursor.move_row(amount);
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
