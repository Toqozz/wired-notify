use dbus::message::SignalArgs;
use dbus::strings::Path;
use serde::Deserialize;

use crate::bus;
use crate::bus::dbus_codegen::OrgFreedesktopNotificationsActionInvoked;
use crate::config::{Color, Config, Padding};
use crate::maths_utility;
use crate::maths_utility::{MinMax, Rect, Vec2};
use crate::rendering::{
    layout::{DrawableLayoutElement, Hook, LayoutBlock},
    text::EllipsizeMode,
    window::NotifyWindow,
};

#[derive(Debug, Deserialize, Clone)]
pub struct Dimensions {
    width: MinMax,
    height: MinMax,
}

#[derive(Debug, Deserialize, Clone)]
pub enum Action {
    DefaultAction,
    OtherAction(usize),
}

#[derive(Debug, Deserialize, Clone)]
pub struct ButtonBlockParameters {
    pub padding: Padding,
    pub action: Action,
    pub text: String,
    pub font: String,
    pub border_width: f64,
    pub border_rounding: f64,
    pub text_color: Color,
    pub border_color: Color,
    pub background_color: Color,
    pub dimensions: Dimensions,

    // -- Optional fields
    pub text_color_hovered: Option<Color>,
    pub border_color_hovered: Option<Color>,
    pub background_color_hovered: Option<Color>,
    #[serde(default)]
    pub ellipsize: EllipsizeMode,

    // -- Runtime fields
    #[serde(skip)]
    real_text: String,
    #[serde(skip)]
    key: String,
    #[serde(skip)]
    hover: bool,
}

impl ButtonBlockParameters {
    fn text_color(&self) -> &Color {
        if self.hover && self.text_color_hovered.is_some() {
            self.text_color_hovered.as_ref().unwrap()
        } else {
            &self.text_color
        }
    }

    fn border_color(&self) -> &Color {
        if self.hover && self.border_color_hovered.is_some() {
            self.border_color_hovered.as_ref().unwrap()
        } else {
            &self.border_color
        }
    }

    fn background_color(&self) -> &Color {
        if self.hover && self.background_color_hovered.is_some() {
            self.background_color_hovered.as_ref().unwrap()
        } else {
            &self.background_color
        }
    }
}

// Much of this is the same as TextBlock, see there for documentation.
impl DrawableLayoutElement for ButtonBlockParameters {
    fn draw(&self, hook: &Hook, offset: &Vec2, parent_rect: &Rect, window: &NotifyWindow) -> Rect {
        let text_col = self.text_color();
        let border_col = self.border_color();
        let background_col = self.background_color();

        // Get would-be text pos and set the text for drawing later.
        window.text.set_text(
            &self.real_text,
            &self.font,
            self.dimensions.width.max,
            self.dimensions.height.max,
            &self.ellipsize,
        );
        let mut rect = window.text.get_sized_padded_rect(
            &self.padding,
            self.dimensions.width.min,
            self.dimensions.height.min,
        );
        let pos = LayoutBlock::find_anchor_pos(hook, offset, parent_rect, &rect);

        // Button background.
        maths_utility::cairo_rounded_bordered_rectangle(
            &window.context,
            pos.x,
            pos.y, // x, y
            rect.width(),
            rect.height(),
            self.border_rounding,
            self.border_width,
            border_col,
            background_col,
        );

        window.context.set_operator(cairo::Operator::Over);
        // Move block to text position (ignoring padding) for draw operation.
        window
            .text
            .paint_padded(&window.context, &pos, text_col, &self.padding);

        // Debug, unpadded drawing, to help users.
        if Config::get().debug {
            let r = window
                .text
                .get_sized_rect(self.dimensions.width.min, self.dimensions.height.min);
            maths_utility::debug_rect(
                &window.context,
                true,
                pos.x + self.padding.left,
                pos.y + self.padding.top,
                r.width(),
                r.height(),
            );
        }

        rect.set_xy(pos.x, pos.y);
        rect
    }

    fn predict_rect_and_init(
        &mut self,
        hook: &Hook,
        offset: &Vec2,
        parent_rect: &Rect,
        window: &NotifyWindow,
    ) -> Rect {
        let maybe_action = match self.action {
            Action::DefaultAction => window.notification.get_default_action(),
            Action::OtherAction(i) => window.notification.get_other_action(i),
        };

        let (key, text) = maybe_action.unwrap_or(("".to_owned(), "".to_owned()));
        let text = maths_utility::format_action_notification_string(
            &self.text,
            &text,
            &window.notification,
        );
        self.key = key;

        window.text.set_text(
            &text,
            &self.font,
            self.dimensions.width.max,
            self.dimensions.height.max,
            &self.ellipsize,
        );
        let mut rect = window.text.get_sized_padded_rect(
            &self.padding,
            self.dimensions.width.min,
            self.dimensions.height.min,
        );

        self.real_text = text;

        let pos = LayoutBlock::find_anchor_pos(hook, offset, parent_rect, &rect);
        rect.set_xy(pos.x, pos.y);
        rect
    }

    fn clicked(&mut self, window: &NotifyWindow) -> bool {
        let message = OrgFreedesktopNotificationsActionInvoked {
            action_key: self.key.clone(),
            id: window.notification.id,
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
