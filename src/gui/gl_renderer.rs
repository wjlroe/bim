use crate::gui::quad;
use cgmath::{vec3, Matrix4, Vector2};
use gfx::{pso, Encoder};
use gfx_glyph::GlyphBrush;

pub struct GlRenderer<'a> {
    pub glyph_brush: GlyphBrush<'a, gfx_device_gl::Resources, gfx_device_gl::Factory>,
    pub encoder: Encoder<gfx_device_gl::Resources, gfx_device_gl::CommandBuffer>,
    pub device: gfx_device_gl::Device,
    pub quad_bundle:
        pso::bundle::Bundle<gfx_device_gl::Resources, quad::pipe::Data<gfx_device_gl::Resources>>,
    window_dim: Vector2<f32>,
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
            window_dim,
        }
    }

    pub fn resize(&mut self, window_dim: Vector2<f32>) {
        self.window_dim = window_dim;
    }

    pub fn draw_quad(&mut self, color: [f32; 3], position: Vector2<f32>, bounds: Vector2<f32>) {
        let transform = self.transform_for_quad(position, bounds);
        quad::draw(&mut self.encoder, &mut self.quad_bundle, color, transform);
    }

    fn transform_for_quad(&self, position: Vector2<f32>, bounds: Vector2<f32>) -> Matrix4<f32> {
        let quad_scale = Matrix4::from_nonuniform_scale(
            bounds.x / self.window_dim.x,
            bounds.y / self.window_dim.y,
            1.0,
        );
        let x_translate = (position.x / self.window_dim.x) * 2.0 - 1.0;
        let y_translate = -((position.y / self.window_dim.y) * 2.0 - 1.0);
        let quad_translate = Matrix4::from_translation(vec3(x_translate, y_translate, 0.2)); // TODO: is 1.0 correct for Z-translate? Or 0.0?
        quad_translate * quad_scale
    }
}
