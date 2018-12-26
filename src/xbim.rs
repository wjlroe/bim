use bim::config::RunConfig;
use gfx::{format, Device};
use gfx_glyph::{GlyphBrushBuilder, Scale, Section};
use glutin::{
  ContextBuilder, Event, EventsLoop, KeyboardInput, VirtualKeyCode,
  WindowBuilder, WindowEvent,
};
use std::{env, error::Error};

fn run(_run_type: RunConfig) -> Result<(), Box<dyn Error>> {
  let mut event_loop = EventsLoop::new();
  let window_builder = WindowBuilder::new()
    .with_title("bim")
    .with_dimensions((400, 600).into());
  let context = ContextBuilder::new().with_vsync(true);
  let (window, mut device, mut factory, mut main_color, mut main_depth) =
    gfx_window_glutin::init::<format::Rgba8, format::Depth>(
      window_builder,
      context,
      &event_loop,
    )
    .unwrap();

  unsafe {
    device.with_gl(|gl| gl.Disable(gfx_gl::FRAMEBUFFER_SRGB));
  }

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
        _ => (),
      },
      _ => (),
    });

    // Purple background
    let background = [0.16078, 0.16471, 0.26667, 1.0];
    encoder.clear(&main_color, background);
    encoder.clear_depth(&main_depth, 1.0);

    let (width, height, ..) = main_color.get_dimensions();
    let (width, height) = (f32::from(width), f32::from(height));

    glyph_brush.queue(Section {
      bounds: (width, height),
      text: include_str!("../testfiles/kilo-dos2.c"),
      color: [0.9, 0.9, 0.9, 1.0],
      scale: Scale::uniform(30.0),
      z: 1.0,
      ..Section::default()
    });

    glyph_brush.draw_queued(&mut encoder, &main_color, &main_depth)?;

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
