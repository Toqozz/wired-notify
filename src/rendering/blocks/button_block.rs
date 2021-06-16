use serde::Deserialize;
use dbus::strings::Path;
use dbus::message::SignalArgs;

use crate::maths_utility::{Vec2, Rect, MinMax};
use crate::config::{Config, Padding, Color};
use crate::bus;
use crate::bus::dbus::Notification;
use crate::bus::dbus_codegen::{OrgFreedesktopNotificationsActionInvoked, OrgFreedesktopNotificationsNotificationClosed};
use crate::rendering::{
    window::NotifyWindow,
    layout::{DrawableLayoutElement, LayoutBlock, Hook},
    text::EllipsizeMode,
};
use crate::maths_utility;

#[derive(Debug, Deserialize, Clone)]
pub struct Dimensions {
    width: MinMax,
    height: MinMax,
}

#[derive(Debug, Deserialize, Clone)]
pub enum Action {
    Primary,
    Other(usize),
}

#[derive(Debug, Deserialize, Clone)]
pub struct ButtonBlockParameters {
    pub action: Action,
    pub font: String,
    pub color: Color,
    pub ellipsize: EllipsizeMode,
    pub padding: Padding,
    pub dimensions: Dimensions,

    // -- Optional fields
    pub color_hovered: Option<Color>,
    #[serde(default)]
    pub render_when_empty: bool,

    // -- Runtime fields
    #[serde(skip)]
    real_text: String,
    #[serde(skip)]
    key: String,
    #[serde(skip)]
    hover: bool,
}

// Much of this is the same as TextBlock, see there for documentation.
impl DrawableLayoutElement for ButtonBlockParameters {
    fn draw(&self, hook: &Hook, offset: &Vec2, parent_rect: &Rect, window: &NotifyWindow) -> Rect {
        if self.real_text.is_empty() && !self.render_when_empty {
            let pos = LayoutBlock::find_anchor_pos(hook, offset, parent_rect, &Rect::EMPTY);
            return Rect::new(pos.x, pos.y, 0.0, 0.0);
        }

        window.context.set_operator(cairo::Operator::Over);

        window.text.set_text(
            &self.real_text,
            &self.font,
            self.dimensions.width.max,
            self.dimensions.height.max,
            &self.ellipsize
        );
        let mut rect = window.text.get_sized_padded_rect(
            &self.padding,
            self.dimensions.width.min,
            self.dimensions.height.min
        );

        let pos = LayoutBlock::find_anchor_pos(hook, offset, parent_rect, &rect);

        let col = if self.hover { self.color_hovered.as_ref().unwrap_or(&self.color) } else { &self.color };
        // Move block to text position (ignoring padding) for draw operation.
        window.text.paint_padded(&window.context, &pos, col, &self.padding);
        // Debug, unpadded drawing, to help users.
        if Config::get().debug {
            let r = window.text.get_sized_rect(self.dimensions.width.min, self.dimensions.height.min);
            maths_utility::debug_rect(&window.context, true, pos.x + self.padding.left, pos.y + self.padding.top, r.width(), r.height());
        }

        rect.set_xy(pos.x, pos.y);
        rect
    }

    fn predict_rect_and_init(&mut self, hook: &Hook, offset: &Vec2, parent_rect: &Rect, window: &NotifyWindow) -> Rect {
        let maybe_text = match self.action {
            Action::Primary => {
                self.key = "default".to_owned();
                window.notification.actions.get("default").cloned()
            },
            Action::Other(i) => {
                // Creates an iterator without the "default" key, which is preserved for action1.
                let mut keys = window.notification.actions.keys().filter(|s| *s != "default");
                let maybe_key = keys.nth(i);
                if let Some(key) = maybe_key {
                    self.key = key.to_owned();
                    window.notification.actions.get(key).cloned()
                } else {
                    None
                }
            }
        };

        let text = maybe_text.unwrap_or("".to_owned());

        if text.is_empty() && !self.render_when_empty {
            self.real_text = text;
            let pos = LayoutBlock::find_anchor_pos(hook, offset, parent_rect, &Rect::EMPTY);
            return Rect::new(pos.x, pos.y, 0.0, 0.0);
        }

        window.text.set_text(
            &text,
            &self.font,
            self.dimensions.width.max,
            self.dimensions.height.max,
            &self.ellipsize
        );
        let mut rect = window.text.get_sized_padded_rect(
            &self.padding,
            self.dimensions.width.min,
            self.dimensions.height.min
        );

        self.real_text = text;

        let pos = LayoutBlock::find_anchor_pos(hook, offset, parent_rect, &rect);
        rect.set_xy(pos.x, pos.y);
        rect
    }

    /*
    fn update(&mut self, delta_time: Duration, _window: &NotifyWindow) -> bool {
        let message = OrgFreedesktopNotificationsActionInvoked {
            action_key: k.to_owned(), id: notification.id
        };
        let path = Path::new(bus::dbus::PATH).expect("Failed to create DBus path.");
        let _result = bus::dbus::get_connection().send(message.to_emit_message(&path));
    }
    */

    fn clicked(&mut self, window: &NotifyWindow) -> bool {
        let message = OrgFreedesktopNotificationsActionInvoked {
            action_key: self.key.clone(), id: window.notification.id
        };
        let path = Path::new(bus::dbus::PATH).expect("Failed to create DBus path.");
        let _result = bus::dbus::get_connection().send(message.to_emit_message(&path));
        false
    }

    fn hovered(&mut self, entered: bool, _window: &NotifyWindow) -> bool {
        self.hover = entered;
        true
    }
}
