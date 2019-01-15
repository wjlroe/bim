use bim::buffer::Buffer;
use bim::config::RunConfig;
use bim::editor::BIM_VERSION;
use bim::highlight::Highlight;
use cgmath::{Matrix4, Vector3};
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
use std::{env, error::Error};

enum Action {
    ResizeWindow,
    Quit,
}

#[derive(Copy, Clone, Default)]
struct DrawState {
    window_width: f32,
    window_height: f32,
    line_height: i32,
    font_size: f32,
    ui_scale: f32,
    left_padding: f32,
    resized: bool,
}

impl DrawState {
    fn font_size(&self) -> f32 {
        self.font_size * self.ui_scale
    }

    fn status_height(&self) -> f32 {
        self.font_size * self.ui_scale
    }

    fn status_transform(&self) -> Matrix4<f32> {
        let status_height = self.status_height();
        let status_scale = Matrix4::from_nonuniform_scale(
            1.0,
            status_height / self.window_height,
            1.0,
        );
        let y_move = -((self.window_height - status_height) / status_height);
        let status_move =
            Matrix4::from_translation(Vector3::new(0.0, y_move, 0.0));
        status_scale * status_move
    }

    fn inner_width(&self) -> f32 {
        self.window_width - self.left_padding
    }

    fn inner_height(&self) -> f32 {
        self.window_height - self.status_height()
    }

    fn print_info(&self) {
        println!(
            "status_height: {}, inner: ({}, {}), status_transform: {:?}",
            self.status_height(),
            self.inner_width(),
            self.inner_height(),
            self.status_transform()
        );
    }

    fn inc_font_size(&mut self) {
        self.font_size += 1.0;
        self.resized = true;
    }

    fn dec_font_size(&mut self) {
        self.font_size -= 1.0;
        self.resized = true;
    }

    fn set_window_dimensions(&mut self, (width, height): (u16, u16)) {
        let (width, height) = (f32::from(width), f32::from(height));
        self.window_height = height;
        self.window_width = width;
    }
}

pub type ColorFormat = format::Rgba8;
pub type DepthFormat = format::Depth;

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

const QUAD: [Vertex; 4] = [
    Vertex { pos: [-1.0, 1.0] },
    Vertex { pos: [-1.0, -1.0] },
    Vertex { pos: [1.0, -1.0] },
    Vertex { pos: [1.0, 1.0] },
];

fn highlight_to_color(hl: Highlight) -> [f32; 4] {
    use self::Highlight::*;

    match hl {
        Normal => [232.0 / 255.0, 230.0 / 255.0, 237.0 / 255.0, 1.0],
        Number => [221.0 / 255.0, 119.0 / 255.0, 85.0 / 255.0, 1.0],
        String => [191.0 / 255.0, 156.0 / 255.0, 249.0 / 255.0, 1.0],
        Comment | MultilineComment => {
            [86.0 / 255.0, 211.0 / 255.0, 194.0 / 255.0, 1.0]
        }
        Keyword1 => [242.0 / 255.0, 231.0 / 255.0, 183.0 / 255.0, 1.0],
        Keyword2 => [4.0 / 255.0, 219.0 / 255.0, 181.0 / 255.0, 1.0],
        _ => [0.9, 0.4, 0.2, 1.0],
    }
}

