mod app;
mod ui;

use anyhow::Result;
use app::InputMode;
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
            let releases: Vec<_> = versions
                .into_iter()
                .filter(|v| v.is_release())
                .take(50)
                .collect();
            let _ = tx.send(app::AsyncMessage::VersionsLoaded(releases));
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

            if app.input_mode == InputMode::Input {
                match key.code {
                    KeyCode::Enter => app.confirm_input(),
                    KeyCode::Esc => app.go_back(),
                    KeyCode::Backspace => {
                        app.input_buffer.pop();
                    }
                    KeyCode::Char(c) => app.input_buffer.push(c),
                    _ => {}
                }
                continue;
            }

            if app.input_mode == InputMode::SelectLoader {
                match key.code {
                    KeyCode::Up | KeyCode::Char('k') => app.loader_prev(),
                    KeyCode::Down | KeyCode::Char('j') => app.loader_next(),
                    KeyCode::Enter => app.confirm_loader(),
                    KeyCode::Esc => {
                        app.input_mode = InputMode::Normal;
                        app.status_message = "Cancelled.".to_string();
                    }
                    _ => {}
                }
                continue;
            }

            if app.input_mode == InputMode::CreateName {
                match key.code {
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
                continue;
            }

            if app.input_mode == InputMode::CreateSelectVersion {
                match key.code {
                    KeyCode::Up | KeyCode::Char('k') => app.create_version_prev(),
                    KeyCode::Down | KeyCode::Char('j') => app.create_version_next(),
                    KeyCode::Enter => app.create_version_confirm(),
                    KeyCode::Esc => {
                        app.input_mode = InputMode::Normal;
                        app.status_message = "Cancelled.".to_string();
                    }
                    _ => {}
                }
                continue;
            }

            if app.input_mode == InputMode::CreateSelectLoader {
                match key.code {
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
                continue;
            }

            if app.input_mode == InputMode::ManageInstance {
                match key.code {
                    KeyCode::Char('d') => {
                        app.input_mode = InputMode::ConfirmDelete;
                        app.status_message = "Delete this instance? [y]es [n]o".to_string();
                    }
                    KeyCode::Char('m') => app.open_manage_mods(),
                    KeyCode::Char('r') => app.open_manage_resourcepacks(),
                    KeyCode::Char('s') => app.open_manage_shaders(),
                    KeyCode::Char('w') => app.open_manage_saves(),
                    KeyCode::Char('o') => app.open_instance_folder(),
                    KeyCode::Char('j') => app.download_java_for_instance(),
                    KeyCode::Esc => {
                        app.input_mode = InputMode::Normal;
                        app.status_message = "Back to instances.".to_string();
                    }
                    _ => {}
                }
                continue;
            }

            if app.input_mode == InputMode::ConfirmDelete {
                match key.code {
                    KeyCode::Char('y') => app.confirm_delete_instance(),
                    _ => {
                        app.input_mode = InputMode::ManageInstance;
                        app.status_message = "Cancelled.".to_string();
                    }
                }
                continue;
            }

            if matches!(
                app.input_mode,
                InputMode::ManageMods
                    | InputMode::ManageResourcePacks
                    | InputMode::ManageShaders
                    | InputMode::ManageSaves
            ) {
                match key.code {
                    KeyCode::Up | KeyCode::Char('k') => app.manage_prev(),
                    KeyCode::Down | KeyCode::Char('j') => app.manage_next(),
                    KeyCode::Enter if app.input_mode == InputMode::ManageMods => {
                        app.toggle_current_mod();
                    }
                    KeyCode::Char('d') => app.delete_current_resource(),
                    KeyCode::Char('o') => {
                        if let Some(inst) = app.instances.get(app.selected_index) {
                            let instance_dir = miao_core::instance::Instance::instance_dir(
                                &app.config.instances_dir(),
                                &inst.name,
                            );
                            let dir = match app.input_mode {
                                InputMode::ManageMods => {
                                    miao_core::instance::Instance::mods_dir(&instance_dir)
                                }
                                InputMode::ManageResourcePacks => {
                                    miao_core::instance::Instance::resourcepacks_dir(&instance_dir)
                                }
                                InputMode::ManageShaders => {
                                    miao_core::instance::Instance::shaderpacks_dir(&instance_dir)
                                }
                                InputMode::ManageSaves => {
                                    miao_core::instance::Instance::saves_dir(&instance_dir)
                                }
                                _ => instance_dir,
                            };
                            let _ = miao_core::instance::open_folder(&dir);
                        }
                    }
                    KeyCode::Esc => {
                        app.input_mode = InputMode::ManageInstance;
                        app.status_message =
                            "[d]elete [m]ods [r]esourcepacks [s]haders [w]orlds [o]pen [j]ava [Esc]back"
                                .to_string();
                    }
                    _ => {}
                }
                continue;
            }

            match key.code {
                KeyCode::Char('q') => return Ok(()),
                KeyCode::Tab => app.next_tab(),
                KeyCode::BackTab => app.prev_tab(),
                KeyCode::Up | KeyCode::Char('k') => app.prev_item(),
                KeyCode::Down | KeyCode::Char('j') => app.next_item(),
                KeyCode::Enter => app.select_item(),
                KeyCode::Char('h') | KeyCode::Left => app.go_back(),
                KeyCode::Char('a') => app.start_add_account(),
                KeyCode::Char('e') => app.start_manage_instance(),
                KeyCode::Char('r') => app.refresh_instances(),
                KeyCode::Char('l') => app.launch_selected(),
                KeyCode::Char('n') => app.start_create_instance(),
                KeyCode::Char('m') => app.start_ms_login(),
                KeyCode::Char('s') => app.toggle_snapshots(),
                _ => {}
            }
        }
    }
}
