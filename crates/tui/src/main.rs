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
                    KeyCode::Enter => app.create_loader_confirm(),
                    KeyCode::Esc => {
                        app.input_mode = InputMode::Normal;
                        app.status_message = "Cancelled.".to_string();
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
