extern crate winit;
//extern crate gl;

mod rendering;
mod notification;
mod bus;
mod config;
mod types;

//use winit::EventsLoop;
use winit::{
    event::{ Event, WindowEvent },
    event_loop::{ ControlFlow, EventLoop },
    platform::desktop::EventLoopExtDesktop,
};

use notification::management::NotifyWindowManager;
use bus::dbus;
use crate::config::LayoutBlock::NotificationBlock;
use winit::event::StartCause;
use std::task::Context;
use std::time::{Instant, Duration};

fn main() {
    // Hack to avoid winit dpi scaling -- we just want pixels.
    // NOTE: currently there is a winit bug where this value doesn't apply if Xft.dpi is set in XResources.
    // This should be fixed in a future winit release, and maybe we can also avoid setting an environment variable here.
    std::env::set_var("WINIT_HIDPI_FACTOR", "1.0");

    let mut event_loop = EventLoop::new();    // TODO: maybe use `EventsLoop::new_x11()` ?
    let event_loop_proxy = event_loop.create_proxy();

    let mut config: config::Config = ron::de::from_str(include_str!("config.ron"))
        .expect("Failed to load config.\n");

    // runtime config setup.
    if let NotificationBlock(params) = &config.layout {
        config.monitor = Some(event_loop.available_monitors()
            .nth(params.monitor as usize)
            .unwrap_or(event_loop.primary_monitor()));
    }

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
                // Restart timer for next loop.
                let now = Instant::now();
                *control_flow = ControlFlow::WaitUntil(now + timer_length);

                // Time passed since last loop.
                let time_passed = now - prev_instant;
                prev_instant = now;

                manager.update_timers(time_passed);

                // Check dbus signals.
                let signal = connection.incoming(0).next();
                if let Some(s) = signal {
                    dbg!(s);
                }

                if let Ok(x) = receiver.try_recv() {
                    //spawn_window(x, &mut manager, &event_loop);
                    manager.new_notification(x, event_loop);
                    // Initial draw, otherwise we won't redraw until the event queue clears again.
                    // @NOTE: is this an issue for framerate draws? -- investigate winit timer.
                    //manager.draw_windows();
                }

            },

            // Window becomes visible and then position is set.  Need fix.
            Event::WindowEvent { window_id, event: WindowEvent::RedrawRequested, .. } => manager.draw_window(window_id),
            Event::WindowEvent { window_id, event: WindowEvent::MouseInput { .. } } => manager.drop_window(window_id),
            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => *control_flow = ControlFlow::Exit,

            // Poll continuously runs the event loop, even if the os hasn't dispatched any events.
            // This is ideal for games and similar applications.
            _ => ()
            //_ => *control_flow = ControlFlow::Poll,
        }
    });
}
