use crate::buffer::Buffer;
use crate::config::RunConfig;
use crate::editor::BIM_VERSION;
use crate::gui::draw_state::DrawState;
use crate::highlight::{highlight_to_color, Highlight};
use gfx;
use gfx::traits::FactoryExt;
use gfx::*;
use gfx::{format, Device};
use gfx_glyph::{
    GlyphBrushBuilder, GlyphCruncher, Scale, Section, SectionText,
    VariedSection,
};
use glutin::dpi::{LogicalPosition, LogicalSize};
use glutin::Api::OpenGl;
use glutin::{
    ContextBuilder, ElementState, Event, EventsLoop, GlProfile, GlRequest,
    KeyboardInput, VirtualKeyCode, WindowBuilder, WindowEvent,
};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fs;
use std::io::Read;

pub type ColorFormat = format::Rgba8;
pub type DepthFormat = format::Depth;

#[derive(Serialize, Deserialize)]
struct PersistWindowState {
    monitor_name: Option<String>,
    logical_position: LogicalPosition,
}

impl PersistWindowState {
    fn save(&self) {
        match toml::to_string(self) {
            Ok(config_string) => {
                fs::write(Self::config_filename(), config_string).unwrap();
            }
            Err(e) => {
                println!("Error saving config to string: {:?}", e);
            }
        }
    }

    fn restore() -> Self {
        match fs::File::open(Self::config_filename()) {
            Ok(mut f) => {
                let mut config = String::new();
                match f.read_to_string(&mut config) {
                    Ok(_) => match toml::from_str::<Self>(&config) {
                        Ok(persised) => return persised,
                        Err(e) => {
                            println!("Error deserializing config: {:?}", e)
                        }
                    },
                    Err(e) => println!("Error reading config file: {:?}", e),
                }
            }
            Err(e) => println!("Error opening config file: {:?}", e),
        }
        Self::default()
    }

    fn config_filename() -> String {
        String::from(".bim_persist_state.toml")
    }
}

impl Default for PersistWindowState {
    fn default() -> Self {
        Self {
            logical_position: LogicalPosition::new(400.0, 50.0),
            monitor_name: None,
        }
    }
}

enum Action {
    ResizeWindow,
    Quit,
}

gfx_defines! {
  vertex Vertex {
    pos: [f32; 2] = "a_Pos",
  }

  constant Locals {
    transform: [[f32; 4]; 4] = "u_Transform",
    color: [f32; 3] = "u_Color",
  }

  pipeline pipe {
    vbuf: gfx::VertexBuffer<Vertex> = (),
    locals: gfx::ConstantBuffer<Locals> = "Locals",
    out_color: gfx::BlendTarget<ColorFormat> = ("Target0", gfx::state::ColorMask::all(), gfx::preset::blend::ALPHA),
    out_depth: gfx::DepthTarget<DepthFormat> = gfx::preset::depth::LESS_EQUAL_WRITE,
  }
}

const STATUS_BG: [f32; 3] = [215.0 / 256.0, 0.0, 135.0 / 256.0];
const CURSOR_BG: [f32; 3] = [250.0 / 256.0, 250.0 / 256.0, 250.0 / 256.0];

const QUAD: [Vertex; 4] = [
    Vertex { pos: [-1.0, 1.0] },
    Vertex { pos: [-1.0, -1.0] },
    Vertex { pos: [1.0, -1.0] },
    Vertex { pos: [1.0, 1.0] },
];

