use std::time::Duration;

use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event_loop::EventLoopWindowTarget,
    platform::unix::{WindowBuilderExtUnix, WindowExtUnix, XWindowType},
    window::{Window, WindowBuilder},
};

use chrono::{DateTime, Local};

use cairo::{Context, Surface};
use cairo_sys;

use crate::{
    bus::dbus::{Notification, Timeout},
    config::Config,
    manager::NotifyWindowManager,
    maths_utility::{Rect, Vec2},
    rendering::layout::LayoutBlock,
    rendering::text::TextRenderer,
};

// FuseOnly probably won't be used, but it's here for completion's sake.
bitflags! {
    #[derive(Default)]
    pub struct UpdateModes: u8 {
        const DRAW = 0b00000001;
        const FUSE = 0b00000010;
    }
}

#[derive(Debug)]
pub struct NotifyWindow {
    // Context/Surface are placed at the top (in order) so that they are dropped first when a
    // window is dropped.
    pub context: Context,
    pub surface: Surface,
    // Each window has a text renderer to handle all text rendering for that window.
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
    pub fuse: Timeout,

    // `update_enabled` is primarily used for pause functionality right now.
    //pub update_enabled: bool,
    pub update_mode: UpdateModes,

    // Dirty state -- will be redrawn if this is true.
    pub dirty: bool,

    pub creation_timestamp: DateTime<Local>,

    // Last mouse pos, relative to top left of window.
    last_mouse_pos: Vec2,
    // We cache the inner rect, since our windows don't resize, and X11 is slow to reply.
    cached_inner_rect: Option<Rect>,
}

impl NotifyWindow {
    pub fn new(
        el: &EventLoopWindowTarget<()>,
        notification: Notification,
        mut layout: LayoutBlock,
        manager: &NotifyWindowManager,
    ) -> Self {
        let cfg = Config::get();
        // The minimum window width and height is 1.0.  We need this size to generate an initial window.
        let (width, height) = (
            (cfg.min_window_width as f64).max(1.0),
            (cfg.min_window_height as f64).max(1.0),
        );

        // @NOTE: this is pretty messed up... It's annoying that winit only exposes a handle to the
        // xlib display through an existing window, which means we have to use a dummy (hidden)
        // window to grab it.
        // We need the display to do `XMatchVisualInfo`, which we can't set after we've created the
        // window.
        // We might consider moving away from winit and just using xlib directly.  The only part
        // we're really using at the moment is the event loop.
        let xlib_display = manager
            .base_window
            .xlib_display()
            .expect("Couldn't get xlib_display.");

        let visual_info = unsafe {
            let mut vinfo = std::mem::MaybeUninit::<x11::xlib::XVisualInfo>::uninit();

            let status = (x11::xlib::XMatchVisualInfo)(
                xlib_display as _,
                x11::xlib::XDefaultScreen(xlib_display as _) as i32,
                32,
                x11::xlib::TrueColor,
                vinfo.as_mut_ptr(),
            );

            if status == 0 {
                panic!("Couldn't get valid XVisualInfo.");
            }

            vinfo.assume_init()
        };

        let winit = WindowBuilder::new()
            .with_inner_size(PhysicalSize { width, height })
            .with_x11_window_type(vec![XWindowType::Notification, XWindowType::Utility])
            .with_title("wired")
            .with_x11_visual(&visual_info)
            .with_transparent(true)
            // This was originally here for the below reason, but it causes issues and I haven't
            // been able to observe any actual issue, so we leave it out.
            //.with_visible(false)  // We don't draw/position stuff until later, so best not to show the
            // window for now.
            // NOTE: you (apparently) can't draw to a window that is not
            // visible!  So we need to make sure we set this to true before drawing.
            // As an alternative to `with_visible(false)`, we can instead spawn the window really far away.
            // Hopefully nobody has a >100k resolution.
            .with_position(PhysicalPosition {
                x: 999_999.0,
                y: 999_999.0,
            })
            .with_override_redirect(true)
            .build(el)
            .expect("Couldn't build winit window.");

        // If these fail, it probably means we aren't on linux.
        // In that case, we should fail before now however (`.with_x11_window_type()`).
        //let xlib_display = winit.xlib_display().expect("Couldn't get xlib display.");
        let xlib_window = winit
            .xlib_window()
            .expect("Couldn't get xlib window, make sure you're running X11.");

        let surface = unsafe {
            /*
            let visual = x11::xlib::XDefaultVisual(
                xlib_display as _,
                0,
            );
            */

            let sfc_raw = cairo_sys::cairo_xlib_surface_create(
                xlib_display as _,
                xlib_window,
                visual_info.visual,
                width as _,
                height as _,
            );

            Surface::from_raw_full(sfc_raw)
        }
        .expect("Failed to create cairo surface.");

        let context = cairo::Context::new(&surface).expect("Failed to create cairo context.");
        let text = TextRenderer::new(&context);
        let fuse = notification.timeout.clone();

        // If notifications should spawn paused, we check against threshold and against
        // `unpause_on_input`.
        // The reason we don't just pause always and let the other system take care of it
        // is because we don't want notifications to do a pause flicker when we're actually
        // active.
        let mut update_mode = UpdateModes::all();
        if let Some(threshold) = cfg.idle_threshold {
            if manager.is_idle_for(threshold) {
                update_mode = UpdateModes::DRAW;
            }
        }
        if (!cfg.unpause_on_input || manager.is_idle_1s()) && cfg.notifications_spawn_paused {
            update_mode = UpdateModes::DRAW;
        }

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
            update_mode,
            dirty: true, // New windows are dirty -- no drawing has happened yet.
            creation_timestamp: Local::now(),
            last_mouse_pos: Vec2::new(0.0, 0.0),
            cached_inner_rect: None,
        };

