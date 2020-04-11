use std::time::Duration;

use winit::{
    window::{WindowBuilder, Window},
    event_loop::EventLoopWindowTarget,
    platform::unix::{WindowBuilderExtUnix, XWindowType, WindowExtUnix},
    dpi::{LogicalSize, LogicalPosition},
};

use cairo_sys;
use cairo::{Surface, Context};

use crate::{
    config::Config,
    rendering::layout::LayoutBlock,
    maths::{Rect, Vec2},
    rendering::text::TextRenderer,
    notification::Notification,
};

#[derive(Debug)]
pub struct NotifyWindow {
    // Context/Surface are placed at the top (in order) so that they are dropped first when a
    // window is dropped.
    pub context: Context,
    pub surface: Surface,
    pub text: TextRenderer,

    pub winit: Window,
    pub notification: Notification,

    // Layout is cloned from config so each notification can have its own mutable copy.
    // This is pretty much just so we can change some params on LayoutBlocks, which is a bit
    // wasteful, but easy.
    pub layout: Option<LayoutBlock>,

    pub marked_for_destroy: bool,
    // Master offset is used to offset all *elements* when drawing.
    // It is useful when the notification expands in either left or top direction.
    pub master_offset: Vec2,
    pub fuse: i32,

    // `update_enabled` is primarily used for pause functionality right now.
    pub update_enabled: bool,
}

impl NotifyWindow {
    pub fn new(el: &EventLoopWindowTarget<()>, notification: Notification) -> Self {
        let cfg = Config::get();
        let (width, height) = (cfg.width as f64, cfg.height as f64);

        let winit = WindowBuilder::new()
            .with_inner_size(LogicalSize { width, height })
            .with_x11_window_type(vec![XWindowType::Utility, XWindowType::Notification])
            .with_title("wiry")
            .with_transparent(true)
            .with_visible(false)    // Window not visible for first draw, because the position will probably be wrong.
            .build(el)
            .expect("Couldn't build winit window.");

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
        let fuse = notification.timeout;

        let mut window = Self {
            context,
            surface,
            text,
            winit,
            notification,
            layout: None,
            marked_for_destroy: false,
            master_offset: Vec2::default(),
            fuse,
            update_enabled: true,
        };

        let mut layout = cfg.layout.clone();
        let rect = layout.predict_rect_tree(&window, &window.get_inner_rect(), Rect::default());
        let delta = Vec2::new(-rect.x(), -rect.y());

        window.layout = Some(layout);
        window.set_size(rect.width(), rect.height());
        window.master_offset = delta;
        window
    }

    pub fn layout(&self) -> &LayoutBlock {
        self.layout.as_ref().unwrap()
    }

    pub fn layout_mut(&mut self) -> &mut LayoutBlock {
        self.layout.as_mut().unwrap()
    }

    pub fn set_position(&self, x: f64, y: f64) {
        self.winit.set_outer_position(LogicalPosition { x, y });
    }

    pub fn set_visible(&self, visible: bool) {
        self.winit.set_visible(visible);
    }

    pub fn set_size(&self, width: f64, height: f64) {
        self.winit.set_inner_size(LogicalSize { width, height });
        unsafe {
            cairo_sys::cairo_xlib_surface_set_size(self.surface.to_raw_none(), width as i32, height as i32);
        }
    }

    // Positioned rect on the desktop.
    pub fn _get_rect(&self) -> Rect {
        let size = self.winit.inner_size();
        let pos = self.winit.outer_position().expect("Window no longer exists.");

        Rect::new(pos.x.into(), pos.y.into(), size.width.into(), size.height.into())
    }

    // Pure rectangle, ignoring the window's position.
    pub fn get_inner_rect(&self) -> Rect {
        let size = self.winit.inner_size();

        Rect::new(0.0, 0.0, size.width.into(), size.height.into())
    }

    /*
    pub fn predict_size(&self) -> (Rect, Vec2) {
        let layout = self.layout();
        let rect = layout.predict_rect_tree(&self, &self.get_inner_rect(), &Rect::default());
        // If x or y are not 0, then we have to offset our drawing by that amount.
        let delta = Vec2::new(-rect.x(), -rect.y());

        (rect, delta)
    }
    */

    pub fn draw(&mut self) {
        let mut inner_rect = self.get_inner_rect();
        // If the master offset is anything other than `(0.0, 0.0)` it means that one of the
        // blocks is going to expand the big rectangle leftwards and/or upwards, which would
        // cause blocks to be drawn off canvas.
        // To fix this, we offset the initial drawing rect to make sure everything fits in the
        // canvas.
        inner_rect.set_xy(self.master_offset.x, self.master_offset.y);
        self.layout().draw_tree(self, &inner_rect, Rect::default());
    }

    pub fn update(&mut self, delta_time: Duration) -> bool {
        if !self.update_enabled {
            return false;
        }

        let dirty = self.layout_mut().update_tree(delta_time);
        if dirty {
            self.winit.request_redraw();
            //self.draw();
        }

        self.fuse -= delta_time.as_millis() as i32;
        if self.fuse <= 0 {
            // Window will be destroyed after others have been repositioned to replace it.
            self.marked_for_destroy = true;
            return true
        }

        false
    }
}
