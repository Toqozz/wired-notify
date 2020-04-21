use std::time::Duration;

use serde::Deserialize;

use crate::{
    rendering::blocks::{
        notification_block::NotificationBlockParameters,
        text_block::TextBlockParameters,
        scrolling_text_block::ScrollingTextBlockParameters,
        image_block::ImageBlockParameters,
    },
    maths_utility::{Vec2, Rect},
    config::{Config, AnchorPosition},
    rendering::window::NotifyWindow,
    wired_derive::DrawableLayoutElement,
};

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


// DrawableLayoutElement is implemented via a macro -- see /wired_derive/lib.rs.
// @IMPORTANT: DO NOT CACHE POSITIONS IN `predict_rect_and_init`! Real drawing uses a `master_offset`
// based on the result of `predict_rect_and_init` to make sure we don't draw off canvas, so
// the result of `LayoutBlock::find_anchor_pos()` can change between `predict_rect_and_init` and
// `draw`!
#[derive(Debug, Deserialize, Clone, DrawableLayoutElement)]
pub enum LayoutElement {
    NotificationBlock(NotificationBlockParameters),
    TextBlock(TextBlockParameters),
    ScrollingTextBlock(ScrollingTextBlockParameters),
    ImageBlock(ImageBlockParameters),
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

    // Call draw on each block in tree.
    pub fn draw_tree(&self, window: &NotifyWindow, parent_rect: &Rect, accum_rect: Rect) -> Rect {
        let rect = self.params.draw(&self.hook, &self.offset, parent_rect, window);
        let mut acc_rect = accum_rect.union(&rect);

        // Draw debug rect around bounding box.
        if Config::get().debug {
            let c = &Config::get().debug_color;
            window.context.set_source_rgba(c.r, c.g, c.b, c.a);
            window.context.set_line_width(1.0);
            window.context.rectangle(rect.x(), rect.y(), rect.width(), rect.height());
            window.context.stroke();
        }

        for child in &self.children {
            acc_rect = child.draw_tree(window, &rect, acc_rect);
        }

        acc_rect
    }

    // Predict the size of an entire layout, and initialize elements.
    pub fn predict_rect_tree_and_init(&mut self, window: &NotifyWindow, parent_rect: &Rect, accum_rect: Rect) -> Rect {
        // Predict size is supposed to be relatively cheap and lets us predict the size of elements,
        // so we can set window size and other stuff ahead of time.  We also initialize some stuff in
        // here to save performance.
        // `predict_rect_and_init` finds the bounding box of an individual element -- children are not
        // involved.
        let rect = self.params.predict_rect_and_init(&self.hook, &self.offset, parent_rect, window);
        let mut acc_rect = accum_rect.union(&rect);

        // Recursively get child rects.
        for child in &mut self.children {
            acc_rect = child.predict_rect_tree_and_init(window, &rect, acc_rect);
        }

        acc_rect
    }

    // Call update on each block in tree.
    pub fn update_tree(&mut self, delta_time: Duration) -> bool {
        let mut dirty = self.params.update(delta_time);
        for elem in &mut self.children {
            dirty |= elem.update_tree(delta_time);
        }

        dirty
    }
}

pub trait DrawableLayoutElement {
    fn draw(&self, hook: &Hook, offset: &Vec2, parent_rect: &Rect, window: &NotifyWindow) -> Rect;
    fn predict_rect_and_init(&mut self, hook: &Hook, offset: &Vec2, parent_rect: &Rect, window: &NotifyWindow) -> Rect;
    fn update(&mut self, _delta_time: Duration) -> bool { false }
}

