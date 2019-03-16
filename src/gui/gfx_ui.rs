use crate::buffer::Buffer;
use crate::config::RunConfig;
use crate::debug_log::DebugLog;
use crate::editor::BIM_VERSION;
use crate::gui::draw_quad::DrawQuad;
use crate::gui::draw_state::DrawState;
use crate::gui::persist_window_state::PersistWindowState;
use crate::gui::{ColorFormat, DepthFormat};
use crate::highlight::{highlight_to_color, Highlight};
use crate::utils::char_position_to_byte_position;
use flame;
use gfx;
use gfx::Device;
use gfx_glyph::{GlyphBrushBuilder, GlyphCruncher, Scale, Section, SectionText, VariedSection};
use glutin::dpi::{LogicalPosition, LogicalSize};
use glutin::Api::OpenGl;
use glutin::{
    ContextBuilder, ElementState, Event, EventsLoop, GlProfile, GlRequest, Icon, KeyboardInput,
    ModifiersState, MouseScrollDelta, VirtualKeyCode, WindowBuilder, WindowEvent,
};
use std::error::Error;

enum Action {
    ResizeWindow,
    Quit,
}

const XBIM_DEBUG_LOG: &str = ".xbim_debug";

const STATUS_BG: [f32; 3] = [215.0 / 256.0, 0.0, 135.0 / 256.0];
const CURSOR_BG: [f32; 3] = [250.0 / 256.0, 250.0 / 256.0, 250.0 / 256.0];
const OTHER_CURSOR_BG: [f32; 3] = [255.0 / 256.0, 165.0 / 256.0, 0.0];
const LINE_COL_BG: [f32; 3] = [0.0, 0.0, 0.0];
const LINE_COLS_AT: [u32; 2] = [80, 120];

