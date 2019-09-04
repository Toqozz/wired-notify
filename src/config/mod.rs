use serde::Deserialize;

use crate::types::maths::{Vec2, Rect};

#[derive(Debug, Deserialize)]
pub struct Config {
    pub max_notifications: u32,
    pub gap: i32,
    pub notification: NotificationConfig,
    pub shortcuts: ShortcutsConfig,
}

#[derive(Debug, Deserialize)]
pub struct NotificationConfig {
    pub root: LayoutBlock,

    pub font: String,

    //pub layout: Tree<LayoutElement>,

    //pub summary: TextArea,
    //pub body: TextArea,

    // Geometry.
    pub width: u32,
    pub height: u32,            // Base height.  NOTE: notification windows will generally be resized, ignoring this value.
    pub x: i32,
    pub y: i32,

    pub border_width: f64,

    pub background_color: Color,
    pub border_color: Color,

    pub timeout: f32,           // Default timeout.

    pub scroll_speed: f32,
    pub bounce: bool,

    // Undecided...
    //bounce_margin: u32,
    //rounding: u32,
}

#[derive(Debug, Deserialize)]
pub struct ShortcutsConfig {
    pub notification_close: u32,
    pub notification_closeall: u32,
    pub notification_pause: u32,
    pub notification_url: u32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Padding {
    pub left: f64,
    pub right: f64,
    pub top: f64,
    pub bottom: f64,
}

#[derive(Debug, Deserialize)]
pub struct Offset {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Deserialize)]
pub enum FieldType {
    Root,
    Icon,
    Summary,
    Body,
}

#[derive(Debug, Deserialize, Clone)]
pub enum AnchorPosition {
    TL,
    TR,
    BL,
    BR,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Color {
    pub r: f64,
    pub g: f64,
    pub b: f64,
    pub a: f64,
}

#[derive(Debug, Deserialize)]
pub struct TextParameters {
    pub offset: Vec2,
    pub padding: Padding,
    pub color: Color,
    pub max_width: i32,
    pub max_height: i32,
}

#[derive(Debug, Deserialize)]
pub struct LayoutBlock {
    pub field: FieldType,
    pub hook: AnchorPosition,
    pub parameters: TextParameters,
    pub children: Vec<LayoutBlock>,
}

impl LayoutBlock {
    pub fn find_anchor_pos(&self, parent_rect: &Rect) -> Vec2 {
        let mut pos = match self.hook {
            AnchorPosition::TL => { parent_rect.top_left() },
            AnchorPosition::TR => { parent_rect.top_right() },
            AnchorPosition::BL => { parent_rect.bottom_left() },
            AnchorPosition::BR => { parent_rect.bottom_right() },
        };

        pos.x += self.parameters.offset.x;
        pos.y += self.parameters.offset.y;

        pos
    }

    pub fn flatten(&self) -> Vec<&LayoutBlock> {
        fn traverse(block: &LayoutBlock) -> Vec<&LayoutBlock> {
            let mut flat_blocks = vec![];
            flat_blocks.push(block);
            for elem in &block.children {
                flat_blocks.extend(traverse(elem));
            }

            flat_blocks
        }

        let thing = traverse(self);
        thing
    }

    // Run a function on each element in the layout, optionally passing in the function's return value.
    pub fn traverse<T, F: Copy>(&self, func: F, pass: Option<&T>)
        where F: Fn(&Self, Option<&T>) -> T {
        for elem in &self.children {
            let result = func(elem, pass);
            elem.traverse(func, Some(&result));
        }
    }

    // Run a function on each child in layout (recursively), accumulating the return value of the function using an accumulator.
    pub fn traverse_accum<T: Clone, F: Copy, N: Copy>(&self, func: F, accumulator: N, initial: &T, pass: &T) -> T
        where F: Fn(&LayoutBlock, &T) -> T,
              N: Fn(&T, &T) -> T {
        let mut accum: T = initial.clone();
        for elem in &self.children {
            let result = func(elem, pass);
            accum = accumulator(&result, &accum);
            accum = accumulator(&elem.traverse_accum(func, accumulator, initial, &result), &accum);
        }

        accum
    }
}
