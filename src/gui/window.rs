use crate::buffer::{Buffer, BufferAction, FileSaveStatus};
use crate::commands::{self, Cmd, MoveCursor};
use crate::config::{RunConfig, BIM_QUIT_TIMES};
use crate::debug_log::DebugLog;
use crate::gui::actions::GuiAction;
use crate::gui::container::Container;
use crate::gui::transform_from_width_height;
// use crate::gui::draw_state::DrawState;
use crate::gui::gl_renderer::GlRenderer;
use crate::gui::keycode_to_char;
use crate::gui::persist_window_state::PersistWindowState;
use crate::gui::quad;
use crate::keycodes::Key;
use crate::options::Options;
use crate::status::Status;
use cgmath::{vec2, Matrix4, Vector2};
use flame;
use gfx::Device;
use gfx_glyph::{
    GlyphCruncher, HorizontalAlign, Layout, Scale, Section, SectionText, VariedSection,
    VerticalAlign,
};
use glutin::dpi::{LogicalPosition, LogicalSize};
use glutin::{
    ElementState, Event, MonitorId, MouseScrollDelta, PossiblyCurrent, WindowEvent, WindowedContext,
};
use std::error::Error;
use std::time::Duration;

pub enum WindowAction {
    SaveFileAs(String),
}

enum Action {
    ResizeWindow,
}

const POPUP_BG: [f32; 3] = [51.0 / 255.0, 0.0, 102.0 / 255.0];
const POPUP_OUTLINE: [f32; 3] = [240.0 / 255.0, 240.0 / 255.0, 240.0 / 255.0];

pub struct Window<'a> {
    monitor: MonitorId,
    window: WindowedContext<PossiblyCurrent>,
    window_dim: Vector2<f32>,
    logical_size: LogicalSize,
    mouse_position: Vector2<f32>,
    font_size: f32,
    ui_scale: f32,
    resized: bool,
    pub fullscreen: bool,
    container: Container<'a>,
    quit_times: i8,
    running: bool,
    pub in_focus: bool,
    pub status_message: Option<Status>,
    persist_window_state: PersistWindowState,
    debug_log: DebugLog<'a>,
    action_queue: Vec<Action>,
    options: Options,
}

impl<'a> Window<'a> {
    pub fn new(
        monitor: MonitorId,
        window: WindowedContext<PossiblyCurrent>,
        window_dim: Vector2<f32>,
        logical_size: LogicalSize,
        font_size: f32,
        ui_scale: f32,
        buffer: Buffer<'a>,
        persist_window_state: PersistWindowState,
        debug_log: DebugLog<'a>,
        options: Options,
    ) -> Result<Self, Box<dyn Error>> {
        let mut gui_window = Self {
            monitor,
            window,
            window_dim,
            logical_size,
            mouse_position: vec2(0.0, 0.0),
            ui_scale,
            font_size,
            resized: false,
            fullscreen: false,
            container: Container::single(window_dim, vec2(0.0, 0.0), font_size, ui_scale, buffer),
            quit_times: BIM_QUIT_TIMES + 1,
            running: true,
            in_focus: true,
            status_message: None,
            persist_window_state,
            debug_log,
            action_queue: vec![],
            options,
        };
        gui_window.open_files()?;
        Ok(gui_window)
    }

    fn open_files(&mut self) -> Result<(), Box<dyn Error>> {
        let mut files = Vec::new();
        if let RunConfig::RunOpenFiles(ref filenames) = self.options.run_type {
            if filenames.len() > 1 {
                for filename in &filenames[1..] {
                    files.push(String::from(filename));
                }
            }
        }
        for file in files {
            self.split_vertically_with_filename(&file)?;
        }
        Ok(())
    }

