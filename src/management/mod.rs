use std::time::Duration;
use std::process::{Command, Stdio};
use std::collections::HashMap;

use winit::{
    event_loop::EventLoopWindowTarget,
    window::WindowId,
    event::ElementState,
    event::MouseButton,
    event::WindowEvent,
    event,
};

use crate::{
    rendering::window::{NotifyWindow, UpdateModes},
    rendering::layout::{LayoutElement, LayoutBlock},
    //notification::Notification,
    bus::dbus::Notification,
    maths_utility::Rect,
    config::Config,
};

pub struct NotifyWindowManager {
    //pub windows: Vec<NotifyWindow<'config>>,
    pub base_window: winit::window::Window,
    pub monitor_windows: HashMap<u32, Vec<NotifyWindow>>,
    pub dirty: bool,
}

impl NotifyWindowManager {
    pub fn new(el: &EventLoopWindowTarget<()>) -> Self {
        let monitor_windows = HashMap::new();

        let base_window = winit::window::WindowBuilder::new()
            .with_visible(false)
            .build(el)
            .expect("Failed to create base window.");

        Self {
            base_window,
            monitor_windows,
            dirty: false,
        }
    }

    // Summon a new notification.
    pub fn new_notification(&mut self, notification: Notification, el: &EventLoopWindowTarget<()>) {
        if let LayoutElement::NotificationBlock(p) = &Config::get().layout.params {
            let window = NotifyWindow::new(el, notification, &self);

            let windows = self.monitor_windows
                .entry(p.monitor)
                .or_insert(vec![]);

            // Push a new notification window.
            windows.push(window);

            // If we've exceeded max notifications, then mark the top-most one for destroy.
            let cfg = Config::get();
            if cfg.max_notifications > 0 && windows.len() > cfg.max_notifications {
                windows.first_mut().unwrap().marked_for_destroy = true;
            }


            // Outer state is now out of sync with internal state because we have an invisible notification.
            self.dirty = true;
        }
    }

    pub fn update(&mut self, delta_time: Duration) {
        // Returning dirty from a window update means the window has been deleted / needs
        // positioning updated.
        for (_monitor, windows) in &mut self.monitor_windows {
            for window in windows {
                self.dirty |= window.update(delta_time);
            }
        }

        if self.dirty {
            self.update_positions();
            // Finally drop windows.
            for (_monitor_id, windows) in &mut self.monitor_windows {
                windows.retain(|w| !w.marked_for_destroy);
            }
        }
    }

    fn update_positions(&mut self) {
        let cfg = Config::get();
        // TODO: gotta do something about this... can't I just cast it?
        if let LayoutElement::NotificationBlock(p) = &cfg.layout.params {
            for (monitor_id, windows) in &self.monitor_windows {
                // If there are no windows for this monitor, leave it alone.
                if windows.len() == 0 {
                    continue;
                }

                // Grab a winit window reference to use to call winit functions.
                // The functions here are just convenience functions to avoid needing a reference
                // to the event loop -- they don't relate to the individual window.
                let winit_utility = &windows[0].winit;
                let monitor = winit_utility
                    .available_monitors()
                    .nth(*monitor_id as usize)
                    .unwrap_or(winit_utility.primary_monitor());

                let (pos, size) = (monitor.position(), monitor.size());
                let monitor_rect = Rect::new(pos.x.into(), pos.y.into(), size.width.into(), size.height.into());
                let mut prev_rect = monitor_rect;

                let mut real_idx = 0;
                for i in 0..windows.len() {
                    // Windows which are marked for destroy should be overlapped so that destroying them
                    // will be less noticeable.
                    if windows[i].marked_for_destroy {
                        continue;
                    }

                    let window = &windows[i];
                    let mut window_rect = window.get_inner_rect();

                    // For the first notification, we attach to the monitor.
                    // For the second and more notifications, we attach to the previous
                    // notification.
                    let pos = if real_idx == 0 {
                        LayoutBlock::find_anchor_pos(
                            &cfg.layout.hook,
                            &cfg.layout.offset,
                            &prev_rect,
                            &Rect::EMPTY,
                        )
                    } else {
                        LayoutBlock::find_anchor_pos(
                            &p.notification_hook,
                            &p.gap,
                            &prev_rect,
                            &window_rect,
                        )
                    };

                    // Note: `set_position` doesn't happen instantly.  If we read
                    // `get_rect()`s position straight after this call it probably won't be correct,
                    // which is why we `set_xy` manually after.
                    window.set_position(pos.x, pos.y);
                    window.set_visible(true);

                    window_rect.set_xy(pos.x, pos.y);
                    prev_rect = window_rect;

                    real_idx += 1;
                }
            }
        } else {
            // Panic because the config must have not been setup properly.
            // @TODO: move this panic to config verify, and then just unwrap or something instead.
            panic!("The root LayoutElement must be a NotificationBlock!");
        }

        // Outer state is now up to date with internal state.
        self.dirty = false;
    }

