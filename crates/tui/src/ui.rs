use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Tabs},
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
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" MiaoMC Launcher "),
        )
        .select(app.current_tab)
        .style(Style::default().fg(Color::White))
        .highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );

    frame.render_widget(tabs, area);
}

fn render_content(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    if matches!(
        app.input_mode,
        crate::app::InputMode::CreateName
            | crate::app::InputMode::CreateSelectVersion
            | crate::app::InputMode::CreateSelectLoader
    ) {
        render_create_wizard(frame, app, area);
        return;
    }

    match app.active_tab() {
        Tab::Instances => render_instances(frame, app, area),
        Tab::Versions => render_versions(frame, app, area),
        Tab::Accounts => render_accounts(frame, app, area),
        Tab::Settings => render_settings(frame, app, area),
    }
}

fn render_create_wizard(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let mut items: Vec<ListItem> = Vec::new();

    items.push(ListItem::new("  ╭─ Create New Instance ─╮"));
    items.push(ListItem::new("  │"));

    let name_display = if app.create_name.is_empty() {
        "(auto from version)".to_string()
    } else {
        app.create_name.clone()
    };

    let name_marker = if app.input_mode == crate::app::InputMode::CreateName {
        "▶"
    } else {
        "✓"
    };
    items.push(ListItem::new(format!(
        "  │ {} Name: {}_",
        name_marker, name_display
    )));
    items.push(ListItem::new("  │"));

    let ver_marker = if app.input_mode == crate::app::InputMode::CreateSelectVersion {
        "▶"
    } else if matches!(app.input_mode, crate::app::InputMode::CreateSelectLoader) {
        "✓"
    } else {
        " "
    };

    if app.input_mode == crate::app::InputMode::CreateSelectVersion {
        items.push(ListItem::new(format!("  │ {} MC Version:", ver_marker)));
        let start = app.create_version_cursor.saturating_sub(3);
        let end = (start + 8).min(app.versions.len());
        for i in start..end {
            let prefix = if i == app.create_version_cursor {
                "    ▶ "
            } else {
                "      "
            };
            items.push(ListItem::new(format!(
                "  │ {}{}",
                prefix, app.versions[i].id
            )));
        }
    } else {
        let ver_name = app
            .versions
            .get(app.create_version_cursor)
            .map(|v| v.id.as_str())
            .unwrap_or("?");
        items.push(ListItem::new(format!(
            "  │ {} MC Version: {}",
            ver_marker, ver_name
        )));
    }

    items.push(ListItem::new("  │"));

    let loader_marker = if app.input_mode == crate::app::InputMode::CreateSelectLoader {
        "▶"
    } else {
        " "
    };

    if app.input_mode == crate::app::InputMode::CreateSelectLoader {
        items.push(ListItem::new(format!("  │ {} Mod Loader:", loader_marker)));

        if app.loading_loader_versions {
            items.push(ListItem::new("  │   Loading loader versions..."));
        } else {
            let available_loaders = app.get_available_loaders();
            for (idx, name, available) in &available_loaders {
                let prefix = if *idx == app.create_loader_cursor {
                    "    ▶ "
                } else {
                    "      "
                };
                let suffix = if !available { " (unavailable)" } else { "" };
                items.push(ListItem::new(format!(
                    "  │ {}{}{}",
                    prefix, name, suffix
                )));
            }

            if app.create_loader_cursor > 0 {
                let loader_versions = app.get_current_loader_versions();
                if !loader_versions.is_empty() {
                    items.push(ListItem::new("  │"));
                    items.push(ListItem::new("  │   Version:"));
                    let start = app.loader_version_cursor.saturating_sub(3);
                    let end = (start + 6).min(loader_versions.len());
                    for (i, v) in loader_versions.iter().enumerate().take(end).skip(start) {
                        let prefix = if i == app.loader_version_cursor {
                            "      ▶ "
                        } else {
                            "        "
                        };
                        let stable_marker = if v.stable { " ★" } else { "" };
                        items.push(ListItem::new(format!(
                            "  │ {}{}{}",
                            prefix, v.version, stable_marker
                        )));
                    }
                }
            }
        }
    }

    items.push(ListItem::new("  │"));
    items.push(ListItem::new("  ╰──────────────────────────╯"));

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" New Instance [Enter=next, Esc=cancel] "),
    );

    frame.render_widget(list, area);
}

