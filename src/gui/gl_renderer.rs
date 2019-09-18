use crate::gui::quad;
use crate::gui::rect::Rect;
use crate::gui::transforms::Transforms;
use cgmath::Vector2;
use gfx::{pso, Encoder};
use gfx_glyph::GlyphBrush;

pub struct GlRenderer<'a> {
    pub glyph_brush: GlyphBrush<'a, gfx_device_gl::Resources, gfx_device_gl::Factory>,
    pub encoder: Encoder<gfx_device_gl::Resources, gfx_device_gl::CommandBuffer>,
    pub device: gfx_device_gl::Device,
    pub quad_bundle:
        pso::bundle::Bundle<gfx_device_gl::Resources, quad::pipe::Data<gfx_device_gl::Resources>>,
    transforms: Transforms,
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
        window_dim: Vector2<f32>,
    ) -> Self {
        Self {
            glyph_brush,
            encoder,
            device,
            quad_bundle,
            transforms: Transforms::new(window_dim),
        }
    }

    pub fn resize(&mut self, window_dim: Vector2<f32>) {
        self.transforms.window_dim = window_dim;
    }

    pub fn draw_quad(&mut self, color: [f32; 3], rect: Rect, z: f32) {
        let transform = self.transforms.transform_for_quad(rect, z);
        quad::draw(&mut self.encoder, &mut self.quad_bundle, color, transform);
    }
}
