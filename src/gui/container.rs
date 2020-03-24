use crate::action::{BufferAction, GuiAction, PaneAction, WindowAction};
use crate::buffer::{Buffer, FileSaveStatus};
use crate::commands::Direction;
use crate::gui::gl_renderer::GlRenderer;
use crate::gui::pane::Pane;
use crate::mouse::MouseMove;
use crate::rect::RectBuilder;
use glam::{vec2, Vec2};
use std::error::Error;
use std::time::Duration;

const PANE_BORDER_BG: [f32; 3] = [0.0, 250.0 / 255.0, 0.0];

pub enum Arrangement {
    VSplit,
}

impl Default for Arrangement {
    fn default() -> Self {
        Self::VSplit
    }
}

pub struct Container<'a> {
    focused_idx: usize,
    panes: Vec<Pane<'a>>,
    bounds: Vec2,
    position: Vec2,
    arrangement: Arrangement,
}

impl<'a> Default for Container<'a> {
    fn default() -> Self {
        Self {
            focused_idx: 0,
            panes: Vec::new(),
            bounds: vec2(0.0, 0.0),
            position: vec2(0.0, 0.0),
            arrangement: Arrangement::default(),
        }
    }
}

impl<'a> Container<'a> {
    pub fn single(bounds: Vec2, position: Vec2, pane: Pane<'a>) -> Self {
        Self {
            bounds,
            position,
            panes: vec![pane],
            ..Container::default()
        }
    }

    fn set_focused_idx(&mut self, idx: usize) {
        self.focused_idx = idx;
    }

    fn push_pane(&mut self, pane: Pane<'a>) {
        self.panes.push(pane);
    }

    fn recalculate_layout(&mut self) {
        match self.arrangement {
            Arrangement::VSplit => {
                let each_width = self.bounds.x() / self.panes.len() as f32;
                let bounds = vec2(each_width, self.bounds.y());
                let mut position = vec2(self.position.x(), self.position.y());
                for pane in self.panes.iter_mut() {
                    pane.do_action(PaneAction::UpdateSize(bounds, position));
                    position += vec2(each_width, 0.0); // TODO: any padding?
                }
            }
        }
    }

