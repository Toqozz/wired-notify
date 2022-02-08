use serde::Deserialize;

use crate::config::{Color, Config, Padding};
use crate::maths_utility;
use crate::maths_utility::{Rect, Vec2};
use crate::rendering::{
    layout::{DrawableLayoutElement, Hook, LayoutBlock},
    window::NotifyWindow,
};

#[derive(Debug, Deserialize, Clone)]
pub struct ProgressBlockParameters {
    pub padding: Padding,
    pub border_width: f64,
    pub border_rounding: f64,
    pub fill_rounding: f64,
    pub border_color: Color,
    pub background_color: Color,
    pub fill_color: Color,
    pub width: f64,
    pub height: f64,

    // -- Optional fields
    pub border_color_hovered: Option<Color>,
    pub background_color_hovered: Option<Color>,
    pub fill_color_hovered: Option<Color>,

    // -- Runtime fields
    #[serde(skip)]
    percentage: f64,
    #[serde(skip)]
    hover: bool,
}

impl ProgressBlockParameters {
    fn border_color(&self) -> &Color {
        if self.hover && self.border_color_hovered.is_some() {
            self.border_color_hovered.as_ref().unwrap()
        } else {
            &self.border_color
        }
    }

    fn background_color(&self) -> &Color {
        if self.hover && self.background_color_hovered.is_some() {
            self.background_color_hovered.as_ref().unwrap()
        } else {
            &self.background_color
        }
    }

    fn fill_color(&self) -> &Color {
        if self.hover && self.background_color_hovered.is_some() {
            self.fill_color_hovered.as_ref().unwrap()
        } else {
            &self.fill_color
        }
    }
}

// Much of this is the same as TextBlock, see there for documentation.
impl DrawableLayoutElement for ProgressBlockParameters {
    fn draw(&self, hook: &Hook, offset: &Vec2, parent_rect: &Rect, window: &NotifyWindow) -> Result<Rect, cairo::Error> {
        let border_col = self.border_color();
        let background_col = self.background_color();
        let fill_col = self.fill_color();

        let width = if self.width < 0.0 { parent_rect.width() } else { self.width + self.padding.width() };
        let height = if self.height < 0.0 { parent_rect.height() } else { self.height + self.padding.height() };
        let mut rect = Rect::new(
            0.0, 0.0,
            width, height,
        );
        let pos = LayoutBlock::find_anchor_pos(hook, offset, parent_rect, &rect);

        // Progress background.
        maths_utility::cairo_rounded_bordered_filled_rectangle(
            &window.context,
            pos.x + self.padding.left, pos.y + self.padding.top,   // x, y
            width - self.padding.width(), height - self.padding.height(),
            self.percentage,
            self.border_rounding,
            self.fill_rounding,
            self.border_width,
            border_col,
            background_col,
            fill_col,
        )?;

        window.context.set_operator(cairo::Operator::Over);
        // Debug, unpadded drawing, to help users.
        if Config::get().debug {
            maths_utility::debug_rect(
                &window.context,
                true,
                pos.x + self.padding.left,
                pos.y + self.padding.top,
                rect.width() - self.padding.width(),
                rect.height() - self.padding.height(),
            )?;
        }

        rect.set_xy(pos.x, pos.y);
        Ok(rect)
    }

    fn predict_rect_and_init(&mut self, hook: &Hook, offset: &Vec2, parent_rect: &Rect, window: &NotifyWindow) -> Rect {
        if self.padding.width() > parent_rect.width() || self.padding.height() > parent_rect.height() {
            eprintln!("Warning: padding width/height exceeds parent rect width/height.");
        }

        let width = if self.width < 0.0 { parent_rect.width() } else { self.width + self.padding.width() };
        let height = if self.height < 0.0 { parent_rect.height() } else { self.height + self.padding.height() };
        let mut rect = Rect::new(
            0.0, 0.0,
            width, height,
        );

        self.percentage = window.notification.percentage.unwrap_or(0.0) as f64;
        let pos = LayoutBlock::find_anchor_pos(hook, offset, parent_rect, &rect);
        rect.set_xy(pos.x, pos.y);
        rect
    }

    fn hovered(&mut self, entered: bool, _window: &NotifyWindow) -> bool {
        self.hover = entered;
        true
    }
}
