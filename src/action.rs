use crate::commands::{Direction, MoveCursor};
use crate::gui::mouse::MouseMove;
use cgmath::Vector2;

#[derive(Clone, Debug, PartialEq)]
pub enum WindowAction {
    SaveFileAs(String), // FIXME: this isn't a _window_ action surely?
    FocusPane(Direction),
}

#[derive(Clone, Debug, PartialEq)]
pub enum BufferAction {
    InsertNewlineAndReturn,
    InsertChar(char),
    DeleteChar(Direction),
    CloneCursor,
    MoveCursor(MoveCursor),
    MouseScroll(MouseMove),
    MouseClick(Vector2<f32>),
    SetFilename(String),
    StartSearch,
    PrintDebugInfo,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum GuiAction {
    SetFontSize(f32),
    SetUiScale(f32),
    SetLineHeight(f32),
    SetCharacterWidth(f32),
    UpdateSize(Vector2<f32>, Vector2<f32>), // FIXME: should be a window action, not entire app
}

#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    OnBuffer(BufferAction),
    OnWindow(WindowAction),
    OnGui(GuiAction),
}