    fn new_pane(&self, buffer: Buffer<'a>, focused: bool) -> Pane<'a> {
        if let Some(pane) = self.panes.get(self.focused_idx) {
            Pane::new(pane.font_size, pane.ui_scale, buffer, focused)
        } else {
            // FIXME: Where to get the default font_size and ui_scale from?
            Pane::new(12.0, 1.0, buffer, focused)
        }
    }

    pub fn update_gui(&mut self, action: GuiAction) {
        if let GuiAction::UpdateSize(bounds, position) = action {
            self.bounds = bounds;
            self.position = position;
            self.recalculate_layout();
        } else {
            for pane in self.panes.iter_mut() {
                pane.update_gui(action);
            }
        }
    }

    pub fn render(&self, renderer: &mut GlRenderer<'_>) -> Result<(), Box<dyn Error>> {
        match self.arrangement {
            Arrangement::VSplit => {
                if let Some(pane) = self.panes.get(0) {
                    let x_on_screen = pane.bounds.x();
                    let rect = RectBuilder::new()
                        .bounds(vec2(1.0, self.bounds.y()))
                        .top_left(vec2(x_on_screen, self.position.y()))
                        .build();
                    renderer.draw_quad(PANE_BORDER_BG, rect, 0.5);
                }
            }
        }

        for (pane_idx, pane) in self.panes.iter().enumerate() {
            pane.render(renderer, pane_idx == self.focused_idx)?;
        }

        Ok(())
    }

    fn which_pane_is_location(&self, location: Vec2) -> Option<usize> {
        match self.arrangement {
            Arrangement::VSplit => {
                // TODO: we assume even splits right now...
                let each_width = self.bounds.x() / self.panes.len() as f32;
                let which_pane = f32::floor(location.x() / each_width);
                Some(which_pane as usize)
            }
        }
    }

    pub fn mouse_scroll(&mut self, mouse_location: Vec2, delta: MouseMove) {
        if let Some(pane_idx) = self.which_pane_is_location(mouse_location) {
            if let Some(pane) = self.panes.get_mut(pane_idx) {
                pane.do_action(PaneAction::MouseScroll(delta));
            }
        }
    }

    fn absolute_position_to_pane_relative(&self, pane_idx: usize, location: Vec2) -> Vec2 {
        // TODO: Only works for horizontal layouts
        if pane_idx > 0 {
            let skip_x: f32 = self
                .panes
                .iter()
                .take(pane_idx)
                .map(|pane| pane.bounds.x())
                .sum();
            vec2(location.x() - skip_x, location.y())
        } else {
            location
        }
    }

    pub fn mouse_click(&mut self, location: Vec2) {
        if let Some(pane_idx) = self.which_pane_is_location(location) {
            self.focus_pane_index(pane_idx);
            let pane_location = self.absolute_position_to_pane_relative(pane_idx, location);
            println!(
                "abs location: {:?}, pane_local: {:?}",
                location, pane_location
            );
            if let Some(pane) = self.panes.get_mut(self.focused_idx) {
                pane.do_action(PaneAction::MouseClick(pane_location));
            }
        }
    }

    pub fn update_dt(&mut self, dt: Duration) {
        if let Some(pane) = self.panes.get_mut(self.focused_idx) {
            pane.update_dt(dt);
        }
    }

    pub fn do_pane_action(&mut self, action: PaneAction) {
        if let Some(pane) = self.panes.get_mut(self.focused_idx) {
            pane.do_action(action);
        }
    }

    pub fn update_current_buffer(&mut self, action: BufferAction) {
        if let Some(pane) = self.panes.get_mut(self.focused_idx) {
            pane.update_buffer(action);
        }
    }

    pub fn split_vertically(&mut self, filename: Option<&str>) -> Result<(), Box<dyn Error>> {
        let mut buffer = Buffer::default();
        if let Some(filename) = filename {
            buffer.open(filename)?;
        }
        let new_pane = self.new_pane(buffer, false);
        self.push_pane(new_pane);
        self.recalculate_layout();
        Ok(())
    }

    pub fn check(&mut self) -> Vec<WindowAction> {
        let mut actions = vec![];

        for pane in self.panes.iter_mut() {
            actions.append(&mut pane.check());
        }

        actions
    }

    fn focus_pane_index(&mut self, pane_idx: usize) {
        self.set_focused_idx(pane_idx);
        for (idx, pane) in self.panes.iter_mut().enumerate() {
            pane.set_focused(idx == pane_idx);
        }
    }

    pub fn focus_pane(&mut self, direction: Direction) {
        match self.arrangement {
            Arrangement::VSplit => {
                let num_panes = self.panes.len();
                if num_panes > 1 {
                    let movement = match direction {
                        Direction::Right => 1,
                        Direction::Left => -1,
                        _ => 0, // TODO: We have no idea how to do Up/Down
                    };
                    let new_pane_idx = (self.focused_idx as i32 + movement) % num_panes as i32;
                    self.focus_pane_index(new_pane_idx as usize);
                }
            }
        }
    }

    pub fn save_file(&mut self) -> Option<Result<FileSaveStatus, Box<dyn Error>>> {
        if let Some(pane) = self.panes.get_mut(self.focused_idx) {
            Some(pane.save_file())
        } else {
            None
        }
    }

    pub fn is_dirty(&self) -> bool {
        self.panes
            .iter()
            .fold(false, |dirty, pane| dirty || pane.is_dirty())
    }
}

#[test]
fn test_which_pane_is_location() {
    let buffer = Buffer::default();
    let bounds = vec2(10.0, 10.0);
    let position = vec2(0.0, 0.0);
    let gui_pane = Pane::new(12.0, 1.0, buffer, true);
    let mut container = Container::single(bounds, position, gui_pane);
    assert_eq!(Some(0), container.which_pane_is_location(vec2(0.0, 0.0)));
    let _ = container.split_vertically(None);
    assert_eq!(Some(0), container.which_pane_is_location(vec2(0.0, 0.0)));
    assert_eq!(Some(0), container.which_pane_is_location(vec2(0.0, 9.9)));
    assert_eq!(Some(1), container.which_pane_is_location(vec2(9.0, 0.0)));
    assert_eq!(Some(1), container.which_pane_is_location(vec2(5.0, 0.0)));
    assert_eq!(Some(1), container.which_pane_is_location(vec2(5.0, 9.9)));
}
