use gfx;
use lyon::tessellation::FillVertex;
use lyon::tessellation::geometry_builder::VertexConstructor;


gfx_defines! {
    vertex Vertex {
        position: [f32; 3] = "v_pos",
        texcoords: [f32; 2] = "v_texcoords",
        normal: [f32; 2] = "v_normal",
        color: [f32; 4] = "v_color",
    }

    pipeline pipe {
        vbuf: gfx::VertexBuffer<Vertex> = (),
        //font: gfx::TextureSampler<[f32; 4]> = "t_font",
        proj: gfx::Global<[[f32; 4]; 4]> = "u_proj",
        out: gfx::RenderTarget<gfx::format::Srgba8> = "out_color",
        out_depth: gfx::DepthTarget<gfx::format::DepthStencil> = gfx::preset::depth::LESS_EQUAL_WRITE,
    }
}

const BLACK: [f32; 4] = [0.0, 0.0, 0.0, 1.0];

struct VertexCtor {
    color: [f32; 4],
}

impl VertexConstructor<FillVertex, Vertex> for VertexCtor {
    fn new_vertex(&mut self, vertex: FillVertex) -> Vertex {
        let vert = [vertex.position.x, vertex.position.y, 0.0];
        Vertex {
            position: vert,
            normal: vertex.normal.to_array(),
            texcoords: [0.0, 0.0],
            color: self.color,
        }
    }
}

