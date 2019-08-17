use crate::gui::{ColorFormat, DepthFormat};
use cgmath::Matrix4;
use gfx;
use gfx::handle::{DepthStencilView, RenderTargetView};
use gfx::traits::FactoryExt;
use gfx::*;

const QUAD: [Vertex; 4] = [
    Vertex { pos: [-1.0, 1.0] },
    Vertex { pos: [-1.0, -1.0] },
    Vertex { pos: [1.0, -1.0] },
    Vertex { pos: [1.0, 1.0] },
];
const QUAD_INDICES: [u16; 6] = [0u16, 1, 2, 2, 3, 0];

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

pub fn create_bundle<R: Resources, F: FactoryExt<R>>(
    factory: &mut F,
    main_color: RenderTargetView<R, ColorFormat>,
    main_depth: DepthStencilView<R, DepthFormat>,
) -> Bundle<R, pipe::Data<R>> {
    let quad_pso = factory
        .create_pipeline_simple(
            include_bytes!("shaders/quad_150_core.vert"),
            include_bytes!("shaders/quad_150_core.frag"),
            pipe::new(),
        )
        .expect("quad pso construction to work");
    let (quad_vbuf, quad_slice) =
        factory.create_vertex_buffer_with_slice(&QUAD, &QUAD_INDICES as &[u16]);
    let data = pipe::Data {
        vbuf: quad_vbuf,
        locals: factory.create_constant_buffer(2),
        out_color: main_color,
        out_depth: main_depth,
    };
    pso::bundle::Bundle::new(quad_slice, quad_pso, data)
}

pub fn draw<R, C>(
    encoder: &mut Encoder<R, C>,
    quad_bundle: &mut Bundle<R, pipe::Data<R>>,
    color: [f32; 3],
    transform: Matrix4<f32>,
) where
    R: Resources,
    C: CommandBuffer<R>,
{
    let locals = Locals {
        color,
        transform: transform.into(),
    };
    encoder.update_constant_buffer(&quad_bundle.data.locals, &locals);
    quad_bundle.encode(encoder);
}
