use anyhow::Ok;
use image::{DynamicImage, GenericImageView, Rgba};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CharacterSet {
    Standard,
    Detailed,
    Blocks,
    Minimal,
    Binary
}

impl CharacterSet {
    pub fn chars(&self) -> &'static str {
        match self {
            CharacterSet::Standard => "@%#*+=-:. ",
            CharacterSet::Detailed => "$@B%8&WM#*oahkbdpqwmZO0QLCJUYXzcvunxrjft/\\|()1{}[]?-_+~<>i!lI;:,\"^`'. ",
            CharacterSet::Blocks => "█▓▒░ ",
            CharacterSet::Minimal => "@#=-. ",
            CharacterSet::Binary => "@ ",
        }
    }
    
    pub fn name(&self) -> &'static str {
        match self {
            CharacterSet::Standard => "Standard",
            CharacterSet::Detailed => "Detailed",
            CharacterSet::Blocks => "Blocks",
            CharacterSet::Minimal => "Minimal",
            CharacterSet::Binary => "Binary"
        }
    }
    
    pub fn all() -> &'static [CharacterSet] {
        &[
            CharacterSet::Standard,
            CharacterSet::Detailed,
            CharacterSet::Blocks,
            CharacterSet::Minimal,
            CharacterSet::Binary
        ]
    }
    
    pub fn next(&self) -> CharacterSet {
        let all = Self::all();
        let idx = all.iter().position(|c| c == self).unwrap_or(0);
        
        all[(idx + 1) % all.len()]
    }
    
    pub fn prev(&self) -> CharacterSet {
        let all = Self::all();
        let idx = all.iter().position(|c| c == self).unwrap_or(0);
        
        all[(idx + all.len() - 1) % all.len()]
    }
}

#[derive(Debug, Clone)]
pub struct AsciiConfig {
    pub width: u32,
    pub char_set: CharacterSet,
    pub invert: bool,
    pub colored: bool,
    pub ascept_ratio_correction: f32 
}

impl Default for AsciiConfig {
    fn default() -> Self {
        Self { 
            width: 80, 
            char_set: CharacterSet::Standard, 
            invert: false, 
            colored: false, 
            ascept_ratio_correction: 0.5 
        }
    }
}

#[derive(Debug, Clone)]
pub struct AsciiChar {
    pub character: char,
    pub color: Option<(u8, u8, u8)>
}

#[derive(Debug, Clone)]
pub struct AsciiArt {
    pub chars: Vec<Vec<AsciiChar>>,
    pub width: usize,
    pub height: usize,
}

impl AsciiArt {
    pub fn to_string_plain(&self) -> String {
        self
            .chars
            .iter()
            .map(|row| row.iter().map(|c| c.character).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }
    
    pub fn to_string_colored(&self) -> String {
        let mut result = String::new();
        for row in &self.chars {
            for ascii_char in row {
                if let Some((r, g, b)) = ascii_char.color {
                    result.push_str(&format!(
                        "\x1b[38;2;{};{};{}m{}\x1b[0m", r, g, b, ascii_char.character
                    ));
                } else {
                    result.push(ascii_char.character);
                }
            }
            result.push('\n');
        }
        
        result
    }
}

pub struct AsciiGenerator {
    config: AsciiConfig
}

impl AsciiGenerator {
    pub fn new(config: AsciiConfig) -> Self {
        Self { config }
    }
    
    pub fn with_default() -> Self {
        Self::new(AsciiConfig::default())
    }
    
    pub fn config(&self) -> &AsciiConfig {
        &self.config
    }
    
    pub fn config_mut(&mut self) -> &mut AsciiConfig {
        &mut self.config 
    }
    
    fn brightness_to_char(&self, brightness: f32) -> char {
        let chars = self
            .config
            .char_set
            .chars()
            .chars()
            .collect::<Vec<_>>();
        
        let brightness = if self.config.invert {
            1.0 - brightness
        } else {
            brightness
        };
        
        let index = ((1.0 - brightness) * (chars.len() - 1) as f32).round() as usize;
        let index = index.min(chars.len() - 1);
        chars[index]
    }
    
