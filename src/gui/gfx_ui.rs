use crate::buffer::Buffer;
use crate::commands::{self, Cmd};
use crate::config::RunConfig;
use crate::debug_log::DebugLog;
use crate::editor::BIM_VERSION;
use crate::gui::draw_quad::DrawQuad;
use crate::gui::persist_window_state::PersistWindowState;
use crate::gui::window::Window;
use crate::gui::{ColorFormat, DepthFormat};
use flame;
use gfx;
use gfx::Device;
use gfx_glyph::{GlyphBrushBuilder, GlyphCruncher, Scale, Section, SectionText, VariedSection};
use glutin::dpi::LogicalSize;
use glutin::Api::OpenGl;
use glutin::{
    ContextBuilder, ElementState, Event, EventsLoop, GlProfile, GlRequest, Icon, KeyboardInput,
    MouseScrollDelta, VirtualKeyCode, WindowBuilder, WindowEvent,
};
use std::error::Error;

enum Action {
    ResizeWindow,
}

const XBIM_DEBUG_LOG: &str = ".xbim_debug";

const STATUS_BG: [f32; 3] = [215.0 / 256.0, 0.0, 135.0 / 256.0];
const CURSOR_BG: [f32; 3] = [250.0 / 256.0, 250.0 / 256.0, 250.0 / 256.0];
const OTHER_CURSOR_BG: [f32; 3] = [255.0 / 256.0, 165.0 / 256.0, 0.0];
const LINE_COL_BG: [f32; 3] = [0.0, 0.0, 0.0];

fn keyboard_event_to_command(event: KeyboardInput) -> Option<Cmd> {
    if event.state == ElementState::Pressed {
        match event.virtual_keycode {
            Some(VirtualKeyCode::Escape) => Some(Cmd::Quit),
            Some(VirtualKeyCode::Left) => Some(Cmd::Move(commands::MoveCursor::left(1))),
            Some(VirtualKeyCode::Right) => Some(Cmd::Move(commands::MoveCursor::right(1))),
            Some(VirtualKeyCode::Up) => Some(Cmd::Move(commands::MoveCursor::up(1))),
            Some(VirtualKeyCode::Down) => Some(Cmd::Move(commands::MoveCursor::down(1))),
            Some(VirtualKeyCode::PageDown) => Some(Cmd::Move(commands::MoveCursor::page_down(1))),
            Some(VirtualKeyCode::PageUp) => Some(Cmd::Move(commands::MoveCursor::page_up(1))),
            Some(VirtualKeyCode::Home) => Some(Cmd::Move(commands::MoveCursor::home())),
            Some(VirtualKeyCode::End) => Some(Cmd::Move(commands::MoveCursor::end())),
            Some(VirtualKeyCode::Equals) => {
                if event.modifiers.shift {
                    Some(Cmd::IncreaseFontSize)
                } else {
                    None
                }
            }
            Some(VirtualKeyCode::Minus) => {
                if event.modifiers.shift {
                    None
                } else {
                    Some(Cmd::DecreaseFontSize)
                }
            }
            Some(VirtualKeyCode::Space) => {
                if event.modifiers.ctrl {
                    Some(Cmd::CloneCursor)
                } else {
                    None
                }
            }
            Some(VirtualKeyCode::Back) => Some(Cmd::DeleteCharBackward),
            Some(VirtualKeyCode::Delete) => Some(Cmd::DeleteCharForward),
            Some(VirtualKeyCode::M) => Some(Cmd::PrintInfo),
            _ => None,
        }
    } else {
        None
    }
}

