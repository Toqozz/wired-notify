use winit::{
    window::{ WindowBuilder, Window },
    event_loop::EventLoopWindowTarget,
    platform::unix::{ WindowBuilderExtUnix, XWindowType, WindowExtUnix },
    dpi::{ LogicalSize, LogicalPosition },
};

use cairo::{ Surface, Context };

use crate::config::Config;
use crate::rendering::layout::LayoutBlock;
use crate::types::maths::{Rect, Vec2};
use crate::rendering::text::TextRenderer;
use crate::notification::Notification;
use cairo_sys;

#[derive(Debug)]
pub struct NotifyWindow<'config> {
    // Context/Surface are placed at the top (in order) so that they are dropped first when a
    // window is dropped.
    pub context: Context,
    pub surface: Surface,

    pub winit: Window,
    pub notification: Notification,

    pub text: TextRenderer,

    pub marked_for_destroy: bool,

    // Master offset is used to offset all *elements* when drawing.
    // It is useful when the notification expands in either left or top direction.
    pub master_offset: Vec2,

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
            .with_visible(false)    // Window not visible for first draw, because the position will probably be wrong.
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
            marked_for_destroy: false,
            master_offset: Vec2::default(),
            config,
        }
    }

    pub fn set_position(&self, x: f64, y: f64) {
        self.winit.set_outer_position(LogicalPosition { x, y });
        // TODO: only do this once?
        self.winit.set_visible(true);
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

    pub fn predict_size(&self) -> (Rect, Vec2) {
        let layout = &self.config.layout;
        let rect = layout.predict_rect_tree(&self, &self.get_inner_rect(), &Rect::default());
        // If x or y are not 0, then we have to offset our drawing by that amount.
        let delta = Vec2::new(-rect.x(), -rect.y());

        (rect, delta)
    }

    pub fn draw(&mut self) {
        let draw = |block: &LayoutBlock, parent_rect: &Rect| -> Rect {
            let rect = block.draw_independent(parent_rect, &self);
            // Draw debug rect around bounding box.
            if self.config.debug {
                self.context.set_source_rgba(1.0, 0.0, 0.0, 1.0);
                self.context.set_line_width(1.0);
                self.context.rectangle(rect.x(), rect.y(), rect.width(), rect.height());
                self.context.stroke();
            }

            rect
        };

        let layout = &self.config.layout;
        let mut inner_rect = self.get_inner_rect();
        // If the master offset is anything other than `(0.0, 0.0)` it means that one of the
        //   blocks is going to expand the big rectangle leftwards and/or upwards, which would
        //   cause blocks to be drawn off canvas.
        //   To fix this, we offset the initial drawing rect to make sure everything fits in the
        //   canvas.
        inner_rect.set_xy(self.master_offset.x, self.master_offset.y);
        layout.traverse(draw, &inner_rect);//&self.get_inner_rect());
    }
}
