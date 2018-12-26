use std::fmt;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum SearchDirection {
    Forwards,
    Backwards,
}

impl fmt::Display for SearchDirection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            &SearchDirection::Forwards => write!(f, "Forwards"),
            &SearchDirection::Backwards => write!(f, "Backwards"),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum MoveUnit {
    Rows,
    Pages,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct MoveCursor {
    pub direction: Direction,
    pub unit: MoveUnit,
    pub amount: usize,
}

impl MoveCursor {
    pub fn left(amount: usize) -> Self {
        MoveCursor {
            direction: Direction::Left,
            unit: MoveUnit::Rows,
            amount,
        }
    }

    pub fn right(amount: usize) -> Self {
        MoveCursor {
            direction: Direction::Right,
            unit: MoveUnit::Rows,
            amount,
        }
    }

    pub fn up(amount: usize) -> Self {
        MoveCursor {
            direction: Direction::Up,
            unit: MoveUnit::Rows,
            amount,
        }
    }

    pub fn down(amount: usize) -> Self {
        MoveCursor {
            direction: Direction::Down,
            unit: MoveUnit::Rows,
            amount,
        }
    }

    pub fn page_up(amount: usize) -> Self {
        MoveCursor {
            direction: Direction::Up,
            unit: MoveUnit::Pages,
            amount,
        }
    }

    pub fn page_down(amount: usize) -> Self {
        MoveCursor {
            direction: Direction::Down,
            unit: MoveUnit::Pages,
            amount,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Cmd {
    Move(MoveCursor),
    JumpCursorX(usize),
    JumpCursorY(usize),
    DeleteCharBackward,
    DeleteCharForward,
    InsertNewline(usize, usize),
    Linebreak(usize, usize),
    Quit,
    Save,
    InsertChar(char),
    Search,
}