pub fn run(run_type: RunConfig) -> Result<(), Box<dyn Error>> {
    let debug_log = DebugLog::new(XBIM_DEBUG_LOG);
    debug_log.start()?;
    use crate::config::RunConfig::*;

    let mut persist_window_state = PersistWindowState::restore();

    let mut event_loop = EventsLoop::new();
    let mut logical_size = LogicalSize::new(650.0, 800.0);
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
    let mut dpi = monitor.get_hidpi_factor() as f32;
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
    let (window, mut device, mut factory, main_color, main_depth) =
        gfx_window_glutin::init::<ColorFormat, DepthFormat>(window_builder, context, &event_loop)
            .unwrap();

    debug_log.debugln_timestamped(&format!("color_view: {:?}", main_color))?;
    debug_log.debugln_timestamped(&format!("depth_view: {:?}", main_depth))?;

    unsafe {
        device.with_gl(|gl| gl.Disable(gfx_gl::FRAMEBUFFER_SRGB));
    }

    window.set_position(persist_window_state.logical_position);

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
    let mut window_resized = true;

    let mut action_queue = vec![];

    let mut buffer = Buffer::default();
    let filename = match run_type {
        RunOpenFile(ref filename_arg) => filename_arg,
        _ => "testfiles/kilo-dos2.c",
    };
    if let Err(e) = buffer.open(filename) {
        panic!("Error: {}", e);
    };
    let mut draw_state =
        DrawState::new(window_width.into(), window_height.into(), 18.0, dpi, buffer);
    let _default_status_text = format!("bim editor - version {}", BIM_VERSION);

    while running {
        flame::start("frame");
        #[allow(clippy::single_match)]
        event_loop.poll_events(|event| match event {
            Event::WindowEvent { event, .. } => {
                // match event {
                //     WindowEvent::KeyboardInput { .. } => {
                //         println!("keyboard event: {:?}", event);
                //     }
                //     _ => {}
                // };

                match event {
                    WindowEvent::CursorMoved { position, .. } => {
                        draw_state.mouse_position = position.into()
                    }
                    WindowEvent::MouseInput {
                        state: ElementState::Pressed,
                        ..
                    } => {
                        let real_position: (f64, f64) =
                            LogicalPosition::from(draw_state.mouse_position)
                                .to_physical(draw_state.ui_scale().into())
                                .into();
                        println!("Mouse click at: {:?}", real_position);
                        draw_state.move_cursor_to_mouse_position(real_position);
                    }
                    WindowEvent::MouseWheel {
                        delta: MouseScrollDelta::LineDelta(delta_x, delta_y),
                        ..
                    } => {
                        draw_state.scroll_window_vertically(-delta_y);
                        draw_state.scroll_window_horizontally(-delta_x);
                    }
                    WindowEvent::CloseRequested | WindowEvent::Destroyed => {
                        action_queue.push(Action::Quit)
                    }
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(VirtualKeyCode::Escape),
                                ..
                            },
                        ..
                    } => action_queue.push(Action::Quit),
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(VirtualKeyCode::Equals),
                                modifiers: ModifiersState { shift: true, .. },
                                ..
                            },
                        ..
                    } => {
                        draw_state.inc_font_size();
                        window_resized = true;
                    }
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(VirtualKeyCode::Minus),
                                modifiers: ModifiersState { shift: false, .. },
                                ..
                            },
                        ..
                    } => {
                        draw_state.dec_font_size();
                        window_resized = true;
                    }
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(VirtualKeyCode::M),
                                ..
                            },
                        ..
                    } => draw_state.print_info(),
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(VirtualKeyCode::Down),
                                ..
                            },
                        ..
                    } => draw_state.move_cursor_row(1),
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(VirtualKeyCode::Up),
                                ..
                            },
                        ..
                    } => draw_state.move_cursor_row(-1),
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(VirtualKeyCode::PageDown),
                                ..
                            },
                        ..
                    } => draw_state.move_cursor_page(1),
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(VirtualKeyCode::PageUp),
                                ..
                            },
                        ..
                    } => draw_state.move_cursor_page(-1),
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(VirtualKeyCode::Left),
                                ..
                            },
                        ..
                    } => draw_state.move_cursor_col(-1),
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(VirtualKeyCode::Right),
                                ..
                            },
                        ..
                    } => draw_state.move_cursor_col(1),
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(VirtualKeyCode::Space),
                                modifiers: ModifiersState { ctrl: true, .. },
                                ..
                            },
                        ..
                    } => draw_state.clone_cursor(),
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(VirtualKeyCode::Back),
                                ..
                            },
                        ..
                    } => draw_state.delete_char(),
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(VirtualKeyCode::Delete),
                                ..
                            },
                        ..
                    } => {
                        draw_state.move_cursor_col(1);
                        draw_state.delete_char();
                    }
                    WindowEvent::Resized(new_logical_size) => {
                        println!("Resized to: {:?}", new_logical_size);
                        logical_size = new_logical_size;
                        let _ = debug_log
                            .debugln_timestamped(&format!("logical_size: {:?}", logical_size,));
                        action_queue.push(Action::ResizeWindow);
                    }
                    WindowEvent::HiDpiFactorChanged(new_dpi) => {
                        println!("DPI changed: {}", new_dpi);
                        dpi = new_dpi as f32;
                        draw_state.set_ui_scale(dpi);
                        action_queue.push(Action::ResizeWindow);
                    }
                    WindowEvent::Moved(new_logical_position) => {
                        println!("Moved to {:?}", new_logical_position);
                        if let Some(monitor_name) = window.get_current_monitor().get_name() {
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
                    window.resize(physical_size);
                    gfx_window_glutin::update_views(
                        &window,
                        &mut draw_quad.data.out_color,
                        &mut draw_quad.data.out_depth,
                    );
                    {
                        let (width, height, ..) = draw_quad.data.out_color.get_dimensions();
                        println!("main_color.get_dimensions: ({}x{})", width, height);
                        println!("DPI: {}", dpi);
                        draw_state.set_window_dimensions((width, height));
                    }
                    window_resized = true;
                }
                Action::Quit => running = false,
            }
        }

        // Purple background
        let background = [0.16078, 0.16471, 0.26667, 1.0];
        encoder.clear(&draw_quad.data.out_color, background);
        encoder.clear_depth(&draw_quad.data.out_depth, 1.0);

        if window_resized {
            let _guard = flame::start_guard("window_resized");

            let test_section = VariedSection {
                bounds: (draw_state.inner_width(), draw_state.inner_height()),
                screen_position: (draw_state.left_padding(), 0.0),
                text: vec![SectionText {
                    text: "AB\nC\n",
                    scale: Scale::uniform(draw_state.font_scale()),
                    ..SectionText::default()
                }],
                ..VariedSection::default()
            };
            println!("Font scale: {:?}", draw_state.font_scale());

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
            println!("Calculated line_height: {:?}", line_height);
            draw_state.set_line_height(line_height);

            let a_pos_x = letter_a.x;
            let b_pos_x = letter_b.x;
            let character_width = b_pos_x - a_pos_x;
            println!("Calculated character_width: {:?}", character_width);
            draw_state.set_character_width(character_width);
            window_resized = false;
        }

        {
            let _guard = flame::start_guard("render cursor quad");
            // Render cursor
            // from top of line of text to bottom of line of text
            // from left of character to right of character
            draw_quad.draw(&mut encoder, CURSOR_BG, draw_state.cursor_transform());

            if let Some(cursor_transform) = draw_state.other_cursor_transform() {
                draw_quad.draw(&mut encoder, OTHER_CURSOR_BG, cursor_transform);
            }
        }

        let mut section_texts = vec![];

        {
            let _guard = flame::start_guard("highlighted_sections -> section_texts");

            let (cursor_text_row, cursor_text_col) = draw_state.cursor();
            for highlighted_section in draw_state.highlighted_sections.iter() {
                if highlighted_section.text_row as i32
                    > draw_state.screen_rows() + draw_state.row_offset().floor() as i32
                {
                    break;
                }
                if (highlighted_section.text_row as i32) < (draw_state.row_offset().floor() as i32)
                {
                    continue;
                }

                let hl = highlighted_section.highlight;
                let row_text = &draw_state.buffer.rows[highlighted_section.text_row].render;
                let first_col_byte =
                    char_position_to_byte_position(row_text, highlighted_section.first_col_idx);
                let last_col_byte =
                    char_position_to_byte_position(row_text, highlighted_section.last_col_idx);
                let render_text = &row_text[first_col_byte..=last_col_byte];
                if highlighted_section.text_row == cursor_text_row
                    && highlighted_section.first_col_idx <= cursor_text_col
                    && highlighted_section.last_col_idx >= cursor_text_col
                {
                    let cursor_offset = cursor_text_col - highlighted_section.first_col_idx;
                    let cursor_byte_offset =
                        char_position_to_byte_position(render_text, cursor_offset);
                    let next_byte_offset =
                        char_position_to_byte_position(render_text, cursor_offset + 1);
                    section_texts.push(SectionText {
                        text: &render_text[0..cursor_byte_offset],
                        scale: Scale::uniform(draw_state.font_scale()),
                        color: highlight_to_color(hl),
                        ..SectionText::default()
                    });
                    section_texts.push(SectionText {
                        text: &render_text[cursor_byte_offset..next_byte_offset],
                        scale: Scale::uniform(draw_state.font_scale()),
                        color: highlight_to_color(Highlight::Cursor),
                        ..SectionText::default()
                    });
                    section_texts.push(SectionText {
                        text: &render_text[next_byte_offset..],
                        scale: Scale::uniform(draw_state.font_scale()),
                        color: highlight_to_color(hl),
                        ..SectionText::default()
                    });
                } else {
                    section_texts.push(SectionText {
                        text: &render_text,
                        scale: Scale::uniform(draw_state.font_scale()),
                        color: highlight_to_color(hl),
                        ..SectionText::default()
                    });
                };
            }
        }

        {
            let _guard = flame::start_guard("render section_texts");

            let section = VariedSection {
                bounds: (draw_state.inner_width(), draw_state.inner_height()),
                screen_position: (draw_state.left_padding(), 0.0),
                text: section_texts,
                z: 1.0,
                ..VariedSection::default()
            };
            glyph_brush.queue(section);

            glyph_brush.draw_queued_with_transform(
                draw_state.row_offset_as_transform(),
                &mut encoder,
                &draw_quad.data.out_color,
                &draw_quad.data.out_depth,
            )?;
        }

        {
            let _guard = flame::start_guard("render lines");
            use cgmath::{Matrix4, Vector3};
            for line in LINE_COLS_AT.iter() {
                let scale =
                    Matrix4::from_nonuniform_scale(1.0 / draw_state.window_width(), 1.0, 1.0);
                let x_on_screen =
                    draw_state.left_padding() + (*line as f32 * draw_state.character_width());
                let x_move = (x_on_screen / draw_state.window_width()) * 2.0 - 1.0;
                let translate = Matrix4::from_translation(Vector3::new(x_move, 0.0, 0.2));
                let transform = translate * scale;
                draw_quad.draw(&mut encoder, LINE_COL_BG, transform);
            }
        }

        {
            let _guard = flame::start_guard("render status quad");
            // Render status background
            draw_quad.draw(&mut encoder, STATUS_BG, draw_state.status_transform());
        }

        {
            let _guard = flame::start_guard("render status text");

            let status_text = format!(
                "{} | {} | {}",
                draw_state.status_line.filename,
                draw_state.status_line.filetype,
                draw_state.status_line.cursor
            );
            let status_section = Section {
                bounds: (draw_state.inner_width(), draw_state.line_height() as f32),
                screen_position: (draw_state.left_padding(), draw_state.inner_height()),
                text: &status_text,
                color: [1.0, 1.0, 1.0, 1.0],
                scale: Scale::uniform(draw_state.font_scale()),
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
        window.swap_buffers()?;
        flame::end("swap_buffers");
        flame::start("device.cleanup");
        device.cleanup();
        flame::end("device.cleanup");

        flame::end_collapse("frame");
    }

    // Dump the report to disk
    // flame::dump_html(&mut File::create("flame-graph.html").unwrap()).unwrap();
    flame::dump_html(&mut std::fs::File::create("flame-graph.html").unwrap())?;

    Ok(())
}
