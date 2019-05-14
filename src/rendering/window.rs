use sdl2::sys::SDL_WindowFlags;
use sdl2::render::WindowCanvas;
use sdl2::pixels::Color;

use super::sdl::{ SDL2State };

use crate::config::Config;

pub struct SDL2Window {
    pub canvas: WindowCanvas,
    clear_color: Color,

    //pub window: sdl2::video::Window,
    //pub ctx: sdl2::video::GLContext,
}

impl SDL2Window {
    pub fn new(sdl: &SDL2State, config: &Config) -> Result<SDL2Window, String> {
        let (width, height) = (config.notification.width, config.notification.height);
        let color = &config.notification.background_color;
        let clear_color = Color::RGBA(color.r, color.g, color.b, color.a);

        let window = sdl.video_subsys.window("wiry", width, height)
            .resizable()
            // TODO: figure out how to add multiple flags.
            .set_window_flags(SDL_WindowFlags::SDL_WINDOW_UTILITY as u32 | SDL_WindowFlags::SDL_WINDOW_ALWAYS_ON_TOP as u32)
            .opengl()
            .build()
            .map_err(|e| e.to_string())?;

        let canvas = window
            .into_canvas()
            .present_vsync()
            .build()
            .map_err(|e| e.to_string())?;

        let win = Self {
            canvas,
            clear_color,
        };

        Ok(win)

        /*
        let ctx = window.gl_create_context()?;
        gl::load_with(|name| sdl.video_subsys.gl_get_proc_address(name) as *const _);
        debug_assert_eq!(sdl.video_subsys.gl_attr().context_profile(), GLProfile::Core);
        debug_assert_eq!(sdl.video_subsys.gl_attr().context_version(), (3, 3));
        let win = Self {
            window,
            ctx,
        };

        */

    }

    pub fn draw(&mut self) {
        self.canvas.set_draw_color(self.clear_color);
        self.canvas.clear();
        self.canvas.present();
    }
}
