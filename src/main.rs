extern crate sdl2;
extern crate winit;
//extern crate gl;

mod rendering;
mod notification;
mod bus;
mod config;

use sdl2::event::Event;
use rendering::sdl::SDL2State;

use std::sync::mpsc;

use winit::EventsLoop;

use notification::management::NotifyWindowManager;
use bus::dbus::Notification;

fn spawn_window(notification: Notification, manager: &mut NotifyWindowManager, el: &EventsLoop) {
    manager.new_notification(notification, el);
}

fn main() -> Result<(), String> {
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
    let sdl = SDL2State::new()?;

    /* We need 2 event receivers.  One for our program (receives SIGTERM) and one for the windows.
     * This would not be necessary if we could use purely SDL2 for windowing, but unfortunately we
     * can't just yet: https://bugzilla.libsdl.org/show_bug.cgi?id=4630 */
    let mut events_pump =  sdl.context.event_pump()?;
    let mut events_loop = EventsLoop::new();    // TODO: maybe use `EventsLoop::new_x11()` ?

    let config: config::Config = toml::from_str(include_str!("config.toml"))
        .expect("Failed to load config.\n");

    let mut manager = NotifyWindowManager::new(&config, &sdl);

    // Allows us to receive messages from dbus.
    let (sender, receiver) = mpsc::channel();
    let connection = bus::dbus::dbus_loop(sender);


    let mut running = true;
    while running {
        for event in events_pump.poll_iter() {
            match event {
                Event::Quit { .. } => running = false,
                _ => {}
            }
        }

        events_loop.poll_events(|event| {
            match event {
                winit::Event::WindowEvent {
                    window_id,
                    event: winit::WindowEvent::MouseInput { .. },   // NOTE: Can use modifiers here, like ctrl, shift, alt.
                } => manager.drop_window(window_id),
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

        // Roughly 60fps.
        std::thread::sleep(std::time::Duration::from_millis(1000 / 60));
    }

    Ok(())
}
