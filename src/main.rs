#[macro_use] extern crate gfx;
extern crate gfx_core;
extern crate gfx_device_gl;
extern crate gfx_window_glutin;
extern crate glutin;
extern crate nalgebra;
extern crate lyon;
extern crate gfx_glyph;

use std::collections::HashMap;
use std::sync::mpsc::channel;
use std::thread;

use gfx::traits::FactoryExt;
use nalgebra::geometry::Orthographic3;

use lyon::math::rect;
use lyon::tessellation::{ VertexBuffers, FillOptions, FillVertex };
use lyon::tessellation::basic_shapes::*;
use lyon::tessellation::geometry_builder::VertexConstructor;
use lyon::tessellation::BuffersBuilder;

mod bus;
mod rendering;
use rendering::window;

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

const GREEN: [f32; 4] = [0.0, 1.0, 0.0, 1.0];
const WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];

struct VertexCtor;
impl VertexConstructor<FillVertex, rendering::window::Vertex> for VertexCtor {
    fn new_vertex(&mut self, vertex: FillVertex) -> rendering::window::Vertex {
        let vert = [vertex.position.x, vertex.position.y, 0.0];
        rendering::window::Vertex {
            position: vert,
            normal: vertex.normal.to_array(),
            texcoords: [0.0, 0.0],
            color: WHITE,
        }
    }
}

struct VertexCtor2;
impl VertexConstructor<FillVertex, rendering::window::Vertex> for VertexCtor2 {
    fn new_vertex(&mut self, vertex: FillVertex) -> rendering::window::Vertex {
        let vert = [vertex.position.x, vertex.position.y, 0.0];
        rendering::window::Vertex {
            position: vert,
            normal: vertex.normal.to_array(),
            texcoords: [0.0, 0.0],
            color: GREEN,
        }
    }
}


pub fn spawn_window<'a>(name: &'a str, map: &mut HashMap<glutin::WindowId, window::GLWindow<'a>>) {
    let (mut window, _) = window::GLWindow::build_window();
    window.set_text(name);

    map.insert(window.context.window().id(), window);
}

fn main() {
    // Window constructor.
    let (mut gl_window, mut events_loop) = window::GLWindow::build_window();




    let mut geometry: VertexBuffers<rendering::window::Vertex, u16> = VertexBuffers::new();
    let options = FillOptions::tolerance(0.01);
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
    let (vertex_buffer, slice) = gl_window.factory.create_vertex_buffer_with_slice(&geometry.vertices.as_slice(), geometry.indices.as_slice());
    let data = rendering::window::pipe::Data {
        vbuf: vertex_buffer,
        //font: (glyph_brush.into, sampler),
        proj: ortho.to_homogeneous().into(),
        out: gl_window.render_target.clone(),
        out_depth: gl_window.depth_target.clone(),
    };

    gl_window.data = Some(data);
    gl_window.set_slice(slice);

    gl_window.set_text("Hello world!");





    // Loop through dbus messages.
    let (sender, receiver) = channel();
    let _handler = thread::spawn(move || {
        bus::dbus::dbus_loop(sender, receiver);
    });

    let mut windows: HashMap<glutin::WindowId, window::GLWindow> = HashMap::new();
    windows.insert(gl_window.context.window().id(), gl_window);


    loop {
        events_loop.poll_events(|event| {
            match event {
                glutin::Event::WindowEvent {
                    event: glutin::WindowEvent::CloseRequested,
                    window_id,
                } => {
                    let window = windows.remove(&window_id).unwrap();
                    drop(window);
                }
                _ => (),
            }
        });

        for (_id, window) in windows.iter_mut() {
            window.draw();
        }
    }

        /*
        for i in kill_list {
            dbg!(i);
            let w = window_list.get_mut(i);
            drop(w.unwrap());
        }
    }
        */


    // Run until manual intervention.
    /*
    while running {
        gl_window.events_loop.poll_events(|event| {

            if let glutin::Event::WindowEvent { event, .. } = &event {
                match event {
                    CloseRequested => running = false,
                    Resized(size) => resize = Some(size.clone()),
                    _ => {}
                }
            }
        });

        if let Some(size) = resize {
            gl_window.resize(&size);
            resize = None;
        }

        //gl_window.resize(size);
        gl_window.draw();
    }
    */
}
