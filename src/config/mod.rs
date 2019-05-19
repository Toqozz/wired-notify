use serde::Deserialize;
use sdl2::pixels;

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
    pub summary_max_lines: u32,

    pub body_width: u32,
    pub body_max_lines: u32,

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

    pub top_margin: u32,
    pub left_margin: u32,
    pub right_margin: u32,
    pub bottom_margin: u32,

    pub summary_body_gap: i32,

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

#[derive(Debug, Deserialize, Clone)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl From<Color> for pixels::Color {
    fn from(c: Color) -> pixels::Color {
        pixels::Color::RGBA(c.r, c.g, c.b, c.a)
    }
}
