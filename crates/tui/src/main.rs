mod app;
mod ui;

use anyhow::Result;
use app::{InputMode, ViewMode};
use crossterm::{
    ExecutableCommand,
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::prelude::*;
use std::io::stdout;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    let mut app = app::App::new()?;

    fetch_versions(&mut app);

    let result = run_app(&mut terminal, &mut app).await;

    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;

    result
}

fn fetch_versions(app: &mut app::App) {
    let tx = app.tx.clone();
    let mirror = app.config.download_mirror.clone();

    tokio::spawn(async move {
        let http = reqwest::Client::new();
        if let Ok(versions) =
            miao_core::version::manifest::fetch_version_manifest(&http, &mirror).await
        {
            let _ = tx.send(app::AsyncMessage::VersionsLoaded(versions));
        }
    });
}

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    app: &mut app::App,
) -> Result<()> {
    loop {
        app.process_messages();
        terminal.draw(|frame| ui::render(frame, app))?;

        if event::poll(Duration::from_millis(100))?
            && let Event::Key(key) = event::read()?
        {
            if key.kind != KeyEventKind::Press {
                continue;
            }

            match app.input_mode {
                InputMode::Normal => handle_normal(app, key.code),
                InputMode::CreateName => handle_create_name(app, key.code),
                InputMode::CreateSelectVersion => handle_create_version(app, key.code),
                InputMode::CreateSelectLoader => handle_create_loader(app, key.code),
                InputMode::ConfirmDelete => handle_confirm_delete(app, key.code),
                InputMode::Settings => handle_settings(app, key.code),
                InputMode::AccountView => handle_account_view(app, key.code),
                InputMode::AccountInput => handle_account_input(app, key.code),
            }

            if matches!(app.input_mode, InputMode::Normal) && key.code == KeyCode::Char('q') {
                return Ok(());
            }
        }
    }
}

fn handle_normal(app: &mut app::App, key: KeyCode) {
    match app.view_mode {
        ViewMode::Detail => match key {
            KeyCode::Up | KeyCode::Char('k') => app.prev_instance(),
            KeyCode::Down | KeyCode::Char('j') => app.next_instance(),
            KeyCode::Enter => app.launch_selected(),
            KeyCode::Char('n') => app.start_create_instance(),
            KeyCode::Char('d') if !app.instances.is_empty() => {
                app.input_mode = InputMode::ConfirmDelete;
                app.status_message = "Delete this instance? [y]es / any key = cancel".to_string();
            }
            KeyCode::Char('J') => app.download_java_for_instance(),
            KeyCode::Char('o') => app.open_instance_folder(),
            KeyCode::Char('m') => app.open_manage_mods(),
            KeyCode::Char('r') => app.open_manage_resourcepacks(),
            KeyCode::Char('s') => app.open_manage_shaders(),
            KeyCode::Char('w') => app.open_manage_saves(),
            KeyCode::Char('a') => app.start_add_account(),
            KeyCode::Char('L') => {
                app.show_log = !app.show_log;
            }
            KeyCode::Char(',') => {
                app.input_mode = InputMode::Settings;
            }
            _ => {}
        },
        ViewMode::Mods | ViewMode::ResourcePacks | ViewMode::Shaders | ViewMode::Saves => match key
        {
            KeyCode::Up | KeyCode::Char('k') => app.manage_prev(),
            KeyCode::Down | KeyCode::Char('j') => app.manage_next(),
            KeyCode::Enter if app.view_mode == ViewMode::Mods => app.toggle_current_mod(),
            KeyCode::Char('d') => app.delete_current_resource(),
            KeyCode::Char('o') => {
                if let Some(inst) = app.instances.get(app.selected_index) {
                    let instance_dir = miao_core::instance::Instance::instance_dir(
                        &app.config.instances_dir(),
                        &inst.name,
                    );
                    let dir = match app.view_mode {
                        ViewMode::Mods => miao_core::instance::Instance::mods_dir(&instance_dir),
                        ViewMode::ResourcePacks => {
                            miao_core::instance::Instance::resourcepacks_dir(&instance_dir)
                        }
                        ViewMode::Shaders => {
                            miao_core::instance::Instance::shaderpacks_dir(&instance_dir)
                        }
                        ViewMode::Saves => miao_core::instance::Instance::saves_dir(&instance_dir),
                        _ => instance_dir,
                    };
                    let _ = miao_core::instance::open_folder(&dir);
                }
            }
            KeyCode::Esc => {
                app.view_mode = ViewMode::Detail;
            }
            _ => {}
        },
    }
}

fn handle_create_name(app: &mut app::App, key: KeyCode) {
    match key {
        KeyCode::Enter => app.create_name_confirm(),
        KeyCode::Esc => {
            app.input_mode = InputMode::Normal;
            app.status_message = "Cancelled.".to_string();
        }
        KeyCode::Backspace => {
            app.create_name.pop();
        }
        KeyCode::Char(c) => app.create_name.push(c),
        _ => {}
    }
}

fn handle_create_version(app: &mut app::App, key: KeyCode) {
    match key {
        KeyCode::Up | KeyCode::Char('k') => app.create_version_prev(),
        KeyCode::Down | KeyCode::Char('j') => app.create_version_next(),
        KeyCode::Enter => app.create_version_confirm(),
        KeyCode::Esc => {
            app.input_mode = InputMode::Normal;
            app.status_message = "Cancelled.".to_string();
        }
        _ => {}
    }
}

fn handle_create_loader(app: &mut app::App, key: KeyCode) {
    match key {
        KeyCode::Up | KeyCode::Char('k') => app.create_loader_prev(),
        KeyCode::Down | KeyCode::Char('j') => app.create_loader_next(),
        KeyCode::Left | KeyCode::Char('h') => app.loader_version_prev(),
        KeyCode::Right | KeyCode::Char('l') => app.loader_version_next(),
        KeyCode::Enter => app.create_loader_confirm(),
        KeyCode::Esc => {
            app.input_mode = InputMode::Normal;
            app.status_message = "Cancelled.".to_string();
        }
        _ => {}
    }
}

fn handle_confirm_delete(app: &mut app::App, key: KeyCode) {
    match key {
        KeyCode::Char('y') => app.confirm_delete_instance(),
        _ => {
            app.input_mode = InputMode::Normal;
            app.status_message = "Cancelled.".to_string();
        }
    }
}

fn handle_settings(app: &mut app::App, key: KeyCode) {
    if key == KeyCode::Esc {
        app.input_mode = InputMode::Normal;
    }
}

fn handle_account_view(app: &mut app::App, key: KeyCode) {
    match key {
        KeyCode::Char('o') => {
            app.input_mode = InputMode::AccountInput;
            app.input_buffer.clear();
            app.status_message = "Enter username (Enter=confirm, Esc=cancel):".to_string();
        }
        KeyCode::Char('m') => app.start_ms_login(),
        KeyCode::Esc => {
            app.input_mode = InputMode::Normal;
        }
        _ => {}
    }
}

fn handle_account_input(app: &mut app::App, key: KeyCode) {
    match key {
        KeyCode::Enter => app.confirm_add_offline_account(),
        KeyCode::Esc => {
            app.input_mode = InputMode::AccountView;
            app.input_buffer.clear();
        }
        KeyCode::Backspace => {
            app.input_buffer.pop();
        }
        KeyCode::Char(c) => app.input_buffer.push(c),
        _ => {}
    }
}