    pub fn process_event(&mut self, window_id: WindowId, event: event::WindowEvent) {
        // Simplify button presses into a uint, which matches our config.
        let pressed = match event {
            WindowEvent::MouseInput { state: ElementState::Pressed, button, .. } => {
                match button {
                    MouseButton::Left => Some(1),
                    MouseButton::Right => Some(2),
                    MouseButton::Middle => Some(3),
                    MouseButton::Other(u) => Some(u),
                }
            }
            _ => None,
        };

        // Match button press to config.
        let config = Config::get();
        if let Some(button) = pressed {
            if button == config.shortcuts.notification_close {
                self.drop_window(window_id);

            } else if button == config.shortcuts.notification_closeall {
                self.drop_windows();

            } else if button == config.shortcuts.notification_url {
                let mut body = String::from("");
                if let Some((monitor, idx)) = self.find_window_idx(window_id) {
                    let window = self.monitor_windows.get(&monitor).unwrap().get(idx).unwrap();
                    body = window.notification.body.clone();
                }

                find_and_open_url(body);

            } else if button == config.shortcuts.notification_pause {
                if let Some((monitor, idx)) = self.find_window_idx(window_id) {
                    let window = self.monitor_windows
                        .get_mut(&monitor).unwrap()
                        .get_mut(idx).unwrap();

                    window.update_mode.toggle(UpdateModes::FUSE);
                    //window.update_enabled = !window.update_enabled;
                }
            }
        }
    }

    // Find window across all monitors based on an id, which we receive from winit events.
    pub fn find_window_idx(&self, window_id: WindowId) -> Option<(u32, usize)> {
        for (monitor, windows) in &self.monitor_windows {
            let found = windows.iter().position(|w| w.winit.id() == window_id);
            if let Some(idx) = found {
                return Some((*monitor, idx))
            }
        }

        None
    }

    pub fn drop_window(&mut self, window_id: WindowId) {
        if let Some((monitor, idx)) = self.find_window_idx(window_id) {
            let window = self.monitor_windows
                .get_mut(&monitor).unwrap()
                .get_mut(idx).unwrap();

            window.marked_for_destroy = true;
            self.dirty = true;
        }
    }

    // @TODO: how about a shortcut for dropping all windows on one monitor?
    pub fn drop_windows(&mut self) {
        for (_monitor, windows) in &mut self.monitor_windows {
            for window in windows.iter_mut() {
                window.marked_for_destroy = true;
                self.dirty = true;
            }
        }
    }

    // Draw an individual window (mostly for expose events).
    pub fn draw_window(&self, window_id: WindowId) {
        if let Some((monitor, idx)) = self.find_window_idx(window_id) {
            let window = self.monitor_windows
                .get(&monitor).unwrap()
                .get(idx).unwrap();

            window.draw();
        }
    }

    // Draw all windows.
    pub fn _draw_windows(&self) {
        for (_monitor, windows) in &self.monitor_windows {
            for window in windows.iter() {
                window.draw();
            }
        }
    }
}

fn find_and_open_url(string: String) {
    // This would be cleaner with regex, but we want to avoid the dependency.
    // Find the first instance of either "http://" or "https://" and then split the
    // string at the end of the word.
    let idx = string.find("http://").or_else(|| string.find("https://"));
    let maybe_url = if let Some(i) = idx {
        let (_, end) = string.split_at(i);
        end.split_whitespace().next()
    } else {
        eprintln!("Was requested to open a url but couldn't find one in the specified string");
        None
    };

    if let Some(url) = maybe_url {
        // `xdg-open` can be blocking, so opening like this can block our whole program because
        // we're grabbing the command's status at the end (which will cause it to wait).
        // I think it's important that we report at least some status back in case of error, so
        // we use `spawn()` instead.
        /*
        let status = Command::new("xdg-open").arg(url).status();
        if status.is_err() {
            eprintln!("Tried to open a url using xdg-open, but the command failed: {:?}", status);
        }
        */

        // For some reason, Ctrl-C closes child processes, even when they're detached
        // (`thread::spawn`), but `SIGINT`, `SIGTERM`, `SIGKILL`, and more (?) don't.
        // Maybe it's this: https://unix.stackexchange.com/questions/149741/why-is-sigint-not-propagated-to-child-process-when-sent-to-its-parent-process
        let child = Command::new("xdg-open")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .arg(url)
            .spawn();

        if child.is_err() {
            eprintln!("Tried to open a url using xdg-open, but the command failed: {:?}", child);
        }
    }
}
