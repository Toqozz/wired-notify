use pango::{self, FontDescription, Layout};

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

#[derive(Debug, Deserialize, Clone)]
pub enum AlignMode {
    Left,
    Center,
    Right,
}
impl Default for AlignMode {
    fn default() -> Self {
        AlignMode::Left
    }
}

impl AlignMode {
    pub fn to_pango_mode(&self) -> pango::Alignment {
        match self {
            AlignMode::Left => pango::Alignment::Left,
            AlignMode::Center => pango::Alignment::Center,
            AlignMode::Right => pango::Alignment::Right,
        }
    }
}

// Whether/how glyphs are smoothed.  Defaults to `System` (`Xft.antialias` and `Xft.rgba` on X11)
// because a user who has deliberately set up subpixel antialiasing should keep it.
#[derive(Debug, Deserialize, Clone)]
pub enum Antialias {
    System,
    None,
    Gray,
    Subpixel,
}
impl Default for Antialias {
    fn default() -> Self {
        Antialias::System
    }
}

impl Antialias {
    fn to_cairo(&self) -> cairo::Antialias {
        match self {
            Antialias::System => cairo::Antialias::Default,
            Antialias::None => cairo::Antialias::None,
            Antialias::Gray => cairo::Antialias::Gray,
            Antialias::Subpixel => cairo::Antialias::Subpixel,
        }
    }
}

// How much glyph outlines are distorted to fit the pixel grid.  `Full` produces crisper stems, but
// pango positions glyphs using unhinted advances, so distorting the outlines leaves uneven gaps
// between letters.  That's worst in fonts that don't ship hinting instructions of their own, where
// the autohinter takes over.
#[derive(Debug, Deserialize, Clone)]
pub enum HintStyle {
    System,
    None,
    Slight,
    Medium,
    Full,
}
impl Default for HintStyle {
    fn default() -> Self {
        HintStyle::Slight
    }
}

impl HintStyle {
    fn to_cairo(&self) -> cairo::HintStyle {
        match self {
            HintStyle::System => cairo::HintStyle::Default,
            HintStyle::None => cairo::HintStyle::None,
            HintStyle::Slight => cairo::HintStyle::Slight,
            HintStyle::Medium => cairo::HintStyle::Medium,
            HintStyle::Full => cairo::HintStyle::Full,
        }
    }
}

// Whether glyph advances are rounded to whole pixels.  `Off` lets pango place glyphs at fractional
// positions, which looks smoother in isolation but uneven next to hinted glyphs.
#[derive(Debug, Deserialize, Clone)]
pub enum HintMetrics {
    System,
    Off,
    On,
}
impl Default for HintMetrics {
    fn default() -> Self {
        HintMetrics::On
    }
}

impl HintMetrics {
    fn to_cairo(&self) -> cairo::HintMetrics {
        match self {
            HintMetrics::System => cairo::HintMetrics::Default,
            HintMetrics::Off => cairo::HintMetrics::Off,
            HintMetrics::On => cairo::HintMetrics::On,
        }
    }
}

// Subpixel geometry of the display; only used when antialiasing is `Subpixel`.
#[derive(Debug, Deserialize, Clone)]
pub enum SubpixelOrder {
    System,
    Rgb,
    Bgr,
    Vrgb,
    Vbgr,
}
impl Default for SubpixelOrder {
    fn default() -> Self {
        SubpixelOrder::System
    }
}

impl SubpixelOrder {
    fn to_cairo(&self) -> cairo::SubpixelOrder {
        match self {
            SubpixelOrder::System => cairo::SubpixelOrder::Default,
            SubpixelOrder::Rgb => cairo::SubpixelOrder::Rgb,
            SubpixelOrder::Bgr => cairo::SubpixelOrder::Bgr,
            SubpixelOrder::Vrgb => cairo::SubpixelOrder::Vrgb,
            SubpixelOrder::Vbgr => cairo::SubpixelOrder::Vbgr,
        }
    }
}

// Font rendering options.  Each field is merged over the display server's setting on its own, so
// anything left as `System` keeps the system value.
//
// We pin `hint_style` rather than inheriting it because cairo reads these from the X resource
// database rather than fontconfig, and falls back to full hinting when the resources aren't set --
// which is common under XWayland and on WMs that don't run `xrdb`.
#[derive(Debug, Deserialize, Clone, Default)]
pub struct FontOptions {
    #[serde(default)]
    pub antialias: Antialias,
    #[serde(default)]
    pub hint_style: HintStyle,
    #[serde(default)]
    pub hint_metrics: HintMetrics,
    #[serde(default)]
    pub subpixel_order: SubpixelOrder,
}

impl FontOptions {
    fn to_cairo(&self) -> cairo::FontOptions {
        let mut options = cairo::FontOptions::new().expect("Failed to create cairo font options.");
        options.set_antialias(self.antialias.to_cairo());
        options.set_hint_style(self.hint_style.to_cairo());
        options.set_hint_metrics(self.hint_metrics.to_cairo());
        options.set_subpixel_order(self.subpixel_order.to_cairo());
        options
    }
}

#[derive(Debug)]
pub struct TextRenderer {
    //config: &'a Config,
    pctx: pango::Context,
    layout: pango::Layout,
}

impl TextRenderer {
    pub fn new(ctx: &cairo::Context, font_options: &FontOptions) -> Self {
        let pctx = pangocairo::functions::create_context(ctx).expect("Failed to create pango context.");
        // Baked in for the life of the renderer.  Windows are created per notification, so a config
        // reload takes effect on the next notification.
        pangocairo::functions::context_set_font_options(&pctx, Some(&font_options.to_cairo()));

        let layout = Layout::new(&pctx);
        layout.set_wrap(pango::WrapMode::WordChar);

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
        alignment: &AlignMode,
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
        self.layout.set_alignment(alignment.to_pango_mode());
        self.layout.set_markup(text);
        self.layout.set_height(height);
        self.layout.set_width(width);
    }

    // Gets a raw, unpadded rect which surrounds the text.
    pub fn _get_rect(&self) -> Rect {
        let (width, height) = self.layout.pixel_size();
        Rect::new(0.0, 0.0, width as f64, height as f64)
    }

    // Gets the rect which surrounds the text, with a minimum width and height applied.
    pub fn get_sized_rect(&self, min_width: i32, min_height: i32) -> Rect {
        let (mut width, mut height) = self.layout.pixel_size();
        if width < min_width {
            width = min_width;
        }
        if height < min_height {
            height = min_height;
        }

        Rect::new(0.0, 0.0, width as f64, height as f64)
    }

    // Gets the sized rect, and then applies padding as well.
    pub fn get_sized_padded_rect(&self, padding: &Padding, min_width: i32, min_height: i32) -> Rect {
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
