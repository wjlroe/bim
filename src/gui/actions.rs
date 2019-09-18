use cgmath::Vector2;

#[derive(Copy, Clone)]
pub enum GuiAction {
    SetFontSize(f32),
    SetUiScale(f32),
    SetLineHeight(f32),
    SetCharacterWidth(f32),
    UpdateSize(Vector2<f32>, Vector2<f32>),
}