    fn handle_actions(&mut self, renderer: &mut GlRenderer) {
        // TODO: Vec::pop() - can we handle only the latest ResizeWindow action and discard the rest?
        while let Some(action) = self.action_queue.pop() {
            match action {
                Action::ResizeWindow => {
                    let physical_size = self.logical_size.to_physical(self.ui_scale.into());
                    let _ = self
                        .debug_log
                        .debugln_timestamped(&format!("physical_size: {:?}", physical_size,));
                    self.window.resize(physical_size);
                    gfx_window_glutin::update_views(
                        &self.window,
                        &mut renderer.quad_bundle.data.out_color,
                        &mut renderer.quad_bundle.data.out_depth,
                    );
                    {
                        let (width, height, ..) =
                            renderer.quad_bundle.data.out_color.get_dimensions();
                        self.set_window_dimensions((width, height));
                    }
                }
            }
        }
    }

    fn recalc_glyph_sizes(&mut self, renderer: &mut GlRenderer<'a>) {
        if self.has_resized() {
            let _guard = flame::start_guard("recalc_glyph_sized");

            let test_section = VariedSection {
                bounds: self.window_dim.into(),
                screen_position: (0.0, 0.0),
                text: vec![SectionText {
                    text: "AB\nC\n",
                    scale: Scale::uniform(self.font_scale()),
                    ..SectionText::default()
                }],
                ..VariedSection::default()
            };

            flame::start("glyphs");
            let test_glyphs = renderer.glyph_brush.glyphs(test_section);
            flame::end("glyphs");
            flame::start("glyphs.position()");
            let positions = test_glyphs
                .map(|glyph| glyph.position())
                .collect::<Vec<_>>();
            flame::end("glyphs.position()");
            let letter_a = positions[0];
            let letter_b = positions[1];
            let letter_c = positions[2];

            let first_line_min_y = letter_a.y;
            let secon_line_min_y = letter_c.y;
            let line_height = secon_line_min_y - first_line_min_y;
            self.set_line_height(line_height);

            let a_pos_x = letter_a.x;
            let b_pos_x = letter_b.x;
            let character_width = b_pos_x - a_pos_x;
            self.set_character_width(character_width);
        }
    }

    pub fn split_vertically_with_filename(&mut self, filename: &str) -> Result<(), Box<dyn Error>> {
        self.container.split_vertically(Some(filename))
    }

    pub fn update(
        &mut self,
        renderer: &mut GlRenderer<'a>,
        event: Event,
    ) -> Result<(), Box<dyn Error>> {
        match event {
            Event::WindowEvent { event, .. } => {
                match event {
                    WindowEvent::CursorMoved { position, .. } => {
                        self.update_mouse_position(position.into())
                    }
                    WindowEvent::MouseInput {
                        state: ElementState::Pressed,
                        ..
                    } => self.mouse_click(),
                    WindowEvent::MouseWheel {
                        delta: MouseScrollDelta::LineDelta(delta_x, delta_y),
                        ..
                    } => self.mouse_scroll(delta_x, delta_y),
                    WindowEvent::CloseRequested | WindowEvent::Destroyed => self.running = false,
                    WindowEvent::KeyboardInput {
                        input: keyboard_input,
                        ..
                    } => {
                        // TODO: partial shortcut recognition: <Ctrl-w> + <l> for move to the pane
                        // on the right...
                        if let Some(key) =
                            keycode_to_char::keyboard_event_to_keycode(keyboard_input)
                        {
                            self.handle_key(key);
                            match key {
                                Key::Control(Some('p')) => flame::dump_html(
                                    &mut std::fs::File::create("flame-graph.html").unwrap(),
                                )
                                .unwrap_or(()),
                                Key::Control(Some('-')) => self.dec_font_size(),
                                Key::Control(Some('+')) => self.inc_font_size(),
                                Key::Function(11) => {
                                    // FIXME: does this mean we will fullscreen on the monitor we
                                    // started on rather than one we move to? We don't reassign the
                                    // monitor variable
                                    let monitor = self.monitor.clone();
                                    self.toggle_fullscreen(monitor)
                                }
                                _ => {}
                            }
                        }
                    }
                    WindowEvent::Resized(new_logical_size) => {
                        self.resize(new_logical_size);
                        self.action_queue.push(Action::ResizeWindow);
                    }
                    WindowEvent::HiDpiFactorChanged(new_dpi) => {
                        let _ = self
                            .debug_log
                            .debugln_timestamped(&format!("new DPI: {}", new_dpi));
                        self.set_ui_scale(new_dpi as f32);
                        self.action_queue.push(Action::ResizeWindow);
                    }
                    WindowEvent::Moved(new_logical_position) => {
                        if let Some(monitor_name) =
                            self.window.window().get_current_monitor().get_name()
                        {
                            self.persist_window_state.monitor_name = Some(monitor_name);
                        }
                        self.persist_window_state.logical_position = new_logical_position;
                        self.persist_window_state.save();
                    }
                    WindowEvent::Focused(in_focus) => self.in_focus = in_focus,
                    _ => (),
                };
            }
            _ => (),
        };

        self.handle_actions(renderer);
        self.recalc_glyph_sizes(renderer);

        Ok(())
    }

