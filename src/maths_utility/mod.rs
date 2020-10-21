#![allow(dead_code)]

use serde::Deserialize;

#[derive(Default, Debug, Deserialize, Clone)]
pub struct MinMax {
    pub min: i32,
    pub max: i32,
}

#[derive(Default, Debug, Clone, Deserialize)]
pub struct Vec2 {
    pub x: f64,
    pub y: f64,
}

impl Vec2 {
    pub fn new(x: f64, y: f64) -> Self {
        Vec2 { x, y }
    }
}

#[derive(Debug, Clone)]
pub struct Rect {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

impl Default for Rect {
    fn default() -> Self {
        Self {
            x: 0.0, y: 0.0, width: 0.0, height: 0.0,
        }
    }
}

impl Rect {
    pub const EMPTY: Self = Self { x: 0.0, y: 0.0, width: 0.0, height: 0.0 };

    pub fn empty() -> Self {
        Self {
            x: 0.0, y: 0.0, width: 0.0, height: 0.0,
        }
    }

    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x, y, width, height,
        }
    }

    pub fn x(&self) -> f64 {
        self.x
    }

    pub fn y(&self) -> f64 {
        self.y
    }

    pub fn width(&self) -> f64 {
        self.width
    }

    pub fn height(&self) -> f64 {
        self.height
    }

    pub fn size(&self) -> (f64, f64) {
        (self.width(), self.height())
    }

    pub fn set_x(&mut self, x: f64) {
        self.x = x;
    }

    pub fn set_y(&mut self, y: f64) {
        self.y = y;
    }

    pub fn set_xy(&mut self, x: f64, y: f64) {
        self.x = x;
        self.y = y;
    }

    pub fn set_width(&mut self, width: f64) {
        self.width = width;
    }

    pub fn set_height(&mut self, height: f64) {
        self.height = height;
    }

    pub fn left(&self) -> f64 {
        self.x
    }

    pub fn right(&self) -> f64 {
        self.x + self.width
    }

    pub fn top(&self) -> f64 {
        self.y
    }

    pub fn bottom(&self) -> f64 {
        self.y + self.height
    }

    pub fn top_left(&self) -> Vec2 {
        Vec2 { x: self.left(), y: self.top() }
    }

    pub fn top_right(&self) -> Vec2 {
        Vec2 { x: self.right(), y: self.top() }
    }

    pub fn bottom_left(&self) -> Vec2 {
        Vec2 { x: self.left(), y: self.bottom() }
    }

    pub fn bottom_right(&self) -> Vec2 {
        Vec2 { x: self.right(), y: self.bottom() }
    }

    pub fn mid_left(&self) -> Vec2 { Vec2 { x: self.left(), y: (self.bottom() + self.top()) / 2.0 } }

    pub fn mid_right(&self) -> Vec2 { Vec2 { x: self.right(), y: (self.bottom() + self.top()) / 2.0 } }

    pub fn mid_top(&self) -> Vec2 { Vec2 { x: (self.left() + self.right()) / 2.0, y: self.top() } }

    pub fn mid_bottom(&self) -> Vec2 { Vec2 { x: (self.left() + self.right()) / 2.0, y: self.bottom() } }

    pub fn set_right(&mut self, right: f64) {
        self.x = right - self.width
    }

    pub fn set_bottom(&mut self, bottom: f64) {
        self.y = bottom - self.height
    }

    pub fn union_new(&self, other: &Rect) -> Rect {
        let x = f64::min(self.x(), other.x());
        let y = f64::min(self.y(), other.y());
        let r = f64::max(self.right(), other.right());
        let b = f64::max(self.bottom(), other.bottom());

        Rect::new(x, y, r - x, b - y)
    }

    pub fn union(mut self, other: &Rect) -> Rect {
        let x = f64::min(self.x(), other.x());
        let y = f64::min(self.y(), other.y());
        let r = f64::max(self.right(), other.right());
        let b = f64::max(self.bottom(), other.bottom());

        self.set_xy(x, y);
        self.set_width(r - x);
        self.set_height(b - y);
        self
    }
}


// Non-clamped lerp.
pub fn lerp(a: f64, b: f64, t: f64) -> f64 {
    return (1.0 - t) * a + t * b;
}

pub fn clamp(mut val: f64, min: f64, max: f64) -> f64 {
    if val < min { val = min }
    if val > max { val = max }
    val
}

