use std::{env, io};

use anyhow::Ok;
use crossterm::{ExecutableCommand, event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind}, execute, terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode}};
use ratatui::{Terminal, prelude::{Backend, CrosstermBackend}};

use crate::app::{App, Mode};

mod ascii;
mod app;
mod ui;

fn main() -> anyhow::Result<()> {
    let args = env::args().collect::<Vec<_>>();
    
    if args.len() > 1 && args.contains(&"--help".to_string()) {
        print_cli_help();
        return Ok(());
    }
    
    if args.len() > 1 && args.contains(&"--tui".to_string()) {
        return run_cli(&args);
    }
    
    run_tui()
}

fn print_cli_help() {
    println!("ASCII Art Generator");
    println!();
    println!("USAGE:");
    println!("    ascii-art [OPTIONS] [IMAGE]");
    println!();
    println!("OPTIONS:");
    println!("    --help              Show this help message");
    println!("    --tui               Force TUI mode");
    println!("    -w, --width <N>     Output width (default: 80)");
    println!("    -c, --charset <S>   Character set: standard, detailed, blocks, minimal, binary");
    println!("    -i, --invert        Invert brightness");
    println!("    -o, --output <F>    Save to file");
    println!("    --color             Enable colored output (ANSI)");
    println!("    --edge              Use edge detection mode");
    println!();
    println!("EXAMPLES:");
    println!("    ascii-art image.png");
    println!("    ascii-art -w 120 --charset detailed image.jpg");
    println!("    ascii-art --tui");
}

fn run_cli(args: &[String]) -> anyhow::Result<()> {
    use ascii::{AsciiConfig, AsciiGenerator, CharacterSet, EdgeDetector};

    let mut config = AsciiConfig::default();
    let mut image_path = None;
    let mut output_file = None;
    let mut edge_mode = false;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-w" | "--width" => {
                i += 1;
                if i < args.len() {
                    config.width = args[i].parse().unwrap_or(80);
                }
            }
            "-c" | "--charset" => {
                i += 1;
                if i < args.len() {
                    config.char_set = match args[i].to_lowercase().as_str() {
                        "detailed" => CharacterSet::Detailed,
                        "blocks" => CharacterSet::Blocks,
                        "minimal" => CharacterSet::Minimal,
                        "binary" => CharacterSet::Binary,
                        _ => CharacterSet::Standard,
                    };
                }
            }
            "-i" | "--invert" => {
                config.invert = true;
            }
            "--color" => {
                config.colored = true;
            }
            "--edge" => {
                edge_mode = true;
            }
            "-o" | "--output" => {
                i += 1;
                if i < args.len() {
                    output_file = Some(args[i].clone());
                }
            }
            path if !path.starts_with('-') => {
                image_path = Some(path.to_string());
            }
            _ => {}
        }
        i += 1;
    }

    let image_path = image_path.ok_or_else(|| anyhow::anyhow!("No image file specified"))?;
    let image = image::open(&image_path)?;

    let art = if edge_mode {
        EdgeDetector::generate_edges(&image, &config)
    } else {
        let generator = AsciiGenerator::new(config.clone());
        generator.generate(&image)
    };

    let output = if config.colored {
        art.to_string_colored()
    } else {
        art.to_string_plain()
    };

    if let Some(file) = output_file {
        std::fs::write(&file, &output)?;
        eprintln!("Saved to: {}", file);
    } else {
        println!("{}", output);
    }

    Ok(())
}

fn setup_terminal() -> io::Result<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    stdout.execute(EnterAlternateScreen)?;
    stdout.execute(EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    Terminal::new(backend)
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) {
    let _ = disable_raw_mode();
    _ = terminal.backend_mut().execute(LeaveAlternateScreen);
    _ = terminal.backend_mut().execute(DisableMouseCapture);
    let _ = terminal.show_cursor();
}

fn run_tui() -> anyhow::Result<()> {
    enable_raw_mode()?;
    let mut terminal = setup_terminal().map_err(|e| anyhow::anyhow!("Failed to setup terminal: {}", e))?;
    
    let mut app = App::new();
    
    let result = run_app(&mut terminal, &mut app);
    
    restore_terminal(&mut terminal);
    
    result
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> anyhow::Result<()> {
    loop {
        terminal.draw(|frame| ui::draw(frame, app))?;
        
        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }
            
            match app.mode {
                Mode::Normal => handle_normal_mode(app, key.code),
                Mode::FileBrowser => handle_file_browser_mode(app, key.code),
                Mode::Help => handle_help_mode(app, key.code),
                Mode::Saving => handle_save_mode(app, key.code),
            }
        }
        
        if app.should_quit {
            break;
        }
    }
    
    Ok(())
}

fn handle_normal_mode(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Char('q') | KeyCode::Esc => app.should_quit = true,
        KeyCode::Char('o') => app.mode = Mode::FileBrowser,
        KeyCode::Char('h') => app.mode = Mode::Help,
        KeyCode::Char('s') => app.mode = Mode::Saving,

        // Scrolling
        KeyCode::Up | KeyCode::Char('k') => app.scroll_up(1),
        KeyCode::Down | KeyCode::Char('j') => app.scroll_down(1),
        KeyCode::Left => app.scroll_left(1),
        KeyCode::Right => app.scroll_right(1),
        KeyCode::PageUp => app.scroll_up(10),
        KeyCode::PageDown => app.scroll_down(10),
        KeyCode::Home => {
            app.scroll_y = 0;
            app.scroll_x = 0;
        }

        // Configuration
        KeyCode::Char('+') | KeyCode::Char('=') => app.increase_width(),
        KeyCode::Char('-') => app.decrease_width(),
        KeyCode::Char('c') => app.next_charset(),
        KeyCode::Char('C') => app.prev_charset(),
        KeyCode::Char('i') => app.toggle_invert(),
        KeyCode::Char('r') => app.toggle_colored(),
        KeyCode::Char('e') => app.toggle_render_mode(),

        _ => {}
    }
}

fn handle_file_browser_mode(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc | KeyCode::Char('q') => app.mode = Mode::Normal,
        KeyCode::Up | KeyCode::Char('k') => app.file_browser.navigate_up(),
        KeyCode::Down | KeyCode::Char('j') => app.file_browser.navigate_down(),
        KeyCode::Enter => {
            if let Some(path) = app.file_browser.enter() {
                if let Err(e) = app.load_image(&path) {
                    app.message = Some(format!("Error: {}", e));
                }
                app.mode = Mode::Normal;
            }
        }
        KeyCode::Backspace => {
            if let Some(parent) = app.file_browser.current_dir.parent() {
                app.file_browser.current_dir = parent.to_path_buf();
                app.file_browser.refresh();
                app.file_browser.selected = 0;
            }
        }
        _ => {}
    }
}

fn handle_help_mode(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('h') => app.mode = Mode::Normal,
        _ => {}
    }
}

fn handle_save_mode(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc => app.mode = Mode::Normal,
        KeyCode::Enter => {
            if let Err(e) = app.save_to_file() {
                app.message = Some(format!("Error saving: {}", e));
            }
            app.mode = Mode::Normal;
        }
        KeyCode::Backspace => {
            app.save_path.pop();
        }
        KeyCode::Char(c) => {
            app.save_path.push(c);
        }
        _ => {}
    }
}