pub fn run(run_type: RunConfig) -> Result<(), Box<dyn Error>> {
    use crate::config::RunConfig::*;

    let mut persist_window_state = PersistWindowState::restore();

    let mut event_loop = EventsLoop::new();
    let mut logical_size = LogicalSize::new(600.0, 800.0);
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
    let window_builder = WindowBuilder::new()
        .with_title("bim")
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

    let quad_pso = factory
        .create_pipeline_simple(
            include_bytes!("shaders/quad_150_core.vert"),
            include_bytes!("shaders/quad_150_core.frag"),
            pipe::new(),
        )
        .expect("quad pso construction to work");
    let (quad_vbuf, quad_slice) = factory.create_vertex_buffer_with_slice(
        &QUAD,
        &[0u16, 1, 2, 2, 3, 0] as &[u16],
    );
    let mut quad_data = pipe::Data {
        vbuf: quad_vbuf,
        locals: factory.create_constant_buffer(2),
        out_color: main_color,
        out_depth: main_depth,
    };

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
    let status_text = format!("bim editor - version {}", BIM_VERSION);

    while running {
        #[allow(clippy::single_match)]
        event_loop.poll_events(|event| match event {
            Event::WindowEvent { event, .. } => match event {
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
                        &mut quad_data.out_color,
                        &mut quad_data.out_depth,
                    );
                    {
                        let (width, height, ..) =
                            quad_data.out_color.get_dimensions();
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
        encoder.clear(&quad_data.out_color, background);
        encoder.clear_depth(&quad_data.out_depth, 1.0);

        if window_resized {
            let test_section = VariedSection {
                bounds: (draw_state.inner_width(), draw_state.inner_height()),
                screen_position: (draw_state.left_padding(), 0.0),
                text: vec![SectionText {
                    text: "A\nBC\n",
                    scale: Scale::uniform(draw_state.font_scale()),
                    ..SectionText::default()
                }],
                ..VariedSection::default()
            };
            println!("Font scale: {:?}", draw_state.font_scale());

            let test_glyphs = glyph_brush.glyphs(test_section);
            let positions = test_glyphs
                .map(|glyph| {
                    (glyph.position(), glyph.pixel_bounding_box().unwrap())
                })
                .collect::<Vec<_>>();
            // .map(|bounding_box| bounding_box.min.y)
            let first_line_min_y = positions[0].0.y;
            let secon_line_min_y = positions[1].0.y;
            let line_height = secon_line_min_y - first_line_min_y;
            println!("Calculated line_height: {:?}", line_height);
            draw_state.set_line_height(line_height as i32);

            let fst_line_bound_min_y = positions[0].1.min.y;
            let snd_line_bound_min_y = positions[1].1.min.y;
            let bound_box_line_height =
                snd_line_bound_min_y - fst_line_bound_min_y;
            println!("Bound box calc line height: {:?}", bound_box_line_height);

            let b_min_x = positions[1].0.x;
            let c_min_x = positions[2].0.x;
            let character_width = c_min_x - b_min_x;
            println!("Calculated character_width: {:?}", character_width);
            draw_state.set_character_width(character_width as i32);
            window_resized = false;
        }

        {
            // Render cursor
            // from top of line of text to bottom of line of text
            // from left of character to right of character
            let cursor_debug_name = std::ffi::CString::new("Cursor").unwrap();
            unsafe {
                device.with_gl(|gl| {
                    gl.PushDebugGroup(
                        gfx_gl::DEBUG_SOURCE_APPLICATION,
                        1,
                        -1,
                        cursor_debug_name.as_ptr(),
                    );
                });
            }
            let quad_locals = Locals {
                color: CURSOR_BG,
                transform: draw_state.cursor_transform().into(),
            };

            // FIXME: Only update if they've changed
            encoder.update_constant_buffer(&quad_data.locals, &quad_locals);
            encoder.draw(&quad_slice, &quad_pso, &quad_data);
            unsafe {
                device.with_gl(|gl| gl.PopDebugGroup());
            }
        }

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
            screen_position: (draw_state.left_padding(), 0.0),
            text: section_texts,
            z: 1.0,
            ..VariedSection::default()
        };
        glyph_brush.queue(section);
        glyph_brush.queue(status_section);

        glyph_brush.draw_queued(
            &mut encoder,
            &quad_data.out_color,
            &quad_data.out_depth,
        )?;

        {
            // Render status background
            let quad_locals = Locals {
                color: STATUS_BG,
                transform: draw_state.status_transform().into(),
            };

            // FIXME: Only update if they've changed
            encoder.update_constant_buffer(&quad_data.locals, &quad_locals);
            encoder.draw(&quad_slice, &quad_pso, &quad_data);
        }

        encoder.flush(&mut device);
        window.swap_buffers()?;
        device.cleanup();
    }

    Ok(())
}