pub fn distance(x: f64, y: f64) -> f64 {
    if x > y {
        (x - y).abs()
    } else {
        (y - x).abs()
    }
}

// http://cairographics.org/samples/rounded_rectangle/
pub fn cairo_rounded_rectangle(ctx: &cairo::Context, x: f64, y: f64, width: f64, height: f64, corner_radius: f64) {
    ctx.save();

    // Aspect ratio.
    let aspect = 1.0;
    let radius = corner_radius / aspect;

    let degrees = std::f64::consts::PI / 180.0;

    ctx.new_sub_path();
    ctx.arc(x + width - radius, y + radius         , radius         , -90.0 * degrees, 0.0 * degrees);
    ctx.arc(x + width - radius, y + height - radius, radius         , 0.0 * degrees  , 90.0 * degrees);
    ctx.arc(x + radius        , y + height - radius, radius         , 90.0 * degrees , 180.0 * degrees);
    ctx.arc(x + radius        , y + radius         , radius         , 180.0 * degrees, 270.0 * degrees);
    ctx.close_path();

    ctx.restore();
}

pub fn debug_rect(ctx: &cairo::Context, alt: bool, x: f64, y: f64, width: f64, height: f64) {
    use crate::config::Config;
    // Often, modules will check for debug before calling this anyway to save work, but it's good
    // to be sure we never draw any debug rects when debug is turned off.
    if !Config::get().debug {
        return;
    }

    ctx.save();

    let c = if alt {
        &Config::get().debug_color_alt
    } else {
        &Config::get().debug_color
    };
    ctx.set_source_rgba(c.r, c.g, c.b, c.a);
    ctx.set_line_width(1.0);
    ctx.rectangle(x, y, width, height);
    ctx.stroke();

    ctx.restore();
}

pub fn escape_decode(to_escape: &str) -> String {
    // Escape ampersand and decode some html stuff manually, for fun.
    // can escape about 6 ampersands without allocating (each is 4 chars, minus the existing char).
    let mut escaped: Vec<u8> = Vec::with_capacity(to_escape.len() + 18);
    let bytes = to_escape.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let byte = bytes[i];
        match byte {
            b'&' => {
                // TODO: not really happy with this, should clean it up.
                if i + 5 <= to_escape.len() {
                    // The end of the slice range is exclusive, so we need to go one higher.
                    match &to_escape[i..i+5] {
                        // If we're trying to write "&amp;" then we should allow it.
                        "&amp;" => { escaped.push(byte); i += 1; continue },
                        "&#39;" => { escaped.push(b'\''); i += 5; continue },
                        "&#34;" => { escaped.push(b'"'); i += 5; continue },
                        _ => (),
                    }
                }

                if i + 6 <= to_escape.len() {
                    match &to_escape[i..i+6] {
                        "&apos;" => { escaped.push(b'\''); i += 6; continue },
                        "&quot;" => { escaped.push(b'\"'); i += 6; continue },
                        _ => (),
                    }
                }

                escaped.extend_from_slice(b"&amp;");
            }

            _ => escaped.push(byte),
        }

        i += 1;
    }

    // We should be safe to use `from_utf8_unchecked` here, but let's be safe.
    String::from_utf8(escaped).expect("Error when escaping ampersand.")
}

// str.replace() won't work for this because we'd have to do it twice: once for the summary and
// once for the body.  The first insertion could insert format strings which would mess up the
// second insertion.
// This solution is pretty fast (microseconds in release).
pub fn format_notification_string(format_string: &str, summary: &str, body: &str) -> String {
    let mut formatted: Vec<u8> = vec![];
    let bytes = format_string.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let byte = bytes[i];
        // We need at least 2 chars to match a format string, so if we only have one, then let's
        // leave.
        if i == bytes.len()-1 {
            formatted.push(byte);
            i += 1;
            continue;
        }

        match byte {
            b'%' => {
                // This range is exclusive on the right hand side, so we go +2.
                match &format_string[i..i+2] {
                    "%s" => { formatted.extend_from_slice(summary.as_bytes()); i += 2; continue },
                    "%b" => { formatted.extend_from_slice(body.as_bytes()); i += 2; continue },
                    _ => (),
                }

                formatted.push(b'%');
            }

            _ => formatted.push(byte),
        }

        i += 1;
    }

    // We should be safe to use `from_utf8_unchecked` here, but let's be safe.
    String::from_utf8(formatted).expect("Error when formatting notification string.")
}
