use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

use crate::app::{App, InputMode, ViewMode};

pub fn render(frame: &mut Frame, app: &App) {
    let main_constraints = if app.show_log {
        vec![
            Constraint::Min(8),
            Constraint::Length(10),
            Constraint::Length(3),
        ]
    } else {
        vec![Constraint::Min(0), Constraint::Length(3)]
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(main_constraints)
        .split(frame.area());

    let content_area = chunks[0];
    let status_area = if app.show_log { chunks[2] } else { chunks[1] };

    match app.input_mode {
        InputMode::CreateName | InputMode::CreateSelectVersion | InputMode::CreateSelectLoader => {
            render_create_wizard(frame, app, content_area);
        }
        InputMode::Settings => {
            render_settings(frame, app, content_area);
        }
        InputMode::AccountInput | InputMode::AccountView => {
            render_accounts(frame, app, content_area);
        }
        _ => {
            render_main_layout(frame, app, content_area);
        }
    }

    if app.show_log {
        render_log_panel(frame, app, chunks[1]);
    }

    render_status_bar(frame, app, status_area);
}

fn render_main_layout(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(area);

    render_instance_list(frame, app, columns[0]);
    render_detail_panel(frame, app, columns[1]);
}

fn render_instance_list(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let items: Vec<ListItem> = if app.instances.is_empty() {
        vec![ListItem::new("  (no instances)")]
    } else {
        app.instances
            .iter()
            .enumerate()
            .map(|(i, inst)| {
                let prefix = if i == app.selected_index {
                    "▶ "
                } else {
                    "  "
                };
                let loader = inst
                    .mod_loader
                    .as_ref()
                    .map(|l| format!(" [{}]", l.loader_type))
                    .unwrap_or_default();
                ListItem::new(format!("{}{}{}", prefix, inst.name, loader))
            })
            .collect()
    };

    let title = " Instances [n]ew [a]ccount [,]settings [q]uit ";
    let list = List::new(items).block(Block::default().borders(Borders::ALL).title(title));
    frame.render_widget(list, area);
}

fn render_detail_panel(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let Some(inst) = app.instances.get(app.selected_index) else {
        let empty = Paragraph::new("\n  Select an instance or press 'n' to create one.")
            .block(Block::default().borders(Borders::ALL).title(" Detail "));
        frame.render_widget(empty, area);
        return;
    };

    let mut lines: Vec<Line> = Vec::new();

    lines.push(Line::from(vec![
        Span::styled(" Name: ", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(&inst.name),
    ]));
    lines.push(Line::from(vec![
        Span::styled(" MC:   ", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(&inst.minecraft_version),
    ]));

    let loader_str = inst
        .mod_loader
        .as_ref()
        .map(|l| format!("{} {}", l.loader_type, l.version))
        .unwrap_or_else(|| "Vanilla".to_string());
    lines.push(Line::from(vec![
        Span::styled(" Loader:", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(format!(" {}", loader_str)),
    ]));

    lines.push(Line::raw(""));
    lines.push(Line::styled(
        " [Enter]launch [d]elete [j]ava [o]pen folder",
        Style::default().fg(Color::DarkGray),
    ));
    lines.push(Line::raw(""));

    match app.view_mode {
        ViewMode::Detail => {
            lines.push(Line::styled(
                " ─── Resources [m]ods [r]esources [s]haders [w]orlds ───",
                Style::default().fg(Color::Cyan),
            ));
            lines.push(Line::raw(""));

            let instance_dir = miao_core::instance::Instance::instance_dir(
                &app.config.instances_dir(),
                &inst.name,
            );

            let mods_dir = miao_core::instance::Instance::mods_dir(&instance_dir);
            let mods = miao_core::modmanager::scan_mods_dir(&mods_dir);
            lines.push(Line::styled(
                format!("  Mods ({})", mods.len()),
                Style::default().add_modifier(Modifier::BOLD),
            ));
            for m in mods.iter().take(5) {
                let status = if m.enabled { "✓" } else { "✗" };
                lines.push(Line::raw(format!("    {} {}", status, m.name)));
            }
            if mods.len() > 5 {
                lines.push(Line::styled(
                    format!("    ... and {} more", mods.len() - 5),
                    Style::default().fg(Color::DarkGray),
                ));
            }

            lines.push(Line::raw(""));

            let res_dir = miao_core::instance::Instance::resourcepacks_dir(&instance_dir);
            let resources = miao_core::resource::scan_resourcepacks(&res_dir);
            lines.push(Line::styled(
                format!("  Resource Packs ({})", resources.len()),
                Style::default().add_modifier(Modifier::BOLD),
            ));
            for r in resources.iter().take(3) {
                lines.push(Line::raw(format!("    {}", r.name)));
            }

            let shader_dir = miao_core::instance::Instance::shaderpacks_dir(&instance_dir);
            let shaders = miao_core::resource::scan_shaderpacks(&shader_dir);
            if !shaders.is_empty() {
                lines.push(Line::raw(""));
                lines.push(Line::styled(
                    format!("  Shaders ({})", shaders.len()),
                    Style::default().add_modifier(Modifier::BOLD),
                ));
                for s in shaders.iter().take(3) {
                    lines.push(Line::raw(format!("    {}", s.name)));
                }
            }

            let saves = miao_core::instance::list_saves(&instance_dir);
            if !saves.is_empty() {
                lines.push(Line::raw(""));
                lines.push(Line::styled(
                    format!("  Worlds ({})", saves.len()),
                    Style::default().add_modifier(Modifier::BOLD),
                ));
                for s in saves.iter().take(3) {
                    lines.push(Line::raw(format!("    {}", s.name)));
                }
            }
        }
        ViewMode::Mods => render_manage_list(&mut lines, app, "Mods"),
        ViewMode::ResourcePacks => render_manage_list(&mut lines, app, "Resource Packs"),
        ViewMode::Shaders => render_manage_list(&mut lines, app, "Shaders"),
        ViewMode::Saves => render_manage_list(&mut lines, app, "Worlds"),
    }

    let title = if app.input_mode == InputMode::ConfirmDelete {
        " ⚠ Delete? [y]es [n]o "
    } else {
        " Instance "
    };

    let detail = Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title(title));
    frame.render_widget(detail, area);
}

fn render_manage_list(lines: &mut Vec<Line>, app: &App, section: &str) {
    lines.push(Line::styled(
        format!(" ─── {} [j/k]nav [d]elete [o]pen [Esc]back ───", section),
        Style::default().fg(Color::Cyan),
    ));

    if section == "Mods" {
        lines.push(Line::styled(
            "      [Enter]toggle",
            Style::default().fg(Color::DarkGray),
        ));
    }

    lines.push(Line::raw(""));

    let items: Vec<String> = match app.view_mode {
        ViewMode::Mods => app
            .manage_mods
            .iter()
            .map(|m| {
                let status = if m.enabled { "✓" } else { "✗" };
                format!("{} {}", status, m.name)
            })
            .collect(),
        ViewMode::ResourcePacks => app
            .manage_resourcepacks
            .iter()
            .map(|r| r.name.clone())
            .collect(),
        ViewMode::Shaders => app.manage_shaders.iter().map(|s| s.name.clone()).collect(),
        ViewMode::Saves => app.manage_saves.iter().map(|s| s.name.clone()).collect(),
        _ => Vec::new(),
    };

    if items.is_empty() {
        lines.push(Line::styled(
            "  (empty)",
            Style::default().fg(Color::DarkGray),
        ));
    } else {
        for (i, item) in items.iter().enumerate() {
            let prefix = if i == app.manage_cursor {
                "  ▶ "
            } else {
                "    "
            };
            lines.push(Line::raw(format!("{}{}", prefix, item)));
        }
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

    let name_marker = if app.input_mode == InputMode::CreateName {
        "▶"
    } else {
        "✓"
    };
    items.push(ListItem::new(format!(
        "  │ {} Name: {}_",
        name_marker, name_display
    )));
    items.push(ListItem::new("  │"));

    let ver_marker = if app.input_mode == InputMode::CreateSelectVersion {
        "▶"
    } else if matches!(app.input_mode, InputMode::CreateSelectLoader) {
        "✓"
    } else {
        " "
    };

    if app.input_mode == InputMode::CreateSelectVersion {
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

    if app.input_mode == InputMode::CreateSelectLoader {
        items.push(ListItem::new("  │ ▶ Mod Loader:"));

        if app.loading_loader_versions {
            items.push(ListItem::new("  │   Loading..."));
        } else {
            let available_loaders = app.get_available_loaders();
            for (idx, name, available) in &available_loaders {
                let prefix = if *idx == app.create_loader_cursor {
                    "    ▶ "
                } else {
                    "      "
                };
                let suffix = if !available { " (unavailable)" } else { "" };
                items.push(ListItem::new(format!("  │ {}{}{}", prefix, name, suffix)));
            }

            if app.create_loader_cursor > 0 {
                let loader_versions = app.get_current_loader_versions();
                if !loader_versions.is_empty() {
                    items.push(ListItem::new("  │"));
                    items.push(ListItem::new("  │   Version [←/→]:"));
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

fn render_settings(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let java_installations = miao_core::java::detect_system_java();
    let java_str = if java_installations.is_empty() {
        "  None detected".to_string()
    } else {
        java_installations
            .iter()
            .map(|j| {
                format!(
                    "  Java {} ({}) - {}",
                    j.major_version,
                    j.version,
                    j.path.display()
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    let text = format!(
        "\n  Data dir: {}\n  Mirror: {:?}\n  Concurrent downloads: {}\n\n  Java installations:\n{}",
        app.config.data_dir.display(),
        app.config.download_mirror,
        app.config.max_concurrent_downloads,
        java_str,
    );

    let paragraph = Paragraph::new(text).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Settings [Esc=back] "),
    );
    frame.render_widget(paragraph, area);
}

fn render_accounts(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let mut lines: Vec<ListItem> = Vec::new();

    if app.config.accounts.is_empty() {
        lines.push(ListItem::new("  No accounts configured."));
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

    lines.push(ListItem::new(""));
    lines.push(ListItem::new(
        "  [o]ffline account  [m]icrosoft login  [Esc]back",
    ));

    if app.input_mode == InputMode::AccountInput {
        lines.push(ListItem::new(""));
        lines.push(ListItem::new(format!("  Username: {}_", app.input_buffer)));
    }

    let list = List::new(lines).block(Block::default().borders(Borders::ALL).title(" Accounts "));
    frame.render_widget(list, area);
}

fn render_log_panel(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let visible_lines = area.height.saturating_sub(2) as usize;
    let start = app.log_messages.len().saturating_sub(visible_lines);
    let items: Vec<ListItem> = app.log_messages[start..]
        .iter()
        .map(|msg| ListItem::new(format!("  {}", msg)))
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Log [L=close] "),
    );
    frame.render_widget(list, area);
}

fn render_status_bar(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let status = Paragraph::new(Line::from(vec![Span::styled(
        format!(" {}", &app.status_message),
        Style::default().fg(Color::Yellow),
    )]))
    .block(Block::default().borders(Borders::ALL));
    frame.render_widget(status, area);
}
