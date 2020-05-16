use std::time::Duration;

use serde::Deserialize;

use crate::maths_utility::{self, Vec2, Rect};
use crate::config::Color;
use crate::bus::dbus::Urgency;
use crate::rendering::window::{NotifyWindow, UpdateModes};
use crate::rendering::layout::{DrawableLayoutElement, Hook};

#[derive(Debug, Deserialize, Clone)]
pub struct NotificationBlockParameters {
    pub monitor: u32,
    //pub monitor_hook: (AnchorPosition, AnchorPosition),
    //pub monitor_offset: Vec2,

    pub border_width: f64,
    pub border_rounding: f64,
    pub background_color: Color,
    pub border_color_urgency_low: Color,
    pub border_color_urgency_normal: Color,
    pub border_color_urgency_critical: Color,
    pub border_color_paused: Color,

    pub gap: Vec2,
    pub notification_hook: Hook,

    #[serde(skip)]
    current_update_mode: UpdateModes,
}

impl DrawableLayoutElement for NotificationBlockParameters {
    fn draw(&self, _hook: &Hook, _offset: &Vec2, parent_rect: &Rect, window: &NotifyWindow) -> Rect {
        // Clear
        window.context.set_operator(cairo::Operator::Clear);
        window.context.paint();

        window.context.set_operator(cairo::Operator::Source);

        // Draw border + background.
        // If anything isn't updating, we count it as paused, which overrides urgency.
        // Otherwise, we evaluate urgency.
        let bd_color = {
            if window.update_mode != UpdateModes::all() {
                &self.border_color_paused
            } else {
                match window.notification.urgency {
                    Urgency::Low => &self.border_color_urgency_low,
                    Urgency::Normal => &self.border_color_urgency_normal,
                    Urgency::Critical => &self.border_color_urgency_critical,
                }
            }
        };

        //let bd_color = &self.border_color;
        window.context.set_source_rgba(bd_color.r, bd_color.g, bd_color.b, bd_color.a);
        window.context.paint();

        let bg_color = &self.background_color;
        let bw = &self.border_width;
        window.context.set_source_rgba(bg_color.r, bg_color.g, bg_color.b, bg_color.a);
        maths_utility::cairo_rounded_rectangle(
            &window.context,
            *bw, *bw,   // x, y
            parent_rect.width() - bw * 2.0, parent_rect.height() - bw * 2.0,
            self.border_rounding,
        );
        window.context.fill();
        /*
        window.context.rectangle(
            *bw, *bw,     // x, y
            parent_rect.width() - bw * 2.0, parent_rect.height() - bw * 2.0,
        );
        */

        // Base notification background doesn't actually take up space, so use same rect.
        parent_rect.clone()
    }

    fn predict_rect_and_init(&mut self, _hook: &Hook, _offset: &Vec2, parent_rect: &Rect, window: &NotifyWindow) -> Rect {
        self.current_update_mode = window.update_mode;
        parent_rect.clone()
    }

    fn update(&mut self, _delta_time: Duration, window: &NotifyWindow) -> bool {
        if window.update_mode != self.current_update_mode {
            self.current_update_mode = window.update_mode;
            return true;
        }

        false
    }
}