    pub fn render(&mut self, renderer: &mut GlRenderer<'a>) -> Result<(), Box<dyn Error>> {
        // Purple background
        let background = [0.16078, 0.16471, 0.26667, 1.0];
        renderer
            .encoder
            .clear(&renderer.quad_bundle.data.out_color, background);
        renderer
            .encoder
            .clear_depth(&renderer.quad_bundle.data.out_depth, 1.0);

        {
            let _guard = flame::start_guard("render buffer");
            self.container.render(renderer)?;
        }

        if let Some(status_msg) = &self.status_message {
            let _guard = flame::start_guard("render popup text");

            let layout = Layout::default()
                .h_align(HorizontalAlign::Center)
                .v_align(VerticalAlign::Center);
            let popup_bounds: Vector2<f32> = self.window_dim - vec2(20.0, 20.0);
            let popup_section = Section {
                bounds: popup_bounds.into(),
                screen_position: (self.window_dim.x / 2.0, self.window_dim.y / 2.0),
                text: &status_msg.message,
                color: [224.0 / 255.0, 224.0 / 255.0, 224.0 / 255.0, 1.0],
                scale: Scale::uniform(self.font_scale() * 2.0),
                z: 0.5,
                layout,
                ..Section::default()
            };

            if let Some(msg_bounds) = renderer.glyph_brush.pixel_bounds(popup_section) {
                let width = msg_bounds.max.x - msg_bounds.min.x;
                let height = msg_bounds.max.y - msg_bounds.min.y;
                let shape = vec2(width as f32, height as f32);
                let text_size_transform = transform_from_width_height(shape, self.window_dim);

                let popup_bg_transform = Matrix4::from_scale(1.1) * text_size_transform;
                quad::draw(
                    &mut renderer.encoder,
                    &mut renderer.quad_bundle,
                    POPUP_BG,
                    popup_bg_transform,
                );

                let bg_transform = Matrix4::from_scale(1.1) * popup_bg_transform;
                quad::draw(
                    &mut renderer.encoder,
                    &mut renderer.quad_bundle,
                    POPUP_OUTLINE,
                    bg_transform,
                );
            }

            renderer.glyph_brush.queue(popup_section);
            renderer
                .glyph_brush
                .use_queue()
                .depth_target(&renderer.quad_bundle.data.out_depth)
                .draw(&mut renderer.encoder, &renderer.quad_bundle.data.out_color)?;
        }

        flame::start("encoder.flush");
        renderer.encoder.flush(&mut renderer.device);
        flame::end("encoder.flush");
        flame::start("swap_buffers");
        self.window.swap_buffers()?;
        flame::end("swap_buffers");
        flame::start("device.cleanup");
        renderer.device.cleanup();
        flame::end("device.cleanup");

        Ok(())
    }

    #[cfg(feature = "event-callbacks")]
    pub fn update_and_render(
        &mut self,
        renderer: &mut GlRenderer<'a>,
        event: Event,
    ) -> Result<bool, Box<dyn Error>> {
        self.start_frame();

        self.update(renderer, event)?;

        self.render(renderer)?;

        self.end_frame();

        Ok(self.keep_running())
    }

    pub fn has_resized(&self) -> bool {
        self.resized
    }

    fn should_quit(&self) -> bool {
        self.quit_times <= 0
    }

