#[macro_use] extern crate gfx;
extern crate gfx_core;
extern crate gfx_device_gl;
extern crate gfx_window_glutin;
extern crate glutin;
extern crate nalgebra;
extern crate lyon;

use gfx::Device;
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
        normal: [f32; 2] = "v_normal",
        color: [f32; 4] = "v_color",
    }

    pipeline pipe {
        vbuf: gfx::VertexBuffer<Vertex> = (),
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
    let (vertex_buffer, slice) = glutin_window.factory.create_vertex_buffer_with_slice(&geometry.vertices.as_slice(), geometry.indices.as_slice());

    let data = pipe::Data {
        vbuf: vertex_buffer,
        proj: ortho.to_homogeneous().into(),
        out: glutin_window.render_target,
    };

    // Mouse struct to wrap some convenient info.
    let mut mouse = Mouse { delta: (0.0, 0.0), down: false };

    // Run until manual intervention.
    let mut running = true;
    while running {
        glutin_window.events_loop.poll_events(|event| {
            if let glutin::Event::WindowEvent { event, .. } = &event {
                match event {
                    CloseRequested => running = false,
                    MouseInput { state, .. } => mouse.update_press(state),
                    _ => {}
                }
            }
        });

        encoder.clear(&data.out, BLACK);
        encoder.draw(&slice, &pso, &data);
        encoder.flush(&mut glutin_window.device);
        glutin_window.context.swap_buffers().unwrap();
        glutin_window.device.cleanup();
    }
}