fn render_instances(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let items: Vec<ListItem> = if app.instances.is_empty() {
        vec![ListItem::new(
            "  No instances. Go to Versions tab and press 'i' to install.",
        )]
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
                let prefix = if i == app.selected_index {
                    "▶ "
                } else {
                    "  "
                };
                ListItem::new(format!(
                    "{}{} - MC {}{}",
                    prefix, inst.name, inst.minecraft_version, loader
                ))
            })
            .collect()
    };

    if app.input_mode == crate::app::InputMode::SelectLoader {
        let mut loader_items: Vec<ListItem> = Vec::new();
        loader_items.push(ListItem::new("  Select Mod Loader:"));
        loader_items.push(ListItem::new(""));
        for (i, name) in crate::app::LOADER_OPTIONS.iter().enumerate() {
            let prefix = if i == app.loader_cursor {
                "  ▶ "
            } else {
                "    "
            };
            loader_items.push(ListItem::new(format!("{}{}", prefix, name)));
        }

        let list = List::new(loader_items).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Install Mod Loader [Enter=confirm, Esc=cancel] "),
        );
        frame.render_widget(list, area);
        return;
    }

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Instances [l=launch, f=loader] "),
    );

    frame.render_widget(list, area);
}

fn render_versions(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let items: Vec<ListItem> = if app.loading {
        vec![ListItem::new("  Loading versions...")]
    } else if app.versions.is_empty() {
        vec![ListItem::new("  No versions loaded. Check network.")]
    } else {
        app.versions
            .iter()
            .enumerate()
            .map(|(i, ver)| {
                let prefix = if i == app.selected_index {
                    "▶ "
                } else {
                    "  "
                };
                ListItem::new(format!("{}{:<16} {}", prefix, ver.id, ver.release_time))
            })
            .collect()
    };

    let title = if app.installing {
        " Versions [installing...] "
    } else {
        " Versions [i=install] "
    };

    let list = List::new(items).block(Block::default().borders(Borders::ALL).title(title));

    frame.render_widget(list, area);
}

fn render_accounts(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let mut lines: Vec<ListItem> = Vec::new();

    if app.config.accounts.is_empty() {
        lines.push(ListItem::new(
            "  No accounts. Press 'a' for offline, 'm' for Microsoft.",
        ));
    } else {
        for (i, acc) in app.config.accounts.iter().enumerate() {
            let active = app.config.active_account_index == Some(i);
            let marker = if active { "★" } else { " " };
            let acc_type = if acc.is_microsoft() { "MS" } else { "Offline" };
            lines.push(ListItem::new(format!(
                "  {} [{}] {}",
                marker,
                acc_type,
                acc.username()
            )));
        }
    }

    if let Some(dc) = &app.ms_device_code {
        lines.push(ListItem::new(""));
        lines.push(ListItem::new(format!(
            "  Microsoft Login: Go to {}",
            dc.verification_uri
        )));
        lines.push(ListItem::new(format!("  Enter code: {}", dc.user_code)));
        lines.push(ListItem::new("  Waiting for authorization..."));
    }

    if app.input_mode == crate::app::InputMode::Input {
        lines.push(ListItem::new(""));
        lines.push(ListItem::new(format!("  Username: {}_", app.input_buffer)));
    }

    let list = List::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Accounts [a=offline, m=microsoft] "),
    );

    frame.render_widget(list, area);
}

fn render_settings(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let text = format!(
        "\n  Data dir: {}\n  Mirror: {:?}\n  Concurrent downloads: {}\n  Java: {}",
        app.config.data_dir.display(),
        app.config.download_mirror,
        app.config.max_concurrent_downloads,
        if app.config.java_paths.is_empty() {
            "auto-detect".to_string()
        } else {
            app.config
                .java_paths
                .iter()
                .map(|p| p.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        }
    );

    let paragraph =
        Paragraph::new(text).block(Block::default().borders(Borders::ALL).title(" Settings "));
    frame.render_widget(paragraph, area);
}

fn render_status_bar(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let status = Paragraph::new(Line::from(vec![Span::styled(
        format!(" {}", &app.status_message),
        Style::default().fg(Color::Yellow),
    )]))
    .block(Block::default().borders(Borders::ALL));

    frame.render_widget(status, area);
}
