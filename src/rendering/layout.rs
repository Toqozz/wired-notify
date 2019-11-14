use serde::Deserialize;

use crate::types::maths::{Vec2, Rect};
use crate::config::{Padding, Color, AnchorPosition};
use crate::rendering::window::NotifyWindow;
use image::{FilterType, GenericImageView};
use cairo::ImageSurface;

#[derive(Debug, Deserialize)]
pub enum LayoutBlock {
    NotificationBlock(NotificationBlockParameters),
    TextBlock(TextBlockParameters),
    ImageBlock(ImageBlockParameters),
}

#[derive(Debug, Deserialize)]
pub struct NotificationBlockParameters {
    pub monitor: i32,
    pub monitor_hook: AnchorPosition,
    pub monitor_offset: Vec2,

    pub border_width: f64,
    pub background_color: Color,
    pub border_color: Color,

    pub gap: Vec2,
    pub notification_hook: AnchorPosition,
    pub children: Vec<LayoutBlock>,
}

#[derive(Debug, Deserialize)]
pub struct TextBlockParameters {
    pub hook: AnchorPosition,
    pub offset: Vec2,
    pub padding: Padding,
    pub text: String,
    pub font: String,
    pub color: Color,
    pub vertical_center: bool,
    pub max_width: i32,
    pub max_height: i32,
    //https://developer.gnome.org/pango/stable/pango-Markup.html
    pub children: Vec<LayoutBlock>,
}

#[derive(Debug, Deserialize)]
pub struct ImageBlockParameters {
    pub hook: AnchorPosition,
    // -1 to scale to size with aspect ratio kept?
    pub offset: Vec2,
    pub padding: Padding,
    pub width: i32,
    pub height: i32,
    pub children: Vec<LayoutBlock>,
}

pub struct DeltaRect {
    rect: Rect,
    delta: Vec2,
}

impl LayoutBlock {
    // TODO: cleanup.
    pub fn find_anchor_pos(&self, parent_rect: &Rect) -> Vec2 {
        let pos = match self {
            LayoutBlock::NotificationBlock(p) => {
                let mut pos = p.monitor_hook.get_pos(parent_rect);
                pos.x += p.monitor_offset.x;
                pos.y += p.monitor_offset.y;
                pos
            }
            LayoutBlock::TextBlock(p) => {
                let mut pos = p.hook.get_pos(parent_rect);
                pos.x += p.offset.x;
                pos.y += p.offset.x;
                pos
            },
            LayoutBlock::ImageBlock(p) => {
                let mut pos = p.hook.get_pos(parent_rect);
                pos.x += p.offset.x;
                pos.y += p.offset.y;
                pos
            },
        };

        pos
    }

    pub fn children(&self) -> &Vec<LayoutBlock> {
        match self {
            LayoutBlock::NotificationBlock(p) => &p.children,
            LayoutBlock::TextBlock(p) => &p.children,
            LayoutBlock::ImageBlock(p) => &p.children,
        }
    }

    // Predict size is relatively cheap and lets us predict the size of elements, so we can set window size ahead of time.
    // Predicts the size of an individual block.
    pub fn predict_rect_independent(&self, parent_rect: &Rect, window: &NotifyWindow) -> Rect {
        let size = match self {
            LayoutBlock::NotificationBlock(_) => {
                Rect::new(0.0, 0.0, 0.0, 0.0)
            },

            LayoutBlock::TextBlock(p) => {
                let mut text = p.text.clone();
                text = text.replace("%s", &window.notification.summary);
                text = text.replace("%b", &window.notification.body);

                let pos = self.find_anchor_pos(parent_rect);
                window.text.get_string_rect(
                    &text,
                    &pos,
                    &p.padding,
                    &p.font,
                    p.vertical_center,
                    p.max_width,
                    p.max_height,
                )
            },

            LayoutBlock::ImageBlock(p) => {
                if window.notification.image.is_some() {
                    let pos = self.find_anchor_pos(parent_rect);

                    Rect::new(
                        pos.x - p.padding.left,
                        pos.y - p.padding.top,
                        p.width as f64 + p.padding.left + p.padding.right,
                        p.height as f64 + p.padding.top + p.padding.bottom,
                    )
                } else {
                    Rect::new(0.0, 0.0, 0.0, 0.0)
                }
            },
        };

        size
    }

