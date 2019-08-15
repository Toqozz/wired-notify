use winit::{
    window::{ WindowBuilder, Window },
    event_loop::{ EventLoop, EventLoopWindowTarget },
    platform::unix::{ WindowBuilderExtUnix, XWindowType, WindowExtUnix },
    dpi::{ LogicalSize, LogicalPosition },
};

use crate::config::Config;
use super::text::TextDrawable;

use crate::rendering::maths::Rect;

use cairo::Surface;
use cairo::Context;

#[derive(Debug)]
pub struct CairoWindow<'config> {
    pub window: Window,
    pub surface: Surface,
    pub context: Context,

    pub drawables: Vec<TextDrawable>,

    pub dirty: bool,

    config: &'config Config,
}

impl<'config> CairoWindow<'config> {
    pub fn new(config: &'config Config, el: &EventLoopWindowTarget<()>) -> CairoWindow<'config> {
        // Hack to avoid dpi scaling -- we just want pixels.
        std::env::set_var("WINIT_HIDPI_FACTOR", "1.0");

        let (width, height) = (config.notification.width, config.notification.height);

        let window = WindowBuilder::new()
            .with_inner_size(LogicalSize { width: width as f64, height: height as f64 })
            .with_title("wiry")
            .with_transparent(true)
            .with_always_on_top(true)
            .with_x11_window_type(XWindowType::Utility)
            .with_x11_window_type(XWindowType::Notification) // try ORing these.
            .build(el)
            .expect("Couldn't build winit window.");

        window.set_outer_position(LogicalPosition { x: config.notification.x as f64, y: config.notification.y as f64 });
        // If these fail, it probably means we aren't on linux.
        // In that case, we should fail before now however (`.with_x11_window_type()`).
        let xlib_display = window.xlib_display().expect("Couldn't get xlib display.");
        let xlib_window = window.xlib_window().expect("Couldn't get xlib window.");

        let surface = unsafe {
            let visual = x11::xlib::XDefaultVisual(
                xlib_display as _,
                0,
            );

            let sfc_raw = cairo_sys::cairo_xlib_surface_create(
                xlib_display as _,
                xlib_window,
                visual,
                width as _,
                height as _,
            );

            Surface::from_raw_full(sfc_raw)
        };

        let context = cairo::Context::new(&surface);

        // TODO: return errors sometimes.
        Self {
            window,
            surface,
            context,
            drawables: Vec::new(),
            dirty: true,
            config,
        }
    }

    pub fn set_position(&self, x: f64, y: f64) {
        self.window.set_outer_position(LogicalPosition { x, y });
    }

    pub fn set_size(&self, width: f64, height: f64) {
        self.window.set_inner_size(LogicalSize { width, height });
        unsafe {
            cairo_sys::cairo_xlib_surface_set_size(self.surface.to_raw_none(), width as i32, height as i32);
        }
    }

    pub fn get_rect(&self) -> Rect {
        let size = self.window.inner_size();
        let pos = self.window.outer_position().expect("Window no longer exists.");

        Rect::new(pos.x, pos.y, size.width, size.height)
    }

    pub fn get_inner_rect(&self) -> Rect {
        let size = self.window.inner_size();

        Rect::new(0.0, 0.0, size.width, size.height)
    }

    pub fn draw_background(&mut self) {
        let ctx = &self.context;
        let rect = self.get_inner_rect();

        // Clear
        ctx.set_operator(cairo::Operator::Clear);
        ctx.paint();

        // Draw border + background.
        ctx.set_operator(cairo::Operator::Source);

        let bd_color = &self.config.notification.border_color;
        ctx.set_source_rgba(bd_color.r, bd_color.g, bd_color.b, bd_color.a);
        ctx.paint();

        let bg_color = &self.config.notification.background_color;
        let bw = &self.config.notification.border_width;
        ctx.set_source_rgba(bg_color.r, bg_color.g, bg_color.b, bg_color.a);
        ctx.rectangle(
            *bw, *bw,     // x, y
            rect.width() - bw * 2.0, rect.height() - bw * 2.0,
        );
        ctx.fill();
    }

    pub fn draw_drawables(&self) {
        for drawable in &self.drawables {
            drawable.paint_to_ctx(&self.context);
        }
    }
}