fn run(run_type: RunConfig) -> Result<(), Box<dyn Error>> {
    use bim::config::RunConfig::*;

    let mut draw_state = DrawState::default();
    let mut event_loop = EventsLoop::new();
    let mut logical_size = LogicalSize::new(400.0, 800.0);
    if let Some(monitor) = event_loop.get_available_monitors().next() {
        draw_state.ui_scale = monitor.get_hidpi_factor() as f32;
    }
    let window_builder = WindowBuilder::new()
        .with_title("bim")
        .with_dimensions(logical_size);
    let context = ContextBuilder::new()
        .with_gl(GlRequest::Specific(OpenGl, (3, 2)))
        .with_gl_profile(GlProfile::Core)
        .with_vsync(true);
    let (window, mut device, mut factory, mut main_color, mut main_depth) =
        gfx_window_glutin::init::<ColorFormat, DepthFormat>(
            window_builder,
            context,
            &event_loop,
        )
        .unwrap();

    unsafe {
        device.with_gl(|gl| gl.Disable(gfx_gl::FRAMEBUFFER_SRGB));
    }

    window.set_position(LogicalPosition::new(400.0, 50.0));

    {
        let (width, height, ..) = main_color.get_dimensions();
        draw_state.set_window_dimensions((width, height));
    }
    draw_state.font_size = 18.0;
    draw_state.left_padding = 12.0;
    draw_state.resized = true;

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
    let quad_data = pipe::Data {
        vbuf: quad_vbuf,
        locals: factory.create_constant_buffer(2),
        out_color: main_color.clone(),
        out_depth: main_depth.clone(),
    };

    let fonts: Vec<&[u8]> = vec![include_bytes!("../iosevka-regular.ttf")];

    let mut glyph_brush = GlyphBrushBuilder::using_fonts_bytes(fonts)
        .initial_cache_size((512, 512))
        .depth_test(gfx::preset::depth::LESS_EQUAL_WRITE)
        .build(factory.clone());

    let mut encoder: gfx::Encoder<_, _> =
        factory.create_command_buffer().into();

    let mut running = true;

    let mut action_queue = vec![];

    let mut buffer = Buffer::new();
    let filename = match run_type {
        RunOpenFile(ref filename_arg) => filename_arg,
        _ => "../testfiles/kilo-dos2.c",
    };
    match buffer.open(filename) {
        Err(e) => panic!("Error: {}", e),
        _ => {}
    };
    let status_text = format!("bim editor - version {}", BIM_VERSION);

    #[derive(Clone)]
    struct HighlightedSection {
        text: String,
        highlight: Option<Highlight>,
    };
    let mut current_section = HighlightedSection {
        text: String::new(),
        highlight: None,
    };
    let mut highlighted_sections = vec![];
    for row in buffer.rows {
        let mut highlights = row.hl.iter();
        for c in row.render.chars() {
            let hl = highlights.next().cloned().unwrap_or(Highlight::Normal);
            if current_section.highlight.is_none() {
                current_section.highlight = Some(hl);
            }
            if current_section.highlight == Some(hl) {
                current_section.text.push(c);
            } else {
                highlighted_sections.push(current_section.clone());
                current_section.text.clear();
                current_section.highlight = None;
                current_section.text.push(c);
            }
        }
        current_section.text.push('\n');
    }
    if current_section.text != "" {
        highlighted_sections.push(current_section.clone());
    }

    while running {
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
                WindowEvent::Resized(new_logical_size) => {
                    println!("Resized to: {:?}", new_logical_size);
                    logical_size = new_logical_size;
                    action_queue.push(Action::ResizeWindow);
                }
                WindowEvent::HiDpiFactorChanged(new_dpi) => {
                    println!("DPI changed: {}", new_dpi);
                    draw_state.ui_scale = new_dpi as f32;
                    action_queue.push(Action::ResizeWindow);
                }
                _ => (),
            },
            _ => (),
        });

        while let Some(action) = action_queue.pop() {
            match action {
                Action::ResizeWindow => {
                    window.resize(
                        logical_size.to_physical(draw_state.ui_scale as f64),
                    );
                    gfx_window_glutin::update_views(
                        &window,
                        &mut main_color,
                        &mut main_depth,
                    );
                    {
                        let (width, height, ..) = main_color.get_dimensions();
                        draw_state.set_window_dimensions((width, height));
                    }
                }
                Action::Quit => running = false,
            }
        }

        // Purple background
        let background = [0.16078, 0.16471, 0.26667, 1.0];
        encoder.clear(&main_color, background);
        encoder.clear_depth(&main_depth, 1.0);

        let section_texts = highlighted_sections
            .iter()
            .map(|hl_section| SectionText {
                text: &hl_section.text,
                scale: Scale::uniform(draw_state.font_size()),
                color: highlight_to_color(
                    hl_section.highlight.unwrap_or(Highlight::Normal),
                ),
                ..SectionText::default()
            })
            .collect::<Vec<_>>();

        let section = VariedSection {
            bounds: (draw_state.inner_width(), draw_state.inner_height()),
            screen_position: (draw_state.left_padding, 0.0),
            text: section_texts,
            z: 1.0,
            ..VariedSection::default()
        };

        if draw_state.resized {
            if let Some(glyph) = glyph_brush.glyphs(section.clone()).next() {
                if let Some(bounding_box) = glyph.pixel_bounding_box() {
                    draw_state.line_height =
                        bounding_box.max.y - bounding_box.min.y;
                    draw_state.resized = false;
                }
            }
        }

        let status_section = Section {
            bounds: (draw_state.inner_width(), draw_state.status_height()),
            screen_position: (
                draw_state.left_padding,
                draw_state.inner_height(),
            ),
            text: &status_text,
            color: [1.0, 1.0, 1.0, 1.0],
            scale: Scale::uniform(draw_state.font_size()),
            z: 0.5,
            ..Section::default()
        };

        glyph_brush.queue(section);
        glyph_brush.queue(status_section);

        glyph_brush.draw_queued(&mut encoder, &main_color, &main_depth)?;

        let quad_locals = Locals {
            color: STATUS_BG,
            transform: draw_state.status_transform().into(),
        };
        // FIXME: Only update if they've changed
        encoder.update_constant_buffer(&quad_data.locals, &quad_locals);

        encoder.draw(&quad_slice, &quad_pso, &quad_data);

        encoder.flush(&mut device);
        window.swap_buffers()?;
        device.cleanup();
    }

    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let filename_arg = env::args().skip(1).nth(0);
    let run_type = if let Some(filename) = filename_arg {
        RunConfig::RunOpenFile(filename)
    } else {
        RunConfig::Run
    };

    run(run_type)?;
    Ok(())
}
