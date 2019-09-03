//use crate::config::Config;
use crate::config::Padding;
use crate::config::AnchorPosition;

use pango;
use pango::prelude::*;
use pango::Layout;
use pango::FontDescription;

use crate::types::maths::{ Rect, Point };


#[derive(Debug)]
pub struct TextRenderer {
    //config: &'a Config,
    font: FontDescription,
    pctx: pango::Context,
    layout: pango::Layout,
}

impl TextRenderer {
    //pub fn new(config: &'a Config, font_name: &str, cairo_context: &'a cairo::Context) -> Self {
    pub fn new(ctx: &cairo::Context, font_name: &str) -> Self {
        let pctx = pangocairo::functions::create_context(ctx)
            .expect("Failed to create pango context.");
        // @IMPORTANT: FontDescription must be freed at some point??????
        // @IMPORTANT: FontDescription must be freed at some point????
        // @IMPORTANT: FontDescription must be freed at some point????
        let font = FontDescription::from_string(font_name);

        // Find font from description -- Arial 10 Bold
        pctx.set_font_description(&font);

        let layout = Layout::new(&pctx);

        Self {
            font,
            pctx,
            layout,
        }
    }

    pub fn get_string_rect(&self, pos: &Point, padding: &Padding, text: &str) -> Rect {
        self.layout.set_text(text);

        let (width, height) = self.layout.get_pixel_size();
        Rect::new(
            pos.x, pos.y,
            width as f64 + (padding.left + padding.right),
            height as f64 + (padding.top + padding.bottom)
        )
    }

    pub fn paint_string(&self, ctx: &cairo::Context, pos: &Point, padding: &Padding, text: &str) -> Rect {
        self.layout.set_text(text);

        // Move cursor to draw position and draw text.
        ctx.move_to(pos.x + padding.left, pos.y + padding.top);
        pangocairo::functions::show_layout(ctx, &self.layout);

        let (width, height) = self.layout.get_pixel_size();
        Rect::new(
            pos.x, pos.y,
            width as f64 + (padding.left + padding.right),
            height as f64 + (padding.top + padding.bottom)
        )
    }
}

#[derive(Debug)]
pub struct TextDrawable {
    anchor: Option<Point>,
    text: String,
    padding: Padding,
    offset: Point,
    renderer: TextRenderer,

    // @NOTE: consider keeping:
    //rect: Rect,
    //dirty: bool,
    //   here to making lookup up the rect more efficient -- it probably shouldn't even change over
    //   the notification's entire runtime, but we should support changing it nonetheless.
}

/*
impl TextDrawable {
    pub fn new(ctx: &cairo::Context, text: String, padding: Padding, offset: Point) -> Self {
        let renderer = TextRenderer::new(ctx, "Arial 10");

        Self {
            anchor: None,
            text,
            padding,
            offset,
            renderer,
        }
    }

    pub fn get_anchor(&self, anchor_pos: &AnchorPosition) -> Point {
        let string_rect = self.get_rect();
        match anchor_pos {
            AnchorPosition::TL => string_rect.top_left(),
            AnchorPosition::TR => string_rect.top_right(),
            AnchorPosition::BL => string_rect.bottom_left(),
            AnchorPosition::BR => string_rect.bottom_right(),
        }
    }

    pub fn set_anchor(&mut self, anchor: &Point) {
        self.anchor = Some(anchor.clone());
    }

    pub fn get_rect(&self) -> Rect {
        let mut origin = Point { x: 0.0, y: 0.0 };
        if let Some(anchor) = &self.anchor {
            origin.x = anchor.x;
            origin.y = anchor.y;
        }

        origin.x += self.offset.x;
        origin.y += self.offset.y;

        self.renderer.get_string_rect(&origin, &self.padding, &self.text)
    }

    pub fn paint_to_ctx(&self, ctx: &cairo::Context) {
        let mut origin = Point { x: 0.0, y: 0.0 };
        if let Some(anchor) = &self.anchor {
            origin.x = anchor.x;
            origin.y = anchor.y;
        }

        origin.x += self.offset.x;
        origin.y += self.offset.y;

        //ctx.set_operator(cairo::Operator::Source);
        ctx.set_source_rgba(1.0, 1.0, 1.0, 1.0);

        self.renderer.paint_string(ctx, &origin, &self.padding, &self.text);
    }
}
*/
