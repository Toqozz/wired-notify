extern crate sdl2;
extern crate winit;
//extern crate gl;

mod rendering;
mod notification;
mod bus;
mod config;

use std::sync::mpsc;

use winit::EventsLoop;

use notification::management::NotifyWindowManager;
use bus::dbus::Notification;

fn spawn_window(notification: Notification, manager: &mut NotifyWindowManager, el: &EventsLoop) {
    manager.new_notification(notification, el);
}

fn main() {
/*
    //let texture_creator = window.canvas.texture_creator();
    //let font_path = std::path::Path::new("./arial.ttf");
    //let font = sdl.ttf_context.load_font(&font_path, 32).unwrap();
    //font.set_style(sdl2::ttf::FontStyle::BOLD);

    // render a surface and convert it to a texture bound to the canvas.
    //let surface = font.render("Hello world!")
        //.blended(Color::RGBA(255, 255, 255, 255)).unwrap();
    //let texture = texture_creator.create_texture_from_surface(&surface).unwrap();

    //let sdl2::render::TextureQuery { width, height, .. } = texture.query();
    //window.canvas.copy(&texture, None, Some(sdl2::rect::Rect::new(640 as i32, 360 as i32, width as u32, height as u32))).unwrap();
    //window.canvas.present();
*/
    // TODO: maybe use EventsLoop::new_x11();
    // winit events loop.
    let mut events_loop = EventsLoop::new();


    // Load config.
    let config: config::Config = toml::from_str(include_str!("config.toml"))
        .expect("Failed to load config.\n");

    let mut manager = NotifyWindowManager::new(&config);

    let (sender, receiver) = mpsc::channel();
    let connection = bus::dbus::dbus_loop(sender);


    let mut running = true;
    while running {
        events_loop.poll_events(|event| {
            match event {
                winit::Event::WindowEvent {
                    event: winit::WindowEvent::CloseRequested,
                    ..
                } => running = false,
                winit::Event::WindowEvent {
                    window_id,
                    // NOTE: can use modifiers here, like ctrl, shift, etc.
                    event: winit::WindowEvent::MouseInput { .. },
                } => {
                    println!("got mouse input, dropping a window.");
                    manager.drop_window(window_id);
                },
                _ => {}
            }
        });

        manager.draw_windows();

        // Check dbus signals.
        let signal = connection.incoming(0).next();
        if let Some(s) = signal {
            dbg!(s);
        }

        if let Ok(x) = receiver.try_recv() {
            spawn_window(x, &mut manager, &events_loop);
        }
    }
}
