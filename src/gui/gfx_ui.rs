use crate::buffer::Buffer;
use crate::config::RunConfig;
use crate::editor::BIM_VERSION;
use crate::gui::draw_quad::DrawQuad;
use crate::gui::draw_state::DrawState;
use crate::gui::persist_window_state::PersistWindowState;
use crate::gui::{ColorFormat, DepthFormat};
use crate::highlight::{highlight_to_color, Highlight};
use gfx;
use gfx::Device;
use gfx_glyph::{
    GlyphBrushBuilder, GlyphCruncher, Scale, Section, SectionText,
    VariedSection,
};
use glutin::dpi::{LogicalPosition, LogicalSize};
use glutin::Api::OpenGl;
use glutin::{
    ContextBuilder, ElementState, Event, EventsLoop, GlProfile, GlRequest,
    Icon, KeyboardInput, ModifiersState, VirtualKeyCode, WindowBuilder,
    WindowEvent,
};
use std::error::Error;

enum Action {
    ResizeWindow,
    Quit,
}

const STATUS_BG: [f32; 3] = [215.0 / 256.0, 0.0, 135.0 / 256.0];
const CURSOR_BG: [f32; 3] = [250.0 / 256.0, 250.0 / 256.0, 250.0 / 256.0];
const OTHER_CURSOR_BG: [f32; 3] = [255.0 / 256.0, 165.0 / 256.0, 0.0];
const LINE_COL_BG: [f32; 3] = [0.0, 0.0, 0.0];
const LINE_COLS_AT: [u32; 2] = [80, 120];

