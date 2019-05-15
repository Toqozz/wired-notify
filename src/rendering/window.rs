use sdl2::sys::SDL_WindowFlags;
use sdl2::render::WindowCanvas;
use sdl2::pixels::Color;

use super::sdl::{ SDL2State };

use crate::config::Config;

use winit::WindowBuilder;
use winit::os::unix::WindowBuilderExt;
use winit::os::unix::XWindowType;
use winit::os::unix::WindowExt;
use winit::dpi::LogicalSize;
use winit::dpi::LogicalPosition;
use winit::EventsLoop;
use winit::WindowId;
use winit::Window;


pub struct SDL2Window {
    pub winit_window: Window,
    pub canvas: WindowCanvas,
    clear_color: Color,

    pub id: WindowId,

    //pub window: sdl2::video::Window,
    //pub ctx: sdl2::video::GLContext,
}

impl SDL2Window {
    pub fn new(sdl: &SDL2State, config: &Config, events_loop: &EventsLoop) -> Result<SDL2Window, String> {
        SDL2Window::new_via_winit(sdl, config, events_loop)
    }

    fn new_via_winit(sdl: &SDL2State, config: &Config, el: &EventsLoop) -> Result<SDL2Window, String> {
        // Hack to avoid dpi scaling.
        std::env::set_var("WINIT_HIDPI_FACTOR", "1.0");

        let color = &config.notification.background_color;
        let clear_color = Color::RGBA(color.r, color.g, color.b, color.a);
        let (width, height) = (config.notification.width, config.notification.height);

        // Dropping winit here is legal, but will cause crashes later, so we should keep it around.
        let winit_window = WindowBuilder::new()
            .with_dimensions(LogicalSize { width: width as f64, height: height as f64 })
            .with_title("wiry")
            .with_transparency(false)
            .with_always_on_top(true)
            .with_x11_window_type(XWindowType::Utility)
            .with_x11_window_type(XWindowType::Notification)
            .build(el)
            .unwrap();

        winit_window.set_position(LogicalPosition { x: 10.0, y: 874.0 });
        let id = winit_window.id();

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
            id,
        })
    }
    /*
    pub fn new_x11(x: i32, y: i32, w: u32, h: u32) -> u32 {
        unsafe {
            // Should we be using libc::PT_NULL?
            let display = xlib::XOpenDisplay(std::ptr::null());
            // Does this even work?
            if display.is_null() {
                return 0;
            }

            // use primary display.
            // TODO: think about letting this be user-selectable.
            let screen = xlib::XDefaultScreen(display);
            let mut vinfo: xlib::XVisualInfo = std::mem::uninitialized();
            let _result = xlib::XMatchVisualInfo(display, screen, 32, xlib::TrueColor, &mut vinfo);

            let mut attr: xlib::XSetWindowAttributes = std::mem::uninitialized();
            attr.colormap = xlib::XCreateColormap(display, xlib::XDefaultRootWindow(display), vinfo.visual, xlib::AllocNone);
            attr.border_pixel = 0;
            attr.backing_pixel = 0;

            let window = xlib::XCreateWindow(
                display,
                xlib::XDefaultRootWindow(display),
                x, y,
                w, h,
                0,
                vinfo.depth, xlib::InputOutput as u32, vinfo.visual,
                xlib::CWColormap | xlib::CWBorderPixel | xlib::CWBackPixel,
                &mut attr
            );

            let wiry_cstring = CString::new("wiry").unwrap();

            xlib::XStoreName(display, window, wiry_cstring.as_ptr());

            let property1 = xlib::XInternAtom(display, CString::new("_NET_WM_NAME").unwrap().as_ptr(), false as i32);
            let property2 = xlib::XInternAtom(display, CString::new("UTF8_STRING").unwrap().as_ptr(), false as i32);
            xlib::XChangeProperty(display, window, property1, property2, 8, xlib::PropModeReplace, wiry_cstring.as_bytes().as_ptr(), 4);

            let mut classhint = xlib::XClassHint {
                res_name: CString::new("wiry").unwrap().into_raw(),
                res_class: CString::new("wiry").unwrap().into_raw(),
            };
            xlib::XSetClassHint(display, window, &mut classhint);

            let property2 = xlib::XInternAtom(display, CString::new("_NET_WM_WINDOW_TYPE").unwrap().as_ptr(), false as i32);
            let property0 = xlib::XInternAtoms(display, CString::new("_NET_WM_WINDOW_TYPE_NOTIFICATION").unwrap().as_ptr(), false as i32);
            let property1 = xlib::XInternAtom(display, CString::new("_NET_WM_WINDOW_TYPE_UTILITY").unwrap().as_ptr(), false as i32);
        }

        0
    }
    */

    pub fn draw(&mut self) {
        self.canvas.set_draw_color(self.clear_color);
        self.canvas.clear();
        self.canvas.present();
    }
}
