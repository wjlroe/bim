use crate::action::{GuiAction, PaneAction};
use crate::buffer::Buffer;
use crate::container::*;
use crate::gui::gl_renderer::GlRenderer;
use crate::gui::gui_pane::GuiPane;
use crate::mouse::MouseMove;
use crate::pane::Pane;
use crate::rect::RectBuilder;
use glam::{vec2, Vec2};
use std::error::Error;
use std::time::Duration;

const PANE_BORDER_BG: [f32; 3] = [0.0, 250.0 / 256.0, 0.0];

pub struct GuiContainer<'a> {
    focused_idx: usize,
    panes: Vec<GuiPane<'a>>,
    bounds: Vec2,
    position: Vec2,
    arrangement: Arrangement,
}

impl<'a> Default for GuiContainer<'a> {
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

impl<'a> Container<'a> for GuiContainer<'a> {
    type PaneType = GuiPane<'a>;

    fn get_panes(&self) -> &Vec<GuiPane<'a>> {
        &self.panes
    }

    fn get_panes_mut(&mut self) -> &mut Vec<GuiPane<'a>> {
        &mut self.panes
    }

    fn get_focused_idx(&self) -> usize {
        self.focused_idx
    }

    fn set_focused_idx(&mut self, idx: usize) {
        self.focused_idx = idx;
    }

    fn push_pane(&mut self, pane: GuiPane<'a>) {
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

    fn new_pane(&self, buffer: Buffer<'a>, focused: bool) -> GuiPane<'a> {
        if let Some(pane) = self.panes.get(self.focused_idx) {
            GuiPane::new(pane.font_size, pane.ui_scale, buffer, focused)
        } else {
            // FIXME: Where to get the default font_size and ui_scale from?
            GuiPane::new(12.0, 1.0, buffer, focused)
        }
    }

    fn get_arrangement(&self) -> &Arrangement {
        &self.arrangement
    }
}

impl<'a> GuiContainer<'a> {
    pub fn single(bounds: Vec2, position: Vec2, pane: GuiPane<'a>) -> Self {
        Self {
            bounds,
            position,
            panes: vec![pane],
            ..GuiContainer::default()
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

    pub fn render(&self, renderer: &mut GlRenderer) -> Result<(), Box<dyn Error>> {
        match self.arrangement {
            Arrangement::VSplit => {
                if let Some(pane) = self.panes.get(self.focused_idx) {
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
}

#[test]
fn test_which_pane_is_location() {
    let buffer = Buffer::default();
    let bounds = vec2(10.0, 10.0);
    let position = vec2(0.0, 0.0);
    let gui_pane = GuiPane::new(12.0, 1.0, buffer, true);
    let mut container = GuiContainer::single(bounds, position, gui_pane);
    assert_eq!(Some(0), container.which_pane_is_location(vec2(0.0, 0.0)));
    let _ = container.split_vertically(None);
    assert_eq!(Some(0), container.which_pane_is_location(vec2(0.0, 0.0)));
    assert_eq!(Some(0), container.which_pane_is_location(vec2(0.0, 9.9)));
    assert_eq!(Some(1), container.which_pane_is_location(vec2(9.0, 0.0)));
    assert_eq!(Some(1), container.which_pane_is_location(vec2(5.0, 0.0)));
    assert_eq!(Some(1), container.which_pane_is_location(vec2(5.0, 9.9)));
}
