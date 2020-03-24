use crate::buffer::Buffer;
use crate::debug_log::DebugLog;
use crate::gui::gl_renderer::{create_bundle, GlRenderer};
use crate::gui::persist_window_state::PersistWindowState;
use crate::gui::window::Window;
use crate::gui::{ColorFormat, DepthFormat};
use crate::options::Options;
use crate::BIM_VERSION;
use gfx;
use gfx_glyph::GlyphBrushBuilder;
use glam::vec2;
use glutin::dpi::LogicalSize;
use glutin::Api::OpenGl;
use glutin::{ContextBuilder, EventsLoop, GlProfile, GlRequest, Icon, WindowBuilder};
use std::error::Error;
use std::time::Instant;

const XBIM_DEBUG_LOG: &str = ".xbim_debug";

pub fn run(options: Options) -> Result<(), Box<dyn Error>> {
    let debug_log = DebugLog::new(XBIM_DEBUG_LOG);
    debug_log.start()?;
    use crate::config::RunConfig::*;

    let persist_window_state = PersistWindowState::restore();

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
    let _ = debug_log.debugln_timestamped(&format!("DPI: {}", dpi));
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
    debug_log.debugln_timestamped(&format!("OpenGL Version: {:?}", device.get_info().version))?;

    unsafe {
        device.with_gl(|gl| gl.Disable(gfx_gl::FRAMEBUFFER_SRGB));
    }

    gfx_window
        .window()
        .set_position(persist_window_state.logical_position);

    let (window_width, window_height, ..) = main_color.get_dimensions();
    let window_dim = vec2(window_width as f32, window_height as f32); // u16->f32, should we do this?
    debug_log.debugln_timestamped(&format!(
        "window_width: {}, window_height: {}",
        window_width, window_height,
    ))?;

    let quad_bundle = create_bundle(&mut factory, main_color, main_depth);
    let fonts: Vec<&[u8]> = vec![include_bytes!("iosevka-regular.ttf")];

    let glyph_brush = GlyphBrushBuilder::using_fonts_bytes(fonts)
        .initial_cache_size((512, 512))
        .depth_test(gfx::preset::depth::LESS_EQUAL_WRITE)
        .build(factory.clone());

    let encoder: gfx::Encoder<_, _> = factory.create_command_buffer().into();

    let mut renderer = GlRenderer::new(glyph_brush, encoder, device, quad_bundle, window_dim);

    let mut buffer = Buffer::default();
    if let RunOpenFiles(filenames) = &options.run_type {
        buffer.open(&filenames[0])?;
    }

    let mut window = Window::new(
        &mut renderer,
        monitor,
        gfx_window,
        window_dim,
        logical_size,
        28.0,
        dpi,
        buffer,
        persist_window_state,
        debug_log,
        options,
    )?;

    let _default_status_text = format!("bim editor - version {}", BIM_VERSION);

    let mut last_frame_time = Instant::now();

    #[cfg(not(feature = "event-callbacks"))]
    {
        while window.keep_running() {
            let elapsed = last_frame_time.elapsed();
            last_frame_time = Instant::now();
            window.start_frame();

            event_loop.poll_events(|event| {
                let _ = window.update(&mut renderer, event);
            });

            window.update_dt(elapsed);
            window.render(&mut renderer)?;

            window.end_frame();
        }
    }

    #[cfg(feature = "event-callbacks")]
    {
        use glutin::ControlFlow;
        const MAX_FRAME_TIME: std::time::Duration = std::time::Duration::from_millis(33);
        let event_proxy = event_loop.create_proxy();

        std::thread::spawn(move || loop {
            let _ = event_proxy.wakeup();
            std::thread::sleep(MAX_FRAME_TIME);
        });

        event_loop.run_forever(|event| match window.update_and_render(renderer, event) {
            Ok(running) => {
                if running {
                    ControlFlow::Continue
                } else {
                    ControlFlow::Break
                }
            }
            Err(error) => {
                println!("{:?}", error);
                ControlFlow::Break
            }
        });
    }

    Ok(())
}
