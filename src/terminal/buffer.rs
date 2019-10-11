use crate::buffer::Buffer;
use crate::terminal::row::TerminalRow;

pub trait TerminalBuffer {
    fn row_onscreen_text(&self, line_num: usize, offset: usize, cols: usize) -> Option<String>;
}

impl TerminalBuffer for Buffer<'_> {
    fn row_onscreen_text(&self, line_num: usize, offset: usize, cols: usize) -> Option<String> {
        self.rows
            .get(line_num)
            .map(|row| row.onscreen_text(offset, cols))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::buffer::Buffer;
    use crate::commands::SearchDirection;

    #[test]
    fn test_search_match_highlighting() {
        let mut buffer = Buffer::default();
        buffer.append_row("nothing abc123 nothing\r\n");
        let match_coords = buffer
            .search_for(None, SearchDirection::Forwards, "abc123")
            .unwrap();
        let row_idx = match_coords.1;
        let row = &buffer.rows[row_idx];
        let onscreen = row.onscreen_text(0, 22);
        assert!(onscreen.contains("\x1b[34mabc123\x1b[39m"));
    }

    #[test]
    fn test_clearing_search_overlay_from_onscreen_text() {
        let mut buffer = Buffer::default();
        buffer.append_row("nothing abc123 nothing\r\n");
        let (_, row_idx) = buffer
            .search_for(None, SearchDirection::Forwards, "abc123")
            .unwrap();
        buffer.clear_search_overlay();
        let row = &buffer.rows[row_idx];
        let onscreen = row.onscreen_text(0, 22);
        assert!(!onscreen.contains("\x1b[34m"));
    }
}
