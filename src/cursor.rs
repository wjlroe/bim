pub trait CursorT {
    fn text_row(&self) -> i32;
    fn text_col(&self) -> i32;
}

#[derive(Copy, Clone, Debug, Default)]
pub struct Cursor {
    pub text_row: i32,
    pub text_col: i32,
    pub moved: bool,
}

impl PartialEq for Cursor {
    fn eq(&self, other: &Cursor) -> bool {
        self.text_row == other.text_row && self.text_col == other.text_col
    }
}

impl Cursor {
    pub fn new(row: i32, col: i32) -> Self {
        Self {
            text_row: row,
            text_col: col,
            moved: false,
        }
    }

    pub fn move_col(&mut self, amount: i32) {
        self.text_col += amount;
        self.moved = true;
    }

    pub fn reset_col_to(&mut self, to: i32) {
        self.text_col = to;
        self.moved = true;
    }

    pub fn move_row(&mut self, amount: i32) {
        self.text_row += amount;
        self.moved = true;
    }

    pub fn reset_row_to(&mut self, to: i32) {
        self.text_row = to;
        self.moved = true;
    }
}

impl CursorT for Cursor {
    fn text_col(&self) -> i32 {
        self.text_col
    }

    fn text_row(&self) -> i32 {
        self.text_row
    }
}

#[derive(Clone, Default, PartialEq)]
pub struct CursorWithHistory {
    current: Cursor,
    saved: Option<Cursor>,
    previous: Vec<Cursor>,
}

impl CursorWithHistory {
    pub fn move_to(&mut self, row: i32, col: i32) {
        self.previous.push(self.current);
        self.current = Cursor::new(row, col);
    }

    pub fn move_to_without_history(&mut self, row: i32, col: i32) {
        self.current.text_row = row;
        self.current.text_col = col;
    }

    pub fn change<F>(&mut self, func: F)
    where
        F: Fn(&mut Cursor),
    {
        let old_current = self.current;
        func(&mut self.current);
        if old_current != self.current {
            self.previous.push(old_current);
        }
    }

    pub fn save_cursor(&mut self) {
        self.saved = Some(self.current)
    }

    pub fn restore_saved(&mut self) {
        if let Some(cursor) = self.saved {
            self.current = cursor;
            self.saved = None;
        }
    }

    pub fn pop_previous(&mut self) {
        if let Some(cursor) = self.previous.pop() {
            self.current = cursor;
        }
    }

    pub fn current(&self) -> Cursor {
        self.current
    }
}

impl CursorT for CursorWithHistory {
    fn text_row(&self) -> i32 {
        self.current.text_row
    }

    fn text_col(&self) -> i32 {
        self.current.text_col
    }
}

#[test]
fn test_change_cursor() {
    let mut cursor_with_history = CursorWithHistory::default();
    cursor_with_history.change(|cursor| {
        cursor.text_row += 3;
        cursor.text_col += 10;
    });
    assert_eq!(3, cursor_with_history.text_row());
    assert_eq!(10, cursor_with_history.text_col());

    cursor_with_history.pop_previous();
    assert_eq!(0, cursor_with_history.text_row());
    assert_eq!(0, cursor_with_history.text_col());
}

#[test]
fn test_change_cursor_same_cursor() {
    let mut cursor_with_history = CursorWithHistory::default();
    cursor_with_history.change(|cursor| {
        cursor.text_row = 3;
        cursor.text_col = 10;
    });
    assert_eq!(1, cursor_with_history.previous.len());

    // Change to the same as it already is...
    cursor_with_history.change(|cursor| {
        cursor.text_row = 3;
        cursor.text_col = 10;
    });
    // Same number of previous cursors, no new ones pushed
    assert_eq!(1, cursor_with_history.previous.len());
}

#[test]
fn test_move_to_without_history() {
    let mut cursor_with_history = CursorWithHistory::default();
    cursor_with_history.change(|cursor| {
        cursor.text_row = 3;
        cursor.text_col = 10;
    });

    assert_eq!(Cursor::new(3, 10), cursor_with_history.current);

    cursor_with_history.change(|cursor| {
        cursor.text_row = 0;
        cursor.text_col = 0;
    });

    cursor_with_history.move_to_without_history(1, 0);
    cursor_with_history.move_to_without_history(2, 0);
    cursor_with_history.move_to_without_history(3, 0);
    cursor_with_history.move_to_without_history(4, 0);

    cursor_with_history.pop_previous();

    assert_eq!(Cursor::new(3, 10), cursor_with_history.current);
}
