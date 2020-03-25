use serde::Deserialize;

use crate::types::maths::{self, Rect, Vec2};
use crate::config::{Padding, Color};
use crate::rendering::window::NotifyWindow;
use crate::rendering::layout::{LayoutBlock, DrawableLayoutElement, Hook};
use std::time::Duration;

#[derive(Debug, Deserialize, Clone)]
pub struct ScrollingTextBlockParameters {
    pub padding: Padding,
    pub text: String,
    pub font: String,
    pub color: Color,
    pub max_width: i32,
    pub scroll_speed: f64,
    pub lhs_dist: f64,
    pub rhs_dist: f64,
    pub bounce: bool,

    pub scroll_t: f64,

    #[serde(skip)]
    clip_rect: Rect,
    #[serde(skip)]
    bounce_left: f64,
    #[serde(skip)]
    bounce_right: f64,
}

impl DrawableLayoutElement for ScrollingTextBlockParameters {
    fn draw(&self, hook: &Hook, offset: &Vec2, parent_rect: &Rect, window: &NotifyWindow) -> Rect {
        window.context.save();

        let mut text = self.text.clone();
        text = text
            .replace("%s", &window.notification.summary)
            .replace("%b", &window.notification.body);

        window.text.set_text(&text, &self.font, -1, 0);
        let mut rect = window.text.get_rect(&self.padding);

        //let mut pos = self.find_anchor_pos(parent_rect, &rect);
        let mut pos = LayoutBlock::find_anchor_pos(hook, offset, parent_rect, &rect);
        pos.x += self.padding.left;
        pos.y += self.padding.top;

        // If we're larger than the max size, then we should scroll, which is just changing the
        // text's x position really.
        if rect.width() > self.max_width as f64 {
            //let clip_right = self.max_width as f64 - self.padding.right;

            dbg!(&self.clip_rect);
            window.context.rectangle(
                self.clip_rect.x(),
                self.clip_rect.y(),
               self.clip_rect.width(),
                self.clip_rect.height()
            );
            window.context.clip();

            let lerp = maths::lerp(self.bounce_right, self.bounce_left, self.scroll_t);
            dbg!(maths::distance(lerp, pos.x));
            pos.x = lerp;
        }

        window.text.paint(&window.context, &pos, &self.color);
        pos.x -= self.padding.left;
        pos.y -= self.padding.top;

        window.context.restore();

        rect.set_xy(
            pos.x - self.padding.left,
            pos.y - self.padding.top
        );
        rect
    }

    fn predict_rect_and_init(&mut self, hook: &Hook, offset: &Vec2, parent_rect: &Rect, window: &NotifyWindow) -> Rect {
        let mut text = self.text.clone();
        text = text
            .replace("%s", &window.notification.summary)
            .replace("%b", &window.notification.body);

        window.text.set_text(&text, &self.font, self.max_width, 0);
        let mut rect = window.text.get_rect(&self.padding);
        let mut clip_rect = window.text.get_rect(&Padding::new(0.0, 0.0, 0.0, 0.0));

        window.text.set_text(&text, &self.font, -1, 0);
        let text_rect = window.text.get_rect(&self.padding);

        let mut pos = LayoutBlock::find_anchor_pos(hook, offset, parent_rect, &clip_rect);
        //pos.x += self.padding.left;
        //pos.y += self.padding.top;

        // TODO: clean up this shit and document.
        // TODO: also add dynamic scroll option.
        clip_rect.set_xy(pos.x + self.padding.left, pos.y + self.padding.top);
        // Q: Why subtract self.padding.right?
        // A: Because `rect` is already padded *and* has been moved right and down to align
        // text according to the other padding.  Therefore, we're actually over the right hand
        // side line.
        //let clip_right = self.max_width as f64 - self.padding.right;
        // pos.x is basically clip_rect.left() (pretending we have a clip_rect).
        self.bounce_left = clip_rect.left() + self.lhs_dist;
        self.bounce_right = clip_rect.right() + self.rhs_dist - text_rect.width();
        self.clip_rect = clip_rect;

        rect.set_xy(pos.x, pos.y);
        rect
    }

    fn update(&mut self, delta_time: Duration) -> bool {
        let distance = maths::distance(self.bounce_left, self.bounce_right);
        //dbg!(distance);
        self.scroll_t +=
            delta_time.as_secs_f64() * self.scroll_speed * (self.max_width as f64 / distance);

        // If scrolling right.
        if self.scroll_speed > 0.0 {
            // If reached right edge.
            if self.scroll_t >= 1.0 {
                // Reverse.
                self.scroll_speed = -self.scroll_speed;
            }
        } else if self.scroll_speed < 0.0 {
            // If reached left edge.
            if self.scroll_t <= 0.0 {
                // Reverse.
                self.scroll_speed = -self.scroll_speed;
            }
        }

        true
    }
}

