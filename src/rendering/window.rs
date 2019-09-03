use winit::{
    window::{ WindowBuilder, Window },
    event_loop::{ EventLoop, EventLoopWindowTarget },
    platform::unix::{ WindowBuilderExtUnix, XWindowType, WindowExtUnix },
    dpi::{ LogicalSize, LogicalPosition },
};

use crate::config::{ Config, FieldType, Padding, LayoutBlock, AnchorPosition };
use super::text::TextDrawable;

use crate::types::maths::{Rect, Point};
use crate::bus::dbus::Notification;

use cairo::Surface;
use cairo::Context;
use crate::rendering::text::TextRenderer;

#[derive(Debug)]
pub struct NotifyWindow<'config> {
    pub winit: Window,
    pub notification: Notification,

    pub surface: Surface,
    pub context: Context,
    pub drawables: Vec<TextDrawable>,

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
            drawables: Vec::new(),
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

        let layout = &self.config.notification.root;
        let ctx = &self.context;

        let size = |block: &LayoutBlock, parent: &LayoutBlock, parent_rect: Option<&Rect>| -> Rect {
            let text = match &block.field {
                FieldType::Summary => &self.notification.summary,
                FieldType::Body => &self.notification.body,
                _ => "ERROR",
            };

            let rect = self.get_inner_rect();
            let mut pos = match (&parent.field, &block.hook) {
                (FieldType::Root, AnchorPosition::TL) => { rect.top_left() },
                (FieldType::Root, AnchorPosition::TR) => { rect.top_right() },
                (FieldType::Root, AnchorPosition::BL) => { rect.bottom_left() },
                (FieldType::Root, AnchorPosition::BR) => { rect.bottom_right() },

                (_, AnchorPosition::TL) => { parent_rect.unwrap().top_left() },
                (_, AnchorPosition::TR) => { parent_rect.unwrap().top_right() },
                (_, AnchorPosition::BL) => { parent_rect.unwrap().bottom_left() },
                (_, AnchorPosition::BR) => { parent_rect.unwrap().bottom_right() },
            };

            pos.x += block.offset.x;
            pos.y += block.offset.y;

            tr.get_string_rect(&pos, &block.padding, text)
        };

        fn traverse<F: Copy>(block: &LayoutBlock, draw_func: F, parent_rect: Option<&Rect>) -> Rect
            where F: Fn(&LayoutBlock, &LayoutBlock, Option<&Rect>) -> Rect {
            let mut rect = Rect::new(0.0, 0.0, 0.0, 0.0);
            for elem in &block.children {
                let string_rect = draw_func(elem, block, parent_rect);
                rect = rect.union(string_rect.clone());
                rect = rect.union(traverse(elem, draw_func, Some(&string_rect)));
            }

            rect
        }

        traverse(layout, size, None)
    }

    pub fn draw(&self) {
        self.draw_background();

        let tr = TextRenderer::new(&self.context, &self.config.notification.font);

        let layout = &self.config.notification.root;
        let ctx = &self.context;

        let draw = |block: &LayoutBlock, parent: &LayoutBlock, parent_rect: Option<&Rect>| -> Rect {
            let text = match &block.field {
                FieldType::Summary => &self.notification.summary,
                FieldType::Body => &self.notification.body,
                _ => "ERROR",
            };

            let rect = self.get_inner_rect();
            let mut pos = match (&parent.field, &block.hook) {
                (FieldType::Root, AnchorPosition::TL) => { rect.top_left() },
                (FieldType::Root, AnchorPosition::TR) => { rect.top_right() },
                (FieldType::Root, AnchorPosition::BL) => { rect.bottom_left() },
                (FieldType::Root, AnchorPosition::BR) => { rect.bottom_right() },

                (_, AnchorPosition::TL) => { parent_rect.unwrap().top_left() },
                (_, AnchorPosition::TR) => { parent_rect.unwrap().top_right() },
                (_, AnchorPosition::BL) => { parent_rect.unwrap().bottom_left() },
                (_, AnchorPosition::BR) => { parent_rect.unwrap().bottom_right() },
            };

            pos.x += block.offset.x;
            pos.y += block.offset.y;

            let text_color = &self.config.notification.text_color;
            ctx.set_source_rgba(text_color.r, text_color.g, text_color.b, text_color.a);
            tr.paint_string(ctx, &pos, &block.padding, text)
        };

        fn traverse<F: Copy>(block: &LayoutBlock, draw_func: F, parent_rect: Option<&Rect>)
            where F: Fn(&LayoutBlock, &LayoutBlock, Option<&Rect>) -> Rect {
            for elem in &block.children {
                let rect = draw_func(elem, block, parent_rect);
                traverse(elem, draw_func, Some(&rect));
            }
        }

        traverse(layout, draw, None);
    }
}
