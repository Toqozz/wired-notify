use sdl2::sys::SDL_WindowFlags;
use sdl2::render::WindowCanvas;
use sdl2::pixels::Color;
use sdl2::rect::Rect;

use super::sdl::{ SDL2State };

use crate::config::Config;

use winit::WindowBuilder;
use winit::os::unix::WindowBuilderExt;
use winit::os::unix::XWindowType;
use winit::os::unix::WindowExt;
use winit::dpi::LogicalSize;
use winit::dpi::LogicalPosition;
use winit::EventsLoop;
use winit::Window;


pub struct SDL2Window {
    pub winit_window: Window,
    pub canvas: WindowCanvas,
    clear_color: Color,
}

impl SDL2Window {
    pub fn new(sdl: &SDL2State, config: &Config, events_loop: &EventsLoop) -> Result<SDL2Window, String> {
        SDL2Window::new_via_winit(sdl, config, events_loop)
    }

    fn new_via_winit(sdl: &SDL2State, config: &Config, el: &EventsLoop) -> Result<SDL2Window, String> {
        // Hack to avoid dpi scaling -- we just want pixels.
        std::env::set_var("WINIT_HIDPI_FACTOR", "1.0");

        let color = &config.notification.background_color;
        let clear_color = Color::RGBA(color.r, color.g, color.b, color.a);
        let (width, height) = (config.notification.width, config.notification.height);

        // Dropping winit here is legal because we're using `unsafe`, but will cause crashes later.
        let winit_window = WindowBuilder::new()
            .with_dimensions(LogicalSize { width: width as f64, height: height as f64 })
            .with_title("wiry")
            .with_transparency(true)
            .with_always_on_top(true)
            .with_x11_window_type(XWindowType::Utility)
            .with_x11_window_type(XWindowType::Notification)
            .build(el)
            .unwrap();

        winit_window.set_position(LogicalPosition { x: config.notification.x as f64, y: config.notification.y as f64 });

        let xlib_id = winit_window.get_xlib_window().unwrap();
        let sdl2window = unsafe {
            sdl2::sys::SDL_CreateWindowFrom(xlib_id as *mut std::ffi::c_void)
        };

        let window = unsafe {
            sdl2::video::Window::from_ll(sdl.video_subsys.clone(), sdl2window)
        };

        let canvas = window
            .into_canvas()
            .present_vsync()
            .build()
            .map_err(|e| e.to_string())?;

        Ok(Self {
            winit_window,
            canvas,
            clear_color,
        })
    }

    pub fn set_position(&mut self, position: LogicalPosition) {
        self.winit_window.set_position(position);
    }

    pub fn get_rect(&self) -> Rect {
        let size = self.winit_window.get_inner_size().unwrap();

        Rect::new(0, 0, size.width as u32, size.height as u32)
    }

    fn vertical_align_rect(text_rect: Rect, window_rect: Rect) -> Rect {
        let mut dup = text_rect.clone();

        dup.center_on(window_rect.center());
        dup.set_x(0);

        dup
    }

    pub fn draw_text(&mut self, sdl: &SDL2State, config: &Config, text: &str) {
        let texture_creator = self.canvas.texture_creator();

        let font_path = std::path::Path::new("./arial.ttf");
        let font = sdl.ttf_context.load_font(&font_path, 18).unwrap();
        //font.set_style(sdl2::ttf::FontStyle::ITALIC);

        let surface = font.render(text)
            .blended(Color::RGBA(255, 255, 255, 255)).unwrap();

        let texture = texture_creator.create_texture_from_surface(&surface).unwrap();

        self.canvas.set_draw_color(Color::RGBA(255, 255, 255, 255));

        let sdl2::render::TextureQuery { width, height, .. } = texture.query();
        let rect = Rect::new(0, 0, width as u32, height as u32);

        let centered_rect = SDL2Window::vertical_align_rect(rect, self.get_rect());


        //let r = vertical_align_rect(rect, )
        self.canvas.copy(&texture, None, Some(centered_rect)).unwrap();
        self.canvas.present();
    }

    pub fn draw(&mut self) {
        self.canvas.set_draw_color(self.clear_color);
        self.canvas.clear();
        //self.canvas.present();
    }
}