        // When we spawn a window, we get a `RedrawRequested` event which we draw from, so we don't
        // manually draw here.
        // `Rect::new(0.0, 0.0, width, height) is basically the same as `window.get_inner_rect()`,
        // but we don't trust it to be initialized yet.
        let rect = layout.predict_rect_tree_and_init(
            &window,
            //&window.get_inner_rect(),
            &Rect::new(0.0, 0.0, width, height), // This parameter is only used for positioning
            Rect::new(0.0, 0.0, width, height),  // .. so we should also pass the min_size rect here
                                                 // to ensure we don't get 0.0 width / 0.0 height.
        );
        let delta = Vec2::new(-rect.x(), -rect.y());

        window.layout = Some(layout);
        window.set_size(rect.width(), rect.height());
        window.master_offset = delta;
        window.cached_inner_rect = Some(Rect::new(0.0, 0.0, rect.width(), rect.height()));
        window
    }

    pub fn replace_notification(&mut self, new_notification: Notification) {
        let cfg = Config::get();

        self.notification = new_notification;

        // Refresh timeout if configured
        if cfg.replacing_resets_timeout {
            self.fuse = self.notification.timeout.clone();
        }

        // The minimum window width and height is 1.0.  We need this size to generate an initial window.
        // TODO: merge the above function with this so we don't get regressions from not doing
        // things the same way.
        let (width, height) = (
            (cfg.min_window_width as f64).max(1.0),
            (cfg.min_window_height as f64).max(1.0),
        );

        // As above.  May be valuable to put this into a function like `prepare_notification` or
        // something if we keep changing stuff.
        let mut layout = self.layout_take();
        let rect = layout.predict_rect_tree_and_init(
            self,
            &Rect::new(0.0, 0.0, width, height),
            Rect::new(0.0, 0.0, width, height),
        );
        let delta = Vec2::new(-rect.x(), -rect.y());

        self.layout = Some(layout);
        self.set_size(rect.width(), rect.height());
        self.master_offset = delta;
        self.dirty = true;
    }

    pub fn _layout(&self) -> &LayoutBlock {
        self.layout.as_ref().unwrap()
    }

    pub fn layout_take(&mut self) -> LayoutBlock {
        self.layout.take().unwrap()
    }

    pub fn set_position(&self, x: f64, y: f64) {
        self.winit.set_outer_position(PhysicalPosition { x, y });
    }

    pub fn _set_visible(&self, visible: bool) {
        self.winit.set_visible(visible);
    }

    pub fn set_size(&self, width: f64, height: f64) {
        self.winit.set_inner_size(PhysicalSize { width, height });
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
        if let Some(rect) = &self.cached_inner_rect {
            //assert!((r.width() - rr.width()).abs() < 0.001);
            //assert!((r.height() - rr.height()).abs() < 0.001);
            rect.clone()
        } else {
            let size = self.winit.inner_size();
            let rect = Rect::new(0.0, 0.0, size.width.into(), size.height.into());
            rect
        }
    }

    /*
    pub fn predict_size(&self) -> (Rect, Vec2) {
        let layout = self.layout();
        let rect = layout.predict_rect_tree(&self, &self.get_inner_rect(), &Rect::EMPTY);
        // If x or y are not 0, then we have to offset our drawing by that amount.
        let delta = Vec2::new(-rect.x(), -rect.y());

        (rect, delta)
    }
    */

    // This should only ever be called by the windows own `update()`.
    // To trigger a redraw, `window.dirty` should be set to `true`.
    fn draw(&mut self) {
        if !self.dirty {
            eprintln!("A draw was triggered for a window that wasn't dirty!");
        }

        let mut inner_rect = self.get_inner_rect();
        // If the master offset is anything other than `(0.0, 0.0)` it means that one of the
        // blocks is going to expand the big rectangle leftwards and/or upwards, which would
        // cause blocks to be drawn off canvas.
        // To fix this, we offset the initial drawing rect to make sure everything fits in the
        // canvas.
        inner_rect.set_xy(self.master_offset.x, self.master_offset.y);
        let mut layout = self.layout_take();
        layout.draw_tree(self, &inner_rect, Rect::empty(), false); // The criteria is parent_is_root, not is_root.  Bad but yeah.
        self.layout = Some(layout);
    }

    pub fn update(&mut self, delta_time: Duration) -> bool {
        if self.update_mode.contains(UpdateModes::FUSE) {
            if let Timeout::Milliseconds(ref mut fuse) = self.fuse {
                *fuse -= delta_time.as_millis() as i32;
                if *fuse <= 0 {
                    // Window will be destroyed after others have been repositioned to replace it.
                    // We can return early because drawing will be discarded anyway.
                    self.marked_for_destroy = true;
                    return true;
                }
            }
        }

        if self.update_mode.contains(UpdateModes::DRAW) {
            let mut layout = self.layout_take();
            self.dirty |= layout.update_tree(delta_time, self);
            self.layout = Some(layout);
        }

        if self.dirty {
            self.draw();
        }

        // Clean now, updated everything, but still need to inform manager that we may have changed.
        let dirty = self.dirty;
        self.dirty = false;
        dirty
    }

    pub fn process_mouse_click(&mut self) {
        let mut layout = self.layout_take();
        self.dirty |= layout.check_and_send_click(&self.last_mouse_pos, self);
        self.layout = Some(layout);
    }

    pub fn process_mouse_move(&mut self, position: PhysicalPosition<f64>) {
        self.last_mouse_pos.x = position.x;
        self.last_mouse_pos.y = position.y;

        let mut layout = self.layout_take();
        self.dirty |= layout.check_and_send_hover(&self.last_mouse_pos, self);
        self.layout = Some(layout);
    }
}
