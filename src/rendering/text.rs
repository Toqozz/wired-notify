use pango::{
    self,
    prelude::*,
    Layout,
    FontDescription,
};

use serde::Deserialize;

use crate::{
    maths_utility::{Rect, Vec2, MinMax},
    config::{Padding, Color},
    bus::dbus::Notification,
};


#[derive(Debug)]
pub struct TextRenderer {
    //config: &'a Config,
    pctx: pango::Context,
    layout: pango::Layout,
}

impl TextRenderer {
    pub fn new(ctx: &cairo::Context) -> Self {
        let pctx = pangocairo::functions::create_context(ctx)
            .expect("Failed to create pango context.");

        let layout = Layout::new(&pctx);
        // TODO: this should be a config option.
        layout.set_ellipsize(pango::EllipsizeMode::Middle);

        Self {
            pctx,
            layout,
        }
    }

    pub fn set_text(&self, text: &str, font: &str, max_width: i32, max_height: i32) {
        let font_dsc = FontDescription::from_string(font);
        self.pctx.set_font_description(&font_dsc);

        self.layout.set_markup(text);
        //self.layout.set_text(text);
        self.layout.set_height(pango::SCALE * max_height);
        self.layout.set_width(pango::SCALE * max_width);
    }

    pub fn _get_rect(&self) -> Rect {
        let (width, height) = self.layout.get_pixel_size();
        Rect::new(0.0, 0.0, width as f64, height as f64)
    }

    pub fn get_sized_rect(&self, padding: &Padding, min_width: i32, min_height: i32) -> Rect {
        let (mut width, mut height) = self.layout.get_pixel_size();
        if width < min_width {
            width = min_width;
        }
        if height < min_height {
            height = min_height;
        }

        Rect::new(0.0, 0.0, width as f64 + padding.width(), height as f64 + padding.height())
    }

    pub fn paint(&self, ctx: &cairo::Context, pos: &Vec2, color: &Color) {
        // Move cursor to draw position and draw text.
        ctx.set_source_rgba(color.r, color.g, color.b, color.a);
        ctx.move_to(pos.x, pos.y);
        pangocairo::functions::show_layout(ctx, &self.layout);
    }
}
