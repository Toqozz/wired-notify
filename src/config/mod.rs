use serde::Deserialize;

use crate::types::maths::{Vec2, Rect};

#[derive(Debug, Deserialize)]
pub struct Config {
    pub max_notifications: u32,
    pub width: u32,
    pub height: u32,            // Base height.  NOTE: notification windows will generally be resized, ignoring this value.

    pub border_width: f64,
    pub background_color: Color,
    pub border_color: Color,

    pub timeout: i32,           // Default timeout.
    pub poll_interval: u64,

    pub font: String,

    pub scroll_speed: f32,
    pub bounce: bool,

    pub layout: LayoutBlock,

    //pub notification: NotificationConfig,
    pub shortcuts: ShortcutsConfig,

    // Runtime useful things related to configuration.
    #[serde(skip)]
    pub monitor: Option<winit::monitor::MonitorHandle>,
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
    pub font: String,
    pub offset: Vec2,
    pub padding: Padding,
    pub color: Color,
    pub max_width: i32,
    pub max_height: i32,
    //https://developer.gnome.org/pango/stable/pango-Markup.html
}

#[derive(Debug, Deserialize)]
pub struct NotificationBlockParameters {
    pub monitor: i32,
    pub monitor_hook: AnchorPosition,
    pub monitor_offset: Vec2,
    pub gap: Vec2,
    pub notification_hook: AnchorPosition,
    pub children: Vec<LayoutBlock>,
}

#[derive(Debug, Deserialize)]
pub struct TextBlockParameters {
    pub text: String,
    pub parameters: TextParameters,
    pub hook: AnchorPosition,
    pub children: Vec<LayoutBlock>,
}

#[derive(Debug, Deserialize)]
pub struct ImageBlockParameters {
    pub hook: AnchorPosition,
    pub children: Vec<LayoutBlock>,
}

#[derive(Debug, Deserialize)]
pub enum LayoutBlock {
    NotificationBlock(NotificationBlockParameters),
    TextBlock(TextBlockParameters),
    ImageBlock(ImageBlockParameters),
}

impl AnchorPosition {
    pub fn get_pos(&self, rect: &Rect) -> Vec2 {
        match self {
            AnchorPosition::TL => rect.top_left(),
            AnchorPosition::TR => rect.top_right(),
            AnchorPosition::BL => rect.bottom_left(),
            AnchorPosition::BR => rect.bottom_right(),
        }
    }
}

impl LayoutBlock {
    // TODO: cleanup.
    pub fn find_anchor_pos(&self, parent_rect: &Rect) -> Vec2 {
        let pos = match self {
            LayoutBlock::NotificationBlock(p) => p.monitor_hook.get_pos(parent_rect),
            LayoutBlock::TextBlock(p) => {
                let mut pos = p.hook.get_pos(parent_rect);
                pos.x += p.parameters.offset.x;
                pos.y += p.parameters.offset.x;
                pos
            },
            LayoutBlock::ImageBlock(p) => p.hook.get_pos(parent_rect),
        };

        pos
    }

    // Run a function on each element in the layout, optionally passing in the function's return value.
    pub fn traverse<T, F: Copy>(&self, func: F, pass: &T)
        where F: Fn(&Self, &T) -> T {

        let children = match self {
            LayoutBlock::NotificationBlock(p) => &p.children,
            LayoutBlock::TextBlock(p) => &p.children,
            LayoutBlock::ImageBlock(p) => &p.children,
        };

        for elem in children {
            let result = func(elem, pass);
            elem.traverse(func, &result);
        }
    }

    // Run a function on each child in layout (recursively), accumulating the return value of the function using an accumulator.
    pub fn traverse_accum<T: Clone, F: Copy, N: Copy>(&self, func: F, accumulator: N, initial: &T, pass: &T) -> T
        where F: Fn(&Self, &T) -> T,
              N: Fn(&T, &T) -> T {

        let children = match self {
            LayoutBlock::NotificationBlock(p) => &p.children,
            LayoutBlock::TextBlock(p) => &p.children,
            LayoutBlock::ImageBlock(p) => &p.children,
        };

        let mut accum: T = initial.clone();
        for elem in children {
            let result = func(elem, pass);
            accum = accumulator(&result, &accum);
            accum = accumulator(&elem.traverse_accum(func, accumulator, initial, &result), &accum);
        }

        accum
    }
}
