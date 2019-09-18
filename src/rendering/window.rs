use winit::{
    window::{ WindowBuilder, Window },
    event_loop::EventLoopWindowTarget,
    platform::unix::{ WindowBuilderExtUnix, XWindowType, WindowExtUnix },
    dpi::{ LogicalSize, LogicalPosition },
};

use cairo::{ Surface, Context };

use crate::bus::dbus::Notification;
use crate::config::{ Config, LayoutBlock };
use crate::types::maths::Rect;
use crate::rendering::text::TextRenderer;
use cairo::prelude::SurfaceExt;

#[derive(Debug)]
pub struct NotifyWindow<'config> {
    pub context: Context,
    pub surface: Surface,

    pub winit: Window,
    pub notification: Notification,

    pub text: TextRenderer,

    pub dirty: bool,

    config: &'config Config,
}

/*
impl Drop for NotifyWindow<'_> {
    fn drop(&mut self) {
        // Setting these to None causes them to be dropped.
        // This is a workaround for not being able to call drop! on them.
        self.context = None;
        self.surface = None;
    }
}
*/

impl<'config> NotifyWindow<'config> {
    pub fn new(config: &'config Config, el: &EventLoopWindowTarget<()>, notification: Notification) -> Self {
        let (width, height) = (config.width, config.height);

        let winit = WindowBuilder::new()
            .with_inner_size(LogicalSize { width: width as f64, height: height as f64 })
            .with_x11_window_type(vec![XWindowType::Utility, XWindowType::Notification])
            .with_title("wiry")
            .with_transparent(true)
            .with_visible(false)    // Window not visible for first draw.
            .build(el)
            .expect("Couldn't build winit window.");

        // @NOTE: does the window appear in a weird place, a weird size?  We should probably hide
        // the window until it's marked clean.
        //winit.set_outer_position(LogicalPosition { x: config.x as f64, y: config.y as f64 });

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

        let text = TextRenderer::new(&context);

        // TODO: return Result? sometimes.
        Self {
            context,
            surface,
            winit,
            notification,
            text,
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

    fn draw_background(&self) {
        let ctx = &self.context;
        let rect = self.get_inner_rect();

        // Clear
        ctx.set_operator(cairo::Operator::Clear);
        ctx.paint();

        // Draw border + background.
        ctx.set_operator(cairo::Operator::Source);

        let bd_color = &self.config.border_color;
        ctx.set_source_rgba(bd_color.r, bd_color.g, bd_color.b, bd_color.a);
        ctx.paint();

        let bg_color = &self.config.background_color;
        let bw = &self.config.border_width;
        ctx.set_source_rgba(bg_color.r, bg_color.g, bg_color.b, bg_color.a);
        ctx.rectangle(
            *bw, *bw,     // x, y
            rect.width() - bw * 2.0, rect.height() - bw * 2.0,
        );
        ctx.fill();
    }

    pub fn predict_size(&self) -> Rect {
        // TODO: this should be cached.
        let get_size = |block: &LayoutBlock, parent_rect: &Rect| -> Rect {
            match &block {
                LayoutBlock::TextBlock(p) => {
                    let mut text = p.text.clone();
                    text = text.replace("%s", &self.notification.summary);
                    text = text.replace("%b", &self.notification.body);

                    let pos = block.find_anchor_pos(parent_rect);
                    self.text.get_string_rect(&p.parameters, &pos, &text)
                }

                _ => Rect::new(0.0, 0.0, 0.0, 0.0)
            }
        };

        let accumulator = |r1: &Rect, r2: &Rect| -> Rect {
            r1.union(r2)
        };

        let layout = &self.config.layout;
        layout.traverse_accum(get_size, accumulator, &Rect::default(), &self.get_inner_rect())
    }

    pub fn draw(&mut self) {
        self.draw_background();

        /*
        if !window.dirty {
            return;
        }
        */

        let ctx = &self.context;

        let draw = |block: &LayoutBlock, parent_rect: &Rect| -> Rect {
            match &block {
                LayoutBlock::TextBlock(p) => {
                    // TODO: Some/None for summary/body?  We don't want to replace or even add the block if there is no body.
                    let mut text = p.text.clone();
                    text = text
                        .replace("%s", &self.notification.summary)
                        .replace("%b", &self.notification.body)
                        .replace("&quot;", "\"")
                        .replace("&amp;", "&")
                        .replace("&lt;", "<")
                        .replace("&gt;", ">");

                    let pos = block.find_anchor_pos(parent_rect);

                    let text_color = &p.parameters.color;
                    ctx.set_source_rgba(text_color.r, text_color.g, text_color.b, text_color.a);
                    self.text.paint_string(ctx, &p.parameters, &pos, &text)
                }

                _ => Rect::new(0.0, 0.0, 0.0, 0.0)
            }
        };

        let layout = &self.config.layout;
        layout.traverse(draw, &self.get_inner_rect());

        // Window is now clean.
        self.dirty = false;
    }
}
