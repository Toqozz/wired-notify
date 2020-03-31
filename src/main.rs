extern crate winit;
extern crate dirs;
//extern crate gl;

mod rendering;
mod notification;
mod bus;
mod config;
mod types;

//use winit::EventsLoop;
use winit::{
    event::{ Event, WindowEvent, ElementState, MouseButton },
    event_loop::{ ControlFlow, EventLoop },
    platform::desktop::EventLoopExtDesktop,
    platform::unix::EventLoopExtUnix,
};

use notification::management::NotifyWindowManager;
use bus::dbus;
use winit::event::StartCause;
use std::time::{Instant, Duration};
use notify::DebouncedEvent;

use std::cell::RefCell;


/*
thread_local!(
    //static CONFIG: RefCell<config::Config> = RefCell::new(ron::de::from_str());
);
*/

// TODO: put these in config module.

fn main() {
    // Hack to avoid winit dpi scaling -- we just want pixels.
    // NOTE: currently there is a winit bug where this value doesn't apply if Xft.dpi is set in XResources.
    // This should be fixed in a future winit release, and maybe we can also avoid setting an environment variable here.
    std::env::set_var("WINIT_X11_SCALE_FACTOR", "1.0");

    let mut event_loop = EventLoop::new_x11().expect("Couldn't create an X11 event loop.");

    let config = config::Config::load();
    let maybe_watcher = config::Config::watch();

    let mut manager = NotifyWindowManager::new(&config);

    // Allows us to receive messages from dbus.
    let (connection, receiver) = dbus::get_connection();

    //let timer_length = std::time::Duration::new(1, 0);
    let timer_length = Duration::from_millis(config.poll_interval);
    let mut prev_instant = Instant::now();
    event_loop.run_return(move |event, event_loop, control_flow| {
        match event {
            // @TODO: maybe we should separate receiving dbus signals and drawing windows.
            Event::NewEvents(StartCause::Init) => *control_flow = ControlFlow::WaitUntil(Instant::now() + timer_length),
            Event::NewEvents(StartCause::ResumeTimeReached { .. }) => {
                let now = Instant::now();

                // Time passed since last loop.
                let time_passed = now - prev_instant;
                prev_instant = now;
                manager.update(time_passed);

                // Check dbus signals.
                // If we don't do this then we will block.
                let signal = connection.incoming(0).next();
                if let Some(s) = signal {
                    //dbg!(s);
                }

                if let Ok(x) = receiver.try_recv() {
                    //spawn_window(x, &mut manager, &event_loop);
                    manager.new_notification(x, event_loop);
                }

                // If the watcher exists (.config/wiry exists), then we should process watcher events.
                if let Some((_watcher, rx)) = &maybe_watcher {
                    if let Ok(x) = rx.try_recv() {
                        match x {
                            DebouncedEvent::Write(path) |
                            DebouncedEvent::Create(path) |
                            DebouncedEvent::Chmod(path) => {
                                // Reload the config.
                                // @TODO: reloading the config could cause a crash -- check if the
                                // config is valid.
                                //config = config::Config::load();
                            },
                            _ => {},
                        }
                    }
                }

                // Restart timer for next loop.
                *control_flow = ControlFlow::WaitUntil(now + timer_length);
            },

            // Window becomes visible and then position is set.  Need fix.
            Event::RedrawRequested(window_id) => manager.draw_window(window_id),
            Event::WindowEvent { window_id, event: WindowEvent::MouseInput { state: ElementState::Pressed,  button: MouseButton::Left, .. } } => manager.drop_window(window_id),
            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => *control_flow = ControlFlow::Exit,

            // Poll continuously runs the event loop, even if the os hasn't dispatched any events.
            // This is ideal for games and similar applications.
            _ => ()
            //_ => *control_flow = ControlFlow::Poll,
        }
    });
}
