use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Tabs},
    Frame,
};

use crate::app::{App, Tab};

pub fn render(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(frame.area());

    render_tabs(frame, app, chunks[0]);
    render_content(frame, app, chunks[1]);
    render_status_bar(frame, app, chunks[2]);
}

fn render_tabs(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let titles: Vec<Line> = Tab::ALL
        .iter()
        .map(|t| Line::from(Span::raw(t.title())))
        .collect();

    let tabs = Tabs::new(titles)
        .block(Block::default().borders(Borders::ALL).title(" MiaoMC Launcher "))
        .select(app.current_tab)
        .style(Style::default().fg(Color::White))
        .highlight_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));

    frame.render_widget(tabs, area);
}

fn render_content(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    match app.active_tab() {
        Tab::Instances => render_instances(frame, app, area),
        Tab::Versions => render_placeholder(frame, "Versions", "Press Enter to browse available Minecraft versions.", area),
        Tab::Accounts => render_accounts(frame, app, area),
        Tab::Settings => render_placeholder(frame, "Settings", "Download mirror, Java paths, concurrent downloads.", area),
    }
}

fn render_instances(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let items: Vec<ListItem> = if app.instances.is_empty() {
        vec![ListItem::new("  No instances. Press 'n' to create one.")]
    } else {
        app.instances
            .iter()
            .enumerate()
            .map(|(i, inst)| {
                let loader = inst
                    .mod_loader
                    .as_ref()
                    .map(|l| format!(" [{}]", l.loader_type))
                    .unwrap_or_default();
                let prefix = if i == app.selected_index { "▶ " } else { "  " };
                ListItem::new(format!(
                    "{}{} - MC {}{}",
                    prefix, inst.name, inst.minecraft_version, loader
                ))
            })
            .collect()
    };

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(" Instances "))
        .highlight_style(Style::default().fg(Color::Yellow));

    frame.render_widget(list, area);
}

fn render_accounts(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let items: Vec<ListItem> = if app.config.accounts.is_empty() {
        vec![ListItem::new("  No accounts. Press 'a' to add one.")]
    } else {
        app.config
            .accounts
            .iter()
            .enumerate()
            .map(|(i, acc)| {
                let active = app
                    .config
                    .active_account_index
                    .map(|idx| idx == i)
                    .unwrap_or(false);
                let marker = if active { "★" } else { " " };
                let acc_type = if acc.is_microsoft() { "MS" } else { "Offline" };
                ListItem::new(format!(" {} [{}] {}", marker, acc_type, acc.username()))
            })
            .collect()
    };

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(" Accounts "));

    frame.render_widget(list, area);
}

fn render_placeholder(frame: &mut Frame, title: &str, msg: &str, area: ratatui::layout::Rect) {
    let paragraph = Paragraph::new(format!("\n  {}", msg))
        .block(Block::default().borders(Borders::ALL).title(format!(" {} ", title)));
    frame.render_widget(paragraph, area);
}

fn render_status_bar(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let status = Paragraph::new(Line::from(vec![
        Span::styled(" [Tab]", Style::default().fg(Color::Cyan)),
        Span::raw(" Switch  "),
        Span::styled("[↑↓/jk]", Style::default().fg(Color::Cyan)),
        Span::raw(" Navigate  "),
        Span::styled("[Enter]", Style::default().fg(Color::Cyan)),
        Span::raw(" Select  "),
        Span::styled("[q]", Style::default().fg(Color::Cyan)),
        Span::raw(" Quit  │  "),
        Span::styled(&app.status_message, Style::default().fg(Color::DarkGray)),
    ]))
    .block(Block::default().borders(Borders::ALL));

    frame.render_widget(status, area);
}
