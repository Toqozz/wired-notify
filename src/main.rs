extern crate sdl2;
//extern crate gl;

mod rendering;
mod notification;
mod bus;
mod config;

use std::sync::mpsc;

use sdl2::{
    event::Event,
    keyboard::Keycode,
};


use notification::management::NotifyWindowManager;
use bus::dbus::Notification;

fn spawn_window(notification: Notification, manager: &mut NotifyWindowManager) {
    manager.new_notification(notification);
}

fn main() {
/*
    let mut sdl = SDL2State::new()
        .expect("Failed to create sdl state.");
    let mut window = SDL2Window::new(&sdl)
        .expect("Failed to create a new window.");
    let mut window2 = SDL2Window::new(&sdl)
        .expect("Failed to create a new window.");
    // Clear canvas before rendering.
    window.canvas.set_draw_color(Color::RGB(0, 0, 0));
    window.canvas.clear();
    window.canvas.present();
*/

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

    // Load config.
    let config: config::Config = toml::from_str(include_str!("config.toml"))
        .expect("Failed to load config.\n");

    let (mut manager, mut event_pump) = NotifyWindowManager::new(&config);

    let (sender, receiver) = mpsc::channel();
    let connection = bus::dbus::dbus_loop(sender);


    'main: loop {
        for event in event_pump.poll_iter() {
            match event {
                // This is called on ^C.
                Event::Quit { .. } => break 'main,
                Event::KeyDown { keycode: Some(Keycode::Escape), window_id, .. } => manager.drop_window(window_id),
                //Event::MouseButtonDown {x, y, ..} => {
                //}
                _ => {}
            }
        }

        manager.draw_windows();

        // Check dbus signals.
        let signal = connection.incoming(0).next();
        if let Some(s) = signal {
            dbg!(s);
        }

        if let Ok(x) = receiver.try_recv() {
            spawn_window(x, &mut manager);
        }

        //std::thread::sleep(std::time::Duration::new(0, 1_000_000_000u32 / 60));

        // Clear frame.
        //canvas.set_draw_color(Color::RGB(0, 0, 0));
        //canvas.clear();
        //canvas.present();
    }
}
