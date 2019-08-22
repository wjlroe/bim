use crate::buffer::Buffer;
use crate::commands::{self, Cmd, MoveCursor};
use crate::config::BIM_QUIT_TIMES;
use crate::debug_log::DebugLog;
use crate::gui::draw_state::DrawState;
use crate::gui::keycode_to_char;
use crate::gui::persist_window_state::PersistWindowState;
use crate::gui::quad;
use crate::keycodes::Key;
use crate::prompt::Prompt;
use crate::status::Status;
use cgmath::{vec2, Matrix4, Vector2};
use flame;
use gfx::{pso, Device, Encoder};
use gfx_device_gl;
use gfx_glyph::{
    GlyphBrush, GlyphCruncher, HorizontalAlign, Layout, Scale, Section, SectionText, VariedSection,
    VerticalAlign,
};
use glutin::dpi::{LogicalPosition, LogicalSize};
use glutin::{
    ElementState, Event, MonitorId, MouseScrollDelta, PossiblyCurrent, WindowEvent, WindowedContext,
};
use std::error::Error;
use std::time::Duration;

enum Action {
    ResizeWindow,
    SaveFileAs(String),
}

const STATUS_BG: [f32; 3] = [215.0 / 256.0, 0.0, 135.0 / 256.0];
const CURSOR_BG: [f32; 3] = [250.0 / 256.0, 250.0 / 256.0, 250.0 / 256.0];
const OTHER_CURSOR_BG: [f32; 3] = [255.0 / 256.0, 165.0 / 256.0, 0.0];
const LINE_COL_BG: [f32; 3] = [0.0, 0.0, 0.0];
const POPUP_BG: [f32; 3] = [51.0 / 255.0, 0.0, 102.0 / 255.0];
const POPUP_OUTLINE: [f32; 3] = [240.0 / 255.0, 240.0 / 255.0, 240.0 / 255.0];

// Marker for what to do when the prompt comes back
enum PromptAction {
    SaveFile,
}

pub struct Window<'a> {
    monitor: MonitorId,
    window: WindowedContext<PossiblyCurrent>,
    logical_size: LogicalSize,
    dpi: f32,
    resized: bool,
    pub fullscreen: bool,
    draw_state: DrawState<'a>,
    prompt: Option<Prompt>,
    prompt_next_action: Option<PromptAction>,
    quit_times: i8,
    pub in_focus: bool,
    pub status_message: Option<Status>,
    persist_window_state: PersistWindowState,
    debug_log: DebugLog<'a>,
    glyph_brush: GlyphBrush<'a, gfx_device_gl::Resources, gfx_device_gl::Factory>,
    device: gfx_device_gl::Device,
    encoder: Encoder<gfx_device_gl::Resources, gfx_device_gl::CommandBuffer>,
    quad_bundle:
        pso::bundle::Bundle<gfx_device_gl::Resources, quad::pipe::Data<gfx_device_gl::Resources>>,
    action_queue: Vec<Action>,
}

impl<'a> Window<'a> {
    pub fn new(
        monitor: MonitorId,
        window: WindowedContext<PossiblyCurrent>,
        logical_size: LogicalSize,
        dpi: f32,
        window_width: f32,
        window_height: f32,
        font_size: f32,
        ui_scale: f32,
        buffer: Buffer<'a>,
        persist_window_state: PersistWindowState,
        debug_log: DebugLog<'a>,
        glyph_brush: GlyphBrush<'a, gfx_device_gl::Resources, gfx_device_gl::Factory>,
        device: gfx_device_gl::Device,
        encoder: Encoder<gfx_device_gl::Resources, gfx_device_gl::CommandBuffer>,
        quad_bundle: pso::bundle::Bundle<
            gfx_device_gl::Resources,
            quad::pipe::Data<gfx_device_gl::Resources>,
        >,
    ) -> Self {
        Self {
            monitor,
            window,
            logical_size,
            dpi,
            resized: false,
            fullscreen: false,
            draw_state: DrawState::new(window_width, window_height, font_size, ui_scale, buffer),
            prompt: None,
            prompt_next_action: None,
            quit_times: BIM_QUIT_TIMES + 1,
            in_focus: true,
            status_message: None,
            persist_window_state,
            debug_log,
            glyph_brush,
            device,
            encoder,
            quad_bundle,
            action_queue: vec![],
        }
    }

