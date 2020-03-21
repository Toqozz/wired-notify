use serde::Deserialize;

// TODO: make this use blocks::NotificationBlockParameters instead.
use crate::rendering::blocks::notification_block::NotificationBlockParameters;
use crate::rendering::blocks::text_block::TextBlockParameters;
use crate::rendering::blocks::scrollable_text_block::ScrollingTextBlockParameters;
use crate::rendering::blocks::image_block::ImageBlockParameters;

use crate::types::maths::{self, Vec2, Rect};
use crate::config::{Padding, Color, AnchorPosition};
use crate::rendering::window::NotifyWindow;
use image::{FilterType, GenericImageView};
use cairo::ImageSurface;
use std::time::Duration;

#[derive(Debug, Deserialize, Clone)]
pub struct LayoutBlock {
    pub hook: Hook,
    pub offset: Vec2,
    pub params: LayoutElement,
    pub children: Vec<LayoutBlock>,
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

impl LayoutBlock {
    pub fn find_anchor_pos(hook: &Hook, offset: &Vec2, parent_rect: &Rect, self_rect: &Rect) -> Vec2 {
        //let hook = &self.hook;
        //let offset = &self.offset;

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

    pub fn update(&mut self, delta_time: Duration) -> bool {
        let mut dirty = false;

        match &mut self.params {
            LayoutElement::NotificationBlock(p) => {
                dirty |= p.update(delta_time);
            }

            LayoutElement::TextBlock(p) => {
                dirty |= p.update(delta_time);
            }

            LayoutElement::ScrollingTextBlock(p) => {
                dirty |= p.update(delta_time);
            }

            LayoutElement::ImageBlock(p) => {
                dirty |= p.update(delta_time);
            }
        };

        dirty
    }

    // Predict size is relatively cheap and lets us predict the size of elements, so we can set window size ahead of time.
    // Predicts the size of an individual block.
    pub fn predict_rect_independent(&self, parent_rect: &Rect, window: &NotifyWindow) -> Rect {
        let rect = match &self.params {
            LayoutElement::NotificationBlock(p) => {
                p.predict_rect_independent(&self.hook, &self.offset, parent_rect, window)
            }

            LayoutElement::TextBlock(p) => {
                p.predict_rect_independent(&self.hook, &self.offset, parent_rect, window)
            }

            LayoutElement::ScrollingTextBlock(p) => {
                p.predict_rect_independent(&self.hook, &self.offset, parent_rect, window)
            }

            LayoutElement::ImageBlock(p) => {
                p.predict_rect_independent(&self.hook, &self.offset, parent_rect, window)
            }

            _ => Rect::new(0.0, 0.0, 0.0, 0.0)
        };

        rect
    }

    pub fn draw_independent(&self, parent_rect: &Rect, window: &NotifyWindow) -> Rect {
        let rect = match &self.params {
            LayoutElement::NotificationBlock(p) => {
                p.draw_independent(&self.hook, &self.offset, parent_rect, window)
            }

            LayoutElement::TextBlock(p) => {
                p.draw_independent(&self.hook, &self.offset, parent_rect, window)
            }

            LayoutElement::ScrollingTextBlock(p) => {
                p.draw_independent(&self.hook, &self.offset, parent_rect, window)
            }

            LayoutElement::ImageBlock(p) => {
                p.draw_independent(&self.hook, &self.offset, parent_rect, window)
            }

            _ => Rect::new(0.0, 0.0, 0.0, 0.0)
        };

        rect
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

    pub fn traverse_update(&mut self, delta_time: Duration) -> bool {
        let mut dirty = self.update(delta_time);
        for elem in &mut self.children {
            //let result = func(elem, pass);
            dirty |= elem.traverse_update(delta_time);
        }

        dirty
    }

    // Predict the size of an entire layout.
    pub fn predict_rect_tree(&self, window: &NotifyWindow, parent_rect: &Rect, accum_rect: &Rect) -> Rect {
        // `predict_rect_independent` finds the bounding box of an individual layout -- children are not involved.
        let rect = self.predict_rect_independent(parent_rect, window);
        let mut acc_rect = accum_rect.union(&rect);

        // Recursively get child rects.
        for child in &self.children {
            let child_rect = child.predict_rect_tree(window, &rect, &acc_rect);
            acc_rect = acc_rect.union(&child_rect);
        }

        acc_rect
    }

    /*
    // Run a function on each child in layout (recursively), accumulating the return value of the function using an accumulator.
    pub fn traverse_accum<T, F: Copy, N: Copy>(&self, func: F, accumulator: N, accum: &T, pass: &T) -> T
        where F: Fn(&Self, &T) -> T,
              N: Fn(&T, &T) -> T {

        let result = func(self, pass);
        let mut acc = accumulator(&result, &accum);
        for elem in self.children() {
            //let result = func(elem, pass);
            acc = accumulator(&elem.traverse_accum(func, accumulator, &accum, &result), &acc);
        }

        acc
    }
    */
}

pub trait DrawableLayoutElement {
    fn draw_independent(&self, hook: &Hook, offset: &Vec2, parent_rect: &Rect, window: &NotifyWindow) -> Rect;
    fn predict_rect_independent(&self, hook: &Hook, offset: &Vec2, parent_rect: &Rect, window: &NotifyWindow) -> Rect;
    fn update(&mut self, delta_time: Duration) -> bool { false }
}