    pub fn draw_independent(&self, parent_rect: &Rect, window: &NotifyWindow) -> Rect {
        match self {
            LayoutBlock::NotificationBlock(p) => {
                //let rect = parent_rect.get_inner_rect();

                // Clear
                window.context.set_operator(cairo::Operator::Clear);
                window.context.paint();

                // Draw border + background.
                window.context.set_operator(cairo::Operator::Source);

                let bd_color = &p.border_color;
                window.context.set_source_rgba(bd_color.r, bd_color.g, bd_color.b, bd_color.a);
                window.context.paint();

                let bg_color = &p.background_color;
                let bw = &p.border_width;
                window.context.set_source_rgba(bg_color.r, bg_color.g, bg_color.b, bg_color.a);
                window.context.rectangle(
                    *bw, *bw,     // x, y
                    parent_rect.width() - bw * 2.0, parent_rect.height() - bw * 2.0,
                );
                window.context.fill();

                // Base notification background doesn't actually take up space, so use same rect.
                parent_rect.clone()
                //Rect::new(0.0, 0.0, 0.0, 0.0)
            },

            LayoutBlock::TextBlock(p) => {
                // TODO: Some/None for summary/body?  We don't want to replace or even add the block if there is no body.
                let mut text = p.text.clone();
                text = text
                    .replace("%s", &window.notification.summary)
                    .replace("%b", &window.notification.body);

                let pos = self.find_anchor_pos(parent_rect);

                //let text_color = &p.color;
                //window.context.set_source_rgba(text_color.r, text_color.g, text_color.b, text_color.a);
                window.text.paint_string(
                    &window.context,
                    &text,
                    &pos,
                    &p.padding,
                    &p.font,
                    &p.color,
                    p.vertical_center,
                    p.max_width,
                    p.max_height,
                )
            }

            LayoutBlock::ImageBlock(p) => {
                if let Some(image) = &window.notification.image {
                    let img = image.resize(p.width as u32, p.height as u32, FilterType::Nearest);
                    let format = cairo::Format::ARgb32;

                    //let (width, height) = img.dimensions();
                    let stride = cairo::Format::stride_for_width(format, p.width as u32).expect("Failed to calculate image stride.");
                    // Cairo reads pixels back-to-front, so ARgb32 is actually BgrA32.
                    let pixels = img.to_bgra().into_raw();
                    let image_sfc = ImageSurface::create_for_data(pixels, format, p.width as i32, p.height as i32, stride)
                        .expect("Failed to create image surface.");

                    let pos = self.find_anchor_pos(parent_rect);

                    window.context.set_source_surface(&image_sfc, pos.x, pos.y);
                    window.context.rectangle(pos.x, pos.y, p.width as f64, p.height as f64);
                    window.context.fill();

                    let rect = Rect::new(
                        pos.x - p.padding.left, pos.y - p.padding.top,
                        p.width as f64 + p.padding.left + p.padding.right,
                        p.height as f64 + p.padding.top + p.padding.bottom);

                    rect
                } else {
                    Rect::new(0.0, 0.0, 0.0, 0.0)
                }
            }
        }
    }

    // Run a function on each element in the layout, optionally passing in the function's return value.
    pub fn traverse<T, F: Copy>(&self, func: F, pass: &T)
        where F: Fn(&Self, &T) -> T {

        let result = func(self, pass);

        for elem in self.children() {
            //let result = func(elem, pass);
            elem.traverse(func, &result);
        }
    }

    // Predict the size of an entire layout.
    pub fn predict_rect_tree(&self, window: &NotifyWindow, parent_rect: &Rect, accum_rect: &Rect) -> Rect {
        // `predict_rect_independent` finds the bounding box of an individual layout -- children are not involved.
        let rect = self.predict_rect_independent(parent_rect, window);
        let mut acc_rect = accum_rect.union(&rect);

        // Recursively get child rects.
        for child in self.children() {
            let child_rect = &child.predict_rect_tree(window, &rect, &acc_rect);
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
