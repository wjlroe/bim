use bim::config::RunConfig;
use cgmath::{Matrix4, Vector3};
use gfx;
use gfx::traits::FactoryExt;
use gfx::*;
use gfx::{format, Device};
use gfx_glyph::{GlyphBrushBuilder, GlyphCruncher, Scale, Section};
use glutin::dpi::LogicalPosition;
use glutin::Api::OpenGl;
use glutin::{
  ContextBuilder, Event, EventsLoop, GlProfile, GlRequest, KeyboardInput,
  VirtualKeyCode, WindowBuilder, WindowEvent,
};
use std::{env, error::Error};

#[derive(Copy, Clone, Default)]
struct DrawState {
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

  fn inc_font_size(&mut self) {
    self.font_size += 1.0;
    self.resized = true;
  }

  fn dec_font_size(&mut self) {
    self.font_size -= 1.0;
    self.resized = true;
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

fn run(_run_type: RunConfig) -> Result<(), Box<dyn Error>> {
  let mut event_loop = EventsLoop::new();
  let window_builder = WindowBuilder::new()
    .with_title("bim")
    .with_dimensions((400, 500).into());
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

  let (width, height, ..) = main_color.get_dimensions();
  let (width, height) = (f32::from(width), f32::from(height));

  let mut draw_state = DrawState::default();
  draw_state.font_size = 18.0;
  draw_state.ui_scale = 1.5;
  draw_state.left_padding = 12.0;
  draw_state.resized = true;

  let quad_pso = factory
    .create_pipeline_simple(
      include_bytes!("shaders/quad_150_core.vert"),
      include_bytes!("shaders/quad_150_core.frag"),
      pipe::new(),
    )
    .expect("quad pso construction to work");
  let (quad_vbuf, quad_slice) = factory
    .create_vertex_buffer_with_slice(&QUAD, &[0u16, 1, 2, 2, 3, 0] as &[u16]);
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

  let mut encoder: gfx::Encoder<_, _> = factory.create_command_buffer().into();

  let mut running = true;

  while running {
    event_loop.poll_events(|event| match event {
      Event::WindowEvent { event, .. } => match event {
        WindowEvent::CloseRequested | WindowEvent::Destroyed => running = false,
        WindowEvent::KeyboardInput {
          input:
            KeyboardInput {
              virtual_keycode: Some(VirtualKeyCode::Escape),
              ..
            },
          ..
        } => running = false,
        WindowEvent::KeyboardInput {
          input:
            KeyboardInput {
              virtual_keycode: Some(VirtualKeyCode::Add),
              ..
            },
          ..
        } => draw_state.inc_font_size(),
        WindowEvent::KeyboardInput {
          input:
            KeyboardInput {
              virtual_keycode: Some(VirtualKeyCode::Subtract),
              ..
            },
          ..
        } => draw_state.dec_font_size(),
        WindowEvent::Resized(size) => {
          draw_state.resized = true;
          window.resize(size.to_physical(window.get_hidpi_factor()));
          gfx_window_glutin::update_views(
            &window,
            &mut main_color,
            &mut main_depth,
          );
        }
        _ => (),
      },
      _ => (),
    });

    // Purple background
    let background = [0.16078, 0.16471, 0.26667, 1.0];
    encoder.clear(&main_color, background);
    encoder.clear_depth(&main_depth, 1.0);

    let section = Section {
      bounds: (
        width - draw_state.left_padding,
        height - draw_state.status_height(),
      ),
      screen_position: (draw_state.left_padding, 0.0),
      text: include_str!("../testfiles/kilo-dos2.c"),
      color: [0.9, 0.9, 0.9, 1.0],
      scale: Scale::uniform(draw_state.font_size()),
      z: 1.0,
      ..Section::default()
    };

    if draw_state.resized {
      if let Some(glyph) = glyph_brush.glyphs(section).next() {
        if let Some(bounding_box) = glyph.pixel_bounding_box() {
          draw_state.line_height = bounding_box.max.y - bounding_box.min.y;
          draw_state.resized = false;
        }
      }
    }

    let status_section = Section {
      bounds: (width - draw_state.left_padding, draw_state.status_height()),
      screen_position: (
        draw_state.left_padding,
        height - draw_state.status_height(),
      ),
      text: "Status",
      color: [1.0, 1.0, 1.0, 1.0],
      scale: Scale::uniform(draw_state.font_size()),
      z: 0.5,
      ..Section::default()
    };

    glyph_brush.queue(section);
    glyph_brush.queue(status_section);

    glyph_brush.draw_queued(&mut encoder, &main_color, &main_depth)?;

    let status_scale = Matrix4::from_nonuniform_scale(
      1.0,
      draw_state.status_height() / height,
      1.0,
    );
    let y_move = -(( height - draw_state.status_height() ) / draw_state.status_height());
    let status_move = Matrix4::from_translation(Vector3::new(0.0, y_move, 0.0));
    let status_transform = status_scale * status_move;
    let quad_locals = Locals {
      color: STATUS_BG,
      transform: status_transform.into(),
    };
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
