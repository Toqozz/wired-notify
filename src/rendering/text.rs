use sdl2::render::WindowCanvas;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::Texture;
use sdl2::ttf::Font;
use sdl2::ttf::FontStyle;

use super::sdl::SDL2State;
use crate::config::Config;

pub struct TextRenderer<'a> {
    config: &'a Config,
    sdl: &'a SDL2State,
    font: Font<'a, 'a>,
}

impl<'a> TextRenderer<'a> {
    pub fn new(config: &'a Config, sdl: &'a SDL2State, font: Font<'a, 'a>) -> Self {
        Self {
            config,
            sdl,
            font,
        }
    }

    pub fn break_text_into_lines(&self, text: &str, max_width: u32) -> Vec<String> {
        let mut lines = Vec::new();

        let mut last_whitespace = 0;
        let mut last_cut = 0;
        for (i, ch) in text.char_indices() {
            if ch.is_whitespace() {
                last_whitespace = i;
            }

            let string = &text[last_cut..i+1];
            let (width, _height) = self.font.size_of(&string).unwrap();
            if width > max_width {
                lines.push(text[last_cut..last_whitespace].to_owned());
                last_cut = last_whitespace+1;
            }
        }

        let end_slice = text[last_cut..text.len()].to_owned();
        if !end_slice.is_empty() {
            lines.push(end_slice);
        }

        lines
    }

    pub fn render_text(&mut self, canvas: &mut WindowCanvas, texts: &mut Vec<(String, Rect)>, style: FontStyle) {
        while let Some((line, rect)) = texts.pop() {
            let texture_creator = canvas.texture_creator();
            self.font.set_style(style);
            let sfc = self.font.render(line.as_str())
                .blended(Color::RGBA(255, 255, 255, 255)).unwrap();

            let tex = texture_creator.create_texture_from_surface(&sfc).unwrap();

            //let sdl2::render::TextureQuery{ width, height, .. } = tex.query();
            //let new_rect = Rect::new(rect.x(), rect.y(), width, height);

            canvas.copy(&tex, None, Some(rect)).unwrap();
        }
    }

    // Should output Vec<(Texture, Rect)> that is ready to be rendered.
    pub fn prepare_text(&mut self, text: &str, style: FontStyle) -> Vec<(String, Rect)> {
        let lines = self.break_text_into_lines(text, self.config.notification.body_width);

        let mut text_rects = Vec::new();
        for line in lines {
            self.font.set_style(style);
            let (width, height) = self.font.size_of(line.as_str()).unwrap();

            let rect = Rect::new(0, 0, width, height);

            text_rects.push((line, rect));
        }

        text_rects
    }
}
