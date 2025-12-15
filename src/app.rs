use std::path::{Path, PathBuf};

use anyhow::Ok;
use image::{DynamicImage, GenericImageView};

use crate::ascii::{AsciiArt, AsciiConfig, AsciiGenerator, EdgeDetector};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Normal,
    FileBrowser,
    Help,
    Saving
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderMode {
    Normal,
    EdgeDetection,
}

impl RenderMode {
    pub fn toggle(&self) -> Self {
        match self {
            RenderMode::Normal => RenderMode::EdgeDetection,
            RenderMode::EdgeDetection => RenderMode::Normal
        }
    }
    
    pub fn name(&self) -> &'static str {
        match self {
            RenderMode::Normal => "Normal",
            RenderMode::EdgeDetection => "Edge Detection"
        }
    }
}

pub struct FileBrowser {
    pub current_dir: PathBuf,
    pub entries: Vec<PathBuf>,
    pub selected: usize,
    pub scroll_offset: usize,
}

impl FileBrowser {
    pub fn new() -> Self {
        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));
        let mut browser = Self {
            current_dir,
            entries: Vec::new(),
            selected: 0,
            scroll_offset: 0
        };
        browser.refresh();
        
        browser
    }
    
    pub fn refresh(&mut self) {
        self.entries.clear();
        
        if let Some(parent) = self.current_dir.parent() {
            self.entries.push(parent.to_path_buf());
        }
        
        if let Result::Ok(entries) = std::fs::read_dir(&self.current_dir) {
            let mut paths = entries
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| {
                    p.is_dir()
                        || p.extension()
                            .map(|ext| {
                                let ext = ext.to_string_lossy().to_lowercase();
                                matches!(
                                    ext.as_str(),
                                    "png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp" | "ico"
                                )
                            })
                            .unwrap_or(false)
                })
                .collect::<Vec<_>>();
            
            paths.sort_by(|a, b| {
                match (a.is_dir(), b.is_dir()) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => a.file_name().cmp(&b.file_name())
                }
            });
            
            self.entries.extend(paths);
        }
        
        self.selected = self.selected.min(self.entries.len().saturating_sub(1));
    }
    
    pub fn navigate_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }
    
    pub fn navigate_down(&mut self) {
        if self.selected < self.entries.len().saturating_sub(1) {
            self.selected += 1;
        }
    }
    
    pub fn enter(&mut self) -> Option<PathBuf> {
        if let Some(path) = self.entries.get(self.selected) {
            if path.is_dir() {
                self.current_dir = path.clone();
                self.refresh();
                self.selected = 0;
                None 
            } else {
                Some(path.clone())
            }
        } else {
            None 
        }
    }
    
    pub fn selected_path(&self) -> Option<&PathBuf> {
        self.entries.get(self.selected)
    }
}

pub struct App {
    pub mode: Mode,
    pub render_mode: RenderMode,
    pub config: AsciiConfig,
    pub image: Option<DynamicImage>,
    pub image_path: Option<PathBuf>,
    pub ascii_art: Option<AsciiArt>,
    pub file_browser: FileBrowser,
    pub scroll_y: usize,
    pub scroll_x: usize,
    pub message: Option<String>,
    pub should_quit: bool,
    pub save_path: String
}

impl App {
    pub fn new() -> Self {
        Self {
            mode: Mode::Normal,
            render_mode: RenderMode::Normal,
            config: AsciiConfig::default(),
            image: None,
            image_path: None,
            ascii_art: None,
            file_browser: FileBrowser::new(),
            scroll_y: 0,
            scroll_x: 0,
            message: Some("Press 'o' to open an image, 'h' for help".to_string()),
            should_quit: false,
            save_path: String::from("output.txt")
        }
    }
    
    pub fn load_image(&mut self, path: &Path) -> anyhow::Result<()> {
        let image = image::open(path)?;
        self.image = Some(image);
        self.image_path = Some(path.to_path_buf());
        self.regenerate();
        self.message = Some(format!("Loaded: {}", path.display()));
        self.scroll_y = 0;
        self.scroll_x = 0;
        
        Ok(())
    }
    
    pub fn regenerate(&mut self) {
        if let Some(ref image) = self.image {
            let generator = AsciiGenerator::new(self.config.clone());
            self.ascii_art = Some(match self.render_mode {
                RenderMode::Normal => generator.generate(image),
                RenderMode::EdgeDetection => EdgeDetector::generate_edges(image, &self.config)
            });
        }
    }
    
    pub fn increase_width(&mut self) {
        self.config.width = (self.config.width + 10).min(300);
        self.regenerate();
    }
    
    pub fn decrease_width(&mut self) {
        self.config.width = self.config.width.saturating_sub(10).max(20);
        self.regenerate();
    }
    
    pub fn toggle_invert(&mut self) {
        self.config.invert = !self.config.invert;
        self.regenerate();
    }
    
    pub fn toggle_colored(&mut self) {
        self.config.colored = !self.config.colored;
        self.regenerate();
    }
    
    pub fn next_charset(&mut self) {
        self.config.char_set = self.config.char_set.next();
        self.regenerate();
    }
    
    pub fn prev_charset(&mut self) {
        self.config.char_set = self.config.char_set.prev();
        self.regenerate();
    }
    
    pub fn toggle_render_mode(&mut self) {
        self.render_mode = self.render_mode.toggle();
        self.regenerate();
    }
    
    pub fn scroll_up(&mut self, amount: usize) {
        self.scroll_y = self.scroll_y.saturating_sub(amount);
    }
    
    pub fn scroll_down(&mut self, amount: usize) {
        if let Some(ref art) = self.ascii_art {
            self.scroll_y = (self.scroll_y + amount).min(art.height.saturating_sub(1));
        }
    }
    
    pub fn scroll_left(&mut self, amount: usize) {
        self.scroll_x = self.scroll_x.saturating_sub(amount);
    }
    
    pub fn scroll_right(&mut self, amount: usize) {
        if let Some(ref art) = self.ascii_art {
            self.scroll_x = (self.scroll_x + amount).min(art.width.saturating_sub(1));
        }
    }
    
    pub fn save_to_file(&mut self) -> anyhow::Result<()> {
        if let Some(ref art) = self.ascii_art {
            let content = if self.config.colored {
                art.to_string_colored()
            } else {
                art.to_string_plain()
            };
            std::fs::write(&self.save_path, content)?;
            self.message = Some(format!("Saved tp: {}", self.save_path));
        } else {
            self.message = Some("No image loaded".to_string());
        }
        
        Ok(())
    }
    
    pub fn get_image_info(&self) -> Option<String> {
        self.image.as_ref().map(|img| {
            let (w, h) = img.dimensions();
            format!("{}x{}", w, h)
        })
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}