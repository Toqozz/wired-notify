#![allow(dead_code)]

use serde::Deserialize;
use crate::config::Padding;

#[derive(Debug, Clone)]
pub struct PaddedRect {
    pub inner_rect: Rect,
    pub outer_rect: Rect,
}

impl PaddedRect {
    pub fn new(inner_rect: Rect, padding: &Padding) -> Self {
        let outer_rect = Rect::new(
            inner_rect.left() - padding.left,
            inner_rect.top() - padding.top,
            inner_rect.width() + padding.width(),
            inner_rect.height() + padding.height(),
        );

        Self {
            inner_rect,
            outer_rect,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Vec2 {
    pub x: f64,
    pub y: f64,
}

impl Vec2 {
    pub fn new(x: f64, y: f64) -> Self {
        Vec2 { x, y }
    }
}

impl Default for Vec2 {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Rect {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

impl Rect {
    pub const EMPTY: Self = Self { x: 0.0, y: 0.0, width: 0.0, height: 0.0 };

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

impl Default for Rect {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,
        }
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
