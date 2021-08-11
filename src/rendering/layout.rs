use std::time::Duration;

use serde::Deserialize;

use crate::{
    rendering::blocks::*,
    maths_utility::{Vec2, Rect},
    config::{Config, AnchorPosition},
    rendering::window::NotifyWindow,
    wired_derive::DrawableLayoutElement,
};

#[derive(Debug, Deserialize, Clone)]
pub struct LayoutBlock {
    pub name: String,
    pub parent: String,
    pub hook: Hook,
    pub offset: Vec2,
    pub params: LayoutElement,
    // Used for deciding when a block should or shouldn't be rendered.
    // Lets users have the freedom of deciding when blocks should / shouldn't be rendered.
    // Defaults to always none (always rendered).
    #[serde(default)]
    pub render_criteria: Vec<RenderCriteria>,
    #[serde(skip)]
    pub children: Vec<LayoutBlock>,

    // The most recent rect that has been drawn.
    // This is updated every draw, so should always be accurate.
    #[serde(skip)]
    pub cache_rect: Rect,
    #[serde(skip)]
    pub hovered: bool,
}

#[derive(Debug, Deserialize, Clone)]
pub enum RenderCriteria {
    Summary,
    Body,
    HintImage,
    AppImage,
    AppName,
    ActionDefault,
    ActionOther(usize),
}

#[derive(Debug, Deserialize, Clone)]
pub struct Hook {
    pub parent_anchor: AnchorPosition,
    pub self_anchor: AnchorPosition,
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
    ButtonBlock(ButtonBlockParameters),
    ProgressBlock(ProgressBlockParameters),
}

impl LayoutBlock {
    pub fn should_draw(&self, window: &NotifyWindow) -> bool {
        // Sometimes users might want to render empty blocks to maintain padding and stuff, so we
        // optionally allow it.
        // TODO: there should be some way to cache this and not do it every draw operation.
        // We can't just do it for the whole notification because the notification can be replaced.
        let mut should_draw = true;
        let n = &window.notification;
        for c in &self.render_criteria {
            match c {
                RenderCriteria::Summary => if n.summary.is_empty() { should_draw = false },
                RenderCriteria::Body => if n.body.is_empty() { should_draw = false },
                RenderCriteria::AppImage => if n.app_image.is_none() { should_draw = false },
                RenderCriteria::HintImage => if n.hint_image.is_none() { should_draw = false },
                RenderCriteria::AppName => if n.app_name.is_empty() { should_draw = false },
                RenderCriteria::ActionDefault => if n.get_default_action().is_none() { should_draw = false },
                RenderCriteria::ActionOther(i) => if n.get_other_action(*i).is_none() { should_draw = false },
            }
        }

        should_draw
    }

    pub fn find_anchor_pos(hook: &Hook, offset: &Vec2, parent_rect: &Rect, self_rect: &Rect) -> Vec2 {
        // Get position of anchor in each rectangle (parent and self).
        let mut anchor = hook.parent_anchor.get_pos(parent_rect);
        let self_anchor = hook.self_anchor.get_pos(self_rect);

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
    pub fn draw_tree(&mut self, window: &NotifyWindow, parent_rect: &Rect, accum_rect: Rect) -> Rect {
        let rect = if self.should_draw(window) {
            self.params.draw(&self.hook, &self.offset, parent_rect, window)
        } else {
            // If block shouldn't be rendered, then we should be safe to just return an
            // empty rect.
            // We still need to set the position correctly, because other layout elements may be
            // depending on its position (e.g. in the center), even if it may not be being rendered.
            let pos = LayoutBlock::find_anchor_pos(&self.hook, &self.offset, parent_rect, &Rect::EMPTY);
            Rect::new(pos.x, pos.y, 0.0, 0.0)
        };
        let mut acc_rect = accum_rect.union(&rect);

        // Draw debug rect around bounding box.
        if Config::get().debug {
            let c = &Config::get().debug_color;
            window.context.set_source_rgba(c.r, c.g, c.b, c.a);
            window.context.set_line_width(1.0);
            window.context.rectangle(rect.x(), rect.y(), rect.width(), rect.height());
            window.context.stroke();
        }

        for child in &mut self.children {
            acc_rect = child.draw_tree(window, &rect, acc_rect);
        }

        self.cache_rect = rect.clone();
        acc_rect
    }

    // Predict the size of an entire layout, and initialize elements.
    pub fn predict_rect_tree_and_init(&mut self, window: &NotifyWindow, parent_rect: &Rect, accum_rect: Rect) -> Rect {
        // Predict size is supposed to be relatively cheap and lets us predict the size of elements,
        // so we can set window size and other stuff ahead of time.  We also initialize some stuff in
        // here to save performance.
        // `predict_rect_and_init` finds the bounding box of an individual element -- children are not
        // involved.
        let rect = if self.should_draw(window) {
            self.params.predict_rect_and_init(&self.hook, &self.offset, parent_rect, window)
        } else {
            let pos = LayoutBlock::find_anchor_pos(&self.hook, &self.offset, parent_rect, &Rect::EMPTY);
            Rect::new(pos.x, pos.y, 0.0, 0.0)
        };
        let mut acc_rect = accum_rect.union(&rect);

        // Recursively get child rects.
        for child in &mut self.children {
            acc_rect = child.predict_rect_tree_and_init(window, &rect, acc_rect);
        }

        acc_rect
    }

    // Call update on each block in tree.
    pub fn update_tree(&mut self, delta_time: Duration, window: &NotifyWindow) -> bool {
        let mut dirty = self.params.update(delta_time, window);
        for elem in &mut self.children {
            dirty |= elem.update_tree(delta_time, window);
        }

        dirty
    }

    pub fn check_and_send_click(&mut self, position: &Vec2, window: &NotifyWindow) -> bool {
        let mut dirty = false;
        if self.cache_rect.contains_point(position) {
            dirty |= self.params.clicked(window);
        }

        for child in &mut self.children {
            dirty |= child.check_and_send_click(position, window);
        }

        dirty
    }

    pub fn check_and_send_hover(&mut self, position: &Vec2, window: &NotifyWindow) -> bool {
        let mut dirty = false;
        // If we aren't hovered already, and we enter the rect, then send event.
        // If we are hovered already, and we leave the rect, then send event.
        if !self.hovered && self.cache_rect.contains_point(position) {
            self.hovered = true;
            dirty |= self.params.hovered(true, window);
        } else if self.hovered && !self.cache_rect.contains_point(position) {
            self.hovered = false;
            dirty |= self.params.hovered(false, window);
        }

        for child in &mut self.children {
            dirty |= child.check_and_send_hover(position, window);
        }

        dirty
    }
}

pub trait DrawableLayoutElement {
    fn draw(&self, hook: &Hook, offset: &Vec2, parent_rect: &Rect, window: &NotifyWindow) -> Rect;
    fn predict_rect_and_init(&mut self, hook: &Hook, offset: &Vec2, parent_rect: &Rect, window: &NotifyWindow) -> Rect;
    fn update(&mut self, _delta_time: Duration, _window: &NotifyWindow) -> bool { false }
    fn clicked(&mut self, _window: &NotifyWindow) -> bool { false }
    fn hovered(&mut self, _entered: bool, _window: &NotifyWindow) -> bool { false }
}

