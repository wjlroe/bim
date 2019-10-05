use crate::action::{BufferAction, GuiAction, PaneAction, WindowAction};
use crate::buffer::{Buffer, FileSaveStatus};
use crate::commands::Direction;
use crate::gui::gl_renderer::GlRenderer;
use crate::gui::mouse::MouseMove;
use crate::gui::pane::Pane;
use crate::gui::rect::RectBuilder;
use crate::keycodes::Key;
use cgmath::{vec2, Vector2};
use std::error::Error;

const PANE_BORDER_BG: [f32; 3] = [0.0, 250.0 / 256.0, 0.0];

enum Arrangement {
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
    bounds: Vector2<f32>,
    position: Vector2<f32>,
    arrangement: Arrangement,
}

impl<'a> Container<'a> {
    pub fn single(
        bounds: Vector2<f32>,
        position: Vector2<f32>,
        font_size: f32,
        ui_scale: f32,
        buffer: Buffer<'a>,
    ) -> Self {
        Self {
            focused_idx: 0,
            bounds,
            position,
            panes: vec![Pane::new(font_size, ui_scale, buffer, true)],
            arrangement: Arrangement::default(),
        }
    }

    pub fn render(&self, renderer: &mut GlRenderer) -> Result<(), Box<dyn Error>> {
        match self.arrangement {
            Arrangement::VSplit => {
                if let Some(pane) = self.panes.get(self.focused_idx) {
                    let x_on_screen = pane.draw_state.bounds.x;
                    let rect = RectBuilder::new()
                        .bounds(vec2(1.0, self.bounds.y))
                        .top_left(vec2(x_on_screen, self.position.y))
                        .build();
                    renderer.draw_quad(PANE_BORDER_BG, rect, 0.5);
                }
            }
        }

        for pane in self.panes.iter() {
            pane.render(renderer)?;
        }

        Ok(())
    }

    pub fn do_pane_action(&mut self, action: PaneAction) {
        if let Some(pane) = self.panes.get_mut(self.focused_idx) {
            pane.do_action(action);
        }
    }

    pub fn update_gui(&mut self, action: GuiAction) {
        if let GuiAction::UpdateSize(bounds, position) = action {
            self.bounds = bounds;
            self.position = position;
            self.recalc_layout();
        } else {
            for pane in self.panes.iter_mut() {
                pane.update_gui(action);
            }
        }
    }

    pub fn update_current_buffer(&mut self, action: BufferAction) {
        if let Some(pane) = self.panes.get_mut(self.focused_idx) {
            pane.update_buffer(action);
        }
    }

    pub fn split_vertically(&mut self, filename: Option<&str>) -> Result<(), Box<dyn Error>> {
        let mut new_pane = None;
        let mut buffer = Buffer::default();
        if let Some(filename) = filename {
            buffer.open(filename)?;
        }
        if let Some(pane) = self.panes.get(self.focused_idx) {
            new_pane = Some(Pane::new(pane.font_size(), pane.ui_scale(), buffer, false));
        }
        if let Some(pane) = new_pane {
            self.panes.push(pane);
            self.recalc_layout();
        }
        Ok(())
    }

    fn recalc_layout(&mut self) {
        match self.arrangement {
            Arrangement::VSplit => {
                let each_width = self.bounds.x / self.panes.len() as f32;
                let bounds = vec2(each_width, self.bounds.y);
                let mut position = vec2(self.position.x, self.position.y);
                for pane in self.panes.iter_mut() {
                    pane.do_action(PaneAction::UpdateSize(bounds, position));
                    position.x += each_width; // TODO: any padding?
                }
            }
        }
    }

    pub fn handle_key(&mut self, key: Key) -> bool {
        let mut handled = false;

        if key == Key::Control(Some('v')) {
            if let Ok(_) = self.split_vertically(None) {
                handled = true;
            }
        }

        handled
    }

    pub fn check(&mut self) -> Vec<WindowAction> {
        let mut actions = vec![];

        for pane in self.panes.iter_mut() {
            actions.append(&mut pane.check());
        }

        actions
    }

    fn which_pane_is_location(&self, location: Vector2<f32>) -> Option<usize> {
        match self.arrangement {
            Arrangement::VSplit => {
                // TODO: we assume even splits right now...
                let each_width = self.bounds.x / self.panes.len() as f32;
                let which_pane = f32::floor(location.x / each_width);
                Some(which_pane as usize)
            }
        }
    }

    fn focus_pane_index(&mut self, pane_idx: usize) {
        self.focused_idx = pane_idx;
        for (idx, pane) in self.panes.iter_mut().enumerate() {
            pane.set_focused(idx == pane_idx);
        }
    }

    pub fn mouse_scroll(&mut self, mouse_location: Vector2<f32>, delta: MouseMove) {
        if let Some(pane_idx) = self.which_pane_is_location(mouse_location) {
            if let Some(pane) = self.panes.get_mut(pane_idx) {
                pane.update_buffer(BufferAction::MouseScroll(delta));
            }
        }
    }

    pub fn mouse_click(&mut self, location: Vector2<f32>) {
        if let Some(pane_idx) = self.which_pane_is_location(location) {
            self.focus_pane_index(pane_idx);
            if let Some(pane) = self.panes.get_mut(self.focused_idx) {
                pane.update_buffer(BufferAction::MouseClick(location));
            }
        }
    }

    pub fn focus_pane(&mut self, direction: Direction) {
        match self.arrangement {
            Arrangement::VSplit => {
                if self.panes.len() > 1 {
                    let movement = match direction {
                        Direction::Right => 1,
                        Direction::Left => -1,
                        _ => 0, // TODO: We have no idea how to do Up/Down
                    };
                    let new_pane_idx =
                        (self.focused_idx as i32 + movement) % self.panes.len() as i32;
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
    let mut container = Container::single(bounds, position, 12.0, 1.0, buffer);
    assert_eq!(Some(0), container.which_pane_is_location(vec2(0.0, 0.0)));
    let _ = container.split_vertically(None);
    assert_eq!(Some(0), container.which_pane_is_location(vec2(0.0, 0.0)));
    assert_eq!(Some(0), container.which_pane_is_location(vec2(0.0, 9.9)));
    assert_eq!(Some(1), container.which_pane_is_location(vec2(9.0, 0.0)));
    assert_eq!(Some(1), container.which_pane_is_location(vec2(5.0, 0.0)));
    assert_eq!(Some(1), container.which_pane_is_location(vec2(5.0, 9.9)));
}
