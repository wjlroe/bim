use crate::commands::{Direction, MoveCursor};
use crate::mouse::MouseMove;
use glam::Vec2;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum GuiAction {
    DecFontSize,
    IncFontSize,
    SetFontSize(f32),
    SetUiScale(f32),
    SetLineHeight(f32),
    SetCharacterWidth(f32),
    UpdateSize(Vec2, Vec2), // FIXME: should be a window action, not entire app
    DumpFlameGraph,
    PrintInfo,
    Quit,
}

#[derive(Clone, Debug, PartialEq)]
pub enum WindowAction {
    SaveFile,           // FIXME: move to buffer actions
    SaveFileAs(String), // FIXME: this isn't a _window_ action surely?
    FocusPane(Direction),
    ToggleFullscreen,
    SplitVertically,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PaneAction {
    UpdateSize(Vec2, Vec2),
    MouseScroll(MouseMove),
    MouseClick(Vec2),
    PrintDebugInfo,
}

#[derive(Clone, Debug, PartialEq)]
pub enum BufferAction {
    InsertNewlineAndReturn,
    InsertChar(char),
    InsertTypedChar,
    DeleteChar(Direction),
    CloneCursor,
    MoveCursor(MoveCursor),
    SetFilename(String),
    SetFiletype(String),
    StartSearch,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    OnGui(GuiAction),
    OnWindow(WindowAction),
    OnPane(PaneAction),
    OnBuffer(BufferAction),
}