pub fn run(run_type: RunConfig) -> Result<(), Box<dyn Error>> {
    let debug_log = DebugLog::new(XBIM_DEBUG_LOG);
    debug_log.start()?;
    use crate::config::RunConfig::*;

    let mut persist_window_state = PersistWindowState::restore();

    let mut event_loop = EventsLoop::new();
    let logical_size = LogicalSize::new(650.0, 800.0);
    let mut monitor = event_loop.get_primary_monitor();
    if let Some(previous_monitor_name) = persist_window_state.monitor_name.as_ref() {
        for available_monitor in event_loop.get_available_monitors() {
            if let Some(avail_monitor_name) = available_monitor.get_name().as_ref() {
                if avail_monitor_name == previous_monitor_name {
                    monitor = available_monitor;
                }
            }
        }
    }
    let dpi = monitor.get_hidpi_factor() as f32;
    // If there's an icon.png lying about, use it as the window_icon...
    let icon = Icon::from_path("icon32.png").ok();
    let window_builder = WindowBuilder::new()
        .with_title("bim")
        .with_window_icon(icon)
        .with_dimensions(logical_size);
    let context = ContextBuilder::new()
        .with_gl(GlRequest::Specific(OpenGl, (4, 3)))
        .with_gl_profile(GlProfile::Core)
        .with_vsync(true);
    let (gfx_window, mut device, mut factory, main_color, main_depth) =
        gfx_window_glutin::init::<ColorFormat, DepthFormat>(window_builder, context, &event_loop)
            .expect("init gfx_window_glutin should work!");

    debug_log.debugln_timestamped(&format!("color_view: {:?}", main_color))?;
    debug_log.debugln_timestamped(&format!("depth_view: {:?}", main_depth))?;

    unsafe {
        device.with_gl(|gl| gl.Disable(gfx_gl::FRAMEBUFFER_SRGB));
    }

    gfx_window.set_position(persist_window_state.logical_position);

    let (window_width, window_height, ..) = main_color.get_dimensions();
    debug_log.debugln_timestamped(&format!(
        "window_width: {}, window_height: {}",
        window_width, window_height,
    ))?;

    let mut draw_quad = DrawQuad::new(&mut factory, main_color, main_depth);
    let fonts: Vec<&[u8]> = vec![include_bytes!("iosevka-regular.ttf")];

    let mut glyph_brush = GlyphBrushBuilder::using_fonts_bytes(fonts)
        .initial_cache_size((512, 512))
        .depth_test(gfx::preset::depth::LESS_EQUAL_WRITE)
        .build(factory.clone());

    let mut encoder: gfx::Encoder<_, _> = factory.create_command_buffer().into();

    let mut running = true;

    let mut action_queue = vec![];

    let mut buffer = Buffer::default();
    let filename = match run_type {
        RunOpenFile(ref filename_arg) => filename_arg,
        _ => "testfiles/kilo-dos2.c",
    };
    if let Err(e) = buffer.open(filename) {
        panic!("Error: {}", e);
    };
    let mut window = Window::new(
        logical_size,
        dpi,
        window_width.into(),
        window_height.into(),
        18.0,
        dpi,
        buffer,
    );

    let _default_status_text = format!("bim editor - version {}", BIM_VERSION);

    while running {
        flame::start("frame");
        window.next_frame();
        #[allow(clippy::single_match)]
        event_loop.poll_events(|event| match event {
            Event::WindowEvent { event, .. } => {
                match event {
                    WindowEvent::CursorMoved { position, .. } => {
                        window.update_mouse_position(position.into())
                    }
                    WindowEvent::MouseInput {
                        state: ElementState::Pressed,
                        ..
                    } => window.mouse_click(),
                    WindowEvent::MouseWheel {
                        delta: MouseScrollDelta::LineDelta(delta_x, delta_y),
                        ..
                    } => window.mouse_scroll(delta_x, delta_y),
                    WindowEvent::CloseRequested | WindowEvent::Destroyed => running = false,
                    WindowEvent::KeyboardInput {
                        input: keyboard_input,
                        ..
                    } => match keyboard_event_to_command(keyboard_input) {
                        Some(Cmd::Move(move_cursor)) => window.move_cursor(move_cursor),
                        Some(Cmd::Quit) => running = false,
                        Some(Cmd::IncreaseFontSize) => window.inc_font_size(),
                        Some(Cmd::DecreaseFontSize) => window.dec_font_size(),
                        Some(Cmd::CloneCursor) => window.clone_cursor(),
                        Some(Cmd::DeleteCharBackward) => window.delete_char_backward(),
                        Some(Cmd::DeleteCharForward) => window.delete_char_forward(),
                        Some(Cmd::PrintInfo) => window.print_info(),
                        Some(Cmd::Linebreak) => {}
                        Some(Cmd::Save) => {}
                        Some(Cmd::InsertChar(_)) => {}
                        Some(Cmd::Search) => {}
                        None => {}
                    },
                    WindowEvent::Resized(new_logical_size) => {
                        window.resize(new_logical_size);
                        action_queue.push(Action::ResizeWindow);
                    }
                    WindowEvent::HiDpiFactorChanged(new_dpi) => {
                        window.set_dpi(new_dpi as f32);
                        action_queue.push(Action::ResizeWindow);
                    }
                    WindowEvent::Moved(new_logical_position) => {
                        if let Some(monitor_name) = gfx_window.get_current_monitor().get_name() {
                            persist_window_state.monitor_name = Some(monitor_name);
                        }
                        persist_window_state.logical_position = new_logical_position;
                        persist_window_state.save();
                    }
                    _ => (),
                };
            }
            _ => (),
        });

        while let Some(action) = action_queue.pop() {
            match action {
                Action::ResizeWindow => {
                    let physical_size = logical_size.to_physical(dpi.into());
                    debug_log
                        .debugln_timestamped(&format!("physical_size: {:?}", physical_size,))?;
                    gfx_window.resize(physical_size);
                    gfx_window_glutin::update_views(
                        &gfx_window,
                        &mut draw_quad.data.out_color,
                        &mut draw_quad.data.out_depth,
                    );
                    {
                        let (width, height, ..) = draw_quad.data.out_color.get_dimensions();
                        window.set_window_dimensions((width, height));
                    }
                }
            }
        }

        // Purple background
        let background = [0.16078, 0.16471, 0.26667, 1.0];
        encoder.clear(&draw_quad.data.out_color, background);
        encoder.clear_depth(&draw_quad.data.out_depth, 1.0);

        if window.has_resized() {
            let _guard = flame::start_guard("window_resized");

            let test_section = VariedSection {
                bounds: window.inner_dimensions(),
                screen_position: (window.left_padding(), 0.0),
                text: vec![SectionText {
                    text: "AB\nC\n",
                    scale: Scale::uniform(window.font_scale()),
                    ..SectionText::default()
                }],
                ..VariedSection::default()
            };

            flame::start("glyphs");
            let test_glyphs = glyph_brush.glyphs(test_section);
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
            window.set_line_height(line_height);

            let a_pos_x = letter_a.x;
            let b_pos_x = letter_b.x;
            let character_width = b_pos_x - a_pos_x;
            window.set_character_width(character_width);
        }

        {
            let _guard = flame::start_guard("render cursor quad");
            draw_quad.draw(&mut encoder, CURSOR_BG, window.cursor_transform());

            if let Some(cursor_transform) = window.other_cursor_transform() {
                draw_quad.draw(&mut encoder, OTHER_CURSOR_BG, cursor_transform);
            }
        }

        let section_texts = window.section_texts();

        {
            let _guard = flame::start_guard("render section_texts");

            let section = VariedSection {
                bounds: window.inner_dimensions(),
                screen_position: (window.left_padding(), 0.0),
                text: section_texts,
                z: 1.0,
                ..VariedSection::default()
            };
            glyph_brush.queue(section);

            glyph_brush.draw_queued_with_transform(
                window.row_offset_as_transform().into(),
                &mut encoder,
                &draw_quad.data.out_color,
                &draw_quad.data.out_depth,
            )?;
        }

        {
            let _guard = flame::start_guard("render lines");
            for transform in window.line_transforms() {
                draw_quad.draw(&mut encoder, LINE_COL_BG, transform);
            }
        }

        {
            let _guard = flame::start_guard("render status quad");
            // Render status background
            draw_quad.draw(&mut encoder, STATUS_BG, window.status_transform());
        }

        {
            let _guard = flame::start_guard("render status text");

            let status_text = window.status_text();
            let status_section = Section {
                bounds: window.inner_dimensions(),
                screen_position: (window.left_padding(), window.inner_dimensions().1),
                text: &status_text,
                color: [1.0, 1.0, 1.0, 1.0],
                scale: Scale::uniform(window.font_scale()),
                z: 0.5,
                ..Section::default()
            };
            glyph_brush.queue(status_section);
            glyph_brush.draw_queued(
                &mut encoder,
                &draw_quad.data.out_color,
                &draw_quad.data.out_depth,
            )?;
        }

        flame::start("encoder.flush");
        encoder.flush(&mut device);
        flame::end("encoder.flush");
        flame::start("swap_buffers");
        gfx_window.swap_buffers()?;
        flame::end("swap_buffers");
        flame::start("device.cleanup");
        device.cleanup();
        flame::end("device.cleanup");

        flame::end_collapse("frame");
    }

    // Dump the report to disk
    flame::dump_html(&mut std::fs::File::create("flame-graph.html").unwrap())?;

    Ok(())
}
