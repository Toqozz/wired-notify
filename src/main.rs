#[macro_use] extern crate gfx;
extern crate gfx_core;
extern crate gfx_device_gl;
extern crate gfx_window_glutin;
extern crate glutin;
extern crate nalgebra;
extern crate lyon;
extern crate rusttype;

use rusttype::*;
use rusttype::gpu_cache::*;

use gfx::Device;
use gfx::Factory as gFactory;
use gfx::traits::FactoryExt;
use glutin::os::unix::WindowBuilderExt;
use gfx_device_gl::{Device as glDevice, Factory, Resources};
use gfx::handle::{RenderTargetView, DepthStencilView};
use gfx::format::{Srgba8, DepthStencil};
use glutin::{WindowedContext, EventsLoop};
use glutin::WindowEvent::*;
use nalgebra::geometry::Orthographic3;
//use nalgebra::base::*;

use lyon::math::rect;
use lyon::tessellation::{ VertexBuffers, FillOptions, FillVertex };
use lyon::tessellation::basic_shapes::*;
use lyon::tessellation::geometry_builder::VertexConstructor;
use lyon::tessellation::BuffersBuilder;

gfx_defines! {
    vertex Vertex {
        position: [f32; 2] = "v_pos",
        texcoords: [f32; 2] = "v_texcoords",
        normal: [f32; 2] = "v_normal",
        color: [f32; 4] = "v_color",
    }

    pipeline pipe {
        vbuf: gfx::VertexBuffer<Vertex> = (),
        font: gfx::TextureSampler<[f32; 4]> = "t_font",
        proj: gfx::Global<[[f32; 4]; 4]> = "u_proj",
        out: gfx::RenderTarget<gfx::format::Srgba8> = "out_color",
    }
}

const WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const BLACK: [f32; 4] = [0.0, 0.0, 0.0, 1.0];
const GREEN: [f32; 4] = [0.0, 1.0, 0.0, 1.0];

struct GLWindow {
    context: WindowedContext,
    events_loop: EventsLoop,
    device: glDevice,
    factory: Factory,
    render_target: RenderTargetView<Resources, Srgba8>,
    depth_target: DepthStencilView<Resources, DepthStencil>
}

impl GLWindow {
    fn build_window() -> GLWindow {
        // Events loop to caputer window events (clicked, moved, resized, etc).
        let events_loop = glutin::EventsLoop::new();

        // Initialize a window and context but don't build them yet.
        let window_builder = glutin::WindowBuilder::new()
            .with_title("yarn")
            .with_class("yarn2".to_owned(), "yarn2".to_owned())
            .with_transparency(true)
            .with_always_on_top(true)
            .with_x11_window_type(glutin::os::unix::XWindowType::Utility);
        let context_builder = glutin::ContextBuilder::new()
            .with_vsync(true);

        // Build the window using the glutin backend for gfx-rs.
        // window -- obvious, device -- rendering device, factory -- creation?, color_view -- base
        // color, depth_view -- ?
        let (window, device, factory, color_view, depth_view) =
            gfx_window_glutin::init::<Srgba8, DepthStencil>(window_builder, context_builder, &events_loop)
                .expect("Failed to create a window.");

        GLWindow { context: window, events_loop, device, factory, render_target: color_view, depth_target: depth_view }
    }
}


struct VertexCtor;
impl VertexConstructor<FillVertex, Vertex> for VertexCtor {
    fn new_vertex(&mut self, vertex: FillVertex) -> Vertex {
        Vertex {
            position: vertex.position.to_array(),
            normal: vertex.normal.to_array(),
            texcoords: [0.0, 0.0],
            color: WHITE,
        }
    }
}

struct VertexCtor2;
impl VertexConstructor<FillVertex, Vertex> for VertexCtor2 {
    fn new_vertex(&mut self, vertex: FillVertex) -> Vertex {
        Vertex {
            position: vertex.position.to_array(),
            normal: vertex.normal.to_array(),
            texcoords: [0.0, 0.0],
            color: GREEN,
        }
    }
}


#[derive(Debug)]
struct Mouse {
    delta: (f64, f64),
    down: bool,
}

