use winit::{
    WindowBuilder,
    EventsLoop,
    Window,
    os::unix::{ WindowBuilderExt, XWindowType, WindowExt },
    dpi::{ LogicalSize, LogicalPosition },
};

use crate::config::{ Config, Anchor, AnchorPosition };
use super::text::TextRenderer;

use super::maths::{ Rect, Point };

use cairo::Surface;
use cairo::Context;


pub struct CairoWindow<'config> {
    pub window: Window,
    pub surface: Surface,
    pub context: Context,

    config: &'config Config,
}

impl<'config> CairoWindow<'config> {
    pub fn new(config: &'config Config, el: &EventsLoop) -> CairoWindow<'config> {
        // Hack to avoid dpi scaling -- we just want pixels.
        std::env::set_var("WINIT_HIDPI_FACTOR", "1.0");

        let color = &config.notification.background_color;
        let (width, height) = (config.notification.width, config.notification.height);

        let window = WindowBuilder::new()
            .with_dimensions(LogicalSize { width: width as f64, height: height as f64 })
            .with_title("wiry")
            .with_transparency(true)
            .with_always_on_top(true)
            .with_x11_window_type(XWindowType::Utility)
            .with_x11_window_type(XWindowType::Notification) // try ORing these.
            .build(el)
            .unwrap();

        window.set_position(LogicalPosition { x: config.notification.x as f64, y: config.notification.y as f64 });

        let surface = unsafe {
            let visual = x11::xlib::XDefaultVisual(
                window.get_xlib_display().unwrap() as _,
                0,
            );

            // TODO: check for Linux to guard unwrapping.
            let sfc_raw = cairo_sys::cairo_xlib_surface_create(
                window.get_xlib_display().unwrap() as _,
                window.get_xlib_window().unwrap(),
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
            config,
        }
    }

    pub fn set_position(&self, x: f64, y: f64) {
        self.window.set_position(LogicalPosition { x, y });
    }

    pub fn set_size(&self, width: f64, height: f64) {
        self.window.set_inner_size(LogicalSize { width, height });
    }

    pub fn get_rect(&self) -> Rect {
        let size = self.window.get_inner_size().unwrap();
        let pos = self.window.get_position().unwrap();

        Rect::new(pos.x, pos.y, size.width, size.height)
    }

    pub fn get_inner_rect(&self) -> Rect {
        let size = self.window.get_inner_size().unwrap();

        Rect::new(0.0, 0.0, size.width, size.height)
    }

    pub fn draw(&mut self) {
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

    pub fn draw_text(&self, summary_str: &str, body_str: &str) {
        let self_rect = self.get_inner_rect();
        let tr = TextRenderer::new(self.config, "Arial 10", &self.context);
        let ctx = &self.context;

        // Draw summary + body text.
        ctx.set_operator(cairo::Operator::Source);

        let font_color = &self.config.notification.summary.color;
        ctx.set_source_rgba(font_color.r, font_color.g, font_color.b, font_color.a);

        let s_text_area = &self.config.notification.summary;
        let (s_pad_top, s_pad_bottom, s_pad_left, s_pad_right) = (
            s_text_area.top_margin,
            s_text_area.bottom_margin,
            s_text_area.left_margin,
            s_text_area.right_margin,
        );

        //let s_anchor = s_text_area.anchor;
        //let s_anchor_pos = s_text_area.anchor_position;

        let mut origin = Point { x: 0.0, y: 0.0 };
        //match s_anchor {
            //_ => { x_origin = 0; y_origin = 0 },
        //}

        let s_rect = tr.render_string_pango(
            origin.x + s_pad_left,
            origin.y + s_pad_top,
            summary_str,
        );

        // @NOTE: Need to clean this padding -- we should probably create a padding struct.
        // Another option may be to include the padding in the rectangle calculation -- this is
        // probably the smartest option.
        let b_text_area = &self.config.notification.body;
        let (b_pad_top, b_pad_bottom, b_pad_left, _b_pad_right) = (
            b_text_area.top_margin,
            b_text_area.bottom_margin,
            b_text_area.left_margin,
            b_text_area.right_margin,
        );

        // @NOTE: We need a way to specify TopLeft, if only for the Root.
        // Consider that TopLeft etc is only relevant in relation to the Root, because padding will
        // screw it up.
        // There must be a better way to describe this.
        match (&b_text_area.anchor, &b_text_area.anchor_position) {
            (Anchor::Summary, AnchorPosition::Left) => { origin.x = s_rect.left() },
            (Anchor::Summary, AnchorPosition::Right) => { origin.x = s_rect.right() },
            (Anchor::Summary, AnchorPosition::Top) => { origin.y = s_rect.top() },
            (Anchor::Summary, AnchorPosition::Bottom) => { origin.y = s_rect.bottom() },

            (Anchor::Root, AnchorPosition::Left) => { origin.x = self_rect.left() },
            (Anchor::Root, AnchorPosition::Right) => { origin.x = self_rect.right() },
            (Anchor::Root, AnchorPosition::Top) => { origin.y = self_rect.top() },
            (Anchor::Root, AnchorPosition::Bottom) => { origin.y = self_rect.bottom() },

            _ => { origin = Point { x: 0.0, y: 0.0 } },
        }

        let b_rect = tr.render_string_pango(
            origin.x + s_pad_right + b_pad_left,
            origin.y + b_pad_top,
            body_str,
        );

        self.set_size(self_rect.width(), b_rect.bottom() + b_pad_bottom);
    }
}