    fn pixel_brightness(pixel: Rgba<u8>) -> f32 {
        let [r, g, b, a] = pixel.0;
        
        let alpha = a as f32 / 255.0;
        let r = (r as f32 * alpha + 255.0 * (1.0 - alpha)) / 255.0;
        let g = (g as f32 * alpha + 255.0 * (1.0 - alpha)) / 255.0;
        let b = (b as f32 * alpha + 255.0 * (1.0 - alpha)) / 255.0;
        
        0.2126 * r + 0.7152 * g + 0.0722 * b
    }
    
    pub fn generate(&self, image: &DynamicImage) -> AsciiArt {
        let (orig_width, orig_height) = image.dimensions();
        
        let new_width = self.config.width;
        let scale = new_width as f32 / orig_width as f32;
        let new_height = (orig_height as f32 * scale + self.config.ascept_ratio_correction) as u32;
        let new_height = new_height.max(1);
        
        let resized = image.resize_exact(new_width, new_height, image::imageops::FilterType::Lanczos3);
        
        let rgba = resized.to_rgba8();
        let mut chars = Vec::with_capacity(new_height as usize);
        
        for y in 0..new_height {
            let mut row = Vec::with_capacity(new_width as usize);
            for x in 0..new_width {
                let pixel = rgba.get_pixel(x, y);
                let brightness = Self::pixel_brightness(*pixel);
                let character = self.brightness_to_char(brightness);
                
                let color = if self.config.colored {
                    let [r, g, b, _] = pixel.0;
                    Some((r, g, b))
                } else {
                    None
                };
                
                row.push(AsciiChar { character, color });
            }
            
            chars.push(row);
        }
        
        AsciiArt { 
            chars, 
            width: new_width as usize, 
            height: new_height as usize 
        }
    }
    
    pub fn generate_from_file(&self, path: &std::path::Path) -> anyhow::Result<AsciiArt> {
        let image = image::open(path)?;
        
        Ok(self.generate(&image))
    }
}

pub struct EdgeDetector;

impl EdgeDetector {
    pub fn generate_edges(image: &DynamicImage, config: &AsciiConfig) -> AsciiArt {
        let gray = image.to_luma8();
        let (width, height) = gray.dimensions();
        
        let scale = config.width as f32 / width as f32;
        let new_height = (height as f32 * scale * config.ascept_ratio_correction) as u32;
        let new_height = new_height.max(1);
        
        let resized = image::imageops::resize(&gray, config.width, new_height, image::imageops::FilterType::Lanczos3);
        
        let (w, h) = (config.width as i32, new_height as i32);
        let mut chars = Vec::new();
        
        let sobel_x = [[-1, 0, 1], [-2, 0, 2], [-1, 0, 1]];
        let sobel_y = [[-1, -2, -1], [0, 0, 0], [1, 2, 1]];
        
        let edge_chars = if config.invert {
            " .:-=+*#%@"
        } else {
            "@%#*+=-:. "
        };
        
        let edge_chars = edge_chars.chars().collect::<Vec<_>>();
        
        for y in 0..h {
            let mut row = Vec::new();
            for x in 0..w {
                let mut gx = 0i32;
                let mut gy = 0i32;
                
                for ky in 0..3 {
                    for kx in 0..3 {
                        let px = (x + ks as i32 - 1).clamp(0, w - 1) as u32;
                        let py = (y + ky as i32 - 1).clamp(0, h - 1) as u32;
                        let pixel = resized.get_pixel(px, py).0[0] as i32;
                        gx += pixel * sobel_x[ky][kx];
                        gy += pixel * sobel_y[ky][kx];
                    }
                }
                
                let magnitude = ((gx * gx + gy * gy) as f32).sqrt();
                let normalized = (magnitude / 1442.0).min(1.0);
                
                let idx = (normalized * (edge_chars.len() - 1) as f32) as usize;
                let character = edge_chars[idx.min(edge_chars.len() - 1)];
                
                row.push(AsciiChar {
                    character,
                    color: None,
                });
            }
            
            chars.push(row);
        }
        
        AsciiArt { 
            chars, 
            width: config.width as usize, 
            height: new_height as usize, 
        }
    }
}