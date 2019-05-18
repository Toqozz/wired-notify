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


pub struct SDL2Window<'a> {
    pub winit_window: Window,
    pub canvas: WindowCanvas,
    clear_color: Color,

    config: &'a Config,
}

impl<'a> SDL2Window<'a> {
    pub fn new(sdl: &SDL2State, config: &'a Config, events_loop: &EventsLoop) -> Result<SDL2Window<'a>, String> {
        SDL2Window::new_via_winit(sdl, config, events_loop)
    }

    fn new_via_winit(sdl: &SDL2State, config: &'a Config, el: &EventsLoop) -> Result<SDL2Window<'a>, String> {
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
            config,
        })
    }

    pub fn set_position(&mut self, position: LogicalPosition) {
        self.winit_window.set_position(position);
    }

    pub fn get_rect(&self) -> Rect {
        let size = self.winit_window.get_inner_size().unwrap();

        Rect::new(0, 0, size.width as u32, size.height as u32)
    }

    fn align_summary_body(&self, summary_rect: &mut Rect, body_rect: &mut Rect, window_rect: Rect) {
        summary_rect.set_y(self.config.notification.top_margin as i32);
        body_rect.set_bottom(window_rect.height() as i32 - self.config.notification.bottom_margin as i32);

        /*
        //summary_rect.set_x(0);
        //summary_rect.set_y(0);
        //body_rect.set_x(0);
        body_rect.set_y(summary_rect.bottom());

        let width = std::cmp::max(summary_rect.width(), body_rect.width());
        let height = summary_rect.height() + body_rect.height() + config.notification.summary_body_gap;

        let mut bounds = Rect::new(summary_rect.x(), summary_rect.y(), width, height);
        bounds.center_on(window_rect.center());
        //bounds.set_x(0);

        summary_rect.set_y(bounds.y());
        body_rect.set_bottom(bounds.bottom());
        */
    }

    pub fn draw_text(&mut self, sdl: &SDL2State, summary: &str, body: &str) {
        // Load font etc.
        let texture_creator = self.canvas.texture_creator();
        let font_path = std::path::Path::new("./arial.ttf");
        let mut font = sdl.ttf_context.load_font(&font_path, 12).unwrap();

        // Render to textures.
        font.set_style(sdl2::ttf::FontStyle::BOLD);
        let sfc = font.render(summary)
            .blended(Color::RGBA(255, 255, 255, 255)).unwrap();
        let summary_texture = texture_creator.create_texture_from_surface(&sfc).unwrap();
        font.set_style(sdl2::ttf::FontStyle::NORMAL);
        let sfc = font.render(body)
            .blended(Color::RGBA(255, 255, 255, 255)).unwrap();
        let body_texture = texture_creator.create_texture_from_surface(&sfc).unwrap();


        let sdl2::render::TextureQuery { width, height, .. } = summary_texture.query();
        let mut summary_rect = Rect::new(self.config.notification.left_margin as i32, 0, width as u32, height as u32);
        let sdl2::render::TextureQuery { width, height, .. } = body_texture.query();
        let mut body_rect = Rect::new(self.config.notification.left_margin as i32, 0, width as u32, height as u32);

        self.align_summary_body(&mut summary_rect, &mut body_rect, self.get_rect());

        //let r = vertical_align_rect(rect, )
        self.canvas.set_draw_color(self.config.notification.summary_color.clone());
        self.canvas.copy(&summary_texture, None, Some(summary_rect)).unwrap();
        self.canvas.copy(&body_texture, None, Some(body_rect)).unwrap();
        self.canvas.present();
    }

    pub fn draw(&mut self) {
        let bc = self.config.notification.border_color.clone();
        //let border_color = Color::RGBA(bc.r, bc.g, bc.b, bc.a);
        self.canvas.set_draw_color(bc);
        self.canvas.clear();

        self.canvas.set_draw_color(self.clear_color);
        let bw = self.config.notification.border_width;
        let w = self.config.notification.width;
        let h = self.config.notification.height;

        let inner_rect = Rect::new(bw as i32, bw as i32, w - (bw * 2), h - (bw * 2));
        //self.canvas.draw_rect(inner_rect).unwrap();
        self.canvas.fill_rect(inner_rect).unwrap();
        //self.canvas.present();
    }
}