    pub fn keep_running(&self) -> bool {
        self.running && !self.should_quit()
    }

    pub fn start_frame(&mut self) {
        flame::start("frame");
        self.resized = false;
    }

    pub fn end_frame(&mut self) {
        flame::end_collapse("frame");
    }

    pub fn toggle_fullscreen(&mut self, monitor: MonitorId) {
        if self.fullscreen {
            self.window.window().set_fullscreen(None);
            self.fullscreen = false;
            self.resized = true;
        } else {
            self.window.window().set_fullscreen(Some(monitor));
            self.fullscreen = true;
            self.resized = true;
        }
    }

    fn font_scale(&self) -> f32 {
        self.ui_scale * self.font_size
    }

    pub fn update_mouse_position(&mut self, mouse: (f64, f64)) {
        self.mouse_position = vec2(mouse.0 as f32, mouse.1 as f32);
    }

    pub fn mouse_click(&mut self) {
        let mouse_pos = (self.mouse_position.x as f64, self.mouse_position.y as f64);
        let real_position = LogicalPosition::from(mouse_pos).to_physical(self.ui_scale.into());
        let real_position_vec = vec2(real_position.x as f32, real_position.y as f32);
        self.container.mouse_click(real_position_vec);
    }

    pub fn mouse_scroll(&mut self, delta_x: f32, delta_y: f32) {
        // FIXME: this is going to have to be relayed to the pane _under_ the mouse cursor position
        self.container
            .update_current_buffer(BufferAction::MouseScroll(vec2(-delta_x, -delta_y)));
    }

    pub fn inc_font_size(&mut self) {
        self.font_size += 1.0;
        self.resized = true;
        self.container
            .update_gui(GuiAction::SetFontSize(self.font_size));
    }

    pub fn dec_font_size(&mut self) {
        self.font_size -= 1.0;
        self.resized = true;
        self.container
            .update_gui(GuiAction::SetFontSize(self.font_size));
    }

    fn print_info(&mut self) {
        self.container
            .update_current_buffer(BufferAction::PrintDebugInfo);
    }

    pub fn handle_key(&mut self, key: Key) {
        let (handled, window_action) = self.container.handle_key(key);

        if let Some(window_action) = window_action {
            self.do_window_action(window_action);
        }

        if !handled {
            self.handle_buffer_key(key);
        }
    }

    fn do_window_action(&mut self, window_action: WindowAction) {
        match window_action {
            WindowAction::SaveFileAs(filename) => self.save_file_as(filename),
        }
    }

    fn handle_buffer_key(&mut self, key: Key) {
        let buffer_cmd = match key {
            Key::ArrowLeft => Some(Cmd::Move(MoveCursor::left(1))),
            Key::ArrowRight => Some(Cmd::Move(MoveCursor::right(1))),
            Key::ArrowUp => Some(Cmd::Move(MoveCursor::up(1))),
            Key::ArrowDown => Some(Cmd::Move(MoveCursor::down(1))),
            Key::PageDown => Some(Cmd::Move(MoveCursor::page_down(1))),
            Key::PageUp => Some(Cmd::Move(MoveCursor::page_up(1))),
            Key::Home => Some(Cmd::Move(MoveCursor::home())),
            Key::End => Some(Cmd::Move(MoveCursor::end())),
            Key::Delete => Some(Cmd::DeleteCharForward),
            Key::Backspace => Some(Cmd::DeleteCharBackward),
            Key::Return => Some(Cmd::Linebreak),
            Key::Other(typed_char) => Some(Cmd::InsertChar(typed_char)),
            Key::Function(fn_key) => {
                println!("Unrecognised key: F{}", fn_key);
                None
            }
            Key::Control(Some('-')) => None,
            Key::Control(Some('+')) => None,
            Key::Control(Some(' ')) => Some(Cmd::CloneCursor),
            Key::Control(Some('m')) => Some(Cmd::PrintInfo),
            Key::Control(Some('f')) => Some(Cmd::Search),
            Key::Control(Some('q')) => Some(Cmd::Quit),
            Key::Control(Some('s')) => Some(Cmd::Save),
            Key::Control(Some(ctrl_char)) => {
                println!("Unrecognised keypress: Ctrl-{}", ctrl_char);
                None
            }
            Key::Control(None) => None,
            Key::Escape => None,
        };
        if let Some(cmd) = buffer_cmd {
            self.handle_buffer_cmd(cmd);
        }
    }

