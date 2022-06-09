use std::time::Duration;

use serde::Deserialize;

use crate::{
    rendering::blocks::*,
    maths_utility::{Vec2, Rect},
    config::{Config, AnchorPosition},
    rendering::window::NotifyWindow, bus::dbus::Notification,
};

use wired_derive::DrawableLayoutElement;

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
    //#[serde(default)]
    //#[deserialize_with(parse_criteria)]
    #[serde(default)]
    pub render_criteria: Vec<RenderCriteria>,
    #[serde(default)]
    pub render_anti_criteria: Vec<RenderCriteria>,
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
    AppName(String),
    Progress,
    ActionDefault,
    ActionOther(usize),

    And(Vec<RenderCriteria>),
    Or(Vec<RenderCriteria>),
}

enum Logic {
    And,
    Or
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
// Really no drawing at all should be done in `predict_rect_and_init`, unless you're using a bad API
// and it doesn't give prediction methods of its own.
#[allow(clippy::enum_variant_names)]
#[derive(Debug, Clone, Deserialize, DrawableLayoutElement)]
pub enum LayoutElement {
    NotificationBlock(NotificationBlockParameters),
    TextBlock(TextBlockParameters),
    ScrollingTextBlock(ScrollingTextBlockParameters),
    ImageBlock(ImageBlockParameters),
    ButtonBlock(ButtonBlockParameters),
    ProgressBlock(ProgressBlockParameters),
}

impl LayoutBlock {
    pub fn should_draw(&self, notification: &Notification) -> bool {
        fn criteria_matches(criteria: &RenderCriteria, notification: &Notification) -> bool {
            match criteria {
                RenderCriteria::Summary =>          !notification.summary.is_empty(),
                RenderCriteria::Body =>             !notification.body.is_empty(),
                RenderCriteria::AppImage =>         !notification.app_image.is_none(),
                RenderCriteria::HintImage =>        !notification.hint_image.is_none(),
                RenderCriteria::AppName(name) =>     notification.app_name.eq(name),
                RenderCriteria::Progress =>         !notification.percentage.is_none(),
                RenderCriteria::ActionDefault =>    !notification.get_default_action().is_none(),
                RenderCriteria::ActionOther(i) =>   !notification.get_other_action(*i).is_none(),

                RenderCriteria::And(criterion) =>   logic_matches(Logic::And, criterion, notification),
                RenderCriteria::Or(criterion) =>    logic_matches(Logic::Or, criterion, notification),
            }
        }

        fn logic_matches(logic: Logic, criterion: &Vec<RenderCriteria>, notification: &Notification) -> bool {
            let mut result;
            match logic {
                Logic::And => {
                    // ANDs start as true to coalesce properly.
                    result = true;
                    for c in criterion {
                        result &= criteria_matches(c, notification);
                    }
                },
                Logic::Or => {
                    // ORs start as false to coalesce properly.
                    result = false;
                    for c in criterion {
                        result |= criteria_matches(c, notification);
                    }
                }
            }

            result
        }

        // Sometimes users might want to render empty blocks to maintain padding and stuff, so we
        // optionally allow it (in the case that both render_criterias are empty).

        // We assume that we want empty render criteria to draw, but as soon as a criteria is
        // present, something must match.
        let mut render_criteria_matches;
        if self.render_criteria.is_empty() {
            render_criteria_matches = true;
        } else {
            render_criteria_matches = false;
            for criteria in &self.render_criteria {
                render_criteria_matches |= criteria_matches(criteria, notification);
            }
        }

        let mut render_anti_criteria_matches = false;
        for criteria in &self.render_anti_criteria {
            render_anti_criteria_matches |= criteria_matches(criteria, notification);
        }

        // For `render_criteria`, we *do* want to draw if any of the criteria match.
        // For `render_anti_criteria`, we *don't* want to draw if any of the criteria match.
        // `render_anti_criteria` takes priority.
        render_criteria_matches && !render_anti_criteria_matches
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
        // This is so dirty but OK.  Eventually it would be better to just build a buffer of things
        // to draw instead of recursing here.
        // If this is a root node, (the first one) we want to surround all following drawing
        // operations in a push group.
        // Using a push group is important because otherwise the X server might update the screen
        // in-between one of our draws, which would be bad: https://www.cairographics.org/Xlib/
        // (Animations and Full Screen section)
        if self.parent.is_empty() {
            window.context.push_group();
        }

        // This block is just to separate things conceptually and hopefully make it easier to see
        // the distinction between the root push group and everything else...  Ideally this won't
        // be kept around because we're going to move away from something recursive eventually...
        // right?
        let (rect, acc_rect) = {
            let rect = if self.should_draw(&window.notification) {
                let rect = self.params.draw(&self.hook, &self.offset, parent_rect, window)
                    .expect("Invalid cairo surface state.");
                rect
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
                window.context.stroke().expect("Invalid cairo surface state.");
            }

            for child in &mut self.children {
                acc_rect = child.draw_tree(window, &rect, acc_rect);
            }

            (rect, acc_rect)
        };

        // The push group from earlier gets popped and all the drawing is done at once.
        if self.parent.is_empty() {
            window.context.pop_group_to_source().expect("Failed to pop group to source.");
            window.context.set_operator(cairo::Operator::Source);
            window.context.paint().expect("Invalid cairo surface state.");
            window.context.set_operator(cairo::Operator::Over);
        }

        self.cache_rect = rect;
        acc_rect
    }

    // Predict the size of an entire layout, and initialize elements.
    pub fn predict_rect_tree_and_init(&mut self, window: &NotifyWindow, parent_rect: &Rect, accum_rect: Rect) -> Rect {
        // Predict size is supposed to be relatively cheap and lets us predict the size of elements,
        // so we can set window size and other stuff ahead of time.  We also initialize some stuff in
        // here to save performance.
        // `predict_rect_and_init` finds the bounding box of an individual element -- children are not
        // involved.
        let rect = if self.should_draw(&window.notification) {
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

    pub fn as_notification_block(&self) -> &NotificationBlockParameters {
        if let LayoutElement::NotificationBlock(p) = &self.params {
            return p;
        } else {
            panic!("Tried to cast a LayoutBlock as type NotificationBlock when it was something else.");
        }
    }
}

pub trait DrawableLayoutElement {
    fn draw(&self, hook: &Hook, offset: &Vec2, parent_rect: &Rect, window: &NotifyWindow) -> Result<Rect, cairo::Error>;
    fn predict_rect_and_init(&mut self, hook: &Hook, offset: &Vec2, parent_rect: &Rect, window: &NotifyWindow) -> Rect;
    fn update(&mut self, _delta_time: Duration, _window: &NotifyWindow) -> bool { false }
    fn clicked(&mut self, _window: &NotifyWindow) -> bool { false }
    fn hovered(&mut self, _entered: bool, _window: &NotifyWindow) -> bool { false }
}

