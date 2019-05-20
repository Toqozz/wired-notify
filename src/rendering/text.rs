use sdl2::{
    render::WindowCanvas,
    pixels::Color,
    rect::Rect,
    ttf::{
        Font,
        FontStyle,
    }
};

use crate::config::Config;

pub struct TextRenderer<'a> {
    config: &'a Config,
    font: Font<'a, 'a>,
}

impl<'a> TextRenderer<'a> {
    pub fn new(config: &'a Config, font: Font<'a, 'a>) -> Self {
        Self {
            config,
            font,
        }
    }

    // Render some lines of text to the canvas.
    pub fn render_text(&mut self, canvas: &mut WindowCanvas, texts: &mut Vec<(String, Rect)>, style: FontStyle) {
        while let Some((line, rect)) = texts.pop() {
            let texture_creator = canvas.texture_creator();

            self.font.set_style(style);
            let sfc = self.font.render(line.as_str())
                .blended(Color::RGBA(255, 255, 255, 255)).unwrap();

            let tex = texture_creator.create_texture_from_surface(&sfc).unwrap();

            canvas.copy(&tex, None, Some(rect)).unwrap();
        }
    }

    // Prepare text for rendering (break into lines and their associated rectangles.)
    pub fn prepare_text(&mut self, text: &str, style: FontStyle) -> Vec<(String, Rect)> {
        self.font.set_style(style);

        let lines = self.break_text_into_lines(text, self.config.notification.body_width, self.config.notification.body_max_lines as usize);
        let mut text_rects = Vec::new();
        for line in lines {
            let (width, height) = self.font.size_of(line.as_str()).unwrap();

            let rect = Rect::new(0, 0, width, height);

            text_rects.push((line, rect));
        }

        text_rects
    }

    // Break text into lines, based a max width value.
    fn break_text_into_lines(&self, text: &str, max_width: u32, max_lines: usize) -> Vec<String> {
        let mut lines = Vec::new();

        let mut last_whitespace = 0;
        let mut last_cut = 0;
        for (i, ch) in text.char_indices() {
            if ch.is_whitespace() {
                last_whitespace = i;
            }

            // String from start of current line to the current char.
            let string = &text[last_cut..i+1];
            // Retrieve size of current line (make sure font style is correct before doing this).
            let (width, _height) = self.font.size_of(&string).unwrap();
            if width > max_width {
                // If this should be the last line we print, ellipsize and set our cut to the end
                //   of the string.
                if lines.len() == max_lines-1 {
                    lines.push(text[last_cut..last_whitespace].to_owned() + "…");
                    last_cut = text.len();
                    break;
                // Otherwise, push the line and continue on the next.
                } else {
                    lines.push(text[last_cut..last_whitespace].to_owned());
                    last_cut = last_whitespace+1;
                }
            }
        }

        // If there are still any characters left over.
        if last_cut < text.len() {
            // Push the final characters, which didn't cause a new line but still need to be displayed.
            let end_slice = text[last_cut..text.len()].to_owned();
            lines.push(end_slice);
        }

        lines
    }
}
