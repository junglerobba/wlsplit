use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub anchor: String,
    pub margin: (i32, i32, i32, i32),
    pub width: usize,
    pub text_size: usize,
    pub padding_h: usize,
    pub padding_v: usize,
    pub background_color: [u8; 3],
    pub background_opacity: u8,
    pub font_color: [u8; 4],
    pub font_color_gain: [u8; 4],
    pub font_color_loss: [u8; 4],
    pub font_color_gold: [u8; 4],
    pub font_family: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            anchor: String::from("top-left"),
            margin: (12, 12, 12, 12),
            width: 400,
            text_size: 20,
            padding_h: 5,
            padding_v: 5,
            background_color: [0, 0, 0],
            background_opacity: 128,
            font_color: [255, 255, 255, 255],
            font_color_gain: [255, 0, 255, 0],
            font_color_loss: [255, 255, 0, 0],
            font_color_gold: [255, 255, 255, 0],
            font_family: None,
        }
    }
}
