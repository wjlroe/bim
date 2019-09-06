use crate::gui::quad;
use gfx::{pso, Encoder};
use gfx_glyph::GlyphBrush;

pub struct GlRenderer<'a> {
    pub glyph_brush: GlyphBrush<'a, gfx_device_gl::Resources, gfx_device_gl::Factory>,
    pub encoder: Encoder<gfx_device_gl::Resources, gfx_device_gl::CommandBuffer>,
    pub device: gfx_device_gl::Device,
    pub quad_bundle:
        pso::bundle::Bundle<gfx_device_gl::Resources, quad::pipe::Data<gfx_device_gl::Resources>>,
}

impl<'a> GlRenderer<'a> {
    pub fn new(
        glyph_brush: GlyphBrush<'a, gfx_device_gl::Resources, gfx_device_gl::Factory>,
        encoder: Encoder<gfx_device_gl::Resources, gfx_device_gl::CommandBuffer>,
        device: gfx_device_gl::Device,
        quad_bundle: pso::bundle::Bundle<
            gfx_device_gl::Resources,
            quad::pipe::Data<gfx_device_gl::Resources>,
        >,
    ) -> Self {
        Self {
            glyph_brush,
            encoder,
            device,
            quad_bundle,
        }
    }
}
