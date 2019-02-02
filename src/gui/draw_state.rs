use cgmath::{Matrix4, Vector3};

#[derive(Copy, Clone, Default)]
pub struct RenderedCursor {
    pub text_row: i32,
    pub text_col: i32,
}

impl RenderedCursor {
    pub fn move_right(&mut self, amount: i32) {
        self.text_col += amount;
    }

    pub fn move_down(&mut self, amount: i32) {
        self.text_row += amount;
    }
}

#[derive(Copy, Clone, Default)]
pub struct DrawState {
    pub window_width: f32,
    pub window_height: f32,
    pub line_height: i32,
    pub font_size: f32,
    pub ui_scale: f32,
    pub left_padding: f32,
    pub resized: bool,
    pub cursor: RenderedCursor,
}

impl DrawState {
    pub fn font_size(&self) -> f32 {
        self.font_size * self.ui_scale
    }

    pub fn status_height(&self) -> f32 {
        self.font_size * self.ui_scale
    }

    pub fn status_transform(&self) -> Matrix4<f32> {
        let status_height = self.status_height();
        let status_scale = Matrix4::from_nonuniform_scale(
            1.0,
            status_height / self.window_height,
            1.0,
        );
        let y_move = -((self.window_height - status_height) / status_height);
        let status_move =
            Matrix4::from_translation(Vector3::new(0.0, y_move, 0.0));
        status_scale * status_move
    }

    pub fn inner_width(&self) -> f32 {
        self.window_width - self.left_padding
    }

    pub fn inner_height(&self) -> f32 {
        self.window_height - self.status_height()
    }

    pub fn print_info(&self) {
        println!(
            "status_height: {}, inner: ({}, {}), status_transform: {:?}",
            self.status_height(),
            self.inner_width(),
            self.inner_height(),
            self.status_transform()
        );
    }

    pub fn inc_font_size(&mut self) {
        self.font_size += 1.0;
        self.resized = true;
    }

    pub fn dec_font_size(&mut self) {
        self.font_size -= 1.0;
        self.resized = true;
    }

    pub fn set_window_dimensions(&mut self, (width, height): (u16, u16)) {
        let (width, height) = (f32::from(width), f32::from(height));
        self.window_height = height;
        self.window_width = width;
    }
}