pub fn run(run_type: RunConfig) -> Result<(), Box<dyn Error>> {
    use crate::config::RunConfig::*;

    let mut persist_window_state = PersistWindowState::restore();

    let mut event_loop = EventsLoop::new();
    let mut logical_size = LogicalSize::new(650.0, 800.0);
    let mut monitor = event_loop.get_primary_monitor();
    if let Some(previous_monitor_name) =
        persist_window_state.monitor_name.as_ref()
    {
        for available_monitor in event_loop.get_available_monitors() {
            if let Some(avail_monitor_name) =
                available_monitor.get_name().as_ref()
            {
                if avail_monitor_name == previous_monitor_name {
                    monitor = available_monitor;
                }
            }
        }
    }
    let mut dpi = monitor.get_hidpi_factor() as f32;
    // If there's an icon.png lying about, use it as the window_icon...
    let icon = "icon.png";
    let window_builder = WindowBuilder::new()
        .with_title("bim")
        .with_window_icon(Icon::from_path(icon).ok())
        .with_dimensions(logical_size);
    let context = ContextBuilder::new()
        .with_gl(GlRequest::Specific(OpenGl, (4, 3)))
        .with_gl_profile(GlProfile::Core)
        .with_vsync(true);
    let (window, mut device, mut factory, main_color, main_depth) =
        gfx_window_glutin::init::<ColorFormat, DepthFormat>(
            window_builder,
            context,
            &event_loop,
        )
        .unwrap();

    unsafe {
        device.with_gl(|gl| gl.Disable(gfx_gl::FRAMEBUFFER_SRGB));
    }

    window.set_position(persist_window_state.logical_position);

    let (window_width, window_height, ..) = main_color.get_dimensions();

    let mut draw_quad = DrawQuad::new(&mut factory, main_color, main_depth);
    let fonts: Vec<&[u8]> = vec![include_bytes!("iosevka-regular.ttf")];

    let mut glyph_brush = GlyphBrushBuilder::using_fonts_bytes(fonts)
        .initial_cache_size((512, 512))
        .depth_test(gfx::preset::depth::LESS_EQUAL_WRITE)
        .build(factory.clone());

    let mut encoder: gfx::Encoder<_, _> =
        factory.create_command_buffer().into();

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
    let mut draw_state = DrawState::new(
        window_width.into(),
        window_height.into(),
        18.0,
        dpi,
        buffer,
    );
    let _default_status_text = format!("bim editor - version {}", BIM_VERSION);

    while running {
        #[allow(clippy::single_match)]
        event_loop.poll_events(|event| match event {
            Event::WindowEvent { event, .. } => match event {
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
                            virtual_keycode: Some(VirtualKeyCode::Add),
                            ..
                        },
                    ..
                } => draw_state.inc_font_size(),
                WindowEvent::KeyboardInput {
                    input:
                        KeyboardInput {
                            state: ElementState::Pressed,
                            virtual_keycode: Some(VirtualKeyCode::Subtract),
                            ..
                        },
                    ..
                } => draw_state.dec_font_size(),
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
                WindowEvent::Resized(new_logical_size) => {
                    println!("Resized to: {:?}", new_logical_size);
                    logical_size = new_logical_size;
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
                    if let Some(monitor_name) =
                        window.get_current_monitor().get_name()
                    {
                        persist_window_state.monitor_name = Some(monitor_name);
                    }
                    persist_window_state.logical_position =
                        new_logical_position;
                    persist_window_state.save();
                }
                _ => (),
            },
            _ => (),
        });

        while let Some(action) = action_queue.pop() {
            match action {
                Action::ResizeWindow => {
                    window.resize(logical_size.to_physical(dpi.into()));
                    gfx_window_glutin::update_views(
                        &window,
                        &mut draw_quad.data.out_color,
                        &mut draw_quad.data.out_depth,
                    );
                    {
                        let (width, height, ..) =
                            draw_quad.data.out_color.get_dimensions();
                        println!(
                            "main_color.get_dimensions: ({}x{})",
                            width, height
                        );
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

            let test_glyphs = glyph_brush.glyphs(test_section);
            let positions = test_glyphs
                .map(|glyph| glyph.position())
                .collect::<Vec<_>>();
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
            // Render cursor
            // from top of line of text to bottom of line of text
            // from left of character to right of character
            draw_quad.draw(
                &mut encoder,
                CURSOR_BG,
                draw_state.cursor_transform(),
            );

            if let Some(cursor_transform) = draw_state.other_cursor_transform()
            {
                draw_quad.draw(&mut encoder, OTHER_CURSOR_BG, cursor_transform);
            }
        }

        let mut section_texts = vec![];
        for highlighted_section in draw_state.highlighted_sections.iter() {
            let hl = highlighted_section.highlight.unwrap_or(Highlight::Normal);
            let section = SectionText {
                text: &highlighted_section.text,
                scale: Scale::uniform(draw_state.font_scale()),
                color: highlight_to_color(hl),
                ..SectionText::default()
            };
            section_texts.push(section);
        }

        let section = VariedSection {
            bounds: (draw_state.inner_width(), draw_state.inner_height()),
            screen_position: (
                draw_state.left_padding(),
                -draw_state.row_offset(),
            ),
            text: section_texts,
            z: 1.0,
            ..VariedSection::default()
        };
        glyph_brush.queue(section);

        glyph_brush.draw_queued(
            &mut encoder,
            &draw_quad.data.out_color,
            &draw_quad.data.out_depth,
        )?;

        {
            use cgmath::{Matrix4, Vector3};
            for line in LINE_COLS_AT.iter() {
                let scale = Matrix4::from_nonuniform_scale(
                    1.0 / draw_state.window_width(),
                    1.0,
                    1.0,
                );
                let x_on_screen = draw_state.left_padding()
                    + (*line as f32 * draw_state.character_width());
                let x_move =
                    (x_on_screen / draw_state.window_width()) * 2.0 - 1.0;
                let translate =
                    Matrix4::from_translation(Vector3::new(x_move, 0.0, 0.2));
                let transform = translate * scale;
                draw_quad.draw(&mut encoder, LINE_COL_BG, transform);
            }
        }

        {
            // Render status background
            draw_quad.draw(
                &mut encoder,
                STATUS_BG,
                draw_state.status_transform(),
            );
        }

        let status_text = format!(
            "{} | {} | {}",
            draw_state.status_line.filename,
            draw_state.status_line.filetype,
            draw_state.status_line.cursor
        );
        let status_section = Section {
            bounds: (draw_state.inner_width(), draw_state.line_height() as f32),
            screen_position: (
                draw_state.left_padding(),
                draw_state.inner_height(),
            ),
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

        encoder.flush(&mut device);
        window.swap_buffers()?;
        device.cleanup();
    }

    Ok(())
}
