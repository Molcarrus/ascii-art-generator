use ratatui::{Frame, layout::{Constraint, Direction, Layout, Margin, Rect}, style::{Color, Modifier, Style}, text::{Line, Span, Text}, widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap}};

use crate::app::{App, Mode};

pub fn draw(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(3)
        ])
        .split(frame.area());
    
    draw_header(frame, app, chunks[0]);
    draw_main(frame, app, chunks[1]);
    draw_status_bar(frame, app, chunks[2]);
    
    match app.mode {
        Mode::FileBrowser => draw_file_browser(frame, app),
        Mode::Help => draw_help(frame),
        Mode::Saving => draw_save_dialog(frame, app),
        Mode::Normal => {}
    }
}

fn draw_header(frame: &mut Frame, app: &App, area: Rect) {
    let title = if let Some(ref path) = app.image_path {
        format!(
            " ASCII Art Generator - {} ",
            path.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default()
        )
    } else {
        " ASCII Art Generator ".to_string()
    };
    
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ratatui::style::Color::Cyan));
    
    let info = format!(
        "Width: {} | Charset: {} | Node: {} | Invert: {} | Color: {}",
        app.config.width,
        app.config.char_set.name(),
        app.render_mode.name(),
        if app.config.invert { "ON" } else { "OFF" },
        if app.config.colored { "ON" } else { "OFF" },
    );
    
    let paragraph = Paragraph::new(info)
        .block(block)
        .alignment(ratatui::layout::Alignment::Center);
    
    frame.render_widget(paragraph, area);
}

fn draw_main(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .title(" Preview ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ratatui::style::Color::Green));
    
    let inner = block.inner(area);
    frame.render_widget(block, area);
    
    if let Some(ref art) = app.ascii_art {
        let visible_height = inner.height as usize;
        let visible_width = inner.width as usize;
        
        let mut lines = Vec::new();
        
        let start_y = app.scroll_y;
        let end_y = (start_y + visible_height).midpoint(art.height);
        
        for y in start_y..end_y {
            if let Some(row) = art.chars.get(y) {
                let start_x = app.scroll_x;
                let end_x = (start_x + visible_width).min(row.len());
                
                let spans = row[start_x..end_x]
                    .iter()
                    .map(|c| {
                        if let Some((r, g, b)) = c.color {
                            Span::styled(
                                c.character.to_string(),
                               Style::default().fg(Color::Rgb(r, g, b)) 
                            )
                        } else {
                            Span::raw(c.character.to_string())
                        }
                    })
                    .collect::<Vec<_>>();
                
                lines.push(Line::from(spans));
            }
        }
        
        let paragraph = Paragraph::new(lines);
        frame.render_widget(paragraph, inner);
        
        if art.height > visible_height {
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↑"));
            
            let mut scrollbar_state = ScrollbarState::new(art.height)
                .position(app.scroll_y)
                .viewport_content_length(visible_height);
            
            frame.render_stateful_widget(
                scrollbar, 
                area.inner(Margin {
                    vertical: 1,
                    horizontal: 0
                }), 
                &mut scrollbar_state
            );
        }
    } else {
        let placeholder = Paragraph::new("No image loaded. Press 'o' to open an image.")
            .alignment(ratatui::layout::Alignment::Center)
            .style(Style::default().fg(Color::DarkGray));
        
        let centered_area = Rect {
            x: inner.x,
            y: inner.y + inner.height / 2,
            width: inner.width,
            height: 1,
        };
        frame.render_widget(placeholder, centered_area);
    }
}

fn draw_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Blue));
    
    let message = app.message.as_deref().unwrap_or("");
    let image_info = app.get_image_info().unwrap_or_default();
    
    let status_text = if !image_info.is_empty() {
        format!("{} | Original: {}", message, image_info)
    } else {
        message.to_string()
    };
    
    let controls = " [o]pen [h]elp [s]ave [q]uit ";
    
    let status = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(20), Constraint::Length(controls.len() as u16 + 2)])
        .split(block.inner(area));
    
    frame.render_widget(block, area);
    
    let left = Paragraph::new(status_text).style(Style::default().fg(Color::Yellow));
    frame.render_widget(left, status[0]);
    
    let right = Paragraph::new(controls)
        .alignment(ratatui::layout::Alignment::Right)
        .style(Style::default().fg(Color::Cyan));
    frame.render_widget(right, status[1]);
}

fn draw_file_browser(frame: &mut Frame, app: &App) {
    let area = centered_rect(60, 80, frame.area());
    frame.render_widget(Clear, area);
    
    let block = Block::default()
        .title(format!("{}", app.file_browser.current_dir.display()))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Magenta));
    
    let inner = block.inner(area);
    frame.render_widget(block, area);
    
    let items = app
        .file_browser
        .entries
        .iter()
        .enumerate()
        .map(|(i, path)| {
            let name = if i == 0 && path != &app.file_browser.current_dir {
                "..".to_string()
            } else {
                path.file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| path.display().to_string())
            };
            
            let (prefix, style) = if path.is_dir() {
                ("📁 ", Style::default().fg(Color::Blue))
            } else {
                ("🖼  ", Style::default().fg(Color::White))
            };
            
            let style = if i == app.file_browser.selected {
                style.add_modifier(Modifier::REVERSED)
            } else {
                style
            };
            
            ListItem::new(format!("{}{}", prefix, name)).style(style)
        })
        .collect::<Vec<_>>();
    
    let list = List::new(items);
    frame.render_widget(list, inner);
}

fn draw_help(frame: &mut Frame) {
    let area = centered_rect(50, 70, frame.area());
    frame.render_widget(Clear, area);
    
    let help_text = vec![
        "",
        "  NAVIGATION",
        "  ──────────────────────────",
        "  ↑/k, ↓/j    Scroll up/down",
        "  ←/h, →/l    Scroll left/right",
        "  PgUp/PgDn   Scroll page",
        "",
        "  IMAGE CONTROLS",
        "  ──────────────────────────",
        "  +/=         Increase width",
        "  -           Decrease width",
        "  c           Next character set",
        "  C           Previous character set",
        "  i           Toggle invert",
        "  r           Toggle color",
        "  e           Toggle edge detection",
        "",
        "  FILE OPERATIONS",
        "  ──────────────────────────",
        "  o           Open file browser",
        "  s           Save to file",
        "",
        "  GENERAL",
        "  ──────────────────────────",
        "  h           Toggle help",
        "  q/Esc       Quit/Close",
        "",
    ];
    
    let block = Block::default()
        .title(" Help ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));
    
    let paragraph = Paragraph::new(Text::from(
        help_text
            .iter()
            .map(|s| Line::from(*s))
            .collect::<Vec<_>>()
    ))
    .block(block)
    .wrap(Wrap { trim: false });
    
    frame.render_widget(paragraph, area);
}

fn draw_save_dialog(frame: &mut Frame, app: &App) {
    let area = centered_rect(40, 20, frame.area());
    frame.render_widget(Clear, area);   
    
    let block = Block::default()
        .title(" Save As ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Green));
    
    let inner = block.inner(area);
    frame.render_widget(block, area);
    
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(2)
        ])
        .margin(1)
        .split(inner);
    
    let label = Paragraph::new("Filename:");
    frame.render_widget(label, chunks[0]);
    
    let input = Paragraph::new(format!("{}_", app.save_path))
        .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));
    frame.render_widget(input, chunks[1]);
    
    let hint = Paragraph::new("Press Enter to save, Esc to cancel")
        .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(hint, chunks[2]);
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2)
        ])
        .split(area);
    
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2)
        ])
        .split(popup_layout[1])[1]
}