    fn handle_buffer_cmd(&mut self, cmd: Cmd) {
        match cmd {
            Cmd::Move(movement) => self
                .container
                .update_current_buffer(BufferAction::MoveCursor(movement)),
            Cmd::DeleteCharBackward => self.delete_char_backward(),
            Cmd::DeleteCharForward => self.delete_char_forward(),
            Cmd::Linebreak => self.insert_newline_and_return(),
            Cmd::InsertChar(typed_char) => self.insert_char(typed_char),
            Cmd::Search => self
                .container
                .update_current_buffer(BufferAction::StartSearch),
            Cmd::CloneCursor => self
                .container
                .update_current_buffer(BufferAction::CloneCursor),
            Cmd::Quit => self.try_quit(),
            Cmd::PrintInfo => self.print_info(),
            Cmd::Escape => {}
            Cmd::Save => self.save_file(),
        }
    }

    fn save_file_as(&mut self, filename: String) {
        self.container
            .update_current_buffer(BufferAction::SetFilename(filename));
        self.save_file();
    }

    fn save_file(&mut self) {
        if let Some(save_status) = self.container.save_file() {
            match save_status {
                Ok(FileSaveStatus::Saved(bytes_saved)) => {
                    self.set_status_msg(format!("{} bytes written to disk", bytes_saved))
                }
                Ok(_) => {}
                Err(err) => {
                    self.set_status_msg(format!("Can't save! Error: {}", err));
                }
            }
        }
    }

    fn try_quit(&mut self) {
        if self.options.show_quit_warning() && self.container.is_dirty() {
            self.quit_times -= 1;
            self.set_status_msg(format!(
                "{} {} {} {}",
                "WARNING! File has unsaved changes.",
                "Press Ctrl-Q",
                self.quit_times,
                "more times to quit"
            ));
        } else {
            self.quit_times = 0;
        }
    }

    fn set_status_msg(&mut self, msg: String) {
        self.status_message = Some(Status::new_with_timeout(msg, Duration::from_secs(5)));
    }

    fn delete_char_backward(&mut self) {
        self.container
            .update_current_buffer(BufferAction::DeleteChar);
    }

    fn delete_char_forward(&mut self) {
        // FIXME: move into DrawState
        self.container
            .update_current_buffer(BufferAction::MoveCursor(commands::MoveCursor::right(1)));
        self.container
            .update_current_buffer(BufferAction::DeleteChar);
    }

    fn insert_newline_and_return(&mut self) {
        self.container
            .update_current_buffer(BufferAction::InsertNewlineAndReturn);
    }

    fn insert_char(&mut self, typed_char: char) {
        self.container
            .update_current_buffer(BufferAction::InsertChar(typed_char));
    }

    pub fn resize(&mut self, logical_size: LogicalSize) {
        self.logical_size = logical_size;
    }

    pub fn set_window_dimensions(&mut self, dimensions: (u16, u16)) {
        self.window_dim = vec2(dimensions.0.into(), dimensions.1.into());
        self.resized = true;
        self.container
            .update_gui(GuiAction::UpdateSize(self.window_dim, vec2(0.0, 0.0)));
    }

    pub fn set_ui_scale(&mut self, dpi: f32) {
        println!("DPI changed: {}", dpi);
        // FIXME: why do we need dpi AND ui_scale?
        self.ui_scale = dpi;
        self.container.update_gui(GuiAction::SetUiScale(dpi));
    }

    pub fn set_line_height(&mut self, line_height: f32) {
        self.container
            .update_gui(GuiAction::SetLineHeight(line_height));
    }

    pub fn set_character_width(&mut self, character_width: f32) {
        self.container
            .update_gui(GuiAction::SetCharacterWidth(character_width));
    }
}
