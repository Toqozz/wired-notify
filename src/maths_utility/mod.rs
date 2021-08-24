#![allow(dead_code)]
use std::process::{Command, Stdio};

use serde::Deserialize;
use crate::config::Color;
use crate::bus::dbus::Notification;

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

    pub fn contains_point(&self, point: &Vec2) -> bool {
        (point.x >= self.x) && (point.x < (self.x + self.width())) &&
        (point.y >= self.y) && (point.y < (self.y + self.height()))
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
pub fn cairo_path_rounded_rectangle(ctx: &cairo::Context, x: f64, y: f64, width: f64, height: f64, corner_radius: f64) {
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

pub fn cairo_path_rounded_rectangle_inverse(ctx: &cairo::Context, x: f64, y: f64, width: f64, height: f64, corner_radius: f64) {
    ctx.save();

    // Aspect ratio.
    let aspect = 1.0;
    let radius = corner_radius / aspect;

    let degrees = std::f64::consts::PI / 180.0;

    ctx.new_sub_path();
    ctx.arc_negative(x + radius        , y + radius         , radius         , 270.0 * degrees, 180.0 * degrees);
    ctx.arc_negative(x + radius        , y + height - radius, radius         , 180.0 * degrees , 90.0 * degrees);
    ctx.arc_negative(x + width - radius, y + height - radius, radius         , 90.0 * degrees  , 0.0 * degrees);
    ctx.arc_negative(x + width - radius, y + radius         , radius         , 0.0 * degrees, -90.0 * degrees);
    ctx.close_path();

    ctx.restore();
}

// Creates a rounded rectangle with a border that acts as a user would expect.
// Obeys background opacity and such -- border color is not present on the background like it would
// be with the naive approach.
pub fn cairo_rounded_bordered_rectangle(ctx: &cairo::Context, x: f64, y: f64, width: f64, height: f64, corner_radius: f64, thickness: f64, fg_color: &Color, bg_color: &Color) {
    ctx.save();

    // To my understanding, push group basically lets us write to another texture, which we can
    // then paint on top of stuff later.
    ctx.push_group();
    ctx.set_operator(cairo::Operator::Source);
    cairo_path_rounded_rectangle(ctx, x, y, width, height, corner_radius);
    ctx.set_source_rgba(fg_color.r, fg_color.g, fg_color.b, fg_color.a);
    ctx.fill();

    cairo_path_rounded_rectangle(ctx, x + thickness, y + thickness, width - thickness * 2.0, height - thickness * 2.0, corner_radius);
    ctx.set_source_rgba(bg_color.r, bg_color.g, bg_color.b, bg_color.a);
    ctx.fill();
    ctx.pop_group_to_source();
    ctx.paint();

    ctx.restore();
}

// Creates a rounded rectangle with a border that acts as a user would expect.
// Obeys background opacity and such -- border color is not present on the background like it would
// be with the naive approach.
pub fn cairo_rounded_bordered_filled_rectangle(ctx: &cairo::Context, x: f64, y: f64, width: f64, height: f64, fill_percent: f64, border_corner_radius: f64, fill_corner_radius: f64, thickness: f64, fg_color: &Color, bg_color: &Color, fill_color: &Color) {
    ctx.save();

    // To my understanding, push group basically lets us write to another texture, which we can
    // then paint on top of stuff later.
    ctx.push_group();
    ctx.set_operator(cairo::Operator::Source);
    cairo_path_rounded_rectangle(ctx, x, y, width, height, border_corner_radius);
    ctx.set_source_rgba(fg_color.r, fg_color.g, fg_color.b, fg_color.a);
    ctx.fill();

    // Background clipping path (to prevent leaks at small fill %s).
    cairo_path_rounded_rectangle(ctx, x + thickness, y + thickness, width - thickness * 2.0, height - thickness * 2.0, fill_corner_radius);
    ctx.clip_preserve();

    // Draw background, which subtracts from the clipping area path.
    cairo_path_rounded_rectangle_inverse(ctx, x + thickness, y + thickness, (width - thickness * 2.0)*fill_percent, height - thickness * 2.0, fill_corner_radius);
    ctx.set_source_rgba(bg_color.r, bg_color.g, bg_color.b, bg_color.a);
    ctx.fill();

    // Draw fill area, on top of the background.
    cairo_path_rounded_rectangle(ctx, x + thickness, y + thickness, (width - thickness * 2.0)*fill_percent, height - thickness * 2.0, fill_corner_radius);
    ctx.set_source_rgba(fill_color.r, fill_color.g, fill_color.b, fill_color.a);
    ctx.fill();

    ctx.pop_group_to_source();
    ctx.paint();

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
    let before = std::time::Instant::now();

    // Escape ampersand and decode some html stuff manually, for fun.
    // can escape about 6 ampersands without allocating (each is 4 chars, minus the existing char).
    let mut escaped: Vec<u8> = Vec::with_capacity(to_escape.len() + 18);
    let bytes = to_escape.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let byte = bytes[i];
        match byte {
            b'<' => escaped.extend_from_slice(b"&lt;"),
            b'&' => {
                // TODO: not really happy with this, should clean it up.
                if i + 4 <= to_escape.len() {
                    match &to_escape[i..i+4] {
                        // If we're trying to write these, leave them be.
                        "&gt;" => { escaped.push(byte); i += 1; continue },
                        "&lt;" => { escaped.push(byte); i += 1; continue },
                        _ => (),
                    }
                }

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

    println!("String: {}, Elapsed time: {:.2?}", to_escape, before.elapsed());

    // We should be safe to use `from_utf8_unchecked` here, but let's be safe.
    String::from_utf8(escaped).expect("Invalid unicode after escape_decode.")
}

// str.replace() won't work for this because we'd have to do it twice: once for the summary and
// once for the body.  The first insertion could insert format strings which would mess up the
// second insertion.
// This solution is pretty fast (microseconds in release).
pub fn format_action_notification_string(format_string: &str, action_name: &str, notification: &Notification) -> String {
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
                    // We need room for at least 2 brackets, so check for that.
                    "%t" => if i+4 < format_string.len() {
                        let (time_format, len) =
                            extract_time_format(&format_string[i+2..]).unwrap_or(("", 0));

                        formatted.extend_from_slice(
                            notification.time.format(time_format).to_string().as_bytes()
                        );

                        i += 2 + len;
                        continue;
                    }
                    "%s" => { formatted.extend_from_slice(notification.summary.as_bytes()); i += 2; continue },
                    "%b" => { formatted.extend_from_slice(notification.body.as_bytes()); i += 2; continue },
                    "%n" => { formatted.extend_from_slice(notification.app_name.as_bytes()); i += 2; continue },
                    "%a" => { formatted.extend_from_slice(action_name.as_bytes()); i += 2; continue },
                    "%i" => { formatted.extend_from_slice(notification.id.to_string().as_bytes()); i += 2; continue },
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

pub fn format_notification_string(format_string: &str, notification: &Notification) -> String {
    format_action_notification_string(format_string, "", notification)
}

// This function expects a string that has an open bracket to start, and a closing bracket
// *somewhere*.  It will return the string between the open bracket and the first closing bracket.
fn extract_time_format(string: &str) -> Option<(&str, usize)> {
    // To extract a format string, we just need to grab whatever is between '(' and ')'.
    // This mostly just means sanity checking.

    // We should also consider checking `string.is_char_boundary(0)`, to make sure the string we're
    // provided is correct.
    if !string.starts_with("(") {
        println!("Warning: tried to parse a time format string, but it didn't start with '('.");
        return None;
    }

    if let Some(close_idx) = string.find(")") {
        // Step forward one to skip past the opening bracket.  We assume it's one byte...
        let time_format = &string[1..close_idx];
        return Some((time_format, time_format.len() + 2));
    } else {
        println!("Warning: tried to parse a time format string, but couldn't find a closing ')'.");
        return None;
    }
}

pub fn find_and_open_url(string: String) {
    // This would be cleaner with regex, but we want to avoid the dependency.
    // Find the first instance of either "http://" or "https://" and then split the
    // string at the end of the word.
    let idx = string.find("http://").or_else(|| string.find("https://"));
    let maybe_url = if let Some(i) = idx {
        let (_, end) = string.split_at(i);
        end.split_whitespace().next()
    } else {
        println!("Was requested to open a url but couldn't find one in the specified string");
        None
    };

    if let Some(url) = maybe_url {
        // `xdg-open` can be blocking, so opening like this can block our whole program because
        // we're grabbing the command's status at the end (which will cause it to wait).
        // I think it's important that we report at least some status back in case of error, so
        // we use `spawn()` instead.
        /*
        let status = Command::new("xdg-open").arg(url).status();
        if status.is_err() {
            eprintln!("Tried to open a url using xdg-open, but the command failed: {:?}", status);
        }
        */

        // For some reason, Ctrl-C closes child processes, even when they're detached
        // (`thread::spawn`), but `SIGINT`, `SIGTERM`, `SIGKILL`, and more (?) don't.
        // Maybe it's this: https://unix.stackexchange.com/questions/149741/why-is-sigint-not-propagated-to-child-process-when-sent-to-its-parent-process
        let child = Command::new("xdg-open")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .arg(url)
            .spawn();

        if child.is_err() {
            eprintln!("Tried to open a url using xdg-open, but the command failed: {:?}", child);
        }
    }
}

// For serde defaults.  So annoying that we need a function for this.
// Issue been open since 2018, so I guess it's never getting fixed.
pub fn val_true() -> bool {
    true
}
