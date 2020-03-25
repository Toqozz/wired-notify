use serde::Deserialize;

// TODO: make this use blocks::NotificationBlockParameters instead.
use crate::rendering::blocks::notification_block::NotificationBlockParameters;
use crate::rendering::blocks::text_block::TextBlockParameters;
use crate::rendering::blocks::scrolling_text_block::ScrollingTextBlockParameters;
use crate::rendering::blocks::image_block::ImageBlockParameters;

use crate::types::maths::{ Vec2, Rect };
use crate::config::{ Padding, Color, AnchorPosition };
use crate::rendering::window::NotifyWindow;
use std::time::Duration;
use std::borrow::Borrow;

#[derive(Debug, Deserialize)]
pub struct LayoutBlock {
    pub hook: Hook,
    pub offset: Vec2,
    pub params: LayoutElement,
    pub children: Vec<LayoutBlock>,
}

impl Clone for LayoutBlock {
    fn clone(&self) -> Self {
        //let params = self.params.clone().init(self);

        Self {
            hook: self.hook.clone(),
            offset: self.offset.clone(),
            params: self.params.clone(),
            children: self.children.clone(),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct Hook {
    pub parent_hook: AnchorPosition,
    pub self_hook: AnchorPosition,
}

#[derive(Debug, Deserialize, Clone)]
pub enum LayoutElement {
    NotificationBlock(NotificationBlockParameters),
    TextBlock(TextBlockParameters),
    ScrollingTextBlock(ScrollingTextBlockParameters),
    ImageBlock(ImageBlockParameters),
}

impl DrawableLayoutElement for LayoutElement {
    fn draw(&self, hook: &Hook, offset: &Vec2, parent_rect: &Rect, window: &NotifyWindow) -> Rect {
        match self {
            LayoutElement::NotificationBlock(p) =>
                p.draw(hook, offset, parent_rect, window),
            LayoutElement::TextBlock(p) =>
                p.draw(hook, offset, parent_rect, window),
            LayoutElement::ScrollingTextBlock(p) =>
                p.draw(hook, offset, parent_rect, window),
            LayoutElement::ImageBlock(p) =>
                p.draw(hook, offset, parent_rect, window),
        }
    }

    fn predict_rect_and_init(&mut self, hook: &Hook, offset: &Vec2, parent_rect: &Rect, window: &NotifyWindow) -> Rect {
        match self {
            LayoutElement::NotificationBlock(p) =>
                p.predict_rect_and_init(hook, offset, parent_rect, window),
            LayoutElement::TextBlock(p) =>
                p.predict_rect_and_init(hook, offset, parent_rect, window),
            LayoutElement::ScrollingTextBlock(p) =>
                p.predict_rect_and_init(hook, offset, parent_rect, window),
            LayoutElement::ImageBlock(p) =>
                p.predict_rect_and_init(hook, offset, parent_rect, window),
        }
    }

    fn update(&mut self, delta_time: Duration) -> bool { 
        match self {
            LayoutElement::NotificationBlock(p) => p.update(delta_time),
            LayoutElement::TextBlock(p) => p.update(delta_time),
            LayoutElement::ScrollingTextBlock(p) => p.update(delta_time),
            LayoutElement::ImageBlock(p) => p.update(delta_time),
        }
    }
}

impl LayoutBlock {
    pub fn find_anchor_pos(hook: &Hook, offset: &Vec2, parent_rect: &Rect, self_rect: &Rect) -> Vec2 {
        // Get position of anchor in each rectangle (parent and self).
        let mut anchor = hook.parent_hook.get_pos(parent_rect);
        let self_anchor = hook.self_hook.get_pos(self_rect);

        // To align the anchor of parent rect and self rect, we just need to move the parent anchor
        // by whatever the offset is for the self rect.
        anchor.x -= self_anchor.x;
        anchor.y -= self_anchor.y;

        // The `offset` config option is just applied on top.
        anchor.x += offset.x;
        anchor.y += offset.y;

        anchor
    }

    // Run a function on each element in the layout, optionally passing in the function's return value.
    pub fn traverse<T, F: Copy>(&self, func: F, pass: &T)
        where F: Fn(&Self, &T) -> T {

        let result = func(self, pass);

        for elem in &self.children {
            //let result = func(elem, pass);
            elem.traverse(func, &result);
        }
    }

    pub fn draw_tree(&self, window: &NotifyWindow, parent_rect: &Rect, accum_rect: Rect) -> Rect {
        // Draw debug rect around bounding box.
        /*
        TODO: make config lazy so globally accessable at all times.
        if self.config.debug {
            self.context.set_source_rgba(1.0, 0.0, 0.0, 1.0);
            self.context.set_line_width(1.0);
            self.context.rectangle(rect.x(), rect.y(), rect.width(), rect.height());
            self.context.stroke();
        }
        */

        let rect = self.params.draw(&self.hook, &self.offset, parent_rect, window);
        let mut acc_rect = accum_rect.union(&rect);

        for child in &self.children {
            acc_rect = child.draw_tree(window, &rect, acc_rect);
            //acc_rect = acc_rect.union(&child_rect);
        }

        acc_rect
    }

    // Predict the size of an entire layout.
    pub fn predict_rect_tree(&mut self, window: &NotifyWindow, parent_rect: &Rect, accum_rect: Rect) -> Rect {
        // Predict size is relatively cheap and lets us predict the size of elements, so we can set window size and other stuff
        // ahead of time.
        // `predict_rect_independent` finds the bounding box of an individual layout -- children are not involved.
        let rect = self.params.predict_rect_and_init(&self.hook, &self.offset, parent_rect, window);
        let mut acc_rect = accum_rect.union(&rect);

        // Recursively get child rects.
        for child in &mut self.children {
            acc_rect = child.predict_rect_tree(window, &rect, acc_rect);
            //acc_rect = acc_rect.union(&child_rect);
        }

        acc_rect
    }

    pub fn update_tree(&mut self, delta_time: Duration) -> bool {
        let mut dirty = self.params.update(delta_time);
        for elem in &mut self.children {
            //let result = func(elem, pass);
            dirty |= elem.update_tree(delta_time);
        }

        dirty
    }
}

pub trait DrawableLayoutElement {
    fn draw(&self, hook: &Hook, offset: &Vec2, parent_rect: &Rect, window: &NotifyWindow) -> Rect;
    fn predict_rect_and_init(&mut self, hook: &Hook, offset: &Vec2, parent_rect: &Rect, window: &NotifyWindow) -> Rect;
    fn update(&mut self, delta_time: Duration) -> bool { false }
}
