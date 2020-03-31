use serde::Deserialize;

use crate::types::maths::{Vec2, Rect};
use crate::rendering::layout::{
    LayoutBlock, Hook,
    LayoutElement::{
        NotificationBlock,
        TextBlock,
        ScrollingTextBlock,
        ImageBlock,
    },
};

use crate::rendering::blocks::{
    notification_block::NotificationBlockParameters,
    text_block::TextBlockParameters,
    scrolling_text_block::ScrollingTextBlockParameters,
    image_block::ImageBlockParameters,
};

#[derive(Debug, Deserialize)]
pub struct Config {
    pub max_notifications: u32,
    pub width: u32,
    pub height: u32,            // Base height.  NOTE: notification windows will generally be resized, ignoring this value.

    // TODO: timeout should be in seconds.
    pub timeout: i32,           // Default timeout.
    pub poll_interval: u64,

    pub debug: bool,
    pub shortcuts: ShortcutsConfig,

    pub layout: LayoutBlock,

    // Runtime useful things related to configuration.
    #[serde(skip)]
    pub monitor: Option<winit::monitor::MonitorHandle>,
}

// TODO: think about adding default() impls for layout blocks?
// It might be as easy as a derive() for most cases.
// TODO: this shouldn't be in rust.  We should just include_str! from the directory -- facepalm.
impl Default for Config {
    fn default() -> Self {
        Self {
            max_notifications: 4,
            width: 1,
            height: 1,

            timeout: 10000,
            poll_interval: 33,

            monitor: None,

            debug: false,

            shortcuts: ShortcutsConfig {
                notification_close: 0,
                notification_closeall: 0,
                notification_pause: 0,
                notification_url: 0,
            },

            layout: LayoutBlock {
                hook: Hook { parent_hook: AnchorPosition::TL, self_hook: AnchorPosition::TL },
                offset: Vec2::new(7.0, 7.0),
                params: NotificationBlock(
                    NotificationBlockParameters {
                        monitor: 0,
                        border_width: 3.0,
                        background_color: Color::new(0.15686, 0.15686, 0.15686, 1.0),
                        border_color: Color::new(0.92157, 0.858824, 0.698039, 1.0),

                        gap: Vec2::new(0.0, 8.0),
                        notification_hook: AnchorPosition::BL,
                    }
                ),
                children: vec![
                    LayoutBlock {
                        hook: Hook { parent_hook: AnchorPosition::TL, self_hook: AnchorPosition::TL },
                        offset: Vec2::new(0.0, 0.0),
                        params: ImageBlock(
                            ImageBlockParameters {
                                padding: Padding::new(7.0, 4.0, 7.0, 4.0),
                                width: 64,
                                height: 64,
                            }
                        ),
                        children: vec![
                            LayoutBlock {         // summary block
                                hook: Hook { parent_hook: AnchorPosition::TR, self_hook: AnchorPosition::TL },
                                offset: Vec2::new(0.0, 0.0),
                                params: TextBlock(
                                    TextBlockParameters {
                                        text: "%s".to_owned(),
                                        font: "Ariel 9".to_owned(),
                                        color: Color::new(0.92157, 0.858824, 0.698039, 1.0),
                                        padding: Padding::new(7.0, 7.0, 7.0, 4.0),
                                        max_width: 300,
                                        max_height: 50,
                                    }
                                ),
                                children: vec![
                                    LayoutBlock {        // body block
                                        hook: Hook { parent_hook: AnchorPosition::BL, self_hook: AnchorPosition::TL },
                                        offset: Vec2::new(0.0, 0.0),
                                        params: ScrollingTextBlock(
                                            ScrollingTextBlockParameters {
                                                text: "%b".to_owned(),
                                                font: "Ariel 9".to_owned(),
                                                color: Color::new(0.92157, 0.858824, 0.698039, 1.0),
                                                padding: Padding::new(7.0, 7.0, 0.0, 7.0),
                                                max_width: 300,
                                                scroll_speed: 0.4,
                                                lhs_dist: 10.0,
                                                rhs_dist: 10.0,
                                                scroll_t: 1.0,

                                                clip_rect: Rect::new(0.0, 0.0, 0.0, 0.0),
                                                bounce_left: 0.0,
                                                bounce_right: 0.0,
                                                update_enabled: false,
                                            }
                                        ),
                                        children: vec![],
                                    },
                                ],
                            },
                        ],
                    },
                ],
            }

        }
    }
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

#[derive(Debug, Deserialize, Clone)]
pub struct Offset {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Deserialize, Clone)]
pub enum AnchorPosition {
    ML,
    TL,
    MT,
    TR,
    MR,
    BR,
    MB,
    BL,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Color {
    pub r: f64,
    pub g: f64,
    pub b: f64,
    pub a: f64,
}

impl Color {
    pub fn new(r: f64, g: f64, b: f64, a: f64) -> Self {
        Color { r, g, b, a }
    }
}

impl Padding {
    pub fn new(left: f64, right: f64, top: f64, bottom: f64) -> Self {
        Padding { left, right, top, bottom }
    }

    pub fn width(&self) -> f64 {
        self.left + self.right
    }
    pub fn height(&self) -> f64 {
        self.top + self.bottom
    }
}

impl AnchorPosition {
    pub fn get_pos(&self, rect: &Rect) -> Vec2 {
        match self {
            AnchorPosition::ML => rect.mid_left(),
            AnchorPosition::TL => rect.top_left(),
            AnchorPosition::MT => rect.mid_top(),
            AnchorPosition::TR => rect.top_right(),
            AnchorPosition::MR => rect.mid_right(),
            AnchorPosition::BR => rect.bottom_right(),
            AnchorPosition::MB => rect.mid_bottom(),
            AnchorPosition::BL => rect.bottom_left(),
        }
    }
}