    fn handle_actions(&mut self) {
        while let Some(action) = self.action_queue.pop() {
            match action {
                Action::ResizeWindow => {
                    let physical_size = self.logical_size.to_physical(self.dpi.into());
                    let _ = self
                        .debug_log
                        .debugln_timestamped(&format!("physical_size: {:?}", physical_size,));
                    self.window.resize(physical_size);
                    gfx_window_glutin::update_views(
                        &self.window,
                        &mut self.quad_bundle.data.out_color,
                        &mut self.quad_bundle.data.out_depth,
                    );
                    {
                        let (width, height, ..) = self.quad_bundle.data.out_color.get_dimensions();
                        self.set_window_dimensions((width, height));
                    }
                }
                Action::SaveFileAs(filename) => {
                    self.save_file_as(filename);
                }
            }
        }
    }

    pub fn update_and_render(&mut self, event: Event) -> Result<bool, Box<dyn Error>> {
        let mut running = true;
        flame::start("frame");
        self.next_frame();
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
                    WindowEvent::CloseRequested | WindowEvent::Destroyed => running = false,
                    WindowEvent::KeyboardInput {
                        input: keyboard_input,
                        ..
                    } => {
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
                        self.set_dpi(new_dpi as f32);
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

        self.handle_actions();

        // Purple background
        let background = [0.16078, 0.16471, 0.26667, 1.0];
        self.encoder
            .clear(&self.quad_bundle.data.out_color, background);
        self.encoder
            .clear_depth(&self.quad_bundle.data.out_depth, 1.0);

        let window_dim: (f32, f32) = self.inner_dimensions().into();

        if self.has_resized() {
            let _guard = flame::start_guard("window_resized");

            let test_section = VariedSection {
                bounds: window_dim,
                screen_position: (self.left_padding(), self.top_padding()),
                text: vec![SectionText {
                    text: "AB\nC\n",
                    scale: Scale::uniform(self.font_scale()),
                    ..SectionText::default()
                }],
                ..VariedSection::default()
            };

            flame::start("glyphs");
            let test_glyphs = self.glyph_brush.glyphs(test_section);
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

        {
            let _guard = flame::start_guard("render cursor quad");
            let cursor_transform = self.cursor_transform();
            quad::draw(
                &mut self.encoder,
                &mut self.quad_bundle,
                CURSOR_BG,
                cursor_transform,
            );

            if let Some(cursor_transform) = self.other_cursor_transform() {
                quad::draw(
                    &mut self.encoder,
                    &mut self.quad_bundle,
                    OTHER_CURSOR_BG,
                    cursor_transform,
                );
            }
        }

        {
            let _guard = flame::start_guard("render section_texts");

            let section = VariedSection {
                bounds: window_dim,
                screen_position: (self.left_padding(), self.top_padding()),
                text: self.draw_state.section_texts(),
                z: 1.0,
                ..VariedSection::default()
            };
            self.glyph_brush.queue(section);

            let default_transform: Matrix4<f32> =
                gfx_glyph::default_transform(&self.quad_bundle.data.out_color).into();
            let transform = self.row_offset_as_transform() * default_transform;
            self.glyph_brush
                .use_queue()
                .transform(transform)
                .depth_target(&self.quad_bundle.data.out_depth)
                .draw(&mut self.encoder, &self.quad_bundle.data.out_color)?;
        }

        {
            let _guard = flame::start_guard("render lines");
            for transform in self.line_transforms() {
                quad::draw(
                    &mut self.encoder,
                    &mut self.quad_bundle,
                    LINE_COL_BG,
                    transform,
                );
            }
        }

        {
            let _guard = flame::start_guard("render status quad");
            // Render status background
            let status_transform = self.status_transform();
            quad::draw(
                &mut self.encoder,
                &mut self.quad_bundle,
                STATUS_BG,
                status_transform,
            );
        }

        {
            let _guard = flame::start_guard("render status text");

            let status_text = self.status_text();
            let status_section = Section {
                bounds: window_dim,
                screen_position: (self.left_padding(), window_dim.1 + self.top_padding()),
                text: &status_text,
                color: [1.0, 1.0, 1.0, 1.0],
                scale: Scale::uniform(self.font_scale()),
                z: 0.5,
                ..Section::default()
            };
            self.glyph_brush.queue(status_section);
            self.glyph_brush
                .use_queue()
                .depth_target(&self.quad_bundle.data.out_depth)
                .draw(&mut self.encoder, &self.quad_bundle.data.out_color)?;
        }

        if let Some(top_left_text) = self.prompt_top_left() {
            let _guard = flame::start_guard("render top left prompt text");

            let top_left_section = Section {
                bounds: window_dim,
                screen_position: (self.left_padding(), 0.0),
                text: &top_left_text,
                color: [0.7, 0.6, 0.5, 1.0],
                scale: Scale::uniform(self.font_scale()),
                z: 0.5,
                ..Section::default()
            };
            self.glyph_brush.queue(top_left_section);
            self.glyph_brush
                .use_queue()
                .depth_target(&self.quad_bundle.data.out_depth)
                .draw(&mut self.encoder, &self.quad_bundle.data.out_color)?;
        }

        if let Some(status_msg) = &self.status_message {
            let _guard = flame::start_guard("render popup text");

            let layout = Layout::default()
                .h_align(HorizontalAlign::Center)
                .v_align(VerticalAlign::Center);
            let popup_bounds: Vector2<f32> = self.inner_dimensions() - vec2(20.0, 20.0);
            let popup_section = Section {
                bounds: popup_bounds.into(),
                screen_position: (self.window_width() / 2.0, self.window_height() / 2.0),
                text: &status_msg.message,
                color: [224.0 / 255.0, 224.0 / 255.0, 224.0 / 255.0, 1.0],
                scale: Scale::uniform(self.font_scale() * 2.0),
                z: 0.5,
                layout,
                ..Section::default()
            };

            if let Some(msg_bounds) = self.glyph_brush.pixel_bounds(popup_section) {
                let width = msg_bounds.max.x - msg_bounds.min.x;
                let height = msg_bounds.max.y - msg_bounds.min.y;
                let text_size_transform =
                    self.transform_from_width_height(width as f32, height as f32);

                let popup_bg_transform = Matrix4::from_scale(1.1) * text_size_transform;
                quad::draw(
                    &mut self.encoder,
                    &mut self.quad_bundle,
                    POPUP_BG,
                    popup_bg_transform,
                );

                let bg_transform = Matrix4::from_scale(1.1) * popup_bg_transform;
                quad::draw(
                    &mut self.encoder,
                    &mut self.quad_bundle,
                    POPUP_OUTLINE,
                    bg_transform,
                );
            }

            self.glyph_brush.queue(popup_section);
            self.glyph_brush
                .use_queue()
                .depth_target(&self.quad_bundle.data.out_depth)
                .draw(&mut self.encoder, &self.quad_bundle.data.out_color)?;
        }

        flame::start("encoder.flush");
        self.encoder.flush(&mut self.device);
        flame::end("encoder.flush");
        flame::start("swap_buffers");
        self.window.swap_buffers()?;
        flame::end("swap_buffers");
        flame::start("device.cleanup");
        self.device.cleanup();
        flame::end("device.cleanup");

        flame::end_collapse("frame");

        let keep_running = running && !self.should_quit();
        Ok(keep_running)
    }

    pub fn has_resized(&self) -> bool {
        self.resized
    }

    pub fn should_quit(&self) -> bool {
        self.quit_times <= 0
    }

    pub fn next_frame(&mut self) {
        self.resized = false;
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

    pub fn inner_dimensions(&self) -> Vector2<f32> {
        (
            self.draw_state.inner_width(),
            self.draw_state.inner_height(),
        )
            .into()
    }

    pub fn window_height(&self) -> f32 {
        self.draw_state.window_height()
    }

    pub fn window_width(&self) -> f32 {
        self.draw_state.window_width()
    }

    pub fn font_scale(&self) -> f32 {
        self.draw_state.font_scale()
    }

    pub fn left_padding(&self) -> f32 {
        self.draw_state.left_padding()
    }

    pub fn top_padding(&self) -> f32 {
        self.draw_state.top_padding()
    }

    pub fn row_offset_as_transform(&self) -> Matrix4<f32> {
        self.draw_state.row_offset_as_transform()
    }

    pub fn cursor_transform(&self) -> Matrix4<f32> {
        self.draw_state.cursor_transform()
    }

    pub fn other_cursor_transform(&self) -> Option<Matrix4<f32>> {
        self.draw_state.other_cursor_transform()
    }

    pub fn line_transforms(&self) -> Vec<Matrix4<f32>> {
        self.draw_state.line_transforms()
    }

    pub fn status_transform(&self) -> Matrix4<f32> {
        self.draw_state.status_transform()
    }

    pub fn transform_from_width_height(&self, width: f32, height: f32) -> Matrix4<f32> {
        self.draw_state.transform_from_width_height(width, height)
    }

    pub fn status_text(&self) -> String {
        format!(
            "{} | {} | {}",
            self.draw_state.status_line.filename,
            self.draw_state.status_line.filetype,
            self.draw_state.status_line.cursor
        )
    }

    pub fn update_mouse_position(&mut self, mouse: (f64, f64)) {
        self.draw_state.mouse_position = mouse;
    }

    pub fn mouse_click(&mut self) {
        let real_position: (f64, f64) = LogicalPosition::from(self.draw_state.mouse_position)
            .to_physical(self.draw_state.ui_scale().into())
            .into();
        self.draw_state.move_cursor_to_mouse_position(real_position);
    }

    pub fn mouse_scroll(&mut self, delta_x: f32, delta_y: f32) {
        self.draw_state.scroll_window_vertically(-delta_y);
        self.draw_state.scroll_window_horizontally(-delta_x);
        self.draw_state.update_cursor();
    }

    pub fn inc_font_size(&mut self) {
        self.draw_state.inc_font_size();
        self.resized = true;
    }

    pub fn dec_font_size(&mut self) {
        self.draw_state.dec_font_size();
        self.resized = true;
    }

    fn print_info(&mut self) {
        self.draw_state.print_info();
    }

    pub fn handle_key(&mut self, key: Key) {
        let mut handled = false;
        if let Some(prompt) = &mut self.prompt {
            handled = prompt.handle_key(key);
            self.check_prompt();
        }

        if !handled {
            self.handle_buffer_key(key);
        }
    }

    fn check_prompt(&mut self) {
        if let Some(prompt) = &mut self.prompt {
            match prompt {
                Prompt::Search(search) => {
                    if search.run_search() {
                        let last_match = self.draw_state.search_for(
                            search.last_match(),
                            search.direction(),
                            search.needle(),
                        );
                        search.set_last_match(last_match);
                    } else {
                        if search.restore_cursor() {
                            self.draw_state.buffer.cursor.restore_saved();
                            self.draw_state.row_offset = search.saved_row_offset();
                            self.draw_state.col_offset = search.saved_col_offset();
                        }
                        self.prompt = None;
                        self.draw_state.top_prompt_visible = false;
                        self.draw_state.stop_search();
                    }
                }
                Prompt::Input(input_prompt) => {
                    if input_prompt.is_done() {
                        // TODO: handle what to do with this string...
                        println!("input is: {}", input_prompt.input);
                        match self.prompt_next_action {
                            Some(PromptAction::SaveFile) => {
                                self.action_queue
                                    .push(Action::SaveFileAs(input_prompt.input.clone()));
                                self.prompt = None;
                                self.prompt_next_action = None;
                                self.draw_state.top_prompt_visible = false;
                            }
                            _ => {}
                        }
                    }
                }
            }
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
            Cmd::Move(movement) => self.move_cursor(movement),
            Cmd::DeleteCharBackward => self.delete_char_backward(),
            Cmd::DeleteCharForward => self.delete_char_forward(),
            Cmd::Linebreak => self.insert_newline_and_return(),
            Cmd::InsertChar(typed_char) => self.insert_char(typed_char),
            Cmd::Search => self.start_search(),
            Cmd::CloneCursor => self.clone_cursor(),
            Cmd::Quit => self.try_quit(),
            Cmd::PrintInfo => self.print_info(),
            Cmd::Escape => {}
            Cmd::Save => self.save_file(),
        }
    }

    fn save_file_as(&mut self, filename: String) {
        self.draw_state.buffer.filename = Some(filename);
        self.save_file();
    }

    fn save_file(&mut self) {
        if self.draw_state.buffer.filename.is_none() {
            // prompt for filename
            // how do we know we're waiting on this?
            // TODO: cursor still showing where the prompt is...
            // TODO: show cursor after input next char
            self.prompt = Some(Prompt::new_input(String::from("Save file as")));
            self.prompt_next_action = Some(PromptAction::SaveFile);
            self.draw_state.top_prompt_visible = true;
        } else {
            match self.draw_state.buffer.save_to_file() {
                Ok(bytes_saved) => {
                    self.set_status_msg(format!("{} bytes written to disk", bytes_saved))
                }
                Err(err) => {
                    self.set_status_msg(format!("Can't save! Error: {}", err));
                }
            }
        }
    }

    fn start_search(&mut self) {
        self.draw_state.buffer.cursor.save_cursor();
        self.prompt = Some(Prompt::new_search(
            self.draw_state.col_offset(),
            self.draw_state.row_offset(),
        ));
        self.draw_state.top_prompt_visible = true;
        self.draw_state.update_search();
    }

    fn try_quit(&mut self) {
        if self.draw_state.buffer.is_dirty() {
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

    fn move_cursor(&mut self, movement: commands::MoveCursor) {
        self.draw_state
            .buffer
            .move_cursor(movement, self.draw_state.screen_rows() as usize);
        self.draw_state.update_cursor();
    }

    fn clone_cursor(&mut self) {
        self.draw_state.clone_cursor();
    }

    fn delete_char_backward(&mut self) {
        self.draw_state.delete_char();
    }

    fn delete_char_forward(&mut self) {
        self.move_cursor(commands::MoveCursor::right(1));
        self.draw_state.delete_char();
    }

    fn insert_newline_and_return(&mut self) {
        self.draw_state.insert_newline_and_return();
    }

    fn insert_char(&mut self, typed_char: char) {
        self.draw_state.insert_char(typed_char);
    }

    pub fn prompt_top_left(&self) -> Option<String> {
        if let Some(prompt) = &self.prompt {
            prompt.top_left_string()
        } else {
            None
        }
    }

    pub fn resize(&mut self, logical_size: LogicalSize) {
        self.logical_size = logical_size;
    }

    pub fn set_window_dimensions(&mut self, dimensions: (u16, u16)) {
        self.draw_state.set_window_dimensions(dimensions);
        self.resized = true;
    }

    pub fn set_dpi(&mut self, dpi: f32) {
        println!("DPI changed: {}", dpi);
        // FIXME: why do we need dpi AND ui_scale?
        self.dpi = dpi;
        self.draw_state.set_ui_scale(dpi);
    }

    pub fn set_line_height(&mut self, line_height: f32) {
        self.draw_state.set_line_height(line_height);
    }

    pub fn set_character_width(&mut self, character_width: f32) {
        self.draw_state.set_character_width(character_width);
    }
}
