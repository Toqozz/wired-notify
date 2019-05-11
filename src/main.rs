#[macro_use] extern crate gfx;
extern crate gfx_core;
extern crate gfx_device_gl;
extern crate gfx_window_glutin;
extern crate glutin;
extern crate nalgebra;
extern crate lyon;
extern crate gfx_glyph;

use std::sync::mpsc;


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

pub fn spawn_window(notification: bus::dbus::Notification, el: &glutin::EventsLoop, factory: &mut window::WindowFactory) {
    let window = factory.create_window(el);
    window.set_notification(notification);
    window.draw_rectangle();
}

fn main() {
    let (mut window_factory, mut events_loop) = window::WindowFactory::new();

    let gl_window = window_factory.create_window(&events_loop);
    gl_window.set_text("Hello world!".to_string());
    gl_window.draw_rectangle();

    // Loop through dbus messages.
    let (sender, receiver) = mpsc::channel();
    let connection = bus::dbus::dbus_loop(sender);


    loop {
        // Poll window events.
        events_loop.poll_events(|event| {
            match event {
                glutin::Event::WindowEvent {
                    event: glutin::WindowEvent::CloseRequested,
                    window_id,
                } => {
                    let window = window_factory.window_map.remove(&window_id)
                        .expect("Trying to drop a window that doesn't exist in the window list.");
                    drop(window);
                }
                _ => (),
            }
        });

        // Draw windows.
        for (_id, window) in window_factory.window_map.iter_mut() {
            window.draw();
        }

        // Check dbus signals.
        // TODO: do this somewhere else.
        let signal = connection.incoming(0).next();
        if let Some(s) = signal {
            dbg!(s);
        }

        if let Ok(x) = receiver.try_recv() {
            spawn_window(x, &events_loop, &mut window_factory);
        }
    }
}
