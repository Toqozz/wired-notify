use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub max_notifications: u32,
    pub gap: i32,
    pub notification: NotificationConfig,
    pub shortcuts: ShortcutsConfig,
}

#[derive(Debug, Deserialize)]
pub struct NotificationConfig {
    pub summary: TextArea,
    pub body: TextArea,

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
pub struct TextArea {
    pub anchor: Anchor,
    pub anchor_position: AnchorPosition,

    pub font: String,

    pub color: Color,

    pub width: f64,
    pub max_lines: f64,

    pub left_margin: f64,
    pub right_margin: f64,
    pub top_margin: f64,
    pub bottom_margin: f64,
}

#[derive(Debug, Deserialize)]
pub struct ShortcutsConfig {
    pub notification_close: u32,
    pub notification_closeall: u32,
    pub notification_pause: u32,
    pub notification_url: u32,
}

#[derive(Debug, Deserialize)]
pub enum Anchor {
    Root,
    Summary,
    Body,
}

#[derive(Debug, Deserialize)]
pub enum AnchorPosition {
    Left,
    Right,
    Top,
    Bottom,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Color {
    pub r: f64,
    pub g: f64,
    pub b: f64,
    pub a: f64,
}
