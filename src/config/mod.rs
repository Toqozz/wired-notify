use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub max_notifications: u32,
    pub gap: u32,
    pub notification: NotificationConfig,
    pub shortcuts: ShortcutsConfig,
}

#[derive(Debug, Deserialize)]
pub struct NotificationConfig {
    // Geometry.
    pub width: u32,
    pub height: u32,
    pub x: u32,
    pub y: u32,

    pub summary_width: u32,
    pub summary_startx: u32,
    pub summary_starty: f32,
    pub body_width: u32,
    pub border_width: u32,

    pub summary_color: Color,
    pub body_color: Color,
    pub background_color: Color,
    pub border_color: Color,

    pub timeout: f32,

    pub font: String,
    pub scroll_speed: f32,
    pub bounce: bool,
    //bounce_margin: u32,

    pub left_margin: u32,
    pub middle_margin: u32,
    pub right_margin: u32,

    // markup?

    //rounding: u32,
}

#[derive(Debug, Deserialize)]
pub struct ShortcutsConfig {
    pub notification_close: u32,
    pub notification_closeall: u32,
    pub notification_pause: u32,
    pub notification_url: u32,
}

#[derive(Debug, Deserialize)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}
