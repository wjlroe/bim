use crate::action::{BufferAction, PaneAction, WindowAction};
use crate::buffer::{Buffer, FileSaveStatus};
use crate::commands::{Direction, MoveCursor};
use crate::cursor::{Cursor, CursorT};
use crate::highlight::Highlight;
use crate::highlight::HighlightedSection;
use crate::input::Input;
use crate::mouse::MouseMove;
use crate::prompt::PromptAction;
use crate::search::Search;
use crate::status_line::StatusLine;
use std::error::Error;

pub trait Pane<'a> {
    type Output;
    fn get_buffer(&self) -> &Buffer<'a>;
    fn get_buffer_mut(&mut self) -> &mut Buffer<'a>;
    fn get_screen_rows(&self) -> i32;
    fn get_row_offset_int(&self) -> i32;
    fn get_search(&self) -> Option<&Search>;
    fn get_search_mut(&mut self) -> Option<&mut Search>;
    fn set_search(&mut self, search: Option<Search>);
    fn get_prompt(&self) -> Option<&Input<'a>>;
    fn get_prompt_mut(&mut self) -> Option<&mut Input<'a>>;
    fn set_prompt(&mut self, prompt: Option<Input<'a>>);
    fn get_other_cursor(&self) -> Option<&Cursor>;
    fn get_other_cursor_mut(&mut self) -> Option<&mut Cursor>;
    fn set_other_cursor(&mut self, cursor: Option<Cursor>);
    fn do_action(&mut self, action: PaneAction);
    fn scroll(&mut self);
    fn print_info(&self);
    fn get_status_line(&self) -> &StatusLine;
    fn update_status_line(&mut self);
    fn set_highlighted_sections(&mut self, highlighted_sections: Vec<HighlightedSection>);
    fn update_cursor(&mut self);
    fn mouse_scroll(&mut self, delta: MouseMove);
    fn get_focused(&self) -> bool;
    fn set_focused(&mut self, focused: bool);
    fn new_search(&self) -> Search;
    fn restore_from_search(&mut self, search: Search);

    fn is_dirty(&self) -> bool {
        self.get_buffer().is_dirty()
    }

    fn update(&mut self) {
        self.update_highlighted_sections();
        self.update_status_line();
    }

    fn cursor(&self) -> (usize, usize) {
        (
            self.get_buffer().cursor.text_row() as usize,
            self.get_buffer().cursor.text_col() as usize,
        )
    }

    fn update_buffer(&mut self, action: BufferAction) {
        use BufferAction::*;

        match action {
            InsertNewlineAndReturn => self.insert_newline_and_return(),
            InsertChar(typed_char) => self.insert_char(typed_char),
            DeleteChar(direction) => self.delete_char(direction),
            CloneCursor => self.clone_cursor(),
            MoveCursor(movement) => self.move_cursor(movement),
            SetFilename(filename) => self.get_buffer_mut().set_filename(filename),
            SetFiletype(filetype) => self.get_buffer_mut().set_filetype(&filetype),
            StartSearch => self.start_search(),
            InsertTypedChar => {
                panic!("Insert typed char received in DrawState.update_buffer, this should not happen!");
            }
        }
    }

    fn update_highlighted_sections(&mut self) {
        let mut highlighted_sections = Vec::new();
        for (row_idx, row) in self.get_buffer().rows.iter().enumerate() {
            // We don't want to push a 0->0 Normal highlight at the beginning of every line
            let mut first_char_seen = false;
            let mut current_section = HighlightedSection::default();
            current_section.text_row = row_idx;
            let mut overlay = row.overlay.iter();

            for (col_idx, hl) in row.hl.iter().enumerate() {
                let char_overlay: Option<Highlight> =
                    overlay.next().cloned().unwrap_or_else(|| None);
                let overlay_or_hl = char_overlay.unwrap_or_else(|| *hl);
                if current_section.highlight == overlay_or_hl {
                    current_section.last_col_idx = col_idx;
                } else {
                    if first_char_seen {
                        highlighted_sections.push(current_section);
                    }
                    current_section.highlight = overlay_or_hl;
                    current_section.first_col_idx = col_idx;
                    current_section.last_col_idx = col_idx;
                }
                first_char_seen = true;
            }

            if first_char_seen {
                highlighted_sections.push(current_section);
            }
        }
        self.set_highlighted_sections(highlighted_sections);
    }

    fn reset_cursor_col(&mut self, to: i32) {
        self.get_buffer_mut()
            .cursor
            .change(|cursor| cursor.text_col = to);
        self.update_cursor();
    }

    fn move_cursor_to_end_of_line(&mut self) {
        if self.get_buffer().cursor.text_row() < self.get_buffer().num_lines() as i32 {
            self.reset_cursor_col(
                self.get_buffer()
                    .line_len(self.get_buffer().cursor.text_row())
                    .unwrap_or(0) as i32,
            );
        }
    }

    fn move_cursor(&mut self, movement: MoveCursor) {
        let screen_rows = self.get_screen_rows() as usize;
        self.get_buffer_mut().move_cursor(movement, screen_rows);
        self.update_cursor();
    }

    fn move_cursor_onscreen(&mut self) {
        let row_offset = self.get_row_offset_int();
        self.get_buffer_mut().cursor.change(|cursor| {
            cursor.text_row = row_offset;
        });
    }

    fn clone_cursor(&mut self) {
        self.set_other_cursor(Some(self.get_buffer().cursor.current()));
        self.update_cursor();
    }

    fn delete_char(&mut self, direction: Direction) {
        if direction == Direction::Right {
            self.update_buffer(BufferAction::MoveCursor(MoveCursor::right(1)));
        }
        self.get_buffer_mut().delete_char_at_cursor();
        self.mark_buffer_changed();
        self.update_cursor();
    }

    fn insert_newline_and_return(&mut self) {
        if let Some(prompt) = self.get_prompt_mut() {
            prompt.done();
            return;
        }
        if let Some(search) = self.get_search_mut() {
            search.stop(false);
            return;
        }
        self.get_buffer_mut().insert_newline_and_return();
        self.mark_buffer_changed();
        self.update_cursor();
    }

    fn insert_char(&mut self, typed_char: char) {
        if let Some(prompt) = self.get_prompt_mut() {
            prompt.type_char(typed_char);
            return;
        }
        if let Some(search) = self.get_search_mut() {
            search.push_char(typed_char);
            return;
        }

        self.get_buffer_mut().insert_char_at_cursor(typed_char);
        self.mark_buffer_changed();
        self.update_cursor();
    }

    fn run_search(&mut self) {
        let mut update_search = false;

        if let Some(search) = self.get_search().cloned() {
            let last_match = self.get_buffer_mut().search_for(
                search.last_match(),
                search.direction(),
                search.needle(),
            );
            self.get_search_mut()
                .map(|search| search.set_last_match(last_match));
            update_search = true;
        }

        if update_search {
            self.update_search();
        }
    }

    fn update_search(&mut self) {
        self.update_cursor();
        self.update_highlighted_sections();
    }

    fn start_search(&mut self) {
        self.set_search(Some(self.new_search()));
        self.get_buffer_mut().cursor.save_cursor();
        self.update_search();
    }

    fn stop_search(&mut self) {
        self.set_search(None);
        self.get_buffer_mut().clear_search_overlay();
        self.update_highlighted_sections();
        self.update_cursor();
    }

    fn mark_buffer_changed(&mut self) {
        self.update_highlighted_sections();
    }

    fn status_text(&self) -> String {
        format!(
            "{} | {} | {}",
            self.get_status_line().filename,
            self.get_status_line().filetype,
            self.get_status_line().cursor
        )
    }

    fn start_prompt(&mut self, prompt: Input<'a>) {
        self.set_prompt(Some(prompt));
        self.get_buffer_mut().cursor.save_cursor();
        self.update_cursor();
    }

    fn top_prompt_visible(&self) -> bool {
        self.get_prompt().is_some() || self.get_search().is_some()
    }

    fn stop_prompt(&mut self) {
        self.set_prompt(None);
        self.get_buffer_mut().cursor.restore_saved();
        self.update_cursor();
    }

    fn check_prompt(&mut self) -> Option<WindowAction> {
        let mut window_action = None;
        let mut stop_prompt = false;

        if let Some(prompt) = self.get_prompt_mut() {
            if prompt.is_done() || prompt.is_cancelled() {
                stop_prompt = true;
            }
            if prompt.is_done() {
                match prompt.next_action() {
                    Some(PromptAction::SaveFile) => {
                        window_action =
                            Some(WindowAction::SaveFileAs(String::from(prompt.input())));
                    }
                    _ => {}
                }
            }
        }

        if stop_prompt {
            self.stop_prompt();
        }

        window_action
    }

    fn check_search(&mut self) {
        if let Some(search) = self.get_search().cloned() {
            if search.run_search() {
                self.run_search();
            } else {
                if search.restore_cursor() {
                    self.get_buffer_mut().cursor.restore_saved();
                    self.restore_from_search(search);
                }
                self.stop_search();
            }
        }
    }

    fn check(&mut self) -> Vec<WindowAction> {
        let mut actions = vec![];

        if let Some(window_action) = self.check_prompt() {
            actions.push(window_action);
        }
        self.check_search();

        actions
    }

    fn save_file(&mut self) -> Result<FileSaveStatus, Box<dyn Error>> {
        // FIXME: this has nothing to do with drawing/rendering, MOVE
        let file_save_status = self.get_buffer_mut().save_file()?;
        if file_save_status == FileSaveStatus::NoFilename {
            self.start_prompt(Input::new_save_file_input("Save file as", true));
        }
        Ok(file_save_status)
    }
}
