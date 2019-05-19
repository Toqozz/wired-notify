use sdl2::render::WindowCanvas;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::Texture;
use sdl2::ttf::FontStyle;


use super::sdl::{ SDL2State };
use super::text::TextRenderer;
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

    pub fn break_text_into_lines(&self, sdl: &SDL2State, text: &str, max_width: u32) -> Vec<String> {
        let mut lines = Vec::new();

        let font_path = std::path::Path::new("./arial.ttf");
        let mut font = sdl.ttf_context.load_font(&font_path, 12).unwrap();

        font.set_style(sdl2::ttf::FontStyle::BOLD);

        let mut last_whitespace = 0;
        let mut last_cut = 0;
        let mut line_history = String::from("");
        for (i, c) in text.char_indices() {
            if c.is_whitespace() {
                last_whitespace = i;
            }


            let string = &text[last_cut..i+1];
            let (width, _height) = font.size_of(&string).unwrap();
            if width > max_width {
                lines.push(text[last_cut..last_whitespace].to_owned());
                last_cut = last_whitespace+1;
            }

            line_history = text[last_cut..i+1].to_owned();
        }

        if !line_history.is_empty() {
            lines.push(line_history);
        }

        lines
    }

    fn align_summary_body(&self, summary_rects: &mut Vec<(String, Rect)>, body_rects: &mut Vec<(String, Rect)>, window_rect: Rect) {
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
    }

    pub fn draw_text(&mut self, sdl: &SDL2State, summary: &str, body: &str) {
        let font_path = std::path::Path::new("./arial.ttf");
        let font = sdl.ttf_context.load_font(&font_path, 12).unwrap();
        let mut text_renderer = TextRenderer::new(self.config, sdl, font);

        //font.set_style(sdl2::ttf::FontStyle::BOLD);
        let mut summary_tex_rects = text_renderer.prepare_text(summary, FontStyle::BOLD);
        //font.set_style(sdl2::ttf::FontStyle::NORMAL);
        let mut body_tex_rects = text_renderer.prepare_text(body, FontStyle::NORMAL);

        self.align_summary_body(&mut summary_tex_rects, &mut body_tex_rects, self.get_rect());

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
        let w = self.config.notification.width;
        let h = self.config.notification.height;

        let inner_rect = Rect::new(bw as i32, bw as i32, w - (bw * 2), h - (bw * 2));
        //self.canvas.draw_rect(inner_rect).unwrap();
        self.canvas.fill_rect(inner_rect).unwrap();
        //self.canvas.present();
    }
}
