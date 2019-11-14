use serde::Deserialize;

use crate::types::maths::{Vec2, Rect};
use crate::rendering::layout::LayoutBlock;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub max_notifications: u32,
    pub width: u32,
    pub height: u32,            // Base height.  NOTE: notification windows will generally be resized, ignoring this value.

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



    pub debug: bool,
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


