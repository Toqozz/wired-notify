use pango::{self, prelude::*, FontDescription, Layout};

use serde::Deserialize;

use crate::{
    config::{Color, Padding},
    maths_utility::{Rect, Vec2},
};

#[derive(Debug, Deserialize, Clone)]
pub enum EllipsizeMode {
    NoEllipsize,
    Start,
    Middle,
    End,
}
impl Default for EllipsizeMode {
    fn default() -> Self {
        EllipsizeMode::Middle
    }
}

impl EllipsizeMode {
    pub fn to_pango_mode(&self) -> pango::EllipsizeMode {
        match self {
            EllipsizeMode::NoEllipsize => pango::EllipsizeMode::None,
            EllipsizeMode::Start => pango::EllipsizeMode::Start,
            EllipsizeMode::Middle => pango::EllipsizeMode::Middle,
            EllipsizeMode::End => pango::EllipsizeMode::End,
        }
    }
}

#[derive(Debug)]
pub struct TextRenderer {
    //config: &'a Config,
    pctx: pango::Context,
    layout: pango::Layout,
}

impl TextRenderer {
    pub fn new(ctx: &cairo::Context) -> Self {
        let pctx =
            pangocairo::functions::create_context(ctx).expect("Failed to create pango context.");

        let layout = Layout::new(&pctx);

        Self { pctx, layout }
    }

    // Sets the current text of the renderer, applying markup and ellipsizing according to
    // ellipsize mode and `max_width` / `max_height`.
    pub fn set_text(
        &self,
        text: &str,
        font: &str,
        max_width: i32,
        max_height: i32,
        ellipsize: &EllipsizeMode,
    ) {
        let font_dsc = FontDescription::from_string(font);
        self.pctx.set_font_description(&font_dsc);

        // Applying scale when `max_width`/`max_height` is < 0 seems to work, but let's not take
        // chances.
        let width = if max_width < 0 {
            -1
        } else {
            pango::SCALE * max_width
        };
        let height = if max_height < 0 {
            -1
        } else {
            pango::SCALE * max_height
        };

        self.layout.set_ellipsize(ellipsize.to_pango_mode());
        self.layout.set_markup(text);
        self.layout.set_height(height);
        self.layout.set_width(width);
    }

    // Gets a raw, unpadded rect which surrounds the text.
    pub fn _get_rect(&self) -> Rect {
        let (width, height) = self.layout.get_pixel_size();
        Rect::new(0.0, 0.0, width as f64, height as f64)
    }

    // Gets the rect which surrounds the text, with a minimum width and height applied.
    pub fn get_sized_rect(&self, min_width: i32, min_height: i32) -> Rect {
        let (mut width, mut height) = self.layout.get_pixel_size();
        if width < min_width {
            width = min_width;
        }
        if height < min_height {
            height = min_height;
        }

        Rect::new(0.0, 0.0, width as f64, height as f64)
    }

    // Gets the sized rect, and then applies padding as well.
    pub fn get_sized_padded_rect(
        &self,
        padding: &Padding,
        min_width: i32,
        min_height: i32,
    ) -> Rect {
        let mut rect = self.get_sized_rect(min_width, min_height);

        rect.set_width(rect.width() as f64 + padding.width());
        rect.set_height(rect.height() as f64 + padding.height());
        rect
    }

    // Paints current text at the specified position in the specified color.
    pub fn paint(&self, ctx: &cairo::Context, pos: &Vec2, color: &Color) {
        // Move cursor to draw position and draw text.
        ctx.set_source_rgba(color.r, color.g, color.b, color.a);
        ctx.move_to(pos.x, pos.y);
        pangocairo::functions::show_layout(ctx, &self.layout);
    }

    // Paints current text at the specified position, offsetting for the provided padding.
    pub fn paint_padded(&self, ctx: &cairo::Context, pos: &Vec2, color: &Color, padding: &Padding) {
        // Text rendered within padded rects need to be moved to the padded position before
        // drawing.
        let pos = Vec2::new(pos.x + padding.left, pos.y + padding.top);
        self.paint(ctx, &pos, color);
    }
}
