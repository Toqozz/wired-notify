use serde::Deserialize;
use sdl2::pixels;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub max_notifications: u32,
    pub gap: i32,
    pub notification: NotificationConfig,
    pub shortcuts: ShortcutsConfig,
}

#[derive(Debug, Deserialize)]
pub struct NotificationConfig {
    // Geometry.
    pub width: u32,
    pub height: u32,            // Base height.  NOTE: notification windows will generally be resized, ignoring this value.
    pub x: i32,
    pub y: i32,

    pub top_margin: i32,        // Margin between window edge (top) and summary text.
    pub left_margin: i32,       // Margin between window edge (left) and text.
    pub right_margin: i32,      // Margin between window edge (right) -- this effectively defines the cutoff for the line.  TOOD: not currently the case -- body_width is used instead?
    pub bottom_margin: i32,     // Margin between window edge (bottom) and the bottom of the body text.


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