impl Mouse {
    fn update_press(&mut self, state: &glutin::ElementState) {
        self.down = &glutin::ElementState::Pressed == state;
    }
}

fn main() {
    let font_data = include_bytes!("../arial.ttf");
    let font = Font::from_bytes(font_data as &[u8]).expect("UHHH");

    let scale = Scale::uniform(32.0);
    let text = "Hello world and goodbye world.";
    let color = [1.0, 0.0, 0.0, 1.0];

    let v_metrics = font.v_metrics(scale);

    let glyphs: Vec<_> = font.layout(text, scale, point(20.0, 20.0 + v_metrics.ascent)).collect();


    // Window constructor.
    let mut glutin_window = GLWindow::build_window();

    // TODO: maybe move these to GLWindow.
    // Using an encoder avoids having to use raw OpenGL procedures.
    let mut encoder: gfx::Encoder<_, _> = glutin_window.factory.create_command_buffer().into();

    // To my understanding, pipeline state objects essentially batch shader commands.
    // TODO: better explanation.
    let pso = glutin_window.factory.create_pipeline_simple(
        include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/shaders/base.glslv")),
        include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/shaders/base.glslf")),
        pipe::new()
    ).unwrap();

    /*
    let (cache_tex, cache_tex_view) = {
        let w = 256 as u16;
        let h = 256 as u16;
        let data = &[0; (256 * 256 * 4) as usize];
        let kind = gfx::texture::Kind::D2(w, h, gfx::texture::AaMode::Single);
        glutin_window.factory.create_texture_immutable_u8::<(gfx::format::R8_G8_B8_A8, gfx::format::Unorm)>(
            kind,
            gfx::texture::Mipmap::Allocated,
            &[data]
        ).unwrap()
    };
    */

    let cache_texture = glutin_window.factory.create_texture::<gfx::format::R8_G8_B8_A8>(
        gfx::texture::Kind::D2(256, 256, gfx::texture::AaMode::Single),
        1,
        gfx::memory::Bind::all(),
        gfx::memory::Usage::Dynamic,
        Some(gfx::format::ChannelType::Srgb),
    ).expect("Couldn't create a texture.");

    let view_texture = glutin_window.factory.view_texture_as_shader_resource::<gfx::format::Rgba8> (
            &cache_texture,
            (0, 0),
            gfx::format::Swizzle::new()
        );


    let mut cache = Cache::builder().build();

    for g in glyphs.iter() {
        cache.queue_glyph(0, g.clone());
    }

    cache.cache_queued(|rect, data| {
        let info = gfx::texture::ImageInfoCommon {
            xoffset: rect.min.x as u16,
            yoffset: rect.min.y as u16,
            zoffset: 0,
            width: rect.width() as u16,
            height: rect.height() as u16,
            depth: 0,
            format: (),
            mipmap: 0,
        };

        let mut newdat = Vec::new();
        let mut i = 0;
        while i < data.len() {
            newdat.push([0,0,0,data[i]]);
            i+=1;
        }

        encoder.update_texture::<gfx::format::R8_G8_B8_A8, (gfx::format::R8_G8_B8_A8, gfx::format::Unorm)>(
            &cache_texture,
            None,
            info,
            newdat.as_slice(),
        ).expect("nup");
    }).expect("fail");

    let mut geometry: VertexBuffers<Vertex, u16> = VertexBuffers::new();
    let options = FillOptions::tolerance(0.01);

    /*
    let mut buffer = BuffersBuilder::new(&mut geometry, |vertex: FillVertex| {
        Vertex {
            position: vertex.position.to_array(),
            normal: vertex.normal.to_array(),
            color: WHITE,
        }
    });
    */
    fill_rounded_rectangle(
        &rect(0.0, 0.0, 640.0, 720.0),
        &BorderRadii {
            top_left: 50.0,
            top_right: 50.0,
            bottom_left: 50.0,
            bottom_right: 50.0,
        },
        &options,
        &mut BuffersBuilder::new(&mut geometry, VertexCtor),
    ).expect("Could not build rectangle.");

    fill_rounded_rectangle(
        &rect(500.0, 0.0, 640.0, 720.0),
        &BorderRadii {
            top_left: 50.0,
            top_right: 50.0,
            bottom_left: 50.0,
            bottom_right: 50.0,
        },
        &options,
        &mut BuffersBuilder::new(&mut geometry, VertexCtor2),
    ).expect("Could not build rectangle.");

    let ortho = Orthographic3::new(0.0, 1280.0, 0.0, 720.0, -1.0, 1.0);

    // Create vertex buffer and slice from supplied vertices.
    // A slice dictates what and in what order vertices are processed.
    //let (vertex_buffer, slice) = glutin_window.factory.create_vertex_buffer_with_slice(&geometry.vertices.as_slice(), geometry.indices.as_slice());
    let sampler = glutin_window.factory.create_sampler_linear();
    let (vertex_buffer, slice) = glutin_window.factory.create_vertex_buffer_with_slice(Vec::<Vertex>::new().as_slice(), Vec::<u16>::new().as_slice());

    let mut data = pipe::Data {
        vbuf: vertex_buffer,
        font: (view_texture.unwrap(), sampler),
        proj: ortho.to_homogeneous().into(),
        out: glutin_window.render_target,
    };

    // Mouse struct to wrap some convenient info.
    let mut mouse = Mouse { delta: (0.0, 0.0), down: false };

    // Run until manual intervention.
    let mut running = true;
    while running {
        let mut vertices = Vec::<Vertex>::new();
        let mut indices = Vec::<u16>::new();
        let mut index = 0;

        for g in glyphs.iter() {
            let rect = cache.rect_for(0, g);
            //dbg!(&rect);
            let (uv_rect, screen_rect) = match rect {
                Ok(Some(r)) => r,
                _ => continue,
            };

            let bounds = g.pixel_bounding_box().unwrap();

            //let bounds = bounds.unwrap();
            vertices.push(
                Vertex {
                    position: [bounds.min.x as f32, bounds.max.y as f32],
                    texcoords: [uv_rect.min.x, uv_rect.max.y],
                    normal: [0.0, 0.0],
                    color,
                });
            vertices.push(
                Vertex {
                    position: [bounds.min.x as f32, bounds.min.y as f32],
                    texcoords: [uv_rect.min.x, uv_rect.min.y],
                    normal: [0.0, 0.0],
                    color,
                });
            vertices.push(
                Vertex {
                    position: [bounds.max.x as f32, bounds.min.y as f32],
                    texcoords: [uv_rect.max.x, uv_rect.min.y],
                    normal: [0.0, 0.0],
                    color,
                });
            vertices.push(
                Vertex {
                    position: [bounds.max.x as f32, bounds.min.y as f32],
                    texcoords: [uv_rect.max.x, uv_rect.min.y],
                    normal: [0.0, 0.0],
                    color,
                });
            vertices.push(
                Vertex {
                    position: [bounds.max.x as f32, bounds.max.y as f32],
                    texcoords: [uv_rect.max.x, uv_rect.max.y],
                    normal: [0.0, 0.0],
                    color,
                });
            vertices.push(
                Vertex {
                    position: [bounds.min.x as f32, bounds.max.y as f32],
                    texcoords: [uv_rect.min.x, uv_rect.max.y],
                    normal: [0.0, 0.0],
                    color,
                });

            indices.push(index + 0);
            indices.push(index + 1);
            indices.push(index + 2);
            indices.push(index + 3);
            indices.push(index + 4);
            indices.push(index + 5);

            index += 6;
        };

        let (vertex_buffer, slice) = glutin_window.factory.create_vertex_buffer_with_slice(vertices.as_slice(), indices.as_slice());

        glutin_window.events_loop.poll_events(|event| {
            if let glutin::Event::WindowEvent { event, .. } = &event {
                match event {
                    CloseRequested => running = false,
                    MouseInput { state, .. } => mouse.update_press(state),
                    _ => {}
                }
            }
        });

        data.vbuf = vertex_buffer;

        encoder.clear(&data.out, BLACK);
        encoder.draw(&slice, &pso, &data);
        encoder.flush(&mut glutin_window.device);
        glutin_window.context.swap_buffers().unwrap();
        glutin_window.device.cleanup();
    }
}
