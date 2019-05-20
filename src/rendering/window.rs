use sdl2::{
    render::WindowCanvas,
    pixels::Color,
    rect::Rect,
    ttf::FontStyle,
    video::WindowPos,
};

use winit::{
    WindowBuilder,
    EventsLoop,
    Window,
    os::unix::{ WindowBuilderExt, XWindowType, WindowExt },
    dpi::{ LogicalSize, LogicalPosition },
};

use crate::config::Config;
use super::sdl::SDL2State;
use super::text::TextRenderer;


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

    pub fn set_position(&mut self, x: WindowPos, y: WindowPos) {
        self.canvas.window_mut().set_position(x, y);
    }

    pub fn get_rect(&self) -> Rect {
        let (width, height) = self.canvas.window().size();
        let (x, y) = self.canvas.window().position();

        Rect::new(x, y, width, height)
    }

    pub fn get_inner_rect(&self) -> Rect {
        let (width, height) = self.canvas.window().size();

        Rect::new(0, 0, width, height)
    }

    // Returns the total space needed to draw the text (including margins).
    fn align_summary_body(&self, summary_rects: &mut Vec<(String, Rect)>, body_rects: &mut Vec<(String, Rect)>, window_rect: Rect) -> Rect {
        let mut prev_y = self.config.notification.top_margin as i32;
        for (_string, rect) in summary_rects {
            rect.set_y(window_rect.y() + prev_y);
            rect.set_x(self.config.notification.left_margin as i32);
            prev_y = rect.bottom();
        }

        prev_y += self.config.notification.summary_body_gap;
        for (_string, rect) in body_rects {
            rect.set_y(window_rect.y() + prev_y);
            rect.set_x(self.config.notification.left_margin as i32);
            prev_y = rect.bottom();
        }

        let height = (prev_y as u32) + (self.config.notification.bottom_margin as u32);
        Rect::new(0, 0, window_rect.width(), height)
    }

    // Returns the rect of the text drawn.
    pub fn draw_text(&mut self, sdl: &SDL2State, summary: &str, body: &str) {
        let font_path = std::path::Path::new("./Carlito-Regular.ttf");
        let font = sdl.ttf_context.load_font(&font_path, 14).unwrap();
        let mut text_renderer = TextRenderer::new(self.config, font);

        let mut summary_tex_rects = text_renderer.prepare_text(summary, FontStyle::BOLD);
        let mut body_tex_rects = text_renderer.prepare_text(body, FontStyle::NORMAL);

        let size = self.align_summary_body(&mut summary_tex_rects, &mut body_tex_rects, self.get_inner_rect());
        self.canvas.window_mut().set_size(size.width(), size.height()).expect("hehe");

        self.canvas.set_draw_color(self.config.notification.summary_color.clone());
        text_renderer.render_text(&mut self.canvas, &mut summary_tex_rects, FontStyle::BOLD);
        self.canvas.set_draw_color(self.config.notification.body_color.clone());
        text_renderer.render_text(&mut self.canvas, &mut body_tex_rects, FontStyle::NORMAL);

        self.canvas.present();
    }

    pub fn draw(&mut self) {
        let bc = self.config.notification.border_color.clone();
        //let border_color = Color::RGBA(bc.r, bc.g, bc.b, bc.a);
        self.canvas.set_draw_color(bc);
        self.canvas.clear();

        self.canvas.set_draw_color(self.clear_color);
        let bw = self.config.notification.border_width;
        let (w, h) = self.canvas.window().size();

        let inner_rect = Rect::new(bw as i32, bw as i32, w - (bw * 2), h - (bw * 2));
        //self.canvas.draw_rect(inner_rect).unwrap();
        self.canvas.fill_rect(inner_rect).unwrap();
        //self.canvas.present();
    }
}
