use winit::{
    window::{ WindowBuilder, Window },
    event_loop::EventLoopWindowTarget,
    platform::unix::{ WindowBuilderExtUnix, XWindowType, WindowExtUnix },
    dpi::{ LogicalSize, LogicalPosition },
};

use cairo::{ Surface, Context };

use crate::bus::dbus::Notification;
use crate::config::{ Config, FieldType, LayoutBlock, AnchorPosition };
use crate::types::maths::{Rect, Vec2};
use crate::rendering::text::TextRenderer;
use std::alloc::Layout;

#[derive(Debug)]
pub struct NotifyWindow<'config> {
    pub winit: Window,
    pub notification: Notification,

    pub surface: Surface,
    pub context: Context,

    pub dirty: bool,

    config: &'config Config,
}

impl<'config> NotifyWindow<'config> {
    pub fn new(config: &'config Config, el: &EventLoopWindowTarget<()>, notification: Notification) -> Self {
        let (width, height) = (config.notification.width, config.notification.height);

        let winit = WindowBuilder::new()
            .with_inner_size(LogicalSize { width: width as f64, height: height as f64 })
            .with_title("wiry")
            .with_transparent(true)
            .with_always_on_top(true)
            .with_x11_window_type(XWindowType::Utility)
            .with_x11_window_type(XWindowType::Notification)
            .build(el)
            .expect("Couldn't build winit window.");

        winit.set_outer_position(LogicalPosition { x: config.notification.x as f64, y: config.notification.y as f64 });

        // If these fail, it probably means we aren't on linux.
        // In that case, we should fail before now however (`.with_x11_window_type()`).
        let xlib_display = winit.xlib_display().expect("Couldn't get xlib display.");
        let xlib_window = winit.xlib_window().expect("Couldn't get xlib window.");

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

        // TODO: return Result? sometimes.
        Self {
            winit,
            notification,
            surface,
            context,
            dirty: true,
            config,
        }
    }

    pub fn set_position(&self, x: f64, y: f64) {
        self.winit.set_outer_position(LogicalPosition { x, y });
    }

    pub fn set_size(&self, width: f64, height: f64) {
        self.winit.set_inner_size(LogicalSize { width, height });
        unsafe {
            cairo_sys::cairo_xlib_surface_set_size(self.surface.to_raw_none(), width as i32, height as i32);
        }
    }

    // Positioned rect on the desktop.
    pub fn get_rect(&self) -> Rect {
        let size = self.winit.inner_size();
        let pos = self.winit.outer_position().expect("Window no longer exists.");

        Rect::new(pos.x, pos.y, size.width, size.height)
    }

    // Pure rectangle, ignoring the window's position.
    pub fn get_inner_rect(&self) -> Rect {
        let size = self.winit.inner_size();

        Rect::new(0.0, 0.0, size.width, size.height)
    }

    pub fn draw_background(&self) {
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

    pub fn predict_size(&self) -> Rect {
        let tr = TextRenderer::new(&self.context, &self.config.notification.font);
        let get_size = |block: &LayoutBlock, parent_rect: &Rect| -> Rect {
            let text = match &block.field {
                FieldType::Summary => &self.notification.summary,
                FieldType::Body => &self.notification.body,
                _ => "ERROR",
            };

            let pos = block.find_anchor_pos(parent_rect);
            tr.get_string_rect(&block.parameters, &pos, text)
        };

        let accumulator = |r1: &Rect, r2: &Rect| -> Rect {
            r1.union(r2)
        };

        let layout = &self.config.notification.root;
        layout.traverse_accum(get_size, accumulator, &Rect::default(), &self.get_inner_rect()).clone()
    }

    pub fn draw(&self) {
        self.draw_background();

        let tr = TextRenderer::new(&self.context, &self.config.notification.font);
        let ctx = &self.context;

        let draw = |block: &LayoutBlock, parent_rect: Option<&Rect>| -> Rect {
            let text = match &block.field {
                FieldType::Summary => &self.notification.summary,
                FieldType::Body => &self.notification.body,
                _ => "ERROR",
            };

            let pos = block.find_anchor_pos(parent_rect.unwrap());
            let text_color = &block.parameters.color;
            ctx.set_source_rgba(text_color.r, text_color.g, text_color.b, text_color.a);
            tr.paint_string(ctx, &block.parameters, &pos, text)
        };

        let layout = &self.config.notification.root;
        layout.traverse(draw, Some(&self.get_inner_rect()));
    }
}
