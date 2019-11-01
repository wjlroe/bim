use crate::action::{BufferAction, PaneAction, WindowAction};
use crate::buffer::{Buffer, FileSaveStatus};
use crate::commands::Direction;
use crate::pane::Pane;
use std::error::Error;

pub enum Arrangement {
    VSplit,
}

impl Default for Arrangement {
    fn default() -> Self {
        Self::VSplit
    }
}

pub trait Container<'a> {
    type PaneType: Pane<'a>;
    fn get_panes(&self) -> &Vec<Self::PaneType>;
    fn get_panes_mut(&mut self) -> &mut Vec<Self::PaneType>;
    fn get_focused_idx(&self) -> usize;
    fn set_focused_idx(&mut self, idx: usize);
    fn push_pane(&mut self, pane: Self::PaneType);
    fn recalculate_layout(&mut self);
    fn new_pane(&self, buffer: Buffer<'a>, focused: bool) -> Self::PaneType;
    fn get_arrangement(&self) -> &Arrangement;

    fn get_pane(&self, idx: usize) -> Option<&Self::PaneType> {
        self.get_panes().get(idx)
    }

    fn get_pane_mut(&mut self, idx: usize) -> Option<&mut Self::PaneType> {
        self.get_panes_mut().get_mut(idx)
    }

    fn do_pane_action(&mut self, action: PaneAction) {
        if let Some(pane) = self.get_pane_mut(self.get_focused_idx()) {
            pane.do_action(action);
        }
    }

    fn update_current_buffer(&mut self, action: BufferAction) {
        if let Some(pane) = self.get_pane_mut(self.get_focused_idx()) {
            pane.update_buffer(action);
        }
    }

    fn split_vertically(&mut self, filename: Option<&str>) -> Result<(), Box<dyn Error>> {
        let mut buffer = Buffer::default();
        if let Some(filename) = filename {
            buffer.open(filename)?;
        }
        let new_pane = self.new_pane(buffer, false);
        self.push_pane(new_pane);
        self.recalculate_layout();
        Ok(())
    }

    fn check(&mut self) -> Vec<WindowAction> {
        let mut actions = vec![];

        for pane in self.get_panes_mut().iter_mut() {
            actions.append(&mut pane.check());
        }

        actions
    }

    fn focus_pane_index(&mut self, pane_idx: usize) {
        self.set_focused_idx(pane_idx);
        for (idx, pane) in self.get_panes_mut().iter_mut().enumerate() {
            pane.set_focused(idx == pane_idx);
        }
    }

    fn focus_pane(&mut self, direction: Direction) {
        match self.get_arrangement() {
            Arrangement::VSplit => {
                let num_panes = self.get_panes().len();
                if num_panes > 1 {
                    let movement = match direction {
                        Direction::Right => 1,
                        Direction::Left => -1,
                        _ => 0, // TODO: We have no idea how to do Up/Down
                    };
                    let new_pane_idx =
                        (self.get_focused_idx() as i32 + movement) % num_panes as i32;
                    self.focus_pane_index(new_pane_idx as usize);
                }
            }
        }
    }

    fn save_file(&mut self) -> Option<Result<FileSaveStatus, Box<dyn Error>>> {
        if let Some(pane) = self.get_pane_mut(self.get_focused_idx()) {
            Some(pane.save_file())
        } else {
            None
        }
    }

    fn is_dirty(&self) -> bool {
        self.get_panes()
            .iter()
            .fold(false, |dirty, pane| dirty || pane.is_dirty())
    }
}
