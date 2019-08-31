use crate::buffer::Buffer;
use crate::config::RunConfig;
use crate::debug_log::DebugLog;
use crate::editor::BIM_VERSION;
use crate::gui::persist_window_state::PersistWindowState;
use crate::gui::quad;
use crate::gui::window::Window;
use crate::gui::{ColorFormat, DepthFormat};
use gfx;
use gfx_glyph::GlyphBrushBuilder;
use glutin::dpi::LogicalSize;
use glutin::Api::OpenGl;
use glutin::{ContextBuilder, EventsLoop, GlProfile, GlRequest, Icon, WindowBuilder};
use std::error::Error;

const XBIM_DEBUG_LOG: &str = ".xbim_debug";

pub fn run(run_type: RunConfig) -> Result<(), Box<dyn Error>> {
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

    unsafe {
        device.with_gl(|gl| gl.Disable(gfx_gl::FRAMEBUFFER_SRGB));
    }

    gfx_window
        .window()
        .set_position(persist_window_state.logical_position);

    let (window_width, window_height, ..) = main_color.get_dimensions();
    debug_log.debugln_timestamped(&format!(
        "window_width: {}, window_height: {}",
        window_width, window_height,
    ))?;

    let quad_bundle = quad::create_bundle(&mut factory, main_color, main_depth);
    let fonts: Vec<&[u8]> = vec![include_bytes!("iosevka-regular.ttf")];

    let glyph_brush = GlyphBrushBuilder::using_fonts_bytes(fonts)
        .initial_cache_size((512, 512))
        .depth_test(gfx::preset::depth::LESS_EQUAL_WRITE)
        .build(factory.clone());

    let encoder: gfx::Encoder<_, _> = factory.create_command_buffer().into();

    let mut buffer = Buffer::default();
    if let RunOpenFile(ref filename) = run_type {
        buffer.open(filename)?;
    }

    let mut window = Window::new(
        monitor,
        gfx_window,
        logical_size,
        dpi,
        window_width.into(),
        window_height.into(),
        18.0,
        dpi,
        buffer,
        persist_window_state,
        debug_log,
        glyph_brush,
        device,
        encoder,
        quad_bundle,
    );

    let _default_status_text = format!("bim editor - version {}", BIM_VERSION);

    #[cfg(feature = "event-polling")]
    {
        while window.keep_running() {
            window.start_frame();

            event_loop.poll_events(|event| {
                let _ = window.update(event);
            });

            window.render()?;

            window.end_frame();
        }
    }

    #[cfg(not(feature = "event-polling"))]
    {
        use glutin::ControlFlow;
        const MAX_FRAME_TIME: Duration = std::time::Duration::from_millis(33);
        let event_proxy = event_loop.create_proxy();

        std::thread::spawn(move || loop {
            let _ = event_proxy.wakeup();
            std::thread::sleep(MAX_FRAME_TIME);
        });

        event_loop.run_forever(|event| match window.update_and_render(event) {
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
