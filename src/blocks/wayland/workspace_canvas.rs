use gtk::prelude::*;
use cairo::Context;
use gtk::{Application, ApplicationWindow, DrawingArea};
use std::sync::Arc;

#[derive(Debug, Clone, Copy)]
pub enum BlockColor {
    Highlight,
    Normal,
    Transparent,
}

pub struct ColorPalette {
    highlight_color: (f64, f64, f64),
    normal_color: (f64, f64, f64),
}

impl ColorPalette {
    pub fn new(highlight_color: (f64, f64, f64), normal_color: (f64, f64, f64)) -> Self {
        Self {
            highlight_color,
            normal_color,
        }
    }

    pub fn get_color(&self, block_color: BlockColor) -> Option<(f64, f64, f64)> {
        match block_color {
            BlockColor::Highlight => Some(self.highlight_color),
            BlockColor::Normal => Some(self.normal_color),
            BlockColor::Transparent => None,
        }
    }
}

pub struct Canvas {
    colors: Vec<BlockColor>,
    palette: Arc<ColorPalette>,
}

impl Canvas {
    pub fn new(colors: Vec<BlockColor>, palette: Arc<ColorPalette>) -> Self {
        Self { colors, palette }
    }

    pub fn draw(&self, cr: &Context) {
        let width = cr.clip_extents().unwrap().1 - cr.clip_extents().unwrap().0;
        let block_width = width / self.colors.len() as f64;

        for (i, color) in self.colors.iter().enumerate() {
            let x = i as f64 * block_width;
            if let Some((r, g, b)) = self.palette.get_color(*color) {
                cr.set_source_rgb(r, g, b);
                cr.rectangle(x, 0.0, block_width, cr.clip_extents().3 - cr.clip_extents().2);
                cr.fill().unwrap();
            }
        }
    }
}
